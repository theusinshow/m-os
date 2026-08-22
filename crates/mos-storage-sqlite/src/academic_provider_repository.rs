//! A aplicacao de um retrato de provedor externo sobre o M/Academic.
//!
//! # A transacao que este arquivo existe para garantir
//!
//! **Uma sincronizacao e um commit so.** Semestres, disciplinas, avaliacoes,
//! trabalhos, materiais, referencias externas e o estado da conexao entram
//! juntos ou nao entram. Em sequencia, uma queda no meio deixaria uma `Exam`
//! gravada sem a referencia que a liga ao `idAvaliacao` — e a proxima
//! sincronizacao, sem achar a referencia, criaria a segunda. A duplicata que o
//! §25 do pedido proibe nasceria de uma falha de rede, e nao de um bug de
//! logica.
//!
//! # O que o provedor pode escrever, e o que ele nunca toca
//!
//! O portal e dono do FATO ACADEMICO: titulo, prazo, peso, teto, nota, estado.
//! A pessoa e dona da ORGANIZACAO: prioridade, Task vinculada, accent, notas
//! pessoais, local da prova. Uma sincronizacao que mudasse prioridade
//! destruiria a decisao de quem estuda toda vez que a UNINTER republicasse a
//! mesma prova — e e o §32 do pedido, alem de ser a fronteira que o ADR-058 ja
//! desenha entre "o que existe" e "o que isso significa para mim".
//!
//! Por isso os UPDATE daqui listam colunas explicitamente. Nenhum deles diz
//! `SET ... priority = ?`, e nenhum deles toca `task_id`, `accent`, `notes` ou
//! `location`.

use std::collections::HashMap;

use mos_core::academic_sync::{
    ExternalAssessmentStatus, ExternalAssignmentStatus, ExternalEntity, ExternalKind, ExternalRef,
    Missing, ProviderConnection, ProviderSnapshot, ProviderStatus, SyncAction, SyncCounts,
    SyncOutcome, SyncReport,
};
use mos_core::{
    CoreError, ErrorCode, NewResource, ResourceKind, SubjectId,
};
use rusqlite::{params, Connection, OptionalExtension};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    map_lock_error, map_sql_error,
    repository::{format_time, parse_time},
    SqliteStorage,
};

const KIND_SEMESTER: &str = "academic_semester";
const KIND_SUBJECT: &str = "academic_subject";
const KIND_ASSIGNMENT: &str = "academic_assignment";
const KIND_EXAM: &str = "academic_exam";
const REL_MATERIAL: &str = "academic_subject_resource";

/// Quem tem acesso ao banco pelo lado do provedor externo.
///
/// Porta separada da `AcademicRepository` de proposito: aquela e a persistencia
/// que a PESSOA opera pela tela, e esta e a que um AVA opera pelo sync. Junta-las
/// deixaria `apply_provider_snapshot` a um `impl` de distancia de qualquer codigo
/// de UI, e o §3 do pedido pede exatamente o contrario.
pub trait AcademicProviderRepository: Send + Sync {
    fn provider_status(&self, provider: &str) -> Result<ProviderStatus, CoreError>;
    fn set_provider_connection(
        &self,
        provider: &str,
        connection: ProviderConnection,
    ) -> Result<(), CoreError>;
    /// Aplica o retrato inteiro num commit. Devolve o que mudou.
    fn apply_provider_snapshot(
        &self,
        snapshot: &ProviderSnapshot,
        started_at: OffsetDateTime,
    ) -> Result<SyncReport, CoreError>;
    /// A media oficial e a situacao que a instituicao publica, por disciplina.
    /// **Nunca** substitui `mos_core::academic::desempenho`.
    fn provider_subject_facts(
        &self,
        provider: &str,
    ) -> Result<Vec<ProviderSubjectFact>, CoreError>;
    /// O ultimo endereco visto de um material. Pode estar vencido: e cache.
    fn material_url(&self, provider: &str, external_id: &str)
        -> Result<Option<String>, CoreError>;
    /// Desconecta: apaga o estado e as referencias, **preservando** as entidades
    /// academicas ja criadas. Quem desconecta nao pede para perder o semestre.
    fn forget_provider(&self, provider: &str) -> Result<(), CoreError>;
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSubjectFact {
    pub subject_id: SubjectId,
    pub situation: String,
    pub official_grade: Option<f64>,
}

// ===========================================================================
// Leitura das referencias
// ===========================================================================

fn ler_refs(
    connection: &Connection,
    provider: &str,
    kind: ExternalKind,
) -> Result<Vec<ExternalRef>, CoreError> {
    let mut stmt = connection
        .prepare(
            "SELECT external_id, local_id, payload_hash, unavailable_since,
                    first_synced_at, last_synced_at
               FROM academic_external_refs
              WHERE provider = ?1 AND kind = ?2",
        )
        .map_err(map_sql_error)?;
    let linhas = stmt
        .query_map(params![provider, kind.as_str()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(map_sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_sql_error)?;

    linhas
        .into_iter()
        .map(|(external_id, local_id, payload_hash, indisponivel, primeiro, ultimo)| {
            Ok(ExternalRef {
                provider: provider.to_owned(),
                kind,
                external_id,
                local_id,
                payload_hash,
                unavailable_since: indisponivel.as_deref().map(parse_time).transpose()?,
                first_synced_at: parse_time(&primeiro)?,
                last_synced_at: parse_time(&ultimo)?,
            })
        })
        .collect()
}

/// A que disciplina externa pertence cada referencia de avaliacao, trabalho ou
/// material.
///
/// Existe por causa do recorte: uma sincronizacao do semestre corrente so
/// pergunta ao portal sobre as disciplinas daquele semestre, e o que nao foi
/// perguntado nao esta ausente. Sem este mapa, `reconcile` marcaria como
/// `unavailable` toda avaliacao de semestre passado, e a rodada seguinte as
/// ressuscitaria — dois eventos falsos por sincronizacao, para sempre.
fn dono_por_referencia(
    connection: &Connection,
    provider: &str,
    kind: ExternalKind,
) -> Result<HashMap<String, String>, CoreError> {
    let sql = match kind {
        ExternalKind::Exam => {
            "SELECT r.external_id, sr.external_id
               FROM academic_external_refs r
               JOIN academic_exams e ON e.id = r.local_id
               JOIN academic_external_refs sr
                 ON sr.local_id = e.subject_id AND sr.kind = 'subject' AND sr.provider = r.provider
              WHERE r.provider = ?1 AND r.kind = 'exam'"
        }
        ExternalKind::Assignment => {
            "SELECT r.external_id, sr.external_id
               FROM academic_external_refs r
               JOIN academic_assignments a ON a.id = r.local_id
               JOIN academic_external_refs sr
                 ON sr.local_id = a.subject_id AND sr.kind = 'subject' AND sr.provider = r.provider
              WHERE r.provider = ?1 AND r.kind = 'assignment'"
        }
        ExternalKind::Material => {
            "SELECT r.external_id, sr.external_id
               FROM academic_external_refs r
               JOIN academic_subject_resources sr_link ON sr_link.resource_id = r.local_id
               JOIN academic_external_refs sr
                 ON sr.local_id = sr_link.subject_id AND sr.kind = 'subject' AND sr.provider = r.provider
              WHERE r.provider = ?1 AND r.kind = 'material'"
        }
        _ => return Ok(HashMap::new()),
    };
    let mut stmt = connection.prepare(sql).map_err(map_sql_error)?;
    let pares = stmt
        .query_map(params![provider], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(map_sql_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(map_sql_error)?;
    Ok(pares.into_iter().collect())
}

// ===========================================================================
// Escrita das referencias
// ===========================================================================

#[allow(clippy::too_many_arguments)]
fn gravar_ref(
    transaction: &Connection,
    provider: &str,
    kind: ExternalKind,
    external_id: &str,
    local_id: &str,
    hash: &str,
    agora: &str,
) -> Result<(), CoreError> {
    transaction
        .execute(
            "INSERT INTO academic_external_refs
                 (provider, kind, external_id, local_id, payload_hash,
                  unavailable_since, first_synced_at, last_synced_at)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?6)
             ON CONFLICT(provider, kind, external_id) DO UPDATE SET
                 local_id = excluded.local_id,
                 payload_hash = excluded.payload_hash,
                 -- Reaparecer limpa a marca de ausencia. Sem isto, um item que
                 -- volta ficaria marcado como sumido para sempre.
                 unavailable_since = NULL,
                 last_synced_at = excluded.last_synced_at",
            params![provider, kind.as_str(), external_id, local_id, hash, agora],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

fn tocar_ref(
    transaction: &Connection,
    provider: &str,
    kind: ExternalKind,
    external_id: &str,
    agora: &str,
) -> Result<(), CoreError> {
    transaction
        .execute(
            "UPDATE academic_external_refs
                SET last_synced_at = ?4, unavailable_since = NULL
              WHERE provider = ?1 AND kind = ?2 AND external_id = ?3",
            params![provider, kind.as_str(), external_id, agora],
        )
        .map_err(map_sql_error)?;
    Ok(())
}

/// Marca o que sumiu — e **nunca** apaga.
fn marcar_ausente(
    transaction: &Connection,
    provider: &str,
    faltantes: &[Missing],
    agora: &str,
) -> Result<usize, CoreError> {
    let mut n = 0;
    for item in faltantes {
        n += transaction
            .execute(
                "UPDATE academic_external_refs
                    SET unavailable_since = ?4, last_synced_at = ?4
                  WHERE provider = ?1 AND kind = ?2 AND external_id = ?3
                    AND unavailable_since IS NULL",
                params![provider, item.kind.as_str(), item.external_id, agora],
            )
            .map_err(map_sql_error)?;
    }
    Ok(n)
}

// ===========================================================================
// A aplicacao
// ===========================================================================

impl AcademicProviderRepository for SqliteStorage {
    fn provider_status(&self, provider: &str) -> Result<ProviderStatus, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let linha = connection
            .query_row(
                "SELECT connection, course_name, last_sync_at, last_outcome
                   FROM academic_provider_state WHERE provider = ?1",
                params![provider],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(map_sql_error)?;

        let mut tracked = std::collections::BTreeMap::new();
        {
            let mut stmt = connection
                .prepare(
                    "SELECT kind, COUNT(*) FROM academic_external_refs
                      WHERE provider = ?1 AND unavailable_since IS NULL
                      GROUP BY kind",
                )
                .map_err(map_sql_error)?;
            let linhas = stmt
                .query_map(params![provider], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })
                .map_err(map_sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(map_sql_error)?;
            for (kind, total) in linhas {
                tracked.insert(kind, total.max(0) as usize);
            }
        }

        let (connection_state, course_name, last_sync_at, last_outcome) = match linha {
            None => (ProviderConnection::Disconnected, String::new(), None, None),
            Some((estado, curso, quando, resultado)) => (
                match estado.as_str() {
                    "connected" => ProviderConnection::Connected,
                    "expired" => ProviderConnection::Expired,
                    _ => ProviderConnection::Disconnected,
                },
                curso,
                quando.as_deref().map(parse_time).transpose()?,
                resultado.as_deref().and_then(|r| match r {
                    "completed" => Some(SyncOutcome::Completed),
                    "completed_with_warnings" => Some(SyncOutcome::CompletedWithWarnings),
                    "requires_authentication" => Some(SyncOutcome::RequiresAuthentication),
                    "failed" => Some(SyncOutcome::Failed),
                    _ => None,
                }),
            ),
        };

        Ok(ProviderStatus {
            provider: provider.to_owned(),
            connection: connection_state,
            last_sync_at,
            last_outcome,
            course_name,
            tracked,
        })
    }

    fn set_provider_connection(
        &self,
        provider: &str,
        connection_state: ProviderConnection,
    ) -> Result<(), CoreError> {
        let agora = format_time(OffsetDateTime::now_utc())?;
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .execute(
                "INSERT INTO academic_provider_state
                     (provider, connection, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?3)
                 ON CONFLICT(provider) DO UPDATE SET
                     connection = excluded.connection,
                     updated_at = excluded.updated_at",
                params![provider, estado_str(connection_state), agora],
            )
            .map_err(map_sql_error)?;
        Ok(())
    }

    fn apply_provider_snapshot(
        &self,
        snapshot: &ProviderSnapshot,
        started_at: OffsetDateTime,
    ) -> Result<SyncReport, CoreError> {
        let provider = snapshot.provider.as_str();
        if provider.trim().is_empty() {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "Retrato sem provedor.",
                false,
            ));
        }
        let agora_dt = OffsetDateTime::now_utc();
        let agora = format_time(agora_dt)?;

        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;

        // --- Semestres -----------------------------------------------------
        let refs_semestre = ler_refs(&transaction, provider, ExternalKind::Semester)?;
        let plano_semestre = mos_core::academic_sync::reconcile(
            ExternalKind::Semester,
            &snapshot.semesters,
            &refs_semestre,
        );
        let mut semestre_local: HashMap<String, String> = refs_semestre
            .iter()
            .map(|r| (r.external_id.clone(), r.local_id.clone()))
            .collect();
        let mut contagem_semestre = SyncCounts::default();

        for acao in &plano_semestre.actions {
            let item = acao.item();
            match acao {
                SyncAction::Create(_) => {
                    let id = Uuid::now_v7();
                    transaction
                        .execute(
                            "INSERT INTO academic_semesters
                                 (id, name, institution, starts_on, ends_on,
                                  lifecycle_state, created_at, updated_at)
                             VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6, ?6)",
                            params![
                                id.to_string(),
                                item.name,
                                item.institution,
                                item.starts_on.as_str(),
                                item.ends_on.as_str(),
                                agora
                            ],
                        )
                        .map_err(map_sql_error)?;
                    self.emitir_update(
                        &transaction,
                        KIND_SEMESTER,
                        id,
                        &[
                            ("name", serde_json::json!(item.name)),
                            ("institution", serde_json::json!(item.institution)),
                            ("startsOn", serde_json::json!(item.starts_on.as_str())),
                            ("endsOn", serde_json::json!(item.ends_on.as_str())),
                        ],
                    )?;
                    semestre_local.insert(item.external_id.clone(), id.to_string());
                    gravar_ref(
                        &transaction,
                        provider,
                        ExternalKind::Semester,
                        &item.external_id,
                        &id.to_string(),
                        &item.fingerprint(),
                        &agora,
                    )?;
                    contagem_semestre.created += 1;
                }
                SyncAction::Update { local_id, .. } => {
                    // O nome do semestre e do provedor. `lifecycle_state` NAO e:
                    // arquivar um semestre e decisao da pessoa, e o sync nao a
                    // desfaz.
                    transaction
                        .execute(
                            "UPDATE academic_semesters
                                SET name = ?2, institution = ?3, starts_on = ?4,
                                    ends_on = ?5, updated_at = ?6
                              WHERE id = ?1",
                            params![
                                local_id,
                                item.name,
                                item.institution,
                                item.starts_on.as_str(),
                                item.ends_on.as_str(),
                                agora
                            ],
                        )
                        .map_err(map_sql_error)?;
                    semestre_local.insert(item.external_id.clone(), local_id.clone());
                    gravar_ref(
                        &transaction,
                        provider,
                        ExternalKind::Semester,
                        &item.external_id,
                        local_id,
                        &item.fingerprint(),
                        &agora,
                    )?;
                    contagem_semestre.updated += 1;
                }
                SyncAction::Unchanged { local_id, .. } => {
                    semestre_local.insert(item.external_id.clone(), local_id.clone());
                    tocar_ref(
                        &transaction,
                        provider,
                        ExternalKind::Semester,
                        &item.external_id,
                        &agora,
                    )?;
                    contagem_semestre.unchanged += 1;
                }
            }
        }

        // --- Disciplinas ---------------------------------------------------
        let refs_disciplina = ler_refs(&transaction, provider, ExternalKind::Subject)?;
        let plano_disciplina = mos_core::academic_sync::reconcile(
            ExternalKind::Subject,
            &snapshot.subjects,
            &refs_disciplina,
        );
        let mut disciplina_local: HashMap<String, String> = refs_disciplina
            .iter()
            .map(|r| (r.external_id.clone(), r.local_id.clone()))
            .collect();
        let mut contagem_disciplina = SyncCounts::default();

        for acao in &plano_disciplina.actions {
            let item = acao.item();
            let Some(semestre_id) = semestre_local.get(&item.semester_external_id).cloned() else {
                // Disciplina de semestre que o retrato nao trouxe. Ignorar em
                // silencio seria pior que avisar: ela sumiria da tela sem
                // explicacao.
                continue;
            };
            let local_id = match acao {
                SyncAction::Create(_) => {
                    let id = Uuid::now_v7();
                    transaction
                        .execute(
                            "INSERT INTO academic_subjects
                                 (id, semester_id, name, code, teacher, accent, notes,
                                  lifecycle_state, created_at, updated_at)
                             VALUES (?1, ?2, ?3, ?4, ?5, '', '', 'active', ?6, ?6)",
                            params![
                                id.to_string(),
                                semestre_id,
                                item.name,
                                item.code,
                                item.teacher,
                                agora
                            ],
                        )
                        .map_err(map_sql_error)?;
                    self.emitir_update(
                        &transaction,
                        KIND_SUBJECT,
                        id,
                        &[
                            ("semesterId", serde_json::json!(semestre_id)),
                            ("name", serde_json::json!(item.name)),
                            ("code", serde_json::json!(item.code)),
                        ],
                    )?;
                    contagem_disciplina.created += 1;
                    id.to_string()
                }
                SyncAction::Update { local_id, .. } => {
                    // `accent` e `notes` NAO entram: sao a organizacao da pessoa.
                    // `teacher` so e escrito quando o provedor tem algo a dizer —
                    // o Univirtus manda vazio, e vazio nao apaga o que a pessoa
                    // escreveu.
                    transaction
                        .execute(
                            "UPDATE academic_subjects
                                SET semester_id = ?2, name = ?3, code = ?4,
                                    teacher = CASE WHEN ?5 = '' THEN teacher ELSE ?5 END,
                                    updated_at = ?6
                              WHERE id = ?1",
                            params![
                                local_id,
                                semestre_id,
                                item.name,
                                item.code,
                                item.teacher,
                                agora
                            ],
                        )
                        .map_err(map_sql_error)?;
                    contagem_disciplina.updated += 1;
                    local_id.clone()
                }
                SyncAction::Unchanged { local_id, .. } => {
                    contagem_disciplina.unchanged += 1;
                    local_id.clone()
                }
            };
            disciplina_local.insert(item.external_id.clone(), local_id.clone());
            gravar_ref(
                &transaction,
                provider,
                ExternalKind::Subject,
                &item.external_id,
                &local_id,
                &item.fingerprint(),
                &agora,
            )?;
            // A media oficial e a situacao, no lugar delas: fato do provedor,
            // ao lado da media propria e nunca no lugar dela.
            transaction
                .execute(
                    "INSERT INTO academic_provider_subject_facts
                         (provider, subject_id, situation, official_grade, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(provider, subject_id) DO UPDATE SET
                         situation = excluded.situation,
                         official_grade = excluded.official_grade,
                         updated_at = excluded.updated_at",
                    params![
                        provider,
                        local_id,
                        item.situation,
                        item.official_grade,
                        agora
                    ],
                )
                .map_err(map_sql_error)?;
        }

        // O recorte desta rodada: as disciplinas sobre as quais o provedor foi
        // efetivamente perguntado.
        let no_recorte: std::collections::HashSet<String> = snapshot
            .subjects
            .iter()
            .map(|s| s.external_id.clone())
            .collect();

        // --- Avaliacoes ----------------------------------------------------
        let refs_exame = ler_refs(&transaction, provider, ExternalKind::Exam)?;
        let dono_exame = dono_por_referencia(&transaction, provider, ExternalKind::Exam)?;
        let plano_exame = mos_core::academic_sync::reconcile_scoped(
            ExternalKind::Exam,
            &snapshot.assessments,
            &refs_exame,
            |r| {
                dono_exame
                    .get(&r.external_id)
                    .map(|dono| no_recorte.contains(dono))
                    .unwrap_or(false)
            },
        );
        let mut contagem_exame = SyncCounts::default();

        for acao in &plano_exame.actions {
            let item = acao.item();
            let Some(subject_id) = disciplina_local.get(&item.subject_external_id).cloned() else {
                continue;
            };
            let at = format_time(item.due_at)?;
            let status = match item.status {
                ExternalAssessmentStatus::Pending => "scheduled",
                ExternalAssessmentStatus::Done => "done",
                ExternalAssessmentStatus::Graded => "graded",
                ExternalAssessmentStatus::Cancelled => "cancelled",
            };
            let (score, max_score) = pontuacao(item.score, item.max_score);
            let local_id = match acao {
                SyncAction::Create(_) => {
                    let id = Uuid::now_v7();
                    transaction
                        .execute(
                            "INSERT INTO academic_exams
                                 (id, subject_id, name, at, location, topics, weight,
                                  max_score, score, status, lifecycle_state, created_at, updated_at)
                             VALUES (?1, ?2, ?3, ?4, '', ?5, ?6, ?7, ?8, ?9, 'active', ?10, ?10)",
                            params![
                                id.to_string(),
                                subject_id,
                                item.title,
                                at,
                                item.category,
                                item.weight,
                                max_score,
                                score,
                                status,
                                agora
                            ],
                        )
                        .map_err(map_sql_error)?;
                    self.emitir_update(
                        &transaction,
                        KIND_EXAM,
                        id,
                        &[
                            ("subjectId", serde_json::json!(subject_id)),
                            ("name", serde_json::json!(item.title)),
                            ("at", serde_json::json!(at)),
                            ("weight", serde_json::json!(item.weight)),
                            ("maxScore", serde_json::json!(max_score)),
                            ("score", serde_json::json!(score)),
                            ("status", serde_json::json!(status)),
                        ],
                    )?;
                    contagem_exame.created += 1;
                    id.to_string()
                }
                SyncAction::Update { local_id, .. } => {
                    // `location` fica de fora: e a pessoa que escreve onde a
                    // prova acontece, e o portal nao sabe.
                    transaction
                        .execute(
                            "UPDATE academic_exams
                                SET subject_id = ?2, name = ?3, at = ?4, topics = ?5,
                                    weight = ?6, max_score = ?7, score = ?8, status = ?9,
                                    updated_at = ?10
                              WHERE id = ?1",
                            params![
                                local_id,
                                subject_id,
                                item.title,
                                at,
                                item.category,
                                item.weight,
                                max_score,
                                score,
                                status,
                                agora
                            ],
                        )
                        .map_err(map_sql_error)?;
                    contagem_exame.updated += 1;
                    local_id.clone()
                }
                SyncAction::Unchanged { local_id, .. } => {
                    contagem_exame.unchanged += 1;
                    local_id.clone()
                }
            };
            gravar_ref(
                &transaction,
                provider,
                ExternalKind::Exam,
                &item.external_id,
                &local_id,
                &item.fingerprint(),
                &agora,
            )?;
        }
        contagem_exame.unavailable =
            marcar_ausente(&transaction, provider, &plano_exame.missing, &agora)?;

        // --- Trabalhos -----------------------------------------------------
        let refs_trabalho = ler_refs(&transaction, provider, ExternalKind::Assignment)?;
        let dono_trabalho = dono_por_referencia(&transaction, provider, ExternalKind::Assignment)?;
        let plano_trabalho = mos_core::academic_sync::reconcile_scoped(
            ExternalKind::Assignment,
            &snapshot.assignments,
            &refs_trabalho,
            |r| {
                dono_trabalho
                    .get(&r.external_id)
                    .map(|dono| no_recorte.contains(dono))
                    .unwrap_or(false)
            },
        );
        let mut contagem_trabalho = SyncCounts::default();

        for acao in &plano_trabalho.actions {
            let item = acao.item();
            let Some(subject_id) = disciplina_local.get(&item.subject_external_id).cloned() else {
                continue;
            };
            let due_at = item.due_at.map(format_time).transpose()?;
            let status = match item.status {
                ExternalAssignmentStatus::Pending => "pending",
                ExternalAssignmentStatus::Submitted => "submitted",
                ExternalAssignmentStatus::Graded => "graded",
                ExternalAssignmentStatus::Cancelled => "cancelled",
            };
            let (score, max_score) = pontuacao(item.score, item.max_score);
            let local_id = match acao {
                SyncAction::Create(_) => {
                    let id = Uuid::now_v7();
                    transaction
                        .execute(
                            "INSERT INTO academic_assignments
                                 (id, subject_id, title, description, due_at, status, priority,
                                  weight, max_score, score, task_id, lifecycle_state,
                                  created_at, updated_at)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'normal', ?7, ?8, ?9, NULL,
                                     'active', ?10, ?10)",
                            params![
                                id.to_string(),
                                subject_id,
                                item.title,
                                item.description,
                                due_at,
                                status,
                                item.weight,
                                max_score,
                                score,
                                agora
                            ],
                        )
                        .map_err(map_sql_error)?;
                    self.emitir_update(
                        &transaction,
                        KIND_ASSIGNMENT,
                        id,
                        &[
                            ("subjectId", serde_json::json!(subject_id)),
                            ("title", serde_json::json!(item.title)),
                            ("dueAt", serde_json::json!(due_at)),
                            ("status", serde_json::json!(status)),
                        ],
                    )?;
                    contagem_trabalho.created += 1;
                    id.to_string()
                }
                SyncAction::Update { local_id, .. } => {
                    // `priority` e `task_id` ficam de fora, e e a regra mais
                    // importante deste arquivo: a pessoa marcou a atividade como
                    // urgente e criou uma Task para ela. Uma republicacao do
                    // portal nao pode desfazer nenhuma das duas coisas.
                    transaction
                        .execute(
                            "UPDATE academic_assignments
                                SET subject_id = ?2, title = ?3, description = ?4, due_at = ?5,
                                    status = ?6, weight = ?7, max_score = ?8, score = ?9,
                                    updated_at = ?10
                              WHERE id = ?1",
                            params![
                                local_id,
                                subject_id,
                                item.title,
                                item.description,
                                due_at,
                                status,
                                item.weight,
                                max_score,
                                score,
                                agora
                            ],
                        )
                        .map_err(map_sql_error)?;
                    contagem_trabalho.updated += 1;
                    local_id.clone()
                }
                SyncAction::Unchanged { local_id, .. } => {
                    contagem_trabalho.unchanged += 1;
                    local_id.clone()
                }
            };
            gravar_ref(
                &transaction,
                provider,
                ExternalKind::Assignment,
                &item.external_id,
                &local_id,
                &item.fingerprint(),
                &agora,
            )?;
        }
        contagem_trabalho.unavailable =
            marcar_ausente(&transaction, provider, &plano_trabalho.missing, &agora)?;

        // --- Materiais -----------------------------------------------------
        let refs_material = ler_refs(&transaction, provider, ExternalKind::Material)?;
        let dono_material = dono_por_referencia(&transaction, provider, ExternalKind::Material)?;
        let plano_material = mos_core::academic_sync::reconcile_scoped(
            ExternalKind::Material,
            &snapshot.materials,
            &refs_material,
            |r| {
                dono_material
                    .get(&r.external_id)
                    .map(|dono| no_recorte.contains(dono))
                    .unwrap_or(false)
            },
        );
        let mut contagem_material = SyncCounts::default();

        for acao in &plano_material.actions {
            let item = acao.item();
            let Some(subject_id) = disciplina_local.get(&item.subject_external_id).cloned() else {
                continue;
            };
            let nota = if item.complementary {
                format!(
                    "Material complementar do Univirtus · {}",
                    item.extension.to_uppercase()
                )
            } else {
                format!("Material da disciplina no Univirtus · {}", item.extension.to_uppercase())
            };
            let local_id = match acao {
                SyncAction::Create(_) => {
                    // `Note`, e nao `Site`: o Resource NAO recebe a URL do
                    // provedor. Ela e assinada e vence em horas, e um Resource
                    // com URL morta e pior que um sem URL — ele promete que
                    // abre. O endereco corrente vive em `academic_material_urls`
                    // e se resolve na hora de abrir.
                    let novo = NewResource::create(
                        ResourceKind::Note,
                        &item.title,
                        "",
                        &nota,
                        None,
                    )?;
                    let id = novo.id;
                    transaction
                        .execute(
                            "INSERT INTO resources
                                 (id, kind, title, url, note, source_capture_id,
                                  lifecycle_state, created_at, updated_at)
                             VALUES (?1, 'note', ?2, '', ?3, NULL, 'active', ?4, ?4)",
                            params![id.to_string(), novo.title, novo.note, agora],
                        )
                        .map_err(map_sql_error)?;
                    contagem_material.created += 1;
                    id.to_string()
                }
                SyncAction::Update { local_id, .. } => {
                    transaction
                        .execute(
                            "UPDATE resources SET title = ?2, note = ?3, updated_at = ?4
                              WHERE id = ?1",
                            params![local_id, item.title, nota, agora],
                        )
                        .map_err(map_sql_error)?;
                    contagem_material.updated += 1;
                    local_id.clone()
                }
                SyncAction::Unchanged { local_id, .. } => {
                    contagem_material.unchanged += 1;
                    local_id.clone()
                }
            };
            // A juncao disciplina→material. `INSERT OR IGNORE` porque ela e o
            // conjunto, e ligar duas vezes tem de terminar ligado uma so.
            transaction
                .execute(
                    "INSERT OR IGNORE INTO academic_subject_resources
                         (subject_id, resource_id, created_at) VALUES (?1, ?2, ?3)",
                    params![subject_id, local_id, agora],
                )
                .map_err(map_sql_error)?;
            if let (Ok(de), Ok(para)) = (
                Uuid::parse_str(&subject_id),
                Uuid::parse_str(&local_id),
            ) {
                self.emitir_relacao(&transaction, REL_MATERIAL, de, para, true)?;
            }
            // O endereco corrente, como cache datado.
            if let Some(url) = &item.temporary_url {
                transaction
                    .execute(
                        "INSERT INTO academic_material_urls (provider, external_id, url, fetched_at)
                         VALUES (?1, ?2, ?3, ?4)
                         ON CONFLICT(provider, external_id) DO UPDATE SET
                             url = excluded.url, fetched_at = excluded.fetched_at",
                        params![provider, item.external_id, url, agora],
                    )
                    .map_err(map_sql_error)?;
            }
            gravar_ref(
                &transaction,
                provider,
                ExternalKind::Material,
                &item.external_id,
                &local_id,
                &item.fingerprint(),
                &agora,
            )?;
        }
        contagem_material.unavailable =
            marcar_ausente(&transaction, provider, &plano_material.missing, &agora)?;

        // --- O estado da conexao -------------------------------------------
        let outcome = if snapshot.is_partial() {
            SyncOutcome::CompletedWithWarnings
        } else {
            SyncOutcome::Completed
        };
        let relatorio = SyncReport {
            provider: provider.to_owned(),
            started_at,
            finished_at: agora_dt,
            outcome,
            semesters: contagem_semestre,
            subjects: contagem_disciplina,
            assessments: contagem_exame,
            assignments: contagem_trabalho,
            materials: contagem_material,
            warnings: snapshot.warnings.clone(),
        };
        let curso = snapshot
            .context
            .as_ref()
            .map(|c| c.course_name.clone())
            .unwrap_or_default();
        let curso_id = snapshot
            .context
            .as_ref()
            .map(|c| c.course_external_id.clone())
            .unwrap_or_default();
        transaction
            .execute(
                "INSERT INTO academic_provider_state
                     (provider, connection, course_name, course_external_id,
                      last_sync_at, last_outcome, last_report, created_at, updated_at)
                 VALUES (?1, 'connected', ?2, ?3, ?4, ?5, ?6, ?4, ?4)
                 ON CONFLICT(provider) DO UPDATE SET
                     connection = 'connected',
                     course_name = CASE WHEN ?2 = '' THEN course_name ELSE ?2 END,
                     course_external_id = CASE WHEN ?3 = '' THEN course_external_id ELSE ?3 END,
                     last_sync_at = excluded.last_sync_at,
                     last_outcome = excluded.last_outcome,
                     last_report = excluded.last_report,
                     updated_at = excluded.updated_at",
                params![
                    provider,
                    curso,
                    curso_id,
                    agora,
                    resultado_str(outcome),
                    serde_json::to_string(&relatorio).unwrap_or_default()
                ],
            )
            .map_err(map_sql_error)?;

        transaction.commit().map_err(map_sql_error)?;
        Ok(relatorio)
    }

    fn provider_subject_facts(
        &self,
        provider: &str,
    ) -> Result<Vec<ProviderSubjectFact>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let mut stmt = connection
            .prepare(
                "SELECT subject_id, situation, official_grade
                   FROM academic_provider_subject_facts WHERE provider = ?1",
            )
            .map_err(map_sql_error)?;
        let linhas = stmt
            .query_map(params![provider], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<f64>>(2)?,
                ))
            })
            .map_err(map_sql_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_sql_error)?;
        linhas
            .into_iter()
            .map(|(id, situation, official_grade)| {
                Ok(ProviderSubjectFact {
                    subject_id: SubjectId::parse(&id)?,
                    situation,
                    official_grade,
                })
            })
            .collect()
    }

    fn material_url(
        &self,
        provider: &str,
        external_id: &str,
    ) -> Result<Option<String>, CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        connection
            .query_row(
                "SELECT url FROM academic_material_urls WHERE provider = ?1 AND external_id = ?2",
                params![provider, external_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(map_sql_error)
    }

    fn forget_provider(&self, provider: &str) -> Result<(), CoreError> {
        let connection = self.connection.lock().map_err(map_lock_error)?;
        let transaction = connection.unchecked_transaction().map_err(map_sql_error)?;
        // As referencias e o cache saem; as entidades academicas FICAM.
        // Desconectar e dizer "pare de trazer", e nao "apague meu semestre".
        for sql in [
            "DELETE FROM academic_external_refs WHERE provider = ?1",
            "DELETE FROM academic_material_urls WHERE provider = ?1",
            "DELETE FROM academic_provider_subject_facts WHERE provider = ?1",
            "DELETE FROM academic_provider_state WHERE provider = ?1",
        ] {
            transaction
                .execute(sql, params![provider])
                .map_err(map_sql_error)?;
        }
        transaction.commit().map_err(map_sql_error)?;
        Ok(())
    }
}

/// O par nota/teto que o CHECK do banco aceita.
///
/// Nota sem teto nao se converte em media (8 de quanto?), e a 0031 recusa o par
/// pela metade. Quando o provedor manda nota sem teto, os dois caem juntos.
fn pontuacao(score: Option<f64>, max_score: Option<f64>) -> (Option<f64>, Option<f64>) {
    match (score, max_score) {
        (Some(s), Some(m)) if m > 0.0 && s >= 0.0 => (Some(s), Some(m)),
        (None, Some(m)) if m > 0.0 => (None, Some(m)),
        _ => (None, None),
    }
}

fn estado_str(estado: ProviderConnection) -> &'static str {
    match estado {
        ProviderConnection::Disconnected => "disconnected",
        ProviderConnection::Connected => "connected",
        ProviderConnection::Expired => "expired",
    }
}

fn resultado_str(resultado: SyncOutcome) -> &'static str {
    match resultado {
        SyncOutcome::Completed => "completed",
        SyncOutcome::CompletedWithWarnings => "completed_with_warnings",
        SyncOutcome::RequiresAuthentication => "requires_authentication",
        SyncOutcome::Failed => "failed",
    }
}
