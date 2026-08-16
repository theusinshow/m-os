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
}

impl ActionKind {
    /// O nome que atravessa a ponte. Prefixado pelo App dono, porque o catalogo
    /// vai crescer com `m-finance.*` e `cronocad.*`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CaptureCreate => "mos.capture.create",
            Self::TaskCreate => "mos.task.create",
            Self::TaskSetState => "mos.task.set_state",
            Self::ProjectCreate => "mos.project.create",
            Self::ResourceCreate => "mos.resource.create",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "mos.capture.create" => Some(Self::CaptureCreate),
            "mos.task.create" => Some(Self::TaskCreate),
            "mos.task.set_state" => Some(Self::TaskSetState),
            "mos.project.create" => Some(Self::ProjectCreate),
            "mos.resource.create" => Some(Self::ResourceCreate),
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
        }
    }

    pub fn all() -> [ActionKind; 5] {
        [
            Self::CaptureCreate,
            Self::TaskCreate,
            Self::TaskSetState,
            Self::ProjectCreate,
            Self::ResourceCreate,
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
        }
    }
}

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
}

impl ActionArgs {
    pub fn kind(&self) -> ActionKind {
        match self {
            Self::CaptureCreate { .. } => ActionKind::CaptureCreate,
            Self::TaskCreate { .. } => ActionKind::TaskCreate,
            Self::TaskSetState { .. } => ActionKind::TaskSetState,
            Self::ProjectCreate { .. } => ActionKind::ProjectCreate,
            Self::ResourceCreate { .. } => ActionKind::ResourceCreate,
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
    })
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
    ArchiveCapture { id: String },
    ArchiveTask { id: String },
    ArchiveProject { id: String },
    ArchiveResource { id: String },
    RestoreTaskState { id: String, state: String },
}

/// O contrato que desce no prompt.
///
/// Diz o que existe e como responder. Nao pede que o modelo execute nada, e a
/// frase final e deliberada: sem ela, um modelo prestativo tende a preencher
/// campos que o usuario nao disse.
pub fn action_contract() -> String {
    let catalog = ActionKind::all()
        .iter()
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
        let contract = action_contract();
        for kind in ActionKind::all() {
            assert!(contract.contains(kind.as_str()));
        }
    }
}
