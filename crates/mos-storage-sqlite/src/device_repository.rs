//! Persistencia da identidade dos dispositivos (migration 0027).
//!
//! Implementa `mos_sync::DeviceRepository`. A regra de quem e quem vive no
//! motor de sincronizacao; aqui so mora o SQL.
//!
//! O erro atravessa a fronteira e vira `SyncError`: `mos-sync` nao depende de
//! `mos-core` de proposito — ele precisa compilar sozinho no iOS —, entao a
//! conversao acontece aqui, que e o lugar de converter.

use mos_sync::{Device, DeviceId, DeviceRepository, Platform, Resultado, SyncError};
use rusqlite::{params, OptionalExtension};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{repository::format_time, SqliteStorage};

fn erro_sql(causa: rusqlite::Error) -> SyncError {
    // Toda falha de SQLite e tratada como retriavel: disco ocupado, lock e
    // I/O transitorio sao o caso comum, e o caso raro — banco corrompido — ja
    // e recusado na abertura pela verificacao de integridade.
    SyncError::novo(format!("O banco local recusou a operacao: {causa}"), true)
}

fn erro_lock() -> SyncError {
    SyncError::novo("A conexao com o banco local ficou envenenada.", false)
}

fn ler_device(row: &rusqlite::Row<'_>) -> rusqlite::Result<Device> {
    let id: String = row.get(0)?;
    let plataforma: String = row.get(2)?;
    Ok(Device {
        id: DeviceId(Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::nil())),
        name: row.get(1)?,
        platform: Platform::ler(&plataforma),
        app_version: row.get(3)?,
        last_sync_at: row.get(4)?,
        is_this_device: row.get::<_, i64>(5)? == 1,
    })
}

const COLUNAS: &str =
    "id, name, platform, app_version, last_sync_at, is_this_device FROM devices";

impl DeviceRepository for SqliteStorage {
    /// Idempotente: e chamado em toda abertura do app.
    ///
    /// Se ja existe um "este dispositivo", ele e atualizado — nome e versao do
    /// app mudam, o id nao. Criar um dispositivo novo a cada abertura encheria
    /// a lista do §9 de fantasmas e faria o desempate do HLC mudar de resposta
    /// entre execucoes, que e pior: a ordem total deixaria de ser estavel.
    fn este_dispositivo(
        &self,
        nome: &str,
        plataforma: &str,
        versao: &str,
    ) -> Resultado<Device> {
        let connection = self.connection.lock().map_err(|_| erro_lock())?;
        // `format_time` devolve `Result<_, CoreError>`, e este crate nao fala
        // `CoreError` — a conversao acontece na fronteira, como o resto do
        // arquivo. Uma data que nao formata e defeito de programa, nao de uso.
        let agora = format_time(OffsetDateTime::now_utc())
            .map_err(|causa| SyncError::novo(causa.to_string(), false))?;

        let existente: Option<String> = connection
            .query_row(
                "SELECT id FROM devices WHERE is_this_device = 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(erro_sql)?;

        let id = match existente {
            Some(id) => {
                connection
                    .execute(
                        "UPDATE devices SET name = ?2, platform = ?3, app_version = ?4, \
                         updated_at = ?5 WHERE id = ?1",
                        params![id, nome, plataforma, versao, agora],
                    )
                    .map_err(erro_sql)?;
                id
            }
            None => {
                let id = DeviceId::novo().to_string();
                connection
                    .execute(
                        "INSERT INTO devices (id, name, platform, app_version, last_sync_at, \
                         is_this_device, created_at, updated_at) \
                         VALUES (?1, ?2, ?3, ?4, '', 1, ?5, ?5)",
                        params![id, nome, plataforma, versao, agora],
                    )
                    .map_err(erro_sql)?;
                id
            }
        };

        connection
            .query_row(
                &format!("SELECT {COLUNAS} WHERE id = ?1"),
                params![id],
                ler_device,
            )
            .map_err(erro_sql)
    }

    fn listar(&self) -> Resultado<Vec<Device>> {
        let connection = self.connection.lock().map_err(|_| erro_lock())?;
        let mut statement = connection
            // Este dispositivo primeiro, depois os outros pelo mais recente:
            // e a ordem que a tela do §9 quer, e ela nao muda de lugar quando
            // outro aparelho sincroniza.
            .prepare(&format!(
                "SELECT {COLUNAS} ORDER BY is_this_device DESC, last_sync_at DESC, name"
            ))
            .map_err(erro_sql)?;
        let linhas = statement
            .query_map([], ler_device)
            .map_err(erro_sql)?
            .collect::<rusqlite::Result<Vec<Device>>>()
            .map_err(erro_sql)?;
        Ok(linhas)
    }

    fn marcar_sync(&self, id: DeviceId, quando: &str) -> Resultado<()> {
        let connection = self.connection.lock().map_err(|_| erro_lock())?;
        connection
            .execute(
                "UPDATE devices SET last_sync_at = ?2, updated_at = ?2 WHERE id = ?1",
                params![id.to_string(), quando],
            )
            .map_err(erro_sql)?;
        Ok(())
    }
}
