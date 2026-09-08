use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{Capture, CaptureId, CoreError, ErrorCode, LifecycleState, Priority, RegisteredApp};

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
entity_id!(ChecklistItemId, "Item de checklist");
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
    /// O contexto da Task: o que ela precisa dizer e nao cabe no titulo.
    ///
    /// NAO e o checklist, e a distincao e do produto: a descricao e informacao
    /// que se LE ("manter cobrimento de 5 cm, conforme a reuniao"); o item de
    /// checklist e trabalho que se CONCLUI. Misturar os dois faz uma lista de
    /// coisas que nunca terminam.
    pub description: String,
    pub project_id: Option<ProjectId>,
    pub source_capture_id: Option<CaptureId>,
    pub state: TaskState,
    pub lifecycle_state: LifecycleState,
    /// Quando o trabalho VENCE.
    ///
    /// Nao e o lembrete, e a diferenca e a razao de a coluna existir (ADR-066):
    /// o prazo descreve o compromisso, o lembrete descreve a interrupcao. Uma
    /// Task pode vencer as 17:00 e pedir aviso as 16:30, e ate 2026-09-08 o
    /// M/OS nao conseguia dizer isso — a decisao D-1 tinha recusado o campo.
    ///
    /// `None` e o estado normal: a maioria das Tasks nao vence dia nenhum.
    #[serde(with = "time::serde::rfc3339::option")]
    pub due_at: Option<OffsetDateTime>,
    /// A mesma escala do Reminder, e nao uma segunda. `Normal` e neutro na tela.
    pub priority: Priority,
    /// Quanto tempo isto deve levar, em minutos. `None` e "nao estimei", que
    /// nao e zero.
    pub estimate_minutes: Option<i64>,
    /// A Task de que esta e subtask. Subtask e Task de verdade, e nao um
    /// terceiro tipo — ver a migration 0039.
    pub parent_task_id: Option<TaskId>,
    /// A Task que precisa terminar antes desta poder andar.
    pub blocked_by_task_id: Option<TaskId>,
    /// Quem esta segurando esta Task. Texto, e nao entidade: nao existe
    /// cadastro de pessoas no M/OS, e criar um para escrever "Victor" seria
    /// construir um CRM por engano.
    pub waiting_for: String,
    /// Quando cobrar quem esta segurando. Diferente do prazo: a Task nao esta
    /// atrasada porque um terceiro nao respondeu.
    #[serde(with = "time::serde::rfc3339::option")]
    pub follow_up_at: Option<OffsetDateTime>,
    /// Quantos itens de checklist esta Task tem, e quantos estao concluidos.
    ///
    /// DERIVADOS, e nao colunas: eles saem de um `GROUP BY` na mesma consulta
    /// que traz a Task. Guarda-los seria um segundo lugar onde a verdade mora,
    /// e o primeiro `UPDATE` esquecido faria o card mentir.
    ///
    /// Vem junto da Task porque o card do Kanban precisa deles e NAO precisa do
    /// resto: pedir o checklist inteiro de cada card seria o N+1 que o desenho
    /// recusa.
    #[serde(default)]
    pub checklist_total: usize,
    #[serde(default)]
    pub checklist_done: usize,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub completed_at: Option<OffsetDateTime>,
}

impl Task {
    /// O progresso do checklist, de 0.0 a 1.0. `None` quando nao ha checklist —
    /// e ai a tela nao desenha barra nenhuma, em vez de desenhar uma vazia que
    /// parece trabalho nao comecado.
    pub fn checklist_progress(&self) -> Option<f64> {
        (self.checklist_total > 0).then(|| self.checklist_done as f64 / self.checklist_total as f64)
    }

    /// Todos os itens concluidos, e a Task ainda aberta.
    ///
    /// A tela usa isto para OFERECER a conclusao, e nunca para executa-la: uma
    /// Task que se fecha sozinha e o sistema afirmando algo que a pessoa nao
    /// disse — a mesma inclinacao que a ADR-035 gravou ao fazer o desfazer
    /// arquivar em vez de apagar.
    pub fn checklist_is_complete(&self) -> bool {
        self.checklist_total > 0
            && self.checklist_done == self.checklist_total
            && self.state != TaskState::Done
    }
}

/// Um passo dentro de uma Task.
///
/// # Por que ele nao e uma Task
///
/// Uma Task tem estado, prazo, prioridade, projeto e lugar no quadro. Um passo
/// tem texto, ordem e um risco de estar feito. Promover cada passo a Task
/// encheria o Kanban de cartoes de trinta segundos — que e exatamente o que o
/// M/OS nao quer ser.
///
/// A fronteira, escrita: **o que merece existir sozinho no quadro e Subtask; o
/// que so faz sentido dentro do trabalho maior e item de checklist.**
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistItem {
    pub id: ChecklistItemId,
    pub task_id: TaskId,
    pub label: String,
    pub position: i64,
    #[serde(with = "time::serde::rfc3339::option")]
    pub completed_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl ChecklistItem {
    pub fn completed(&self) -> bool {
        self.completed_at.is_some()
    }
}

/// O maior checklist que uma Task aceita de uma vez.
///
/// Teto e nao promessa: ele existe para uma colagem acidental de um documento
/// inteiro nao virar oitocentas linhas riscaveis. Vinte e cinco cobre qualquer
/// checklist que uma pessoa de fato executa; passou disso, o que existe ali sao
/// Subtasks ou outra Task.
pub const MAX_CHECKLIST_PASTE: usize = 25;

/// Texto colado vira uma lista de itens — sem IA, e sem adivinhacao.
///
/// # Por que determinismo, e nao modelo
///
/// A pessoa cola quatro linhas e quer quatro itens. Mandar isso para um modelo
/// custaria rede, espera e a chance de ele reescrever as palavras dela. O que
/// esta funcao faz cabe numa regra: uma linha e um item, e a decoracao de lista
/// que a origem trouxe (`-`, `*`, `1.`, `[ ]`, `[x]`) nao faz parte do texto.
///
/// O `[x]` NAO marca o item como concluido. Ele so e removido do texto: quem
/// cola uma lista esta descrevendo o trabalho, e presumir que metade dele ja
/// esta feita e o tipo de esperteza que faz perder confianca no sistema.
pub fn parse_checklist_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(strip_list_marker)
        .filter(|linha| !linha.is_empty())
        .take(MAX_CHECKLIST_PASTE)
        .collect()
}

/// Tira a decoracao de lista de UMA linha.
fn strip_list_marker(linha: &str) -> String {
    let mut resto = linha.trim();
    // O marcador de item: hifen, asterisco, bolinha, ou "12." / "12)".
    //
    // O corte e por PRIMEIRO CARACTERE e nao por prefixo de dois: "- " ja veio
    // aparado pelo `trim`, entao procurar o par nao acharia nada e a linha vazia
    // de um bullet solto viraria um item chamado "-".
    if resto
        .chars()
        .next()
        .is_some_and(|inicial| matches!(inicial, '-' | '*' | '•' | '–'))
    {
        let sem_bullet = &resto[resto.chars().next().map_or(0, char::len_utf8)..];
        // So e marcador se o que vem depois for espaco ou nada. Sem esta
        // guarda, "-5cm de cobrimento" perderia o sinal de menos.
        if sem_bullet.is_empty() || sem_bullet.starts_with(char::is_whitespace) {
            resto = sem_bullet.trim_start();
        }
    } else if let Some(posicao) = resto.find(['.', ')']) {
        let (numero, depois) = resto.split_at(posicao);
        if !numero.is_empty()
            && numero.len() <= 3
            && numero.chars().all(|c| c.is_ascii_digit())
            && depois[1..].starts_with(' ')
        {
            resto = depois[1..].trim_start();
        }
    }
    // A caixa, quando a origem ja escrevia checklist.
    for caixa in ["[ ]", "[x]", "[X]", "[]"] {
        if let Some(sem_caixa) = resto.strip_prefix(caixa) {
            resto = sem_caixa.trim_start();
            break;
        }
    }
    resto.trim().to_owned()
}

#[derive(Clone, Debug)]
pub struct NewTask {
    pub id: TaskId,
    pub title: String,
    pub description: String,
    pub project_id: Option<ProjectId>,
    pub due_at: Option<OffsetDateTime>,
    pub priority: Priority,
    pub estimate_minutes: Option<i64>,
    pub parent_task_id: Option<TaskId>,
    /// Os passos com que a Task nasce. Vazio e o caso comum — a criacao rapida
    /// continua sendo um titulo e mais nada.
    pub checklist: Vec<String>,
    pub created_at: OffsetDateTime,
}

impl NewTask {
    /// A criacao minima: titulo, contexto e Project.
    ///
    /// Os campos novos da 0039 entram por `with_*` e nao por parametro, e isso
    /// e deliberado: sete lugares do M/OS criam Task (voz, reuniao, faculdade,
    /// Capture, quadro, bolso, Hermes), e nenhum deles precisa saber de prazo
    /// para continuar funcionando. Um construtor de nove argumentos faria toda
    /// chamada existente carregar seis `None`.
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
            due_at: None,
            priority: Priority::Normal,
            estimate_minutes: None,
            parent_task_id: None,
            checklist: Vec::new(),
            created_at: OffsetDateTime::now_utc(),
        })
    }

    pub fn with_due_at(mut self, due_at: Option<OffsetDateTime>) -> Self {
        self.due_at = due_at;
        self
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_estimate(mut self, minutes: Option<i64>) -> Self {
        self.estimate_minutes = minutes.filter(|valor| *valor > 0);
        self
    }

    pub fn with_parent(mut self, parent: Option<TaskId>) -> Self {
        self.parent_task_id = parent;
        self
    }

    /// Os itens ja chegam limpos: vazio fora, decoracao de lista fora, teto
    /// aplicado. Assim nenhum caminho de entrada precisa lembrar da regra.
    pub fn with_checklist(mut self, itens: &[String]) -> Self {
        self.checklist = itens
            .iter()
            .map(|item| strip_list_marker(item))
            .filter(|item| !item.is_empty())
            .take(MAX_CHECKLIST_PASTE)
            .collect();
        self
    }
}

/// O que uma edicao de Task grava.
///
/// # Por que uma estrutura, e nao dez parametros
///
/// `update_task` tinha quatro parametros e ganharia mais seis. Trocar dois
/// `Option<OffsetDateTime>` de lugar por engano — prazo no lugar do follow-up —
/// compilaria sem uma reclamacao, e o defeito apareceria como uma Task que
/// vence no dia em que alguem deveria ser cobrado. E a mesma razao de
/// `calendar::ComposeInput` existir.
///
/// A escrita e **autoritativa**: o que chega aqui e o que fica gravado, campo
/// por campo. Nao ha "nao mexi neste" — `due_at: None` significa *tire o
/// prazo*, e e assim que se desfaz um prazo. Quem tem edicao parcial (o bolso
/// manda so o que mudou) le a Task atual e preenche o resto, que e o unico
/// lugar onde essa regra pode morar sem virar `COALESCE` no banco.
#[derive(Clone, Debug)]
pub struct EditTask {
    pub title: String,
    pub description: String,
    pub project_id: Option<ProjectId>,
    pub due_at: Option<OffsetDateTime>,
    pub priority: Priority,
    pub estimate_minutes: Option<i64>,
    pub parent_task_id: Option<TaskId>,
    pub blocked_by_task_id: Option<TaskId>,
    pub waiting_for: String,
    pub follow_up_at: Option<OffsetDateTime>,
}

impl EditTask {
    /// A edicao que nao muda nada, a partir da Task como ela esta.
    ///
    /// E o ponto de partida de toda edicao parcial: leia a Task, mude o campo,
    /// grave. Sem isto, cada superficie reescreveria a regra de "o que nao veio
    /// continua como estava" — e uma delas erraria.
    pub fn from_task(task: &Task) -> Self {
        Self {
            title: task.title.clone(),
            description: task.description.clone(),
            project_id: task.project_id,
            due_at: task.due_at,
            priority: task.priority,
            estimate_minutes: task.estimate_minutes,
            parent_task_id: task.parent_task_id,
            blocked_by_task_id: task.blocked_by_task_id,
            waiting_for: task.waiting_for.clone(),
            follow_up_at: task.follow_up_at,
        }
    }

    /// Valida o que o banco e o produto exigem, e devolve a edicao limpa.
    ///
    /// Duas regras que so podem morar aqui, porque as duas dependem de saber
    /// QUAL Task esta sendo editada:
    ///
    /// - uma Task nao e pai de si mesma;
    /// - uma Task nao se bloqueia sozinha.
    ///
    /// As duas produziriam um ciclo de tamanho um — e um ciclo de tamanho um e
    /// uma Task que nunca pode andar, desenhada por um clique distraido.
    pub fn validate(mut self, id: TaskId) -> Result<Self, CoreError> {
        self.title = required(&self.title, "O titulo da Task nao pode estar vazio.")?;
        self.description = self.description.trim().to_owned();
        self.waiting_for = self.waiting_for.trim().to_owned();
        self.estimate_minutes = self.estimate_minutes.filter(|valor| *valor > 0);
        if self.parent_task_id == Some(id) {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "Uma Task nao pode ser subtask dela mesma.",
                false,
            ));
        }
        if self.blocked_by_task_id == Some(id) {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "Uma Task nao pode estar bloqueada por ela mesma.",
                false,
            ));
        }
        Ok(self)
    }
}

/// O rascunho de um item de checklist.
#[derive(Clone, Debug)]
pub struct NewChecklistItem {
    pub id: ChecklistItemId,
    pub task_id: TaskId,
    pub label: String,
    pub created_at: OffsetDateTime,
}

impl NewChecklistItem {
    pub fn create(task_id: TaskId, label: &str) -> Result<Self, CoreError> {
        Ok(Self {
            id: ChecklistItemId::new(),
            task_id,
            label: required(
                &strip_list_marker(label),
                "O item de checklist nao pode estar vazio.",
            )?,
            created_at: OffsetDateTime::now_utc(),
        })
    }
}

/// A Task com tudo que a folha de detalhe mostra, numa ida so ao banco.
///
/// # Por que uma estrutura, e nao cinco chamadas
///
/// Abrir a gaveta pedia Task, checklist, subtasks, referencias e lembretes. Em
/// cinco chamadas isso sao cinco viagens e cinco estados de carregamento na
/// tela — e a gaveta piscaria montada pela metade. Aqui e uma consulta so, e a
/// folha aparece inteira ou nao aparece.
///
/// **O que NAO esta aqui e tao importante quanto o que esta.** O card do quadro
/// nao usa isto: ele usa `Task`, que ja carrega os dois numeros do progresso.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDetail {
    pub task: Task,
    pub checklist: Vec<ChecklistItem>,
    /// As Tasks filhas, na ordem em que foram criadas.
    pub subtasks: Vec<Task>,
    /// A Task que bloqueia esta, ja resolvida — a gaveta mostra o TITULO dela,
    /// e um id nao diz nada a ninguem.
    pub blocked_by: Option<Task>,
    /// Os Resources ligados a esta Task. Sao os mesmos Resources da Library:
    /// referencia aqui nao e um segundo sistema de anexo.
    pub references: Vec<crate::Resource>,
    /// Os lembretes que apontam para esta Task. Vem do Attention System, e nao
    /// de um agendador proprio.
    pub reminders: Vec<crate::Reminder>,
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
        assert_eq!(
            validate_span(7).unwrap(),
            7,
            "forma, e nao o vocabulario do desenho"
        );
    }

    #[test]
    fn a_section_id_follows_the_same_shape_as_a_widget_id() {
        assert_eq!(validate_section_id("overview").unwrap(), "overview");
        assert_eq!(validate_section_id("faixa_2").unwrap(), "faixa_2");
        assert!(validate_section_id("Overview").is_err());
        assert!(validate_section_id("2overview").is_err());
        assert!(validate_section_id("").is_err());
    }

    // ------------------------------------------------------------- checklist

    #[test]
    fn colar_linhas_vira_uma_lista_de_itens() {
        let itens = parse_checklist_lines(
            "Corrigir armadura\nAtualizar corte\n\nRevisar niveis\nEnviar para Victor",
        );
        assert_eq!(
            itens,
            [
                "Corrigir armadura",
                "Atualizar corte",
                "Revisar niveis",
                "Enviar para Victor"
            ]
        );
    }

    #[test]
    fn a_decoracao_de_lista_nao_faz_parte_do_texto() {
        let itens = parse_checklist_lines(
            "- Conferir niveis\n* Conferir formas\n1. Conferir armaduras\n2) Atualizar PDF\n• Enviar",
        );
        assert_eq!(
            itens,
            [
                "Conferir niveis",
                "Conferir formas",
                "Conferir armaduras",
                "Atualizar PDF",
                "Enviar"
            ]
        );
    }

    /// A caixa some do texto e NAO marca nada: quem cola uma lista esta
    /// descrevendo o trabalho, e presumir que metade ja esta feita e a esperteza
    /// que custa confianca.
    #[test]
    fn a_caixa_sai_do_texto_e_nao_conclui_o_item() {
        let itens = parse_checklist_lines("[x] Corrigir nivel\n[ ] Gerar PDF\n- [x] Enviar");
        assert_eq!(itens, ["Corrigir nivel", "Gerar PDF", "Enviar"]);
    }

    #[test]
    fn a_colagem_tem_teto() {
        let colado = (0..80)
            .map(|indice| format!("item {indice}"))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(parse_checklist_lines(&colado).len(), MAX_CHECKLIST_PASTE);
    }

    #[test]
    fn um_item_vazio_e_recusado() {
        let task = TaskId::new();
        assert!(NewChecklistItem::create(task, "   ").is_err());
        assert!(NewChecklistItem::create(task, "- ").is_err());
        assert_eq!(
            NewChecklistItem::create(task, "  - Gerar PDF ")
                .unwrap()
                .label,
            "Gerar PDF"
        );
    }

    #[test]
    fn a_task_nasce_sem_prazo_prioridade_neutra_e_sem_checklist() {
        let task = NewTask::create("Revisar projeto", "", None).unwrap();
        assert!(task.due_at.is_none());
        assert_eq!(task.priority, Priority::Normal);
        assert!(task.checklist.is_empty());
        assert!(task.parent_task_id.is_none());
    }

    #[test]
    fn estimativa_zero_ou_negativa_e_ausencia() {
        assert_eq!(
            NewTask::create("x", "", None)
                .unwrap()
                .with_estimate(Some(0))
                .estimate_minutes,
            None
        );
        assert_eq!(
            NewTask::create("x", "", None)
                .unwrap()
                .with_estimate(Some(30))
                .estimate_minutes,
            Some(30)
        );
    }

    #[test]
    fn o_progresso_so_existe_quando_ha_checklist() {
        let mut task = exemplo();
        assert_eq!(task.checklist_progress(), None);
        assert!(!task.checklist_is_complete());

        task.checklist_total = 6;
        task.checklist_done = 3;
        assert_eq!(task.checklist_progress(), Some(0.5));
        assert!(!task.checklist_is_complete());

        task.checklist_done = 6;
        assert!(
            task.checklist_is_complete(),
            "todos feitos, Task ainda aberta"
        );

        task.state = TaskState::Done;
        assert!(
            !task.checklist_is_complete(),
            "Task ja concluida nao se oferece para concluir de novo"
        );
    }

    fn exemplo() -> Task {
        Task {
            id: TaskId::new(),
            title: "Revisar projeto estrutural".into(),
            description: String::new(),
            project_id: None,
            source_capture_id: None,
            state: TaskState::Backlog,
            lifecycle_state: LifecycleState::Active,
            due_at: None,
            priority: Priority::Normal,
            estimate_minutes: None,
            parent_task_id: None,
            blocked_by_task_id: None,
            waiting_for: String::new(),
            follow_up_at: None,
            checklist_total: 0,
            checklist_done: 0,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
            completed_at: None,
        }
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
