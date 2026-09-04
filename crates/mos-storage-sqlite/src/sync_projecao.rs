//! A ponte entre operacao e dominio: a `Projecao` de verdade.
//!
//! O motor sabe reconciliar, mas nao sabe o que e uma Task. Este modulo e o
//! unico lugar do M/OS que sabe as duas coisas — e por isso ele e curto de
//! proposito.
//!
//! # As duas metades
//!
//! - `estado_de` devolve o que este dispositivo ja sabe da entidade, com o
//!   instante de cada campo. Sai da tabela sombra `sync_state`, e nao das
//!   tabelas de dominio, porque o dominio nao guarda instante por campo — ver o
//!   comentario da migration 0035.
//! - `guardar` grava o estado reconciliado e MATERIALIZA nas colunas do
//!   dominio, para as telas, a busca e os relatorios enxergarem.
//!
//! # Tipo desconhecido nao e erro
//!
//! Um cliente antigo pode receber uma operacao sobre um tipo que ele ainda nao
//! conhece (§27 e §74: versoes N e N-1 convivem). Ele guarda o estado e nao
//! materializa nada — e no dia em que atualizar, a materializacao acontece com
//! o dado que ja estava aqui. Descartar seria perder dado por ser velho.
//!
//! # Por que o `UPSERT` e nao um `UPDATE`
//!
//! A operacao que cria uma Task e um `Update` com titulo, descricao e project,
//! e ela pode chegar depois de um `Update` que so mexe no `workState`. Quem
//! recebe fora de ordem precisa terminar com a linha certa nas duas ordens, e
//! um `UPDATE` puro perderia a primeira metade.

use mos_core::CoreError;
use mos_sync::{EstadoDaEntidade, Op, Projecao, Resultado, SyncError};
use rusqlite::{params, Connection, OptionalExtension};
use time::OffsetDateTime;

use crate::{map_sql_error, SqliteStorage};

/// Como um tipo de entidade vira linha.
struct Mapa {
    tabela: &'static str,
    /// (campo emitido, coluna). O nome do campo e o que `emitir_update` usa; a
    /// coluna e a do banco.
    colunas: &'static [(&'static str, &'static str)],
    /// (coluna, literal SQL) para as colunas `NOT NULL` sem `DEFAULT`.
    ///
    /// Sao valores PROVISORIOS, e existem por causa da ordem de chegada: a
    /// operacao que traz o titulo pode chegar depois da que mexe no
    /// `workState`, e sem um valor aqui o `INSERT` bateria no `NOT NULL` e a
    /// entidade nunca apareceria. O `UPDATE` logo em seguida os substitui assim
    /// que o campo de verdade chega.
    obrigatorias: &'static [(&'static str, &'static str)],
    /// A coluna que identifica a linha.
    ///
    /// Quase sempre `id`, e por isso ela era fixa no codigo. Mas
    /// `project_tracking` e uma extensao 1:1 de `projects` e nao tem coluna
    /// `id` nenhuma — a chave dela e `project_id`. Assumir `id` gerava um SQL
    /// contra uma coluna inexistente, e o valor/hora do projeto e um dado que
    /// vira fatura.
    chave: &'static str,
    /// Quais colunas de carimbo a tabela tem.
    ///
    /// `message_parts` nao tem nenhuma e `messages` so tem `created_at`.
    /// Escrever `updated_at` nelas seria SQL contra coluna que nao existe; a
    /// alternativa — alterar a tabela por migracao — foi recusada porque a
    /// 0027 nao tocou em nenhuma tabela existente de proposito (SYNC.md §6).
    carimbos: Carimbos,
    /// O literal SQL da chave, quando a tabela tem UMA linha so.
    ///
    /// `tracking_settings` e `id INTEGER PRIMARY KEY` com a linha 1, sempre. O
    /// id da entidade e um UUID fixo — ele existe para os dois aparelhos falarem
    /// da mesma coisa, e nao para virar valor de coluna. Sem isto o `WHERE id =
    /// '<uuid>'` nao acharia linha nenhuma e a configuracao nunca chegaria.
    linha_unica: Option<&'static str>,
    /// O campo emitido de onde sai o VALOR da chave.
    ///
    /// Existe para chave composta. O id da entidade dessas linhas e derivado
    /// (`sync_emit::id_composto`) e nao aparece em coluna nenhuma — enfia-lo em
    /// `subject_id`, que referencia `academic_subjects`, seria uma chave
    /// estrangeira apontando para uma disciplina que nao existe.
    ///
    /// Com isto a chave real viaja como campo, e a projecao a usa para achar e
    /// gravar a linha.
    chave_do_campo: Option<&'static str>,
}

/// As colunas de carimbo que uma tabela sincronizavel tem.
#[derive(Clone, Copy, PartialEq)]
enum Carimbos {
    Ambos,
    SoCriacao,
    /// So `updated_at`. `academic_provider_subject_facts` guarda um fato do
    /// provedor: quando ele foi informado importa, quando a linha nasceu nao.
    SoAtualizacao,
    Nenhum,
}

impl Carimbos {
    fn tem_criacao(self) -> bool {
        matches!(self, Self::Ambos | Self::SoCriacao)
    }

    fn tem_atualizacao(self) -> bool {
        matches!(self, Self::Ambos | Self::SoAtualizacao)
    }
}

impl Mapa {
    /// O caso comum: chave `id`, os dois carimbos.
    ///
    /// Existe para uma entrada do mapa declarar so o que a distingue. Doze das
    /// treze entradas nao mencionam chave nem carimbo, e e assim que deve ser:
    /// o que aparece escrito e a excecao.
    const fn padrao() -> Self {
        Self {
            tabela: "",
            colunas: &[],
            obrigatorias: &[],
            chave: "id",
            carimbos: Carimbos::Ambos,
            linha_unica: None,
            chave_do_campo: None,
        }
    }
}

/// O id de entidade da linha unica de `tracking_settings`.
///
/// Constante e arbitrario, como o namespace das relacoes: o que importa e ser o
/// MESMO nos dois aparelhos e nunca mudar. Muda-lo faria a configuracao existente
/// virar orfa e uma nova nascer vazia.
pub(crate) const ID_TRACKING_SETTINGS: uuid::Uuid = uuid::Uuid::from_bytes([
    0x6d, 0x6f, 0x73, 0x74, 0x72, 0x61, 0x63, 0x6b, 0x73, 0x65, 0x74, 0x74, 0x69, 0x6e, 0x67, 0x73,
]);

/// Os tipos que este M/OS sabe materializar.
///
/// Cresce um por um, junto com a emissao. Um tipo fora desta lista continua
/// sincronizando — o estado e guardado e reenviado —, so nao aparece nas telas
/// ate alguem escrever a linha dele aqui.
fn mapa_de(kind: &str) -> Option<Mapa> {
    match kind {
        "task" => Some(Mapa {
            tabela: "tasks",
            colunas: &[
                ("title", "title"),
                ("description", "description"),
                ("projectId", "project_id"),
                ("sourceCaptureId", "source_capture_id"),
                ("workState", "work_state"),
                ("lifecycleState", "lifecycle_state"),
                ("createdAt", "created_at"),
            ],
            obrigatorias: &[("title", "'(sem titulo)'"), ("description", "''")],
            ..Mapa::padrao()
        }),
        "project" => Some(Mapa {
            tabela: "projects",
            colunas: &[
                ("name", "name"),
                ("description", "description"),
                ("repository", "repository"),
                ("lifecycleState", "lifecycle_state"),
                ("createdAt", "created_at"),
            ],
            obrigatorias: &[("name", "'(sem nome)'"), ("description", "''")],
            ..Mapa::padrao()
        }),
        "workspace" => Some(Mapa {
            tabela: "workspaces",
            colunas: &[
                ("name", "name"),
                ("description", "description"),
                ("lifecycleState", "lifecycle_state"),
                ("createdAt", "created_at"),
            ],
            // `name` tem `CHECK (length(trim(name)) > 0)`.
            obrigatorias: &[("name", "'(sincronizando)'"), ("description", "''")],
            ..Mapa::padrao()
        }),
        "capture" => Some(Mapa {
            tabela: "captures",
            colunas: &[
                ("content", "content"),
                ("source", "source_kind"),
                ("capturedAt", "captured_at"),
                ("processingState", "processing_state"),
                ("lifecycleState", "lifecycle_state"),
            ],
            // `content` tem `CHECK (length(trim(content)) > 0)`: o provisorio
            // precisa ser NAO-VAZIO, senao o `INSERT` e recusado e a Capture
            // desaparece em vez de aparecer incompleta.
            obrigatorias: &[
                ("content", "'(sincronizando)'"),
                ("source_kind", "'home'"),
                ("captured_at", "?2"),
            ],
            ..Mapa::padrao()
        }),
        "resource" => Some(Mapa {
            tabela: "resources",
            colunas: &[
                ("kind", "kind"),
                ("title", "title"),
                ("url", "url"),
                ("note", "note"),
                ("sourceCaptureId", "source_capture_id"),
                ("lifecycleState", "lifecycle_state"),
                ("createdAt", "created_at"),
            ],
            obrigatorias: &[("kind", "'link'"), ("title", "'(sem titulo)'")],
            ..Mapa::padrao()
        }),
        "reminder" => Some(Mapa {
            tabela: "reminders",
            colunas: &[
                ("title", "title"),
                ("body", "body"),
                ("targetType", "target_type"),
                ("targetId", "target_id"),
                ("triggerKind", "trigger_kind"),
                ("trigger", "trigger"),
                ("priority", "priority"),
                ("status", "status"),
                ("snoozeAllowed", "snooze_allowed"),
                ("privacy", "privacy"),
                ("nextDueAt", "next_due_at"),
                ("snoozeCount", "snooze_count"),
                ("completedAt", "completed_at"),
                ("lifecycleState", "lifecycle_state"),
            ],
            obrigatorias: &[
                ("title", "'(sincronizando)'"),
                ("trigger_kind", "'at'"),
                ("trigger", "'{}'"),
                ("priority", "'normal'"),
                ("status", "'scheduled'"),
                ("source", "'system'"),
            ],
            ..Mapa::padrao()
        }),
        "academic_semester" => Some(Mapa {
            tabela: "academic_semesters",
            colunas: &[
                ("name", "name"),
                ("institution", "institution"),
                ("startsOn", "starts_on"),
                ("endsOn", "ends_on"),
                ("lifecycleState", "lifecycle_state"),
                ("createdAt", "created_at"),
            ],
            obrigatorias: &[
                ("name", "'(sincronizando)'"),
                ("starts_on", "''"),
                ("ends_on", "''"),
            ],
            ..Mapa::padrao()
        }),
        // As quatro do diario JA emitiam antes desta spec — faltava so a linha
        // aqui. A operacao viajava e o outro lado guardava o estado sem nunca
        // materializar, que de fora e indistinguivel de nao sincronizar.
        "daily_session" => Some(Mapa {
            tabela: "daily_sessions",
            colunas: &[
                ("day", "day"),
                ("status", "status"),
                ("note", "note"),
                ("startedAt", "started_at"),
                ("endedAt", "ended_at"),
            ],
            obrigatorias: &[("day", "''"), ("status", "'active'"), ("started_at", "?2")],
            ..Mapa::padrao()
        }),
        "daily_objective" => Some(Mapa {
            tabela: "daily_objectives",
            colunas: &[
                ("sessionId", "session_id"),
                ("title", "title"),
                ("description", "description"),
                ("linkKind", "link_kind"),
                ("linkId", "link_id"),
                ("priority", "priority"),
                ("status", "status"),
                ("position", "position"),
                ("carriedFrom", "carried_from"),
                ("completedAt", "completed_at"),
            ],
            obrigatorias: &[
                ("session_id", "''"),
                ("title", "'(sincronizando)'"),
                ("priority", "'secondary'"),
                ("status", "'pending'"),
            ],
            ..Mapa::padrao()
        }),
        // A chave e `session_id`: a reflexao e uma-para-uma com o dia, e ja
        // viajava com o id da sessao como id de entidade.
        "daily_reflection" => Some(Mapa {
            tabela: "daily_reflections",
            chave: "session_id",
            colunas: &[("mood", "mood"), ("summary", "summary")],
            obrigatorias: &[],
            ..Mapa::padrao()
        }),
        "weekly_review" => Some(Mapa {
            tabela: "weekly_reviews",
            colunas: &[
                ("weekStart", "week_start"),
                ("summary", "summary"),
                ("closedAt", "closed_at"),
            ],
            obrigatorias: &[("week_start", "''"), ("closed_at", "?2")],
            ..Mapa::padrao()
        }),
        // `project_tracking.client_id` REFERENCIA esta tabela (migration 0013):
        // sincronizar a cobranca sem o cliente faz a linha ser recusada no
        // destino por chave estrangeira.
        // Linha unica, e METADE dela. Arredondamento e emissor sao seus;
        // ociosidade, monitoramento de processo e deteccao de reuniao descrevem
        // a maquina, e replicados fariam este PC vigiar o que o outro vigia.
        "tracking_settings" => Some(Mapa {
            tabela: "tracking_settings",
            linha_unica: Some("1"),
            colunas: &[
                ("roundingEnabled", "rounding_enabled"),
                ("roundingIntervalMinutes", "rounding_interval_minutes"),
                ("roundingMode", "rounding_mode"),
                ("defaultHourlyRateCents", "default_hourly_rate_cents"),
                ("issuerName", "issuer_name"),
                ("issuerDocument", "issuer_document"),
                ("issuerContact", "issuer_contact"),
            ],
            obrigatorias: &[],
            carimbos: Carimbos::Nenhum,
            ..Mapa::padrao()
        }),
        // Chave composta `(provider, subject_id)`: o id da entidade e DERIVADO
        // dela (`sync_emit::id_composto`), como a relacao ja fazia. `provider` e
        // `subjectId` viajam como campos porque a chave nao e uma coluna so.
        "academic_provider_subject_fact" => Some(Mapa {
            tabela: "academic_provider_subject_facts",
            chave: "subject_id",
            chave_do_campo: Some("subjectId"),
            carimbos: Carimbos::SoAtualizacao,
            colunas: &[
                ("provider", "provider"),
                ("subjectId", "subject_id"),
                ("situation", "situation"),
                ("officialGrade", "official_grade"),
            ],
            obrigatorias: &[("provider", "'univirtus'")],
            ..Mapa::padrao()
        }),
        "client" => Some(Mapa {
            tabela: "clients",
            colunas: &[
                ("name", "name"),
                ("companyName", "company_name"),
                ("email", "email"),
                ("phone", "phone"),
                ("notes", "notes"),
                ("archivedAt", "archived_at"),
                ("createdAt", "created_at"),
            ],
            obrigatorias: &[("name", "'(sincronizando)'")],
            ..Mapa::padrao()
        }),
        "conversation" => Some(Mapa {
            tabela: "conversations",
            colunas: &[
                ("title", "title"),
                ("hermesSessionId", "hermes_session_id"),
                ("lifecycleState", "lifecycle_state"),
                ("createdAt", "created_at"),
            ],
            obrigatorias: &[],
            ..Mapa::padrao()
        }),
        // `messages` so tem `created_at`: uma mensagem nao e editada, ela e
        // fechada. O que muda depois e o `status`, e quando mudou esta no HLC da
        // operacao — repetir isso numa coluna seria duas verdades sobre o mesmo
        // instante.
        "message" => Some(Mapa {
            tabela: "messages",
            carimbos: Carimbos::SoCriacao,
            colunas: &[
                ("conversationId", "conversation_id"),
                ("seq", "seq"),
                ("role", "role"),
                ("status", "status"),
            ],
            obrigatorias: &[
                ("conversation_id", "''"),
                ("seq", "0"),
                ("role", "'user'"),
                ("status", "'done'"),
            ],
            ..Mapa::padrao()
        }),
        // Sem carimbo NENHUM, e nao por descuido: a parte e o conteudo imutavel
        // de uma mensagem. Nao ter quando-mudou e a verdade sobre ela.
        "message_part" => Some(Mapa {
            tabela: "message_parts",
            carimbos: Carimbos::Nenhum,
            colunas: &[
                ("messageId", "message_id"),
                ("seq", "seq"),
                ("kind", "kind"),
                ("payload", "payload"),
                ("searchText", "search_text"),
            ],
            obrigatorias: &[
                ("message_id", "''"),
                ("seq", "0"),
                ("kind", "'text'"),
                ("payload", "'{}'"),
            ],
            ..Mapa::padrao()
        }),
        // Extensao 1:1 de `projects`: a chave e `project_id`, e nao existe
        // coluna `id`. O id da entidade E o id do projeto — os dois aparelhos
        // chegam ao mesmo sem combinar nada.
        "project_tracking" => Some(Mapa {
            tabela: "project_tracking",
            chave: "project_id",
            colunas: &[
                ("hourlyRateCents", "hourly_rate_cents"),
                ("code", "code"),
                ("color", "color"),
                ("trackingStatus", "tracking_status"),
                ("clientId", "client_id"),
                ("budgetMinutes", "budget_minutes"),
                ("paidAt", "paid_at"),
                ("createdAt", "created_at"),
            ],
            obrigatorias: &[],
            ..Mapa::padrao()
        }),
        "time_entry" => Some(Mapa {
            tabela: "time_entries",
            colunas: &[
                ("projectId", "project_id"),
                ("startedAt", "started_at"),
                ("endedAt", "ended_at"),
                ("durationSeconds", "duration_seconds"),
                ("idleSeconds", "idle_seconds"),
                ("description", "description"),
                ("activityType", "activity_type"),
                ("billable", "billable"),
                ("hourlyRateSnapshotCents", "hourly_rate_snapshot_cents"),
                ("source", "source"),
                ("deletedAt", "deleted_at"),
                ("createdAt", "created_at"),
            ],
            obrigatorias: &[("project_id", "''"), ("started_at", "''")],
            ..Mapa::padrao()
        }),
        "academic_subject" => Some(Mapa {
            tabela: "academic_subjects",
            colunas: &[
                ("semesterId", "semester_id"),
                ("name", "name"),
                ("code", "code"),
                ("teacher", "teacher"),
                ("accent", "accent"),
                ("notes", "notes"),
                ("lifecycleState", "lifecycle_state"),
                ("createdAt", "created_at"),
            ],
            obrigatorias: &[("semester_id", "''"), ("name", "'(sincronizando)'")],
            ..Mapa::padrao()
        }),
        "academic_assignment" => Some(Mapa {
            tabela: "academic_assignments",
            colunas: &[
                ("subjectId", "subject_id"),
                ("title", "title"),
                ("description", "description"),
                ("dueAt", "due_at"),
                ("status", "status"),
                ("priority", "priority"),
                ("weight", "weight"),
                ("maxScore", "max_score"),
                ("score", "score"),
                ("taskId", "task_id"),
                ("lifecycleState", "lifecycle_state"),
                ("createdAt", "created_at"),
            ],
            obrigatorias: &[("subject_id", "''"), ("title", "'(sincronizando)'")],
            ..Mapa::padrao()
        }),
        "academic_exam" => Some(Mapa {
            tabela: "academic_exams",
            colunas: &[
                ("subjectId", "subject_id"),
                ("name", "name"),
                ("at", "at"),
                ("location", "location"),
                ("topics", "topics"),
                ("weight", "weight"),
                ("maxScore", "max_score"),
                ("score", "score"),
                ("status", "status"),
                ("lifecycleState", "lifecycle_state"),
                ("createdAt", "created_at"),
            ],
            obrigatorias: &[
                ("subject_id", "''"),
                ("name", "'(sincronizando)'"),
                ("at", "''"),
            ],
            ..Mapa::padrao()
        }),
        "academic_study_session" => Some(Mapa {
            tabela: "academic_study_sessions",
            colunas: &[
                ("subjectId", "subject_id"),
                ("topic", "topic"),
                ("notes", "notes"),
                ("startedAt", "started_at"),
                ("endedAt", "ended_at"),
                ("seconds", "seconds"),
                ("createdAt", "created_at"),
            ],
            obrigatorias: &[("subject_id", "''"), ("started_at", "''")],
            ..Mapa::padrao()
        }),
        // `relation` fica de fora, e nao por esquecimento: uma relacao do
        // Knowledge Graph nao e linha de tabela propria — ela e uma aresta com
        // id DERIVADO do par (ver `mos_sync::Relacao`), e cada tipo de vinculo
        // mora numa tabela diferente. Materializa-la exige um caminho proprio,
        // e enfia-la neste mapa daria a impressao de resolvido.
        _ => None,
    }
}

/// A tabela de juncao de cada tipo de vinculo: (tabela, coluna de `from`,
/// coluna de `to`).
///
/// A ordem de `from` e `to` FAZ PARTE da identidade da relacao (ver
/// `mos_sync::Relacao::id`), entao mapear a ponta errada aqui nao daria erro —
/// daria um vinculo silenciosamente invertido.
fn juncao_de(kind: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match kind {
        "resourceProject" => Some(("resource_projects", "resource_id", "project_id")),
        "resourceWorkspace" => Some(("resource_workspaces", "resource_id", "workspace_id")),
        "projectWorkspace" => Some(("project_workspaces", "project_id", "workspace_id")),
        "academic_subject_resource" => {
            Some(("academic_subject_resources", "subject_id", "resource_id"))
        }
        // Vinculo de um tipo que este M/OS ainda nao conhece. Guardar e
        // reenviar, sem materializar — a mesma regra dos outros tipos.
        _ => None,
    }
}

/// Quantas passadas do motor uma rodada faz, no maximo.
///
/// Teto e nao promessa: com limite de 100 por passada, isto cobre dez mil
/// operacoes num clique — mais do que qualquer fila real deste M/OS. Ele existe
/// para o laco ter fim mesmo se o outro lado passar a responder algo que o
/// motor nao consegue drenar, e nesse caso a rodada DIZ que parou pelo teto em
/// vez de fingir que terminou.
const MAX_PASSADAS: usize = 100;

fn falha(causa: CoreError) -> SyncError {
    SyncError::novo(causa.message, causa.retryable)
}

impl SqliteStorage {
    /// O estado reconciliado guardado para esta entidade.
    pub(crate) fn estado_guardado(
        conexao: &Connection,
        kind: &str,
        id: uuid::Uuid,
    ) -> Result<EstadoDaEntidade, CoreError> {
        let json: Option<String> = conexao
            .query_row(
                "SELECT estado FROM sync_state WHERE entity_kind = ?1 AND entity_id = ?2",
                params![kind, id.to_string()],
                |linha| linha.get(0),
            )
            .ok();
        match json {
            Some(texto) => serde_json::from_str(&texto).map_err(|causa| {
                crate::sync_emit::erro_de_sync(format!("Estado de sync ilegivel: {causa}"))
            }),
            None => Ok(EstadoDaEntidade::default()),
        }
    }

    /// Grava o estado reconciliado. **Nao** materializa: quem materializa
    /// decide separado, porque a mudanca local ja escreveu a linha de dominio
    /// pelo caminho normal e reescrever seria desfazer o que ela acabou de
    /// fazer.
    pub(crate) fn guardar_estado(
        transacao: &Connection,
        kind: &str,
        id: uuid::Uuid,
        estado: &EstadoDaEntidade,
    ) -> Result<(), CoreError> {
        let json = serde_json::to_string(estado).map_err(|causa| {
            crate::sync_emit::erro_de_sync(format!("Estado de sync ilegivel: {causa}"))
        })?;
        let momento = crate::repository::format_time(OffsetDateTime::now_utc())?;
        transacao
            .execute(
                "INSERT INTO sync_state (entity_kind, entity_id, estado, updated_at) \
                 VALUES (?1, ?2, ?3, ?4) \
                 ON CONFLICT(entity_kind, entity_id) DO UPDATE SET estado = ?3, updated_at = ?4",
                params![kind, id.to_string(), json, momento],
            )
            .map_err(map_sql_error)?;
        Ok(())
    }

    /// Escreve o estado reconciliado nas colunas do dominio.
    fn materializar(
        transacao: &Connection,
        kind: &str,
        id: uuid::Uuid,
        estado: &EstadoDaEntidade,
    ) -> Result<(), CoreError> {
        if kind == "relation" {
            return Self::materializar_relacao(transacao, estado);
        }
        let Some(mapa) = mapa_de(kind) else {
            return Ok(());
        };
        let momento = crate::repository::format_time(OffsetDateTime::now_utc())?;

        // O apagamento vira `trashed`, e nao `DELETE`.
        //
        // Sao dois motivos independentes, e os dois ja eram regra do M/OS antes
        // da sincronizacao: um dispositivo offline precisa distinguir "apagado"
        // de "nunca chegou", e todo Undo daqui e restauracao de estado.
        if estado.deleted_at.is_some() {
            transacao
                .execute(
                    &format!(
                        "UPDATE {} SET lifecycle_state = 'trashed', updated_at = ?1 \
                         WHERE {} = ?2",
                        mapa.tabela, mapa.chave
                    ),
                    params![momento, id.to_string()],
                )
                .map_err(map_sql_error)?;
            return Ok(());
        }

        // O `INSERT` ja leva os valores REAIS que o estado tem.
        //
        // Antes ele gravava so os provisorios e deixava o `UPDATE` corrigir, e
        // isso quebrava de verdade: `academic_subjects.semester_id` e chave
        // ESTRANGEIRA, e um `''` provisorio nao aponta para semestre nenhum —
        // a linha era recusada antes de o `UPDATE` ter chance de acertar. O
        // provisorio existe para o campo que AINDA NAO CHEGOU, e nao para
        // substituir o que ja esta na mao.
        // O VALOR da chave: do campo quando ela e composta, senao o id.
        let valor_da_chave = match mapa.chave_do_campo {
            Some(campo) => match estado.campos.get(campo) {
                Some(resolvido) => match &resolvido.valor {
                    serde_json::Value::String(texto) => texto.clone(),
                    outro => outro.to_string(),
                },
                // O campo que carrega a chave ainda nao chegou. Sair sem
                // materializar deixa a entidade nos pendentes, e a retentativa
                // resolve quando ele vier.
                None => return Ok(()),
            },
            None => id.to_string(),
        };
        // Numa tabela de linha unica a chave e um literal, e nao o id: o
        // `?1` traria o UUID da entidade para uma coluna que guarda `1`.
        let chave_marcador = mapa.linha_unica.unwrap_or("?1");
        let mut nomes: Vec<&str> = vec![mapa.chave];
        let mut marcadores: Vec<String> = vec![chave_marcador.to_owned()];
        let mut valores: Vec<rusqlite::types::Value> = vec![
            rusqlite::types::Value::Text(valor_da_chave.clone()),
            rusqlite::types::Value::Text(momento.clone()),
        ];
        if mapa.carimbos.tem_criacao() {
            nomes.push("created_at");
            marcadores.push("?2".into());
        }
        if mapa.carimbos.tem_atualizacao() {
            nomes.push("updated_at");
            marcadores.push("?2".into());
        }
        for (campo, coluna) in mapa.colunas {
            // `created_at` ja entrou como o instante de agora; sobrescrever
            // aqui duplicaria a coluna no `INSERT`. O `UPDATE` abaixo poe o
            // original no lugar.
            if *coluna == "created_at" || *coluna == "updated_at" {
                continue;
            }
            let Some(resolvido) = estado.campos.get(*campo) else {
                continue;
            };
            nomes.push(coluna);
            valores.push(valor_sql(&resolvido.valor));
            marcadores.push(format!("?{}", valores.len()));
        }
        // O que o estado nao tem, e a coluna exige, entra provisorio.
        for (coluna, literal) in mapa.obrigatorias {
            if nomes.contains(coluna) {
                continue;
            }
            nomes.push(coluna);
            marcadores.push((*literal).to_owned());
        }

        // A pergunta "ja existe?" e feita explicitamente, e o `INSERT` e nu.
        //
        // Era `INSERT OR IGNORE`, e isso escondia o defeito em vez de evitar: o
        // `OR IGNORE` engole QUALQUER violacao de restricao, chave estrangeira
        // inclusive. Uma prova cuja disciplina ainda nao tinha chegado nao era
        // inserida, o `UPDATE` seguinte nao encontrava linha, e a rodada
        // terminava dizendo que deu tudo certo — o estado de sincronizacao tinha
        // a prova, a tabela nao tinha, e nada na tela indicava a diferenca.
        //
        // Nu, a violacao sobe, a entidade entra na fila de pendentes e a
        // retentativa a resolve quando o pai chegar. Erro que aparece e erro que
        // pode ser corrigido.
        let ja_existe: bool = transacao
            .query_row(
                &format!(
                    "SELECT 1 FROM {} WHERE {} = {chave_marcador}",
                    mapa.tabela, mapa.chave
                ),
                // Numa tabela de linha unica a chave e literal, e ligar o id
                // aqui seria um parametro a mais do que o comando referencia.
                rusqlite::params_from_iter(
                    mapa.linha_unica
                        .is_none()
                        .then(|| rusqlite::types::Value::Text(valor_da_chave.clone()))
                        .iter(),
                ),
                |_| Ok(()),
            )
            .optional()
            .map_err(map_sql_error)?
            .is_some();

        if !ja_existe {
            transacao
                .execute(
                    &format!(
                        "INSERT INTO {} ({}) VALUES ({})",
                        mapa.tabela,
                        nomes.join(", "),
                        marcadores.join(", ")
                    ),
                    rusqlite::params_from_iter(valores.iter()),
                )
                .map_err(map_sql_error)?;
        }

        // E agora o `UPDATE`, que e o caminho da linha que JA existia — criada
        // aqui, ou criada por uma operacao anterior deste mesmo lote.
        for (campo, coluna) in mapa.colunas {
            let Some(resolvido) = estado.campos.get(*campo) else {
                continue;
            };
            // Os parametros sao montados junto com o SQL, e nao fixos em tres.
            //
            // Com `?2` e `?3` sempre ligados, uma tabela sem `updated_at` ou de
            // linha unica recebia mais valores do que o comando referencia, e o
            // `UPDATE` falhava — silenciosamente, porque a projecao manda a
            // entidade para a fila de pendentes e tenta de novo. O sintoma era
            // uma configuracao que emitia, viajava e nunca aparecia.
            let mut valores: Vec<rusqlite::types::Value> = vec![valor_sql(&resolvido.valor)];
            let toque = if mapa.carimbos.tem_atualizacao() {
                valores.push(rusqlite::types::Value::Text(momento.clone()));
                format!(", updated_at = ?{}", valores.len())
            } else {
                String::new()
            };
            let alvo = match mapa.linha_unica {
                Some(literal) => literal.to_owned(),
                None => {
                    valores.push(rusqlite::types::Value::Text(valor_da_chave.clone()));
                    format!("?{}", valores.len())
                }
            };
            transacao
                .execute(
                    &format!(
                        "UPDATE {} SET {coluna} = ?1{toque} WHERE {} = {alvo}",
                        mapa.tabela, mapa.chave
                    ),
                    rusqlite::params_from_iter(valores.iter()),
                )
                .map_err(map_sql_error)?;
        }
        Ok(())
    }

    /// Uma aresta do Knowledge Graph vira (ou deixa de ser) linha de juncao.
    ///
    /// # Por que a relacao nao cabe no mapa dos outros tipos
    ///
    /// Ela nao tem tabela propria nem id na tabela: `resource_projects` sao duas
    /// colunas e uma chave primaria composta. O id da operacao e um UUID v5
    /// DERIVADO do par — ele existe para os dois dispositivos chegarem ao mesmo
    /// id sem se falarem, e nao para virar coluna.
    ///
    /// # Ligar e desligar, e nao criar e apagar
    ///
    /// `linked` e um campo, e o merge por campo decide pelo instante: desvincular
    /// as 10:00 e revincular as 10:05 termina VINCULADO. Se desligar fosse
    /// `Delete`, a semantica de "apagar ganha de editar" faria o contrario —
    /// certa para uma Task, errada para um interruptor.
    ///
    /// Por isso o `false` apaga a linha da juncao e nao deixa rastro: o rastro
    /// mora no estado de sync, com o instante, que e onde a proxima operacao vai
    /// olhar para decidir.
    fn materializar_relacao(
        transacao: &Connection,
        estado: &EstadoDaEntidade,
    ) -> Result<(), CoreError> {
        let texto = |campo: &str| {
            estado
                .campos
                .get(campo)
                .and_then(|resolvido| resolvido.valor.as_str())
                .map(str::to_owned)
        };
        // Sem os tres, nao da para saber O QUE foi ligado — o id sozinho e um
        // hash. Eles viajam junto justamente para o dispositivo que ve a relacao
        // pela primeira vez conseguir materializa-la.
        let (Some(kind), Some(de), Some(para)) = (texto("kind"), texto("from"), texto("to")) else {
            return Ok(());
        };
        let Some((tabela, coluna_de, coluna_para)) = juncao_de(&kind) else {
            return Ok(());
        };
        let ligado = estado
            .campos
            .get("linked")
            .and_then(|resolvido| resolvido.valor.as_bool())
            .unwrap_or(false);

        if ligado {
            let momento = crate::repository::format_time(OffsetDateTime::now_utc())?;
            // `OR IGNORE` porque ligar duas vezes e o mesmo vinculo: a chave
            // primaria composta recusa a segunda, e essa e a idempotencia que o
            // id derivado do par promete.
            //
            // Mas ele engole a chave ESTRANGEIRA junto — foi assim que a prova
            // sumia em silencio. Por isso a linha e conferida logo abaixo.
            transacao
                .execute(
                    &format!(
                        "INSERT OR IGNORE INTO {tabela} ({coluna_de}, {coluna_para}, created_at) \
                         VALUES (?1, ?2, ?3)"
                    ),
                    params![de, para, momento],
                )
                .map_err(map_sql_error)?;

            // Uma aresta cuja ponta ainda nao chegou seria "inserida" sem erro e
            // sem linha. O erro precisa subir para ela entrar na fila de
            // pendentes e ser tentada de novo quando a ponta chegar.
            let existe: bool = transacao
                .query_row(
                    &format!(
                        "SELECT 1 FROM {tabela} WHERE {coluna_de} = ?1 AND {coluna_para} = ?2"
                    ),
                    params![de, para],
                    |_| Ok(()),
                )
                .optional()
                .map_err(map_sql_error)?
                .is_some();
            if !existe {
                return Err(crate::sync_emit::erro_de_sync(format!(
                    "O vinculo {kind} nao pode ser criado: uma das pontas ainda nao chegou."
                )));
            }
        } else {
            transacao
                .execute(
                    &format!("DELETE FROM {tabela} WHERE {coluna_de} = ?1 AND {coluna_para} = ?2"),
                    params![de, para],
                )
                .map_err(map_sql_error)?;
        }
        Ok(())
    }

    /// Aplica uma operacao LOCAL sobre o estado de sync, na mesma transacao.
    ///
    /// Sem isto, a tabela sombra so saberia do que veio de fora, e a proxima
    /// operacao remota reconciliaria contra um estado sem as edicoes deste
    /// dispositivo — que e o mesmo que perde-las.
    pub(crate) fn absorver_local(&self, transacao: &Connection, op: &Op) -> Result<(), CoreError> {
        let kind = op.entity.kind.as_str();
        let base = Self::estado_guardado(transacao, kind, op.entity.id)?;
        let reconciliado = mos_sync::aplicar(base, std::slice::from_ref(op)).estado;
        Self::guardar_estado(transacao, kind, op.entity.id, &reconciliado)
    }
}

/// Traduz o JSON do campo para o que a coluna aceita.
///
/// # Por que nao basta "vira texto"
///
/// As tabelas do M/OS sao `STRICT`, e nelas o tipo e conferido na gravacao.
/// `academic_exams.weight`, `.score`, `academic_study_sessions.seconds` e
/// `reminders.snooze_count` sao numericas — mandar `"5"` como texto para uma
/// coluna `INTEGER` nao e um valor arredondado, e um erro que derruba a rodada
/// inteira. E `snoozeAllowed` viaja como booleano, que em SQLite e 0 ou 1.
///
/// Objeto e lista viram texto de proposito: o `trigger` do Reminder ja e
/// guardado como JSON numa coluna `TEXT` (ver a migration dos lembretes), e
/// serializar de volta e exatamente o que a coluna espera.
fn valor_sql(valor: &serde_json::Value) -> rusqlite::types::Value {
    use rusqlite::types::Value;
    match valor {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::String(texto) => Value::Text(texto.clone()),
        serde_json::Value::Bool(sim) => Value::Integer(i64::from(*sim)),
        serde_json::Value::Number(numero) => match numero.as_i64() {
            Some(inteiro) => Value::Integer(inteiro),
            None => match numero.as_f64() {
                Some(real) => Value::Real(real),
                // Numero que nao cabe em i64 nem f64 e patologico; guardar o
                // texto preserva o dado em vez de perde-lo por nao caber.
                None => Value::Text(numero.to_string()),
            },
        },
        outro => Value::Text(outro.to_string()),
    }
}

/// A `Projecao` do motor, sobre o banco de verdade.
///
/// **Deliberadamente nao publica.** Ela sozinha nao completa uma rodada: quem
/// nao chamar `resolver_pendentes` depois deixa entidades guardadas no estado e
/// invisiveis na tela, sem erro nenhum. Foi exatamente isso que aconteceu num
/// teste que montava o caminho a mao — ele passava por um caminho que o M/OS nao
/// usa. A unica porta e `SqliteStorage::sincronizar_agora`, que faz as duas
/// metades.
///
/// # NAO troque `connection.lock()` por `escrita()` aqui dentro
///
/// Toda escrita do crate passa pelo portao (`SqliteStorage::portao`), e a regra
/// vale — mas esta struct e a excecao, e nao por descuido: ela roda POR BAIXO de
/// `sincronizar_agora`, que ja tomou o portao para a rodada inteira. O cadeado
/// nao e reentrante, entao pedi-lo de novo aqui prenderia a rodada em si mesma.
///
/// Pelo mesmo motivo nada aqui emite operacao: o que a projecao grava veio de
/// FORA, e reemiti-lo devolveria ao hub o que o hub acabou de mandar.
pub(crate) struct ProjecaoSqlite<'a> {
    pub(crate) storage: &'a SqliteStorage,
    /// Entidades cujo estado foi gravado mas que ainda nao viraram linha.
    ///
    /// Existe por causa de uma ordem que o motor nao promete e nao deveria: ele
    /// agrupa as operacoes por `(kind, id)` num `BTreeMap`, e itera em ordem
    /// ALFABETICA. `academic_exam` vem antes de `academic_semester`, entao a
    /// prova era materializada antes de a disciplina e o semestre existirem —
    /// `FOREIGN KEY constraint failed`, de forma deterministica.
    ///
    /// A saida NAO e uma tabela de profundidades mantida a mao: ela envelhece
    /// mal, e a primeira coluna nova com chave estrangeira que alguem esquecer
    /// de declarar vira o mesmo defeito de volta. E uma retentativa ate parar de
    /// progredir — quem sabe quem depende de quem e o banco, e ele ja responde
    /// isso recusando a linha.
    pendentes: Vec<(String, uuid::Uuid)>,
}

impl<'a> ProjecaoSqlite<'a> {
    pub(crate) fn nova(storage: &'a SqliteStorage) -> Self {
        Self {
            storage,
            pendentes: Vec::new(),
        }
    }

    /// Tenta materializar de novo o que ficou pendente, ate parar de progredir.
    ///
    /// Ponto fixo, e nao um numero de passadas: a profundidade da arvore de
    /// dependencias e do esquema, nao deste laco. Ele para quando uma rodada
    /// inteira nao resolve nada — e o que sobra continua guardado no estado, que
    /// e a fonte da verdade para reconciliar.
    pub(crate) fn resolver_pendentes(&mut self) -> Vec<String> {
        while !self.pendentes.is_empty() {
            let tentativa = std::mem::take(&mut self.pendentes);
            let antes = tentativa.len();
            let mut faltas = Vec::new();
            for (kind, id) in tentativa {
                match self.materializar_um(&kind, id) {
                    Ok(()) => self.limpar_pendente(&kind, id),
                    Err(causa) => faltas.push((kind, id, causa)),
                }
            }
            if faltas.len() == antes {
                // Ninguem avancou: o que falta depende de algo que nao chegou.
                return faltas
                    .into_iter()
                    .map(|(kind, id, causa)| format!("{kind} {id}: {causa}"))
                    .collect();
            }
            self.pendentes = faltas.into_iter().map(|(k, i, _)| (k, i)).collect();
        }
        Vec::new()
    }

    /// Tira a entidade da fila — ela virou linha.
    fn limpar_pendente(&self, kind: &str, id: uuid::Uuid) {
        let Ok(conexao) = self.storage.connection.lock() else {
            return;
        };
        let _ = conexao.execute(
            "DELETE FROM sync_pendentes WHERE entity_kind = ?1 AND entity_id = ?2",
            rusqlite::params![kind, id.to_string()],
        );
    }

    /// Anota que esta entidade chegou e ainda nao virou linha.
    ///
    /// `tentativas` sobe a cada passagem: e o numero que separa "acabou de
    /// chegar fora de ordem" de "esta encalhada ha dias", e sem ele o reparo nao
    /// teria como dizer qual das duas coisas esta acontecendo.
    fn anotar_pendente(&self, kind: &str, id: uuid::Uuid, erro: &str) {
        // Falhar ao ANOTAR nao pode derrubar a rodada: a entidade continua no
        // estado, e a varredura da abertura a encontra de qualquer jeito — a
        // tabela e um atalho, e nao a unica memoria.
        let Ok(conexao) = self.storage.connection.lock() else {
            return;
        };
        let agora =
            crate::repository::format_time(time::OffsetDateTime::now_utc()).unwrap_or_default();
        let _ = conexao.execute(
            "INSERT INTO sync_pendentes (entity_kind, entity_id, tentativas, ultimo_erro,              atualizado_em) VALUES (?1, ?2, 1, ?3, ?4)              ON CONFLICT(entity_kind, entity_id) DO UPDATE SET              tentativas = tentativas + 1, ultimo_erro = ?3, atualizado_em = ?4",
            rusqlite::params![kind, id.to_string(), erro, agora],
        );
    }

    /// Materializa uma entidade a partir do estado ja guardado.
    fn materializar_um(&self, kind: &str, id: uuid::Uuid) -> Result<(), String> {
        let conexao = self
            .storage
            .connection
            .lock()
            .map_err(|_| String::from("Banco local ocupado."))?;
        let estado = SqliteStorage::estado_guardado(&conexao, kind, id)
            .map_err(|causa| causa.message.clone())?;
        let transacao = conexao
            .unchecked_transaction()
            .map_err(|causa| causa.to_string())?;
        SqliteStorage::materializar(&transacao, kind, id, &estado)
            .map_err(|causa| causa.message.clone())?;
        transacao.commit().map_err(|causa| causa.to_string())
    }
}

impl Projecao for ProjecaoSqlite<'_> {
    fn estado_de(&self, op: &Op) -> EstadoDaEntidade {
        let Ok(conexao) = self.storage.connection.lock() else {
            return EstadoDaEntidade::default();
        };
        SqliteStorage::estado_guardado(&conexao, op.entity.kind.as_str(), op.entity.id)
            .unwrap_or_default()
    }

    fn guardar(&mut self, op: &Op, estado: &EstadoDaEntidade) -> Resultado<()> {
        let conexao = self
            .storage
            .connection
            .lock()
            .map_err(|_| SyncError::novo("Banco local ocupado.", true))?;
        let transacao = conexao
            .unchecked_transaction()
            .map_err(map_sql_error)
            .map_err(falha)?;
        let kind = op.entity.kind.as_str();

        // O ESTADO commita sempre; a materializacao pode esperar.
        //
        // Eram uma transacao so, e a razao era boa: gravar um e falhar no outro
        // deixaria a tela mostrando o valor antigo enquanto a reconciliacao ja
        // considera o novo. O que essa razao nao previa e que a materializacao
        // falha por um motivo LEGITIMO e temporario — o pai ainda nao chegou —,
        // e ai derrubar a rodada inteira e a resposta errada.
        //
        // A invariante que fica de pe: o estado e a fonte da verdade, e a
        // materializacao converge. `resolver_pendentes` fecha o ciclo antes de a
        // rodada terminar.
        SqliteStorage::guardar_estado(&transacao, kind, op.entity.id, estado).map_err(falha)?;
        transacao.commit().map_err(map_sql_error).map_err(falha)?;
        drop(conexao);

        if self.materializar_um(kind, op.entity.id).is_err() {
            self.pendentes.push((kind.to_owned(), op.entity.id));
            // A fila em memoria serve a ESTA rodada; a tabela e o que faz a
            // proxima abertura saber que ficou algo para tras.
            self.anotar_pendente(kind, op.entity.id, "materializacao adiada");
        }
        Ok(())
    }
}

impl SqliteStorage {
    /// Uma rodada de sincronizacao, com o relogio DESTE dispositivo.
    ///
    /// # Por que o motor e chamado daqui, e nao do app
    ///
    /// O `sincronizar()` precisa de `&mut HlcClock`, e o relogio ja mora aqui
    /// dentro, ligado por `habilitar_sync`. Se o app criasse o proprio, este
    /// dispositivo teria DOIS relogios emitindo instantes — e duas operacoes
    /// diferentes com o mesmo instante e o mesmo dispositivo quebram a ordem
    /// total, que e a unica coisa que a reconciliacao tem para desempatar.
    ///
    /// O mutex do relogio fica preso durante a rodada inteira. E de proposito:
    /// uma mutacao local no meio dela espera, em vez de emitir um instante que
    /// o motor ja passou. Rodadas sao curtas; ordem total nao se recupera.
    pub fn sincronizar_agora(
        &self,
        transporte: &dyn mos_sync::Transport,
        agora_ms: i64,
        limite: usize,
    ) -> Result<mos_sync::Rodada, CoreError> {
        // O PORTAO ANTES DO RELOGIO, e o "antes" e o conserto inteiro.
        //
        // Esta rodada vai pedir a conexao muitas vezes la dentro, pela projecao,
        // enquanto segura o relogio. Uma escrita que chegasse no meio disso
        // estaria com a conexao na mao esperando o relogio, e as duas esperariam
        // uma pela outra para sempre. Ver `SqliteStorage::portao`.
        let _portao = self.portao()?;
        let mut slot = self.sync.lock().map_err(crate::map_lock_error)?;
        let Some(relogio) = slot.as_mut() else {
            return Err(crate::sync_emit::erro_de_sync(
                "A sincronizacao ainda nao foi ligada neste dispositivo.",
            ));
        };
        let mut projecao = ProjecaoSqlite::nova(self);
        let deposito = mos_sync::Deposito {
            outbox: self,
            conflitos: self,
            relogio: self,
            dispositivos: self,
        };

        // O botao diz "sincronizar", entao ele sincroniza — e nao "manda ate
        // cem".
        //
        // Uma passada do motor empurra um lote e puxa um lote. Com 370 na fila e
        // limite 100, um clique deixava 270 para tras e a tela mostrava o numero
        // certo com a impressao errada: parecia que tinha acabado. Quem sabe se
        // acabou e o proprio motor — `pendentes` de um lado, `tem_mais` do
        // outro.
        let mut rodada = mos_sync::Rodada::default();
        let mut passadas = 0;
        loop {
            let passada = mos_sync::sincronizar(
                &deposito,
                transporte,
                relogio,
                &mut projecao,
                agora_ms,
                limite,
            );
            passadas += 1;
            rodada.enviadas += passada.enviadas;
            rodada.recebidas += passada.recebidas;
            // Funde, e nao substitui: com a fila grande sao varias passadas, e
            // atribuir deixaria a faixa da Home contando so a ultima.
            for (tipo, quantas) in passada.recebidas_por_tipo {
                *rodada.recebidas_por_tipo.entry(tipo).or_insert(0) += quantas;
            }
            rodada.conflitos += passada.conflitos;
            rodada.pendentes = passada.pendentes;
            rodada.tem_mais = passada.tem_mais;

            if passada.erro.is_some() {
                // O que ja subiu e desceu nas passadas anteriores permanece
                // feito. Continuar depois de um erro seria bater na mesma
                // parede com a mesma pedra.
                rodada.erro = passada.erro;
                break;
            }
            if passada.pendentes == 0 && !passada.tem_mais {
                break;
            }
            // Nada saiu e nada entrou, mas ainda ha o que fazer: insistir
            // repetiria a mesma passada para sempre. Acontece quando a fila tem
            // uma operacao que o outro lado nao aceita.
            if passada.enviadas == 0 && passada.recebidas == 0 {
                break;
            }
            if passadas >= MAX_PASSADAS {
                rodada.erro = Some(format!(
                    "Parei em {MAX_PASSADAS} passadas com {} ainda na fila. \
                     Sincronize de novo.",
                    passada.pendentes
                ));
                break;
            }
        }

        // O que nao virou linha por dependencia ainda ausente, agora vira.
        //
        // O que sobrar depois disto depende de algo que nao chegou nesta rodada
        // — e continua guardado no estado, inteiro. A rodada NAO mente sobre
        // isso: a falha vai no `erro`, porque uma entidade que existe no banco
        // de sincronizacao e nao aparece na tela e exatamente o tipo de coisa
        // que o usuario precisa saber antes de concluir que o M/OS perdeu algo.
        let faltando = projecao.resolver_pendentes();
        if !faltando.is_empty() && rodada.erro.is_none() {
            rodada.erro = Some(format!(
                "{} entidade(s) chegaram mas ainda nao aparecem: dependem de algo \
                 que nao veio nesta rodada. Sincronize de novo. [{}]",
                faltando.len(),
                faltando.join("; ")
            ));
        }
        Ok(rodada)
    }
}

/// Se alguma entrada de `mapa_de` materializa nesta tabela.
///
/// Existe para o teste de cobertura: ele conhece tabelas, e o mapa e indexado
/// por tipo. Sem isto, a lista de tipos seria uma TERCEIRA lista a manter em
/// acordo com as outras duas — que e o problema, e nao a solucao.
#[cfg(test)]
pub(crate) fn tem_mapa_para_tabela(tabela: &str) -> bool {
    // Uma tabela de juncao nao tem entrada em `mapa_de`: ela viaja como
    // `relation`, e quem sabe materializa-la e `juncao_de`. Perguntar as duas e
    // o que faz o teste aceitar as duas formas de atravessar sem precisar saber
    // qual e qual.
    TIPOS_DE_VINCULO
        .iter()
        .any(|kind| juncao_de(kind).is_some_and(|(juncao, _, _)| juncao == tabela))
        || TIPOS_CONHECIDOS
            .iter()
            .any(|kind| mapa_de(kind).is_some_and(|mapa| mapa.tabela == tabela))
}

/// Os tipos de vinculo que `juncao_de` responde.
#[cfg(test)]
const TIPOS_DE_VINCULO: &[&str] = &[
    "resourceProject",
    "resourceWorkspace",
    "projectWorkspace",
    "academic_subject_resource",
];

#[cfg(test)]
/// Os tipos que `mapa_de` responde. Enumerar um `match` nao e possivel em Rust,
/// entao a lista existe — e o teste de cobertura a mantem honesta comparando com
/// as tabelas.
const TIPOS_CONHECIDOS: &[&str] = &[
    "task",
    "project",
    "workspace",
    "capture",
    "resource",
    "reminder",
    "academic_semester",
    "academic_subject",
    "academic_assignment",
    "academic_exam",
    "academic_study_session",
    "academic_subject_resource",
    "time_entry",
    "project_tracking",
    "conversation",
    "message",
    "message_part",
    "daily_session",
    "daily_objective",
    "daily_reflection",
    "weekly_review",
    "client",
    "tracking_settings",
    "academic_provider_subject_fact",
];

/// Enfileira um `Create` por linha existente da tabela daquele tipo.
///
/// Le as colunas pelo proprio `Mapa`, e nao por uma lista escrita a mao aqui: o
/// que o backfill manda tem que ser exatamente o que a emissao mandaria, e duas
/// listas divergem.
pub(crate) fn enfileirar_tabela(
    storage: &SqliteStorage,
    transacao: &Connection,
    kind: &str,
) -> Result<usize, CoreError> {
    let Some(mapa) = mapa_de(kind) else {
        return Ok(0);
    };

    let colunas: Vec<&str> = mapa.colunas.iter().map(|(_, coluna)| *coluna).collect();
    // Numa tabela de linha unica a chave nem e lida: ela e `1`, um inteiro que
    // nao vira UUID, e o id da entidade e a constante conhecida pelos dois
    // aparelhos. Ler a coluna aqui daria "Invalid column type Integer".
    let id_fixo = match kind {
        "tracking_settings" => Some(ID_TRACKING_SETTINGS),
        _ => None,
    };
    let sql = if id_fixo.is_some() {
        format!("SELECT {} FROM {}", colunas.join(", "), mapa.tabela)
    } else {
        format!(
            "SELECT {}, {} FROM {}",
            mapa.chave,
            colunas.join(", "),
            mapa.tabela
        )
    };
    let deslocamento = usize::from(id_fixo.is_none());
    let mut consulta = transacao.prepare(&sql).map_err(map_sql_error)?;
    let linhas = consulta
        .query_map([], |linha| {
            let chave = match id_fixo {
                Some(fixo) => fixo.to_string(),
                None => linha.get::<_, String>(0)?,
            };
            let mut campos = Vec::with_capacity(mapa.colunas.len());
            for (indice, (campo, _)) in mapa.colunas.iter().enumerate() {
                campos.push((
                    (*campo).to_owned(),
                    json_de_sql(linha.get_ref(indice + deslocamento)?),
                ));
            }
            Ok((chave, campos))
        })
        .map_err(map_sql_error)?;

    let mut quantas = 0;
    for linha in linhas {
        let (chave, campos) = linha.map_err(map_sql_error)?;
        // Uma chave que nao e UUID nao tem como virar id de entidade. Pular e
        // certo: as tabelas de chave composta viajam por outro caminho, e
        // inventar um id aqui criaria uma entidade que so existe neste banco.
        let Ok(id) = uuid::Uuid::parse_str(&chave) else {
            continue;
        };
        storage.emitir(
            transacao,
            mos_sync::EntityRef::new(kind, id),
            mos_sync::OpBody::Create {
                fields: campos.into_iter().collect(),
            },
        )?;
        quantas += 1;
    }
    Ok(quantas)
}

/// Um valor do banco como o JSON que a operacao carrega.
fn json_de_sql(valor: rusqlite::types::ValueRef<'_>) -> serde_json::Value {
    use rusqlite::types::ValueRef;
    match valor {
        ValueRef::Null => serde_json::Value::Null,
        ValueRef::Integer(numero) => serde_json::json!(numero),
        ValueRef::Real(numero) => serde_json::json!(numero),
        ValueRef::Text(texto) => serde_json::json!(String::from_utf8_lossy(texto)),
        // Nenhuma coluna sincronizavel e BLOB hoje. Virar texto perdido e melhor
        // que entrar em panico numa passagem que roda uma vez so.
        ValueRef::Blob(bytes) => serde_json::json!(String::from_utf8_lossy(bytes)),
    }
}

#[cfg(test)]
mod tests {
    use mos_core::ConversationRepository;
    use mos_core::{NewProject, NewTimeEntry, TimeTrackingRepository, WorkRepository};
    use mos_sync::{DeviceRepository, Op, Projecao};

    use super::*;

    fn storage_que_emite() -> (SqliteStorage, tempfile::TempDir) {
        let pasta = tempfile::tempdir().unwrap();
        let storage =
            SqliteStorage::open(pasta.path().join("mos.db"), pasta.path().join("backups")).unwrap();
        let dispositivo = storage
            .este_dispositivo("teste", "windows", "0.0.0")
            .unwrap();
        storage.habilitar_sync(dispositivo.id).unwrap();
        (storage, pasta)
    }

    /// As operacoes que este dispositivo tem para mandar.
    fn ops_da_fila(storage: &SqliteStorage, kind: &str) -> Vec<Op> {
        let conexao = storage.connection.lock().unwrap();
        let mut consulta = conexao
            .prepare("SELECT payload FROM sync_outbox WHERE entity_kind = ?1")
            .unwrap();
        let linhas = consulta
            .query_map(rusqlite::params![kind], |linha| linha.get::<_, String>(0))
            .unwrap();
        linhas
            .map(|payload| serde_json::from_str(&payload.unwrap()).unwrap())
            .collect()
    }

    /// Aplica em `destino` as operacoes como se tivessem vindo do hub.
    ///
    /// Drena os pendentes no fim, e falha se sobrar algum. Sem isso um teste
    /// verde nao provaria nada: a projecao adia a materializacao que falha em
    /// vez de estourar, e um erro de SQL viraria "a linha nao apareceu" sem
    /// dizer por que.
    fn receber(destino: &SqliteStorage, ops: &[Op]) {
        let mut projecao = ProjecaoSqlite::nova(destino);
        for op in ops {
            let base = projecao.estado_de(op);
            let estado = mos_sync::aplicar(base, std::slice::from_ref(op)).estado;
            projecao.guardar(op, &estado).unwrap();
        }
        let faltas = projecao.resolver_pendentes();
        assert!(faltas.is_empty(), "a projecao nao materializou: {faltas:?}");
    }

    /// O caso que deixou entidade invisivel para sempre.
    ///
    /// A materializacao falha porque o pai nao chegou, e o processo morre. A
    /// fila tem que sobreviver a isso — em memoria ela nao sobrevivia, e a
    /// abertura seguinte nao sabia que havia o que consertar.
    #[test]
    fn o_pendente_sobrevive_ao_fechamento_do_app() {
        let (storage, _guarda) = storage_que_emite();

        // Uma hora cujo projeto nao existe: a chave estrangeira recusa.
        let hora = uuid::Uuid::now_v7();
        let op = Op::new(
            uuid::Uuid::now_v7(),
            mos_sync::EntityRef::new("time_entry", hora),
            mos_sync::OpBody::Create {
                fields: [
                    (
                        "projectId".to_owned(),
                        serde_json::json!(uuid::Uuid::now_v7().to_string()),
                    ),
                    (
                        "startedAt".to_owned(),
                        serde_json::json!("2026-09-04T10:00:00Z"),
                    ),
                    ("durationSeconds".to_owned(), serde_json::json!(3600)),
                ]
                .into_iter()
                .collect(),
            },
            mos_sync::Hlc::new(1, 0, mos_sync::DeviceId(uuid::Uuid::now_v7())),
        );

        {
            let mut projecao = ProjecaoSqlite::nova(&storage);
            let base = projecao.estado_de(&op);
            let estado = mos_sync::aplicar(base, std::slice::from_ref(&op)).estado;
            projecao.guardar(&op, &estado).unwrap();
            // A projecao morre aqui, como quando o app fecha.
        }

        let guardados: Vec<(String, String)> = {
            let conexao = storage.connection.lock().unwrap();
            let mut consulta = conexao
                .prepare("SELECT entity_kind, entity_id FROM sync_pendentes")
                .unwrap();
            let linhas = consulta
                .query_map([], |linha| {
                    Ok((linha.get::<_, String>(0)?, linha.get::<_, String>(1)?))
                })
                .unwrap();
            linhas.map(|linha| linha.unwrap()).collect()
        };
        assert_eq!(
            guardados,
            vec![("time_entry".to_owned(), hora.to_string())],
            "o pendente nao sobreviveu: na proxima abertura ninguem tentaria de novo"
        );
    }

    /// O diario JA emitia, e a operacao viajava — so nunca virava linha.
    ///
    /// E o caso que o cabecalho do modulo descreve como "tipo desconhecido nao e
    /// erro": o outro lado guardava o estado e nao materializava nada. Do lado
    /// de fora e indistinguivel de nao sincronizar, e por isso o teste de
    /// cobertura pergunta pelas duas metades e nao so pela emissao.
    #[test]
    fn o_dia_iniciado_num_pc_aparece_no_outro() {
        let (origem, _guarda_origem) = storage_que_emite();
        let (destino, _guarda_destino) = storage_que_emite();

        let agora = time::OffsetDateTime::now_utc();
        let dia = mos_core::Day::parse("2026-08-31").unwrap();
        let nova =
            mos_core::NewDailySession::create(dia.clone(), "fechar o Rancho", agora).unwrap();
        let id = nova.id;
        mos_core::DailyRepository::start_day(&origem, nova, Vec::new(), agora).unwrap();

        receber(&destino, &ops_da_fila(&origem, "daily_session"));

        let sessao = mos_core::DailyRepository::session(&destino, id).unwrap();
        assert_eq!(
            sessao.note, "fechar o Rancho",
            "o dia iniciado na origem nao virou linha no destino"
        );
        assert_eq!(sessao.day, dia);
    }

    /// `messages` so tem `created_at`; `message_parts` nao tem carimbo nenhum.
    ///
    /// Sao os dois tipos que exigem o `Carimbos`: escrever `updated_at` neles
    /// seria SQL contra coluna inexistente. E o corpo da mensagem mora nas
    /// partes — sem elas a conversa chega com remetente e sem texto, que e pior
    /// que nao chegar.
    #[test]
    fn uma_conversa_do_hermes_chega_inteira_no_outro_pc() {
        let (origem, _guarda_origem) = storage_que_emite();
        let (destino, _guarda_destino) = storage_que_emite();

        let conversa = origem
            .create_conversation(mos_core::NewConversation::create())
            .unwrap();
        origem
            .set_conversation_title(conversa.id, "Orcamento do Rancho")
            .unwrap();
        origem
            .append_message(
                mos_core::NewMessage::user(conversa.id, "quanto ficou a obra?").unwrap(),
            )
            .unwrap();

        for kind in ["conversation", "message", "message_part"] {
            receber(&destino, &ops_da_fila(&origem, kind));
        }

        let conversas = destino.conversations(false, 10).unwrap();
        assert_eq!(conversas.len(), 1, "a conversa nao atravessou");
        assert_eq!(conversas[0].title, "Orcamento do Rancho");

        let mensagens = destino.messages(conversa.id).unwrap();
        assert_eq!(mensagens.len(), 1, "a mensagem nao atravessou");
        assert!(
            !mensagens[0].parts.is_empty(),
            "a mensagem chegou sem as partes: o remetente atravessou e o texto nao"
        );
    }

    /// Um semestre e uma disciplina minimos, direto no banco.
    fn semear_disciplina(storage: &SqliteStorage, disciplina: uuid::Uuid) {
        let semestre = uuid::Uuid::now_v7().to_string();
        let conexao = storage.escrita().unwrap();
        conexao
            .execute(
                "INSERT INTO academic_semesters
                     (id, name, institution, starts_on, ends_on, lifecycle_state,
                      created_at, updated_at)
                 VALUES (?1, '2026.2', 'UFSC', '2026-08-01', '2026-11-30', 'active',
                         '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z')",
                rusqlite::params![semestre],
            )
            .unwrap();
        conexao
            .execute(
                "INSERT INTO academic_subjects
                     (id, semester_id, name, code, teacher, accent, notes,
                      lifecycle_state, created_at, updated_at)
                 VALUES (?1, ?2, 'Calculo III', 'MAT03', '', '', '', 'active',
                         '2026-08-01T00:00:00Z', '2026-08-01T00:00:00Z')",
                rusqlite::params![disciplina.to_string(), semestre],
            )
            .unwrap();
    }

    /// A chave e `(provider, subject_id)`, e nao um UUID.
    ///
    /// O `Op` exige `entity.id: Uuid`, entao o id e DERIVADO da chave composta —
    /// o mesmo recurso que `mos_sync::Relacao` usa para as juncoes, e com a
    /// mesma advertencia: o namespace nunca muda, porque muda-lo faria todas as
    /// notas existentes ganharem ids novos e as antigas ficarem orfas.
    #[test]
    fn a_nota_oficial_atravessa_apesar_da_chave_composta() {
        let (origem, _guarda_origem) = storage_que_emite();
        let (destino, _guarda_destino) = storage_que_emite();

        let disciplina = uuid::Uuid::now_v7();
        // Semestre e disciplina nos DOIS lados: `subject_id` e chave
        // estrangeira, e a nota de uma disciplina que nao existe e recusada
        // antes de chegar a ser um problema de sincronizacao.
        for banco in [&origem, &destino] {
            semear_disciplina(banco, disciplina);
        }
        // A linha nasce por fora do repositorio de dominio: o que este teste
        // exercita e a emissao e a projecao, e nao a importacao do provedor.
        origem
            .escrita()
            .unwrap()
            .execute(
                "INSERT INTO academic_provider_subject_facts
                     (provider, subject_id, situation, official_grade, updated_at)
                 VALUES ('univirtus', ?1, 'aprovado', 8.5, '2026-08-31T00:00:00Z')",
                rusqlite::params![disciplina.to_string()],
            )
            .unwrap();
        SqliteStorage::emitir_fato_de_disciplina(
            &origem,
            "univirtus",
            &disciplina.to_string(),
            Some("aprovado"),
            Some(8.5),
        )
        .unwrap();

        receber(
            &destino,
            &ops_da_fila(&origem, "academic_provider_subject_fact"),
        );

        let (situacao, nota): (String, f64) = destino
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT situation, official_grade FROM academic_provider_subject_facts
                 WHERE provider = 'univirtus' AND subject_id = ?1",
                rusqlite::params![disciplina.to_string()],
                |linha| Ok((linha.get(0)?, linha.get(1)?)),
            )
            .expect("a nota oficial nao atravessou");
        assert_eq!(situacao, "aprovado");
        assert_eq!(nota, 8.5);
    }

    /// O arredondamento MUDA o numero cobravel.
    ///
    /// `cronocad_import.rs` ja registra isso para a importacao: trazer as horas
    /// sem a configuracao faz o M/OS mostrar um valor diferente do que o
    /// CronoCAD mostrava. Entre dois PCs o estrago e o mesmo — as horas
    /// atravessam, a regra nao, e os dois mostram totais faturaveis diferentes
    /// para o mesmo trabalho.
    ///
    /// So a metade que e SUA viaja. Deteccao de ociosidade e monitoramento de
    /// processo descrevem a maquina, e replicados fariam este PC vigiar o que o
    /// outro vigia.
    #[test]
    fn a_regra_de_arredondamento_atravessa_e_a_config_de_maquina_nao() {
        let (origem, _guarda_origem) = storage_que_emite();
        let (destino, _guarda_destino) = storage_que_emite();

        let mut regras = mos_core::TimeTrackingRepository::tracking_settings(&origem).unwrap();
        regras.rounding.interval_minutes = 30;
        regras.rounding.mode = mos_core::RoundingMode::Up;
        // A tarifa padrao viaja pela mesma razao que o arredondamento: as duas
        // decidem quanto vale a hora, e so uma delas atravessar faria os dois
        // PCs cobrarem numeros diferentes pelo mesmo trabalho.
        regras.default_hourly_rate_cents = 9_000;
        mos_core::TimeTrackingRepository::set_tracking_settings(&origem, regras).unwrap();
        // A metade de maquina nem aparece no tipo de dominio: mexo nela por SQL
        // para provar que ela existe na origem e NAO viaja.
        origem
            .connection
            .lock()
            .unwrap()
            .execute(
                "UPDATE tracking_settings SET idle_threshold_minutes = 99 WHERE id = 1",
                [],
            )
            .unwrap();

        receber(&destino, &ops_da_fila(&origem, "tracking_settings"));

        let chegou = mos_core::TimeTrackingRepository::tracking_settings(&destino).unwrap();
        assert_eq!(
            chegou.rounding.interval_minutes, 30,
            "o intervalo de arredondamento nao atravessou"
        );
        assert_eq!(chegou.rounding.mode, mos_core::RoundingMode::Up);
        assert_eq!(
            chegou.default_hourly_rate_cents, 9_000,
            "a tarifa padrao nao atravessou"
        );
        let ociosidade_no_destino: i64 = destino
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT idle_threshold_minutes FROM tracking_settings WHERE id = 1",
                [],
                |linha| linha.get(0),
            )
            .unwrap();
        assert_ne!(
            ociosidade_no_destino, 99,
            "a config de maquina atravessou e nao devia"
        );
    }

    /// `project_tracking.client_id` e CHAVE ESTRANGEIRA para `clients`
    /// (migration 0013). Sincronizar a cobranca sem o cliente faz a linha ser
    /// recusada no destino — e o commit que ligou `project_tracking` sem ligar
    /// `clients` introduziu exatamente isso.
    #[test]
    fn a_cobranca_com_cliente_nao_quebra_a_chave_estrangeira_no_destino() {
        let (origem, _guarda_origem) = storage_que_emite();
        let (destino, _guarda_destino) = storage_que_emite();

        let projeto = NewProject::create("Rancho Queimado", "", "").unwrap();
        let id_projeto = projeto.id;
        origem.create_project(projeto.clone()).unwrap();
        destino.create_project(projeto).unwrap();

        let cliente = mos_core::TimeTrackingRepository::create_client(
            &origem,
            mos_core::ClientInput {
                name: String::from("Juliano"),
                company_name: String::new(),
                email: String::new(),
                phone: String::new(),
                notes: String::new(),
            },
        )
        .unwrap();
        origem
            .set_project_tracking(mos_core::ProjectTracking {
                project_id: id_projeto,
                hourly_rate_cents: 12_000,
                code: String::new(),
                color: String::new(),
                tracking_status: mos_core::TrackingStatus::Active,
                client_id: Some(cliente.id),
                budget_minutes: 0,
                paid_at: None,
            })
            .unwrap();

        receber(&destino, &ops_da_fila(&origem, "client"));
        receber(&destino, &ops_da_fila(&origem, "project_tracking"));

        let clientes = mos_core::TimeTrackingRepository::clients(&destino, false).unwrap();
        assert_eq!(clientes.len(), 1, "o cliente nao atravessou");
        let cobranca = destino.project_tracking().unwrap();
        assert_eq!(
            cobranca.first().and_then(|linha| linha.client_id),
            Some(cliente.id),
            "a cobranca chegou sem o cliente: a chave estrangeira foi recusada"
        );
    }

    /// A chave de `project_tracking` e `project_id`, e nao `id`.
    ///
    /// Este teste existe para o `INSERT` da projecao parar de assumir uma coluna
    /// chamada `id`: a tabela nao tem nenhuma, e o valor/hora do projeto e um
    /// dado que vira FATURA — errar aqui e errar quanto se cobra.
    #[test]
    fn o_valor_hora_do_projeto_atravessa_mesmo_sem_coluna_id() {
        let (origem, _guarda_origem) = storage_que_emite();
        let (destino, _guarda_destino) = storage_que_emite();

        let projeto = NewProject::create("Rancho Queimado", "", "").unwrap();
        let id_projeto = projeto.id;
        origem.create_project(projeto.clone()).unwrap();
        destino.create_project(projeto).unwrap();

        origem
            .set_project_tracking(mos_core::ProjectTracking {
                project_id: id_projeto,
                hourly_rate_cents: 12_000,
                code: String::from("043"),
                color: String::new(),
                tracking_status: mos_core::TrackingStatus::Active,
                client_id: None,
                budget_minutes: 2_400,
                paid_at: None,
            })
            .unwrap();

        receber(&destino, &ops_da_fila(&origem, "project_tracking"));

        let cobranca = destino.project_tracking().unwrap();
        assert_eq!(cobranca.len(), 1, "o valor/hora do projeto nao atravessou");
        assert_eq!(cobranca[0].hourly_rate_cents, 12_000);
        assert_eq!(cobranca[0].code, "043");
        assert_eq!(cobranca[0].budget_minutes, 2_400);
    }

    #[test]
    fn horas_registradas_num_pc_viram_linha_no_outro() {
        let (origem, _guarda_origem) = storage_que_emite();
        let (destino, _guarda_destino) = storage_que_emite();

        // O projeto existe nos dois: ele ja sincroniza hoje, e sem ele a chave
        // estrangeira de `time_entries` recusaria a linha no destino.
        let projeto = NewProject::create("Rancho Queimado", "", "").unwrap();
        let id_projeto = projeto.id;
        origem.create_project(projeto.clone()).unwrap();
        destino.create_project(projeto).unwrap();

        origem
            .create_time_entry(NewTimeEntry {
                project_id: id_projeto,
                started_at: time::OffsetDateTime::now_utc(),
                ended_at: None,
                duration_seconds: 3_600,
                idle_seconds: 0,
                description: String::from("desenho da prancha"),
                activity_type: mos_core::ActivityType::Drawing,
                billable: true,
                hourly_rate_snapshot_cents: 12_000,
                source: mos_core::EntrySource::Timer,
            })
            .unwrap();

        receber(&destino, &ops_da_fila(&origem, "time_entry"));

        let horas = destino.time_entries(None).unwrap();
        assert_eq!(
            horas.len(),
            1,
            "a hora registrada na origem nao virou linha no destino"
        );
        assert_eq!(horas[0].duration_seconds, 3_600);
        assert_eq!(horas[0].description, "desenho da prancha");
    }
}
