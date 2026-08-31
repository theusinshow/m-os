//! Emissao de operacoes junto com a mutacao.
//!
//! # A regra que este arquivo existe para garantir
//!
//! **A operacao entra na MESMA transacao da mudanca.** Nao é detalhe de
//! implementacao — é a diferenca entre um sistema que sincroniza e um que
//! mente:
//!
//! - Gravar a Capture e falhar ao enfileirar deixa uma Capture que **nunca vai
//!   sair deste dispositivo**, e ninguem fica sabendo. É perda silenciosa, que
//!   é justamente o que o desenho de sync recusa.
//! - Enfileirar e falhar ao gravar manda para o outro lado uma mudanca que
//!   **nao aconteceu aqui**. O outro dispositivo passa a mostrar algo que este
//!   nao tem.
//!
//! Uma transacao só torna os dois impossiveis. Ou as duas coisas acontecem, ou
//! nenhuma.
//!
//! # Por que o relogio tambem entra na transacao
//!
//! O instante emitido precisa sobreviver junto com a operacao que o usou. Se a
//! operacao commitar e o relogio nao, reabrir o app reemitiria aquele instante
//! para outra operacao — e duas operacoes diferentes com o mesmo instante e o
//! mesmo dispositivo quebram a ordem total.
//!
//! # Quando o sync esta desligado
//!
//! `habilitar_sync` nunca foi chamado: nada é emitido, e nenhuma mutacao falha
//! por causa disso. É o estado dos testes que nao falam de sync e de qualquer
//! instalacao antes de a sincronizacao ser ligada — e é o que permite ligar isto
//! por entidade, uma de cada vez, sem parar o desktop.

use mos_core::{CoreError, ErrorCode};
use mos_sync::{DeviceId, EntityRef, Hlc, HlcClock, Op, OpBody};
use rusqlite::{params, Connection};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{map_lock_error, map_sql_error, SqliteStorage};

fn erro(mensagem: impl Into<String>) -> CoreError {
    CoreError::new(ErrorCode::StorageUnavailable, mensagem, false)
}

/// O mesmo erro, visivel para a projecao — que falha pelas mesmas razoes.
pub(crate) fn erro_de_sync(mensagem: impl Into<String>) -> CoreError {
    erro(mensagem)
}

impl SqliteStorage {
    /// Liga a emissao de operacoes para este dispositivo.
    ///
    /// Chamado uma vez, na abertura, depois de `este_dispositivo`. Antes disso
    /// o M/OS funciona igual e nao emite nada — a sincronizacao é uma camada
    /// por cima, e nao um requisito para o sistema existir.
    pub fn habilitar_sync(&self, device: DeviceId) -> Result<(), CoreError> {
        use mos_sync::ClockRepository;

        let guardado = self
            .carregar()
            .map_err(|causa| erro(causa.mensagem))?
            // Só herda o relogio se ele for DESTE dispositivo: um banco
            // restaurado de backup de outra maquina traz o relogio dela junto,
            // e herdar identidade alheia quebraria o desempate.
            .filter(|ultimo| ultimo.device == device);

        let relogio = match guardado {
            Some(ultimo) => HlcClock::restaurar(device, ultimo),
            None => HlcClock::new(device),
        };
        let mut slot = self.sync.lock().map_err(map_lock_error)?;
        *slot = Some(relogio);
        Ok(())
    }

    /// Se a emissao esta ligada.
    pub fn sync_ligado(&self) -> bool {
        self.sync.lock().map(|slot| slot.is_some()).unwrap_or(false)
    }

    /// Registra uma mudanca, **dentro da transacao que a fez**.
    ///
    /// Silencioso quando o sync esta desligado. Devolve erro apenas quando a
    /// emissao esta ligada e falhou — e ai a transacao inteira precisa cair,
    /// porque a alternativa é gravar a mudanca sem a operacao.
    pub(crate) fn emitir(
        &self,
        transacao: &Connection,
        entidade: EntityRef,
        corpo: OpBody,
    ) -> Result<(), CoreError> {
        // O CANARIO DO PORTAO.
        //
        // Aqui e a linha exata onde o abraco mortal nascia: a conexao ja esta na
        // mao de quem chamou, e agora o relogio vai ser pedido. Se o portao
        // estiver LIVRE, quem chamou nao passou por `escrita()` — e essa escrita
        // e uma bomba-relogio que so estoura quando cair dentro de uma rodada.
        //
        // `try_lock` responde `Err` de duas formas, e as duas sao aceitaveis: o
        // portao esta com esta thread (o caso correto) ou com outra. Ele responde
        // `Ok` numa unica situacao — ninguem o tem —, e ai o defeito e certo.
        //
        // Fica em `debug_assert` porque o preco de errar o julgamento e alto nos
        // dois sentidos: em teste ele para na hora e aponta o metodo; em release
        // ele some, e um falso positivo nao derruba escrita de ninguem.
        debug_assert!(
            self.portao.try_lock().is_err(),
            "escrita emitindo operacao sem passar por SqliteStorage::escrita():              pegue a conexao com `self.escrita()?` em vez de `self.connection.lock()`"
        );

        let mut slot = self.sync.lock().map_err(map_lock_error)?;
        let Some(relogio) = slot.as_mut() else {
            return Ok(());
        };

        let agora_ms = OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000;
        let instante = relogio.tick(agora_ms as i64);
        let op = Op::new(Uuid::now_v7(), entidade, corpo, instante);

        gravar_op(transacao, &op)?;
        gravar_relogio(transacao, instante)?;
        // A mudanca local entra na tabela sombra AGORA, e nao quando voltar do
        // servidor. Sem isto a reconciliacao so conheceria o que veio de fora, e
        // uma operacao remota antiga venceria uma edicao local recente que ela
        // nunca viu — perda silenciosa, que e exatamente o que este arquivo
        // existe para impedir.
        //
        // O `slot` e solto antes: `absorver_local` nao toca no relogio, mas
        // segurar o mutex por mais tempo que o necessario e como se cria o
        // deadlock que so aparece com dois caminhos concorrentes.
        drop(slot);
        self.absorver_local(transacao, &op)?;
        Ok(())
    }

    /// Liga ou desliga um vinculo do Knowledge Graph.
    ///
    /// Sempre `Update`, e nunca `Delete`: o `Delete` do motor tem semantica de
    /// "apagar ganha de editar", que esta certa para uma Task e ERRADA para um
    /// interruptor — desvincular as 10:00 e revincular as 10:05 tem que
    /// terminar vinculado. Ver `mos_sync::Relacao`.
    pub(crate) fn emitir_relacao(
        &self,
        transacao: &Connection,
        kind: &str,
        from: Uuid,
        to: Uuid,
        linked: bool,
    ) -> Result<(), CoreError> {
        let relacao = mos_sync::Relacao::nova(kind, from, to);
        self.emitir(transacao, relacao.entidade(), relacao.alternar(linked))
    }

    /// Açucar para o caso comum: uma mudanca de campos.
    pub(crate) fn emitir_update(
        &self,
        transacao: &Connection,
        kind: &str,
        id: Uuid,
        campos: &[(&str, serde_json::Value)],
    ) -> Result<(), CoreError> {
        let fields = campos
            .iter()
            .map(|(nome, valor)| ((*nome).to_owned(), valor.clone()))
            .collect();
        self.emitir(
            transacao,
            EntityRef::new(kind, id),
            OpBody::Update { fields },
        )
    }
}

fn gravar_op(transacao: &Connection, op: &Op) -> Result<(), CoreError> {
    let payload = serde_json::to_string(op)
        .map_err(|causa| erro(format!("Operacao de sincronizacao ilegivel: {causa}")))?;
    let momento = crate::repository::format_time(OffsetDateTime::now_utc())?;
    transacao
        .execute(
            "INSERT OR IGNORE INTO sync_outbox \
             (id, entity_kind, entity_id, hlc_wall_ms, hlc_counter, hlc_device, payload, \
              status, attempts, last_error, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', 0, '', ?8, ?8)",
            params![
                op.id.to_string(),
                op.entity.kind.as_str(),
                op.entity.id.to_string(),
                op.at.wall_ms,
                op.at.counter,
                op.at.device.to_string(),
                payload,
                momento,
            ],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

fn gravar_relogio(transacao: &Connection, instante: Hlc) -> Result<(), CoreError> {
    let momento = crate::repository::format_time(OffsetDateTime::now_utc())?;
    transacao
        .execute(
            "INSERT INTO sync_clock (only_row, hlc_wall_ms, hlc_counter, hlc_device, \
             pull_cursor, updated_at) VALUES (1, ?1, ?2, ?3, '', ?4) \
             ON CONFLICT(only_row) DO UPDATE SET hlc_wall_ms = ?1, hlc_counter = ?2, \
             hlc_device = ?3, updated_at = ?4",
            params![
                instante.wall_ms,
                instante.counter,
                instante.device.to_string(),
                momento
            ],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

/// O espaco de nomes dos ids derivados de chave composta.
///
/// Constante e arbitrario, como todo namespace de UUID v5 — e como o das
/// relacoes (`mos_sync::Relacao`), inclusive na advertencia que vem junto: muda-lo
/// faria todas as entidades existentes ganharem ids novos, e as antigas ficariam
/// orfas no hub e nos dois aparelhos.
const NAMESPACE_COMPOSTO: Uuid = Uuid::from_bytes([
    0x6d, 0x6f, 0x73, 0x63, 0x68, 0x61, 0x76, 0x65, 0x63, 0x6f, 0x6d, 0x70, 0x6f, 0x73, 0x74, 0x61,
]);

/// O id de entidade de uma linha cuja chave primaria e composta.
///
/// Tres tabelas do M/OS tem chave `(provider, ...)` em vez de UUID, e o `Op`
/// exige `entity.id: Uuid`. Derivar em vez de sortear e o que faz os dois
/// aparelhos chegarem ao MESMO id sem se falarem — a mesma razao pela qual a
/// relacao deriva o dela do par que ela liga.
pub(crate) fn id_composto(kind: &str, partes: &[&str]) -> Uuid {
    let mut semente = String::from(kind);
    for parte in partes {
        semente.push('\u{1f}');
        semente.push_str(parte);
    }
    Uuid::new_v5(&NAMESPACE_COMPOSTO, semente.as_bytes())
}

impl SqliteStorage {
    /// Emite a nota e a situacao que o provedor informou para uma disciplina.
    ///
    /// Separado do resto da importacao porque a importacao roda dentro da
    /// transacao dela e este metodo precisa ser chamavel dos dois lugares: do
    /// caminho do provedor e do backfill.
    pub(crate) fn emitir_fato_de_disciplina(
        &self,
        provider: &str,
        subject_id: &str,
        situacao: Option<&str>,
        nota: Option<f64>,
    ) -> Result<(), CoreError> {
        let connection = self.escrita()?;
        let transacao = connection.unchecked_transaction().map_err(map_sql_error)?;
        self.emitir_fato_de_disciplina_em(&transacao, provider, subject_id, situacao, nota)?;
        transacao.commit().map_err(map_sql_error)?;
        Ok(())
    }

    /// A mesma emissao, dentro de uma transacao que ja existe.
    pub(crate) fn emitir_fato_de_disciplina_em(
        &self,
        transacao: &Connection,
        provider: &str,
        subject_id: &str,
        situacao: Option<&str>,
        nota: Option<f64>,
    ) -> Result<(), CoreError> {
        self.emitir_update(
            transacao,
            "academic_provider_subject_fact",
            id_composto("academic_provider_subject_fact", &[provider, subject_id]),
            &[
                ("provider", serde_json::json!(provider)),
                ("subjectId", serde_json::json!(subject_id)),
                (
                    "situation",
                    situacao.map_or(serde_json::Value::Null, |valor| serde_json::json!(valor)),
                ),
                (
                    "officialGrade",
                    nota.map_or(serde_json::Value::Null, |valor| serde_json::json!(valor)),
                ),
            ],
        )
    }
}
