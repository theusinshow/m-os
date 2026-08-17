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
    TaskCreate,
    TaskSetState,
    ProjectCreate,
    ResourceCreate,
    TimeStart,
    TimeStop,
    TimeRecord,
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
            Self::TaskCreate => "mos.task.create",
            Self::TaskSetState => "mos.task.set_state",
            Self::ProjectCreate => "mos.project.create",
            Self::ResourceCreate => "mos.resource.create",
            Self::TimeStart => "mos.time.start",
            Self::TimeStop => "mos.time.stop",
            Self::TimeRecord => "mos.time.record",
            Self::MFinanceCreateBill => "m-finance.create_bill",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "mos.capture.create" => Some(Self::CaptureCreate),
            "mos.task.create" => Some(Self::TaskCreate),
            "mos.task.set_state" => Some(Self::TaskSetState),
            "mos.project.create" => Some(Self::ProjectCreate),
            "mos.resource.create" => Some(Self::ResourceCreate),
            "mos.time.start" => Some(Self::TimeStart),
            "mos.time.stop" => Some(Self::TimeStop),
            "mos.time.record" => Some(Self::TimeRecord),
            "m-finance.create_bill" => Some(Self::MFinanceCreateBill),
            _ => None,
        }
    }

    /// Liga a acao a funcao ja declarada em `functions.rs`, que e onde risco e
    /// confirmacao moram. Duas listas de risco seriam duas fontes de verdade.
    pub fn function_id(self) -> &'static str {
        match self {
            Self::CaptureCreate => "capture.create",
            Self::TaskCreate => "task.create",
            Self::TaskSetState => "task.set_state",
            Self::ProjectCreate => "project.create",
            Self::ResourceCreate => "resource.create",
            Self::TimeStart => "time.start",
            Self::TimeStop => "time.stop",
            Self::TimeRecord => "time.record",
            Self::MFinanceCreateBill => "m-finance.create_bill",
        }
    }

    pub fn all() -> [ActionKind; 9] {
        [
            Self::CaptureCreate,
            Self::TaskCreate,
            Self::TaskSetState,
            Self::ProjectCreate,
            Self::ResourceCreate,
            Self::TimeStart,
            Self::TimeStop,
            Self::TimeRecord,
            Self::MFinanceCreateBill,
        ]
    }

    /// Assinatura em uma linha, para o prompt. Curta de proposito: o catalogo
    /// inteiro desce a cada mensagem, e cada palavra aqui e token gasto em toda
    /// conversa.
    pub fn signature(self) -> &'static str {
        match self {
            Self::CaptureCreate => "{ content }",
            Self::TaskCreate => "{ title, description?, project? }",
            Self::TaskSetState => "{ task, state: inbox|backlog|planned|doing|review|done }",
            Self::ProjectCreate => "{ name, description? }",
            Self::ResourceCreate => "{ kind: site|library|image|note, title, url?, note? }",
            Self::TimeStart => "{ project, activity?, description? }",
            Self::TimeStop => "{ }",
            // Em MINUTOS, e nao "1h30": tres jeitos de escrever a mesma duracao
            // dao tres jeitos de errar um quarto dela, e o erro sai na fatura.
            Self::TimeRecord => "{ project, minutes, day?: AAAA-MM-DD, activity?, description? }",
            Self::MFinanceCreateBill => "{ amountCents, description, dueDay?: 1-31, isRecurring }",
        }
    }
}

/// As atividades que uma sessao pode ter. Espelha `ActivityType` do dominio.
const ACTIVITIES: &str = "drawing|detailing|revision|meeting|study|other";

/// Argumentos ja validados. Sair do JSON solto o quanto antes e o que impede um
/// campo inventado pelo modelo de viajar ate a camada de servico.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "action", content = "args")]
pub enum ActionArgs {
    CaptureCreate {
        content: String,
    },
    TaskCreate {
        title: String,
        description: String,
        /// Nome do Project, como o usuario fala. Resolvido na execucao.
        project: Option<String>,
    },
    TaskSetState {
        /// Titulo da Task, como o usuario fala.
        task: String,
        state: String,
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
            Self::TaskCreate { .. } => ActionKind::TaskCreate,
            Self::TaskSetState { .. } => ActionKind::TaskSetState,
            Self::ProjectCreate { .. } => ActionKind::ProjectCreate,
            Self::ResourceCreate { .. } => ActionKind::ResourceCreate,
            Self::TimeStart { .. } => ActionKind::TimeStart,
            Self::TimeStop => ActionKind::TimeStop,
            Self::TimeRecord { .. } => ActionKind::TimeRecord,
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

/// Le uma proposta e devolve argumentos validados.
///
/// Argumento fora do esquema faz a proposta ser RECUSADA, e nao corrigida.
/// Corrigir seria o M/OS adivinhando o que o modelo quis dizer — e adivinhar e
/// exatamente o que o preview existe para evitar.
pub fn parse_action(raw: &str) -> Result<ActionArgs, CoreError> {
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
fn format_cents(cents: i64) -> String {
    format!("R$ {:.2}", cents as f64 / 100.0)
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
    /// Manda a sessao para a lixeira. Soft delete, entao ela continua no banco e
    /// volta pela lixeira do Historico — o desfazer aqui obedece a mesma regra
    /// do resto: some da vista, nao some do registro.
    TrashTimeEntry {
        id: String,
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
        let effect = ActionEffect {
            message: "feito".into(),
            undo: None,
        };
        assert!(effect.undo.is_none());
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
}
