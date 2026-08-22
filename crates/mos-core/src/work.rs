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

            /// O UUID cru, para quem enderessa esta entidade FORA do M/OS.
            ///
            /// Existe para a sincronizacao: o id que viaja entre dispositivos e
            /// o mesmo que identifica aqui, porque todo id do M/OS ja e UUID v7
            /// — ordenavel por tempo e sem colisao entre maquinas. Foi o que
            /// dispensou um mapa de "id local para id remoto".
            ///
            /// Continua sem `From<Uuid>`: construir um id a partir de um UUID
            /// qualquer e o caminho para um id que nao existe em lugar nenhum.
            /// Quem entra vem de `parse`, que valida.
            pub fn as_uuid(&self) -> Uuid {
                self.0
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
    /// Vazio e a visao "Todos", e nao um dado faltando (migration 0019).
    pub workspace_id: Option<WorkspaceId>,
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
    /// Um objetivo de um dia, com a data em que ele foi escrito.
    ///
    /// Entra na Search porque *"o que eu estava fazendo terca?"* e uma pergunta
    /// de verdade, e a resposta dela e o dia — nao a Task. Sem isto, a Daily
    /// Session seria o unico substantivo do M/OS que a busca nao alcanca, e um
    /// silo e exatamente o que o `CORE-FOUNDATION.md` §2 recusa.
    ///
    /// Carrega o `day` junto porque um objetivo sem data nao se distingue de
    /// outro: dois dias podem ter escrito a mesma frase, e a data e o que faz o
    /// resultado significar alguma coisa.
    DailyObjective {
        objective: crate::DailyObjective,
        day: crate::Day,
    },
    /// Uma disciplina, uma avaliacao ou uma atividade da faculdade.
    ///
    /// Entram na busca global pelo mesmo motivo do objetivo do dia: sem isto, o
    /// M/Academic seria o unico substantivo do M/OS que a busca nao alcanca, e
    /// silo e o que o `CORE-FOUNDATION.md` §2 recusa. Procurar "Estatica" tem de
    /// achar a disciplina, a P1 dela e a Task de exercicios — que ja aparece
    /// como Task.
    ///
    /// A avaliacao e a atividade carregam o NOME da disciplina junto: "P1"
    /// sozinha nao se distingue da P1 de outra materia.
    Subject {
        subject: crate::Subject,
    },
    Exam {
        exam: crate::Exam,
        subject: String,
    },
    Assignment {
        assignment: crate::Assignment,
        subject: String,
    },
    /// A REUNIAO, e nunca um segmento de transcricao.
    ///
    /// Uma reuniao de uma hora tem ~600 segmentos; tres reunioes dominariam
    /// qualquer busca por qualquer palavra comum. A transcricao tem indice
    /// proprio e chega aqui promovendo a Meeting, com o trecho como snippet
    /// (`MEETING-AGENT.md` §15).
    Meeting {
        meeting: crate::Meeting,
        project: Option<Project>,
        /// O trecho que casou, quando o acerto veio da transcricao. `None`
        /// quando casou por titulo, resumo ou item.
        snippet: Option<String>,
    },
}

/// Onde um widget foi posto na Home de um Workspace.
///
/// Espelha a inversao de `workspace_hidden_widgets`: **ausencia de linha
/// significa o que o desenho escolheu.** Workspace novo nao precisa de nenhuma
/// escrita, e widget criado depois nasce onde o catalogo o pos, em vez de
/// nascer no lugar que uma tabela vazia sortear.
///
/// A REGRA que resolve isto contra o catalogo NAO mora aqui. Ela vive no front,
/// em `apps/desktop/src/homeLayout.ts`, junto do catalogo de widgets — que e do
/// desenho da Home e nao do dominio. O `CORE.md` lista os conceitos que este
/// crate carrega (Capture, Inbox, Project, Task, Workspace); largura de widget
/// nao e um deles. O que fica deste lado e o que o BANCO precisa para nao
/// aceitar lixo: o tipo, e os validadores logo abaixo.
///
/// `section` e `span` sao `Option` pelo mesmo motivo, um degrau mais fundo:
/// dentro de uma linha que existe, o campo vazio continua significando "o que o
/// desenho escolheu". Sem isso, o primeiro arrasto de qualquer widget
/// petrificaria a largura e a faixa que ele tinha naquele dia, e mudar o
/// desenho depois nao alcancaria mais ninguem que ja tivesse arrumado a Home.
/// `workspace_id` vazio e a visao "Todos", e nao um dado faltando. Ela e um
/// contexto de verdade — o unico de quem nunca criou Workspace nenhum — e tem
/// arranjo proprio desde a migration 0018. Ver o comentario dela para por que o
/// NULL diz isso melhor que um id sentinela.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetPlacement {
    pub workspace_id: Option<WorkspaceId>,
    pub widget_id: String,
    pub position: i64,
    pub section: Option<String>,
    pub span: Option<i64>,
}

/// O que o front pede para gravar: a mesma linha, sem o Workspace, que vem por
/// fora porque a escrita inteira e de um Workspace so.
///
/// A escrita e AUTORITATIVA — o que chega aqui e o que fica gravado, campo por
/// campo. Nao ha "nao mexi neste": `span: None` significa **volte ao desenho**,
/// e e assim que se desfaz um redimensionamento. Um `COALESCE` no banco daria a
/// leitura oposta e tornaria impossivel voltar atras, que foi o motivo de ele
/// sair daqui.
///
/// `section` e obrigatoria porque posicao sem faixa nao quer dizer nada: sao a
/// mesma informacao — onde na Home o widget esta. E por isso reordenar uma
/// faixa FIXA a faixa dos widgets dela, de proposito: quem arrumou aquela faixa
/// escolheu quem mora nela, e o desenho mudar de ideia depois nao pode arrastar
/// um widget para fora de um arranjo que a pessoa montou. E a mesma regra que
/// faz widget novo ir para o fim, aplicada a outra dimensao.
///
/// `span` NAO segue essa regra, e a assimetria e deliberada: largura e uma
/// escolha ortogonal a arrumacao. Quem so arrastou nunca escolheu largura
/// nenhuma, entao reordenar tem de deixar `span: None` passar intacto — e a
/// responsabilidade de quem monta a lista.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetPlacementInput {
    pub widget_id: String,
    pub position: i64,
    pub section: String,
    pub span: Option<i64>,
}

/// Uma petala fixada no leque.
///
/// `workspace_id` nulo e a visao "Todos", e nao um dado faltando — mesma leitura
/// da 0018, agora na 0021. A AUSENCIA de linha para um slot tambem significa
/// algo: "o que o desenho escolheu". Quem resolve isso e `lequePetalas.ts`, que e a
/// unica copia da regra.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RadialPin {
    pub workspace_id: Option<WorkspaceId>,
    pub slot: i64,
    pub kind: String,
    pub target: String,
}

/// O que o front pede para fixar. Sem o Workspace, que vem por fora porque a
/// escrita e de um escopo so.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RadialPinInput {
    pub slot: i64,
    pub kind: String,
    pub target: String,
}

/// Quantas das doze colunas o widget ocupa.
///
/// Valida FORMA e nao vocabulario, igual ao id: a grade tem doze colunas e
/// desenha qualquer numero delas. Qual subconjunto a interface oferece
/// (3,4,5,6,8,9,12) e escolha de desenho, e desenho muda mais rapido que core.
pub fn validate_span(value: i64) -> Result<i64, CoreError> {
    if (1..=12).contains(&value) {
        Ok(value)
    } else {
        Err(CoreError::new(
            ErrorCode::InvalidInput,
            "A largura de um widget vai de 1 a 12 colunas.",
            false,
        ))
    }
}

/// Mesma forma do id de widget, e pelo mesmo motivo: as faixas da Home vivem no
/// front, e enum aqui faria de cada faixa nova uma migration.
pub fn validate_section_id(value: &str) -> Result<String, CoreError> {
    validate_widget_id(value).map_err(|_| {
        CoreError::new(
            ErrorCode::InvalidInput,
            "ID de faixa da Home invalido.",
            false,
        )
    })
}

/// Espelha o CHECK da migration 0008: minuscula inicial, depois minuscula,
/// digito ou `_`. O core valida forma, nao vocabulario — quem conhece o catalogo
/// de widgets e o front, em HOME_WIDGETS.
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

/// A forma de um `kind` de petala, e so a forma.
///
/// O vocabulario — `app`, `acao`, `pagina` — vive no front, em `lequePetalas.ts`, pelo
/// mesmo motivo que `widget_id` e opaco aqui: um enum no banco faria de cada
/// tipo novo de petala uma migration, e tipo de petala muda mais rapido que
/// schema. Espelha o CHECK da migration 0021.
pub fn validate_pin_kind(value: &str) -> Result<String, CoreError> {
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
            "Tipo de petala invalido.",
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

    // ------------------------------------------------ validacao do que entra

    #[test]
    fn a_span_outside_the_grid_is_refused() {
        assert!(validate_span(0).is_err());
        assert!(validate_span(13).is_err());
        assert!(validate_span(-1).is_err());
        assert_eq!(validate_span(1).unwrap(), 1);
        assert_eq!(validate_span(12).unwrap(), 12);
        assert_eq!(validate_span(7).unwrap(), 7, "forma, e nao o vocabulario do desenho");
    }

    #[test]
    fn a_section_id_follows_the_same_shape_as_a_widget_id() {
        assert_eq!(validate_section_id("overview").unwrap(), "overview");
        assert_eq!(validate_section_id("faixa_2").unwrap(), "faixa_2");
        assert!(validate_section_id("Overview").is_err());
        assert!(validate_section_id("2overview").is_err());
        assert!(validate_section_id("").is_err());
    }

    #[test]
    fn kind_de_petala_aceita_forma_e_recusa_lixo() {
        assert_eq!(validate_pin_kind("app").unwrap(), "app");
        assert_eq!(validate_pin_kind("  pagina  ").unwrap(), "pagina");
        assert_eq!(validate_pin_kind("acao_rapida").unwrap(), "acao_rapida");

        // Forma, e nao vocabulario: um kind novo passa sem migration.
        assert_eq!(validate_pin_kind("widget3").unwrap(), "widget3");

        for lixo in ["", "  ", "App", "3app", "app-ficha", "app.ficha", "açao"] {
            assert!(validate_pin_kind(lixo).is_err(), "deveria recusar {lixo:?}");
        }
    }
}
