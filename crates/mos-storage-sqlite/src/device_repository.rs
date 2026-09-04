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

const COLUNAS: &str = "id, name, platform, app_version, last_sync_at, is_this_device FROM devices";

impl DeviceRepository for SqliteStorage {
    /// Idempotente: e chamado em toda abertura do app.
    ///
    /// Se ja existe um "este dispositivo", ele e atualizado — nome e versao do
    /// app mudam, o id nao. Criar um dispositivo novo a cada abertura encheria
    /// a lista do §9 de fantasmas e faria o desempate do HLC mudar de resposta
    /// entre execucoes, que e pior: a ordem total deixaria de ser estavel.
    fn este_dispositivo(&self, nome: &str, plataforma: &str, versao: &str) -> Resultado<Device> {
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

        // A ANCORA, e por que ela existe.
        //
        // A linha de `devices` pode sumir — banco recriado, limpeza por fora — e
        // com ela ia o id. Id novo significa relogio novo e cursor zerado: o
        // aparelho volta a baixar tudo, e aparece no hub como um SEGUNDO
        // dispositivo com os mesmos dados. Aconteceu em 02/09/2026, no PC do
        // trabalho, e custou uma manha para ser entendido.
        //
        // `app_metadata` sobrevive a isso porque nada no sync a apaga: ela nao
        // esta em `SINCRONIZAVEIS` nem e tocada pela projecao.
        let ancorado: Option<String> = connection
            .query_row(
                "SELECT value FROM app_metadata WHERE key = 'sync_device_id'",
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
                // A ancora tambem nasce AQUI, e nao so no ramo de baixo.
                //
                // Sem isto ela so apareceria em instalacao nova — e as maquinas
                // que ja existem, que sao justamente as que podem perder a
                // linha, nunca ganhariam a protecao. `OR IGNORE` porque a
                // ancora, uma vez gravada, nao se corrige: ela e a memoria de
                // quem este aparelho SEMPRE foi.
                connection
                    .execute(
                        "INSERT OR IGNORE INTO app_metadata (key, value) \
                         VALUES ('sync_device_id', ?1)",
                        params![id],
                    )
                    .map_err(erro_sql)?;
                id
            }
            None => {
                // Sem linha: o id vem da ancora, e so nasce novo quando nem ela
                // existe. As duas gravacoes acontecem na MESMA transacao — uma
                // ancora sem linha ressuscitaria um id que o hub nunca viu.
                let id = ancorado.unwrap_or_else(|| DeviceId::novo().to_string());
                let transacao = connection.unchecked_transaction().map_err(erro_sql)?;
                transacao
                    .execute(
                        "INSERT INTO devices (id, name, platform, app_version, last_sync_at, \
                         is_this_device, created_at, updated_at) \
                         VALUES (?1, ?2, ?3, ?4, '', 1, ?5, ?5)",
                        params![id, nome, plataforma, versao, agora],
                    )
                    .map_err(erro_sql)?;
                transacao
                    .execute(
                        "INSERT INTO app_metadata (key, value) VALUES ('sync_device_id', ?1) \
                         ON CONFLICT(key) DO UPDATE SET value = ?1",
                        params![id],
                    )
                    .map_err(erro_sql)?;
                transacao.commit().map_err(erro_sql)?;
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

#[cfg(test)]
mod tests {
    use mos_sync::DeviceRepository;

    use crate::SqliteStorage;

    fn storage() -> (SqliteStorage, tempfile::TempDir) {
        let pasta = tempfile::tempdir().unwrap();
        let storage =
            SqliteStorage::open(pasta.path().join("mos.db"), pasta.path().join("backups")).unwrap();
        (storage, pasta)
    }

    /// O caso que aconteceu de verdade, em 02/09/2026.
    ///
    /// O PC do trabalho apareceu no hub com uma identidade NOVA, e com ela um
    /// relogio novo e um cursor zerado — comecou a baixar tudo de novo. A linha
    /// de `devices` pode sumir; o id nao pode.
    #[test]
    fn um_dispositivo_que_perdeu_a_linha_volta_com_o_mesmo_id() {
        let (storage, _guarda) = storage();
        let antes = storage.este_dispositivo("PC", "windows", "0.3.5").unwrap();

        // O sumico, direto no banco: e o que um banco recriado ou uma limpeza
        // por fora produzem.
        storage
            .escrita()
            .unwrap()
            .execute("DELETE FROM devices WHERE is_this_device = 1", [])
            .unwrap();

        let depois = storage.este_dispositivo("PC", "windows", "0.3.5").unwrap();
        assert_eq!(
            antes.id, depois.id,
            "o dispositivo nasceu de novo: o cursor e o relogio iriam junto"
        );
    }

    /// O banco que JA existia tambem ganha a ancora.
    ///
    /// O primeiro conserto so gravava a ancora quando a linha faltava — ou
    /// seja, so em instalacao nova. As maquinas que ja existem sao exatamente
    /// as que podem perder a linha, e elas ficariam sem protecao nenhuma. Este
    /// teste simula isso: linha presente, ancora ausente, e a abertura seguinte
    /// precisa criar a ancora antes que faca falta.
    #[test]
    fn um_banco_antigo_ganha_a_ancora_na_primeira_abertura() {
        let (storage, _guarda) = storage();
        let antes = storage.este_dispositivo("PC", "windows", "0.3.6").unwrap();

        // O estado de quem instalou o M/OS antes desta versao.
        storage
            .escrita()
            .unwrap()
            .execute("DELETE FROM app_metadata WHERE key = 'sync_device_id'", [])
            .unwrap();

        storage.este_dispositivo("PC", "windows", "0.3.6").unwrap();

        let ancora: String = storage
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT value FROM app_metadata WHERE key = 'sync_device_id'",
                [],
                |linha| linha.get(0),
            )
            .expect("o banco antigo continuou sem ancora");
        assert_eq!(ancora, antes.id.to_string());
    }

    /// A ancora e a linha nascem juntas, ou nenhuma das duas.
    ///
    /// Ancora sem linha faria a proxima abertura ressuscitar um id que nunca
    /// existiu no hub.
    #[test]
    fn a_ancora_guarda_o_id_do_primeiro_registro() {
        let (storage, _guarda) = storage();
        let device = storage.este_dispositivo("PC", "windows", "0.3.5").unwrap();

        let ancora: String = storage
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT value FROM app_metadata WHERE key = 'sync_device_id'",
                [],
                |linha| linha.get(0),
            )
            .expect("a ancora nao foi gravada");
        assert_eq!(ancora, device.id.to_string());
    }
}
