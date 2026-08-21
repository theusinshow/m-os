//! Acoes que o Hermes pode PROPOR, e que o M/OS executa.
//!
//! A regra que sustenta o arquivo inteiro: o modelo escreve uma proposta, e
//! quem age e o M/OS. Nada aqui executa — este modulo define o catalogo, valida
//! o que chegou e monta o preview. A execucao vive no orquestrador do desktop,
//! porque e la que os servicos de aplicacao estao, e ela usa os mesmos servicos
//! que a interface usa. Ver `SPEC-ACOES-ENTRE-APPS.md`.
//!
//! O catalogo desce no prompt, entao ele sai da maquina. Nomes de acao e forma
//! de argumento vao para a VPS — nao sao dados pessoais, mas sao um mapa do que
//! o sistema sabe fazer, e isso entra no registro da ADR-027 como o resto.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{CoreError, ErrorCode, FunctionConfirmation, FunctionRisk};

/// O que o M/OS aceita executar a partir de uma proposta.
///
/// Fase 1 so tem acao LOCAL, e de proposito: criar uma Task pelo Hermes nao
/// impressiona ninguem, mas e ela que prova preview, confirmacao, recibo e
/// Undo com risco baixo — antes de a primeira frase virar uma conta a pagar.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    CaptureCreate,
    /// Converte uma Capture que ja existe. E o inverso da tentacao do modelo
    /// prestativo: sem esta acao, "transforma isso em task" so podia virar
    /// `TaskCreate` — uma Task NOVA, com o texto copiado, e a Capture original
    /// continuando na Inbox como se nada tivesse acontecido.
    CaptureToTask,
    TaskCreate,
    TaskSetState,
    TaskSetProject,
    ProjectCreate,
    ResourceCreate,
    /// O lembrete, que e o que faltava para o M/OS ter ancora de tempo futuro
    /// pela conversa. Ate aqui o Hermes sabia criar Task e nao sabia agendar
    /// nada — e "me lembra hoje as 20:30" nao tinha para onde ir.
    ReminderCreate,
    ReminderResolve,
    TimeStart,
    TimeStop,
    TimeRecord,
    /// As cinco da Daily Session. Elas existem por um motivo estrutural, e nao
    /// por conveniencia: sem elas, "inicia meu dia" so podia virar
    /// `mos.task.create` — a mesma armadilha que o §2 do
    /// `HERMES-ACTION-LAYER.md` registrou quando faltava `ReminderCreate`. Um
    /// modelo sem a acao certa usa a errada, e o resultado e uma Task duplicada
    /// onde se pediu foco.
    DayStart,
    DayAddObjective,
    DaySetObjective,
    DaySetMain,
    DayEnd,
    MFinanceCreateBill,
}

impl ActionKind {
    /// O nome que atravessa a ponte. Prefixado pelo App dono, porque o catalogo
    /// vai crescer com `m-finance.*`.
    ///
    /// As de tempo levam `mos.` e nao `cronocad.`, apesar de a spec ter previsto
    /// o segundo: ela foi escrita antes da absorcao (ADR-032). O CronoCAD deixou
    /// de ser App a parte e virou a pagina de Tempo do M/OS, entao o dono da
    /// acao e o M/OS. Um prefixo que nomeia um App que nao existe mais ensinaria
    /// o modelo a falar de um sistema que o usuario nao tem.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CaptureCreate => "mos.capture.create",
            Self::CaptureToTask => "mos.capture.to_task",
            Self::TaskCreate => "mos.task.create",
            Self::TaskSetState => "mos.task.set_state",
            Self::TaskSetProject => "mos.task.set_project",
            Self::ProjectCreate => "mos.project.create",
            Self::ResourceCreate => "mos.resource.create",
            Self::ReminderCreate => "mos.reminder.create",
            Self::ReminderResolve => "mos.reminder.resolve",
            Self::TimeStart => "mos.time.start",
            Self::TimeStop => "mos.time.stop",
            Self::TimeRecord => "mos.time.record",
            Self::DayStart => "mos.day.start",
            Self::DayAddObjective => "mos.day.add_objective",
            Self::DaySetObjective => "mos.day.set_objective",
            Self::DaySetMain => "mos.day.set_main",
            Self::DayEnd => "mos.day.end",
            Self::MFinanceCreateBill => "m-finance.create_bill",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "mos.capture.create" => Some(Self::CaptureCreate),
            "mos.capture.to_task" => Some(Self::CaptureToTask),
            "mos.task.create" => Some(Self::TaskCreate),
            "mos.task.set_state" => Some(Self::TaskSetState),
            "mos.task.set_project" => Some(Self::TaskSetProject),
            "mos.project.create" => Some(Self::ProjectCreate),
            "mos.resource.create" => Some(Self::ResourceCreate),
            "mos.reminder.create" => Some(Self::ReminderCreate),
            "mos.reminder.resolve" => Some(Self::ReminderResolve),
            "mos.time.start" => Some(Self::TimeStart),
            "mos.time.stop" => Some(Self::TimeStop),
            "mos.time.record" => Some(Self::TimeRecord),
            "mos.day.start" => Some(Self::DayStart),
            "mos.day.add_objective" => Some(Self::DayAddObjective),
            "mos.day.set_objective" => Some(Self::DaySetObjective),
            "mos.day.set_main" => Some(Self::DaySetMain),
            "mos.day.end" => Some(Self::DayEnd),
            "m-finance.create_bill" => Some(Self::MFinanceCreateBill),
            _ => None,
        }
    }

    /// Liga a acao a funcao ja declarada em `functions.rs`, que e onde risco e
    /// confirmacao moram. Duas listas de risco seriam duas fontes de verdade.
    pub fn function_id(self) -> &'static str {
        match self {
            Self::CaptureCreate => "capture.create",
            Self::CaptureToTask => "task.create_from_capture",
            Self::TaskCreate => "task.create",
            Self::TaskSetState => "task.set_state",
            Self::TaskSetProject => "task.set_project",
            Self::ProjectCreate => "project.create",
            Self::ResourceCreate => "resource.create",
            Self::ReminderCreate => "attention.create_reminder",
            Self::ReminderResolve => "attention.resolve_reminder",
            Self::TimeStart => "time.start",
            Self::TimeStop => "time.stop",
            Self::TimeRecord => "time.record",
            Self::DayStart => "daily.start_day",
            Self::DayAddObjective => "daily.add_objective",
            Self::DaySetObjective => "daily.set_objective_status",
            Self::DaySetMain => "daily.set_main",
            Self::DayEnd => "daily.end_day",
            Self::MFinanceCreateBill => "m-finance.create_bill",
        }
    }

    pub fn all() -> [ActionKind; 18] {
        [
            Self::CaptureCreate,
            Self::CaptureToTask,
            Self::TaskCreate,
            Self::TaskSetState,
            Self::TaskSetProject,
            Self::ProjectCreate,
            Self::ResourceCreate,
            Self::ReminderCreate,
            Self::ReminderResolve,
            Self::TimeStart,
            Self::TimeStop,
            Self::TimeRecord,
            Self::DayStart,
            Self::DayAddObjective,
            Self::DaySetObjective,
            Self::DaySetMain,
            Self::DayEnd,
            Self::MFinanceCreateBill,
        ]
    }

    /// Assinatura em uma linha, para o prompt. Curta de proposito: o catalogo
    /// inteiro desce a cada mensagem, e cada palavra aqui e token gasto em toda
    /// conversa.
    pub fn signature(self) -> &'static str {
        match self {
            Self::CaptureCreate => "{ content }",
            Self::CaptureToTask => "{ capture, title?, project? }",
            Self::TaskCreate => "{ title, description?, project? }",
            Self::TaskSetState => "{ task, state: inbox|backlog|planned|doing|review|done }",
            Self::TaskSetProject => "{ task, project }",
            Self::ProjectCreate => "{ name, description? }",
            Self::ResourceCreate => "{ kind: site|library|image|note, title, url?, note? }",
            // `at` e `when` sao alternativos, e os dois existem por motivos
            // diferentes. `at` e para quando o modelo consegue fazer a conta
            // sozinho; `when` e para quando ele nao consegue — e nesse caso
            // quem resolve a frase e o M/OS, com o mesmo leitor de datas
            // faladas que a voz usa. Sem `when`, "sexta que vem" viraria uma
            // data inventada com cara de certa.
            Self::ReminderCreate => {
                "{ title, at?: AAAA-MM-DDTHH:MM, when?: \"hoje 20:30\", body?, taskRef?, projectRef?, captureRef? }"
            }
            Self::ReminderResolve => "{ reminder, state: done|cancelled }",
            Self::TimeStart => "{ project, activity?, description? }",
            Self::TimeStop => "{ }",
            // Em MINUTOS, e nao "1h30": tres jeitos de escrever a mesma duracao
            // dao tres jeitos de errar um quarto dela, e o erro sai na fatura.
            Self::TimeRecord => "{ project, minutes, day?: AAAA-MM-DD, activity?, description? }",
            // `mainRef` e `taskRef` existem para a conclusao automatica: um
            // objetivo que E uma Task fecha junto com ela, e um objetivo de
            // texto solto nunca fecha sozinho. Sem o vinculo, um dia montado
            // pelo Hermes teria de ser conferido a mao o dia inteiro.
            Self::DayStart => "{ main, mainRef?: id da Task/Project, secondaries?: [\"...\"], note? }",
            Self::DayAddObjective => "{ title, priority?: main|secondary, taskRef?, projectRef? }",
            Self::DaySetObjective => {
                "{ objective, status: completed|carried_over|dropped|pending }"
            }
            Self::DaySetMain => "{ objective }",
            Self::DayEnd => "{ mood?: productive|normal|blocked, summary? }",
            Self::MFinanceCreateBill => "{ amountCents, description, dueDay?: 1-31, isRecurring }",
        }
    }
}

/// As atividades que uma sessao pode ter. Espelha `ActivityType` do dominio.
const ACTIVITIES: &str = "drawing|detailing|revision|meeting|study|other";

/// A entidade a que um Reminder se prende, como o modelo a citou.
///
/// # Por que um alvo so, e por que ele nao e uma lista
///
/// `ReminderTarget` e um enum de um braco so por decisao registrada: a ADR-012
/// recusou tabela generica de arestas, e o preco aceito foi este. A proposta,
/// porem, chega quase sempre com dois — "lembrete da task X, do projeto Y" — e
/// recusar por isso devolveria erro para uma frase perfeitamente normal.
///
/// A saida e a especificidade: entre Task e Project, o vinculo util e a Task,
/// porque e nela que o trabalho esta e e por ela que se chega ao Project.
/// Escolher em silencio seria adivinhar; por isso o preview NOMEIA o alvo
/// escolhido, e quem le o cartao ve a decisao antes de autorizar.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetRef {
    /// `task`, `project`, `capture`, `resource` ou `meeting`.
    pub kind: String,
    /// Id ou titulo, como o modelo escreveu.
    pub reference: String,
}

/// Argumentos ja validados. Sair do JSON solto o quanto antes e o que impede um
/// campo inventado pelo modelo de viajar ate a camada de servico.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "action", content = "args")]
pub enum ActionArgs {
    CaptureCreate {
        content: String,
    },
    CaptureToTask {
        /// Id ou trecho do conteudo. Resolvido na execucao.
        capture: String,
        /// Vazio significa "use o conteudo da Capture".
        title: String,
        project: Option<String>,
    },
    TaskCreate {
        title: String,
        description: String,
        /// Nome do Project, como o usuario fala. Resolvido na execucao.
        project: Option<String>,
    },
    TaskSetState {
        /// Id ou titulo da Task, como o usuario fala. Resolvido na execucao.
        task: String,
        state: String,
    },
    TaskSetProject {
        task: String,
        project: String,
    },
    ProjectCreate {
        name: String,
        description: String,
    },
    ResourceCreate {
        kind: String,
        title: String,
        url: String,
        note: String,
    },
    ReminderCreate {
        title: String,
        body: String,
        /// O instante JA RESOLVIDO, em RFC3339 com o offset de quem falou.
        ///
        /// Resolvido na leitura da proposta, e nao na execucao, por causa do
        /// preview: o cartao precisa dizer "hoje às 20:30" para o usuário
        /// conferir ANTES de autorizar. Guardar a frase crua e resolver depois
        /// mostraria no cartao exatamente o texto que se quer verificar.
        at: String,
        /// A frase como foi dita, quando o instante veio de `when`. Vazia
        /// quando o modelo mandou `at` pronto.
        when_raw: String,
        /// A que entidade o lembrete se prende. Resolvido na execucao.
        target: Option<TargetRef>,
    },
    ReminderResolve {
        /// Id ou titulo do lembrete.
        reminder: String,
        /// `done` ou `cancelled`.
        state: String,
    },
    TimeStart {
        /// Nome do Project, como o usuario fala. Resolvido na execucao.
        project: String,
        activity: String,
        description: String,
    },
    TimeStop,
    TimeRecord {
        project: String,
        /// Duracao em minutos. Validada na entrada: ver `parse_action`.
        minutes: i64,
        /// `AAAA-MM-DD`. Vazio significa hoje.
        day: String,
        activity: String,
        description: String,
    },
    DayStart {
        main: String,
        /// Id ou titulo da Task/Project que o objetivo principal E. Resolvido
        /// na execucao. Vazio significa intencao livre.
        main_ref: String,
        secondaries: Vec<String>,
        /// A justificativa curta — "voce tem duas entregas hoje e uma reuniao
        /// as 15h". NAO e raciocinio: o §7 do pedido e explicito em nao guardar
        /// chain-of-thought, e o dominio ainda corta o texto por tamanho.
        note: String,
    },
    DayAddObjective {
        title: String,
        /// `main` ou `secondary`.
        priority: String,
        /// Id ou titulo da entidade que o objetivo E. Um so: Task ganha de
        /// Project pela mesma regra de especificidade do `ReminderCreate`.
        link: Option<TargetRef>,
    },
    DaySetObjective {
        /// Id ou titulo do objetivo.
        objective: String,
        status: String,
    },
    DaySetMain {
        objective: String,
    },
    DayEnd {
        mood: String,
        summary: String,
    },
    MFinanceCreateBill {
        /// Centavos. Sempre positivo — zero ou negativo nao e uma conta.
        amount_cents: i64,
        description: String,
        /// Dia do mes, 1-31. Ausente quando a conta nao tem vencimento fixo.
        due_day: Option<u8>,
        is_recurring: bool,
    },
}

impl ActionArgs {
    pub fn kind(&self) -> ActionKind {
        match self {
            Self::CaptureCreate { .. } => ActionKind::CaptureCreate,
            Self::CaptureToTask { .. } => ActionKind::CaptureToTask,
            Self::TaskCreate { .. } => ActionKind::TaskCreate,
            Self::TaskSetState { .. } => ActionKind::TaskSetState,
            Self::TaskSetProject { .. } => ActionKind::TaskSetProject,
            Self::ProjectCreate { .. } => ActionKind::ProjectCreate,
            Self::ResourceCreate { .. } => ActionKind::ResourceCreate,
            Self::ReminderCreate { .. } => ActionKind::ReminderCreate,
            Self::ReminderResolve { .. } => ActionKind::ReminderResolve,
            Self::TimeStart { .. } => ActionKind::TimeStart,
            Self::TimeStop => ActionKind::TimeStop,
            Self::TimeRecord { .. } => ActionKind::TimeRecord,
            Self::DayStart { .. } => ActionKind::DayStart,
            Self::DayAddObjective { .. } => ActionKind::DayAddObjective,
            Self::DaySetObjective { .. } => ActionKind::DaySetObjective,
            Self::DaySetMain { .. } => ActionKind::DaySetMain,
            Self::DayEnd { .. } => ActionKind::DayEnd,
            Self::MFinanceCreateBill { .. } => ActionKind::MFinanceCreateBill,
        }
    }
}

fn text(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

fn required(value: &serde_json::Value, key: &str, action: ActionKind) -> Result<String, CoreError> {
    let found = text(value, key);
    if found.is_empty() {
        return Err(CoreError::new(
            ErrorCode::InvalidInput,
            format!("A proposta de `{}` veio sem `{key}`.", action.as_str()),
            false,
        ));
    }
    Ok(found)
}

/// Le uma proposta contra o relogio de quem esta na frente da tela.
///
/// O `now_local` existe por uma acao so — `mos.reminder.create` —, e nao ha
/// como evitar: "hoje às 20:30" nao e uma data ate alguem dizer que dia e hoje
/// e em que fuso. A mesma lei que o `voice_when` obedece (`CORE-FOUNDATION.md`
/// §5): o fuso entra como parametro, e o banco continua guardando UTC.
pub fn parse_action(raw: &str) -> Result<ActionArgs, CoreError> {
    parse_action_at(raw, OffsetDateTime::now_utc())
}

/// Le uma proposta e devolve argumentos validados.
///
/// Argumento fora do esquema faz a proposta ser RECUSADA, e nao corrigida.
/// Corrigir seria o M/OS adivinhando o que o modelo quis dizer — e adivinhar e
/// exatamente o que o preview existe para evitar.
pub fn parse_action_at(raw: &str, now_local: OffsetDateTime) -> Result<ActionArgs, CoreError> {
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|error| {
        CoreError::new(
            ErrorCode::InvalidInput,
            format!("A proposta nao e um JSON valido: {error}"),
            false,
        )
    })?;

    let name = text(&value, "action");
    let kind = ActionKind::parse(&name).ok_or_else(|| {
        CoreError::new(
            ErrorCode::InvalidInput,
            if name.is_empty() {
                "A proposta veio sem `action`.".to_owned()
            } else {
                format!("`{name}` nao e uma acao que o M/OS conhece.")
            },
            false,
        )
    })?;

    let args = value
        .get("args")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    Ok(match kind {
        ActionKind::CaptureCreate => ActionArgs::CaptureCreate {
            content: required(&args, "content", kind)?,
        },
        ActionKind::CaptureToTask => ActionArgs::CaptureToTask {
            capture: required(&args, "capture", kind)?,
            title: text(&args, "title"),
            project: {
                let project = text(&args, "project");
                (!project.is_empty()).then_some(project)
            },
        },
        ActionKind::TaskSetProject => ActionArgs::TaskSetProject {
            task: required(&args, "task", kind)?,
            project: required(&args, "project", kind)?,
        },
        ActionKind::ReminderCreate => {
            let (at, when_raw) = reminder_instant(&args, now_local)?;
            ActionArgs::ReminderCreate {
                title: required(&args, "title", kind)?,
                body: text(&args, "body"),
                at,
                when_raw,
                target: target_of(&args),
            }
        }
        ActionKind::ReminderResolve => {
            let state = required(&args, "state", kind)?;
            if !matches!(state.as_str(), "done" | "cancelled") {
                return Err(CoreError::new(
                    ErrorCode::InvalidInput,
                    format!("`{state}` nao e um desfecho de lembrete. Use `done` ou `cancelled`."),
                    false,
                ));
            }
            ActionArgs::ReminderResolve {
                reminder: required(&args, "reminder", kind)?,
                state,
            }
        }
        ActionKind::TaskCreate => ActionArgs::TaskCreate {
            title: required(&args, "title", kind)?,
            description: text(&args, "description"),
            project: {
                let project = text(&args, "project");
                (!project.is_empty()).then_some(project)
            },
        },
        ActionKind::TaskSetState => {
            let state = required(&args, "state", kind)?;
            crate::TaskState::parse(&state).map_err(|_| {
                CoreError::new(
                    ErrorCode::InvalidInput,
                    format!("`{state}` nao e um estado de Task."),
                    false,
                )
            })?;
            ActionArgs::TaskSetState {
                task: required(&args, "task", kind)?,
                state,
            }
        }
        ActionKind::ProjectCreate => ActionArgs::ProjectCreate {
            name: required(&args, "name", kind)?,
            description: text(&args, "description"),
        },
        ActionKind::ResourceCreate => {
            let kind_value = required(&args, "kind", kind)?;
            crate::ResourceKind::parse(&kind_value).map_err(|_| {
                CoreError::new(
                    ErrorCode::InvalidInput,
                    format!("`{kind_value}` nao e um tipo de Resource."),
                    false,
                )
            })?;
            ActionArgs::ResourceCreate {
                kind: kind_value,
                title: required(&args, "title", kind)?,
                url: text(&args, "url"),
                note: text(&args, "note"),
            }
        }
        ActionKind::TimeStart => ActionArgs::TimeStart {
            project: required(&args, "project", kind)?,
            activity: activity_of(&args)?,
            description: text(&args, "description"),
        },
        ActionKind::TimeStop => ActionArgs::TimeStop,
        ActionKind::TimeRecord => ActionArgs::TimeRecord {
            project: required(&args, "project", kind)?,
            minutes: minutes_of(&args)?,
            day: day_of(&args)?,
            activity: activity_of(&args)?,
            description: text(&args, "description"),
        },
        ActionKind::DayStart => ActionArgs::DayStart {
            main: required(&args, "main", kind)?,
            main_ref: text(&args, "mainRef"),
            secondaries: args
                .get("secondaries")
                .and_then(serde_json::Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(|item| item.trim().to_owned())
                        .filter(|item| !item.is_empty())
                        .collect()
                })
                .unwrap_or_default(),
            note: text(&args, "note"),
        },
        ActionKind::DayAddObjective => {
            let priority = match text(&args, "priority").as_str() {
                "" => "secondary".to_owned(),
                value => {
                    crate::ObjectivePriority::parse(value).map_err(|_| {
                        CoreError::new(
                            ErrorCode::InvalidInput,
                            format!("`{value}` nao e uma prioridade. Use `main` ou `secondary`."),
                            false,
                        )
                    })?;
                    value.to_owned()
                }
            };
            ActionArgs::DayAddObjective {
                title: required(&args, "title", kind)?,
                priority,
                link: target_of(&args),
            }
        }
        ActionKind::DaySetObjective => {
            let status = required(&args, "status", kind)?;
            crate::ObjectiveStatus::parse(&status).map_err(|_| {
                CoreError::new(
                    ErrorCode::InvalidInput,
                    format!(
                        "`{status}` nao e um desfecho de objetivo. Use `completed`,                          `carried_over`, `dropped` ou `pending`."
                    ),
                    false,
                )
            })?;
            ActionArgs::DaySetObjective {
                objective: required(&args, "objective", kind)?,
                status,
            }
        }
        ActionKind::DaySetMain => ActionArgs::DaySetMain {
            objective: required(&args, "objective", kind)?,
        },
        ActionKind::DayEnd => {
            let mood = text(&args, "mood");
            if !mood.is_empty() {
                crate::DayMood::parse(&mood).map_err(|_| {
                    CoreError::new(
                        ErrorCode::InvalidInput,
                        format!(
                            "`{mood}` nao e um humor de dia. Use `productive`, `normal`                              ou `blocked`."
                        ),
                        false,
                    )
                })?;
            }
            ActionArgs::DayEnd {
                mood,
                summary: text(&args, "summary"),
            }
        }
        ActionKind::MFinanceCreateBill => {
            let amount_cents = args
                .get("amountCents")
                .and_then(serde_json::Value::as_i64)
                .filter(|value| *value > 0)
                .ok_or_else(|| {
                    CoreError::new(
                        ErrorCode::InvalidInput,
                        "A proposta de `m-finance.create_bill` veio sem `amountCents` valido."
                            .to_owned(),
                        false,
                    )
                })?;
            let due_day = args
                .get("dueDay")
                .and_then(serde_json::Value::as_u64)
                .map(|value| value as u8);
            if let Some(day) = due_day {
                if !(1..=31).contains(&day) {
                    return Err(CoreError::new(
                        ErrorCode::InvalidInput,
                        format!("`{day}` nao e um dia valido de vencimento."),
                        false,
                    ));
                }
            }
            ActionArgs::MFinanceCreateBill {
                amount_cents,
                description: required(&args, "description", kind)?,
                due_day,
                is_recurring: args
                    .get("isRecurring")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            }
        }
    })
}

/// Quanto no passado uma proposta de lembrete ainda vale.
///
/// Espelha o `CREATION_GRACE` do dominio, e existe aqui pelo mesmo motivo que o
/// teto de 24h existe no `minutes_of`: recusar na LEITURA devolve o erro para o
/// cartao, onde ele e uma frase; recusar so na execucao devolveria o erro para
/// o recibo, depois de o usuario ja ter autorizado.
const REMINDER_GRACE: time::Duration = time::Duration::minutes(1);

/// O instante do lembrete, e a frase que o produziu.
///
/// Dois caminhos, e a ordem entre eles importa: `at` ganha de `when` porque um
/// modelo que conseguiu fazer a conta ja aplicou o que sabe: mandar os dois e
/// dizer a mesma coisa duas vezes, e reinterpretar a frase por cima da data
/// pronta poderia trocar uma pela outra.
fn reminder_instant(
    args: &serde_json::Value,
    now_local: OffsetDateTime,
) -> Result<(String, String), CoreError> {
    let recusa = |detalhe: &str| {
        CoreError::new(
            ErrorCode::InvalidInput,
            format!("A proposta de `mos.reminder.create` {detalhe}"),
            false,
        )
    };

    let at = text(args, "at");
    let when = text(args, "when");

    let (instant, when_raw) = if !at.is_empty() {
        (
            parse_local_moment(&at, now_local)
                .ok_or_else(|| recusa(&format!("veio com `at` ilegivel: \"{at}\".")))?,
            String::new(),
        )
    } else if !when.is_empty() {
        // O mesmo leitor de datas faladas que a voz usa. Duas gramaticas de
        // "sexta que vem" no mesmo app dariam duas sextas diferentes.
        let resolved = crate::resolve_when(&when, now_local)
            .ok_or_else(|| recusa(&format!("nao consegui entender \"{when}\" como data.")))?;
        (resolved.instant, resolved.raw)
    } else {
        return Err(recusa("veio sem `at` nem `when`. Um lembrete precisa de hora."));
    };

    if instant < now_local - REMINDER_GRACE {
        return Err(recusa(
            "aponta para um instante que ja passou. Nao da para ser lembrado de algo no passado.",
        ));
    }

    let formatted = instant
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|error| recusa(&format!("produziu uma data que nao sei escrever: {error}.")))?;
    Ok((formatted, when_raw))
}

/// Le uma data que pode ou nao trazer fuso.
///
/// Sem fuso, o instante herda o de quem falou. E a diferenca entre um lembrete
/// as oito e meia da noite e um lembrete as cinco e meia da tarde — que e o que
/// aconteceria lendo `2026-08-20T20:30` como UTC no Brasil.
fn parse_local_moment(value: &str, now_local: OffsetDateTime) -> Option<OffsetDateTime> {
    // "2026-08-20 20:30" e uma escrita natural, e recusa-la seria recusar por
    // causa de um espaco.
    let normalizado = value.trim().replacen(' ', "T", 1);

    if let Ok(instant) =
        OffsetDateTime::parse(&normalizado, &time::format_description::well_known::Rfc3339)
    {
        return Some(instant.to_offset(now_local.offset()));
    }

    const COM_SEGUNDOS: &[time::format_description::FormatItem<'_>] =
        time::macros::format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]");
    const SEM_SEGUNDOS: &[time::format_description::FormatItem<'_>] =
        time::macros::format_description!("[year]-[month]-[day]T[hour]:[minute]");

    for formato in [COM_SEGUNDOS, SEM_SEGUNDOS] {
        if let Ok(naive) = time::PrimitiveDateTime::parse(&normalizado, formato) {
            return Some(naive.assume_offset(now_local.offset()));
        }
    }
    None
}

/// O alvo do lembrete, do mais especifico para o mais geral.
///
/// A ordem e a decisao: entre a Task e o Project, o vinculo util e a Task —
/// dela se chega ao Project, e do Project nao se chega a Task. Ver [`TargetRef`].
fn target_of(args: &serde_json::Value) -> Option<TargetRef> {
    [
        ("taskRef", "task"),
        ("captureRef", "capture"),
        ("resourceRef", "resource"),
        ("meetingRef", "meeting"),
        ("projectRef", "project"),
    ]
    .into_iter()
    .find_map(|(campo, kind)| {
        let reference = text(args, campo);
        (!reference.is_empty()).then(|| TargetRef {
            kind: kind.to_owned(),
            reference,
        })
    })
}

/// Duracao em minutos, dentro do que um dia comporta.
///
/// O teto de 24h nao e paranoia: e a unica defesa contra o modo de falha mais
/// caro deste catalogo. "Duas horas" ouvido como "duzentas" viraria 12.000
/// minutos, e o numero so seria notado na hora de faturar. Recusar aqui devolve
/// o erro para a conversa, onde ele ainda custa uma frase.
fn minutes_of(args: &serde_json::Value) -> Result<i64, CoreError> {
    let raw = args.get("minutes").and_then(serde_json::Value::as_i64);
    let refuse = |detail: &str| {
        CoreError::new(
            ErrorCode::InvalidInput,
            format!("A proposta de `mos.time.record` {detail}"),
            false,
        )
    };
    match raw {
        None => Err(refuse(
            "veio sem `minutes`, ou com um valor que nao e numero inteiro.",
        )),
        Some(value) if value <= 0 => Err(refuse("veio com duracao zero ou negativa.")),
        Some(value) if value > 24 * 60 => Err(refuse(
            "veio com mais de 24 horas numa sessao so. Se for isso mesmo, lance dia a dia.",
        )),
        Some(value) => Ok(value),
    }
}

/// `AAAA-MM-DD`, ou vazio para hoje.
///
/// A forma e conferida aqui e o instante e montado na execucao: um dia mal
/// escrito viraria hora lancada no mes errado, e o total do mes fecharia errado
/// sem nada parecer quebrado.
fn day_of(args: &serde_json::Value) -> Result<String, CoreError> {
    let day = text(args, "day");
    if day.is_empty() {
        return Ok(day);
    }
    let shaped = day.len() == 10
        && day.as_bytes()[4] == b'-'
        && day.as_bytes()[7] == b'-'
        && day
            .bytes()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit());
    if !shaped {
        return Err(CoreError::new(
            ErrorCode::InvalidInput,
            format!("`{day}` nao e uma data no formato AAAA-MM-DD."),
            false,
        ));
    }
    Ok(day)
}

/// Atividade do vocabulario fixo. Vazio vira `other` na execucao.
fn activity_of(args: &serde_json::Value) -> Result<String, CoreError> {
    let activity = text(args, "activity");
    if activity.is_empty() || ACTIVITIES.split('|').any(|known| known == activity) {
        return Ok(activity);
    }
    Err(CoreError::new(
        ErrorCode::InvalidInput,
        format!("`{activity}` nao e uma atividade. Use uma de: {ACTIVITIES}."),
        false,
    ))
}

/// O que a tela mostra antes de executar.
///
/// O preview nao e cerimonia proporcional ao risco — ele e a EXPLICACAO do que
/// o Hermes entendeu, e por isso aparece para toda proposta, inclusive as de
/// risco baixo. Quem clica "Criar Task" na interface escolheu; quem falou uma
/// frase pode ter sido mal interpretado, e `UX-PRINCIPLES` §19 pede que o
/// sistema mostre o que compreendeu. O risco decide o PESO da confirmacao, e
/// nao a existencia dela.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionLine {
    pub label: String,
    pub value: String,
}

fn line(label: &str, value: &str) -> ActionLine {
    ActionLine {
        label: label.to_owned(),
        value: value.to_owned(),
    }
}

/// R$ a partir de centavos. Nao e formatacao de moeda completa (sem milhar) —
/// e so o preview; o M-Finance formata de verdade na tela dele.
/// Formata centavos no padrao brasileiro: `R$ 1.234,56`.
///
/// Antes saia `R$ 1234.56` — ponto no decimal, sem separador de milhar, o
/// formato de outra lingua num app inteiro em portugues. Num card que pede
/// autorizacao para mover dinheiro isso nao e cosmetico: ler `R$ 1234.56` de
/// relance como "mil duzentos e trinta e quatro" ou como "mil, duzentos e
/// trinta e quatro reais e cinquenta e seis" muda o que a pessoa acha que esta
/// autorizando. O resto do M/OS ja usa este padrao (`TempoShared.tsx`).
///
/// Feito com inteiros de proposito: `f64` para dinheiro arredonda onde nao
/// deve, e o valor exibido aqui e o valor que vai ser gravado.
fn format_cents(cents: i64) -> String {
    let absoluto = cents.unsigned_abs();
    let digitos = (absoluto / 100).to_string();

    let mut inteiro = String::with_capacity(digitos.len() + digitos.len() / 3);
    for (indice, digito) in digitos.char_indices() {
        if indice > 0 && (digitos.len() - indice) % 3 == 0 {
            inteiro.push('.');
        }
        inteiro.push(digito);
    }

    format!(
        "{}R$ {inteiro},{:02}",
        if cents < 0 { "-" } else { "" },
        absoluto % 100,
    )
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionPreview {
    pub action: String,
    /// Rotulo curto: "CRIAR TASK".
    pub title: String,
    /// Uma linha por campo, na ordem em que devem ser lidas.
    pub lines: Vec<ActionLine>,
    pub risk: FunctionRisk,
    pub confirmation: FunctionConfirmation,
}

pub fn preview_of(args: &ActionArgs) -> ActionPreview {
    let kind = args.kind();
    let definition = crate::function_registry()
        .into_iter()
        .find(|entry| entry.id == kind.function_id());

    let (title, lines) = match args {
        ActionArgs::CaptureCreate { content } => ("CRIAR CAPTURE", vec![line("Conteúdo", content)]),
        ActionArgs::CaptureToTask {
            capture,
            title,
            project,
        } => {
            let mut lines = vec![line("Capture", capture)];
            if !title.is_empty() {
                lines.push(line("Título da Task", title));
            }
            if let Some(project) = project {
                lines.push(line("Project", project));
            }
            ("CONVERTER CAPTURE EM TASK", lines)
        }
        ActionArgs::TaskSetProject { task, project } => (
            "MOVER TASK DE PROJECT",
            vec![line("Task", task), line("Para o Project", project)],
        ),
        ActionArgs::ReminderCreate {
            title,
            body,
            at,
            when_raw,
            target,
        } => {
            let quando = crate::parse_moment(at)
                .map(crate::spoken_moment)
                // O `at` foi escrito por `reminder_instant`, entao chegar aqui
                // ilegivel seria bug nosso. Mostrar o cru e melhor que esconder:
                // um cartao sem hora nao pode ser autorizado com consciencia.
                .unwrap_or_else(|_| at.clone());
            let mut lines = vec![line("Lembrar", title), line("Quando", &quando)];
            if !when_raw.is_empty() {
                // A frase original entra no cartao porque o instante e uma
                // INTERPRETACAO dela. Ver as duas lado a lado e o que permite
                // pegar "sexta" entendida como a sexta errada.
                lines.push(line("Você disse", when_raw));
            }
            if let Some(target) = target {
                lines.push(line(
                    match target.kind.as_str() {
                        "task" => "Vinculado à Task",
                        "project" => "Vinculado ao Project",
                        "capture" => "Vinculado à Capture",
                        "resource" => "Vinculado ao Resource",
                        _ => "Vinculado a",
                    },
                    &target.reference,
                ));
            }
            if !body.is_empty() {
                lines.push(line("Detalhe", body));
            }
            ("CRIAR LEMBRETE", lines)
        }
        ActionArgs::ReminderResolve { reminder, state } => (
            "RESOLVER LEMBRETE",
            vec![
                line("Lembrete", reminder),
                line(
                    "Desfecho",
                    if state == "done" {
                        "concluído"
                    } else {
                        "cancelado"
                    },
                ),
            ],
        ),
        ActionArgs::TaskCreate {
            title,
            description,
            project,
        } => {
            let mut lines = vec![line("Título", title)];
            if let Some(project) = project {
                lines.push(line("Project", project));
            }
            if !description.is_empty() {
                lines.push(line("Descrição", description));
            }
            ("CRIAR TASK", lines)
        }
        ActionArgs::TaskSetState { task, state } => {
            ("MOVER TASK", vec![line("Task", task), line("Para", state)])
        }
        // Os cinco cartoes do dia. O `note` entra no preview de proposito: e a
        // justificativa que o Hermes deu, e autorizar um dia montado por outro
        // sem ver o porque seria assinar em branco.
        ActionArgs::DayStart {
            main,
            main_ref,
            secondaries,
            note,
        } => {
            let mut lines = vec![line("Principal", main)];
            if !main_ref.is_empty() {
                lines.push(line("Vinculado a", main_ref));
            }
            for (numero, titulo) in secondaries.iter().enumerate() {
                lines.push(line(&format!("Secundário {}", numero + 1), titulo));
            }
            if !note.is_empty() {
                lines.push(line("Por quê", note));
            }
            ("INICIAR O DIA", lines)
        }
        ActionArgs::DayAddObjective {
            title,
            priority,
            link,
        } => {
            let mut lines = vec![
                line("Objetivo", title),
                line(
                    "Peso",
                    if priority == "main" { "principal" } else { "secundário" },
                ),
            ];
            if let Some(link) = link {
                lines.push(line("Vinculado a", &format!("{} {}", link.kind, link.reference)));
            }
            ("ADICIONAR OBJETIVO DO DIA", lines)
        }
        ActionArgs::DaySetObjective { objective, status } => (
            "RESOLVER OBJETIVO DO DIA",
            vec![
                line("Objetivo", objective),
                line(
                    "Desfecho",
                    match status.as_str() {
                        "completed" => "concluído",
                        "carried_over" => "levado para amanhã",
                        "dropped" => "abandonado",
                        _ => "de volta a pendente",
                    },
                ),
            ],
        ),
        ActionArgs::DaySetMain { objective } => (
            "DEFINIR O OBJETIVO PRINCIPAL",
            vec![line("Objetivo", objective)],
        ),
        ActionArgs::DayEnd { mood, summary } => {
            let mut lines = Vec::new();
            if !mood.is_empty() {
                lines.push(line(
                    "Como foi",
                    match mood.as_str() {
                        "productive" => "dia produtivo",
                        "blocked" => "dia travado",
                        _ => "dia normal",
                    },
                ));
            }
            if !summary.is_empty() {
                lines.push(line("Resumo", summary));
            }
            // Sem linha nenhuma o cartao ainda diz o que faz pelo titulo, e o
            // que ele faz — resolver os pendentes e fechar o dia — nao depende
            // de campo nenhum.
            ("ENCERRAR O DIA", lines)
        }
        ActionArgs::ProjectCreate { name, description } => {
            let mut lines = vec![line("Nome", name)];
            if !description.is_empty() {
                lines.push(line("Descrição", description));
            }
            ("CRIAR PROJECT", lines)
        }
        ActionArgs::ResourceCreate {
            kind: resource_kind,
            title,
            url,
            note,
        } => {
            let mut lines = vec![line("Título", title), line("Tipo", resource_kind)];
            if !url.is_empty() {
                lines.push(line("URL", url));
            }
            if !note.is_empty() {
                lines.push(line("Nota", note));
            }
            ("SALVAR RESOURCE", lines)
        }
        ActionArgs::TimeStart {
            project,
            activity,
            description,
        } => {
            let mut lines = vec![line("Project", project)];
            if !activity.is_empty() {
                lines.push(line("Atividade", activity));
            }
            if !description.is_empty() {
                lines.push(line("Descrição", description));
            }
            ("INICIAR CRONÔMETRO", lines)
        }
        ActionArgs::TimeStop => (
            "ENCERRAR CRONÔMETRO",
            vec![line("Sessão", "grava o tempo contado até agora")],
        ),
        ActionArgs::TimeRecord {
            project,
            minutes,
            day,
            activity,
            description,
        } => {
            // A duracao aparece nas DUAS formas — "2h30" e "150 min" — e isso e
            // proposital. Este e o campo em que um engano do modelo vira erro de
            // fatura, e ler o mesmo numero escrito de dois jeitos e o que faz o
            // absurdo saltar: "12h00" ao lado de "720 min" e obviamente errado
            // para quem trabalhou duas horas.
            let mut lines = vec![
                line("Project", project),
                line(
                    "Duração",
                    &format!("{}h{:02} · {minutes} min", minutes / 60, minutes % 60),
                ),
                line("Dia", if day.is_empty() { "hoje" } else { day }),
            ];
            if !activity.is_empty() {
                lines.push(line("Atividade", activity));
            }
            if !description.is_empty() {
                lines.push(line("Descrição", description));
            }
            ("LANÇAR TEMPO", lines)
        }
        ActionArgs::MFinanceCreateBill {
            amount_cents,
            description,
            due_day,
            is_recurring,
        } => {
            let mut lines = vec![
                line("Valor", &format_cents(*amount_cents)),
                line("Descrição", description),
            ];
            if let Some(day) = due_day {
                lines.push(line("Vencimento", &format!("dia {day}")));
            }
            lines.push(line("Recorrente", if *is_recurring { "sim" } else { "não" }));
            ("CRIAR CONTA NO M-FINANCE", lines)
        }
    };

    ActionPreview {
        action: kind.as_str().to_owned(),
        title: title.to_owned(),
        lines,
        // Sem funcao declarada, o default e o mais cauteloso possivel. Uma acao
        // que escapou do registro nao pode herdar risco baixo por omissao.
        risk: definition
            .as_ref()
            .map(|entry| entry.risk)
            .unwrap_or(FunctionRisk::High),
        confirmation: definition
            .map(|entry| entry.confirmation)
            .unwrap_or(FunctionConfirmation::Explicit),
    }
}

/// O que a acao fez, e o caminho de volta.
///
/// A execucao devolvia so uma frase, e a identidade do que nasceu se perdia
/// ali. Sem ela o recibo consegue dizer "Task criada" e nao consegue oferecer o
/// desfazer — que era a metade da cerimonia que faltava (`SPEC-ACOES` fase 2).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionEffect {
    /// A frase do recibo, e tambem o desfecho gravado na conversa.
    pub message: String,
    /// Ausente quando nao ha inverso honesto.
    pub undo: Option<UndoStep>,
    /// O que a acao tocou, ja resolvido em id.
    ///
    /// A proposta guarda o que o modelo ESCREVEU — "a task do Victor" — e isso
    /// nao identifica nada depois que o titulo muda. Sem esta lista, o registro
    /// da conversa diz que algo foi feito e nao diz sobre o que, que e
    /// exatamente a pergunta que uma auditoria faz.
    #[serde(default)]
    pub entities: Vec<TouchedEntity>,
}

impl ActionEffect {
    /// Um efeito sem rastro de entidade. Existe para as acoes que nao tocam
    /// entidade nenhuma — encerrar cronometro, por exemplo, so mexe no que ja
    /// estava correndo.
    pub fn new(message: impl Into<String>, undo: Option<UndoStep>) -> Self {
        Self {
            message: message.into(),
            undo,
            entities: Vec::new(),
        }
    }

    pub fn touching(
        mut self,
        kind: impl Into<String>,
        id: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        self.entities.push(TouchedEntity {
            kind: kind.into(),
            id: id.into(),
            label: label.into(),
        });
        self
    }
}

/// Uma entidade que uma acao do Hermes alcancou.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TouchedEntity {
    pub kind: String,
    pub id: String,
    pub label: String,
}

/// O rastro de uma proposta executada.
///
/// # Por que dentro da parte, e nao numa tabela nova
///
/// O §11 do pedido lista o que uma auditoria precisa: acao, instante, origem,
/// conversa, entidade alvo, antes e depois. Seis desses sete ja existiam na
/// conversa — a proposta guarda a acao crua, a mensagem guarda o instante e o
/// id da conversa, e `source = hermes` e o proprio fato de a parte ser uma
/// proposta. Faltavam **a entidade resolvida e o estado anterior**, e os dois
/// cabem aqui.
///
/// Uma tabela propria custaria migration e uma segunda fonte de verdade sobre o
/// que o Hermes fez. As partes ja sao persistidas como JSON (ADR-025), entao
/// este campo entra sem tocar no esquema — e `Option` com `serde(default)`
/// porque toda proposta gravada antes de hoje continua legivel sem ele.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionAudit {
    /// RFC3339, no instante em que a execucao terminou.
    pub executed_at: String,
    pub entities: Vec<TouchedEntity>,
    /// O estado anterior, quando havia um. E o "antes" da auditoria, e e o
    /// mesmo dado que o desfazer usa — guardar duas versoes dele seria deixar
    /// que divergissem.
    #[serde(default)]
    pub undo: Option<UndoStep>,
}

/// O inverso de uma acao executada.
///
/// Criar se desfaz ARQUIVANDO, e nao apagando. Duas razoes que se somam: a
/// exclusao definitiva recusa o que ainda esta ativo (`ports.rs`), e todo Undo
/// que o M/OS ja oferece e restauracao de estado — nenhum remove. Um desfazer
/// que destroi seria o unico caminho do app sem volta, e ainda por cima o
/// caminho oferecido logo depois de o usuario dizer que errou.
///
/// Mover se desfaz voltando ao estado anterior, que por isso precisa ser lido
/// ANTES da mudanca. Depois nao ha de onde tirar.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "step")]
pub enum UndoStep {
    ArchiveCapture {
        id: String,
    },
    ArchiveTask {
        id: String,
    },
    ArchiveProject {
        id: String,
    },
    ArchiveResource {
        id: String,
    },
    RestoreTaskState {
        id: String,
        state: String,
    },
    /// O Project que a Task tinha antes. `None` significa "nenhum", e por isso
    /// e um passo com campo opcional em vez de dois passos diferentes: tirar a
    /// Task de um Project e uma mudanca tao real quanto move-la para outro, e
    /// desfazer precisa saber devolver ao vazio.
    RestoreTaskProject {
        id: String,
        project_id: Option<String>,
    },
    /// O inverso de cancelar um lembrete que o Hermes criou.
    ///
    /// Cancela — nao apaga (ADR-035). O lembrete cancelado continua no
    /// historico, que e o mesmo destino de um lembrete que a pessoa descartou
    /// na tela.
    CancelReminder {
        id: String,
    },
    /// O inverso de converter uma Capture em Task.
    ///
    /// Dois passos num so, pela mesma razao do `UndoVoiceAction`: a conversao
    /// foi UMA acao. **A Capture volta para a Inbox** em vez de ser arquivada,
    /// porque o que se desfez foi a decisao sobre ela, e o lugar de uma Capture
    /// ainda nao decidida e a Inbox.
    UndoCaptureToTask {
        capture_id: String,
        task_id: String,
    },
    /// Tira do dia um objetivo que o Hermes acabou de acrescentar.
    ///
    /// APAGA, e a excecao a ADR-035 e a mesma que o repositorio ja registra: um
    /// objetivo removido antes de o dia acabar nunca chegou a ser historia.
    /// Arquivar nao serve porque objetivo nao tem `lifecycle_state` — o dia
    /// inteiro e que e o registro, e ele continua de pe.
    RemoveDailyObjective {
        id: String,
    },
    /// O estado que o objetivo tinha antes. Espelha o `RestoreTaskState`, e por
    /// isso precisa ser lido ANTES da mudanca: depois nao ha de onde tirar.
    RestoreObjectiveStatus {
        id: String,
        status: String,
    },
    /// O principal que o dia tinha antes de a promocao acontecer.
    ///
    /// Dois campos porque "nao havia principal" e um estado de verdade: com
    /// `previous_id` ausente, desfazer significa REBAIXAR o promovido, e nao
    /// promover ninguem. Um passo com um campo so nao saberia distinguir isso de
    /// "o principal anterior sumiu".
    RestoreDailyMain {
        previous_id: Option<String>,
        demote_id: String,
    },
    /// Reabre o dia que a acao encerrou.
    ///
    /// E o unico desfazer do dia que devolve exatamente o estado anterior: os
    /// desfechos gravados nos objetivos continuam, e e assim que tem de ser —
    /// concluir tres objetivos e encerrar o dia foram decisoes diferentes, e
    /// so a segunda esta sendo desfeita.
    ReopenDay {
        session_id: String,
    },
    /// Manda a sessao para a lixeira. Soft delete, entao ela continua no banco e
    /// volta pela lixeira do Historico — o desfazer aqui obedece a mesma regra
    /// do resto: some da vista, nao some do registro.
    TrashTimeEntry {
        id: String,
    },
    /// O inverso de aceitar um item de reuniao.
    ///
    /// Tres reversoes num passo so porque a aceitacao foi UMA acao: arquivar a
    /// Task, cancelar o Reminder e devolver o item a `proposed`. Desfazer
    /// parcialmente deixaria um lembrete tocando para uma Task que nao existe
    /// mais na tela.
    ///
    /// Arquiva e cancela — nao apaga (ADR-035). O item volta a ser oferecido, o
    /// que e o ponto: quem desfez provavelmente quer refazer diferente.
    UndoMeetingInsight {
        insight_id: String,
        task_id: String,
        reminder_id: Option<String>,
    },
    /// O inverso de uma acao que a VOZ executou sozinha.
    ///
    /// Tres reversoes num passo so, pela mesma razao do
    /// `UndoMeetingInsight`: a acao foi uma. Desfazer pela metade deixaria um
    /// lembrete tocando para uma Task arquivada.
    ///
    /// **A Capture NAO e apagada nem arquivada — ela volta para a Inbox.** Ela
    /// e a fala, e desfazer a acao nao desfaz o ter falado. O que se desfaz e a
    /// leitura que o M/OS fez dela, e o lugar de uma fala ainda nao decidida e
    /// a Inbox.
    UndoVoiceAction {
        capture_id: String,
        task_id: String,
        reminder_id: Option<String>,
    },
}

/// O contrato que desce no prompt.
///
/// Diz o que existe e como responder. Nao pede que o modelo execute nada, e a
/// frase final e deliberada: sem ela, um modelo prestativo tende a preencher
/// campos que o usuario nao disse.
///
/// `finance_enabled` decide se `m-finance.create_bill` desce no catalogo.
/// Sem a capacidade `can_write` no App M-Finance, o Hermes nunca aprende que
/// a acao existe — a mesma logica que impede a UI de oferecer uma acao que o
/// usuario nao habilitou.
pub fn action_contract(finance_enabled: bool) -> String {
    let catalog = ActionKind::all()
        .iter()
        .filter(|kind| finance_enabled || **kind != ActionKind::MFinanceCreateBill)
        .map(|kind| format!("- {} {}", kind.as_str(), kind.signature()))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "[Ações disponíveis no M/OS]\n\
         Quando o usuário pedir para criar ou mudar algo no M/OS, responda \
         normalmente E inclua um bloco:\n\n\
         ```mos-action\n\
         {{ \"action\": \"...\", \"args\": {{ ... }} }}\n\
         ```\n\n\
         {catalog}\n\n\
         Campos que apontam para uma entidade que já existe — `task`, \
         `project`, `capture`, `reminder`, `taskRef` — aceitam o id mostrado \
         nos candidatos ou o título. **Prefira o id:** título ambíguo faz o \
         M/OS parar e perguntar em vez de agir.\n\n\
         Você não executa nada: o M/OS mostra o que você propôs e o usuário \
         confirma. Não invente valor que o usuário não disse — deixe o campo \
         fora e pergunte, se for essencial. Uma proposta por mensagem.\n\
         [Fim das ações]\n\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(args: &str) -> Result<ActionArgs, CoreError> {
        parse_action(&format!(r#"{{"action":"mos.time.record","args":{args}}}"#))
    }

    #[test]
    fn parses_a_time_record() {
        assert_eq!(
            record(r#"{"project":"Rancho Queimado","minutes":150,"day":"2026-08-14"}"#).unwrap(),
            ActionArgs::TimeRecord {
                project: "Rancho Queimado".into(),
                minutes: 150,
                day: "2026-08-14".into(),
                activity: String::new(),
                description: String::new(),
            }
        );
    }

    /// O modo de falha mais caro do catalogo: "duas horas" ouvido como
    /// "duzentas" viraria 12.000 minutos, e o numero so apareceria na fatura.
    #[test]
    fn a_session_longer_than_a_day_is_refused() {
        let error = record(r#"{"project":"X","minutes":12000}"#).unwrap_err();
        assert!(error.message.contains("24 horas"), "{}", error.message);

        // A borda passa: 24h exatas sao um dia de trabalho absurdo mas possivel,
        // e recusar o legitimo junto com o absurdo ensina a nao usar o recurso.
        assert!(record(r#"{"project":"X","minutes":1440}"#).is_ok());
    }

    #[test]
    fn zero_and_negative_durations_are_refused() {
        assert!(record(r#"{"project":"X","minutes":0}"#).is_err());
        assert!(record(r#"{"project":"X","minutes":-30}"#).is_err());
    }

    /// Sem `minutes` a acao nao tem o que gravar. Um default silencioso aqui
    /// gravaria uma sessao de duracao inventada.
    #[test]
    fn a_record_without_minutes_is_refused() {
        assert!(record(r#"{"project":"X"}"#).is_err());
        // Texto tambem nao serve: `"120"` viraria zero num parse tolerante.
        assert!(record(r#"{"project":"X","minutes":"120"}"#).is_err());
    }

    /// Um dia mal escrito jogaria a hora no mes errado, e o total do mes
    /// fecharia errado sem nada parecer quebrado.
    #[test]
    fn a_malformed_day_is_refused() {
        assert!(record(r#"{"project":"X","minutes":60,"day":"14/08/2026"}"#).is_err());
        assert!(record(r#"{"project":"X","minutes":60,"day":"2026-8-14"}"#).is_err());
        // Ausente e valido, e significa hoje.
        assert!(record(r#"{"project":"X","minutes":60}"#).is_ok());
    }

    #[test]
    fn an_unknown_activity_is_refused() {
        assert!(record(r#"{"project":"X","minutes":60,"activity":"almoco"}"#).is_err());
        assert!(record(r#"{"project":"X","minutes":60,"activity":"drawing"}"#).is_ok());
    }

    /// Encerrar nao tem argumento, e isso precisa continuar aceitando `args`
    /// ausente: um modelo que manda `{"action":"mos.time.stop"}` esta certo.
    #[test]
    fn stopping_needs_no_arguments() {
        assert_eq!(
            parse_action(r#"{"action":"mos.time.stop"}"#).unwrap(),
            ActionArgs::TimeStop
        );
    }

    /// O preview e a ultima defesa antes de a hora virar fatura, e ele mostra a
    /// duracao nas duas formas justamente para o absurdo saltar aos olhos.
    #[test]
    fn the_record_preview_spells_the_duration_twice() {
        let preview = preview_of(&record(r#"{"project":"X","minutes":150}"#).unwrap());
        let duration = preview
            .lines
            .iter()
            .find(|line| line.label == "Duração")
            .expect("o preview precisa dizer a duracao");
        assert!(duration.value.contains("2h30"), "{}", duration.value);
        assert!(duration.value.contains("150 min"), "{}", duration.value);
        assert_eq!(preview.confirmation, FunctionConfirmation::Explicit);
    }

    /// Iniciar e barato; encerrar e lancar escrevem hora cobravel. Se os tres
    /// tivessem o mesmo peso, ou a cerimonia atrapalharia o barato, ou o caro
    /// passaria sem ninguem ler.
    #[test]
    fn starting_is_cheap_and_the_other_two_are_not() {
        let start = preview_of(&ActionArgs::TimeStart {
            project: "X".into(),
            activity: String::new(),
            description: String::new(),
        });
        assert_eq!(start.confirmation, FunctionConfirmation::None);
        assert_eq!(start.risk, FunctionRisk::Low);

        let stop = preview_of(&ActionArgs::TimeStop);
        assert_eq!(stop.confirmation, FunctionConfirmation::Explicit);
        assert_eq!(stop.risk, FunctionRisk::Medium);
    }

    #[test]
    fn parses_a_task_proposal() {
        let args = parse_action(
            r#"{"action":"mos.task.create","args":{"title":"Refatorar navbar","project":"Minarum"}}"#,
        )
        .unwrap();
        assert_eq!(
            args,
            ActionArgs::TaskCreate {
                title: "Refatorar navbar".into(),
                description: String::new(),
                project: Some("Minarum".into()),
            }
        );
    }

    /// Acao desconhecida e recusada nomeando o que veio. O catalogo vai crescer
    /// com `m-finance.*`, e uma proposta para um App que ainda nao expoe acao
    /// precisa dizer isso em vez de falhar de lado.
    #[test]
    fn an_unknown_action_is_refused_by_name() {
        let error =
            parse_action(r#"{"action":"m-finance.recurrence.create","args":{}}"#).unwrap_err();
        assert!(error.message.contains("m-finance.recurrence.create"));
    }

    /// Campo obrigatorio ausente RECUSA. Corrigir seria o M/OS adivinhando o
    /// que o modelo quis dizer.
    #[test]
    fn a_missing_required_field_refuses_instead_of_guessing() {
        let error = parse_action(r#"{"action":"mos.task.create","args":{}}"#).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);
        assert!(error.message.contains("title"));
    }

    #[test]
    fn an_invalid_state_is_refused_before_reaching_the_service() {
        assert!(parse_action(
            r#"{"action":"mos.task.set_state","args":{"task":"x","state":"quase"}}"#
        )
        .is_err());
    }

    #[test]
    fn a_broken_payload_is_refused_with_the_reason() {
        assert!(parse_action("{isso nao e json}").is_err());
    }

    /// O preview le risco e confirmacao do registro de functions, e nao de uma
    /// segunda lista — duas listas divergiriam no dia em que uma mudasse.
    #[test]
    fn the_preview_reads_risk_from_the_function_registry() {
        let preview = preview_of(&ActionArgs::CaptureCreate {
            content: "uma ideia".into(),
        });
        assert_eq!(preview.action, "mos.capture.create");
        assert_eq!(preview.risk, FunctionRisk::Low);
        assert_eq!(preview.lines.len(), 1);
    }

    /// Toda acao do catalogo precisa existir no registro de functions. Sem este
    /// teste, uma acao nova sem funcao correspondente cairia no default de
    /// risco alto sem ninguem perceber.
    #[test]
    fn every_action_maps_to_a_declared_function() {
        let registry = crate::function_registry();
        for kind in ActionKind::all() {
            assert!(
                registry.iter().any(|entry| entry.id == kind.function_id()),
                "{} aponta para uma funcao que nao existe",
                kind.as_str()
            );
        }
    }

    /// A forma que atravessa a ponte esta escrita a mao do outro lado, em
    /// `hermes.ts`. Um `rename` aqui quebraria o desfazer em silencio: o comando
    /// receberia um passo que nao casa com variante nenhuma e falharia so em
    /// tempo de execucao — dentro da janela de cinco segundos em que o usuario
    /// esta justamente tentando consertar um engano.
    #[test]
    fn the_undo_steps_serialize_with_the_names_the_renderer_expects() {
        let cases = [
            (
                UndoStep::ArchiveCapture { id: "c".into() },
                "archiveCapture",
            ),
            (UndoStep::ArchiveTask { id: "t".into() }, "archiveTask"),
            (
                UndoStep::ArchiveProject { id: "p".into() },
                "archiveProject",
            ),
            (
                UndoStep::ArchiveResource { id: "r".into() },
                "archiveResource",
            ),
            (
                UndoStep::RestoreTaskState {
                    id: "t".into(),
                    state: "doing".into(),
                },
                "restoreTaskState",
            ),
        ];
        for (step, expected) in cases {
            let json = serde_json::to_value(&step).unwrap();
            assert_eq!(json["step"], expected);
            assert!(json.get("id").is_some(), "{expected} perdeu o id");
        }
    }

    #[test]
    fn an_undo_step_round_trips() {
        let step = UndoStep::RestoreTaskState {
            id: "t1".into(),
            state: "backlog".into(),
        };
        let json = serde_json::to_string(&step).unwrap();
        assert_eq!(serde_json::from_str::<UndoStep>(&json).unwrap(), step);
    }

    /// Um efeito sem inverso e possivel, e por isso `undo` e opcional — mas as
    /// cinco acoes da fase 1 tem todas caminho de volta. Se uma nova entrar sem
    /// inverso, que seja por decisao registrada, e nao por esquecimento.
    #[test]
    fn an_effect_can_declare_no_way_back() {
        let effect = ActionEffect::new("feito", None);
        assert!(effect.undo.is_none());
        assert!(effect.entities.is_empty());
    }

    /// O rastro e o que transforma "algo foi feito" em "isto foi feito nisto".
    #[test]
    fn an_effect_records_what_it_touched() {
        let effect = ActionEffect::new("Lembrete criado", None)
            .touching("reminder", "r1", "Enviar bases")
            .touching("task", "t1", "Enviar tipos de bases faltantes");
        assert_eq!(effect.entities.len(), 2);
        assert_eq!(effect.entities[1].kind, "task");
    }

    /// A auditoria e persistida como JSON dentro da parte, e por isso precisa
    /// sobreviver a ida e volta com o desfazer junto.
    #[test]
    fn the_audit_round_trips_with_the_previous_state() {
        let audit = ActionAudit {
            executed_at: "2026-08-20T20:30:00-03:00".into(),
            entities: vec![TouchedEntity {
                kind: "task".into(),
                id: "t1".into(),
                label: "Enviar bases".into(),
            }],
            undo: Some(UndoStep::RestoreTaskState {
                id: "t1".into(),
                state: "backlog".into(),
            }),
        };
        let json = serde_json::to_string(&audit).unwrap();
        assert_eq!(serde_json::from_str::<ActionAudit>(&json).unwrap(), audit);
        // Proposta gravada antes de hoje continua legivel: o campo e opcional.
        let antigo = r#"{"executedAt":"2026-08-20T20:30:00-03:00","entities":[]}"#;
        assert!(serde_json::from_str::<ActionAudit>(antigo).unwrap().undo.is_none());
    }

    #[test]
    fn the_contract_lists_every_action() {
        let contract = action_contract(true);
        for kind in ActionKind::all() {
            assert!(contract.contains(kind.as_str()));
        }
    }

    #[test]
    fn parses_m_finance_create_bill() {
        let raw = r#"{"action":"m-finance.create_bill","args":{"amountCents":18000,"description":"Conta de luz","dueDay":10,"isRecurring":true}}"#;
        assert_eq!(
            parse_action(raw).unwrap(),
            ActionArgs::MFinanceCreateBill {
                amount_cents: 18000,
                description: "Conta de luz".into(),
                due_day: Some(10),
                is_recurring: true,
            }
        );
    }

    #[test]
    fn refuses_a_bill_with_zero_or_negative_amount() {
        let raw = r#"{"action":"m-finance.create_bill","args":{"amountCents":0,"description":"X","isRecurring":false}}"#;
        assert!(parse_action(raw).is_err());
    }

    #[test]
    fn refuses_a_due_day_outside_the_month() {
        let raw = r#"{"action":"m-finance.create_bill","args":{"amountCents":100,"description":"X","dueDay":32,"isRecurring":false}}"#;
        assert!(parse_action(raw).is_err());
    }

    #[test]
    fn the_m_finance_preview_shows_currency_and_due_day() {
        let args = ActionArgs::MFinanceCreateBill {
            amount_cents: 18000,
            description: "Conta de luz".into(),
            due_day: Some(10),
            is_recurring: true,
        };
        let preview = preview_of(&args);
        assert_eq!(preview.title, "CRIAR CONTA NO M-FINANCE");
        assert!(preview.lines.iter().any(|l| l.value.contains("180")));
        assert!(preview.lines.iter().any(|l| l.value.contains("10")));
        assert_eq!(preview.risk, FunctionRisk::High);
    }

    #[test]
    fn the_contract_hides_m_finance_when_not_enabled() {
        assert!(!action_contract(false).contains("m-finance.create_bill"));
        assert!(action_contract(true).contains("m-finance.create_bill"));
    }

    /// Esconder a acao do catalogo nao pode esconder o resto dele: sem
    /// `can_write` o Hermes continua sabendo de todas as outras acoes.
    #[test]
    fn hiding_m_finance_keeps_every_other_action_in_the_contract() {
        let contract = action_contract(false);
        for kind in ActionKind::all() {
            if kind == ActionKind::MFinanceCreateBill {
                continue;
            }
            assert!(contract.contains(kind.as_str()), "faltou {}", kind.as_str());
        }
    }

    /// Espelha a tabela `recusados` de
    /// `apps/m-finance/lib/mos/action-bridge.test.ts`. Os dois validadores
    /// guardam a mesma acao em pontos diferentes — o `mos-core` antes de
    /// desenhar o card, o M-Finance antes do INSERT. Uma folga so de um lado
    /// vira um card que o usuario confirma para receber um erro no recibo.
    #[test]
    fn refuses_the_same_arguments_the_m_finance_bridge_refuses() {
        let recusados = [
            ("valor negativo", r#"{"amountCents":-1,"description":"X","isRecurring":false}"#),
            ("valor zero", r#"{"amountCents":0,"description":"X","isRecurring":false}"#),
            ("valor fracionado", r#"{"amountCents":10.5,"description":"X","isRecurring":false}"#),
            ("valor como texto", r#"{"amountCents":"1800","description":"X","isRecurring":false}"#),
            ("descricao vazia", r#"{"amountCents":100,"description":"   ","isRecurring":false}"#),
            ("descricao ausente", r#"{"amountCents":100,"isRecurring":false}"#),
            ("dia 0", r#"{"amountCents":100,"description":"X","dueDay":0,"isRecurring":false}"#),
            ("dia 32", r#"{"amountCents":100,"description":"X","dueDay":32,"isRecurring":false}"#),
        ];

        for (caso, args) in recusados {
            let raw = format!(r#"{{"action":"m-finance.create_bill","args":{args}}}"#);
            assert!(parse_action(&raw).is_err(), "deveria ter recusado: {caso}");
        }
    }

    #[test]
    fn accepts_the_first_and_last_day_of_the_month() {
        for day in [1u8, 31] {
            let raw = format!(
                r#"{{"action":"m-finance.create_bill","args":{{"amountCents":100,"description":"X","dueDay":{day},"isRecurring":false}}}}"#
            );
            match parse_action(&raw).unwrap() {
                ActionArgs::MFinanceCreateBill { due_day, .. } => assert_eq!(due_day, Some(day)),
                other => panic!("virou outra acao: {other:?}"),
            }
        }
    }

    #[test]
    fn a_bill_without_a_due_day_parses_as_none() {
        for args in [
            r#"{"amountCents":100,"description":"X","isRecurring":false}"#,
            r#"{"amountCents":100,"description":"X","dueDay":null,"isRecurring":false}"#,
        ] {
            let raw = format!(r#"{{"action":"m-finance.create_bill","args":{args}}}"#);
            match parse_action(&raw).unwrap() {
                ActionArgs::MFinanceCreateBill { due_day, .. } => assert_eq!(due_day, None),
                other => panic!("virou outra acao: {other:?}"),
            }
        }
    }

    /// O `mos-core` aceita a proposta sem `isRecurring` e assume `false`,
    /// enquanto o schema zod do M-Finance exige o campo. A divergencia nao
    /// vaza porque quem fala com a Action API e `finance.rs`, que serializa
    /// o bool sempre — a chave nunca chega ausente do outro lado.
    #[test]
    fn a_bill_without_is_recurring_defaults_to_false() {
        let raw = r#"{"action":"m-finance.create_bill","args":{"amountCents":100,"description":"X"}}"#;
        match parse_action(raw).unwrap() {
            ActionArgs::MFinanceCreateBill { is_recurring, .. } => assert!(!is_recurring),
            other => panic!("virou outra acao: {other:?}"),
        }
    }

    #[test]
    fn the_m_finance_preview_omits_the_due_date_line_when_there_is_no_day() {
        let preview = preview_of(&ActionArgs::MFinanceCreateBill {
            amount_cents: 100,
            description: "X".into(),
            due_day: None,
            is_recurring: false,
        });

        assert!(!preview.lines.iter().any(|l| l.label == "Vencimento"));
        assert!(preview
            .lines
            .iter()
            .any(|l| l.label == "Recorrente" && l.value == "não"));
    }

    /// O card e a ultima coisa que o usuario le antes de autorizar uma acao de
    /// risco alto. Se o valor exibido nao for o valor enviado, o preview esta
    /// mentindo — e o consentimento nao vale.
    #[test]
    fn the_m_finance_preview_never_rounds_the_amount_away() {
        for (cents, esperado) in [(1i64, "0,01"), (99, "0,99"), (12345, "123,45")] {
            let preview = preview_of(&ActionArgs::MFinanceCreateBill {
                amount_cents: cents,
                description: "X".into(),
                due_day: None,
                is_recurring: false,
            });
            let valor = preview
                .lines
                .iter()
                .find(|l| l.label == "Valor")
                .expect("o card sempre mostra o valor");
            assert!(
                valor.value.contains(esperado),
                "{cents} centavos apareceram como {}",
                valor.value
            );
        }
    }

    /// O valor sai no padrao brasileiro porque o app e em portugues e o card e
    /// o ultimo lugar onde a pessoa confere quanto vai ser gravado.
    #[test]
    fn the_amount_reads_like_brazilian_money() {
        for (cents, esperado) in [
            (1i64, "R$ 0,01"),
            (99, "R$ 0,99"),
            (100, "R$ 1,00"),
            (1990, "R$ 19,90"),
            (123456, "R$ 1.234,56"),
            (100000000, "R$ 1.000.000,00"),
        ] {
            assert_eq!(format_cents(cents), esperado, "{cents} centavos");
        }
    }

    /// Nenhuma acao propoe valor negativo — o parser recusa antes. Mas se um
    /// dia propuser, o card tem de mostrar o sinal, e nao esconde-lo.
    #[test]
    fn a_negative_amount_keeps_its_sign() {
        assert_eq!(format_cents(-12345), "-R$ 123,45");
    }

    #[test]
    fn zero_is_not_a_special_case() {
        assert_eq!(format_cents(0), "R$ 0,00");
    }
    // ------------------------------------------------------------- lembrete

    /// O relogio dos testes de lembrete. Fixo, porque "hoje" so quer dizer
    /// alguma coisa contra um dia.
    fn agora() -> OffsetDateTime {
        time::macros::datetime!(2026-08-20 14:32:00 -03:00)
    }

    fn lembrete(args: &str) -> Result<ActionArgs, CoreError> {
        parse_action_at(
            &format!(r#"{{"action":"mos.reminder.create","args":{args}}}"#),
            agora(),
        )
    }

    /// O caso do pedido, inteiro: hora dita em portugues, alvo apontado por id
    /// curto, e nada de Task nova.
    #[test]
    fn the_motivating_reminder_parses_with_its_task_link() {
        let args = lembrete(
            r#"{"title":"Enviar tipos de bases faltantes para o Victor","when":"hoje às 20:30","taskRef":"7c3e2b19"}"#,
        )
        .unwrap();
        match args {
            ActionArgs::ReminderCreate {
                ref title,
                ref at,
                ref when_raw,
                ref target,
                ..
            } => {
                assert!(title.contains("Victor"));
                assert!(at.starts_with("2026-08-20T20:30:00"), "{at}");
                // O offset de quem falou atravessa: sem ele, 20:30 no Brasil
                // viraria 17:30 na tela.
                assert!(at.ends_with("-03:00"), "{at}");
                assert!(when_raw.contains("20:30"), "{when_raw}");
                let target = target.as_ref().expect("o vinculo com a Task");
                assert_eq!(target.kind, "task");
                assert_eq!(target.reference, "7c3e2b19");
            }
            outro => panic!("virou outra acao: {outro:?}"),
        }
    }

    /// `at` sem fuso herda o de quem falou. Lido como UTC, o lembrete tocaria
    /// tres horas depois — no dia seguinte, se fosse de noite.
    #[test]
    fn an_instant_without_a_timezone_inherits_the_speakers() {
        match lembrete(r#"{"title":"X","at":"2026-08-20T20:30"}"#).unwrap() {
            ActionArgs::ReminderCreate { at, .. } => {
                assert_eq!(at, "2026-08-20T20:30:00-03:00")
            }
            outro => panic!("virou outra acao: {outro:?}"),
        }
    }

    /// Um espaco no lugar do T e escrita natural, e recusar por causa dele
    /// seria recusar por causa de um caractere.
    #[test]
    fn a_space_instead_of_the_t_still_parses() {
        assert!(lembrete(r#"{"title":"X","at":"2026-08-21 09:00"}"#).is_ok());
    }

    /// `at` ganha de `when`: um modelo que fez a conta ja aplicou o que sabe, e
    /// reinterpretar a frase por cima trocaria uma pela outra.
    #[test]
    fn an_explicit_instant_wins_over_the_spoken_phrase() {
        match lembrete(r#"{"title":"X","at":"2026-08-25T10:00","when":"amanhã"}"#).unwrap() {
            ActionArgs::ReminderCreate { at, when_raw, .. } => {
                assert!(at.starts_with("2026-08-25"), "{at}");
                assert!(when_raw.is_empty());
            }
            outro => panic!("virou outra acao: {outro:?}"),
        }
    }

    /// Sem hora nao ha lembrete, e inventar uma seria agendar o que ninguem
    /// pediu.
    #[test]
    fn a_reminder_without_an_instant_is_refused() {
        let error = lembrete(r#"{"title":"X"}"#).unwrap_err();
        assert!(error.message.contains("precisa de hora"), "{}", error.message);
    }

    /// Recusar na LEITURA devolve o erro para o cartao. Recusar so na execucao
    /// devolveria depois de o usuario ja ter autorizado.
    #[test]
    fn a_reminder_in_the_past_is_refused_before_the_card() {
        let error = lembrete(r#"{"title":"X","at":"2026-08-20T09:00"}"#).unwrap_err();
        assert!(error.message.contains("passado"), "{}", error.message);
        // "Hoje as nove", dito as duas da tarde, e o mesmo caso — e ele chega
        // por `when`, que e como a pessoa fala.
        assert!(lembrete(r#"{"title":"X","when":"hoje às 9h"}"#).is_err());
    }

    #[test]
    fn an_unreadable_instant_is_refused_naming_it() {
        let error = lembrete(r#"{"title":"X","at":"20/08/2026 20:30"}"#).unwrap_err();
        assert!(error.message.contains("20/08/2026"), "{}", error.message);
    }

    /// Entre Task e Project, o vinculo util e a Task: dela se chega ao Project,
    /// e do Project nao se chega a Task.
    #[test]
    fn the_most_specific_target_wins() {
        match lembrete(r#"{"title":"X","when":"amanhã","taskRef":"t1","projectRef":"p1"}"#).unwrap()
        {
            ActionArgs::ReminderCreate { target, .. } => {
                assert_eq!(target.unwrap().kind, "task")
            }
            outro => panic!("virou outra acao: {outro:?}"),
        }
    }

    /// O cartao e onde o engano de data e pego. Ele mostra o instante por
    /// extenso E a frase que o produziu, lado a lado.
    #[test]
    fn the_reminder_card_shows_the_instant_and_the_phrase_that_made_it() {
        let args = lembrete(r#"{"title":"Enviar bases","when":"hoje às 20:30","taskRef":"7c3e2b19"}"#)
            .unwrap();
        let preview = preview_of(&args);
        assert_eq!(preview.title, "CRIAR LEMBRETE");
        let quando = preview
            .lines
            .iter()
            .find(|linha| linha.label == "Quando")
            .expect("o cartao sempre diz quando");
        assert!(quando.value.contains("quinta-feira"), "{}", quando.value);
        assert!(quando.value.contains("20 de agosto"), "{}", quando.value);
        assert!(quando.value.contains("20:30"), "{}", quando.value);
        assert!(preview.lines.iter().any(|linha| linha.label == "Você disse"));
        assert!(preview
            .lines
            .iter()
            .any(|linha| linha.label == "Vinculado à Task"));
    }

    #[test]
    fn a_reminder_outcome_outside_the_vocabulary_is_refused() {
        assert!(parse_action(r#"{"action":"mos.reminder.resolve","args":{"reminder":"r1","state":"talvez"}}"#).is_err());
        assert!(parse_action(r#"{"action":"mos.reminder.resolve","args":{"reminder":"r1","state":"done"}}"#).is_ok());
    }

    // -------------------------------------------------------- capture e task

    #[test]
    fn converting_a_capture_needs_the_capture_and_nothing_else() {
        match parse_action(r#"{"action":"mos.capture.to_task","args":{"capture":"1f4c9a2b"}}"#)
            .unwrap()
        {
            ActionArgs::CaptureToTask {
                capture,
                title,
                project,
            } => {
                assert_eq!(capture, "1f4c9a2b");
                assert!(title.is_empty());
                assert!(project.is_none());
            }
            outro => panic!("virou outra acao: {outro:?}"),
        }
        assert!(parse_action(r#"{"action":"mos.capture.to_task","args":{}}"#).is_err());
    }

    /// Mover de Project pede os dois lados. Sem o Project, a acao seria "tirar
    /// de todos" — que e uma intencao diferente e precisa ser dita.
    #[test]
    fn moving_a_task_between_projects_needs_both_sides() {
        assert!(parse_action(r#"{"action":"mos.task.set_project","args":{"task":"t1"}}"#).is_err());
        assert!(parse_action(
            r#"{"action":"mos.task.set_project","args":{"task":"t1","project":"063-26"}}"#
        )
        .is_ok());
    }

    /// Trocar o Project de uma Task mexe em onde as horas serao contadas, e por
    /// isso nao herda o risco de arrastar um cartao entre colunas.
    #[test]
    fn changing_the_project_is_heavier_than_changing_the_column() {
        let coluna = preview_of(&ActionArgs::TaskSetState {
            task: "t1".into(),
            state: "doing".into(),
        });
        let projeto = preview_of(&ActionArgs::TaskSetProject {
            task: "t1".into(),
            project: "063-26".into(),
        });
        assert_eq!(coluna.risk, FunctionRisk::Low);
        assert_eq!(projeto.risk, FunctionRisk::Medium);
        assert_eq!(projeto.confirmation, FunctionConfirmation::Explicit);
    }

    /// O contrato precisa ensinar que id vence titulo. Sem esta frase o modelo
    /// manda o titulo por padrao, e titulo ambiguo faz o M/OS parar e perguntar
    /// — que e exatamente o fluxo que este trabalho existe para evitar.
    #[test]
    fn the_contract_teaches_that_the_id_beats_the_title() {
        let contract = action_contract(false);
        assert!(contract.contains("Prefira o id"), "{contract}");
    }

    #[test]
    fn the_m_finance_action_id_round_trips() {
        let kind = ActionKind::MFinanceCreateBill;
        assert_eq!(ActionKind::parse(kind.as_str()), Some(kind));
        assert_eq!(kind.function_id(), "m-finance.create_bill");
    }

    // ------------------------------------------------------- Daily Session

    fn dia(action: &str, args: &str) -> Result<ActionArgs, CoreError> {
        parse_action(&format!(r#"{{"action":"mos.day.{action}","args":{args}}}"#))
    }

    #[test]
    fn le_um_inicio_de_dia_com_principal_e_secundarios() {
        let lido = dia(
            "start",
            r#"{"main":"Finalizar planta de formas","mainRef":"7c3e2b19","secondaries":["Revisar memorial","  ","Implementar Daily Session"],"note":"duas entregas hoje"}"#,
        )
        .unwrap();
        assert_eq!(
            lido,
            ActionArgs::DayStart {
                main: "Finalizar planta de formas".into(),
                main_ref: "7c3e2b19".into(),
                // O secundario em branco cai fora na leitura. Um objetivo vazio
                // ocuparia uma das tres vagas de foco sem dizer nada.
                secondaries: vec![
                    "Revisar memorial".into(),
                    "Implementar Daily Session".into()
                ],
                note: "duas entregas hoje".into(),
            }
        );
    }

    #[test]
    fn um_dia_sem_principal_e_recusado() {
        // O principal e a pergunta que a feature existe para fazer. Aceitar a
        // proposta sem ele devolveria um dia que nao responde nada.
        assert!(dia("start", r#"{"secondaries":["a"]}"#).is_err());
    }

    #[test]
    fn um_objetivo_sem_prioridade_nasce_secundario() {
        let lido = dia("add_objective", r#"{"title":"Revisar memorial"}"#).unwrap();
        assert_eq!(
            lido,
            ActionArgs::DayAddObjective {
                title: "Revisar memorial".into(),
                // Secundario e o padrao seguro: promover a principal em silencio
                // rebaixaria o que a pessoa escolheu de manha.
                priority: "secondary".into(),
                link: None,
            }
        );
        assert!(dia("add_objective", r#"{"title":"x","priority":"urgente"}"#).is_err());
    }

    #[test]
    fn um_objetivo_pode_apontar_para_a_task_que_ele_e() {
        let lido = dia(
            "add_objective",
            r#"{"title":"Enviar arquivos","priority":"main","taskRef":"7c3e2b19"}"#,
        )
        .unwrap();
        let ActionArgs::DayAddObjective { link, priority, .. } = lido else {
            panic!("esperava DayAddObjective");
        };
        assert_eq!(priority, "main");
        let link = link.expect("o vinculo e o que faz a conclusao automatica existir");
        assert_eq!(link.kind, "task");
        assert_eq!(link.reference, "7c3e2b19");
    }

    #[test]
    fn desfecho_de_objetivo_desconhecido_e_recusado_na_leitura() {
        // Recusar na LEITURA devolve o erro para o cartao, onde ele e uma frase.
        // Recusar so na execucao devolveria o erro depois de a pessoa autorizar.
        assert!(dia("set_objective", r#"{"objective":"memorial","status":"talvez"}"#).is_err());
        assert_eq!(
            dia("set_objective", r#"{"objective":"memorial","status":"carried_over"}"#).unwrap(),
            ActionArgs::DaySetObjective {
                objective: "memorial".into(),
                status: "carried_over".into(),
            }
        );
    }

    #[test]
    fn encerrar_o_dia_nao_exige_campo_nenhum() {
        // A reflexao e opcional por desenho: o pedido pede menos de dois minutos
        // para fechar o dia, e um campo obrigatorio ali e friccao pura.
        assert_eq!(
            dia("end", "{}").unwrap(),
            ActionArgs::DayEnd {
                mood: String::new(),
                summary: String::new()
            }
        );
        assert!(dia("end", r#"{"mood":"cansado"}"#).is_err(), "humor fora do vocabulario e recusado");
        assert!(dia("end", r#"{"mood":"blocked","summary":"o 063-26 tomou o dia"}"#).is_ok());
    }

    /// O cartao precisa dizer POR QUE o Hermes montou aquele dia. Autorizar um
    /// dia montado por outro sem ver a justificativa seria assinar em branco.
    #[test]
    fn o_cartao_do_inicio_do_dia_mostra_a_justificativa() {
        let args = dia(
            "start",
            r#"{"main":"Planta de formas","secondaries":["Memorial"],"note":"reunião às 15h"}"#,
        )
        .unwrap();
        let preview = preview_of(&args);
        assert_eq!(preview.title, "INICIAR O DIA");
        let rotulos: Vec<_> = preview.lines.iter().map(|linha| linha.label.as_str()).collect();
        assert!(rotulos.contains(&"Principal"), "{rotulos:?}");
        assert!(rotulos.contains(&"Secundário 1"), "{rotulos:?}");
        assert!(rotulos.contains(&"Por quê"), "{rotulos:?}");
    }

    /// Encerrar o dia resolve varios objetivos de uma vez. O risco continua
    /// baixo — reabrir devolve tudo —, mas a confirmacao e explicita: risco e
    /// peso sao campos diferentes de proposito.
    #[test]
    fn encerrar_o_dia_pede_confirmacao_explicita_sem_ser_risco_alto() {
        let preview = preview_of(&dia("end", "{}").unwrap());
        assert_eq!(preview.risk, FunctionRisk::Low);
        assert_eq!(preview.confirmation, FunctionConfirmation::Explicit);
        assert_eq!(preview.title, "ENCERRAR O DIA");
    }

    /// Toda acao do catalogo tem de achar a funcao dela em `functions.rs`, senao
    /// o preview sai sem risco e sem confirmacao — e um cartao sem peso e um
    /// cartao que se autoriza no automatico.
    #[test]
    fn toda_acao_do_catalogo_tem_funcao_declarada() {
        let registro = crate::function_registry();
        for kind in ActionKind::all() {
            assert!(
                registro.iter().any(|entry| entry.id == kind.function_id()),
                "{} aponta para `{}`, que nao existe em functions.rs",
                kind.as_str(),
                kind.function_id()
            );
        }
    }

    /// O nome de cada acao atravessa a ponte e desce no prompt. Um rename
    /// silencioso faria o modelo propor uma acao que o M/OS recusa.
    #[test]
    fn todo_nome_de_acao_volta_pelo_parse() {
        for kind in ActionKind::all() {
            assert_eq!(ActionKind::parse(kind.as_str()), Some(kind));
        }
    }
}
