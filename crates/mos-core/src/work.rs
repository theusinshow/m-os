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

/// Onde um widget foi posto na Home de um Workspace.
///
/// Espelha a inversao de `workspace_hidden_widgets`: **ausencia de linha
/// significa o que o desenho escolheu.** Workspace novo nao precisa de nenhuma
/// escrita, e widget criado depois nasce onde o catalogo o pos, em vez de
/// nascer no lugar que uma tabela vazia sortear.
///
/// `section` e `span` sao `Option` pelo mesmo motivo, um degrau mais fundo:
/// dentro de uma linha que existe, o campo vazio continua significando "o que o
/// desenho escolheu". Sem isso, o primeiro arrasto de qualquer widget
/// petrificaria a largura e a faixa que ele tinha naquele dia, e mudar o
/// desenho depois nao alcancaria mais ninguem que ja tivesse arrumado a Home.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetPlacement {
    pub workspace_id: WorkspaceId,
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

/// Um widget do catalogo, com a faixa e a largura que o DESENHO escolheu.
///
/// O core continua sem conhecer o catalogo — ele chega por parametro. O que
/// esta estrutura acrescenta e que o catalogo agora carrega tres coisas por
/// widget, e nao so o id: sem a faixa e a largura de origem, nao ha contra o
/// que comparar o que foi guardado.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetSlot {
    pub id: String,
    pub section: String,
    pub span: i64,
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
pub fn order_widgets(catalog: &[String], saved: &[WidgetPlacement]) -> Vec<String> {
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

/// Resolve o catalogo inteiro contra o que foi guardado: faixa, largura e ordem.
///
/// E a regra completa do arranjo da Home, e ela vive aqui porque e dominio. A
/// Home a repete em TypeScript (`arrangeHome`, em `App.tsx`) por uma razao
/// especifica: resolver isso e trabalho de cada render, e um round-trip por
/// render seria um custo desproporcional. As duas implementacoes apontam uma
/// para a outra de proposito — mudar uma sem a outra e o defeito a evitar.
///
/// Tres decisoes, e cada uma existe para um caso que da errado sozinho:
///
/// 1. campo guardado vazio significa "o que o desenho escolheu", e nao zero.
///    Um widget que foi so REORDENADO nao pode ter a largura congelada no valor
///    que ela tinha naquele dia;
/// 2. as faixas saem na ordem em que o catalogo as apresenta. Faixa que so
///    existe por escrita — alguem moveu um widget para uma faixa que o desenho
///    esvaziou — vai para o fim, em vez de sumir com o widget dentro dela;
/// 3. dentro da faixa vale `order_widgets`, com o desempate dele.
pub fn arrange_widgets(catalog: &[WidgetSlot], saved: &[WidgetPlacement]) -> Vec<WidgetSlot> {
    let resolved: Vec<WidgetSlot> = catalog
        .iter()
        .map(|slot| {
            let placement = saved.iter().find(|entry| entry.widget_id == slot.id);
            WidgetSlot {
                id: slot.id.clone(),
                section: placement
                    .and_then(|entry| entry.section.clone())
                    .unwrap_or_else(|| slot.section.clone()),
                span: placement.and_then(|entry| entry.span).unwrap_or(slot.span),
            }
        })
        .collect();

    // Primeiro as faixas do desenho, na ordem dele; depois as que so aparecem
    // porque alguem moveu um widget para la.
    let mut sections: Vec<String> = Vec::new();
    for slot in catalog.iter().map(|slot| &slot.section).chain(resolved.iter().map(|slot| &slot.section)) {
        if !sections.iter().any(|known| known == slot) {
            sections.push(slot.clone());
        }
    }

    let mut arranged = Vec::with_capacity(resolved.len());
    for section in sections {
        let ids: Vec<String> = resolved
            .iter()
            .filter(|slot| slot.section == section)
            .map(|slot| slot.id.clone())
            .collect();
        for id in order_widgets(&ids, saved) {
            if let Some(slot) = resolved.iter().find(|slot| slot.id == id) {
                arranged.push(slot.clone());
            }
        }
    }
    arranged
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

    fn placed(workspace: WorkspaceId, id: &str, position: i64) -> WidgetPlacement {
        WidgetPlacement {
            workspace_id: workspace,
            widget_id: id.to_owned(),
            position,
            section: None,
            span: None,
        }
    }

    /// O catalogo do arranjo: dois widgets em `agora`, dois em `visao`.
    fn slots() -> Vec<WidgetSlot> {
        [
            ("timer", "agora", 3),
            ("now", "agora", 6),
            ("today_hours", "visao", 3),
            ("inbox_pulse", "visao", 4),
        ]
        .iter()
        .map(|(id, section, span)| WidgetSlot {
            id: (*id).to_owned(),
            section: (*section).to_owned(),
            span: *span,
        })
        .collect()
    }

    fn shape(arranged: &[WidgetSlot]) -> Vec<(&str, &str, i64)> {
        arranged
            .iter()
            .map(|slot| (slot.id.as_str(), slot.section.as_str(), slot.span))
            .collect()
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

    // ------------------------------------------------ arranjo: faixa e largura

    /// Sem escrita nenhuma, o arranjo e o desenho — faixa, largura e ordem.
    #[test]
    fn without_anything_saved_the_design_wins() {
        assert_eq!(
            shape(&arrange_widgets(&slots(), &[])),
            [
                ("timer", "agora", 3),
                ("now", "agora", 6),
                ("today_hours", "visao", 3),
                ("inbox_pulse", "visao", 4),
            ]
        );
    }

    /// O caso que a inversao existe para proteger: a pessoa REORDENOU, e nada
    /// mais. Largura e faixa tem de continuar sendo as do desenho — se a
    /// reordenacao congelasse o valor efetivo, mudar o desenho de um widget
    /// nunca mais alcancaria quem tivesse arrastado qualquer coisa uma vez.
    #[test]
    fn reordering_never_freezes_width_or_section() {
        let workspace = WorkspaceId::new();
        let saved = [placed(workspace, "now", 0), placed(workspace, "timer", 1)];

        assert_eq!(
            shape(&arrange_widgets(&slots(), &saved)),
            [
                ("now", "agora", 6),
                ("timer", "agora", 3),
                ("today_hours", "visao", 3),
                ("inbox_pulse", "visao", 4),
            ]
        );
    }

    #[test]
    fn a_saved_span_beats_the_designed_one() {
        let workspace = WorkspaceId::new();
        let mut entry = placed(workspace, "timer", 0);
        entry.span = Some(12);

        let arranged = arrange_widgets(&slots(), &[entry]);
        assert_eq!(arranged[0], WidgetSlot { id: "timer".to_owned(), section: "agora".to_owned(), span: 12 });
        assert_eq!(arranged[1].span, 6, "o vizinho nao muda de largura");
    }

    /// Mover entre faixas e a operacao que mais mexe no resultado: o widget
    /// sai de uma lista e entra em outra, e a posicao guardada passa a valer
    /// contra os vizinhos NOVOS.
    #[test]
    fn a_saved_section_moves_the_widget_between_bands() {
        let workspace = WorkspaceId::new();
        let mut entry = placed(workspace, "timer", 0);
        entry.section = Some("visao".to_owned());

        assert_eq!(
            shape(&arrange_widgets(&slots(), &[entry])),
            [
                ("now", "agora", 6),
                ("timer", "visao", 3),
                ("today_hours", "visao", 3),
                ("inbox_pulse", "visao", 4),
            ],
            "sai de `agora`, e entra em `visao` na posicao que foi gravada"
        );
    }

    /// Uma faixa pode ficar vazia por escrita, e o desenho pode esvaziar outra.
    /// Nos dois casos ninguem some: a faixa que so existe porque alguem moveu
    /// um widget para la sai no fim, e nao desaparece com o widget dentro.
    #[test]
    fn a_section_that_only_exists_because_someone_moved_a_widget_still_renders() {
        let workspace = WorkspaceId::new();
        let mut entry = placed(workspace, "now", 0);
        entry.section = Some("acervo".to_owned());

        let arranged = arrange_widgets(&slots(), &[entry]);
        assert_eq!(arranged.len(), 4, "ninguem se perde");
        assert_eq!(
            shape(&arranged).last().copied(),
            Some(("now", "acervo", 6)),
            "a faixa fora do desenho vai para o fim"
        );
    }

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
}
