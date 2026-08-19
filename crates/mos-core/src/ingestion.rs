//! Ingestao universal: o caminho unico por onde qualquer coisa entra no M/OS.
//!
//! Aqui mora APENAS a decisao. Nada neste modulo toca disco, banco ou rede: o
//! que ele sabe fazer e olhar para um nome de arquivo, alguns bytes de cabecalho
//! e o contexto da tela e responder o que aquilo e, como deve se chamar, onde
//! deve ser guardado e a que isso provavelmente pertence.
//!
//! A regra que organiza o modulo inteiro e **preservar primeiro**:
//! classificacao, extracao e relacao sao enriquecimento. Nenhuma delas pode ser
//! condicao para o conteudo entrar — e por isso todas devolvem valores neutros
//! (`Unknown`, `None`, confianca zero) em vez de erro quando nao sabem.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{CaptureId, CoreError, ErrorCode, ProjectId, ResourceId, TaskId, WorkspaceId};

/// Teto de um unico item. Acima disso o M/OS recusa ANTES de escrever qualquer
/// byte, porque o custo de descobrir tarde e um arquivo pela metade no disco.
///
/// 512 MiB e generoso para o uso real (PDF de projeto executivo, video curto de
/// referencia) e ainda cabe na copia de um disco pessoal.
pub const MAX_INGEST_BYTES: u64 = 512 * 1024 * 1024;

/// Quanto texto extraido vai para o banco.
///
/// O texto existe para a busca reencontrar o arquivo, e nao para substituir o
/// arquivo. Um memorial de 400 paginas indexado inteiro cresceria o banco sem
/// melhorar o reencontro — as primeiras centenas de milhares de caracteres ja
/// carregam titulo, sumario e vocabulario do documento.
pub const MAX_EXTRACTED_CHARS: usize = 256 * 1024;

/// Acima disto o sistema relaciona sozinho.
pub const CONFIDENCE_LINK: f32 = 0.8;
/// Entre isto e `CONFIDENCE_LINK` o sistema sugere, e nao age.
pub const CONFIDENCE_SUGGEST: f32 = 0.45;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IngestionId(Uuid);

impl IngestionId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        Uuid::parse_str(value)
            .map(Self)
            .map_err(|_| CoreError::new(ErrorCode::InvalidInput, "Ingestao invalida.", false))
    }
}

impl Default for IngestionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for IngestionId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Por onde a coisa entrou.
///
/// Hoje as tres portas sao o mesmo gesto (soltar sobre a janela). Elas ja nascem
/// separadas porque a proxima porta — voz, share do iOS, clipboard, Hermes —
/// entra como variante nova aqui, e nao como pipeline paralelo.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestionSource {
    DropFile,
    DropText,
    DropUrl,
}

impl IngestionSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DropFile => "drop_file",
            Self::DropText => "drop_text",
            Self::DropUrl => "drop_url",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "drop_file" => Ok(Self::DropFile),
            "drop_text" => Ok(Self::DropText),
            "drop_url" => Ok(Self::DropUrl),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Origem de ingestao desconhecida.",
                false,
            )),
        }
    }
}

/// O que o conteudo aparenta ser.
///
/// Deliberadamente grosseiro. Nao e o MIME (que continua guardado inteiro) nem
/// um tipo do dominio: e o eixo que decide qual extrator tentar e qual rotulo
/// mostrar. `Unknown` e um resultado legitimo e nunca impede a preservacao.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectedKind {
    Pdf,
    Image,
    Text,
    Markdown,
    Data,
    Code,
    Archive,
    Url,
    Unknown,
}

impl DetectedKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Image => "image",
            Self::Text => "text",
            Self::Markdown => "markdown",
            Self::Data => "data",
            Self::Code => "code",
            Self::Archive => "archive",
            Self::Url => "url",
            Self::Unknown => "unknown",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "pdf" => Ok(Self::Pdf),
            "image" => Ok(Self::Image),
            "text" => Ok(Self::Text),
            "markdown" => Ok(Self::Markdown),
            "data" => Ok(Self::Data),
            "code" => Ok(Self::Code),
            "archive" => Ok(Self::Archive),
            "url" => Ok(Self::Url),
            "unknown" => Ok(Self::Unknown),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Tipo detectado desconhecido.",
                false,
            )),
        }
    }

    /// Se vale a pena tentar ler texto de dentro.
    pub fn has_text(self) -> bool {
        matches!(
            self,
            Self::Pdf | Self::Text | Self::Markdown | Self::Data | Self::Code
        )
    }
}

/// Onde a ingestao parou.
///
/// `Preserved` existe separado de `Completed` porque e exatamente a fronteira da
/// promessa: dali para tras nada mais se perde, dali para frente tudo e
/// enriquecimento. Uma ingestao que morre em `Preserved` continua tendo bytes no
/// disco e Capture na Inbox.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestionState {
    /// Recebendo bytes. A Capture ja existe; o arquivo ainda nao.
    Receiving,
    /// Original no lugar definitivo. Falta criar entidade e relacionar.
    Preserved,
    /// Entidade criada e relacionada.
    Completed,
    /// O app fechou no meio da transferencia.
    Interrupted,
    /// Recusada ou quebrada antes de preservar.
    Failed,
    /// A pessoa desfez. O que esta ingestao criou foi arquivado ou desligado; o
    /// que ela apenas encontrou continua onde estava.
    Undone,
}

impl IngestionState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Receiving => "receiving",
            Self::Preserved => "preserved",
            Self::Completed => "completed",
            Self::Interrupted => "interrupted",
            Self::Failed => "failed",
            Self::Undone => "undone",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "receiving" => Ok(Self::Receiving),
            "preserved" => Ok(Self::Preserved),
            "completed" => Ok(Self::Completed),
            "interrupted" => Ok(Self::Interrupted),
            "failed" => Ok(Self::Failed),
            "undone" => Ok(Self::Undone),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Estado de ingestao desconhecido.",
                false,
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionState {
    /// Ainda nao tentou.
    Pending,
    /// Leu e havia texto.
    Done,
    /// Leu e nao havia texto util. Um PDF escaneado cai aqui, e e daqui que o
    /// OCR futuro tira sua fila de trabalho.
    Empty,
    /// Nao existe extrator para este tipo. Nao e falha.
    Unsupported,
    /// Tentou e quebrou. O original continua intacto.
    Failed,
}

impl ExtractionState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Done => "done",
            Self::Empty => "empty",
            Self::Unsupported => "unsupported",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "pending" => Ok(Self::Pending),
            "done" => Ok(Self::Done),
            "empty" => Ok(Self::Empty),
            "unsupported" => Ok(Self::Unsupported),
            "failed" => Ok(Self::Failed),
            _ => Err(CoreError::new(
                ErrorCode::DataIntegrity,
                "Estado de extracao desconhecido.",
                false,
            )),
        }
    }
}

/// De onde a pessoa estava olhando quando soltou.
///
/// Capturado no instante do drop e guardado junto da ingestao mesmo quando nao
/// gera relacao nenhuma: sem isso, descobrir depois que uma relacao deveria ter
/// existido nao teria como ser respondido.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DropContext {
    /// A pagina do M/OS que estava na tela. Texto livre de proposito: e registro,
    /// nao chave.
    pub page: String,
    pub project_id: Option<ProjectId>,
    pub workspace_id: Option<WorkspaceId>,
    pub task_id: Option<TaskId>,
}

/// O que fazer com a relacao inferida.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationDecision {
    /// Confianca alta: relaciona em silencio.
    Link,
    /// Confianca media: oferece, nao faz.
    Suggest,
    /// Confianca baixa: nao inventa contexto.
    None,
}

/// A leitura do contexto, com a confianca que a sustenta.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationPlan {
    /// Relacionado agora, sem perguntar.
    pub link_project: Option<ProjectId>,
    pub link_workspace: Option<WorkspaceId>,
    /// Oferecido ao usuario, com um clique de distancia.
    pub suggest_project: Option<ProjectId>,
    pub confidence: f32,
    pub decision: RelationDecision,
    /// Por que. Vai para o recibo e para o log; sem isso a relacao automatica
    /// vira magica, e o §19 do UX-PRINCIPLES proibe magia sem explicacao.
    pub reason: String,
}

impl RelationPlan {
    fn empty() -> Self {
        Self {
            link_project: None,
            link_workspace: None,
            suggest_project: None,
            confidence: 0.0,
            decision: RelationDecision::None,
            reason: String::new(),
        }
    }
}

/// Um Project visto pelo planejador de relacoes: so o que ele precisa saber.
#[derive(Clone, Debug)]
pub struct ProjectHint {
    pub id: ProjectId,
    pub name: String,
}

/// Decide a que o conteudo recem-chegado pertence.
///
/// A ordem das perguntas e a propria escala de confianca, da mais explicita para
/// a mais frouxa:
///
/// 1. havia um Project aberto na tela — o drop aconteceu DENTRO dele;
/// 2. havia uma Task aberta — o Project dela e fato, nao inferencia;
/// 3. ha uma lente de Workspace ativa — a pessoa declarou onde esta trabalhando;
/// 4. o nome do arquivo carrega o nome de um Project — pista, e so.
///
/// Os tres primeiros linkam; o quarto sugere. Nada abaixo disso acontece: um
/// palpite fraco que erra custa mais que a relacao que ele economizaria.
pub fn plan_relations(
    context: &DropContext,
    file_name: &str,
    projects: &[ProjectHint],
) -> RelationPlan {
    let mut plan = RelationPlan::empty();
    let mut reasons: Vec<String> = Vec::new();

    if let Some(project) = context.project_id {
        plan.link_project = Some(project);
        // A Task aberta e um alvo ainda mais preciso que a pagina do Project,
        // mas hoje ela nao tem relacao propria com Resource: o que se relaciona
        // e o Project dela. O `task_id` fica registrado na ingestao, e o dia em
        // que a relacao existir ela e reconstruivel a partir dali.
        if context.task_id.is_some() {
            plan.confidence = 0.9;
            reasons.push("O drop aconteceu sobre a Task aberta.".into());
        } else {
            plan.confidence = 0.95;
            reasons.push("O drop aconteceu dentro do Project aberto.".into());
        }
    } else if let Some(hint) = match_project_by_name(file_name, projects) {
        plan.suggest_project = Some(hint.id);
        plan.confidence = plan.confidence.max(0.6);
        reasons.push(format!("O nome do arquivo cita {}.", hint.name));
    }

    // A lente de Workspace entra em TODOS os caminhos em que existe, e nao
    // apenas quando nada mais foi encontrado: a Library filtra por ela por
    // padrao, entao um Resource sem o vinculo nasce invisivel exatamente na
    // tela onde a pessoa esta parada. Errar aqui custa uma relacao a mais,
    // visivel e desfazivel; nao vincular custa o item sumir.
    if context.workspace_id.is_some() {
        plan.link_workspace = context.workspace_id;
        plan.confidence = plan.confidence.max(0.8);
        reasons.push("O contexto ativo estava aberto.".into());
    }

    plan.decision = if plan.link_project.is_some() || plan.link_workspace.is_some() {
        RelationDecision::Link
    } else if plan.suggest_project.is_some() {
        RelationDecision::Suggest
    } else {
        RelationDecision::None
    };
    if matches!(plan.decision, RelationDecision::None) {
        plan.confidence = 0.0;
        reasons.clear();
    }
    plan.reason = reasons.join(" ");
    plan
}

/// O Project cujo nome aparece no nome do arquivo.
///
/// Comparacao por palavra normalizada, e nao por substring solta: `api.pdf` nao
/// pode casar com o Project "Rapidinhas" so porque as letras cabem la dentro.
/// Palavras de menos de quatro letras sao ignoradas pelo mesmo motivo.
fn match_project_by_name<'a>(file_name: &str, projects: &'a [ProjectHint]) -> Option<&'a ProjectHint> {
    let haystack = normalize_words(file_name);
    if haystack.is_empty() {
        return None;
    }
    projects
        .iter()
        .filter(|project| {
            let needle = normalize_words(&project.name);
            !needle.is_empty()
                && needle.iter().all(|word| word.chars().count() >= 4)
                && needle.iter().all(|word| haystack.contains(word))
        })
        // O nome mais longo ganha: entre "Nexo" e "NexoDoc" num arquivo que cita
        // os dois, o especifico e o que informa.
        .max_by_key(|project| project.name.chars().count())
}

fn normalize_words(value: &str) -> Vec<String> {
    value
        .chars()
        .map(|character| {
            let lowered = fold_char(character);
            if lowered.is_alphanumeric() {
                lowered
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

/// Minusculiza e tira acento das letras que o portugues usa.
///
/// Sem tabela Unicode completa e sem dependencia nova: o alvo aqui e nome de
/// arquivo e nome de Project, nao normalizacao de texto geral.
fn fold_char(character: char) -> char {
    let lowered = character.to_lowercase().next().unwrap_or(character);
    match lowered {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        other => other,
    }
}

/// Nomes que o Windows recusa como arquivo, com ou sem extensao.
const RESERVED_WINDOWS_NAMES: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// O nome que o M/OS vai exibir e guardar.
///
/// O nome que chega do navegador e dado do usuario, e um dado do usuario que
/// vira caminho e uma vulnerabilidade esperando o dia. Aqui ele deixa de ser
/// caminho: separadores, `..`, dois-pontos, controle e nomes reservados do
/// Windows saem. O que sobra e rotulo — o caminho real e derivado do hash
/// (`stored_path`), e nunca deste texto.
pub fn sanitize_file_name(raw: &str) -> String {
    let without_path = raw
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(raw)
        .trim()
        .trim_matches('.');
    let cleaned: String = without_path
        .chars()
        .map(|character| {
            if character.is_control() || matches!(character, ':' | '*' | '?' | '"' | '<' | '>' | '|')
            {
                '_'
            } else {
                character
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').to_owned();
    let stem = cleaned
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if cleaned.is_empty() || RESERVED_WINDOWS_NAMES.contains(&stem.as_str()) {
        return "arquivo-sem-nome".to_owned();
    }
    // Teto generoso, bem abaixo do limite do NTFS: nomes maiores que isto sao
    // acidente de exportador, nao intencao.
    if cleaned.chars().count() > 180 {
        return cleaned.chars().take(180).collect();
    }
    cleaned
}

/// A extensao normalizada, ou vazio.
///
/// So aceita o que parece extensao de verdade: ate 12 caracteres alfanumericos.
/// Assim `arquivo.tar.gz` devolve `gz` e `nome.com espaco` nao devolve nada.
pub fn extension_of(name: &str) -> String {
    let candidate = match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => extension,
        _ => return String::new(),
    };
    let lowered = candidate.to_ascii_lowercase();
    if lowered.is_empty()
        || lowered.chars().count() > 12
        || !lowered.chars().all(|character| character.is_ascii_alphanumeric())
    {
        return String::new();
    }
    lowered
}

/// O que isto e, pela extensao primeiro e pelo MIME depois.
///
/// A extensao vem antes porque o MIME que o navegador declara e frequentemente
/// `application/octet-stream` ou simplesmente vazio — e um palpite ruim nao pode
/// vencer um dado que existe.
pub fn detect_kind(name: &str, declared_mime: &str) -> DetectedKind {
    let extension = extension_of(name);
    let by_extension = match extension.as_str() {
        "pdf" => Some(DetectedKind::Pdf),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "avif" | "svg" | "tif" | "tiff"
        | "ico" | "heic" => Some(DetectedKind::Image),
        "md" | "markdown" | "mdx" => Some(DetectedKind::Markdown),
        "txt" | "log" | "rtf" => Some(DetectedKind::Text),
        "csv" | "tsv" | "json" | "xml" | "yaml" | "yml" | "toml" | "ini" | "xlsx" | "xls"
        | "ods" | "parquet" => Some(DetectedKind::Data),
        "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "java" | "kt" | "swift" | "c" | "h"
        | "cpp" | "hpp" | "cs" | "rb" | "php" | "sh" | "ps1" | "sql" | "css" | "scss" | "html"
        | "htm" | "vue" | "svelte" => Some(DetectedKind::Code),
        "zip" | "rar" | "7z" | "gz" | "tar" | "bz2" | "xz" => Some(DetectedKind::Archive),
        _ => None,
    };
    if let Some(kind) = by_extension {
        return kind;
    }

    let mime = declared_mime.trim().to_ascii_lowercase();
    if mime.starts_with("image/") {
        return DetectedKind::Image;
    }
    if mime == "application/pdf" {
        return DetectedKind::Pdf;
    }
    if mime == "text/markdown" {
        return DetectedKind::Markdown;
    }
    if mime == "application/json" || mime == "text/csv" || mime.contains("spreadsheet") {
        return DetectedKind::Data;
    }
    if mime.starts_with("text/") {
        return DetectedKind::Text;
    }
    if mime.contains("zip") || mime.contains("compressed") {
        return DetectedKind::Archive;
    }
    DetectedKind::Unknown
}

/// O MIME que vai para o banco.
///
/// Prefere o que o navegador declarou; quando ele cala, deriva do que a extensao
/// diz. `application/octet-stream` e o ultimo recurso honesto: significa "bytes",
/// e nao "erro".
pub fn resolve_mime(name: &str, declared_mime: &str) -> String {
    let declared = declared_mime.trim();
    if !declared.is_empty() && declared != "application/octet-stream" {
        return declared.to_ascii_lowercase();
    }
    match extension_of(name).as_str() {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "md" | "markdown" | "mdx" => "text/markdown",
        "txt" | "log" => "text/plain",
        "csv" => "text/csv",
        "json" => "application/json",
        "xml" => "application/xml",
        "html" | "htm" => "text/html",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
    .to_owned()
}

/// Extensoes que o M/OS nunca abre pelo shell do Windows.
///
/// Abrir um arquivo pelo handler padrao e, para estes, o mesmo que executa-lo —
/// e §24 e categorico: nada recebido e executado. Eles continuam guardados,
/// pesquisaveis e exportaveis; o que nao existe e o botao que os dispara.
const NEVER_OPEN: [&str; 22] = [
    "exe", "msi", "bat", "cmd", "com", "scr", "pif", "ps1", "psm1", "vbs", "vbe", "js", "jse",
    "wsf", "wsh", "hta", "cpl", "msc", "reg", "jar", "lnk", "url",
];

/// Se o M/OS pode pedir ao Windows para abrir este arquivo.
pub fn is_openable(name: &str) -> bool {
    let extension = extension_of(name);
    !extension.is_empty() && !NEVER_OPEN.contains(&extension.as_str())
}

/// O caminho relativo do original dentro do diretorio de dados.
///
/// Enderecado pelo conteudo: dois drops do mesmo arquivo apontam para o mesmo
/// lugar, e nenhum nome vindo do usuario participa do caminho. Os dois primeiros
/// pares de digitos viram pastas para nao produzir um diretorio com dezenas de
/// milhares de entradas.
pub fn stored_path(sha256: &str, extension: &str) -> Result<String, CoreError> {
    if sha256.len() != 64 || !sha256.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(CoreError::new(
            ErrorCode::DataIntegrity,
            "Hash de conteudo invalido.",
            false,
        ));
    }
    let sha256 = sha256.to_ascii_lowercase();
    let suffix = if extension.is_empty() {
        String::new()
    } else {
        format!(".{extension}")
    };
    Ok(format!(
        "drops/{}/{}/{}{}",
        &sha256[0..2],
        &sha256[2..4],
        sha256,
        suffix
    ))
}

/// Normaliza uma URL solta em algo que vale a pena guardar.
///
/// Nao e um parser de URL completo e nao quer ser: aceita http(s), tira espacos
/// e fragmento vazio, e recusa o resto. Esquema desconhecido vira erro em vez de
/// virar Resource quebrado.
pub fn normalize_url(raw: &str) -> Result<String, CoreError> {
    let trimmed = raw.trim().trim_end_matches('#');
    if trimmed.is_empty() {
        return Err(CoreError::new(
            ErrorCode::InvalidInput,
            "URL vazia.",
            false,
        ));
    }
    let lowered = trimmed.to_ascii_lowercase();
    if !(lowered.starts_with("http://") || lowered.starts_with("https://")) {
        return Err(CoreError::new(
            ErrorCode::InvalidInput,
            "So http:// e https:// entram como link.",
            false,
        ));
    }
    if trimmed.contains(char::is_whitespace) {
        return Err(CoreError::new(
            ErrorCode::InvalidInput,
            "URL com espaco nao e uma URL.",
            false,
        ));
    }
    Ok(trimmed.to_owned())
}

/// O dominio, para servir de titulo enquanto ninguem leu a pagina.
pub fn host_of(url: &str) -> String {
    url.split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(url)
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .trim_start_matches("www.")
        .to_owned()
}

/// A primeira linha util de um texto, para servir de titulo.
pub fn title_from_text(text: &str) -> String {
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();
    if line.chars().count() <= 80 {
        return line.to_owned();
    }
    let mut title: String = line.chars().take(79).collect();
    title.push('…');
    title
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageSize {
    pub width: u32,
    pub height: u32,
}

/// Le largura e altura direto do cabecalho, sem decodificar a imagem.
///
/// PNG, JPEG, GIF, BMP e WebP cobrem o que sai de um screenshot ou de um
/// navegador. Formato fora dessa lista devolve `None` — e uma imagem sem
/// dimensao registrada continua sendo uma imagem guardada.
///
/// A alternativa seria a crate `image`, que traz dezenas de decodificadores para
/// responder duas perguntas que cabem em cem linhas.
pub fn image_size(bytes: &[u8]) -> Option<ImageSize> {
    if bytes.len() >= 24 && bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some(ImageSize {
            width: u32::from_be_bytes(bytes[16..20].try_into().ok()?),
            height: u32::from_be_bytes(bytes[20..24].try_into().ok()?),
        });
    }
    if bytes.len() >= 10 && bytes.starts_with(b"GIF8") {
        return Some(ImageSize {
            width: u16::from_le_bytes(bytes[6..8].try_into().ok()?) as u32,
            height: u16::from_le_bytes(bytes[8..10].try_into().ok()?) as u32,
        });
    }
    if bytes.len() >= 26 && bytes.starts_with(b"BM") {
        return Some(ImageSize {
            width: i32::from_le_bytes(bytes[18..22].try_into().ok()?).unsigned_abs(),
            height: i32::from_le_bytes(bytes[22..26].try_into().ok()?).unsigned_abs(),
        });
    }
    if bytes.len() >= 30 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return webp_size(bytes);
    }
    if bytes.len() >= 4 && bytes[0] == 0xFF && bytes[1] == 0xD8 {
        return jpeg_size(bytes);
    }
    None
}

fn webp_size(bytes: &[u8]) -> Option<ImageSize> {
    match &bytes[12..16] {
        b"VP8 " if bytes.len() >= 30 => Some(ImageSize {
            width: (u16::from_le_bytes(bytes[26..28].try_into().ok()?) & 0x3FFF) as u32,
            height: (u16::from_le_bytes(bytes[28..30].try_into().ok()?) & 0x3FFF) as u32,
        }),
        b"VP8L" if bytes.len() >= 25 => {
            let bits = u32::from_le_bytes(bytes[21..25].try_into().ok()?);
            Some(ImageSize {
                width: (bits & 0x3FFF) + 1,
                height: ((bits >> 14) & 0x3FFF) + 1,
            })
        }
        b"VP8X" if bytes.len() >= 30 => {
            let width = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], 0]) + 1;
            let height = u32::from_le_bytes([bytes[27], bytes[28], bytes[29], 0]) + 1;
            Some(ImageSize { width, height })
        }
        _ => None,
    }
}

/// Caminha pelos marcadores do JPEG ate o SOF, que e onde as dimensoes moram.
fn jpeg_size(bytes: &[u8]) -> Option<ImageSize> {
    let mut cursor = 2usize;
    while cursor + 9 <= bytes.len() {
        if bytes[cursor] != 0xFF {
            cursor += 1;
            continue;
        }
        let marker = bytes[cursor + 1];
        // SOF0..SOF15, menos os marcadores que nao carregam dimensao.
        if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
            return Some(ImageSize {
                height: u16::from_be_bytes(bytes[cursor + 5..cursor + 7].try_into().ok()?) as u32,
                width: u16::from_be_bytes(bytes[cursor + 7..cursor + 9].try_into().ok()?) as u32,
            });
        }
        let length = u16::from_be_bytes(bytes[cursor + 2..cursor + 4].try_into().ok()?) as usize;
        if length < 2 {
            return None;
        }
        cursor += 2 + length;
    }
    None
}

/// Corta o texto extraido no teto, sem partir caractere no meio.
pub fn clamp_extracted_text(text: &str) -> String {
    let cleaned = text.trim();
    if cleaned.chars().count() <= MAX_EXTRACTED_CHARS {
        return cleaned.to_owned();
    }
    cleaned.chars().take(MAX_EXTRACTED_CHARS).collect()
}

/// O que a Capture vai dizer.
///
/// A Capture e o registro de que ALGO entrou, e ela precisa fazer sentido sozinha
/// meses depois, na Inbox, sem o Resource ao lado. Por isso ela carrega o nome do
/// arquivo por extenso e nao um identificador.
pub fn capture_content(source: IngestionSource, subject: &str) -> String {
    let subject = subject.trim();
    match source {
        IngestionSource::DropFile => format!("Arquivo recebido: {subject}"),
        IngestionSource::DropUrl => subject.to_owned(),
        IngestionSource::DropText => subject.to_owned(),
    }
}

/// A linha da ingestao como ela vive no banco.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ingestion {
    pub id: IngestionId,
    pub source: IngestionSource,
    pub original_name: String,
    pub mime: String,
    pub byte_size: u64,
    pub sha256: String,
    pub stored_path: String,
    pub detected_kind: DetectedKind,
    pub state: IngestionState,
    pub failure: String,
    pub capture_id: Option<CaptureId>,
    pub resource_id: Option<ResourceId>,
    pub duplicate_of: Option<ResourceId>,
    pub context: DropContext,
    pub suggested_project_id: Option<ProjectId>,
    pub relation_confidence: f32,
    pub relation_reason: String,
    pub extraction_state: ExtractionState,
    pub extraction_error: String,
    pub page_count: Option<u32>,
    pub image_size: Option<ImageSize>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// O pedido de ingestao, ja validado.
#[derive(Clone, Debug)]
pub struct NewIngestion {
    pub id: IngestionId,
    pub source: IngestionSource,
    pub original_name: String,
    pub mime: String,
    pub declared_size: u64,
    pub detected_kind: DetectedKind,
    pub context: DropContext,
    pub created_at: OffsetDateTime,
}

impl NewIngestion {
    /// Valida o que chegou do renderer ANTES de qualquer escrita.
    ///
    /// Tamanho declarado acima do teto para aqui; nome vira rotulo; MIME e tipo
    /// sao resolvidos. Nada disso pode falhar por conteudo desconhecido — a
    /// unica recusa possivel e a de tamanho.
    pub fn file(
        original_name: &str,
        declared_mime: &str,
        declared_size: u64,
        context: DropContext,
    ) -> Result<Self, CoreError> {
        if declared_size > MAX_INGEST_BYTES {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                format!(
                    "Arquivo maior que o limite de {} MB.",
                    MAX_INGEST_BYTES / (1024 * 1024)
                ),
                false,
            ));
        }
        let original_name = sanitize_file_name(original_name);
        Ok(Self {
            id: IngestionId::new(),
            source: IngestionSource::DropFile,
            mime: resolve_mime(&original_name, declared_mime),
            detected_kind: detect_kind(&original_name, declared_mime),
            original_name,
            declared_size,
            context,
            created_at: OffsetDateTime::now_utc(),
        })
    }

    pub fn url(url: &str, context: DropContext) -> Result<Self, CoreError> {
        let url = normalize_url(url)?;
        Ok(Self {
            id: IngestionId::new(),
            source: IngestionSource::DropUrl,
            original_name: url,
            mime: "text/uri-list".to_owned(),
            declared_size: 0,
            detected_kind: DetectedKind::Url,
            context,
            created_at: OffsetDateTime::now_utc(),
        })
    }

    pub fn text(text: &str, context: DropContext) -> Result<Self, CoreError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(CoreError::new(
                ErrorCode::InvalidInput,
                "Texto vazio nao entra.",
                false,
            ));
        }
        Ok(Self {
            id: IngestionId::new(),
            source: IngestionSource::DropText,
            original_name: title_from_text(text),
            mime: "text/plain".to_owned(),
            declared_size: text.len() as u64,
            detected_kind: DetectedKind::Text,
            context,
            created_at: OffsetDateTime::now_utc(),
        })
    }
}

/// O que aconteceu, dito para a tela.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestionReceipt {
    pub ingestion: Ingestion,
    /// Existe quando o conteudo ja estava no M/OS. O Resource apontado e o
    /// antigo, e o contexto novo foi aplicado sobre ele.
    pub duplicate: bool,
    /// Rotulo curto do destino, para o recibo: "Library", "NexoDoc", "Inbox".
    pub destination: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nome_perde_o_caminho_e_a_travessia() {
        assert_eq!(sanitize_file_name("../../etc/passwd"), "passwd");
        assert_eq!(sanitize_file_name(r"C:\Windows\System32\evil.dll"), "evil.dll");
        assert_eq!(sanitize_file_name("../.."), "arquivo-sem-nome");
        assert_eq!(sanitize_file_name("   "), "arquivo-sem-nome");
        assert_eq!(sanitize_file_name("rela:torio*.pdf"), "rela_torio_.pdf");
    }

    #[test]
    fn nome_reservado_do_windows_nao_vira_arquivo() {
        assert_eq!(sanitize_file_name("CON.txt"), "arquivo-sem-nome");
        assert_eq!(sanitize_file_name("lpt1"), "arquivo-sem-nome");
        assert_eq!(sanitize_file_name("console.txt"), "console.txt");
    }

    #[test]
    fn extensao_so_aceita_o_que_parece_extensao() {
        assert_eq!(extension_of("memorial.PDF"), "pdf");
        assert_eq!(extension_of("arquivo.tar.gz"), "gz");
        assert_eq!(extension_of("sem-extensao"), "");
        assert_eq!(extension_of(".gitignore"), "");
        assert_eq!(extension_of("nome.extensao muito estranha"), "");
    }

    #[test]
    fn extensao_vence_mime_vazio_e_mime_salva_extensao_ausente() {
        assert_eq!(detect_kind("memorial.pdf", ""), DetectedKind::Pdf);
        assert_eq!(
            detect_kind("memorial.pdf", "application/octet-stream"),
            DetectedKind::Pdf
        );
        assert_eq!(detect_kind("captura", "image/png"), DetectedKind::Image);
        assert_eq!(detect_kind("coisa", ""), DetectedKind::Unknown);
        assert_eq!(detect_kind("planilha.xlsx", ""), DetectedKind::Data);
    }

    #[test]
    fn tipo_desconhecido_nunca_e_erro() {
        let pedido = NewIngestion::file("coisa.qualquer", "", 10, DropContext::default()).unwrap();
        assert_eq!(pedido.detected_kind, DetectedKind::Unknown);
        assert_eq!(pedido.mime, "application/octet-stream");
    }

    #[test]
    fn arquivo_grande_demais_para_antes_de_escrever_byte() {
        let erro = NewIngestion::file(
            "gigante.bin",
            "",
            MAX_INGEST_BYTES + 1,
            DropContext::default(),
        )
        .unwrap_err();
        assert_eq!(erro.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn caminho_de_armazenamento_vem_do_hash_e_nunca_do_nome() {
        let hash = "a".repeat(64);
        let caminho = stored_path(&hash, "pdf").unwrap();
        assert_eq!(caminho, format!("drops/aa/aa/{hash}.pdf"));
        assert!(stored_path("curto", "pdf").is_err());
        assert!(stored_path(&"z".repeat(64), "pdf").is_err());
    }

    #[test]
    fn executavel_nunca_e_aberto() {
        assert!(!is_openable("instalador.exe"));
        assert!(!is_openable("script.ps1"));
        assert!(!is_openable("atalho.lnk"));
        assert!(is_openable("memorial.pdf"));
        assert!(is_openable("captura.png"));
        // Sem extensao o M/OS nao sabe o que o shell faria, entao nao pede.
        assert!(!is_openable("arquivo"));
    }

    #[test]
    fn url_so_entra_com_esquema_conhecido() {
        assert_eq!(
            normalize_url("  https://motion.dev/docs  ").unwrap(),
            "https://motion.dev/docs"
        );
        assert!(normalize_url("motion.dev").is_err());
        assert!(normalize_url("file:///C:/segredos.txt").is_err());
        assert!(normalize_url("javascript:alert(1)").is_err());
        assert!(normalize_url("https://a b.dev").is_err());
    }

    #[test]
    fn host_serve_de_titulo() {
        assert_eq!(host_of("https://www.motion.dev/docs?x=1"), "motion.dev");
        assert_eq!(host_of("http://localhost:1420/"), "localhost:1420");
    }

    #[test]
    fn project_aberto_ganha_de_tudo() {
        let project = ProjectId::new();
        let workspace = WorkspaceId::new();
        let plano = plan_relations(
            &DropContext {
                page: "projects".into(),
                project_id: Some(project),
                workspace_id: Some(workspace),
                task_id: None,
            },
            "qualquer.pdf",
            &[],
        );
        assert_eq!(plano.decision, RelationDecision::Link);
        assert_eq!(plano.link_project, Some(project));
        assert_eq!(plano.link_workspace, Some(workspace));
        assert!(plano.confidence >= CONFIDENCE_LINK);
    }

    #[test]
    fn sem_contexto_nenhum_o_sistema_nao_inventa() {
        let plano = plan_relations(&DropContext::default(), "memorial.pdf", &[]);
        assert_eq!(plano.decision, RelationDecision::None);
        assert!(plano.link_project.is_none());
        assert!(plano.link_workspace.is_none());
        assert_eq!(plano.confidence, 0.0);
    }

    #[test]
    fn nome_de_arquivo_sugere_e_nunca_relaciona_sozinho() {
        let projeto = ProjectHint {
            id: ProjectId::new(),
            name: "NexoDoc".into(),
        };
        let plano = plan_relations(
            &DropContext::default(),
            "NexoDoc-pricing.pdf",
            std::slice::from_ref(&projeto),
        );
        assert_eq!(plano.decision, RelationDecision::Suggest);
        assert_eq!(plano.suggest_project, Some(projeto.id));
        assert!(plano.link_project.is_none());
        assert!(plano.confidence < CONFIDENCE_LINK);
        assert!(plano.confidence >= CONFIDENCE_SUGGEST);
    }

    #[test]
    fn palavra_curta_nao_casa_project_por_acidente() {
        let projeto = ProjectHint {
            id: ProjectId::new(),
            name: "API".into(),
        };
        let plano = plan_relations(&DropContext::default(), "rapido.pdf", &[projeto]);
        assert_eq!(plano.decision, RelationDecision::None);
    }

    #[test]
    fn dimensao_de_png_sai_do_cabecalho() {
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        png.extend_from_slice(&[0, 0, 0, 13]);
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&1280u32.to_be_bytes());
        png.extend_from_slice(&720u32.to_be_bytes());
        assert_eq!(
            image_size(&png),
            Some(ImageSize {
                width: 1280,
                height: 720
            })
        );
    }

    #[test]
    fn dimensao_de_gif_e_de_jpeg() {
        let mut gif = b"GIF89a".to_vec();
        gif.extend_from_slice(&640u16.to_le_bytes());
        gif.extend_from_slice(&480u16.to_le_bytes());
        assert_eq!(
            image_size(&gif),
            Some(ImageSize {
                width: 640,
                height: 480
            })
        );

        // SOI, um marcador APP0 curto e entao o SOF0 com as dimensoes.
        let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00];
        jpeg.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 0x08]);
        jpeg.extend_from_slice(&1080u16.to_be_bytes());
        jpeg.extend_from_slice(&1920u16.to_be_bytes());
        assert_eq!(
            image_size(&jpeg),
            Some(ImageSize {
                width: 1920,
                height: 1080
            })
        );
    }

    #[test]
    fn formato_desconhecido_nao_tem_dimensao_e_nao_quebra() {
        assert_eq!(image_size(b"nao sou imagem"), None);
        assert_eq!(image_size(&[]), None);
        assert_eq!(image_size(&[0x89, b'P', b'N', b'G']), None);
    }

    #[test]
    fn texto_extraido_respeita_o_teto() {
        let gigante = "a".repeat(MAX_EXTRACTED_CHARS + 100);
        assert_eq!(
            clamp_extracted_text(&gigante).chars().count(),
            MAX_EXTRACTED_CHARS
        );
    }

    #[test]
    fn titulo_de_texto_usa_a_primeira_linha_util() {
        assert_eq!(
            title_from_text("\n\n  Conversar com Joao sobre orcamento\nresto"),
            "Conversar com Joao sobre orcamento"
        );
        assert_eq!(title_from_text("").len(), 0);
    }
}
