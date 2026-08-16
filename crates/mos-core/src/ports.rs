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
    fn tracking_settings(&self) -> Result<crate::TrackingSettings, CoreError>;
    fn set_tracking_settings(
        &self,
        settings: crate::TrackingSettings,
    ) -> Result<crate::TrackingSettings, CoreError>;
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

pub trait DataMaintenance: Send + Sync {
    fn create_backup(&self, destination: &Path) -> Result<BackupReceipt, CoreError>;
    fn inspect_backup(&self, source: &Path) -> Result<BackupInspection, CoreError>;
    fn restore_backup(&self, source: &Path) -> Result<BackupReceipt, CoreError>;
    fn ensure_daily_snapshot(&self) -> Result<Option<BackupReceipt>, CoreError>;
    fn export_json(&self, destination: &Path) -> Result<BackupReceipt, CoreError>;
}
