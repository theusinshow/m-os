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
        let mut slot = self.sync.lock().map_err(map_lock_error)?;
        let Some(relogio) = slot.as_mut() else {
            return Ok(());
        };

        let agora_ms = OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000;
        let instante = relogio.tick(agora_ms as i64);
        let op = Op::new(Uuid::now_v7(), entidade, corpo, instante);

        gravar_op(transacao, &op)?;
        gravar_relogio(transacao, instante)?;
        Ok(())
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
