//! A superficie: capturar, inbox, tasks.
//!
//! Tres verbos e meia duzia de rotas. O desktop expoe 280 comandos, e a
//! distancia entre os dois numeros NAO e uma lacuna a ser fechada — e a
//! fronteira: isto e uma porta, e uma porta que vira casa deixa de ser porta.
//!
//! # Toda escrita responde ANTES de sincronizar
//!
//! A captura ja esta gravada no banco local quando a tela responde; a subida
//! acontece depois, em segundo plano. O contrario ligaria "tirar da cabeca" a
//! ter sinal — e a ideia que nao se escreve porque o 4G caiu e uma ideia
//! perdida.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use mos_core::{CaptureSource, CoreError, CreateCaptureInput, CreateTaskInput, TaskState};
use serde::{Deserialize, Serialize};

use crate::estado::Estado;

/// Quantos itens uma lista devolve.
///
/// A tela do celular mostra o que cabe no polegar; pedir mais seria gastar
/// bateria e rede para desenhar o que ninguem rola. Quem quer o resto abre o
/// desktop, que e onde o recorte existe.
const LIMITE: usize = 50;

pub fn rotas() -> Router<Estado> {
    Router::new()
        .route("/api/capturar", post(capturar))
        .route("/api/inbox", get(inbox))
        .route("/api/tasks", get(tasks).post(criar_task))
        .route("/api/tasks/{id}", get(task).patch(editar_task))
        .route("/api/tasks/{id}/estado", post(mudar_estado))
        .route("/api/tasks/{id}/arquivar", post(arquivar_task))
        .route("/api/projetos", get(projetos))
        .route("/api/lembretes", get(lembretes).post(criar_lembrete))
        .route("/api/lembretes/resolvidos", get(lembretes_resolvidos))
        .route("/api/lembretes/{id}", get(lembrete).patch(editar_lembrete))
        .route("/api/lembretes/{id}/concluir", post(concluir_lembrete))
        .route("/api/lembretes/{id}/cancelar", post(cancelar_lembrete))
        .route("/api/lembretes/{id}/adiar", post(adiar_lembrete))
        .route("/api/lembretes/{id}/arquivar", post(arquivar_lembrete))
        .route("/api/estado", get(estado_do_aparelho))
        .route("/api/panorama", get(panorama))
        .route("/api/agenda", get(agenda))
        .route("/api/horas", get(horas))
        .route("/api/academico", get(academico))
        .route("/api/push/assinar", post(assinar_push))
        .route("/api/push/testar", post(testar_push))
}

/// As rotas mais a pagina.
pub fn rotas_com_pagina() -> Router<Estado> {
    rotas()
        .route("/api/porta/estado", get(porta_estado))
        .route("/api/porta/sair", post(sair))
        .fallback(get(pagina))
}

/// O servidor pronto: rotas, estado e o GUARDIAO.
///
/// Existe como funcao separada porque o guardiao precisa do estado ja
/// resolvido, e porque montar a porta a mao em cada lugar que sobe o servidor e
/// exatamente como uma delas fica sem porta. Quem sobe o `mos-web` de verdade
/// chama isto — o `main.rs` e o teste da porta.
pub fn servidor(estado: Estado) -> Router {
    let sessoes = estado.sessoes.clone();

    // A cerimonia entra ANTES do guardiao, e por isso ela e um `merge` e nao um
    // sub-router qualquer: as rotas dela vivem sob `/api/porta/`, que e o unico
    // prefixo livre. Fora dele, quem ainda nao entrou nao consegue nem pedir
    // para entrar.
    #[cfg(feature = "passkey")]
    let cerimonia = match (&estado.webauthn, &estado.sessoes) {
        (Some(webauthn), Some(sessoes)) => crate::auth::rotas(
            std::sync::Arc::clone(webauthn),
            std::sync::Arc::clone(sessoes),
        ),
        _ => Router::new(),
    };

    let servidor = rotas_com_pagina().with_state(estado);

    #[cfg(feature = "passkey")]
    let servidor = servidor.merge(cerimonia);

    servidor.layer(axum::middleware::from_fn_with_state(
        sessoes,
        crate::porta::guarda,
    ))
}

// ------------------------------------------------------------------- porta

/// O que a tela precisa saber ANTES de qualquer login.
///
/// Rota livre, e a unica informacao que ela entrega e se ha porta e se ja existe
/// aparelho registrado — o suficiente para a tela escolher entre "registrar" e
/// "entrar", e nada alem disso.
async fn porta_estado(State(estado): State<Estado>) -> Json<serde_json::Value> {
    let (tem_porta, registrado) = match &estado.sessoes {
        Some(sessoes) => (true, sessoes.ha_credencial().unwrap_or(false)),
        None => (false, false),
    };
    Json(serde_json::json!({
        "porta": tem_porta,
        "registrado": registrado,
        // A cerimonia WebAuthn so existe com a feature compilada. Sem ela, a
        // tela nao deve oferecer um botao que nao tem servidor do outro lado.
        "passkey": cfg!(feature = "passkey"),
    }))
}

async fn sair(
    State(estado): State<Estado>,
    jar: axum_extra::extract::cookie::CookieJar,
) -> (
    axum_extra::extract::cookie::CookieJar,
    Json<serde_json::Value>,
) {
    if let Some(sessoes) = &estado.sessoes {
        let _ = sessoes.encerrar(&jar);
    }
    (
        jar.add(crate::porta::cookie_vazio()),
        Json(serde_json::json!({ "ok": true })),
    )
}

// ----------------------------------------------------------------- pagina

/// Os arquivos que o `vite build` produziu, dentro do binario.
///
/// Embutidos, e nao lidos do disco: um servico que depende de uma pasta ao lado
/// do executavel quebra quando alguem move o executavel, e o `systemd` roda com
/// um `WorkingDirectory` que nem sempre e o que se imagina.
///
/// A pasta precisa existir no momento da COMPILACAO — `npm run build` na `ui/`
/// vem antes de `cargo build`. O Cargo nao sabe disso sozinho, e por isso esta
/// escrito no README e no workflow.
#[derive(rust_embed::Embed)]
#[folder = "static/"]
struct Estaticos;

/// Por quanto tempo o navegador pode guardar cada arquivo.
///
/// # Sem isto, todo deploy podia deixar a tela em branco
///
/// O `vite` poe um hash no nome de cada bundle, entao o JS de hoje se chama
/// diferente do de ontem. O `index.html` e quem aponta para o nome certo — e,
/// sem cabecalho nenhum, o Safari aplica cache heuristico e pode servir o
/// `index.html` VELHO, que aponta para um arquivo que este binario nao tem mais.
/// Resultado: 404 no bundle e uma pagina em branco, num app instalado na tela de
/// inicio, sem nada indicando o que houve.
///
/// Entao:
///
/// - o que tem hash no nome pode ser guardado para sempre — um nome novo e um
///   arquivo novo, e o velho nunca mais e pedido;
/// - o `index.html`, o `sw.js` e o manifest sao revalidados SEMPRE. Sao os tres
///   arquivos cujo nome nao muda, e por isso os tres unicos que podem envelhecer
///   sem ninguem notar.
fn cache_de(nome: &str) -> String {
    if nome.starts_with("assets/") || nome.starts_with("fontes/") {
        // Um ano, e `immutable`: o navegador nem pergunta.
        //
        // As fontes entram aqui porque o numero da versao esta no NOME
        // (`...-v9.woff2`): elas nao ganham hash do vite por virem de `public/`,
        // entao a regra e humana — trocar o arquivo obriga a trocar o nome. Sem
        // isso, um ano de cache serviria a fonte velha para sempre.
        String::from("public, max-age=31536000, immutable")
    } else {
        // `no-cache` NAO e "nao guarde": e "guarde, mas pergunte antes de usar".
        // Com `no-store` a PWA baixaria tudo de novo a cada abertura, inclusive
        // no 4G.
        String::from("no-cache")
    }
}

/// Caminho desconhecido devolve o `index.html`, e nao 404: a PWA e uma pagina
/// so, e um app instalado na tela de inicio recarrega numa rota interna o tempo
/// todo.
///
/// # A excecao, e por que ela custou uma tarde
///
/// `/assets/*` e `/fontes/*` NAO caem no `index.html`. Eles tem hash ou versao
/// no nome, entao um pedido a um arquivo que este binario nao tem so acontece
/// quando o navegador guardou um `index.html` VELHO — o de antes do deploy.
///
/// Devolver o `index.html` ali produzia o pior sintoma possivel: o navegador
/// pedia um `.js`, recebia HTML com `Content-Type: text/html`, recusava
/// executar por causa do tipo, e nao mostrava erro nenhum. Tela branca, no app
/// instalado, sem nada dizendo o que houve — foi exatamente o que aconteceu.
///
/// Com 404 de verdade, a mesma falha aparece no console como "404 em
/// index-ABC.js" e diz sozinha o que fazer: recarregar.
fn e_arquivo_carimbado(caminho: &str) -> bool {
    caminho.starts_with("assets/") || caminho.starts_with("fontes/")
}

async fn pagina(uri: axum::http::Uri) -> Response {
    let caminho = uri.path().trim_start_matches('/');
    let (nome, arquivo) = match Estaticos::get(caminho) {
        Some(encontrado) => (caminho, Some(encontrado)),
        None if e_arquivo_carimbado(caminho) => (caminho, None),
        None => ("index.html", Estaticos::get("index.html")),
    };
    match arquivo {
        Some(conteudo) => {
            let tipo = mime_guess::from_path(nome).first_or_octet_stream();
            (
                StatusCode::OK,
                [
                    (axum::http::header::CONTENT_TYPE, tipo.as_ref().to_owned()),
                    (axum::http::header::CACHE_CONTROL, cache_de(nome)),
                ],
                conteudo.data,
            )
                .into_response()
        }
        None if e_arquivo_carimbado(nome) => (
            StatusCode::NOT_FOUND,
            [(axum::http::header::CACHE_CONTROL, "no-store")],
            "Este arquivo nao existe nesta versao. Recarregue a pagina.",
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            "A interface nao foi embutida neste binario.",
        )
            .into_response(),
    }
}

// ------------------------------------------------------------------ erros

struct Erro(StatusCode, String);

impl IntoResponse for Erro {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({ "erro": self.1 }))).into_response()
    }
}

/// O erro do dominio vira status HTTP pela sua PROPRIA classificacao.
///
/// Adivinhar pelo texto seria decidir por acaso — e o `mos-core` ja responde a
/// pergunta com `ErrorCode`, que existe justamente para nao ser interpretado.
fn de_core(causa: CoreError) -> Erro {
    use mos_core::ErrorCode;
    let status = match causa.code {
        ErrorCode::InvalidInput => StatusCode::BAD_REQUEST,
        ErrorCode::NotFound => StatusCode::NOT_FOUND,
        ErrorCode::InvalidTransition => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    Erro(status, causa.message)
}

type Resultado<T> = Result<T, Erro>;

/// Dispara o sync sem fazer a resposta esperar por ele.
fn empurrar(estado: &Estado) {
    if let Some(hub) = &estado.hub {
        crate::sync::agora(
            std::sync::Arc::clone(&estado.storage),
            std::sync::Arc::clone(hub),
        );
    }
}

/// TODA escrita de dominio passa por aqui, numa thread de bloqueio.
///
/// # Por que nao direto no handler
///
/// O que corre aqui dentro e SQLite bloqueante, e ele pode esperar: o portao do
/// `SqliteStorage` faz uma escrita aguardar a rodada de sync em curso terminar —
/// e uma rodada e uma ida a rede. Num worker do tokio essa espera prenderia a
/// thread que serve as outras requisicoes, e a inbox pararia de carregar porque
/// alguem mandou uma captura.
///
/// # O que este arquivo NAO precisa mais fazer
///
/// Serializar escrita contra rodada. Isso morava aqui como remendo — os dois
/// caminhos pegavam os cadeados do `SqliteStorage` em ordem contraria, e o
/// encontro travava o servidor para sempre. A ordem foi consertada no crate
/// (`SqliteStorage::portao`), que e onde os cadeados moram; um remendo aqui em
/// cima so faria parecer que o crate ainda nao resolve isso.
async fn escrever<T, F>(estado: &Estado, tarefa: F) -> Resultado<T>
where
    F: FnOnce(&Estado) -> Result<T, CoreError> + Send + 'static,
    T: Send + 'static,
{
    let meu = estado.clone();
    let feito = tokio::task::spawn_blocking(move || tarefa(&meu))
        .await
        .map_err(|causa| Erro(StatusCode::INTERNAL_SERVER_ERROR, causa.to_string()))?
        .map_err(de_core)?;

    empurrar(estado);
    Ok(feito)
}

// ------------------------------------------------------------------ rotas

#[derive(Deserialize)]
pub struct Captura {
    pub texto: String,
}

async fn capturar(
    State(estado): State<Estado>,
    Json(pedido): Json<Captura>,
) -> Resultado<Json<serde_json::Value>> {
    let capture = escrever(&estado, move |estado| {
        estado.captures.create(CreateCaptureInput {
            content: pedido.texto,
            // A origem diz de ONDE veio, e isso e informacao de verdade: uma
            // captura feita no celular no meio da rua tem outra natureza da que
            // foi digitada no PC com o projeto aberto.
            source: CaptureSource::QuickCapture,
        })
    })
    .await?;

    Ok(Json(serde_json::json!({ "id": capture.id.to_string() })))
}

async fn inbox(State(estado): State<Estado>) -> Resultado<Json<serde_json::Value>> {
    let itens = estado.captures.inbox(LIMITE).map_err(de_core)?;
    Ok(Json(serde_json::to_value(itens).unwrap_or_default()))
}

async fn tasks(State(estado): State<Estado>) -> Resultado<Json<serde_json::Value>> {
    let itens = estado.work.tasks(false).map_err(de_core)?;
    Ok(Json(serde_json::to_value(itens).unwrap_or_default()))
}

#[derive(Deserialize)]
pub struct NovaTask {
    pub titulo: String,
    #[serde(default)]
    pub descricao: String,
    #[serde(default)]
    pub project_id: Option<String>,
}

async fn criar_task(
    State(estado): State<Estado>,
    Json(pedido): Json<NovaTask>,
) -> Resultado<Json<serde_json::Value>> {
    let task = escrever(&estado, move |estado| {
        estado.work.create_task(CreateTaskInput {
            title: pedido.titulo,
            description: pedido.descricao,
            project_id: pedido.project_id,
            source_capture_id: None,
        })
    })
    .await?;

    Ok(Json(serde_json::to_value(task).unwrap_or_default()))
}

#[derive(Deserialize)]
pub struct MudarEstado {
    pub estado: TaskState,
}

async fn mudar_estado(
    State(estado): State<Estado>,
    Path(id): Path<String>,
    Json(pedido): Json<MudarEstado>,
) -> Resultado<Json<serde_json::Value>> {
    let task = escrever(&estado, move |estado| {
        estado.work.set_task_state(&id, pedido.estado)
    })
    .await?;

    Ok(Json(serde_json::to_value(task).unwrap_or_default()))
}

/// Uma Task so, pelo id.
async fn task(
    State(estado): State<Estado>,
    Path(id): Path<String>,
) -> Resultado<Json<serde_json::Value>> {
    let task = estado.work.task(&id).map_err(de_core)?;
    Ok(Json(serde_json::to_value(task).unwrap_or_default()))
}

/// O que se pode mudar numa Task pela tela. Ausente significa "nao mexi".
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EdicaoDeTask {
    titulo: Option<String>,
    descricao: Option<String>,
    /// `Some(None)` — `"projectId": null` — desliga o projeto. `None` deixa como
    /// esta. A dupla-opcao existe porque desvincular e uma escolha, e sem ela
    /// nao haveria como expressa-la.
    #[serde(default, deserialize_with = "opcao_dupla")]
    project_id: Option<Option<String>>,
}

/// Distingue "campo ausente" de "campo presente com null".
fn opcao_dupla<'de, D>(deserializer: D) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer).map(Some)
}

/// Editar uma Task.
///
/// # Por que le antes de escrever
///
/// O `UpdateTaskInput` do nucleo pede titulo e descricao INTEIROS — ele nasceu
/// para um formulario de desktop, que sempre tem os dois na tela. A tela do
/// bolso manda so o que mudou, entao a rota completa o resto com o que esta
/// gravado AGORA, e nao com o que a tela leu ha dois minutos.
///
/// A diferenca importa quando os dois aparelhos mexem na mesma Task: sem a
/// leitura, o celular que corrigiu o titulo reescreveria a descricao com uma
/// versao velha, e o sync — que resolve por campo — nao teria como saber que
/// aquilo nao foi uma edicao.
async fn editar_task(
    State(estado): State<Estado>,
    Path(id): Path<String>,
    Json(pedido): Json<EdicaoDeTask>,
) -> Resultado<Json<serde_json::Value>> {
    if pedido.titulo.is_none() && pedido.descricao.is_none() && pedido.project_id.is_none() {
        return Err(Erro(
            StatusCode::BAD_REQUEST,
            String::from("nada para mudar"),
        ));
    }
    let task = escrever(&estado, move |estado| {
        let atual = estado.work.task(&id)?;
        estado.work.update_task(mos_core::UpdateTaskInput {
            id,
            title: pedido.titulo.unwrap_or(atual.title),
            description: pedido.descricao.unwrap_or(atual.description),
            project_id: match pedido.project_id {
                Some(escolha) => escolha,
                None => atual.project_id.map(|id| id.to_string()),
            },
        })
    })
    .await?;
    Ok(Json(serde_json::to_value(task).unwrap_or_default()))
}

/// Arquivar: o "excluir" da tela, pela mesma razao do lembrete — um toque errado
/// no onibus nao deveria apagar a linha nos dois aparelhos.
async fn arquivar_task(
    State(estado): State<Estado>,
    Path(id): Path<String>,
) -> Resultado<Json<serde_json::Value>> {
    let task = escrever(&estado, move |estado| {
        estado.work.set_task_archived(&id, true)
    })
    .await?;
    Ok(Json(serde_json::to_value(task).unwrap_or_default()))
}

/// Os projetos ativos.
///
/// Existe para a tela poder DIZER a que projeto uma Task pertence, e para
/// agrupar horas e tasks por projeto. Sem eles o bolso mostra um id, que nao e
/// nome de nada.
async fn projetos(State(estado): State<Estado>) -> Resultado<Json<serde_json::Value>> {
    let itens = estado.work.projects(false).map_err(de_core)?;
    Ok(Json(serde_json::to_value(itens).unwrap_or_default()))
}

// -------------------------------------------------------------- lembretes

/// # Por que CRIAR lembrete aqui nao contradiz o `avisos.rs`
///
/// O `avisos.rs` abre dizendo que este aparelho **le** lembretes e **nao
/// escreve** nenhum, e isso continua verdade do jeito que importa: ele nao
/// escreve estado de ENTREGA. Marcar "entregue" e uma decisao que o desktop
/// tambem toma sobre o mesmo lembrete, e dois agendadores disputando a mesma
/// coluna produzem o lembrete que some do PC porque o celular achou que ja tinha
/// dado conta.
///
/// Criar e concluir sao outra coisa: sao a PESSOA decidindo, uma vez, num
/// aparelho so. Elas sincronizam como a Task criada no bolso ja sincroniza — e
/// recusa-las aqui significaria que a unica forma de lembrar de algo na rua e
/// esperar chegar em casa.
///
/// # O instante chega RESOLVIDO
///
/// "Amanha de manha" e um conceito local, e este servidor roda numa VPS cujo
/// fuso nao e o de quem tocou no botao. A tela calcula e manda RFC 3339; o
/// servidor guarda UTC e nunca adivinha. E a regra normativa da
/// `CORE-FOUNDATION.md` §5, e o mesmo caminho que o `ReminderComposer` do
/// desktop segue.
#[derive(Deserialize)]
pub struct NovoLembrete {
    pub titulo: String,
    #[serde(default)]
    pub nota: String,
    /// RFC 3339, ja no instante exato. Ver acima.
    pub quando: String,
    /// A entidade a que ele se prende, quando se prende. Tipo e id andam
    /// juntos — um alvo pela metade e um alvo que nao abre nada ao ser tocado.
    #[serde(default)]
    pub alvo_tipo: Option<String>,
    #[serde(default)]
    pub alvo_id: Option<String>,
}

fn instante(valor: &str) -> Result<time::OffsetDateTime, Erro> {
    time::OffsetDateTime::parse(valor, &time::format_description::well_known::Rfc3339).map_err(
        |_| {
            Erro(
                StatusCode::BAD_REQUEST,
                String::from("Instante invalido: esperava RFC 3339."),
            )
        },
    )
}

fn alvo(
    tipo: Option<String>,
    id: Option<String>,
) -> Result<Option<mos_core::ReminderTarget>, Erro> {
    match (tipo, id) {
        (Some(tipo), Some(id)) => mos_core::ReminderTarget::from_columns(&tipo, &id)
            .map(Some)
            .map_err(de_core),
        (None, None) => Ok(None),
        _ => Err(Erro(
            StatusCode::BAD_REQUEST,
            String::from("Alvo incompleto: tipo e id andam juntos."),
        )),
    }
}

/// O que a tela mostra: o que ainda espera alguma coisa.
async fn lembretes(State(estado): State<Estado>) -> Resultado<Json<serde_json::Value>> {
    let itens = estado.attention.open().map_err(de_core)?;
    Ok(Json(serde_json::to_value(itens).unwrap_or_default()))
}

async fn criar_lembrete(
    State(estado): State<Estado>,
    Json(pedido): Json<NovoLembrete>,
) -> Resultado<Json<serde_json::Value>> {
    let quando = instante(&pedido.quando)?;
    let alvo = alvo(pedido.alvo_tipo, pedido.alvo_id)?;

    let lembrete = escrever(&estado, move |estado| {
        estado.attention.create_at(
            &pedido.titulo,
            &pedido.nota,
            quando,
            alvo,
            // `User` e nao `System`: quem tocou no botao foi a pessoa. A origem
            // alimenta o Attention Score, e um lembrete que a pessoa criou
            // contando como regra automatica falsearia a conta.
            mos_core::ReminderSource::User,
        )
    })
    .await?;

    Ok(Json(serde_json::to_value(lembrete).unwrap_or_default()))
}

/// Concluir e cancelar, e mais nada.
///
/// Adiar existe no dominio e NAO esta aqui de proposito: `Snooze` mexe no
/// `next_due_at`, que e exatamente a coluna que o agendador do desktop le. As
/// duas transicoes abaixo levam o lembrete para estado TERMINAL — depois delas
/// nenhum agendador olha mais para ele, e nao ha o que disputar.
async fn transitar(
    estado: &Estado,
    id: &str,
    transicao: mos_core::Transition,
) -> Resultado<Json<serde_json::Value>> {
    let id = mos_core::ReminderId::parse(id).map_err(de_core)?;
    let lembrete = escrever(estado, move |estado| {
        estado.attention.transition(id, transicao)
    })
    .await?;

    Ok(Json(serde_json::to_value(lembrete).unwrap_or_default()))
}

async fn concluir_lembrete(
    State(estado): State<Estado>,
    Path(id): Path<String>,
) -> Resultado<Json<serde_json::Value>> {
    transitar(&estado, &id, mos_core::Transition::Complete).await
}

async fn cancelar_lembrete(
    State(estado): State<Estado>,
    Path(id): Path<String>,
) -> Resultado<Json<serde_json::Value>> {
    transitar(&estado, &id, mos_core::Transition::Cancel).await
}

/// O que se pode mudar num lembrete pela tela.
///
/// Todo campo e opcional, e a ausencia significa "nao mexi" — nao "apague". E a
/// mesma distincao do `EditReminder` do nucleo, e ela existe porque o sync
/// resolve conflito POR CAMPO: a tela que so mexeu no titulo nao pode reescrever
/// a hora com o valor que leu ha dois minutos.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EdicaoDeLembrete {
    titulo: Option<String>,
    nota: Option<String>,
    /// RFC 3339, com fuso. Mesmo formato da criacao.
    quando: Option<String>,
    prioridade: Option<String>,
}

/// Editar: titulo, nota, hora, prioridade.
///
/// PATCH e nao PUT: o corpo carrega o que mudou, e nao o lembrete inteiro.
async fn editar_lembrete(
    State(estado): State<Estado>,
    Path(id): Path<String>,
    Json(pedido): Json<EdicaoDeLembrete>,
) -> Resultado<Json<serde_json::Value>> {
    let id = mos_core::ReminderId::parse(&id).map_err(de_core)?;
    let quando = match pedido.quando.as_deref() {
        Some(texto) => Some(instante(texto)?),
        None => None,
    };
    let prioridade = match pedido.prioridade.as_deref() {
        Some(texto) => Some(mos_core::Priority::parse(texto).map_err(de_core)?),
        None => None,
    };
    let mudanca = mos_core::EditReminder {
        title: pedido.titulo,
        body: pedido.nota,
        instant: quando,
        priority: prioridade,
    };
    if mudanca.is_empty() {
        return Err(Erro(
            StatusCode::BAD_REQUEST,
            String::from("nada para mudar"),
        ));
    }

    let lembrete = escrever(&estado, move |estado| estado.attention.update(id, mudanca)).await?;
    Ok(Json(serde_json::to_value(lembrete).unwrap_or_default()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Adiamento {
    /// Ate quando. RFC 3339, com fuso.
    ate: String,
}

/// Adiar.
///
/// # Por que ele agora esta aqui, se o comentario dizia que nao estaria
///
/// O comentario acima de `transitar` recusava `Snooze` porque ele mexe no
/// `next_due_at`, a coluna que o agendador do desktop le — e dois agendadores
/// disputando a mesma coluna produziria o lembrete que some.
///
/// A disputa nao existe: o `mos-web` nao TEM agendador de lembrete que escreva.
/// O `avisos.rs` le e nao escreve, de proposito e por escrito. Quem escreve
/// `next_due_at` e a pessoa — aqui ou no PC — e isso o sync ja resolve por
/// campo, como resolve qualquer outra edicao.
///
/// O que continua valendo do comentario antigo: nenhum aviso automatico deste
/// servidor mexe em lembrete. Adiar e um toque, nao uma regra.
async fn adiar_lembrete(
    State(estado): State<Estado>,
    Path(id): Path<String>,
    Json(pedido): Json<Adiamento>,
) -> Resultado<Json<serde_json::Value>> {
    let ate = instante(&pedido.ate)?;
    transitar(&estado, &id, mos_core::Transition::Snooze { until: ate }).await
}

/// Arquivar: o "excluir" da tela.
///
/// Nao ha apagar de verdade aqui, e e decisao e nao limitacao. Apagar um
/// lembrete no celular apagaria a linha nos dois aparelhos, e um toque errado no
/// onibus nao deveria ser irreversivel. Arquivado some da lista e continua no
/// banco — o Desktop, que e onde se organiza a fundo, e quem apaga de vez.
async fn arquivar_lembrete(
    State(estado): State<Estado>,
    Path(id): Path<String>,
) -> Resultado<Json<serde_json::Value>> {
    let id = mos_core::ReminderId::parse(&id).map_err(de_core)?;
    let lembrete = escrever(&estado, move |estado| {
        estado
            .attention
            .set_lifecycle(id, mos_core::LifecycleState::Archived)
    })
    .await?;
    Ok(Json(serde_json::to_value(lembrete).unwrap_or_default()))
}

/// Um lembrete so, pelo id.
async fn lembrete(
    State(estado): State<Estado>,
    Path(id): Path<String>,
) -> Resultado<Json<serde_json::Value>> {
    let id = mos_core::ReminderId::parse(&id).map_err(de_core)?;
    let lembrete = estado.attention.reminder(id).map_err(de_core)?;
    Ok(Json(serde_json::to_value(lembrete).unwrap_or_default()))
}

/// O historico: o que ja foi resolvido.
///
/// Separado da lista aberta, e nao misturado nela: sao duas perguntas — *o que
/// falta* e *o que eu resolvi* — e juntar as duas faria a primeira, que e a
/// urgente, ser lida atraves da segunda.
async fn lembretes_resolvidos(State(estado): State<Estado>) -> Resultado<Json<serde_json::Value>> {
    let itens = estado.attention.resolved(50).map_err(de_core)?;
    Ok(Json(serde_json::to_value(itens).unwrap_or_default()))
}

// -------------------------------------------------------------------- push

/// O push, ou o erro que explica a ausencia dele.
///
/// 501 e nao 500: nao ha defeito aqui, ha uma configuracao ausente. A tela
/// mostra a diferenca, e quem le o log tambem.
fn sem_push(estado: &Estado) -> Resultado<&crate::estado::PushLigado> {
    estado.push.as_ref().ok_or_else(|| {
        Erro(
            StatusCode::NOT_IMPLEMENTED,
            String::from(
                "este servidor nao tem chave VAPID configurada, entao nao manda                  notificacao nenhuma",
            ),
        )
    })
}

/// A tela assina.
///
/// O corpo e o `PushSubscription.toJSON()` do navegador, repassado inteiro. O
/// servidor nao interpreta nada dele — ver `push.rs`.
async fn assinar_push(
    State(estado): State<Estado>,
    Json(assinatura): Json<crate::push::Assinatura>,
) -> Resultado<Json<serde_json::Value>> {
    let push = sem_push(&estado)?;

    let agora_ms = (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64;
    push.assinaturas
        .salvar(&assinatura, agora_ms)
        .map_err(|causa| Erro(StatusCode::INTERNAL_SERVER_ERROR, causa.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// Manda uma notificacao agora, para conferir.
///
/// Existe porque a alternativa para saber se o push funciona seria criar um
/// lembrete e ESPERAR ele vencer. Depois de instalar na tela de inicio, esta e a
/// unica pergunta que importa — chega ou nao chega? —, e ela merece resposta em
/// dois segundos.
async fn testar_push(State(estado): State<Estado>) -> Resultado<Json<serde_json::Value>> {
    let push = sem_push(&estado)?;

    let avisador = std::sync::Arc::clone(&push.avisador);
    let aviso = crate::avisos::Aviso {
        titulo: String::from("M/OS"),
        corpo: String::from("Se voce esta lendo isto, a notificacao funciona."),
        tag: String::from("teste"),
        url: String::from("/"),
        // O teste nao mexe no badge. Ele existe para responder "chega ou nao
        // chega?", e trocar o numero do icone de passagem faria a prova mentir
        // sobre o estado do app.
        badge: None,
    };
    // `spawn_blocking` porque o envio e bloqueante — ver `push::enviar`.
    let aceitos = tokio::task::spawn_blocking(move || avisador.disparar(&aviso))
        .await
        .unwrap_or(0);

    Ok(Json(serde_json::json!({ "enviadas": aceitos })))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EstadoDoAparelho {
    /// Mudancas locais esperando para subir. E o unico sinal honesto de que a
    /// sincronizacao esta ou nao acontecendo: se este numero nao baixa, algo
    /// esta errado, e a tela precisa poder dizer isso.
    pendentes: usize,
    /// Se ha hub configurado. Sem ele o `mos-web` funciona — sozinho.
    sincroniza: bool,
    /// A chave publica VAPID, ou vazio quando este servidor nao manda
    /// notificacao. A tela decide o que mostrar a partir disto: com chave, o
    /// botao de ativar; sem chave, a frase que explica por que ele nao existe.
    chave_push: Option<String>,
    /// Quantos aparelhos ja assinaram. Serve para voce saber que ativou — sem
    /// isso, "ativar" e um botao que muda de cor e nao prova nada.
    aparelhos_avisados: usize,
}

async fn estado_do_aparelho(State(estado): State<Estado>) -> Json<EstadoDoAparelho> {
    use mos_sync::OutboxRepository;
    Json(EstadoDoAparelho {
        pendentes: estado.storage.quantidade_pendente().unwrap_or(0),
        sincroniza: estado.hub.is_some(),
        chave_push: estado.push.as_ref().map(|push| push.chave_publica.clone()),
        aparelhos_avisados: estado
            .push
            .as_ref()
            .and_then(|push| push.assinaturas.quantas().ok())
            .unwrap_or(0),
    })
}

// --------------------------------------------------------------- panorama

/// O instante do APARELHO, com o fuso dele.
///
/// O servidor roda na VPS em UTC, e quem pergunta esta em UTC-3. Calcular "esta
/// semana" pelo relogio do servidor cortaria a semana as 21h de sabado, no fuso
/// de quem le. Entao o corte vem do aparelho: o fuso fica onde ele e conhecido,
/// e o servidor nao ganha configuracao de timezone para alguem errar depois.
#[derive(Deserialize)]
struct QuandoPergunta {
    /// RFC3339 com offset. Ausente ou ilegivel: cai no relogio do servidor, que
    /// e melhor que recusar a tela inteira por causa de um parametro.
    agora: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Horas {
    /// Segundos faturaveis da semana, ja arredondados por sessao.
    semana_segundos: i64,
    /// O que isso vale, em centavos.
    semana_valor_cents: i64,
    /// Segundos faturaveis de hoje.
    hoje_segundos: i64,
    /// Os sete dias da semana, de segunda a domingo, em segundos faturaveis.
    ///
    /// Existe para o desenho, e nao para o numero: o cartao da Home mostra uma
    /// barra por dia, e a pergunta que ela responde — *onde foi o meu tempo* —
    /// nao tem resposta num total. Dia futuro vem zero, e a tela o desenha como
    /// traco apagado: zero de "ainda nao aconteceu" nao e o mesmo zero de "nao
    /// trabalhei", mas essa distincao e da tela, que sabe que dia e hoje.
    dias_segundos: Vec<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CompromissoProximo {
    titulo: String,
    disciplina: String,
    /// RFC3339, como o dominio guarda.
    quando: String,
    /// `assignment` ou `exam`.
    tipo: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Panorama {
    horas: Horas,
    /// Ate tres, do mais proximo para o mais distante. Vazio e resposta valida.
    proximos: Vec<CompromissoProximo>,
}

/// O que a Home do bolso mostra alem do que ela ja tinha.
///
/// Uma chamada so, e nao tres: o celular abre no 4G, e cada ida a rede e um
/// segundo de tela vazia.
async fn panorama(
    State(estado): State<Estado>,
    Query(pergunta): Query<QuandoPergunta>,
) -> Resultado<Json<Panorama>> {
    let agora = pergunta
        .agora
        .as_deref()
        .and_then(|texto| {
            time::OffsetDateTime::parse(texto, &time::format_description::well_known::Rfc3339).ok()
        })
        .unwrap_or_else(time::OffsetDateTime::now_utc);

    // A semana comeca na segunda, como no desktop. `days_from_monday` conta a
    // partir dela, entao subtrair isso do dia de hoje da o inicio.
    let dias_desde_segunda = agora.weekday().number_days_from_monday() as i64;
    let inicio_do_dia = agora.replace_time(time::Time::MIDNIGHT);
    let inicio_da_semana = inicio_do_dia - time::Duration::days(dias_desde_segunda);

    let linhas = estado
        .tracking
        .report(Some(inicio_da_semana), Some(agora))
        .map_err(de_core)?;
    let semana_segundos = linhas
        .iter()
        .map(|linha| linha.totals.billable_seconds)
        .sum();
    let semana_valor_cents = linhas.iter().map(|linha| linha.totals.amount_cents).sum();
    let hoje_segundos = linhas
        .iter()
        .filter(|linha| linha.started_at >= inicio_do_dia)
        .map(|linha| linha.totals.billable_seconds)
        .sum();

    // A sessao cai no dia em que COMECOU. Uma sessao que atravessa a
    // meia-noite existe, e reparti-la entre os dois dias exigiria conhecer o
    // fuso de quem olha para saber onde cortar — o servidor nao conhece.
    let mut dias_segundos = vec![0_i64; 7];
    for linha in &linhas {
        let indice = (linha.started_at.date() - inicio_da_semana.date()).whole_days();
        if (0..7).contains(&indice) {
            dias_segundos[indice as usize] += linha.totals.billable_seconds;
        }
    }

    // O academico falha em silencio: sem semestre cadastrado ele nao tem o que
    // dizer, e derrubar o panorama inteiro por causa disso apagaria as horas da
    // tela junto.
    let proximos = estado
        .academic
        .today(agora)
        .map(|hoje| {
            let mut compromissos: Vec<_> = hoje
                .due_today
                .into_iter()
                .chain(hoje.exams_soon)
                .map(|compromisso| CompromissoProximo {
                    titulo: compromisso.title,
                    disciplina: compromisso.subject,
                    quando: compromisso
                        .at
                        .format(&time::format_description::well_known::Rfc3339)
                        .unwrap_or_default(),
                    tipo: compromisso.kind,
                })
                .collect();
            compromissos.sort_by(|a, b| a.quando.cmp(&b.quando));
            compromissos.truncate(3);
            compromissos
        })
        .unwrap_or_default();

    Ok(Json(Panorama {
        horas: Horas {
            semana_segundos,
            semana_valor_cents,
            hoje_segundos,
            dias_segundos,
        },
        proximos,
    }))
}

// ----------------------------------------------------------------- agenda

/// A janela vem como INSTANTE, e nao como data.
///
/// Quem decide onde um dia comeca e o aparelho, que conhece o fuso de quem esta
/// olhando — mesma razao do `agora` do panorama. O servidor so responde "o que
/// aconteceu entre X e Y".
#[derive(Deserialize)]
struct Janela {
    desde: String,
    ate: String,
}

/// Tudo o que o M/OS registrou entre dois instantes, em ordem crescente.
///
/// A composicao vive em `mos_core::compose`, que e pura e testada, e e a MESMA
/// que o desktop usa. Esta rota so busca e delega: duplicar aqui a regra de o
/// que entra na janela daria duas respostas para "esta prova conta?".
async fn agenda(
    State(estado): State<Estado>,
    Query(janela): Query<Janela>,
) -> Resultado<Json<Vec<mos_core::CalendarItem>>> {
    let de = mos_core::parse_moment(&janela.desde).map_err(de_core)?;
    let ate = mos_core::parse_moment(&janela.ate).map_err(de_core)?;
    if ate < de {
        return Err(Erro(
            StatusCode::BAD_REQUEST,
            "O fim da janela vem antes do inicio.".to_owned(),
        ));
    }

    // Cada leitura numa variavel propria: passadas direto como referencia, os
    // temporarios morreriam antes de `compose` usa-los.
    let projetos = estado.work.projects(true).map_err(de_core)?;
    let horas = estado.tracking.entries(None).map_err(de_core)?;
    let tasks = estado.work.tasks(true).map_err(de_core)?;
    let capturas = estado.captures.between(de, ate).map_err(de_core)?;
    let arredondamento = estado.tracking.settings().map_err(de_core)?.rounding;
    let sessoes = estado.daily.sessions(365).map_err(de_core)?;
    let ids: Vec<_> = sessoes.iter().map(|sessao| sessao.id).collect();
    let objetivos = estado.daily.objectives_of(&ids).map_err(de_core)?;
    let academico = estado
        .academic
        .compromissos_entre(de, ate, ate)
        .map_err(de_core)?;

    let nome_do_projeto = |id: mos_core::ProjectId| {
        projetos
            .iter()
            .find(|projeto| projeto.id == id)
            .map(|projeto| projeto.name.clone())
            .unwrap_or_else(|| "Project removido".to_owned())
    };

    Ok(Json(mos_core::compose(mos_core::ComposeInput {
        since: de,
        until: ate,
        rounding: arredondamento,
        entries: &horas,
        tasks: &tasks,
        captures: &capturas,
        // O bolso NAO tem eventos de monitoramento, e nao e falta: `apps` e
        // `activity_events` sao tabelas locais por decisao — elas descrevem o
        // que aconteceu NAQUELA maquina, e o celular nao vigia programa nenhum.
        events: &[],
        sessions: &sessoes,
        objectives: &objetivos,
        academic: &academico,
        project_name: &nome_do_projeto,
    })))
}

// ------------------------------------------------------------------ horas

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HorasDeProjeto {
    projeto: String,
    /// Segundos faturaveis, ja arredondados por sessao.
    segundos: i64,
    valor_cents: i64,
    /// Quantos lancamentos somaram isso. E o numero que separa "um dia inteiro"
    /// de "vinte visitas de dez minutos".
    lancamentos: usize,
}

/// As horas da janela, agrupadas por projeto e do maior para o menor.
///
/// Agrupar AQUI e nao na tela: o arredondamento acontece por sessao, entao somar
/// depois de arredondar e a unica ordem que da o mesmo numero do desktop.
async fn horas(
    State(estado): State<Estado>,
    Query(janela): Query<Janela>,
) -> Resultado<Json<Vec<HorasDeProjeto>>> {
    let de = mos_core::parse_moment(&janela.desde).map_err(de_core)?;
    let ate = mos_core::parse_moment(&janela.ate).map_err(de_core)?;
    if ate < de {
        return Err(Erro(
            StatusCode::BAD_REQUEST,
            "O fim da janela vem antes do inicio.".to_owned(),
        ));
    }

    let projetos = estado.work.projects(true).map_err(de_core)?;
    let linhas = estado
        .tracking
        .report(Some(de), Some(ate))
        .map_err(de_core)?;

    let mut por_projeto: std::collections::HashMap<String, HorasDeProjeto> =
        std::collections::HashMap::new();
    for linha in linhas {
        let nome = projetos
            .iter()
            .find(|projeto| projeto.id == linha.project_id)
            .map(|projeto| projeto.name.clone())
            .unwrap_or_else(|| "Project removido".to_owned());
        let entrada = por_projeto
            .entry(linha.project_id.to_string())
            .or_insert_with(|| HorasDeProjeto {
                projeto: nome,
                segundos: 0,
                valor_cents: 0,
                lancamentos: 0,
            });
        entrada.segundos += linha.totals.billable_seconds;
        entrada.valor_cents += linha.totals.amount_cents;
        entrada.lancamentos += 1;
    }

    let mut resposta: Vec<_> = por_projeto.into_values().collect();
    // Do maior para o menor: a pergunta e "onde foi o meu tempo", e a resposta
    // comeca pelo projeto que mais consumiu.
    resposta.sort_by_key(|linha| std::cmp::Reverse(linha.segundos));
    Ok(Json(resposta))
}

// -------------------------------------------------------------- academico

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CompromissoDaLista {
    titulo: String,
    disciplina: String,
    quando: String,
    /// `assignment` ou `exam`.
    tipo: String,
    /// `hoje`, `atrasado`, ou vazio para o que so vem por ai. E o que a tela usa
    /// para decidir o que pinta de sodio.
    urgencia: String,
}

/// O que vem por ai no academico, ate trinta dias.
///
/// Trinta e nao noventa: um compromisso a mais de um mes nao muda o que se faz
/// hoje, e uma lista que desce ate o fim do semestre e uma lista que ninguem le.
async fn academico(
    State(estado): State<Estado>,
    Query(pergunta): Query<QuandoPergunta>,
) -> Resultado<Json<Vec<CompromissoDaLista>>> {
    let agora = pergunta
        .agora
        .as_deref()
        .and_then(|texto| {
            time::OffsetDateTime::parse(texto, &time::format_description::well_known::Rfc3339).ok()
        })
        .unwrap_or_else(time::OffsetDateTime::now_utc);

    let hoje = match estado.academic.today(agora) {
        Ok(hoje) => hoje,
        // Sem semestre cadastrado nao ha o que listar, e isso nao e erro: a tela
        // sabe dizer "nada por aqui" melhor que um 500.
        Err(_) => return Ok(Json(Vec::new())),
    };

    let em = |compromisso: mos_core::Compromisso, urgencia: &str| CompromissoDaLista {
        titulo: compromisso.title,
        disciplina: compromisso.subject,
        quando: compromisso
            .at
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default(),
        tipo: compromisso.kind,
        urgencia: urgencia.to_owned(),
    };

    // O atrasado vem PRIMEIRO, e nao em ordem de data junto com o resto: ele e o
    // que ja falhou, e enterra-lo no meio da lista cronologica seria escondê-lo
    // justamente de quem precisa agir.
    let mut lista: Vec<_> = hoje
        .overdue
        .into_iter()
        .map(|compromisso| em(compromisso, "atrasado"))
        .collect();
    lista.extend(
        hoje.due_today
            .into_iter()
            .map(|compromisso| em(compromisso, "hoje")),
    );
    lista.extend(
        hoje.exams_soon
            .into_iter()
            .map(|compromisso| em(compromisso, "")),
    );
    Ok(Json(lista))
}
