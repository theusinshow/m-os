//! Os sinais de uso (ADR-082), lidos de tabelas que já existem.
//!
//! Só contagens, idades e desfechos: nenhuma coluna de conteúdo é lida. É
//! leitura — não passa pelo portão de escrita.

use mos_core::{
    mediana, CoreError, SinaisDaInbox, SinaisDeAdiamento, SinaisDePropostas, SinaisDeReunioes,
    SinaisDeSync, SinaisDeUso, SinaisDoDia,
};
use rusqlite::{params, Connection};
use time::OffsetDateTime;

use crate::{
    map_lock_error, map_sql_error,
    repository::{format_time, parse_time},
    SqliteStorage,
};

fn contar(
    connection: &Connection,
    sql: &str,
    args: impl rusqlite::Params,
) -> Result<usize, CoreError> {
    let n: i64 = connection
        .query_row(sql, args, |row| row.get(0))
        .map_err(map_sql_error)?;
    Ok(n.max(0) as usize)
}

fn instantes(
    connection: &Connection,
    sql: &str,
    args: impl rusqlite::Params,
) -> Result<Vec<String>, CoreError> {
    let mut statement = connection.prepare(sql).map_err(map_sql_error)?;
    let linhas = statement
        .query_map(args, |row| row.get::<_, String>(0))
        .map_err(map_sql_error)?;
    linhas.collect::<Result<Vec<_>, _>>().map_err(map_sql_error)
}

impl SqliteStorage {
    /// O retrato dos últimos `janela_dias` dias até `agora`.
    pub fn sinais_de_uso(
        &self,
        agora: OffsetDateTime,
        janela_dias: i64,
    ) -> Result<SinaisDeUso, CoreError> {
        let desde = format_time(agora - time::Duration::days(janela_dias))?;
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let c = &*connection;

        let horas = |texto: &str| parse_time(texto).ok().map(|t| (agora - t).whole_hours());

        // ---- Inbox
        let capturas = instantes(
            c,
            "SELECT captured_at FROM captures
              WHERE processing_state = 'inbox' AND lifecycle_state = 'active'",
            [],
        )?;
        let idades: Vec<i64> = capturas.iter().filter_map(|t| horas(t)).collect();
        let inbox = SinaisDaInbox {
            pendentes: capturas.len(),
            idade_mediana_horas: mediana(&idades),
            mais_de_7_dias: idades.iter().filter(|h| **h >= 24 * 7).count(),
        };

        // ---- Propostas do Hermes, pelo status gravado na parte
        let proposta = |status: &str| {
            contar(
                c,
                "SELECT COUNT(*) FROM message_parts p JOIN messages m ON m.id = p.message_id
                  WHERE p.kind = 'action_proposal' AND m.created_at >= ?1
                    AND json_extract(p.payload, '$.status') = ?2",
                params![desde, status],
            )
        };
        let propostas = SinaisDePropostas {
            executadas: proposta("executed")?,
            canceladas: proposta("cancelled")?,
            recusadas: proposta("refused")?,
            falharam: proposta("failed")?,
            incertas: proposta("executing")?,
        };

        // ---- Reuniões: o job e o que diz se andou sozinho
        let prontas_duracoes: Vec<i64> = {
            let mut statement = c
                .prepare(
                    "SELECT created_at, finished_at FROM meeting_jobs
                      WHERE status = 'done' AND finished_at IS NOT NULL AND finished_at >= ?1",
                )
                .map_err(map_sql_error)?;
            let linhas = statement
                .query_map([&desde], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(map_sql_error)?;
            let mut duracoes = Vec::new();
            for linha in linhas {
                let (inicio, fim) = linha.map_err(map_sql_error)?;
                if let (Ok(inicio), Ok(fim)) = (parse_time(&inicio), parse_time(&fim)) {
                    duracoes.push((fim - inicio).whole_minutes());
                }
            }
            duracoes
        };
        let reunioes = SinaisDeReunioes {
            prontas: prontas_duracoes.len(),
            pedem_atencao: contar(
                c,
                "SELECT COUNT(*) FROM meeting_jobs WHERE status = 'needs_attention'",
                [],
            )?,
            ate_pronta_mediana_minutos: mediana(&prontas_duracoes),
            tentativas_extras: contar(
                c,
                "SELECT COALESCE(SUM(MAX(attempt_count - 1, 0)), 0) FROM meeting_jobs
                  WHERE updated_at >= ?1",
                [&desde],
            )?,
        };

        // ---- Adiamentos
        let adiamentos = SinaisDeAdiamento {
            lembretes_adiados_na_janela: contar(
                c,
                "SELECT COUNT(*) FROM reminder_events WHERE kind = 'snoozed' AND at >= ?1",
                [&desde],
            )?,
            lembretes_adiados_3x: contar(
                c,
                "SELECT COUNT(*) FROM reminders
                  WHERE snooze_count >= 3
                    AND status IN ('scheduled', 'due', 'delivered', 'acknowledged', 'snoozed')
                    AND lifecycle_state = 'active'",
                [],
            )?,
            tasks_adiadas_3x: contar(
                c,
                "SELECT COUNT(*) FROM tasks
                  WHERE postponed_count >= 3 AND work_state <> 'done' AND lifecycle_state = 'active'",
                [],
            )?,
        };

        // ---- Sync
        let fila = instantes(
            c,
            "SELECT created_at FROM sync_outbox WHERE status <> 'acked' ORDER BY created_at",
            [],
        )?;
        let sync = SinaisDeSync {
            fila: fila.len(),
            mais_antiga_horas: fila.first().and_then(|t| horas(t)),
        };

        // ---- O dia
        let dia = SinaisDoDia {
            dias_iniciados: contar(
                c,
                "SELECT COUNT(*) FROM daily_sessions WHERE started_at >= ?1",
                [&desde],
            )?,
            dias_encerrados: contar(
                c,
                "SELECT COUNT(*) FROM daily_sessions WHERE status = 'completed' AND started_at >= ?1",
                [&desde],
            )?,
        };

        Ok(SinaisDeUso {
            janela_dias,
            inbox,
            propostas,
            reunioes,
            adiamentos,
            sync,
            dia,
        })
    }
}
