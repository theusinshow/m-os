//! A fila de saida, os conflitos e o relogio logico (migration 0027).
//!
//! Implementa as portas restantes de `mos_sync`. Tudo que decide *o que*
//! acontece mora no motor; aqui mora *onde fica guardado*.
//!
//! # Por que o payload e JSON e nao colunas
//!
//! O formato da operacao pertence ao CONTRATO, que versiona por conta propria.
//! Espalhar o contrato pelo schema faria toda mudanca de contrato virar
//! migration — e o contrato precisa poder evoluir mais rapido que o banco,
//! porque e ele que atravessa duas versoes de aplicativo que nao atualizam
//! juntas.
//!
//! O HLC sai do JSON e vira coluna porque a ORDEM se consulta: o envio sai em
//! ordem de instante, e ordenar por JSON custaria varredura.

use mos_sync::{
    ClockRepository, ConflictRepository, Conflito, Hlc, Op, OutboxRepository, Resultado, SyncError,
};
use rusqlite::{params, OptionalExtension};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{repository::format_time, SqliteStorage};

fn erro_sql(causa: rusqlite::Error) -> SyncError {
    SyncError::novo(format!("O banco local recusou a operacao: {causa}"), true)
}

fn erro_lock() -> SyncError {
    SyncError::novo("A conexao com o banco local ficou envenenada.", false)
}

fn agora() -> Resultado<String> {
    format_time(OffsetDateTime::now_utc())
        .map_err(|causa| SyncError::novo(causa.to_string(), false))
}

impl OutboxRepository for SqliteStorage {
    /// Enfileira uma operacao.
    ///
    /// `INSERT OR IGNORE` e a idempotencia virando SQL: a chave e o id da
    /// operacao, que nasceu na origem antes de qualquer envio. Enfileirar a
    /// mesma operacao dez vezes deixa uma linha — e e assim que o retry de um
    /// app que fechou no meio do envio nao duplica nada.
    fn enfileirar(&self, op: &Op) -> Resultado<()> {
        let connection = self.connection.lock().map_err(|_| erro_lock())?;
        let payload = serde_json::to_string(op)
            .map_err(|causa| SyncError::novo(format!("Operacao ilegivel: {causa}"), false))?;
        let momento = agora()?;
        connection
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
            .map_err(erro_sql)?;
        Ok(())
    }

    /// As proximas a enviar, na ordem em que aconteceram.
    ///
    /// A ordem importa mesmo o motor sendo indiferente a ela: enviar em ordem
    /// faz o outro lado ver a criacao antes da edicao, e um lote interrompido
    /// no meio deixa um estado que ja faz sentido.
    fn pendentes(&self, limite: usize) -> Resultado<Vec<Op>> {
        let connection = self.connection.lock().map_err(|_| erro_lock())?;
        let mut statement = connection
            .prepare(
                "SELECT payload FROM sync_outbox WHERE status IN ('pending', 'failed') \
                 ORDER BY hlc_wall_ms, hlc_counter, hlc_device LIMIT ?1",
            )
            .map_err(erro_sql)?;
        let linhas = statement
            .query_map(params![limite as i64], |row| row.get::<_, String>(0))
            .map_err(erro_sql)?
            .collect::<rusqlite::Result<Vec<String>>>()
            .map_err(erro_sql)?;

        linhas
            .into_iter()
            .map(|json| {
                serde_json::from_str::<Op>(&json).map_err(|causa| {
                    // Uma linha ilegivel nao pode derrubar o envio inteiro nem
                    // ser reenviada para sempre. Erro nao-retriavel: quem chama
                    // marca como falha permanente e segue.
                    SyncError::novo(format!("Operacao guardada esta ilegivel: {causa}"), false)
                })
            })
            .collect()
    }

    fn confirmar(&self, ids: &[Uuid]) -> Resultado<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let mut connection = self.connection.lock().map_err(|_| erro_lock())?;
        let transacao = connection.transaction().map_err(erro_sql)?;
        let momento = agora()?;
        {
            let mut statement = transacao
                .prepare("UPDATE sync_outbox SET status = 'acked', updated_at = ?2 WHERE id = ?1")
                .map_err(erro_sql)?;
            for id in ids {
                statement
                    .execute(params![id.to_string(), momento])
                    .map_err(erro_sql)?;
            }
        }
        transacao.commit().map_err(erro_sql)?;
        Ok(())
    }

    fn falhou(&self, id: Uuid, motivo: &str) -> Resultado<()> {
        let connection = self.connection.lock().map_err(|_| erro_lock())?;
        connection
            .execute(
                "UPDATE sync_outbox SET status = 'failed', attempts = attempts + 1, \
                 last_error = ?2, updated_at = ?3 WHERE id = ?1",
                params![id.to_string(), motivo, agora()?],
            )
            .map_err(erro_sql)?;
        Ok(())
    }

    fn quantidade_pendente(&self) -> Resultado<usize> {
        let connection = self.connection.lock().map_err(|_| erro_lock())?;
        let total: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sync_outbox WHERE status IN ('pending', 'failed')",
                [],
                |row| row.get(0),
            )
            .map_err(erro_sql)?;
        Ok(total as usize)
    }
}

impl ConflictRepository for SqliteStorage {
    /// Guarda o lado que perdeu.
    ///
    /// Esta funcao e a razao de a tabela existir. Sem ela, "resolver conflito"
    /// seria escolher um valor e apagar o outro sem ninguem saber — que e
    /// exatamente a perda silenciosa que o desenho recusa.
    fn registrar(
        &self,
        entity_kind: &str,
        entity_id: Uuid,
        conflitos: &[Conflito],
    ) -> Resultado<()> {
        if conflitos.is_empty() {
            return Ok(());
        }
        let mut connection = self.connection.lock().map_err(|_| erro_lock())?;
        let transacao = connection.transaction().map_err(erro_sql)?;
        let momento = agora()?;
        {
            let mut statement = transacao
                .prepare(
                    "INSERT INTO sync_conflicts \
                     (id, entity_kind, entity_id, field, winner_value, winner_device, \
                      winner_wall_ms, loser_value, loser_device, loser_wall_ms, \
                      acknowledged_at, created_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, '', ?11)",
                )
                .map_err(erro_sql)?;
            for conflito in conflitos {
                statement
                    .execute(params![
                        Uuid::now_v7().to_string(),
                        entity_kind,
                        entity_id.to_string(),
                        conflito.campo,
                        conflito.vencedor.valor.to_string(),
                        conflito.vencedor.at.device.to_string(),
                        conflito.vencedor.at.wall_ms,
                        conflito.perdedor.valor.to_string(),
                        conflito.perdedor.at.device.to_string(),
                        conflito.perdedor.at.wall_ms,
                        momento,
                    ])
                    .map_err(erro_sql)?;
            }
        }
        transacao.commit().map_err(erro_sql)?;
        Ok(())
    }

    fn abertos(&self, limite: usize) -> Resultado<Vec<Conflito>> {
        use mos_sync::{CampoResolvido, DeviceId};

        let connection = self.connection.lock().map_err(|_| erro_lock())?;
        let mut statement = connection
            .prepare(
                "SELECT field, winner_value, winner_device, winner_wall_ms, \
                 loser_value, loser_device, loser_wall_ms FROM sync_conflicts \
                 WHERE acknowledged_at = '' ORDER BY created_at DESC LIMIT ?1",
            )
            .map_err(erro_sql)?;
        let linhas = statement
            .query_map(params![limite as i64], |row| {
                let ler = |texto: String| {
                    serde_json::from_str::<serde_json::Value>(&texto)
                        .unwrap_or(serde_json::Value::Null)
                };
                let dispositivo = |texto: String| {
                    DeviceId(Uuid::parse_str(&texto).unwrap_or_else(|_| Uuid::nil()))
                };
                Ok(Conflito {
                    campo: row.get(0)?,
                    vencedor: CampoResolvido {
                        valor: ler(row.get(1)?),
                        at: Hlc::new(row.get(3)?, 0, dispositivo(row.get(2)?)),
                    },
                    perdedor: CampoResolvido {
                        valor: ler(row.get(4)?),
                        at: Hlc::new(row.get(6)?, 0, dispositivo(row.get(5)?)),
                    },
                })
            })
            .map_err(erro_sql)?
            .collect::<rusqlite::Result<Vec<Conflito>>>()
            .map_err(erro_sql)?;
        Ok(linhas)
    }
}

impl ClockRepository for SqliteStorage {
    fn carregar(&self) -> Resultado<Option<Hlc>> {
        use mos_sync::DeviceId;

        let connection = self.connection.lock().map_err(|_| erro_lock())?;
        connection
            .query_row(
                "SELECT hlc_wall_ms, hlc_counter, hlc_device FROM sync_clock WHERE only_row = 1",
                [],
                |row| {
                    let device: String = row.get(2)?;
                    Ok(Hlc::new(
                        row.get(0)?,
                        row.get(1)?,
                        DeviceId(Uuid::parse_str(&device).unwrap_or_else(|_| Uuid::nil())),
                    ))
                },
            )
            .optional()
            .map_err(erro_sql)
    }

    /// Guarda o ultimo instante emitido.
    ///
    /// Sem isto, reabrir o app com o relogio de parede atrasado geraria eventos
    /// que se ordenam ANTES de coisas ja sincronizadas — e a tela do outro
    /// dispositivo passaria a discordar da propria.
    fn guardar(&self, momento: Hlc) -> Resultado<()> {
        let connection = self.connection.lock().map_err(|_| erro_lock())?;
        connection
            .execute(
                "INSERT INTO sync_clock (only_row, hlc_wall_ms, hlc_counter, hlc_device, \
                 pull_cursor, updated_at) VALUES (1, ?1, ?2, ?3, '', ?4) \
                 ON CONFLICT(only_row) DO UPDATE SET hlc_wall_ms = ?1, hlc_counter = ?2, \
                 hlc_device = ?3, updated_at = ?4",
                params![
                    momento.wall_ms,
                    momento.counter,
                    momento.device.to_string(),
                    agora()?
                ],
            )
            .map_err(erro_sql)?;
        Ok(())
    }

    fn cursor(&self) -> Resultado<String> {
        let connection = self.connection.lock().map_err(|_| erro_lock())?;
        let cursor: Option<String> = connection
            .query_row(
                "SELECT pull_cursor FROM sync_clock WHERE only_row = 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(erro_sql)?;
        Ok(cursor.unwrap_or_default())
    }

    fn guardar_cursor(&self, cursor: &str) -> Resultado<()> {
        let connection = self.connection.lock().map_err(|_| erro_lock())?;
        connection
            .execute(
                "INSERT INTO sync_clock (only_row, hlc_wall_ms, hlc_counter, hlc_device, \
                 pull_cursor, updated_at) VALUES (1, 0, 0, '', ?1, ?2) \
                 ON CONFLICT(only_row) DO UPDATE SET pull_cursor = ?1, updated_at = ?2",
                params![cursor, agora()?],
            )
            .map_err(erro_sql)?;
        Ok(())
    }
}
