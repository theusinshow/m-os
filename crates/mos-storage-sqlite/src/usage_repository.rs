//! Persistencia do consumo de IA (migration 0036).
//!
//! O `mos-usage` le os transcripts e devolve fatos; aqui eles viram linhas, e a
//! duplicacao de 36% que os arquivos carregam morre na chave primaria.
//!
//! # Por que este arquivo NAO usa o portao de escrita
//!
//! O portao existe para quem EMITE operacao de sincronizacao — ele ordena
//! `portao` antes de `connection` para que uma rodada de sync e uma escrita
//! nunca se cruzem em ordens contrarias. Consumo de IA nao emite nada: e
//! observacao local de uma ferramenta de terceiro, nao dado do dominio que
//! outro aparelho precise ver.
//!
//! Sem emissao nao ha relogio, e sem relogio nao ha ciclo — o mesmo argumento
//! que o `lib.rs` ja usa para deixar a LEITURA fora do portao. E ha um motivo
//! pratico somado: a primeira carga varre 507 MB, e segurar o portao por ela
//! travaria o sync por minutos sem nenhum ganho.

use std::collections::HashMap;

use mos_core::CoreError;
use mos_usage::{inicio_da_janela, Evento, Ponteiro, JANELA};
use rusqlite::{params, Connection, OptionalExtension};
use time::{OffsetDateTime, UtcOffset};

use crate::{map_lock_error, map_sql_error, repository::format_time, SqliteStorage};

/// A janela de 5h corrente, quando ela existe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sessao {
    pub inicio: OffsetDateTime,
    pub fim: OffsetDateTime,
    pub peso: u64,
    pub requisicoes: u64,
}

/// Tudo o que a faixa precisa para desenhar um anel.
///
/// Note o que NAO esta aqui: porcentagem. Ela e calculada por quem desenha, a
/// partir do peso e do pico — e se o pico ainda nao existe, nao ha porcentagem
/// para calcular. Devolver um `0.73` daqui esconderia essa diferenca.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LeituraDeUso {
    pub sessao: Option<Sessao>,
    /// A maior janela de 5h ja observada. O denominador do anel.
    pub pico_sessao: u64,
    pub peso_hoje: u64,
    pub requisicoes_hoje: u64,
    /// O maior dia civil ja observado.
    pub pico_dia: u64,
    /// Quantas janelas o banco conhece. Uma so significa que o pico e a propria
    /// sessao corrente, e ai o anel marcaria 100% por falta de comparacao.
    pub janelas_conhecidas: u64,
}

impl SqliteStorage {
    /// Onde a leitura de cada arquivo parou, indexado por caminho.
    pub fn usage_ponteiros(&self) -> Result<HashMap<String, Ponteiro>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut statement = connection
            .prepare("SELECT caminho, offset, tamanho, mtime FROM usage_fonte")
            .map_err(map_sql_error)?;
        let linhas = statement
            .query_map([], |row| {
                Ok(Ponteiro {
                    caminho: row.get(0)?,
                    offset: row.get::<_, i64>(1)? as u64,
                    tamanho: row.get::<_, i64>(2)? as u64,
                    mtime: row.get(3)?,
                })
            })
            .map_err(map_sql_error)?;
        let mut ponteiros = HashMap::new();
        for linha in linhas {
            let ponteiro = linha.map_err(map_sql_error)?;
            ponteiros.insert(ponteiro.caminho.clone(), ponteiro);
        }
        Ok(ponteiros)
    }

    /// Grava o que a varredura encontrou. Devolve quantas requisicoes eram novas.
    ///
    /// Numa transacao so, e por um motivo que nao e desempenho: se os eventos
    /// entrassem e os ponteiros nao, a passada seguinte releria o mesmo trecho —
    /// inofensivo, porque o `requestId` recusa — mas se os PONTEIROS entrassem e
    /// os eventos nao, o trecho seria pulado para sempre e o consumo sumiria.
    /// A transacao torna as duas metades inseparaveis.
    pub fn usage_registrar(
        &self,
        eventos: &[Evento],
        ponteiros: &[Ponteiro],
    ) -> Result<u64, CoreError> {
        let mut connection = self.connection.lock().map_err(map_lock_error)?;
        let transacao = connection.transaction().map_err(map_sql_error)?;

        let mut novas = 0u64;
        let mut janelas_tocadas: Vec<String> = Vec::new();
        for evento in eventos {
            // SEMPRE em UTC. As colunas de instante sao TEXT, e a comparacao que
            // as consulta e de TEXTO: "2026-08-29T14:00:00-03:00" e
            // "2026-08-29T17:00:00Z" sao o mesmo momento e nao se parecem em
            // nada como string. Misturar os dois formatos na mesma coluna faz a
            // janela corrente sumir das buscas — foi exatamente o que aconteceu
            // na primeira vez que a faixa apareceu na tela, marcando 0%.
            let em = evento.em.to_offset(UtcOffset::UTC);
            let inicio = format_time(inicio_da_janela(em))?;
            let inseriu = transacao
                .execute(
                    "INSERT OR IGNORE INTO usage_requisicao (request_id, em, modelo, janela_inicio, peso)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        evento.request_id,
                        format_time(em)?,
                        evento.modelo,
                        inicio,
                        evento.peso() as i64
                    ],
                )
                .map_err(map_sql_error)?;
            if inseriu > 0 {
                novas += 1;
                if !janelas_tocadas.contains(&inicio) {
                    janelas_tocadas.push(inicio);
                }
            }
        }

        for ponteiro in ponteiros {
            transacao
                .execute(
                    "INSERT INTO usage_fonte (caminho, offset, tamanho, mtime)
                     VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(caminho) DO UPDATE SET
                       offset = excluded.offset,
                       tamanho = excluded.tamanho,
                       mtime = excluded.mtime",
                    params![
                        ponteiro.caminho,
                        ponteiro.offset as i64,
                        ponteiro.tamanho as i64,
                        ponteiro.mtime
                    ],
                )
                .map_err(map_sql_error)?;
        }

        for inicio in &janelas_tocadas {
            recalcular_janela(&transacao, inicio)?;
        }

        transacao.commit().map_err(map_sql_error)?;
        Ok(novas)
    }

    /// O que a faixa desenha.
    ///
    /// `agora` chega com o deslocamento LOCAL preso, e as DUAS metades dele sao
    /// usadas, cada uma no seu lugar:
    ///
    /// * o INSTANTE, normalizado para UTC, procura a janela de 5h — porque a
    ///   coluna e TEXT e a comparacao e de texto, e um "-03:00" nunca casaria
    ///   com um "Z";
    /// * o DESLOCAMENTO decide o que e "hoje" — num fuso de -3, cortar o dia em
    ///   UTC zeraria o anel as 21h, no meio da noite de trabalho.
    pub fn usage_leitura(&self, agora: OffsetDateTime) -> Result<LeituraDeUso, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;

        let agora_utc = agora.to_offset(UtcOffset::UTC);
        let corrente = format_time(inicio_da_janela(agora_utc))?;
        let sessao = connection
            .query_row(
                "SELECT inicio, fim, peso, requisicoes FROM usage_janela WHERE inicio = ?1",
                params![corrente],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(map_sql_error)?;

        // A janela corrente pode ter comecado ANTES da hora cheia de agora: um
        // bloco aberto as 03:00 ainda esta valendo as 07:30. Procurar so pela
        // hora cheia de agora perderia justamente a sessao em andamento.
        let sessao = match sessao {
            Some(linha) => Some(linha),
            None => connection
                .query_row(
                    "SELECT inicio, fim, peso, requisicoes FROM usage_janela
                     WHERE inicio <= ?1 AND fim > ?1
                     ORDER BY inicio DESC LIMIT 1",
                    params![format_time(agora_utc)?],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, i64>(3)?,
                        ))
                    },
                )
                .optional()
                .map_err(map_sql_error)?,
        };

        let sessao = sessao
            .map(
                |(inicio, fim, peso, requisicoes)| -> Result<Sessao, CoreError> {
                    Ok(Sessao {
                        inicio: crate::repository::parse_time(&inicio)?,
                        fim: crate::repository::parse_time(&fim)?,
                        peso: peso.max(0) as u64,
                        requisicoes: requisicoes.max(0) as u64,
                    })
                },
            )
            .transpose()?;

        let pico_sessao: i64 = connection
            .query_row(
                "SELECT COALESCE(MAX(peso), 0) FROM usage_janela",
                [],
                |row| row.get(0),
            )
            .map_err(map_sql_error)?;

        let janelas_conhecidas: i64 = connection
            .query_row("SELECT COUNT(*) FROM usage_janela", [], |row| row.get(0))
            .map_err(map_sql_error)?;

        // O deslocamento local, no formato que o `datetime()` do SQLite entende.
        // Passado como parametro e nao interpolado: o offset vem do sistema, e
        // sistema e entrada.
        let deslocamento = format!("{} seconds", agora.offset().whole_seconds());
        let dia_de_hoje = format!("{}", agora.date());

        let peso_hoje: i64 = connection
            .query_row(
                "SELECT COALESCE(SUM(peso), 0) FROM usage_requisicao
                 WHERE substr(datetime(em, ?1), 1, 10) = ?2",
                params![deslocamento, dia_de_hoje],
                |row| row.get(0),
            )
            .map_err(map_sql_error)?;

        let requisicoes_hoje: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM usage_requisicao
                 WHERE substr(datetime(em, ?1), 1, 10) = ?2",
                params![deslocamento, dia_de_hoje],
                |row| row.get(0),
            )
            .map_err(map_sql_error)?;

        let pico_dia: i64 = connection
            .query_row(
                "SELECT COALESCE(MAX(peso_dia), 0) FROM (
                   SELECT SUM(peso) AS peso_dia FROM usage_requisicao
                   GROUP BY substr(datetime(em, ?1), 1, 10)
                 )",
                params![deslocamento],
                |row| row.get(0),
            )
            .map_err(map_sql_error)?;

        Ok(LeituraDeUso {
            sessao,
            pico_sessao: pico_sessao.max(0) as u64,
            peso_hoje: peso_hoje.max(0) as u64,
            requisicoes_hoje: requisicoes_hoje.max(0) as u64,
            pico_dia: pico_dia.max(0) as u64,
            janelas_conhecidas: janelas_conhecidas.max(0) as u64,
        })
    }
}

/// Reescreve o agregado de uma janela A PARTIR das requisicoes.
///
/// Somar o delta sobre o valor anterior seria mais barato e estaria errado na
/// primeira vez que uma requisicao chegasse duas vezes por caminhos diferentes.
/// Recalcular da fonte torna o agregado uma funcao pura das linhas, e uma
/// varredura repetida nao muda nada.
fn recalcular_janela(connection: &Connection, inicio: &str) -> Result<(), CoreError> {
    let fim = crate::repository::parse_time(inicio)? + JANELA;
    connection
        .execute(
            "INSERT INTO usage_janela (inicio, fim, peso, requisicoes)
             SELECT ?1, ?2, COALESCE(SUM(peso), 0), COUNT(*)
               FROM usage_requisicao WHERE janela_inicio = ?1
             ON CONFLICT(inicio) DO UPDATE SET
               fim = excluded.fim,
               peso = excluded.peso,
               requisicoes = excluded.requisicoes",
            params![inicio, format_time(fim)?],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mos_usage::Evento;
    use time::macros::datetime;

    fn storage() -> (tempfile::TempDir, SqliteStorage) {
        let dir = tempfile::tempdir().unwrap();
        let storage =
            SqliteStorage::open(dir.path().join("m-os.db"), dir.path().join("backups")).unwrap();
        (dir, storage)
    }

    fn evento(request_id: &str, em: OffsetDateTime, output: u64) -> Evento {
        Evento {
            request_id: request_id.into(),
            em,
            modelo: "claude-opus-5".into(),
            input: 0,
            cache_creation: 0,
            cache_read: 0,
            output,
        }
    }

    #[test]
    fn o_mesmo_request_duas_vezes_conta_uma() {
        // O achado que decidiu o schema: 3277 linhas com `usage` para 2108
        // `requestId` unicos no maior transcript da maquina.
        let (_dir, storage) = storage();
        let em = datetime!(2026-08-29 03:13:00 UTC);
        let novas = storage
            .usage_registrar(&[evento("req_a", em, 100), evento("req_a", em, 100)], &[])
            .unwrap();
        assert_eq!(novas, 1);

        let leitura = storage
            .usage_leitura(datetime!(2026-08-29 04:00:00 UTC))
            .unwrap();
        let sessao = leitura.sessao.expect("a janela das 03:00 existe");
        assert_eq!(sessao.requisicoes, 1);
        assert_eq!(sessao.peso, 100 * 5_000);
    }

    #[test]
    fn varrer_duas_vezes_nao_muda_o_peso() {
        let (_dir, storage) = storage();
        let em = datetime!(2026-08-29 03:13:00 UTC);
        storage
            .usage_registrar(&[evento("req_a", em, 100)], &[])
            .unwrap();
        let primeira = storage
            .usage_leitura(datetime!(2026-08-29 04:00:00 UTC))
            .unwrap();

        let novas = storage
            .usage_registrar(&[evento("req_a", em, 100)], &[])
            .unwrap();
        assert_eq!(novas, 0);
        let segunda = storage
            .usage_leitura(datetime!(2026-08-29 04:00:00 UTC))
            .unwrap();
        assert_eq!(primeira, segunda);
    }

    #[test]
    fn a_sessao_em_andamento_e_encontrada_fora_da_hora_cheia() {
        // Um bloco aberto as 03:00 ainda vale as 07:30. Procurar so pela hora
        // cheia de agora perderia a sessao em andamento.
        let (_dir, storage) = storage();
        storage
            .usage_registrar(
                &[evento("req_a", datetime!(2026-08-29 03:13:00 UTC), 100)],
                &[],
            )
            .unwrap();
        let leitura = storage
            .usage_leitura(datetime!(2026-08-29 07:30:00 UTC))
            .unwrap();
        let sessao = leitura
            .sessao
            .expect("a janela das 03:00 ainda esta aberta");
        assert_eq!(sessao.inicio, datetime!(2026-08-29 03:00:00 UTC));
        assert_eq!(sessao.fim, datetime!(2026-08-29 08:00:00 UTC));
    }

    #[test]
    fn a_sessao_e_achada_com_agora_em_fuso_local() {
        // A REGRESSAO que a primeira faixa na tela mostrou: com `agora` em -3, o
        // texto "2026-08-29T04:00:00-03:00" era comparado contra o
        // "2026-08-29T03:00:00Z" gravado, e a janela corrente sumia. A faixa
        // desenhava 0% com o banco cheio.
        let (_dir, storage) = storage();
        let offset = time::UtcOffset::from_hms(-3, 0, 0).unwrap();
        storage
            .usage_registrar(
                &[evento("req_a", datetime!(2026-08-29 03:13:00 UTC), 100)],
                &[],
            )
            .unwrap();
        let agora = datetime!(2026-08-29 04:00:00 UTC).to_offset(offset);
        let leitura = storage.usage_leitura(agora).unwrap();
        let sessao = leitura
            .sessao
            .expect("a janela das 03:00Z esta aberta as 01:00-03:00");
        assert_eq!(sessao.peso, 100 * 5_000);
    }

    #[test]
    fn janela_vencida_nao_e_sessao() {
        let (_dir, storage) = storage();
        storage
            .usage_registrar(
                &[evento("req_a", datetime!(2026-08-29 03:13:00 UTC), 100)],
                &[],
            )
            .unwrap();
        let leitura = storage
            .usage_leitura(datetime!(2026-08-29 09:00:00 UTC))
            .unwrap();
        assert!(leitura.sessao.is_none(), "a janela fechou as 08:00");
        assert_eq!(leitura.pico_sessao, 100 * 5_000, "mas o pico continua la");
    }

    #[test]
    fn o_dia_e_civil_e_local_nao_utc() {
        // Com -3, o que aconteceu as 02:00Z do dia 30 e ainda dia 29 aqui. Um
        // corte em UTC zeraria o anel do dia as 21h.
        let (_dir, storage) = storage();
        let offset = time::UtcOffset::from_hms(-3, 0, 0).unwrap();
        storage
            .usage_registrar(
                &[
                    evento("req_tarde", datetime!(2026-08-29 20:00:00 UTC), 100),
                    evento("req_madrugada", datetime!(2026-08-30 02:00:00 UTC), 100),
                ],
                &[],
            )
            .unwrap();
        let agora = datetime!(2026-08-30 02:30:00 UTC).to_offset(offset);
        assert_eq!(
            agora.date().to_string(),
            "2026-08-29",
            "ainda e dia 29 aqui"
        );
        let leitura = storage.usage_leitura(agora).unwrap();
        assert_eq!(
            leitura.peso_hoje,
            200 * 5_000,
            "os dois eventos caem no mesmo dia civil local"
        );
    }

    #[test]
    fn o_pico_e_a_maior_janela_ja_vista() {
        let (_dir, storage) = storage();
        storage
            .usage_registrar(
                &[
                    evento("a", datetime!(2026-08-28 03:00:00 UTC), 1_000),
                    evento("b", datetime!(2026-08-29 03:00:00 UTC), 200),
                ],
                &[],
            )
            .unwrap();
        let leitura = storage
            .usage_leitura(datetime!(2026-08-29 04:00:00 UTC))
            .unwrap();
        assert_eq!(leitura.pico_sessao, 1_000 * 5_000);
        assert_eq!(leitura.sessao.unwrap().peso, 200 * 5_000);
        assert_eq!(leitura.janelas_conhecidas, 2);
    }

    #[test]
    fn banco_vazio_nao_tem_sessao_nem_pico() {
        let (_dir, storage) = storage();
        let leitura = storage
            .usage_leitura(datetime!(2026-08-29 04:00:00 UTC))
            .unwrap();
        assert_eq!(leitura, LeituraDeUso::default());
    }

    #[test]
    fn o_ponteiro_volta_como_foi_gravado() {
        let (_dir, storage) = storage();
        let ponteiro = Ponteiro {
            caminho: "C:/x/y.jsonl".into(),
            offset: 120,
            tamanho: 400,
            mtime: 1_756_000_000,
        };
        storage
            .usage_registrar(&[], std::slice::from_ref(&ponteiro))
            .unwrap();
        let lidos = storage.usage_ponteiros().unwrap();
        assert_eq!(lidos.get("C:/x/y.jsonl"), Some(&ponteiro));

        let avancado = Ponteiro {
            offset: 400,
            ..ponteiro.clone()
        };
        storage
            .usage_registrar(&[], std::slice::from_ref(&avancado))
            .unwrap();
        let lidos = storage.usage_ponteiros().unwrap();
        assert_eq!(lidos.len(), 1, "o mesmo caminho atualiza, nao duplica");
        assert_eq!(lidos["C:/x/y.jsonl"].offset, 400);
    }
}
