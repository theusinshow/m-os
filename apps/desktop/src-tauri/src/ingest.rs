//! Os comandos da Universal Drop Zone.
//!
//! Esta camada e casca: ela cola o disco (`mos-ingest`), o banco
//! (`mos-storage-sqlite`) e as decisoes (`mos-core`), e nao decide nada por
//! conta propria. E casca de proposito — `cargo test -p mos-desktop` nao roda
//! nesta maquina (`SETUP-MAQUINA.md` §4), entao tudo que precisa de teste mora
//! nos crates que testam.
//!
//! A ordem dos passos e a promessa da feature, e ela esta escrita em
//! `ingest_finish`: o original vai para o lugar definitivo ANTES de qualquer
//! tentativa de entender o que ele e. Da linha do `mark_preserved` para baixo,
//! nada que falhe pode custar o arquivo.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use mos_core::{
    CaptureSource, CoreError, DropContext, ErrorCode, ExtractionState, Ingestion, IngestionId,
    IngestionReceipt, IngestionRepository, IngestionSource, IngestionState, NewCapture,
    NewIngestion, NewResource, ProjectHint, ProjectId, RelationPlan, ResourceId, ResourceKind,
    WorkspaceId,
};
use mos_ingest::{FileStore, Transfer};
use serde::Deserialize;
use tauri::{ipc::InvokeBody, AppHandle, Emitter, Manager};

use crate::AppState;

/// O que o renderer diz sobre o que esta soltando.
///
/// Tudo aqui e dado do usuario e nada aqui e confiado: nome vira rotulo, MIME e
/// palpite, tamanho e conferido de novo byte a byte no `mos-ingest`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DropDescriptor {
    pub name: String,
    #[serde(default)]
    pub mime: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub context: DropContextInput,
}

/// O contexto da tela no instante do drop, como o renderer o conhece: ids em
/// texto. A conversao para os tipos do dominio acontece aqui, e um id invalido
/// vira ausencia de contexto em vez de erro — perder a relacao e aceitavel,
/// perder o arquivo nao.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DropContextInput {
    #[serde(default)]
    pub page: String,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
}

impl DropContextInput {
    fn resolve(self) -> DropContext {
        DropContext {
            page: self.page.chars().take(40).collect(),
            project_id: self
                .project_id
                .filter(|value| !value.is_empty())
                .and_then(|value| ProjectId::parse(&value).ok()),
            workspace_id: self
                .workspace_id
                .filter(|value| !value.is_empty())
                .and_then(|value| WorkspaceId::parse(&value).ok()),
            task_id: self
                .task_id
                .filter(|value| !value.is_empty())
                .and_then(|value| mos_core::TaskId::parse(&value).ok()),
        }
    }
}

/// As transferencias em curso.
///
/// Um `Transfer` aberto e um arquivo aberto no staging. Ele existe apenas entre
/// `ingest_begin` e `ingest_finish`; se o processo morrer no meio, a abertura
/// seguinte limpa o staging e marca a ingestao como interrompida — o que nao se
/// perde, em nenhum desses caminhos, e a Capture.
pub struct IngestState {
    store: Arc<FileStore>,
    transfers: Mutex<HashMap<String, Transfer>>,
}

impl IngestState {
    pub fn new(store: FileStore) -> Self {
        Self {
            store: Arc::new(store),
            transfers: Mutex::new(HashMap::new()),
        }
    }
}

fn lock_error<T>(_: T) -> CoreError {
    CoreError::new(
        ErrorCode::StorageBusy,
        "A recepcao de arquivos esta ocupada.",
        true,
    )
}

// ---------------------------------------------------------------------------
// Comandos
// ---------------------------------------------------------------------------

/// Passo 1: abre a ingestao e grava a Capture.
///
/// Devolve antes de qualquer byte trafegar. E deliberado: a tela pode mostrar o
/// item na lista imediatamente porque, deste ponto em diante, o M/OS ja sabe
/// dizer o que a pessoa soltou mesmo que nada mais funcione.
#[tauri::command]
pub fn ingest_begin(
    descriptor: DropDescriptor,
    state: tauri::State<'_, AppState>,
    ingest: tauri::State<'_, IngestState>,
) -> Result<Ingestion, CoreError> {
    let request = NewIngestion::file(
        &descriptor.name,
        &descriptor.mime,
        descriptor.size,
        descriptor.context.resolve(),
    )?;
    let ingestion = open(&state, request)?;
    let transfer = ingest.store.receive(ingestion.id)?;
    ingest
        .transfers
        .lock()
        .map_err(lock_error)?
        .insert(ingestion.id.to_string(), transfer);
    Ok(ingestion)
}

/// Passo 2: mais um pedaco de arquivo.
///
/// Os bytes chegam no corpo BRUTO da chamada, e nao como JSON. Um `Vec<u8>`
/// serializado em JSON custa de tres a quatro vezes o tamanho do arquivo em
/// texto, e um PDF de 40 MB viraria mais de 120 MB de string atravessando a
/// ponte — o suficiente para a janela travar visivelmente.
#[tauri::command]
pub fn ingest_chunk(
    request: tauri::ipc::Request<'_>,
    ingest: tauri::State<'_, IngestState>,
) -> Result<(), CoreError> {
    let id = request
        .headers()
        .get("x-mos-ingestion")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            CoreError::new(
                ErrorCode::InvalidInput,
                "Pedaco de arquivo sem ingestao.",
                false,
            )
        })?
        .to_owned();
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err(CoreError::new(
            ErrorCode::InvalidInput,
            "Pedaco de arquivo precisa chegar como bytes.",
            false,
        ));
    };
    let mut transfers = ingest.transfers.lock().map_err(lock_error)?;
    let transfer = transfers.get_mut(&id).ok_or_else(|| {
        CoreError::new(
            ErrorCode::NotFound,
            "Esta transferencia nao esta aberta.",
            false,
        )
    })?;
    transfer.write(bytes)
}

/// Passo 3: fecha o arquivo e so entao tenta entender o que ele e.
///
/// A ordem desta funcao E a especificacao. Cada bloco abaixo pode falhar, e o
/// que muda e o custo da falha: antes do `mark_preserved`, perde-se o arquivo;
/// depois dele, perde-se apenas o entendimento — e a Capture continua na Inbox
/// para a pessoa decidir.
#[tauri::command]
pub fn ingest_finish(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    ingest: tauri::State<'_, IngestState>,
) -> Result<IngestionReceipt, CoreError> {
    let ingestion_id = IngestionId::parse(id)?;
    let transfer = ingest
        .transfers
        .lock()
        .map_err(lock_error)?
        .remove(id)
        .ok_or_else(|| {
            CoreError::new(
                ErrorCode::NotFound,
                "Esta transferencia nao esta aberta.",
                false,
            )
        })?;

    let opened = state.storage.get_ingestion(ingestion_id)?;
    let extension = mos_core::extension_of(&opened.original_name);

    // --- Fronteira da promessa -------------------------------------------
    let preserved = match transfer
        .finish()
        .and_then(|finished| ingest.store.commit(finished, &extension))
    {
        Ok(preserved) => preserved,
        Err(error) => return Err(abandon(&state, &app, ingestion_id, &error.message)),
    };
    let image_size = if opened.detected_kind == mos_core::DetectedKind::Image {
        ingest
            .store
            .read(&preserved.stored_path)
            .ok()
            .and_then(|bytes| mos_core::image_size(&bytes))
    } else {
        None
    };
    let ingestion = state.storage.mark_preserved(
        ingestion_id,
        &preserved.sha256,
        preserved.byte_size,
        &preserved.stored_path,
        None,
        image_size,
    )?;
    // --- Daqui para baixo, nada mais custa o arquivo ----------------------

    let receipt = derive(&state, &app, ingestion)?;
    // Duplicata nao le de novo: o texto daqueles bytes ja foi lido pela ingestao
    // que os trouxe da primeira vez, e reler custaria segundos de CPU para
    // gravar a mesma coisa num lugar que a busca nem alcanca.
    if receipt.ingestion.resource_id.is_some() && receipt.ingestion.detected_kind.has_text() {
        spawn_extraction(&app, ingest.store.clone(), receipt.ingestion.clone());
    }
    Ok(receipt)
}

/// Desiste de uma transferencia em curso.
///
/// Chamado quando o renderer falha no meio da leitura do arquivo. O staging e
/// apagado — um arquivo truncado nao e o original — e a Capture fica na Inbox.
#[tauri::command]
pub fn ingest_abort(
    id: &str,
    reason: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    ingest: tauri::State<'_, IngestState>,
) -> Result<(), CoreError> {
    let ingestion_id = IngestionId::parse(id)?;
    if let Some(transfer) = ingest.transfers.lock().map_err(lock_error)?.remove(id) {
        transfer.abort();
    }
    abandon(&state, &app, ingestion_id, reason);
    Ok(())
}

/// Texto solto: entra pelo mesmo pipeline, sem passar pelo disco.
///
/// O conteudo JA e o texto, e a Capture ja o preserva com a durabilidade do
/// `synchronous=FULL`. Guardar uma copia em arquivo seria uma segunda verdade
/// sobre o mesmo texto.
#[tauri::command]
pub fn ingest_text(
    text: &str,
    context: DropContextInput,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<IngestionReceipt, CoreError> {
    let request = NewIngestion::text(text, context.resolve())?;
    let ingestion = open_with_content(&state, request, text)?;
    // Texto vira Capture e para por ai. Ele ja esta na Inbox, ja e pesquisavel e
    // ja pode virar Task, Note ou Resource pelo caminho que existe. Criar um
    // Resource automaticamente seria decidir, com confianca nenhuma, o que a
    // frase significa — exatamente o que o §13 do briefing proibe.
    let ingestion = state
        .storage
        .mark_preserved(ingestion.id, "", 0, "", None, None)?;
    let closed = state.storage.complete_as_capture(ingestion.id)?;
    crate::notify_data_changed(&app, "ingestion-completed");
    Ok(IngestionReceipt {
        destination: "Inbox".to_owned(),
        duplicate: false,
        ingestion: closed,
    })
}

/// URL: vira Resource de site, com o dominio servindo de titulo.
#[tauri::command]
pub fn ingest_url(
    url: &str,
    context: DropContextInput,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<IngestionReceipt, CoreError> {
    let request = NewIngestion::url(url, context.resolve())?;
    // O endereco normalizado e a identidade do link, e por isso ele e hasheado
    // como se fosse conteudo: soltar a mesma URL de novo cai no mesmo caminho de
    // deduplicacao que soltar o mesmo arquivo de novo.
    let identity = mos_ingest::hash_of(request.original_name.as_bytes());
    let ingestion = open(&state, request)?;
    let ingestion = state
        .storage
        .mark_preserved(ingestion.id, &identity, 0, "", None, None)?;
    derive(&state, &app, ingestion)
}

/// Desfaz uma ingestao recente.
#[tauri::command]
pub fn ingest_undo(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), CoreError> {
    state.storage.undo_ingestion(IngestionId::parse(id)?)?;
    crate::notify_data_changed(&app, "ingestion-undone");
    Ok(())
}

/// Aceita a sugestao de relacao que o sistema nao teve confianca para aplicar.
#[tauri::command]
pub fn ingest_accept_suggestion(
    id: &str,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), CoreError> {
    let ingestion = state.storage.get_ingestion(IngestionId::parse(id)?)?;
    let (Some(resource), Some(project)) = (
        ingestion.resource_id.or(ingestion.duplicate_of),
        ingestion.suggested_project_id,
    ) else {
        return Err(CoreError::new(
            ErrorCode::InvalidTransition,
            "Esta ingestao nao tem sugestao a aceitar.",
            false,
        ));
    };
    mos_core::ResourceRepository::set_resource_project(
        state.storage.as_ref(),
        resource,
        project,
        true,
    )?;
    crate::notify_data_changed(&app, "ingestion-related");
    Ok(())
}

/// As ingestoes que resultaram em Resource, para a Library saber o que cada
/// arquivo e sem uma chamada por linha.
#[tauri::command]
pub fn list_ingestions(state: tauri::State<'_, AppState>) -> Result<Vec<Ingestion>, CoreError> {
    state.storage.file_ingestions()
}

/// Abre o original no programa padrao do Windows.
///
/// Recusa o que o shell executaria (§24): um `.exe` guardado continua guardado,
/// exportavel e pesquisavel — o que nao existe e o botao que o dispara.
#[tauri::command]
pub fn open_ingested_file(
    resource_id: &str,
    state: tauri::State<'_, AppState>,
    ingest: tauri::State<'_, IngestState>,
) -> Result<(), CoreError> {
    let path = stored_file(&state, &ingest, resource_id, true)?;
    crate::open_stored_path(&path)
}

/// Mostra o original na pasta, sem abri-lo.
///
/// E a saida para o que o M/OS se recusa a abrir: a pessoa continua dona do
/// arquivo e chega ate ele; quem decide executar e o Windows, a pedido dela, e
/// nao o M/OS.
#[tauri::command]
pub fn reveal_ingested_file(
    resource_id: &str,
    state: tauri::State<'_, AppState>,
    ingest: tauri::State<'_, IngestState>,
) -> Result<(), CoreError> {
    let path = stored_file(&state, &ingest, resource_id, false)?;
    crate::reveal_stored_path(&path)
}

// ---------------------------------------------------------------------------
// Costura
// ---------------------------------------------------------------------------

fn open(state: &tauri::State<'_, AppState>, request: NewIngestion) -> Result<Ingestion, CoreError> {
    let content = mos_core::capture_content(request.source, &request.original_name);
    open_with_content(state, request, &content)
}

fn open_with_content(
    state: &tauri::State<'_, AppState>,
    request: NewIngestion,
    content: &str,
) -> Result<Ingestion, CoreError> {
    let capture = NewCapture::create(content, CaptureSource::Drop)?;
    state.storage.begin_ingestion(request, capture)
}

/// Encerra uma ingestao que nao chegou a preservar nada.
///
/// Devolve o erro recebido para que o chamador o repasse: a falha e do arquivo,
/// e nao do M/OS, e a tela precisa dizer qual arquivo falhou.
fn abandon(
    state: &tauri::State<'_, AppState>,
    app: &AppHandle,
    id: IngestionId,
    reason: &str,
) -> CoreError {
    let _ = state
        .storage
        .fail_ingestion(id, IngestionState::Failed, reason);
    crate::notify_data_changed(app, "ingestion-failed");
    CoreError::new(ErrorCode::Io, reason.to_owned(), true)
}

/// Le o contexto e monta o plano de relacoes.
fn plan_for(state: &tauri::State<'_, AppState>, ingestion: &Ingestion) -> RelationPlan {
    // A lista de Projects so e carregada quando pode mudar a resposta: sem
    // contexto de Project, o nome do arquivo e a unica pista que sobra.
    let hints = if ingestion.context.project_id.is_none() {
        state
            .work
            .projects(false)
            .map(|projects| {
                projects
                    .into_iter()
                    .map(|project| ProjectHint {
                        id: project.id,
                        name: project.name,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    mos_core::plan_relations(&ingestion.context, &ingestion.original_name, &hints)
}

/// Cria (ou reconhece) a entidade e fecha a ingestao.
fn derive(
    state: &tauri::State<'_, AppState>,
    app: &AppHandle,
    ingestion: Ingestion,
) -> Result<IngestionReceipt, CoreError> {
    let plan = plan_for(state, &ingestion);

    if let Some(existing) = state
        .storage
        .duplicate_of(&ingestion.sha256, ingestion.id)?
    {
        let closed = state
            .storage
            .complete_as_duplicate(ingestion.id, existing, &plan)?;
        crate::notify_data_changed(app, "ingestion-completed");
        return Ok(IngestionReceipt {
            destination: destination_label(state, &plan),
            duplicate: true,
            ingestion: closed,
        });
    }

    let resource = match ingestion.source {
        IngestionSource::DropUrl => NewResource::create(
            ResourceKind::Site,
            &mos_core::host_of(&ingestion.original_name),
            &ingestion.original_name,
            "",
            ingestion.capture_id,
        )?,
        _ => NewResource::create(
            ResourceKind::File,
            &ingestion.original_name,
            "",
            "",
            ingestion.capture_id,
        )?,
    };
    let (closed, _) = state
        .storage
        .complete_ingestion(ingestion.id, resource, &plan)?;
    crate::notify_data_changed(app, "ingestion-completed");
    Ok(IngestionReceipt {
        destination: destination_label(state, &plan),
        duplicate: false,
        ingestion: closed,
    })
}

/// O nome do lugar, para o recibo dizer onde a coisa foi parar.
fn destination_label(state: &tauri::State<'_, AppState>, plan: &RelationPlan) -> String {
    if let Some(project) = plan.link_project {
        if let Ok(project) = state.work.project(&project.to_string()) {
            return project.name;
        }
    }
    if let Some(workspace) = plan.link_workspace {
        if let Ok(workspace) = state.work.workspace(&workspace.to_string()) {
            return workspace.name;
        }
    }
    "Library".to_owned()
}

/// Le o conteudo do original numa thread propria.
///
/// Fora da thread principal porque abrir um PDF de trezentas paginas leva
/// segundos, e a janela nao pode congelar por causa de um enriquecimento que,
/// por definicao, ja nao era necessario para guardar o arquivo.
fn spawn_extraction(app: &AppHandle, store: Arc<FileStore>, ingestion: Ingestion) {
    let app = app.clone();
    std::thread::spawn(move || {
        let outcome = match store.read(&ingestion.stored_path) {
            Ok(bytes) => mos_ingest::extract(ingestion.detected_kind, &bytes),
            Err(error) => mos_ingest::Extraction {
                state: ExtractionState::Failed,
                text: String::new(),
                page_count: None,
                error: error.message,
            },
        };
        let state = app.state::<AppState>();
        let written = state.storage.set_extraction(
            ingestion.id,
            outcome.state,
            &outcome.text,
            &outcome.error,
            outcome.page_count,
        );
        if written.is_ok() {
            let _ = app.emit_to("main", "ingestion-extracted", ingestion.id.to_string());
        }
    });
}

/// O caminho absoluto do original de um Resource, validado.
fn stored_file(
    state: &tauri::State<'_, AppState>,
    ingest: &tauri::State<'_, IngestState>,
    resource_id: &str,
    require_openable: bool,
) -> Result<std::path::PathBuf, CoreError> {
    let resource = ResourceId::parse(resource_id)?;
    let ingestion = state
        .storage
        .ingestion_for_resource(resource)?
        .ok_or_else(|| {
            CoreError::new(
                ErrorCode::NotFound,
                "Este Resource nao tem arquivo guardado.",
                false,
            )
        })?;
    if ingestion.stored_path.is_empty() {
        return Err(CoreError::new(
            ErrorCode::NotFound,
            "Este Resource nao tem arquivo guardado.",
            false,
        ));
    }
    if require_openable && !mos_core::is_openable(&ingestion.original_name) {
        return Err(CoreError::new(
            ErrorCode::InvalidTransition,
            "O M/OS nao abre este tipo de arquivo. Use 'mostrar na pasta'.",
            false,
        ));
    }
    let path = ingest.store.resolve(&ingestion.stored_path)?;
    if !path.exists() {
        return Err(CoreError::new(
            ErrorCode::NotFound,
            "O original nao esta mais no lugar onde foi guardado.",
            false,
        ));
    }
    Ok(path)
}

/// Reconcilia o que ficou pela metade quando o processo morreu.
///
/// Espelha `meeting::reconcile_on_open`: a abertura e o unico momento em que da
/// para saber que uma transferencia "em curso" na verdade acabou junto com o
/// processo anterior.
pub fn reconcile_on_open(app: &AppHandle, store: &FileStore) -> Result<usize, CoreError> {
    store.clear_staging()?;
    let state = app.state::<AppState>();
    let mut recovered = 0;
    for ingestion in state.storage.unfinished_ingestions()? {
        match ingestion.state {
            // Bytes pela metade: o arquivo nao existe, mas a Capture sim.
            IngestionState::Receiving => {
                state.storage.fail_ingestion(
                    ingestion.id,
                    IngestionState::Interrupted,
                    "O M/OS fechou durante a transferencia.",
                )?;
                recovered += 1;
            }
            // Preservado sem entidade: o arquivo existe. Vale terminar.
            IngestionState::Preserved => {
                if store.exists(&ingestion.stored_path) {
                    let _ = derive(&state, app, ingestion);
                } else {
                    state.storage.fail_ingestion(
                        ingestion.id,
                        IngestionState::Interrupted,
                        "O original nao foi encontrado na reabertura.",
                    )?;
                }
                recovered += 1;
            }
            _ => {}
        }
    }
    Ok(recovered)
}

/// Retoma leituras de conteudo que o fechamento interrompeu.
pub fn resume_extractions(app: &AppHandle, store: Arc<FileStore>) {
    let state = app.state::<AppState>();
    let pending = state.storage.pending_extractions().unwrap_or_default();
    for ingestion in pending {
        if ingestion.detected_kind.has_text() && store.exists(&ingestion.stored_path) {
            spawn_extraction(app, store.clone(), ingestion);
        }
    }
}
