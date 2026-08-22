use std::path::Path;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
    Assignment, AssignmentId, AssignmentStatus, Day, Exam, ExamId, ExamStatus, NewAssignment,
    NewExam, NewSemester, NewSubject, Resource, ResourceId, Semester, SemesterId, StudySession,
    StudySessionId, Subject, SubjectId,
    AppId, Capture, CaptureId, CoreError, HiddenWidget, LifecycleState, NewCapture, NewProject,
    NewRegisteredApp, NewReminder, NewTask, NewVoiceNote, NewWorkspace, ProcessingState, Project,
    ProjectId, RegisteredApp, Reminder, SearchItem, Task, TaskId, TaskState, VoiceNote,
    VoiceNoteId, Workspace, WorkspaceId,
};

#[derive(Clone, Debug)]
pub struct SearchRequest {
    pub query: String,
    pub include_archived: bool,
    pub limit: usize,
}

pub trait CaptureRepository: Send + Sync {
    fn create(&self, capture: NewCapture) -> Result<Capture, CoreError>;
    fn get(&self, id: CaptureId) -> Result<Capture, CoreError>;
    fn recent(&self, limit: usize) -> Result<Vec<Capture>, CoreError>;
    fn inbox(&self, limit: usize) -> Result<Vec<Capture>, CoreError>;
    /// As Captures entre dois instantes, da mais antiga para a mais nova.
    ///
    /// Existe para o Calendario. `recent` tem teto de 50, e um calendario que
    /// para de mostrar Capture depois da quinquagesima fica errado em silencio
    /// — o dia aparece vazio e nada indica que houve corte.
    fn captures_between(
        &self,
        since: time::OffsetDateTime,
        until: time::OffsetDateTime,
    ) -> Result<Vec<crate::Capture>, CoreError>;
    fn by_lifecycle(
        &self,
        lifecycle: LifecycleState,
        limit: usize,
    ) -> Result<Vec<Capture>, CoreError>;
    fn search(&self, request: SearchRequest) -> Result<Vec<Capture>, CoreError>;
    fn set_processing_state(
        &self,
        id: CaptureId,
        state: ProcessingState,
    ) -> Result<Capture, CoreError>;
    fn set_lifecycle_state(
        &self,
        id: CaptureId,
        state: LifecycleState,
    ) -> Result<Capture, CoreError>;
    fn rebuild_search(&self) -> Result<usize, CoreError>;
    /// Exclusao definitiva. Recusa o que ainda esta ativo: arquivar primeiro e a
    /// regra, e ela existe para que nenhum apagamento aconteca por engano no
    /// meio do uso normal.
    fn delete_capture(&self, id: CaptureId) -> Result<(), CoreError>;
}

pub trait WorkRepository: Send + Sync {
    fn create_workspace(&self, workspace: NewWorkspace) -> Result<Workspace, CoreError>;
    fn update_workspace(
        &self,
        id: WorkspaceId,
        name: &str,
        description: &str,
    ) -> Result<Workspace, CoreError>;
    fn get_workspace(&self, id: WorkspaceId) -> Result<Workspace, CoreError>;
    fn workspaces(&self, include_archived: bool) -> Result<Vec<Workspace>, CoreError>;
    fn set_workspace_lifecycle(
        &self,
        id: WorkspaceId,
        lifecycle: LifecycleState,
    ) -> Result<Workspace, CoreError>;
    fn workspace_projects(
        &self,
        id: WorkspaceId,
        include_archived: bool,
    ) -> Result<Vec<Project>, CoreError>;
    fn workspace_apps(
        &self,
        id: WorkspaceId,
        include_archived: bool,
    ) -> Result<Vec<RegisteredApp>, CoreError>;
    fn project_workspaces(&self, id: ProjectId) -> Result<Vec<Workspace>, CoreError>;
    fn app_workspaces(&self, id: AppId) -> Result<Vec<Workspace>, CoreError>;
    fn set_project_workspace(
        &self,
        project_id: ProjectId,
        workspace_id: WorkspaceId,
        linked: bool,
    ) -> Result<(), CoreError>;
    fn set_app_workspace(
        &self,
        app_id: AppId,
        workspace_id: WorkspaceId,
        linked: bool,
    ) -> Result<(), CoreError>;
    /// Exclusao definitiva. As tres recusam o que ainda esta ativo.
    fn delete_task(&self, id: TaskId) -> Result<(), CoreError>;
    fn delete_project(&self, id: ProjectId) -> Result<(), CoreError>;
    fn delete_workspace(&self, id: WorkspaceId) -> Result<(), CoreError>;
    /// A LINHA SIGNIFICA OCULTO — ausencia dela significa visivel. `workspace_id`
    /// ausente e a visao "Todos", que esconde os proprios widgets.
    fn set_widget_hidden(
        &self,
        workspace_id: Option<WorkspaceId>,
        widget_id: &str,
        hidden: bool,
    ) -> Result<(), CoreError>;
    fn hidden_widgets(&self) -> Result<Vec<HiddenWidget>, CoreError>;

    fn widget_placements(&self) -> Result<Vec<crate::WidgetPlacement>, CoreError>;

    /// Grava o arranjo de um conjunto de widgets, numa transacao.
    ///
    /// Recebe a lista inteira e nao um movimento: gravar "o widget X foi para
    /// a posicao 3" obrigaria o banco a saber o que acontece com quem estava
    /// la, e essa regra ja existe no front, que e quem conhece a faixa.
    ///
    /// Mover entre faixas manda as DUAS faixas na mesma chamada, pela mesma
    /// razao — a de origem tambem mudou, e as duas precisam cair juntas.
    ///
    /// `workspace` ausente e a visao "Todos", que arruma a propria Home.
    fn set_widget_layout(
        &self,
        workspace: Option<crate::WorkspaceId>,
        placements: &[crate::WidgetPlacementInput],
    ) -> Result<Vec<crate::WidgetPlacement>, CoreError>;

    /// Apaga o arranjo de um Workspace, devolvendo a Home ao desenho.
    ///
    /// APAGAR e a operacao certa, e nao gravar os valores do catalogo: a
    /// inversao da 0016 diz que ausencia de linha significa o desenho, entao
    /// gravar o desenho de hoje seria petrifica-lo — exatamente o que o resto
    /// desta feature evita.
    fn reset_widget_layout(
        &self,
        workspace: Option<crate::WorkspaceId>,
    ) -> Result<Vec<crate::WidgetPlacement>, CoreError>;

    /// Todas as petalas fixadas, de todos os escopos.
    ///
    /// Devolve tudo de uma vez pelo mesmo motivo de `widget_placements`: sao
    /// poucas linhas, e uma chamada so deixa a troca de Workspace filtrar em
    /// memoria em vez de ir ao core a cada clique.
    fn radial_pins(&self) -> Result<Vec<crate::RadialPin>, CoreError>;

    /// Fixa UMA petala. Um slot por chamada, e nao a lista inteira: aqui nao ha
    /// o efeito em cadeia que obriga `set_widget_layout` a receber tudo junto —
    /// a posicao de uma petala nao depende das outras, e essa independencia e
    /// justamente o que o leque promete a memoria muscular.
    fn set_radial_pin(
        &self,
        workspace: Option<crate::WorkspaceId>,
        pin: crate::RadialPinInput,
    ) -> Result<Vec<crate::RadialPin>, CoreError>;

    /// Devolve um slot ao desenho, APAGANDO a linha.
    ///
    /// Mesma razao do `reset_widget_layout`: a 0021 diz que ausencia de linha
    /// significa o padrao, entao gravar o padrao de hoje o petrificaria.
    fn clear_radial_pin(
        &self,
        workspace: Option<crate::WorkspaceId>,
        slot: i64,
    ) -> Result<Vec<crate::RadialPin>, CoreError>;
    fn create_project(&self, project: NewProject) -> Result<Project, CoreError>;
    fn update_project(
        &self,
        id: ProjectId,
        name: &str,
        description: &str,
        repository: &str,
    ) -> Result<Project, CoreError>;
    fn get_project(&self, id: ProjectId) -> Result<Project, CoreError>;
    fn projects(&self, include_archived: bool) -> Result<Vec<Project>, CoreError>;
    fn set_project_lifecycle(
        &self,
        id: ProjectId,
        lifecycle: LifecycleState,
    ) -> Result<Project, CoreError>;
    fn create_task(&self, task: NewTask) -> Result<Task, CoreError>;
    fn create_task_from_capture(
        &self,
        capture_id: CaptureId,
        task: NewTask,
    ) -> Result<Task, CoreError>;
    /// Task, Reminder e o processamento da Capture — **numa transacao so**.
    ///
    /// Os tres juntos, e nao em sequencia, pela mesma razao do
    /// `accept_insight`: existe um instante entre eles em que a Task existe e o
    /// aviso dela nao, e uma queda ali deixa o compromisso mudo. Numa acao
    /// derivada de voz isso e pior ainda, porque ninguem digitou nada — a
    /// pessoa falou, viu "lembrete criado" e foi embora.
    ///
    /// O Reminder aponta para a Task, e nao para a Capture: quando ele tocar,
    /// "o que eu tenho de fazer?" tem de ter resposta sem passar pela Inbox.
    fn create_task_from_capture_with_reminder(
        &self,
        capture_id: CaptureId,
        task: NewTask,
        reminder: Option<NewReminder>,
    ) -> Result<(Task, Option<Reminder>), CoreError>;
    fn update_task(
        &self,
        id: TaskId,
        title: &str,
        description: &str,
        project_id: Option<ProjectId>,
    ) -> Result<Task, CoreError>;
    fn get_task(&self, id: TaskId) -> Result<Task, CoreError>;
    fn tasks(&self, include_archived: bool) -> Result<Vec<Task>, CoreError>;
    fn set_task_state(&self, id: TaskId, state: TaskState) -> Result<Task, CoreError>;
    fn set_task_lifecycle(&self, id: TaskId, lifecycle: LifecycleState) -> Result<Task, CoreError>;
    fn search_all(&self, request: SearchRequest) -> Result<Vec<SearchItem>, CoreError>;
    fn rebuild_all_search(&self) -> Result<usize, CoreError>;
}

pub trait AppRepository: Send + Sync {
    fn create_app(&self, app: NewRegisteredApp) -> Result<RegisteredApp, CoreError>;
    fn register_catalog_apps(
        &self,
        apps: Vec<NewRegisteredApp>,
    ) -> Result<Vec<RegisteredApp>, CoreError>;
    /// `fields` carrega os campos editaveis ja validados; so `id` e
    /// `created_at` sao ignorados. Agrupar aqui evita uma assinatura de oito
    /// parametros onde trocar dois `Option<&str>` de lugar compila em silencio.
    fn update_app(
        &self,
        id: AppId,
        fields: &crate::NewRegisteredApp,
        capabilities: crate::AppCapabilities,
    ) -> Result<RegisteredApp, CoreError>;
    fn get_app(&self, id: AppId) -> Result<RegisteredApp, CoreError>;
    fn apps(&self, include_archived: bool) -> Result<Vec<RegisteredApp>, CoreError>;
    fn set_app_lifecycle(
        &self,
        id: AppId,
        lifecycle: LifecycleState,
    ) -> Result<RegisteredApp, CoreError>;
    fn mark_app_opened(&self, id: AppId) -> Result<RegisteredApp, CoreError>;
    fn search_apps(&self, request: SearchRequest) -> Result<Vec<RegisteredApp>, CoreError>;
    fn rebuild_app_search(&self) -> Result<usize, CoreError>;
    /// Exclusao definitiva. Recusa o que ainda esta ativo.
    fn delete_app(&self, id: AppId) -> Result<(), CoreError>;
}

pub trait ResourceRepository: Send + Sync {
    fn create_resource(&self, resource: crate::NewResource) -> Result<crate::Resource, CoreError>;
    fn update_resource(
        &self,
        id: crate::ResourceId,
        kind: crate::ResourceKind,
        title: &str,
        url: &str,
        note: &str,
    ) -> Result<crate::Resource, CoreError>;
    fn get_resource(&self, id: crate::ResourceId) -> Result<crate::Resource, CoreError>;
    fn resources(&self, include_archived: bool) -> Result<Vec<crate::Resource>, CoreError>;
    fn trashed_resources(&self) -> Result<Vec<crate::Resource>, CoreError>;
    fn set_resource_lifecycle(
        &self,
        id: crate::ResourceId,
        lifecycle: LifecycleState,
    ) -> Result<crate::Resource, CoreError>;
    fn search_resources(&self, request: SearchRequest) -> Result<Vec<crate::Resource>, CoreError>;
    fn rebuild_resource_search(&self) -> Result<usize, CoreError>;
    fn set_resource_workspace(
        &self,
        resource_id: crate::ResourceId,
        workspace_id: crate::WorkspaceId,
        linked: bool,
    ) -> Result<(), CoreError>;
    fn resource_workspaces(&self) -> Result<Vec<crate::ResourceWorkspace>, CoreError>;
    /// O elo que faltava na cadeia da fase 3 do ROADMAP: um Resource pertence ao
    /// Project a que ele serve. N-para-N pelo mesmo motivo de
    /// `resource_workspaces`: o mesmo memorial pode valer em dois Projects.
    fn set_resource_project(
        &self,
        resource_id: crate::ResourceId,
        project_id: crate::ProjectId,
        linked: bool,
    ) -> Result<(), CoreError>;
    fn resource_projects(&self) -> Result<Vec<crate::ResourceProject>, CoreError>;
    /// Exclusao definitiva. Recusa o que ainda esta ativo.
    fn delete_resource(&self, id: crate::ResourceId) -> Result<(), CoreError>;
}

/// Persistencia do rastreio de tempo por Project (ADR-032, etapa B).
///
/// Le e escreve o tempo REAL. Arredondamento e desconto de inatividade nao
/// aparecem em assinatura nenhuma daqui de proposito: sao decisao de
/// apresentacao, e um repositorio que os aplicasse tornaria impossivel recuperar
/// o que de fato aconteceu.
pub trait TimeTrackingRepository: Send + Sync {
    fn create_time_entry(&self, entry: crate::NewTimeEntry) -> Result<crate::TimeEntry, CoreError>;
    /// Todas as sessoes vivas, ou so as de um Project.
    fn time_entries(
        &self,
        project_id: Option<crate::ProjectId>,
    ) -> Result<Vec<crate::TimeEntry>, CoreError>;
    /// Corrige uma sessao ja gravada.
    ///
    /// O Project NAO entra: mover hora entre Projects mexeria no snapshot de
    /// valor/hora, e reprecificar em silencio pode alterar um valor ja
    /// faturado. Corrigir o rotulo e corrigir o preco sao duas decisoes, e
    /// misturar as duas dentro de um formulario de edicao esconde a segunda.
    fn update_time_entry(
        &self,
        id: crate::TimeEntryId,
        edit: crate::TimeEntryEdit,
    ) -> Result<crate::TimeEntry, CoreError>;
    /// O que esta na lixeira, da remocao mais recente para a mais antiga.
    ///
    /// Existe porque soft delete sem tela de lixeira e so uma forma educada de
    /// perder: o registro fica no banco, mas ninguem consegue chegar nele.
    fn trashed_time_entries(&self) -> Result<Vec<crate::TimeEntry>, CoreError>;
    /// Soft delete: hora de trabalho e registro de cobranca e sai da vista sem
    /// sair do banco.
    fn trash_time_entry(&self, id: crate::TimeEntryId) -> Result<(), CoreError>;
    /// O inverso de `trash_time_entry`, e o que faz o desfazer existir aqui
    /// como existe no resto do M/OS (ADR-035).
    fn restore_time_entry(&self, id: crate::TimeEntryId) -> Result<(), CoreError>;
    fn set_project_tracking(
        &self,
        tracking: crate::ProjectTracking,
    ) -> Result<crate::ProjectTracking, CoreError>;
    fn project_tracking(&self) -> Result<Vec<crate::ProjectTracking>, CoreError>;
    /// O cronometro em curso, se houver.
    fn active_timer(&self) -> Result<Option<crate::ActiveTimer>, CoreError>;
    /// Comeca a contar. RECUSA se ja houver um cronometro.
    ///
    /// Recusar em vez de substituir e a regra: encerrar o anterior por conta
    /// descartaria tempo que o usuario nao mandou descartar.
    fn start_timer(&self, start: crate::StartTimer) -> Result<crate::ActiveTimer, CoreError>;
    /// Pausa ou retoma. Cada transicao grava na hora — um estado que so vive na
    /// memoria some junto com a janela, e o trabalho nao.
    fn set_timer_running(&self, running: bool) -> Result<crate::ActiveTimer, CoreError>;
    /// Encerra e devolve a sessao gravada. Nada e descartado em silencio.
    fn stop_timer(&self) -> Result<crate::TimeEntry, CoreError>;
    /// Joga fora o cronometro SEM gravar sessao.
    ///
    /// Existe para o caso real de ter iniciado no Project errado. E a unica
    /// operacao que descarta tempo, e por isso a interface tem que pedir
    /// confirmacao — o dominio nao pode ser a ultima defesa disso.
    fn discard_timer(&self) -> Result<(), CoreError>;
    fn clients(&self, include_archived: bool) -> Result<Vec<crate::Client>, CoreError>;
    fn create_client(&self, input: crate::ClientInput) -> Result<crate::Client, CoreError>;
    fn update_client(
        &self,
        id: crate::ClientId,
        input: crate::ClientInput,
    ) -> Result<crate::Client, CoreError>;
    /// Arquivar e nao apagar: o cliente pode estar em faturas ja emitidas, e
    /// remove-lo deixaria horas apontando para um pagador que sumiu.
    fn set_client_archived(
        &self,
        id: crate::ClientId,
        archived: bool,
    ) -> Result<crate::Client, CoreError>;
    fn tracking_settings(&self) -> Result<crate::TrackingSettings, CoreError>;
    fn set_tracking_settings(
        &self,
        settings: crate::TrackingSettings,
    ) -> Result<crate::TrackingSettings, CoreError>;
    /// Quem esta cobrando. So a fatura le isto.
    fn issuer(&self) -> Result<crate::Issuer, CoreError>;
    fn set_issuer(&self, issuer: crate::Issuer) -> Result<crate::Issuer, CoreError>;
}

/// O que o sistema observa: programas abertos e periodos parados.
///
/// Separado de `TimeTrackingRepository` porque observacao NAO vira hora
/// sozinha. O evento fica guardado, a Linha do Tempo mostra o vao, e quem
/// decide se aquilo foi trabalho e a pessoa — misturar os dois num repositorio
/// so abriria a porta para uma sessao nascer de um `app_opened`.
pub trait MonitoringRepository: Send + Sync {
    fn monitored_apps(&self) -> Result<Vec<crate::MonitoredApp>, CoreError>;
    fn save_monitored_app(
        &self,
        app: crate::MonitoredApp,
    ) -> Result<crate::MonitoredApp, CoreError>;
    fn delete_monitored_app(&self, id: &str) -> Result<(), CoreError>;
    /// Os eventos de um periodo, do mais antigo para o mais novo — a Linha do
    /// Tempo se le na ordem em que o dia aconteceu.
    fn activity_events(
        &self,
        since: time::OffsetDateTime,
        until: time::OffsetDateTime,
    ) -> Result<Vec<crate::ActivityEvent>, CoreError>;
    fn record_activity(
        &self,
        event: crate::NewActivityEvent,
    ) -> Result<crate::ActivityEvent, CoreError>;
    /// Marca o evento como resolvido, para o mesmo periodo nao ser reoferecido.
    fn mark_activity_processed(&self, id: crate::ActivityEventId) -> Result<(), CoreError>;
    /// O quanto o aplicativo olha por cima do ombro.
    fn monitoring_settings(&self) -> Result<crate::MonitoringSettings, CoreError>;
    fn set_monitoring_settings(
        &self,
        settings: crate::MonitoringSettings,
    ) -> Result<crate::MonitoringSettings, CoreError>;
}

/// Persistencia da conversa do Hermes (ADR-025).
///
/// Este trait vive no Core, e nao em `mos-hermes`, de proposito: a ponte
/// continua sem conhecer o banco, e a traducao entre `Outcome` e parte de
/// mensagem acontece no orquestrador do desktop — o unico lugar onde ponte e
/// dominio se encontram.
pub trait ConversationRepository: Send + Sync {
    fn create_conversation(
        &self,
        conversation: crate::NewConversation,
    ) -> Result<crate::Conversation, CoreError>;
    fn get_conversation(&self, id: crate::ConversationId)
        -> Result<crate::Conversation, CoreError>;
    fn conversations(
        &self,
        include_archived: bool,
        limit: usize,
    ) -> Result<Vec<crate::ConversationSummary>, CoreError>;
    fn set_conversation_title(
        &self,
        id: crate::ConversationId,
        title: &str,
    ) -> Result<crate::Conversation, CoreError>;
    /// Guarda o vinculo com a sessao da VPS. E o que faltava em disco: sem ele
    /// `session.resume` nunca rodava entre aberturas do app.
    fn set_conversation_session(
        &self,
        id: crate::ConversationId,
        hermes_session_id: Option<&str>,
    ) -> Result<crate::Conversation, CoreError>;
    fn set_conversation_lifecycle(
        &self,
        id: crate::ConversationId,
        lifecycle: LifecycleState,
    ) -> Result<crate::Conversation, CoreError>;
    fn delete_conversation(&self, id: crate::ConversationId) -> Result<(), CoreError>;

    fn append_message(&self, message: crate::NewMessage) -> Result<crate::Message, CoreError>;
    fn messages(&self, id: crate::ConversationId) -> Result<Vec<crate::Message>, CoreError>;
    /// Uma mensagem só. Resolver uma proposta precisa reler as partes atuais
    /// antes de reescrevê-las, senão duas propostas na mesma resposta se
    /// sobrescreveriam.
    fn message(&self, id: crate::MessageId) -> Result<crate::Message, CoreError>;
    /// Fecha uma mensagem substituindo as partes e o estado.
    ///
    /// E assim que o streaming termina: uma escrita por mensagem, nunca por
    /// delta. Um INSERT por token sob `synchronous=FULL` seria um fsync por
    /// token (ADR-017).
    fn finish_message(
        &self,
        id: crate::MessageId,
        status: crate::MessageStatus,
        parts: Vec<crate::PartBody>,
    ) -> Result<crate::Message, CoreError>;
    /// Apaga uma mensagem e tudo que veio depois dela na conversa.
    ///
    /// Regenerate e editar-e-reenviar precisam disso: a resposta antiga e o que
    /// veio atras dela deixam de valer quando a pergunta muda.
    fn truncate_from(&self, message_id: crate::MessageId) -> Result<(), CoreError>;
    /// Troca a conversa inteira pelo que veio da VPS.
    fn replace_messages(
        &self,
        id: crate::ConversationId,
        messages: Vec<crate::NewMessage>,
    ) -> Result<(), CoreError>;
    fn search_conversations(
        &self,
        request: SearchRequest,
    ) -> Result<Vec<crate::ConversationSummary>, CoreError>;
    /// Fecha mensagens que ficaram `pending` ou `streaming`.
    ///
    /// O app pode ter sido fechado no meio de um turno, e sem isto a resposta
    /// voltaria eternamente em curso na proxima abertura.
    fn settle_unfinished_messages(&self) -> Result<usize, CoreError>;
    fn rebuild_conversation_search(&self) -> Result<usize, CoreError>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupReceipt {
    pub path: String,
    pub bytes: u64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInspection {
    pub path: String,
    pub schema_version: u32,
    pub capture_count: u64,
    pub bytes: u64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

/// Persistencia do Attention System.
///
/// Duas listas de leitura e nao uma consulta generica: `waiting` e o que o
/// agendador precisa a cada acordada, e `open` e o que a superficie mostra.
/// Sao perguntas diferentes com indices diferentes, e uma consulta que
/// servisse as duas usaria o indice errado para uma delas.
pub trait AttentionRepository: Send + Sync {
    fn create_reminder(&self, reminder: crate::NewReminder) -> Result<crate::Reminder, CoreError>;

    fn reminder(&self, id: crate::ReminderId) -> Result<crate::Reminder, CoreError>;

    /// Ativos e esperando a hora. E o que alimenta `next_wake` e `reconcile`.
    fn waiting_reminders(&self) -> Result<Vec<crate::Reminder>, CoreError>;

    /// Ativos e nao terminais. E o que o Attention Center e o badge leem.
    fn open_reminders(&self) -> Result<Vec<crate::Reminder>, CoreError>;

    /// Grava o resultado de uma transicao. Devolve o que ficou gravado, e nao
    /// o que foi mandado: quem le depois le do banco, nunca da memoria de quem
    /// escreveu.
    fn save_reminder(&self, reminder: &crate::Reminder) -> Result<crate::Reminder, CoreError>;

    fn set_reminder_lifecycle(
        &self,
        id: crate::ReminderId,
        state: crate::LifecycleState,
    ) -> Result<crate::Reminder, CoreError>;

    fn record_notification(
        &self,
        notification: crate::NewNotification,
    ) -> Result<crate::Notification, CoreError>;

    fn save_notification(
        &self,
        notification: &crate::Notification,
    ) -> Result<crate::Notification, CoreError>;

    /// A entrega viva com esta chave, se houver. E o que o dedupe consulta
    /// antes de criar outra.
    fn live_notification(
        &self,
        dedupe_key: &str,
    ) -> Result<Option<crate::Notification>, CoreError>;

    fn notifications_for(
        &self,
        reminder: crate::ReminderId,
    ) -> Result<Vec<crate::Notification>, CoreError>;
}
/// Persistencia do Meeting Agent.
///
/// Duas regras que a assinatura impoe, e nao a documentacao:
///
/// 1. **`replace_transcript` e `replace_analysis` substituem tudo de uma vez.**
///    Nao ha `append_segment` nem `add_insight`. Transcrever de novo produz uma
///    transcricao inteira, e analisar de novo produz uma analise inteira —
///    metade de uma transcricao no banco seria uma reuniao que le errado sem
///    nada falhar.
/// 2. **Nao ha `delete_meeting_audio` aqui.** Apagar bytes e trabalho de
///    filesystem, e este trait so conhece o banco. Ele apenas MARCA
///    (`mark_audio_deleted`), e quem apaga e o adapter — que assim nao pode
///    apagar sem registrar.
pub trait MeetingRepository: Send + Sync {
    fn create_meeting(&self, meeting: crate::NewMeeting) -> Result<crate::Meeting, CoreError>;
    fn meeting(&self, id: crate::MeetingId) -> Result<crate::Meeting, CoreError>;
    fn meetings(&self, include_archived: bool) -> Result<Vec<crate::Meeting>, CoreError>;

    /// Grava o resultado de uma transicao. Devolve o que FICOU GRAVADO, e nao o
    /// que foi mandado: quem le depois le do banco, nunca da memoria de quem
    /// escreveu. Mesma regra do `save_reminder`.
    fn save_meeting(&self, meeting: &crate::Meeting) -> Result<crate::Meeting, CoreError>;

    /// As reunioes que o processo anterior deixou em captura.
    ///
    /// E a consulta da reconciliacao de abertura (§9.1). Uma linha em
    /// `recording` ou `stopping` num processo recem-nascido significa,
    /// necessariamente, que o anterior morreu sem terminar.
    fn capturing_meetings(&self) -> Result<Vec<crate::Meeting>, CoreError>;

    /// Reunioes cujo audio a politica de retencao ja autoriza apagar.
    fn meetings_with_deletable_audio(
        &self,
        now: time::OffsetDateTime,
    ) -> Result<Vec<crate::Meeting>, CoreError>;

    fn mark_audio_deleted(
        &self,
        id: crate::MeetingId,
        at: time::OffsetDateTime,
    ) -> Result<crate::Meeting, CoreError>;

    fn set_meeting_project(
        &self,
        id: crate::MeetingId,
        project_id: Option<crate::ProjectId>,
    ) -> Result<crate::Meeting, CoreError>;

    fn set_meeting_title(
        &self,
        id: crate::MeetingId,
        title: &str,
    ) -> Result<crate::Meeting, CoreError>;

    /// Grava as anotacoes. Vazio e valido: apagar tudo e uma escolha.
    fn set_meeting_notes(
        &self,
        id: crate::MeetingId,
        notes: &str,
    ) -> Result<crate::Meeting, CoreError>;

    fn set_meeting_lifecycle(
        &self,
        id: crate::MeetingId,
        lifecycle: LifecycleState,
    ) -> Result<crate::Meeting, CoreError>;

    /// Troca a transcricao inteira, numa transacao.
    fn replace_transcript(
        &self,
        id: crate::MeetingId,
        segments: Vec<crate::TranscriptSegment>,
    ) -> Result<usize, CoreError>;

    fn transcript(
        &self,
        id: crate::MeetingId,
    ) -> Result<Vec<crate::TranscriptSegment>, CoreError>;

    /// Troca resumo e itens inteiros, numa transacao.
    ///
    /// Os itens ja ACEITOS sao preservados: uma reanalise nao pode desfazer uma
    /// Task que a pessoa criou. Sem essa regra, reanalisar apagaria a
    /// proveniencia de trabalho que ja existe no M/OS.
    fn replace_analysis(
        &self,
        analysis: crate::MeetingAnalysis,
        insights: Vec<crate::MeetingInsight>,
    ) -> Result<usize, CoreError>;

    fn analysis(
        &self,
        id: crate::MeetingId,
    ) -> Result<Option<crate::MeetingAnalysis>, CoreError>;

    fn insights(&self, id: crate::MeetingId) -> Result<Vec<crate::MeetingInsight>, CoreError>;

    /// Marca um item como aceito e liga a Task e ao Reminder criados, numa
    /// transacao com a criacao deles.
    fn link_insight_result(
        &self,
        insight_id: crate::InsightId,
        task_id: Option<crate::TaskId>,
        reminder_id: Option<crate::ReminderId>,
    ) -> Result<crate::MeetingInsight, CoreError>;

    fn set_insight_status(
        &self,
        insight_id: crate::InsightId,
        status: crate::InsightStatus,
    ) -> Result<crate::MeetingInsight, CoreError>;

    /// A reuniao a que um item pertence.
    ///
    /// Existe porque o corpo do Reminder cita o titulo da reuniao: quando ele
    /// tocar amanha as 9h, "de onde veio isto?" precisa ter resposta sem abrir
    /// mais nada.
    fn insights_meeting(&self, insight_id: crate::InsightId)
        -> Result<crate::MeetingId, CoreError>;

    /// Cria Task, opcionalmente Reminder, e liga o item aos dois — **numa
    /// transacao so**.
    ///
    /// Os tres juntos, e nao em sequencia, porque existe um instante entre eles
    /// em que a Task existe e o lembrete dela nao. Uma queda ali deixaria o
    /// compromisso sem aviso, que e exatamente o modo de falhar que esta feature
    /// existe para nao ter.
    ///
    /// Recusa item que nao esteja em `proposed`: aceitar duas vezes criaria duas
    /// Tasks para o mesmo compromisso.
    fn accept_insight(
        &self,
        accept: crate::AcceptInsight,
        task: crate::NewTask,
        reminder: Option<crate::NewReminder>,
    ) -> Result<crate::AcceptedInsight, CoreError>;

    /// Compromissos meus que continuam em aberto, de todas as reunioes.
    ///
    /// E uma query, e nao uma pergunta de linguagem: *"quais compromissos de
    /// reunioes eu ainda nao conclui?"* tem resposta em SQL, e onde a regra
    /// deterministica serve ela ganha da IA (§15.3).
    fn open_commitments(&self) -> Result<Vec<crate::MeetingInsight>, CoreError>;

    fn search_meetings(&self, request: SearchRequest) -> Result<Vec<crate::Meeting>, CoreError>;

    /// Busca na transcricao. Devolve (reuniao, trecho), DEDUPLICADO por reuniao:
    /// uma reuniao, um resultado, mesmo que a palavra apareca quarenta vezes.
    fn search_transcripts(
        &self,
        request: SearchRequest,
    ) -> Result<Vec<(crate::Meeting, String)>, CoreError>;

    fn rebuild_meeting_search(&self) -> Result<usize, CoreError>;
}

pub trait DataMaintenance: Send + Sync {
    fn create_backup(&self, destination: &Path) -> Result<BackupReceipt, CoreError>;
    fn inspect_backup(&self, source: &Path) -> Result<BackupInspection, CoreError>;
    fn restore_backup(&self, source: &Path) -> Result<BackupReceipt, CoreError>;
    fn ensure_daily_snapshot(&self) -> Result<Option<BackupReceipt>, CoreError>;
    fn export_json(&self, destination: &Path) -> Result<BackupReceipt, CoreError>;
}

/// Persistencia da ingestao universal.
///
/// A fronteira aqui e mais estreita que um CRUD de proposito: cada metodo e um
/// PASSO do pipeline, e cada passo e uma transacao. `begin` grava a Capture e a
/// linha de ingestao juntas; `complete` grava o Resource, as relacoes e o novo
/// estado juntos. Nenhum caminho permite gravar metade de um passo — que e como
/// a atomicidade da secao 23 do spec vira codigo em vez de intencao.
pub trait IngestionRepository: Send + Sync {
    /// Abre a ingestao e grava a Capture na MESMA transacao.
    ///
    /// A Capture existir antes dos bytes e o que sustenta a promessa: se tudo
    /// falhar daqui para frente, a Inbox ainda diz o que a pessoa soltou.
    fn begin_ingestion(
        &self,
        ingestion: crate::NewIngestion,
        capture: NewCapture,
    ) -> Result<crate::Ingestion, CoreError>;

    /// O original chegou inteiro ao lugar definitivo.
    fn mark_preserved(
        &self,
        id: crate::IngestionId,
        sha256: &str,
        byte_size: u64,
        stored_path: &str,
        page_count: Option<u32>,
        image_size: Option<crate::ImageSize>,
    ) -> Result<crate::Ingestion, CoreError>;

    /// O Resource vivo que ja guarda estes mesmos bytes, se houver.
    fn duplicate_of(
        &self,
        sha256: &str,
        except: crate::IngestionId,
    ) -> Result<Option<crate::ResourceId>, CoreError>;

    /// Cria o Resource, aplica as relacoes e fecha a ingestao — em uma transacao.
    fn complete_ingestion(
        &self,
        id: crate::IngestionId,
        resource: crate::NewResource,
        plan: &crate::RelationPlan,
    ) -> Result<(crate::Ingestion, crate::Resource), CoreError>;

    /// Fecha a ingestao cuja unica entidade derivada e a propria Capture.
    ///
    /// E o caminho do texto solto: ele ja esta preservado na Capture, com a
    /// durabilidade do `synchronous=FULL`, e criar um Resource automaticamente
    /// seria decidir por inferencia o que a frase significa. A Capture continua
    /// na Inbox — que e onde uma decisao por tomar deve estar.
    fn complete_as_capture(
        &self,
        id: crate::IngestionId,
    ) -> Result<crate::Ingestion, CoreError>;

    /// Fecha a ingestao apontando para um Resource que ja existia, aplicando
    /// nele o contexto novo. O que ja estava ligado permanece ligado, e a
    /// ingestao registra o que ELA acrescentou — sem isso, desfazer removeria
    /// contexto que nao era dela.
    fn complete_as_duplicate(
        &self,
        id: crate::IngestionId,
        existing: crate::ResourceId,
        plan: &crate::RelationPlan,
    ) -> Result<crate::Ingestion, CoreError>;

    /// Encerra sem entidade derivada. A Capture continua na Inbox.
    fn fail_ingestion(
        &self,
        id: crate::IngestionId,
        state: crate::IngestionState,
        failure: &str,
    ) -> Result<crate::Ingestion, CoreError>;

    /// Resultado da leitura de conteudo. Roda depois do recibo e nunca desfaz
    /// nada do que ja foi gravado.
    fn set_extraction(
        &self,
        id: crate::IngestionId,
        state: crate::ExtractionState,
        text: &str,
        error: &str,
        page_count: Option<u32>,
    ) -> Result<(), CoreError>;

    fn get_ingestion(&self, id: crate::IngestionId) -> Result<crate::Ingestion, CoreError>;

    /// A ingestao que produziu (ou aponta para) este Resource.
    fn ingestion_for_resource(
        &self,
        resource: crate::ResourceId,
    ) -> Result<Option<crate::Ingestion>, CoreError>;

    /// As ingestoes que resultaram em Resource, para a Library decorar a lista.
    fn file_ingestions(&self) -> Result<Vec<crate::Ingestion>, CoreError>;

    /// As que ficaram em `receiving` quando o processo morreu.
    fn unfinished_ingestions(&self) -> Result<Vec<crate::Ingestion>, CoreError>;

    /// As que foram preservadas e ainda esperam leitura de conteudo.
    fn pending_extractions(&self) -> Result<Vec<crate::Ingestion>, CoreError>;

    /// Desfaz o que ESTA ingestao criou, e nada alem disso.
    fn undo_ingestion(&self, id: crate::IngestionId) -> Result<(), CoreError>;

    fn rebuild_ingestion_search(&self) -> Result<usize, CoreError>;
}

/// Persistencia do Voice Inbox.
///
/// **Duas escritas atomicas, e nenhum `update_transcript`.** `capture_note`
/// grava a Capture e liga a nota a ela de uma vez; `realize` nao existe aqui
/// porque criar Task e trabalho de `WorkRepository`, e a nota nunca precisa
/// saber que uma Task nasceu — ela aponta para a Capture, e a Capture e que
/// carrega a proveniencia do resto do M/OS.
///
/// **Nao existe `delete_note_audio`.** Apagar bytes e trabalho de filesystem, e
/// este trait so conhece o banco. Ele apenas MARCA (`mark_audio_deleted`), e
/// quem apaga e o adapter — que assim nao pode apagar sem registrar. E a mesma
/// regra do `MeetingRepository`.
pub trait VoiceRepository: Send + Sync {
    fn create_note(&self, note: NewVoiceNote) -> Result<VoiceNote, CoreError>;
    fn note(&self, id: VoiceNoteId) -> Result<VoiceNote, CoreError>;

    /// Grava o resultado de uma transicao. Devolve o que FICOU GRAVADO, e nao o
    /// que foi mandado — mesma regra do `save_reminder` e do `save_meeting`.
    fn save_note(&self, note: &VoiceNote) -> Result<VoiceNote, CoreError>;

    /// As notas que ainda guardam informacao que o banco nao tem.
    ///
    /// E a consulta da reconciliacao de abertura: uma linha em `recording` num
    /// processo recem-nascido significa, necessariamente, que o anterior morreu
    /// sem terminar. E uma em `failed` e audio esperando um retry.
    fn unfinished_notes(&self) -> Result<Vec<VoiceNote>, CoreError>;

    /// Cria a Capture e fecha a nota sobre ela, numa transacao.
    ///
    /// A nota chega ja transicionada — o dominio decide, o adapter grava. Uma
    /// Capture sem nota apontando para ela seria uma transcricao sem origem; uma
    /// nota apontando para uma Capture que nao existe seria pior.
    fn capture_note(
        &self,
        note: &VoiceNote,
        capture: NewCapture,
    ) -> Result<(VoiceNote, Capture), CoreError>;

    fn mark_audio_deleted(
        &self,
        id: VoiceNoteId,
        at: time::OffsetDateTime,
    ) -> Result<VoiceNote, CoreError>;

    /// Exclusao definitiva de uma nota que nao virou nada.
    ///
    /// Recusa nota com Capture: apagar a origem de um texto que continua no
    /// banco deixaria a proveniencia apontando para o vazio.
    fn delete_note(&self, id: VoiceNoteId) -> Result<(), CoreError>;
}

/// Persistencia da Daily Session.
///
/// Duas regras que a assinatura impoe, e nao a documentacao:
///
/// 1. **`start_day` recebe a sessao E os objetivos, e fecha a sessao velha na
///    mesma chamada.** Nao ha `create_session` sozinho. Comecar o dia e um
///    gesto so, e um instante entre "a sessao existe" e "ela tem objetivos"
///    deixaria a Home mostrando um dia vazio que a pessoa acabou de montar.
///    Fechar a anterior entra junto pelo mesmo motivo: se ela ficasse aberta, o
///    banco teria dois dias `active` e a pergunta "qual e o dia de hoje?"
///    passaria a ter duas respostas.
/// 2. **`end_day` recebe os destinos E a reflexao juntos.** Mesma razao: o
///    encerramento e uma decisao unica, e metade dela gravada e um dia que
///    mente sobre o proprio placar.
pub trait DailyRepository: Send + Sync {
    /// A sessao de uma data, se houver. `None` e "o dia nao comecou".
    fn session_on(&self, day: &crate::Day) -> Result<Option<crate::DailySession>, CoreError>;

    fn session(&self, id: crate::DailySessionId) -> Result<crate::DailySession, CoreError>;

    /// A sessao mais recente ANTERIOR a uma data. E de onde vem o carry-over.
    fn session_before(&self, day: &crate::Day)
        -> Result<Option<crate::DailySession>, CoreError>;

    /// A sessao de um dia anterior que ficou `active`.
    ///
    /// Existe separada de `session_before` porque as duas perguntas sao
    /// diferentes: uma e "o que sobrou de ontem?", a outra e "ficou alguma
    /// porta aberta?". Um dia encerrado responde a primeira e nao a segunda.
    fn stale_session(&self, day: &crate::Day)
        -> Result<Option<crate::DailySession>, CoreError>;

    fn objectives(
        &self,
        session: crate::DailySessionId,
    ) -> Result<Vec<crate::DailyObjective>, CoreError>;

    /// Os objetivos de VARIAS sessoes, numa consulta.
    ///
    /// E o que impede a tela de historico de fazer uma consulta por dia listado
    /// — trinta dias virariam trinta idas ao banco para desenhar trinta linhas.
    fn objectives_of(
        &self,
        sessions: &[crate::DailySessionId],
    ) -> Result<Vec<crate::DailyObjective>, CoreError>;

    fn objective(
        &self,
        id: crate::DailyObjectiveId,
    ) -> Result<crate::DailyObjective, CoreError>;

    /// Comeca o dia: fecha o que ficou aberto de dias anteriores, cria a sessao
    /// e grava os objetivos — **numa transacao so**.
    ///
    /// Recusa se ja existir sessao para aquela data: uma segunda sessao no mesmo
    /// dia partiria o dia em dois placares.
    fn start_day(
        &self,
        session: crate::NewDailySession,
        objectives: Vec<crate::NewDailyObjective>,
        now: time::OffsetDateTime,
    ) -> Result<crate::DailySession, CoreError>;

    /// Acrescenta um objetivo a uma sessao que ja existe.
    ///
    /// Quando ele nasce `main`, o principal anterior e rebaixado na MESMA
    /// transacao: dois principais e um dia sem principal nenhum.
    fn add_objective(
        &self,
        objective: crate::NewDailyObjective,
    ) -> Result<crate::DailyObjective, CoreError>;

    /// Grava o que mudou num objetivo. Devolve o que FICOU gravado, e nao o que
    /// foi mandado — mesma regra do `save_reminder` e do `save_meeting`.
    fn save_objective(
        &self,
        objective: &crate::DailyObjective,
    ) -> Result<crate::DailyObjective, CoreError>;

    /// Promove um objetivo a principal, rebaixando o anterior na mesma
    /// transacao.
    fn set_main_objective(
        &self,
        id: crate::DailyObjectiveId,
        now: time::OffsetDateTime,
    ) -> Result<Vec<crate::DailyObjective>, CoreError>;

    /// Tira o objetivo do dia, de vez.
    ///
    /// Aqui APAGA, e a excecao e deliberada: o M/OS arquiva em vez de apagar
    /// porque o que ele guarda tem valor de memoria, e um objetivo removido
    /// antes de o dia acabar nunca chegou a ser historia. Quem quer manter o
    /// registro usa `dropped`, que e a outra porta e continua no placar.
    fn remove_objective(&self, id: crate::DailyObjectiveId) -> Result<(), CoreError>;

    /// Regrava a ordem da sessao inteira, numa transacao.
    ///
    /// A lista toda e nao um movimento, pelo mesmo motivo do
    /// `set_widget_layout`: gravar "este foi para a posicao 2" obrigaria o banco
    /// a saber o que acontece com quem estava la.
    fn reorder_objectives(
        &self,
        session: crate::DailySessionId,
        order: &[crate::DailyObjectiveId],
        now: time::OffsetDateTime,
    ) -> Result<Vec<crate::DailyObjective>, CoreError>;

    /// Encerra o dia: resolve os pendentes, grava a reflexao e fecha a sessao —
    /// **numa transacao so**.
    fn end_day(
        &self,
        session: crate::DailySessionId,
        resolutions: &[(crate::DailyObjectiveId, crate::ObjectiveStatus)],
        reflection: Option<crate::NewDailyReflection>,
        now: time::OffsetDateTime,
    ) -> Result<crate::DailySession, CoreError>;

    /// Reabre um dia encerrado.
    ///
    /// Existe porque encerrar por engano as 16h nao pode custar o resto do dia.
    /// Recusa se ja houver outra sessao ativa — reabrir ontem com hoje aberto
    /// devolveria o banco ao estado de dois dias ativos que o `start_day` evita.
    fn reopen_day(
        &self,
        session: crate::DailySessionId,
        now: time::OffsetDateTime,
    ) -> Result<crate::DailySession, CoreError>;

    fn reflection(
        &self,
        session: crate::DailySessionId,
    ) -> Result<Option<crate::DailyReflection>, CoreError>;

    /// As sessoes mais recentes, da mais nova para a mais antiga.
    fn sessions(&self, limit: usize) -> Result<Vec<crate::DailySession>, CoreError>;

    /// Quantos elos a corrente de carry-over de um objetivo tem.
    ///
    /// Feito no banco porque a corrente pode ter dez dias, e segui-la em memoria
    /// exigiria carregar o historico inteiro para responder a um numero que
    /// aparece ao lado de um titulo.
    fn carry_depth(&self, id: crate::DailyObjectiveId) -> Result<usize, CoreError>;

    /// As reflexoes de VARIAS sessoes, numa consulta.
    ///
    /// A semana precisa de sete de uma vez, e o `history()` fazia uma consulta
    /// por dia listado. Mesma forma do `objectives_of`, e pelo mesmo motivo:
    /// trinta dias de historico nao podem custar trinta idas ao banco.
    fn reflections_of(
        &self,
        sessions: &[crate::DailySessionId],
    ) -> Result<Vec<crate::DailyReflection>, CoreError>;

    /// As sessoes de uma semana, da segunda ao domingo. As duas bordas entram.
    ///
    /// Recebe a `Week` e nao um par de datas: a semana e um tipo, e passar duas
    /// datas soltas abriria a porta para alguem montar uma janela de seis dias
    /// sem que nada reclamasse.
    fn sessions_between(&self, week: &crate::Week) -> Result<Vec<crate::DailySession>, CoreError>;

    /// O fecho de uma semana, se houver.
    fn weekly_review(&self, week: &crate::Week)
        -> Result<Option<crate::WeeklyReview>, CoreError>;

    /// Grava o fecho, ou corrige o texto de um que ja existe.
    ///
    /// UPSERT por semana, e `closed_at` NAO se move na correcao: quando a
    /// semana foi fechada e um fato, e o texto e outro.
    fn save_weekly_review(
        &self,
        review: crate::NewWeeklyReview,
        now: time::OffsetDateTime,
    ) -> Result<crate::WeeklyReview, CoreError>;

    /// Os fechos mais recentes, da semana mais nova para a mais antiga.
    fn weekly_reviews(&self, limit: usize) -> Result<Vec<crate::WeeklyReview>, CoreError>;

    /// Objetivos cujo titulo casa com o texto. Alimenta a Search unificada.
    fn search_objectives(
        &self,
        request: SearchRequest,
    ) -> Result<Vec<(crate::DailyObjective, crate::Day)>, CoreError>;
}

/// A persistencia do M/Academic.
///
/// Uma porta so para as cinco entidades, e nao cinco portas: elas nascem e
/// morrem juntas — apagar o semestre leva as disciplinas, e a disciplina leva as
/// atividades —, e separa-las faria uma transacao atravessar duas
/// implementacoes.
pub trait AcademicRepository: Send + Sync {
    // --- Semestre
    fn semesters(&self, include_archived: bool) -> Result<Vec<Semester>, CoreError>;
    fn create_semester(&self, semester: NewSemester) -> Result<Semester, CoreError>;
    fn update_semester(
        &self,
        id: SemesterId,
        name: &str,
        institution: &str,
        starts_on: &Day,
        ends_on: &Day,
    ) -> Result<Semester, CoreError>;
    fn set_semester_lifecycle(
        &self,
        id: SemesterId,
        state: LifecycleState,
    ) -> Result<Semester, CoreError>;

    // --- Disciplina
    fn subjects(&self, include_archived: bool) -> Result<Vec<Subject>, CoreError>;
    fn create_subject(&self, subject: NewSubject) -> Result<Subject, CoreError>;
    fn update_subject(
        &self,
        id: SubjectId,
        name: &str,
        code: &str,
        teacher: &str,
        accent: &str,
        notes: &str,
    ) -> Result<Subject, CoreError>;
    fn set_subject_lifecycle(
        &self,
        id: SubjectId,
        state: LifecycleState,
    ) -> Result<Subject, CoreError>;

    // --- Atividade
    fn assignments(&self, include_archived: bool) -> Result<Vec<Assignment>, CoreError>;
    fn create_assignment(&self, assignment: NewAssignment) -> Result<Assignment, CoreError>;
    fn update_assignment(&self, assignment: UpdateAssignment) -> Result<Assignment, CoreError>;
    fn set_assignment_status(
        &self,
        id: AssignmentId,
        status: AssignmentStatus,
    ) -> Result<Assignment, CoreError>;
    fn set_assignment_lifecycle(
        &self,
        id: AssignmentId,
        state: LifecycleState,
    ) -> Result<Assignment, CoreError>;
    /// Cria a Task do M/OS que executa esta atividade, e liga as duas.
    ///
    /// Numa transacao so: uma Task criada sem o vinculo seria uma tarefa orfa
    /// que ninguem relaciona de volta a faculdade, e um vinculo sem Task
    /// apontaria para o vazio.
    fn create_task_for_assignment(&self, id: AssignmentId) -> Result<Task, CoreError>;
    /// Desfaz o vinculo sem tocar na Task. Ela continua existindo — quem quer
    /// apagar a Task usa o caminho da Task.
    fn unlink_assignment_task(&self, id: AssignmentId) -> Result<Assignment, CoreError>;

    // --- Avaliacao
    fn exams(&self, include_archived: bool) -> Result<Vec<Exam>, CoreError>;
    fn create_exam(&self, exam: NewExam) -> Result<Exam, CoreError>;
    fn update_exam(&self, exam: UpdateExam) -> Result<Exam, CoreError>;
    fn set_exam_lifecycle(&self, id: ExamId, state: LifecycleState) -> Result<Exam, CoreError>;

    // --- Materiais
    fn subject_resources(&self, id: SubjectId) -> Result<Vec<Resource>, CoreError>;
    fn material_counts(&self) -> Result<Vec<(SubjectId, usize)>, CoreError>;
    fn link_material(
        &self,
        subject: SubjectId,
        resource: ResourceId,
        linked: bool,
    ) -> Result<(), CoreError>;

    // --- Estudo
    fn study_sessions(&self, limit: usize) -> Result<Vec<StudySession>, CoreError>;
    fn running_study(&self) -> Result<Option<StudySession>, CoreError>;
    fn start_study(&self, subject: SubjectId, topic: &str) -> Result<StudySession, CoreError>;
    /// Fecha a sessao aberta. `seconds` vem de fora porque a tela e quem sabe se
    /// houve pausa — o relogio de parede nao reproduz isso.
    fn finish_study(
        &self,
        id: StudySessionId,
        seconds: i64,
        notes: &str,
    ) -> Result<StudySession, CoreError>;
    fn discard_study(&self, id: StudySessionId) -> Result<(), CoreError>;

    // --- A decisao da pessoa
    //
    // Separada das mutacoes de `status` de proposito: aquelas descrevem o fato
    // academico e o sync as escreve; estas sao a decisao de quem estuda, e
    // nenhum provedor externo as toca. Ver `mos_core::academic_decision`.
    /// "Ja entreguei", "nao vou fazer", ou de volta a indefinido.
    fn set_assignment_decision(
        &self,
        id: AssignmentId,
        decision: crate::Decision,
    ) -> Result<Assignment, CoreError>;
    fn set_exam_decision(
        &self,
        id: ExamId,
        decision: crate::Decision,
    ) -> Result<Exam, CoreError>;
    /// Quando pretendo fazer, e por quanto tempo. `None` desfaz o plano.
    fn plan_assignment(
        &self,
        id: AssignmentId,
        plano: Option<crate::Plano>,
    ) -> Result<Assignment, CoreError>;
    fn plan_exam(&self, id: ExamId, plano: Option<crate::Plano>) -> Result<Exam, CoreError>;

    // --- Busca
    /// Disciplinas, avaliacoes e atividades que casam com o termo.
    ///
    /// LIKE e nao FTS, pela mesma razao do `search_objectives`: o volume e
    /// limitado por disciplinas vezes um punhado de itens — um curso inteiro sao
    /// centenas de linhas curtas, que cabem num scan. Uma tabela FTS a mais
    /// custaria uma projecao a manter em toda escrita para ganhar nada.
    fn search_academic(&self, request: SearchRequest) -> Result<Vec<SearchItem>, CoreError>;
}

/// Os campos editaveis de uma atividade.
///
/// Struct em vez de dez parametros: metade deles e `Option<f64>`, e trocar dois
/// de lugar compilaria sem reclamacao nenhuma.
#[derive(Clone, Debug)]
pub struct UpdateAssignment {
    pub id: AssignmentId,
    pub title: String,
    pub description: String,
    pub due_at: Option<OffsetDateTime>,
    pub priority: crate::Priority,
    pub weight: f64,
    pub score: Option<f64>,
    pub max_score: Option<f64>,
    pub status: AssignmentStatus,
}

#[derive(Clone, Debug)]
pub struct UpdateExam {
    pub id: ExamId,
    pub name: String,
    pub at: OffsetDateTime,
    pub location: String,
    pub topics: String,
    pub weight: f64,
    pub score: Option<f64>,
    pub max_score: Option<f64>,
    pub status: ExamStatus,
}
