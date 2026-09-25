//! O laco de sincronizacao: empurra o que saiu daqui, puxa o que veio de la.
//!
//! # O que o motor garante
//!
//! - **Nunca perde uma operacao local.** Ela sai da fila quando o outro lado
//!   confirma, e nao quando foi enviada. App fechado no meio do envio reenvia.
//! - **Nunca duplica.** A chave de idempotencia nasceu na origem, e o `push`
//!   pode ser repetido a vontade.
//! - **Custo proporcional ao que mudou.** O `pull` leva um cursor; ninguem
//!   baixa o banco inteiro para descobrir que nada mudou.
//! - **Ordem de chegada nao importa.** Quem decide e o instante da operacao.
//!
//! # O que o motor NAO decide
//!
//! Como falar com o outro lado. Isso e o `Transport`, e existe de proposito
//! como trait: hoje nao ha servidor, e o motor precisa poder ser exercitado
//! inteiro sem um. A implementacao de teste em `LocalTransport` e tambem a
//! especificacao executavel do que um servidor tera que fazer.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    aplicar, ClockRepository, ConflictRepository, DeviceRepository, EstadoDaEntidade, Hlc,
    HlcClock, Op, OutboxRepository, Resultado, SyncError, CONTRACT_VERSION,
};

/// O que um lote de `pull` traz.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lote {
    pub ops: Vec<Op>,
    /// Onde continuar da proxima vez. Vazio significa "acabou".
    pub proximo_cursor: String,
    /// Verdadeiro quando ha mais para buscar. O cliente decide se busca agora
    /// ou depois — no iPhone, "depois" pode ser a proxima abertura.
    pub tem_mais: bool,
}

/// Como este dispositivo fala com o outro lado.
///
/// Trait, e nao HTTP direto, por dois motivos independentes: o motor precisa
/// ser testavel sem rede, e o transporte real ainda nao existe. Quando existir,
/// ele implementa isto e nada no motor muda.
pub trait Transport: Send + Sync {
    /// Manda operacoes. Devolve os ids aceitos.
    ///
    /// Aceitar e diferente de aplicar: o outro lado pode ja ter aquela
    /// operacao, e ainda assim confirma — e isso que faz o retry ser seguro.
    fn push(&self, contrato: u32, ops: &[Op]) -> Resultado<Vec<uuid::Uuid>>;

    /// Busca o que mudou desde o cursor.
    fn pull(&self, contrato: u32, cursor: &str, limite: usize) -> Resultado<Lote>;
}

/// O resultado de uma rodada, para o log estruturado e para a interface.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rodada {
    pub enviadas: usize,
    pub recebidas: usize,
    pub conflitos: usize,
    pub pendentes: usize,
    /// O outro lado ainda tem mais depois deste lote.
    ///
    /// Vem do `Lote`, e nao de `recebidas == limite`. O atalho erra justamente
    /// quando o lote acaba exato: quem chama pediria mais uma rodada para
    /// receber vazio — e no celular uma ida a toa e radio ligado a toa. Quem
    /// esvazia a fila em varias rodadas precisa desta resposta para saber
    /// quando parar.
    pub tem_mais: bool,
    /// Quantas ENTIDADES de cada tipo mudaram nesta rodada.
    ///
    /// Nao e `recebidas` reparticionado: aquele conta OPERACOES, e tres edicoes
    /// da mesma Task sao tres operacoes e uma task. A faixa da Home usa este
    /// mapa justamente porque "3 tasks chegaram" precisa significar tres tasks.
    ///
    /// Chave `String` e nao enum: `EntityKind` e texto (§9), e um tipo que este
    /// cliente ainda nao conhece precisa CONTAR em vez de sumir — senao a faixa
    /// diz que nada chegou quando algo chegou.
    #[serde(default)]
    pub recebidas_por_tipo: BTreeMap<String, usize>,
    /// Preenchido quando a rodada parou por erro. O que ja foi feito ate ali
    /// permanece feito — sincronizacao parcial e melhor que nenhuma.
    pub erro: Option<String>,
    /// Se o `erro` e do tipo que passa numa nova tentativa sem ninguem mexer.
    ///
    /// Viajava desde o transporte em `SyncError::retriavel` e morria aqui. E o
    /// que separa "sem rede, volto em dez segundos" de "segredo errado, avise".
    #[serde(default)]
    pub erro_retriavel: bool,
}

/// A ponte entre operacao e dominio.
///
/// O motor sabe reconciliar, mas nao sabe o que e uma Task. Quem traduz
/// operacao em entidade e o adaptador — e essa fronteira e o que permite
/// acrescentar um tipo novo de entidade sem tocar no motor.
///
/// Trait com `&mut self`, e nao duas closures: as duas metades mexem no MESMO
/// estado materializado, e duas closures nao conseguem emprestar isso ao mesmo
/// tempo. A trait tambem deixa a fronteira com nome, que e o que ela merece.
pub trait Projecao {
    /// O estado que este dispositivo ja tem da entidade que a operacao toca.
    fn estado_de(&self, op: &Op) -> EstadoDaEntidade;

    /// Grava o estado reconciliado.
    fn guardar(&mut self, op: &Op, estado: &EstadoDaEntidade) -> Resultado<()>;
}

/// As quatro portas de que o motor precisa, juntas.
pub struct Deposito<'a> {
    pub outbox: &'a dyn OutboxRepository,
    pub conflitos: &'a dyn ConflictRepository,
    pub relogio: &'a dyn ClockRepository,
    pub dispositivos: &'a dyn DeviceRepository,
}

/// Uma rodada completa: empurra, puxa, reconcilia.
///
/// A `Projecao` e o gancho com o dominio. Ver a trait.
pub fn sincronizar(
    deposito: &Deposito<'_>,
    transporte: &dyn Transport,
    relogio: &mut HlcClock,
    projecao: &mut dyn Projecao,
    agora_ms: i64,
    limite: usize,
) -> Rodada {
    let mut rodada = Rodada::default();
    if let Some(pendentes) = preparar_envio(deposito, limite, &mut rodada) {
        let resposta = transporte.push(CONTRACT_VERSION, &pendentes);
        registrar_envio(deposito, &pendentes, resposta, &mut rodada);
    }
    if rodada.erro.is_none() {
        let cursor = deposito.relogio.cursor().unwrap_or_default();
        let lote = transporte.pull(CONTRACT_VERSION, &cursor, limite);
        aplicar_recebido(deposito, relogio, projecao, lote, agora_ms, &mut rodada);
    }
    rodada.pendentes = deposito.outbox.quantidade_pendente().unwrap_or(0);
    rodada
}

// ---------------------------------------------------------------- as fases
//
// A rodada em quatro pedacos, para quem precisa soltar os cadeados enquanto a
// rede anda (F3.1 do MOS-OVERHAUL-PLAN, ADR-078). A ordem e sempre esta:
// `preparar_envio` → push → `registrar_envio` → pull → `aplicar_recebido`.
// So as fases com nome tocam o deposito; o push e o pull sao do transporte, e
// podem demorar o quanto a rede quiser sem prender escrita nenhuma.
//
// Por que e seguro: as operacoes lidas em `preparar_envio` ja estao emitidas e
// sao imutaveis; o ack confirma POR ID, entao uma operacao emitida durante o
// push nunca e confirmada por ele. E o lote puxado e aplicado sobre o estado
// de sombra do momento da aplicacao, com o relogio observando cada instante
// remoto — uma escrita local que caiu entre o pull e a aplicacao tem instante
// proprio e entra no merge por campo como qualquer outra.

/// Le o lote a enviar. `None` quando nao ha nada, ou quando a leitura falhou
/// (a falha fica na rodada).
pub fn preparar_envio(
    deposito: &Deposito<'_>,
    limite: usize,
    rodada: &mut Rodada,
) -> Option<Vec<Op>> {
    match deposito.outbox.pendentes(limite) {
        Ok(pendentes) if !pendentes.is_empty() => Some(pendentes),
        Ok(_) => None,
        Err(causa) => {
            rodada.erro_retriavel = causa.retriavel;
            rodada.erro = Some(causa.mensagem);
            None
        }
    }
}

/// Registra a resposta do push: confirma o que o outro lado aceitou, ou marca
/// a falha em cada operacao enviada.
pub fn registrar_envio(
    deposito: &Deposito<'_>,
    enviadas: &[Op],
    resposta: Resultado<Vec<uuid::Uuid>>,
    rodada: &mut Rodada,
) {
    match resposta {
        Ok(aceitas) => {
            rodada.enviadas = aceitas.len();
            if let Err(causa) = deposito.outbox.confirmar(&aceitas) {
                rodada.erro = Some(causa.mensagem);
            }
        }
        Err(causa) => {
            // Falha de envio marca cada operacao, e nao a fila inteira:
            // e a contagem por operacao que alimenta o backoff e o
            // diagnostico de "esta travada ha quanto tempo".
            for op in enviadas {
                let _ = deposito.outbox.falhou(op.id, &causa.mensagem);
            }
            rodada.erro_retriavel = causa.retriavel;
            rodada.erro = Some(causa.mensagem);
        }
    }
}

/// Reconcilia o que o pull trouxe, avanca o cursor e guarda o relogio.
pub fn aplicar_recebido(
    deposito: &Deposito<'_>,
    relogio: &mut HlcClock,
    projecao: &mut dyn Projecao,
    lote: Resultado<Lote>,
    agora_ms: i64,
    rodada: &mut Rodada,
) {
    let lote = match lote {
        Ok(lote) => lote,
        Err(causa) => {
            rodada.erro_retriavel = causa.retriavel;
            rodada.erro = Some(causa.mensagem);
            return;
        }
    };
    rodada.recebidas = lote.ops.len();
    rodada.tem_mais = lote.tem_mais;

    // Agrupa por entidade: reconciliar uma entidade de cada vez e o
    // que permite gravar estado e conflito juntos, sem meio-termo.
    let mut por_entidade: BTreeMap<(String, uuid::Uuid), Vec<Op>> = BTreeMap::new();
    for op in &lote.ops {
        // Absorve o instante remoto ANTES de qualquer coisa: depois
        // disto, tudo que este dispositivo emitir vem depois do que
        // ele acabou de ver.
        relogio.observar(op.at, agora_ms);
        por_entidade
            .entry((op.entity.kind.as_str().to_owned(), op.entity.id))
            .or_default()
            .push(op.clone());
    }

    for ((kind, id), ops) in por_entidade {
        // Conta ANTES do que pode falhar: o `break` do erro esta
        // logo abaixo, e o que ja chegou permanece chegado.
        *rodada.recebidas_por_tipo.entry(kind.clone()).or_insert(0) += 1;
        let base = projecao.estado_de(&ops[0]);
        let resultado = aplicar(base, &ops);
        rodada.conflitos += resultado.conflitos.len();
        if !resultado.conflitos.is_empty() {
            let _ = deposito
                .conflitos
                .registrar(&kind, id, &resultado.conflitos);
        }
        if let Err(causa) = projecao.guardar(&ops[0], &resultado.estado) {
            rodada.erro_retriavel = causa.retriavel;
            rodada.erro = Some(causa.mensagem);
            break;
        }
    }

    if rodada.erro.is_none() && !lote.proximo_cursor.is_empty() {
        let _ = deposito.relogio.guardar_cursor(&lote.proximo_cursor);
    }
    let _ = deposito.relogio.guardar(relogio.ultimo());
}

/// Restaura o relogio do disco, ou comeca um novo.
///
/// Chamado uma vez, na abertura. Comecar do zero quando ha um relogio guardado
/// faria este dispositivo reemitir instantes que ja usou.
pub fn carregar_relogio(
    relogio_repo: &dyn ClockRepository,
    device: crate::DeviceId,
) -> Resultado<HlcClock> {
    match relogio_repo.carregar()? {
        // So restaura se o relogio guardado for DESTE dispositivo. Um banco
        // restaurado de backup de outra maquina traria o relogio dela junto, e
        // herdar identidade alheia quebraria o desempate.
        Some(ultimo) if ultimo.device == device => Ok(HlcClock::restaurar(device, ultimo)),
        _ => Ok(HlcClock::new(device)),
    }
}

/// Erro de contrato incompativel, para o transporte devolver.
pub fn erro_de_contrato(recebido: u32) -> SyncError {
    SyncError::novo(
        format!(
            "O outro lado fala o contrato {recebido}, e este M/OS fala {CONTRACT_VERSION}. \
             Atualize o aplicativo mais antigo."
        ),
        false,
    )
}

/// O instante em que uma operacao entra numa fila, para quem precisa registrar.
pub fn instante_de(op: &Op) -> Hlc {
    op.at
}
