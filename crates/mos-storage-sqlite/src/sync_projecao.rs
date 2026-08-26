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
}

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
                        "UPDATE {} SET lifecycle_state = 'trashed', updated_at = ?1 WHERE id = ?2",
                        mapa.tabela
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
        let mut nomes: Vec<&str> = vec!["id", "created_at", "updated_at"];
        let mut marcadores: Vec<String> = vec!["?1".into(), "?2".into(), "?2".into()];
        let mut valores: Vec<rusqlite::types::Value> = vec![
            rusqlite::types::Value::Text(id.to_string()),
            rusqlite::types::Value::Text(momento.clone()),
        ];
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
                &format!("SELECT 1 FROM {} WHERE id = ?1", mapa.tabela),
                params![id.to_string()],
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
            let valor = valor_sql(&resolvido.valor);
            transacao
                .execute(
                    &format!(
                        "UPDATE {} SET {coluna} = ?1, updated_at = ?2 WHERE id = ?3",
                        mapa.tabela
                    ),
                    params![valor, momento, id.to_string()],
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
    fn resolver_pendentes(&mut self) -> Vec<String> {
        while !self.pendentes.is_empty() {
            let tentativa = std::mem::take(&mut self.pendentes);
            let antes = tentativa.len();
            let mut faltas = Vec::new();
            for (kind, id) in tentativa {
                if let Err(causa) = self.materializar_um(&kind, id) {
                    faltas.push((kind, id, causa));
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
        let mut rodada = mos_sync::sincronizar(
            &deposito,
            transporte,
            relogio,
            &mut projecao,
            agora_ms,
            limite,
        );

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
