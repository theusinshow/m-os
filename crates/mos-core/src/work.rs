use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Capture, CaptureId, CoreError, ErrorCode, LifecycleState, RegisteredApp};

macro_rules! entity_id {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            pub fn parse(value: &str) -> Result<Self, CoreError> {
                Uuid::parse_str(value).map(Self).map_err(|_| {
                    CoreError::new(
                        ErrorCode::InvalidInput,
                        concat!($label, " ID invalido."),
                        false,
                    )
                })
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

entity_id!(ProjectId, "Project");
entity_id!(TaskId, "Task");
entity_id!(WorkspaceId, "Workspace");

/// Estado de trabalho da Task.
///
/// A ordem das variantes e a ordem das colunas do kanban.
///
/// NOTA: `Inbox` aqui NAO e a Inbox de Captures. Sao conceitos distintos que
/// compartilham o nome porque o design usa INBOX como rotulo da primeira coluna.
/// Capture tem `processing_state`; Task tem `state`. Nunca sao a mesma coisa.
/// Ver docs/superpowers/specs/2026-08-13-mos-v03-design.md secao 4.3.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Inbox,
    Backlog,
    Planned,
    Doing,
    Review,
    Done,
}

impl TaskState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inbox => "inbox",
            Self::Backlog => "backlog",
            Self::Planned => "planned",
            Self::Doing => "doing",
            Self::Review => "review",
            Self::Done => "done",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "inbox" => Ok(Self::Inbox),
            "backlog" => Ok(Self::Backlog),
            "planned" => Ok(Self::Planned),
            "doing" => Ok(Self::Doing),
            "review" => Ok(Self::Review),
            "done" => Ok(Self::Done),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Estado de Task desconhecido.",
                false,
            )),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub description: String,
    /// Repositorio associado. Vazio significa sem repositorio.
    /// Nesta fase e so o campo: sem API, sem token, sem sincronizacao.
    pub repository: String,
    pub lifecycle_state: LifecycleState,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub description: String,
    pub lifecycle_state: LifecycleState,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// Um item desta lista significa OCULTO — ausencia e o padrao visivel.
/// Ver a migration 0008 para o porque da inversao.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HiddenWidget {
    pub workspace_id: WorkspaceId,
    pub widget_id: String,
}

#[derive(Clone, Debug)]
pub struct NewWorkspace {
    pub id: WorkspaceId,
    pub name: String,
    pub description: String,
    pub created_at: OffsetDateTime,
}

impl NewWorkspace {
    pub fn create(name: &str, description: &str) -> Result<Self, CoreError> {
        let name = required(name, "O nome do Workspace nao pode estar vazio.")?;
        Ok(Self {
            id: WorkspaceId::new(),
            name,
            description: description.trim().to_owned(),
            created_at: OffsetDateTime::now_utc(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct NewProject {
    pub id: ProjectId,
    pub name: String,
    pub description: String,
    pub repository: String,
    pub created_at: OffsetDateTime,
}

impl NewProject {
    pub fn create(name: &str, description: &str, repository: &str) -> Result<Self, CoreError> {
        let name = required(name, "O nome do Project nao pode estar vazio.")?;
        Ok(Self {
            id: ProjectId::new(),
            name,
            description: description.trim().to_owned(),
            repository: repository.trim().to_owned(),
            created_at: OffsetDateTime::now_utc(),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: TaskId,
    pub title: String,
    pub description: String,
    pub project_id: Option<ProjectId>,
    pub source_capture_id: Option<CaptureId>,
    pub state: TaskState,
    pub lifecycle_state: LifecycleState,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub completed_at: Option<OffsetDateTime>,
}

#[derive(Clone, Debug)]
pub struct NewTask {
    pub id: TaskId,
    pub title: String,
    pub description: String,
    pub project_id: Option<ProjectId>,
    pub created_at: OffsetDateTime,
}

impl NewTask {
    pub fn create(
        title: &str,
        description: &str,
        project_id: Option<ProjectId>,
    ) -> Result<Self, CoreError> {
        Ok(Self {
            id: TaskId::new(),
            title: required(title, "O titulo da Task nao pode estar vazio.")?,
            description: description.trim().to_owned(),
            project_id,
            created_at: OffsetDateTime::now_utc(),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SearchItem {
    Capture {
        capture: Capture,
        derived_task: Option<Task>,
        project: Option<Project>,
    },
    Task {
        task: Task,
        project: Option<Project>,
    },
    Project {
        project: Project,
    },
    Workspace {
        workspace: Workspace,
    },
    App {
        app: RegisteredApp,
    },
}

/// Espelha o CHECK da migration 0008: minuscula inicial, depois minuscula,
/// digito ou `_`. O core valida forma, nao vocabulario — quem conhece o catalogo
/// de widgets e o front, em HOME_WIDGETS.
/// A posicao de um widget na Home de um Workspace.
///
/// Espelha a inversao de `workspace_hidden_widgets`: **ausencia de linha
/// significa a ordem do catalogo.** Workspace novo nao precisa de nenhuma
/// escrita, e widget criado depois nasce onde o catalogo o pos, em vez de
/// nascer no lugar que uma tabela vazia sortear.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetPosition {
    pub workspace_id: WorkspaceId,
    pub widget_id: String,
    pub position: i64,
}

/// Ordena ids de widget aplicando as posicoes guardadas.
///
/// O core NAO conhece o catalogo — ele vive no front, em `HOME_WIDGETS`, e a
/// mesma razao que fez `workspace_hidden_widgets` guardar string opaca vale
/// aqui: enum no nucleo faria de cada widget novo uma migration.
///
/// Tres regras, e cada uma existe para um caso que da errado sozinho:
///
/// 1. quem tem posicao guardada vem primeiro, na ordem dela;
/// 2. quem nao tem vai para o fim, preservando a ordem em que chegou — que e a
///    do catalogo. Widget novo aparecendo no meio de um arranjo que a pessoa
///    montou seria o sistema desfazendo a escolha dela;
/// 3. posicao repetida ou salteada nao quebra nada: o desempate e a ordem de
///    chegada. Banco com linha orfa ou meio gravada nao pode sumir com widget.
pub fn order_widgets(catalog: &[String], saved: &[WidgetPosition]) -> Vec<String> {
    let mut ordered: Vec<(Option<i64>, usize, &String)> = catalog
        .iter()
        .enumerate()
        .map(|(index, id)| {
            let position = saved
                .iter()
                .find(|entry| &entry.widget_id == id)
                .map(|entry| entry.position);
            (position, index, id)
        })
        .collect();

    ordered.sort_by(|left, right| match (left.0, right.0) {
        (Some(a), Some(b)) => a.cmp(&b).then(left.1.cmp(&right.1)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => left.1.cmp(&right.1),
    });

    ordered.into_iter().map(|(_, _, id)| id.clone()).collect()
}

pub fn validate_widget_id(value: &str) -> Result<String, CoreError> {
    let value = value.trim();
    let valid = !value.is_empty()
        && value.len() <= 40
        && value.starts_with(|character: char| character.is_ascii_lowercase())
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        });
    if valid {
        Ok(value.to_owned())
    } else {
        Err(CoreError::new(
            ErrorCode::InvalidInput,
            "ID de widget invalido.",
            false,
        ))
    }
}

fn required(value: &str, message: &str) -> Result<String, CoreError> {
    let value = value.trim();
    if value.is_empty() {
        Err(CoreError::new(ErrorCode::InvalidInput, message, false))
    } else {
        Ok(value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_and_task_require_names() {
        assert!(NewProject::create(" ", "", "").is_err());
        assert!(NewTask::create("", "", None).is_err());
        assert!(NewWorkspace::create("", "").is_err());
    }

    #[test]
    fn task_states_have_stable_storage_values() {
        assert_eq!(TaskState::Inbox.as_str(), "inbox");
        assert_eq!(TaskState::Backlog.as_str(), "backlog");
        assert_eq!(TaskState::Planned.as_str(), "planned");
        assert_eq!(TaskState::Doing.as_str(), "doing");
        assert_eq!(TaskState::Review.as_str(), "review");
        assert_eq!(TaskState::Done.as_str(), "done");
    }

    #[test]
    fn task_states_round_trip_through_parse() {
        for state in [
            TaskState::Inbox,
            TaskState::Backlog,
            TaskState::Planned,
            TaskState::Doing,
            TaskState::Review,
            TaskState::Done,
        ] {
            assert_eq!(TaskState::parse(state.as_str()).unwrap(), state);
        }
    }

    #[test]
    fn unknown_task_state_is_rejected() {
        assert!(TaskState::parse("arquivado").is_err());
        assert!(TaskState::parse("").is_err());
    }

    // ------------------------------------------------------ ordem dos widgets

    fn catalog() -> Vec<String> {
        ["timer", "now", "today_hours", "inbox_pulse"]
            .iter()
            .map(|id| (*id).to_owned())
            .collect()
    }

    fn placed(workspace: WorkspaceId, id: &str, position: i64) -> WidgetPosition {
        WidgetPosition {
            workspace_id: workspace,
            widget_id: id.to_owned(),
            position,
        }
    }

    /// Sem nenhuma escrita, a Home e a do catalogo. E o caso de quem nunca
    /// arrastou nada, que e a maioria.
    #[test]
    fn without_saved_positions_the_catalog_order_wins() {
        assert_eq!(order_widgets(&catalog(), &[]), catalog());
    }

    #[test]
    fn saved_positions_decide_the_order() {
        let workspace = WorkspaceId::new();
        let saved = [
            placed(workspace, "inbox_pulse", 0),
            placed(workspace, "timer", 1),
            placed(workspace, "now", 2),
            placed(workspace, "today_hours", 3),
        ];
        assert_eq!(
            order_widgets(&catalog(), &saved),
            ["inbox_pulse", "timer", "now", "today_hours"]
        );
    }

    /// O caso que decide se a feature envelhece bem: alguem arrumou a Home, e
    /// meses depois um widget novo entra no catalogo. Ele NAO pode aparecer no
    /// meio do arranjo — isso seria o sistema desfazendo a escolha da pessoa.
    #[test]
    fn a_widget_added_later_goes_to_the_end_of_an_arranged_home() {
        let workspace = WorkspaceId::new();
        let saved = [
            placed(workspace, "inbox_pulse", 0),
            placed(workspace, "timer", 1),
        ];
        let mut with_newcomer = catalog();
        with_newcomer.push("brand_new".to_owned());

        assert_eq!(
            order_widgets(&with_newcomer, &saved),
            ["inbox_pulse", "timer", "now", "today_hours", "brand_new"],
            "os sem posicao vao para o fim, na ordem do catalogo"
        );
    }

    /// Linha de widget que nao existe mais e inofensiva, do mesmo jeito que a
    /// tabela de ocultos ja trata.
    #[test]
    fn a_position_for_a_widget_that_no_longer_exists_is_ignored() {
        let workspace = WorkspaceId::new();
        let saved = [placed(workspace, "widget_extinto", 0), placed(workspace, "now", 1)];
        assert_eq!(
            order_widgets(&catalog(), &saved),
            ["now", "timer", "today_hours", "inbox_pulse"]
        );
    }

    /// Banco meio gravado nao pode sumir com widget nenhum.
    #[test]
    fn repeated_or_gapped_positions_never_drop_a_widget() {
        let workspace = WorkspaceId::new();
        let saved = [
            placed(workspace, "now", 5),
            placed(workspace, "timer", 5),
            placed(workspace, "inbox_pulse", 900),
        ];
        let result = order_widgets(&catalog(), &saved);

        assert_eq!(result.len(), catalog().len(), "ninguem se perde");
        for id in catalog() {
            assert!(result.contains(&id), "{id} sumiu");
        }
        assert_eq!(
            &result[..2],
            ["timer", "now"],
            "empate desempata pela ordem do catalogo"
        );
    }

    #[test]
    fn an_empty_catalog_orders_to_nothing() {
        assert!(order_widgets(&[], &[]).is_empty());
    }
}
