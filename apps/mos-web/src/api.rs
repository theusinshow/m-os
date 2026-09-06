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
        .route("/api/capturas/{id}/task", post(capturar_para_task))
        .route(
            "/api/capturas/{id}/referencia",
            post(capturar_para_referencia),
        )
        .route("/api/capturas/{id}/arquivar", post(arquivar_captura))
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
        .route("/api/dia", get(dia))
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

// ---------------------------------------------------------------- triagem

/// # Por que a Capture nao tem TIPO
///
/// Porque ela e o registro cru, e nao a coisa. Um link colado as onze da noite
/// pode virar uma task, uma referencia para consultar depois, ou nada — e qual
/// dos tres so se sabe depois de olhar. Dar um tipo a ela na hora da captura
/// obrigaria a decidir no pior momento possivel: aquele em que a pessoa so
/// queria nao esquecer.
///
/// O tipo aparece quando ela e PROCESSADA. E o que estas rotas fazem, e as duas
/// operacoes ja existiam inteiras no nucleo — o bolso e que nao as alcancava.
/// A proveniencia sobrevive nas duas: `source_capture_id` diz de onde veio.

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VirarTask {
    /// O titulo da Task. Ausente usa o conteudo da propria Capture.
    titulo: Option<String>,
    #[serde(default)]
    descricao: String,
    project_id: Option<String>,
}

/// A Capture vira Task.
async fn capturar_para_task(
    State(estado): State<Estado>,
    Path(id): Path<String>,
    Json(pedido): Json<VirarTask>,
) -> Resultado<Json<serde_json::Value>> {
    let task = escrever(&estado, move |estado| {
        let captura = estado.captures.get(&id)?;
        let titulo = pedido
            .titulo
            .clone()
            .unwrap_or_else(|| captura.content.clone());
        let projeto = pedido
            .project_id
            .as_deref()
            .map(mos_core::ProjectId::parse)
            .transpose()?;
        // A versao com reminder, sem reminder: e a mesma chamada que o desktop
        // faz, e ela ja marca a Capture como processada na mesma transacao —
        // duas escritas separadas deixariam a Capture na inbox se a segunda
        // falhasse.
        estado
            .work
            .create_task_from_capture_with_reminder(&id, &titulo, &pedido.descricao, projeto, None)
            .map(|(task, _)| task)
    })
    .await?;
    Ok(Json(serde_json::to_value(task).unwrap_or_default()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VirarReferencia {
    titulo: Option<String>,
    /// O endereco. Ausente, a tela manda o proprio conteudo quando ele e um link.
    #[serde(default)]
    url: String,
    #[serde(default)]
    nota: String,
    /// `site`, `library`, `image`, `note`. Ausente vira `note` quando nao ha
    /// url, e `site` quando ha — que e o que o conteudo diz sobre si mesmo.
    tipo: Option<String>,
}

/// A Capture vira Resource — a referencia que se consulta, e nao a coisa que se
/// faz.
async fn capturar_para_referencia(
    State(estado): State<Estado>,
    Path(id): Path<String>,
    Json(pedido): Json<VirarReferencia>,
) -> Resultado<Json<serde_json::Value>> {
    let recurso = escrever(&estado, move |estado| {
        let captura = estado.captures.get(&id)?;
        let url = if pedido.url.trim().is_empty() {
            endereco_em(&captura.content).unwrap_or_default()
        } else {
            pedido.url.clone()
        };
        let tipo = match pedido.tipo.as_deref() {
            Some(texto) => mos_core::ResourceKind::parse(texto)?,
            None if url.is_empty() => mos_core::ResourceKind::Note,
            None => mos_core::ResourceKind::Site,
        };
        estado
            .memoria
            .create_resource(mos_core::CreateResourceInput {
                kind: tipo,
                title: pedido
                    .titulo
                    .clone()
                    .unwrap_or_else(|| titulo_curto(&captura.content)),
                url,
                note: pedido.nota.clone(),
                source_capture_id: Some(id.clone()),
            })
    })
    .await?;
    Ok(Json(serde_json::to_value(recurso).unwrap_or_default()))
}

/// Arquivar a Capture: nem task, nem referencia — nao era nada.
async fn arquivar_captura(
    State(estado): State<Estado>,
    Path(id): Path<String>,
) -> Resultado<Json<serde_json::Value>> {
    let captura = escrever(&estado, move |estado| estado.captures.archive(&id)).await?;
    Ok(Json(serde_json::to_value(captura).unwrap_or_default()))
}

/// O primeiro endereco dentro de um texto, se houver.
///
/// Existe porque a queixa que originou tudo isto era um LINK que parecia task.
/// Reconhece-lo permite a tela oferecer "guardar como referencia" antes de a
/// pessoa pedir — que e a diferenca entre o app entender o que voce colou e o
/// app tratar tudo como texto.
fn endereco_em(texto: &str) -> Option<String> {
    texto.split_whitespace().find_map(|bruto| {
        // A pontuacao que cerca o link sai dos DOIS lados. Um link colado no
        // fim de uma frase leva o ponto junto — e endereco com ponto no fim
        // abre pagina que nao existe. Entre parenteses, ele nem seria
        // reconhecido, porque a palavra comeca em `(`.
        let palavra = bruto
            .trim_start_matches(['(', '[', '<', '"', '\''])
            .trim_end_matches(['.', ',', ';', ':', ')', ']', '>', '"', '\'']);
        (palavra.starts_with("http://") || palavra.starts_with("https://"))
            .then(|| palavra.to_owned())
    })
}

/// Um titulo a partir do conteudo cru.
///
/// Uma Capture pode ser um paragrafo; um Resource com paragrafo no titulo fica
/// ilegivel em qualquer lista. Corta na primeira quebra de linha, e depois em
/// 80 — o suficiente para uma frase e pouco para um texto.
fn titulo_curto(conteudo: &str) -> String {
    let primeira = conteudo.lines().next().unwrap_or("").trim();
    if primeira.chars().count() <= 80 {
        return primeira.to_owned();
    }
    let cortado: String = primeira.chars().take(79).collect();
    format!("{}…", cortado.trim_end())
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

// ------------------------------------------------------------------- dia

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ObjetivoDoDia {
    id: String,
    titulo: String,
    /// `pending`, `done`, `dropped`, `carried`.
    status: String,
    prioridade: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ODia {
    /// `not_started`, `active` ou `ended`.
    status: String,
    objetivos: Vec<ObjetivoDoDia>,
    /// Quantos objetivos ja foram resolvidos — o numerador do anel.
    resolvidos: usize,
    /// Tasks concluidas hoje. Nao e o mesmo que objetivos: uma pessoa fecha
    /// tasks que nunca virou objetivo do dia, e o dia rendeu do mesmo jeito.
    feitas_hoje: usize,
    /// Dias seguidos com o dia ENCERRADO, contando para tras a partir de hoje
    /// ou de ontem.
    sequencia: usize,
}

/// O dia: o Start My Day visto do bolso.
///
/// # Por que uma rota, e nao um campo do panorama
///
/// O panorama responde *como estao as coisas* — numeros que nao mudam quando se
/// toca neles. O dia e estado com ciclo proprio: comeca, ganha objetivos,
/// encerra. Junta-los faria a Home recarregar o panorama inteiro a cada
/// objetivo marcado.
async fn dia(
    State(estado): State<Estado>,
    Query(pergunta): Query<QuandoPergunta>,
) -> Resultado<Json<ODia>> {
    let agora = pergunta
        .agora
        .as_deref()
        .and_then(|texto| {
            time::OffsetDateTime::parse(texto, &time::format_description::well_known::Rfc3339).ok()
        })
        .unwrap_or_else(time::OffsetDateTime::now_utc);

    let hoje = mos_core::Day::from_local(agora);
    let dia = estado.daily.today(&hoje).map_err(de_core)?;

    let objetivos: Vec<_> = dia
        .objectives
        .iter()
        .map(|objetivo| ObjetivoDoDia {
            id: objetivo.id.to_string(),
            titulo: objetivo.title.clone(),
            status: objetivo.status.as_str().to_owned(),
            prioridade: objetivo.priority.as_str().to_owned(),
        })
        .collect();
    let resolvidos = dia
        .objectives
        .iter()
        .filter(|objetivo| objetivo.status.is_resolved())
        .count();

    // Concluida HOJE, e nao "concluida": a pergunta que o cartao responde e
    // *o dia rendeu?*, e uma task fechada semana passada nao responde isso.
    let inicio_do_dia = agora.replace_time(time::Time::MIDNIGHT);
    let feitas_hoje = estado
        .work
        .tasks(false)
        .map_err(de_core)?
        .iter()
        .filter(|task| {
            task.completed_at
                .is_some_and(|quando| quando >= inicio_do_dia && quando <= agora)
        })
        .count();

    let sessoes = estado.daily.sessions(400).map_err(de_core)?;
    let sequencia = sequencia_de_dias(&sessoes, hoje);

    Ok(Json(ODia {
        status: dia.status.as_str().to_owned(),
        objetivos,
        resolvidos,
        feitas_hoje,
        sequencia,
    }))
}

/// Quantos dias seguidos foram ENCERRADOS, contando para tras.
///
/// # As duas decisoes que fazem a conta ser justa
///
/// **Começa em hoje OU em ontem.** Se exigisse hoje, a sequencia zeraria toda
/// manha e so voltaria a existir a noite — e um numero que passa metade do dia
/// mentindo nao serve para nada. Se aceitasse qualquer ponto de partida, ela
/// nunca zeraria.
///
/// **Conta o dia ENCERRADO, e nao o comecado.** Comecar o dia e uma intencao;
/// encerra-lo e o fato. Uma sequencia de dias que so foram abertos mediria
/// quantas vezes o app foi aberto de manha.
fn sequencia_de_dias(sessoes: &[mos_core::DailySession], hoje: mos_core::Day) -> usize {
    use std::collections::HashSet;
    let encerrados: HashSet<String> = sessoes
        .iter()
        .filter(|sessao| sessao.ended_at.is_some())
        .map(|sessao| sessao.day.as_str().to_owned())
        .collect();

    let mut cursor = if encerrados.contains(hoje.as_str()) {
        hoje
    } else {
        hoje.previous()
    };
    let mut dias = 0;
    // Teto igual ao que se leu: sem ele, um banco corrompido com o mesmo dia
    // repetido faria isto girar para sempre.
    while encerrados.contains(cursor.as_str()) && dias < sessoes.len() + 1 {
        dias += 1;
        cursor = cursor.previous();
    }
    dias
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

    // Os lembretes ABERTOS. Os resolvidos ficam de fora: o calendario mostra o
    // que vai acontecer e o que aconteceu, e um lembrete cancelado nao e nenhum
    // dos dois.
    let lembretes = estado.attention.open().map_err(de_core)?;
    // Nacionais, calculados a partir da janela. Estadual e municipal ficam para
    // quando existir uma fonte — ver `feriados.rs`.
    let feriados = mos_core::nacionais_entre(de.date(), ate.date());

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
        reminders: &lembretes,
        holidays: &feriados,
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

#[cfg(test)]
mod testes {
    use super::*;

    fn sessao(dia: &str, encerrada: bool) -> mos_core::DailySession {
        let instante = time::OffsetDateTime::UNIX_EPOCH;
        mos_core::DailySession {
            id: mos_core::DailySessionId::new(),
            day: mos_core::Day::parse(dia).unwrap(),
            status: if encerrada {
                mos_core::SessionStatus::Completed
            } else {
                mos_core::SessionStatus::Active
            },
            note: String::new(),
            started_at: instante,
            ended_at: encerrada.then_some(instante),
            created_at: instante,
            updated_at: instante,
        }
    }

    fn dia(texto: &str) -> mos_core::Day {
        mos_core::Day::parse(texto).unwrap()
    }

    #[test]
    fn a_sequencia_conta_dias_seguidos_encerrados() {
        let sessoes = [
            sessao("2026-09-05", true),
            sessao("2026-09-04", true),
            sessao("2026-09-03", true),
        ];
        assert_eq!(sequencia_de_dias(&sessoes, dia("2026-09-05")), 3);
    }

    /// A sequencia comeca em hoje OU em ontem.
    ///
    /// Se exigisse hoje, ela zeraria toda manha e so voltaria a existir a
    /// noite — e um numero que passa metade do dia mentindo nao serve.
    #[test]
    fn a_sequencia_sobrevive_ao_dia_que_ainda_nao_foi_encerrado() {
        let sessoes = [sessao("2026-09-04", true), sessao("2026-09-03", true)];
        assert_eq!(sequencia_de_dias(&sessoes, dia("2026-09-05")), 2);
    }

    /// Um buraco quebra a corrente. E o que faz o numero significar alguma
    /// coisa: sem isso ele contaria dias encerrados, e nao dias SEGUIDOS.
    #[test]
    fn um_dia_pulado_quebra_a_sequencia() {
        let sessoes = [
            sessao("2026-09-05", true),
            // 04 faltando
            sessao("2026-09-03", true),
            sessao("2026-09-02", true),
        ];
        assert_eq!(sequencia_de_dias(&sessoes, dia("2026-09-05")), 1);
    }

    /// Comecar o dia e uma intencao; encerra-lo e o fato. Uma sequencia de dias
    /// so abertos mediria quantas vezes o app foi aberto de manha.
    #[test]
    fn dia_comecado_e_nao_encerrado_nao_conta() {
        let sessoes = [sessao("2026-09-05", false), sessao("2026-09-04", false)];
        assert_eq!(sequencia_de_dias(&sessoes, dia("2026-09-05")), 0);
    }

    #[test]
    fn sem_sessao_nenhuma_a_sequencia_e_zero() {
        assert_eq!(sequencia_de_dias(&[], dia("2026-09-05")), 0);
    }

    /// A virada do mes e onde "o dia anterior" costuma quebrar.
    #[test]
    fn a_sequencia_atravessa_a_virada_do_mes() {
        let sessoes = [
            sessao("2026-09-01", true),
            sessao("2026-08-31", true),
            sessao("2026-08-30", true),
        ];
        assert_eq!(sequencia_de_dias(&sessoes, dia("2026-09-01")), 3);
    }
}

#[cfg(test)]
mod testes_de_link {
    use super::*;

    #[test]
    fn acha_o_link_no_meio_da_frase() {
        assert_eq!(
            endereco_em("tabela de aco https://exemplo.com/ca50 boa"),
            Some(String::from("https://exemplo.com/ca50"))
        );
    }

    /// A pontuacao sai dos dois lados: um endereco com ponto no fim abre pagina
    /// que nao existe, e entre parenteses ele nem era reconhecido.
    #[test]
    fn a_pontuacao_em_volta_nao_entra_no_endereco() {
        for texto in [
            "ver https://exemplo.com/a.",
            "ver (https://exemplo.com/a)",
            "ver [https://exemplo.com/a],",
            "ver \"https://exemplo.com/a\"",
        ] {
            assert_eq!(
                endereco_em(texto),
                Some(String::from("https://exemplo.com/a")),
                "{texto}"
            );
        }
    }

    #[test]
    fn texto_sem_link_nao_inventa_um() {
        assert_eq!(endereco_em("o fck do concreto e 30 MPa"), None);
        assert_eq!(endereco_em("exemplo.com sem protocolo"), None);
    }

    /// Uma Capture pode ser um paragrafo, e paragrafo em titulo de Resource
    /// fica ilegivel em qualquer lista.
    #[test]
    fn o_titulo_curto_corta_na_primeira_linha() {
        assert_eq!(titulo_curto("Primeira linha\nsegunda"), "Primeira linha");
    }

    #[test]
    fn o_titulo_curto_tem_teto() {
        let longo = "a".repeat(200);
        let curto = titulo_curto(&longo);
        assert!(curto.chars().count() <= 80, "{}", curto.chars().count());
        assert!(curto.ends_with('…'));
    }
}
