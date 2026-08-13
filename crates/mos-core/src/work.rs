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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Backlog,
    Doing,
    Done,
}

impl TaskState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Backlog => "backlog",
            Self::Doing => "doing",
            Self::Done => "done",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "backlog" => Ok(Self::Backlog),
            "doing" => Ok(Self::Doing),
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
    pub lifecycle_state: LifecycleState,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Clone, Debug)]
pub struct NewProject {
    pub id: ProjectId,
    pub name: String,
    pub description: String,
    pub created_at: OffsetDateTime,
}

impl NewProject {
    pub fn create(name: &str, description: &str) -> Result<Self, CoreError> {
        let name = required(name, "O nome do Project nao pode estar vazio.")?;
        Ok(Self {
            id: ProjectId::new(),
            name,
            description: description.trim().to_owned(),
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
    App {
        app: RegisteredApp,
    },
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
        assert!(NewProject::create(" ", "").is_err());
        assert!(NewTask::create("", "", None).is_err());
    }

    #[test]
    fn task_states_have_stable_storage_values() {
        assert_eq!(TaskState::Backlog.as_str(), "backlog");
        assert_eq!(TaskState::parse("done").unwrap(), TaskState::Done);
    }
}
