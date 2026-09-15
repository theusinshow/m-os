//! O que o piloto precisa guardar neste aparelho (migration 0041).
//!
//! Quatro coisas, todas LOCAIS: a saude da ultima rodada de sync, a ultima
//! presenca da pessoa, os avisos ja entregues (deduplicacao e snooze) e o
//! interruptor do Autopilot. Nenhuma viaja — o `sync_cobertura.rs` diz por
//! que, tabela por tabela.

use mos_core::{AvisoEntregue, CoreError, TipoDeAviso};
use mos_sync::{RegistroDeSaude, TipoDeFalha};
use rusqlite::{params, OptionalExtension};
use time::OffsetDateTime;

use crate::{map_lock_error, map_sql_error, repository::format_time, SqliteStorage};

const CHAVE_PRESENCA: &str = "piloto_ultima_presenca";
const CHAVE_AUTOPILOT: &str = "piloto_autopilot";

fn rfc(instante: OffsetDateTime) -> Result<String, CoreError> {
    format_time(instante)
}

fn parse_rfc(texto: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(texto, &time::format_description::well_known::Rfc3339).ok()
}

impl SqliteStorage {
    // ----------------------------------------------------------------- saude

    /// O registro da ultima rodada, como ficou entre execucoes.
    pub fn saude_do_sync(&self) -> Result<RegistroDeSaude, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .query_row(
                "SELECT ultimo_ok_em, ultima_rodada_em, ultimo_erro, tipo_do_erro, \
                 falhas_seguidas, proxima_tentativa_em FROM sync_saude WHERE only_row = 1",
                [],
                |row| {
                    Ok(RegistroDeSaude {
                        ultimo_ok_em: row.get(0)?,
                        ultima_rodada_em: row.get(1)?,
                        ultimo_erro: row.get(2)?,
                        tipo_do_erro: row
                            .get::<_, Option<String>>(3)?
                            .as_deref()
                            .and_then(TipoDeFalha::parse),
                        falhas_seguidas: row.get::<_, i64>(4)?.max(0) as u32,
                        proxima_tentativa_em: row.get(5)?,
                    })
                },
            )
            .optional()
            .map_err(map_sql_error)
            .map(|registro| registro.unwrap_or_default())
    }

    /// Grava como uma rodada terminou e devolve o registro novo.
    ///
    /// `erro = None` e sucesso: zera a escada e carimba `last_sync_at` no
    /// dispositivo — a coluna existia desde a 0027 e ninguem a escrevia.
    pub fn registrar_rodada_de_sync(
        &self,
        erro: Option<(&str, bool)>,
        agora: OffsetDateTime,
    ) -> Result<RegistroDeSaude, CoreError> {
        let mut registro = self.saude_do_sync()?;
        let agora_txt = rfc(agora)?;
        match erro {
            None => registro.sucesso(&agora_txt),
            Some((mensagem, retriavel)) => {
                registro.falha(&agora_txt, mensagem, retriavel, |espera| {
                    rfc(agora + espera).unwrap_or_default()
                });
            }
        }
        {
            let connection = self.connection.lock().map_err(map_lock_error)?;
            connection
                .execute(
                    "INSERT INTO sync_saude (only_row, ultimo_ok_em, ultima_rodada_em, ultimo_erro, \
                     tipo_do_erro, falhas_seguidas, proxima_tentativa_em, updated_at) \
                     VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7) \
                     ON CONFLICT(only_row) DO UPDATE SET ultimo_ok_em = ?1, ultima_rodada_em = ?2, \
                     ultimo_erro = ?3, tipo_do_erro = ?4, falhas_seguidas = ?5, \
                     proxima_tentativa_em = ?6, updated_at = ?7",
                    params![
                        registro.ultimo_ok_em,
                        registro.ultima_rodada_em,
                        registro.ultimo_erro,
                        registro.tipo_do_erro.map(|t| t.as_str()),
                        registro.falhas_seguidas as i64,
                        registro.proxima_tentativa_em,
                        agora_txt,
                    ],
                )
                .map_err(map_sql_error)?;
        }
        if erro.is_none() {
            use mos_sync::DeviceRepository;
            if let Ok(eu) = self.listar().and_then(|lista| {
                lista
                    .into_iter()
                    .find(|d| d.is_this_device)
                    .ok_or_else(|| mos_sync::SyncError::novo("sem identidade", false))
            }) {
                let _ = self.marcar_sync(eu.id, &agora_txt);
            }
        }
        Ok(registro)
    }

    /// Quantas operacoes da fila ja falharam ao menos uma vez.
    pub fn quantidade_em_retry(&self) -> Result<usize, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let total: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sync_outbox WHERE status = 'failed'",
                [],
                |row| row.get(0),
            )
            .map_err(map_sql_error)?;
        Ok(total as usize)
    }

    /// Quantos conflitos ninguem olhou.
    pub fn conflitos_abertos(&self) -> Result<usize, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let total: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sync_conflicts WHERE acknowledged_at = ''",
                [],
                |row| row.get(0),
            )
            .map_err(map_sql_error)?;
        Ok(total as usize)
    }

    /// Marca todos os conflitos abertos como vistos.
    pub fn reconhecer_conflitos(&self, agora: OffsetDateTime) -> Result<usize, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .execute(
                "UPDATE sync_conflicts SET acknowledged_at = ?1 WHERE acknowledged_at = ''",
                params![rfc(agora)?],
            )
            .map_err(map_sql_error)
    }

    // -------------------------------------------------------------- presenca

    /// A ultima presenca gravada ANTES desta, e grava a de agora.
    ///
    /// Uma operacao so, de proposito: o resgate precisa da anterior, e ler e
    /// depois gravar em dois passos deixaria uma janela em que a abertura de
    /// hoje ja se sobrescreveu.
    pub fn trocar_presenca(
        &self,
        agora: OffsetDateTime,
    ) -> Result<Option<OffsetDateTime>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let anterior: Option<String> = connection
            .query_row(
                "SELECT value FROM app_metadata WHERE key = ?1",
                params![CHAVE_PRESENCA],
                |row| row.get(0),
            )
            .optional()
            .map_err(map_sql_error)?;
        connection
            .execute(
                "INSERT INTO app_metadata (key, value) VALUES (?1, ?2) \
                 ON CONFLICT(key) DO UPDATE SET value = ?2",
                params![CHAVE_PRESENCA, rfc(agora)?],
            )
            .map_err(map_sql_error)?;
        Ok(anterior.as_deref().and_then(parse_rfc))
    }

    /// So le, sem gravar.
    pub fn ultima_presenca(&self) -> Result<Option<OffsetDateTime>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let valor: Option<String> = connection
            .query_row(
                "SELECT value FROM app_metadata WHERE key = ?1",
                params![CHAVE_PRESENCA],
                |row| row.get(0),
            )
            .optional()
            .map_err(map_sql_error)?;
        Ok(valor.as_deref().and_then(parse_rfc))
    }

    // ------------------------------------------------------------- autopilot

    /// Ligado por default: a feature nasce ligada em toda maquina.
    pub fn autopilot_ligado(&self) -> Result<bool, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let valor: Option<String> = connection
            .query_row(
                "SELECT value FROM app_metadata WHERE key = ?1",
                params![CHAVE_AUTOPILOT],
                |row| row.get(0),
            )
            .optional()
            .map_err(map_sql_error)?;
        Ok(valor.as_deref() != Some("off"))
    }

    pub fn set_autopilot(&self, ligado: bool) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .execute(
                "INSERT INTO app_metadata (key, value) VALUES (?1, ?2) \
                 ON CONFLICT(key) DO UPDATE SET value = ?2",
                params![CHAVE_AUTOPILOT, if ligado { "on" } else { "off" }],
            )
            .map_err(map_sql_error)?;
        Ok(())
    }

    // ---------------------------------------------------------------- avisos

    /// Os avisos entregues desde um instante — o historico que a politica le.
    pub fn avisos_desde(&self, desde: OffsetDateTime) -> Result<Vec<AvisoEntregue>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare(
                "SELECT chave, tipo, entregue_em, adiado_ate, resolvido_em FROM autopilot_avisos \
                 WHERE entregue_em >= ?1 OR (adiado_ate IS NOT NULL AND adiado_ate >= ?1) \
                 ORDER BY entregue_em DESC",
            )
            .map_err(map_sql_error)?;
        let linhas = statement
            .query_map(params![rfc(desde)?], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            })
            .map_err(map_sql_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(map_sql_error)?;
        Ok(linhas
            .into_iter()
            .filter_map(|(chave, tipo, entregue, adiado, resolvido)| {
                Some(AvisoEntregue {
                    chave,
                    tipo: tipo_de_aviso(&tipo)?,
                    entregue_em: parse_rfc(&entregue)?,
                    adiado_ate: adiado.as_deref().and_then(parse_rfc),
                    resolvido_em: resolvido.as_deref().and_then(parse_rfc),
                })
            })
            .collect())
    }

    /// Registra uma entrega. A mesma chave sobrescreve — e o snooze vencido
    /// que volta a tocar.
    pub fn registrar_aviso(
        &self,
        chave: &str,
        tipo: TipoDeAviso,
        alvo: (&str, &str),
        titulo: &str,
        corpo: &str,
        agora: OffsetDateTime,
    ) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .execute(
                "INSERT INTO autopilot_avisos (id, chave, tipo, entity_kind, entity_id, titulo, corpo, \
                 entregue_em, adiado_ate, resolvido_em) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, NULL) \
                 ON CONFLICT(chave) DO UPDATE SET entregue_em = ?8, adiado_ate = NULL, titulo = ?6, corpo = ?7",
                params![
                    uuid::Uuid::now_v7().to_string(),
                    chave,
                    tipo.as_str(),
                    alvo.0,
                    alvo.1,
                    titulo,
                    corpo,
                    rfc(agora)?,
                ],
            )
            .map_err(map_sql_error)?;
        Ok(())
    }

    pub fn adiar_aviso(&self, chave: &str, ate: OffsetDateTime) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .execute(
                "UPDATE autopilot_avisos SET adiado_ate = ?2 WHERE chave = ?1",
                params![chave, rfc(ate)?],
            )
            .map_err(map_sql_error)?;
        Ok(())
    }

    pub fn resolver_aviso(&self, chave: &str, agora: OffsetDateTime) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .execute(
                "UPDATE autopilot_avisos SET resolvido_em = ?2 WHERE chave = ?1",
                params![chave, rfc(agora)?],
            )
            .map_err(map_sql_error)?;
        Ok(())
    }

    /// Apaga o que tem mais de `dias`. O historico serve a deduplicacao de
    /// hoje, nao a arqueologia.
    pub fn podar_avisos(&self, antes_de: OffsetDateTime) -> Result<usize, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .execute(
                "DELETE FROM autopilot_avisos WHERE entregue_em < ?1 \
                 AND (adiado_ate IS NULL OR adiado_ate < ?1)",
                params![rfc(antes_de)?],
            )
            .map_err(map_sql_error)
    }

    // --------------------------------------------------------------- habitos

    /// Os minutos locais de inicio das ultimas sessoes, para a mediana.
    ///
    /// `started_at` e UTC; o deslocamento vem de quem chama, porque o banco
    /// nao sabe o fuso de quem estava na frente da tela.
    pub fn inicios_de_sessao(
        &self,
        quantas: usize,
        offset: time::UtcOffset,
    ) -> Result<Vec<u16>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare("SELECT started_at FROM daily_sessions ORDER BY day DESC LIMIT ?1")
            .map_err(map_sql_error)?;
        let linhas = statement
            .query_map(params![quantas as i64], |row| row.get::<_, String>(0))
            .map_err(map_sql_error)?
            .collect::<rusqlite::Result<Vec<String>>>()
            .map_err(map_sql_error)?;
        Ok(linhas
            .iter()
            .filter_map(|t| parse_rfc(t))
            .map(|t| {
                let l = t.to_offset(offset);
                (l.hour() as u16) * 60 + l.minute() as u16
            })
            .collect())
    }
}

fn tipo_de_aviso(texto: &str) -> Option<TipoDeAviso> {
    Some(match texto {
        "upcoming" => TipoDeAviso::Upcoming,
        "forgotten_task" => TipoDeAviso::ForgottenTask,
        "academic" => TipoDeAviso::Academic,
        "waiting_for" => TipoDeAviso::WaitingFor,
        "sync" => TipoDeAviso::Sync,
        "unfinished_day" => TipoDeAviso::UnfinishedDay,
        "day_not_started" => TipoDeAviso::DayNotStarted,
        "overdue" => TipoDeAviso::Overdue,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn banco() -> (SqliteStorage, tempfile::TempDir) {
        let pasta = tempfile::tempdir().unwrap();
        let storage =
            SqliteStorage::open(pasta.path().join("mos.db"), pasta.path().join("backups")).unwrap();
        (storage, pasta)
    }

    #[test]
    fn a_saude_sobe_a_escada_e_zera_no_sucesso() {
        let (s, _pasta) = banco();
        let t0 = datetime!(2026-09-15 10:00 UTC);
        let r = s
            .registrar_rodada_de_sync(Some(("Sem alcancar o hub: connection refused", true)), t0)
            .unwrap();
        assert_eq!(r.falhas_seguidas, 1);
        assert_eq!(r.tipo_do_erro, Some(TipoDeFalha::Offline));
        assert_eq!(
            r.proxima_tentativa_em.as_deref(),
            Some("2026-09-15T10:00:10Z")
        );
        let r = s
            .registrar_rodada_de_sync(
                Some(("Sem alcancar o hub", true)),
                t0 + time::Duration::seconds(10),
            )
            .unwrap();
        assert_eq!(r.falhas_seguidas, 2);
        assert_eq!(s.saude_do_sync().unwrap(), r, "sobrevive a releitura");
        let r = s
            .registrar_rodada_de_sync(None, t0 + time::Duration::minutes(1))
            .unwrap();
        assert_eq!(r.falhas_seguidas, 0);
        assert!(r.ultimo_erro.is_none());
        assert_eq!(r.ultimo_ok_em.as_deref(), Some("2026-09-15T10:01:00Z"));
    }

    #[test]
    fn presenca_devolve_a_anterior_e_grava_a_nova() {
        let (s, _pasta) = banco();
        let t0 = datetime!(2026-09-10 10:00 UTC);
        assert_eq!(s.trocar_presenca(t0).unwrap(), None);
        let t1 = datetime!(2026-09-15 10:00 UTC);
        assert_eq!(s.trocar_presenca(t1).unwrap(), Some(t0));
        assert_eq!(s.ultima_presenca().unwrap(), Some(t1));
    }

    #[test]
    fn avisos_deduplicam_adiam_e_resolvem() {
        let (s, _pasta) = banco();
        let t0 = datetime!(2026-09-15 10:00 UTC);
        s.registrar_aviso("k1", TipoDeAviso::Upcoming, ("", ""), "T", "C", t0)
            .unwrap();
        s.registrar_aviso(
            "k1",
            TipoDeAviso::Upcoming,
            ("", ""),
            "T",
            "C",
            t0 + time::Duration::minutes(5),
        )
        .unwrap();
        let lista = s.avisos_desde(t0 - time::Duration::hours(1)).unwrap();
        assert_eq!(lista.len(), 1);
        s.adiar_aviso("k1", t0 + time::Duration::hours(1)).unwrap();
        assert!(s.avisos_desde(t0).unwrap()[0].adiado_ate.is_some());
        s.resolver_aviso("k1", t0 + time::Duration::minutes(6))
            .unwrap();
        assert!(s.avisos_desde(t0).unwrap()[0].resolvido_em.is_some());
        assert_eq!(s.podar_avisos(t0 + time::Duration::days(2)).unwrap(), 1);
    }

    #[test]
    fn autopilot_nasce_ligado() {
        let (s, _pasta) = banco();
        assert!(s.autopilot_ligado().unwrap());
        s.set_autopilot(false).unwrap();
        assert!(!s.autopilot_ligado().unwrap());
    }
}
