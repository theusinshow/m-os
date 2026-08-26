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
    /// Preenchido quando a rodada parou por erro. O que ja foi feito ate ali
    /// permanece feito — sincronizacao parcial e melhor que nenhuma.
    pub erro: Option<String>,
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

    // ---- empurra ----------------------------------------------------------
    match deposito.outbox.pendentes(limite) {
        Ok(pendentes) if !pendentes.is_empty() => {
            match transporte.push(CONTRACT_VERSION, &pendentes) {
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
                    for op in &pendentes {
                        let _ = deposito.outbox.falhou(op.id, &causa.mensagem);
                    }
                    rodada.erro = Some(causa.mensagem);
                }
            }
        }
        Ok(_) => {}
        Err(causa) => rodada.erro = Some(causa.mensagem),
    }

    // ---- puxa -------------------------------------------------------------
    if rodada.erro.is_none() {
        let cursor = deposito.relogio.cursor().unwrap_or_default();
        match transporte.pull(CONTRACT_VERSION, &cursor, limite) {
            Ok(lote) => {
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
                    let base = projecao.estado_de(&ops[0]);
                    let resultado = aplicar(base, &ops);
                    rodada.conflitos += resultado.conflitos.len();
                    if !resultado.conflitos.is_empty() {
                        let _ = deposito
                            .conflitos
                            .registrar(&kind, id, &resultado.conflitos);
                    }
                    if let Err(causa) = projecao.guardar(&ops[0], &resultado.estado) {
                        rodada.erro = Some(causa.mensagem);
                        break;
                    }
                }

                if rodada.erro.is_none() && !lote.proximo_cursor.is_empty() {
                    let _ = deposito.relogio.guardar_cursor(&lote.proximo_cursor);
                }
                let _ = deposito.relogio.guardar(relogio.ultimo());
            }
            Err(causa) => rodada.erro = Some(causa.mensagem),
        }
    }

    rodada.pendentes = deposito.outbox.quantidade_pendente().unwrap_or(0);
    rodada
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
