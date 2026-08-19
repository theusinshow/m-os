use std::path::Path;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
    AppId, Capture, CaptureId, CoreError, HiddenWidget, LifecycleState, NewCapture, NewProject,
    NewRegisteredApp, NewTask, NewWorkspace, ProcessingState, Project, ProjectId, RegisteredApp,
    SearchItem, Task, TaskId, TaskState, Workspace, WorkspaceId,
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
    fn set_widget_hidden(
        &self,
        workspace_id: WorkspaceId,
        widget_id: &str,
        hidden: bool,
    ) -> Result<(), CoreError>;
    fn hidden_widgets(&self) -> Result<Vec<HiddenWidget>, CoreError>;

    fn widget_positions(&self) -> Result<Vec<crate::WidgetPosition>, CoreError>;

    /// Grava a ordem de um conjunto de widgets, numa transacao.
    ///
    /// Recebe a lista inteira e nao um movimento: gravar "o widget X foi para
    /// a posicao 3" obrigaria o banco a saber o que acontece com quem estava
    /// la, e essa regra ja existe no front, que e quem conhece a secao.
    fn set_widget_order(
        &self,
        workspace: crate::WorkspaceId,
        ordered: &[String],
    ) -> Result<Vec<crate::WidgetPosition>, CoreError>;
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
