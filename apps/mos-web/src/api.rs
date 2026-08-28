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
    extract::{Path, State},
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
        .route("/api/tasks/{id}/estado", post(mudar_estado))
        .route("/api/lembretes", get(lembretes).post(criar_lembrete))
        .route("/api/lembretes/{id}/concluir", post(concluir_lembrete))
        .route("/api/lembretes/{id}/cancelar", post(cancelar_lembrete))
        .route("/api/estado", get(estado_do_aparelho))
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
    if nome.starts_with("assets/") {
        // Um ano, e `immutable`: o navegador nem pergunta.
        String::from("public, max-age=31536000, immutable")
    } else {
        // `no-cache` NAO e "nao guarde": e "guarde, mas pergunte antes de usar".
        // Com `no-store` a PWA baixaria tudo de novo a cada abertura, inclusive
        // no 4G.
        String::from("no-cache")
    }
}

/// Qualquer caminho desconhecido devolve o `index.html`, e nao 404: a PWA e uma
/// pagina so, e um app instalado na tela de inicio recarrega numa rota interna o
/// tempo todo.
async fn pagina(uri: axum::http::Uri) -> Response {
    let caminho = uri.path().trim_start_matches('/');
    let (nome, arquivo) = match Estaticos::get(caminho) {
        Some(encontrado) => (caminho, Some(encontrado)),
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
            std::sync::Arc::clone(&estado.vez),
        );
    }
}

/// TODA escrita de dominio passa por aqui, e a razao nao e arrumacao.
///
/// # A vez
///
/// Uma escrita e uma rodada de sync pegam os dois cadeados do `SqliteStorage` em
/// ordem contraria, e o encontro das duas trava o servidor **para sempre** — o
/// desenho inteiro do abraco esta em `estado::Estado::vez`. Como toda escrita
/// dispara uma rodada, o encontro nao e raro: e o que acontece quando alguem
/// captura duas coisas seguidas.
///
/// Passar por uma funcao so e o que impede o conserto de envelhecer: uma rota
/// nova escrita a mao nasceria sem a vez, e o defeito voltaria sem nada na tela
/// dizendo isso.
///
/// # E por que `spawn_blocking`
///
/// O que corre aqui dentro e SQLite bloqueante, e agora ele tambem pode esperar
/// uma rodada de rede terminar. Num worker do tokio isso prenderia a thread que
/// serve as outras requisicoes — a inbox pararia de carregar porque alguem
/// mandou uma captura.
async fn escrever<T, F>(estado: &Estado, tarefa: F) -> Resultado<T>
where
    F: FnOnce(&Estado) -> Result<T, CoreError> + Send + 'static,
    T: Send + 'static,
{
    let meu = estado.clone();
    let feito = tokio::task::spawn_blocking(move || {
        let _vez = crate::estado::tomar(&meu.vez);
        tarefa(&meu)
    })
    .await
    .map_err(|causa| Erro(StatusCode::INTERNAL_SERVER_ERROR, causa.to_string()))?
    .map_err(de_core)?;

    // Depois de soltar a vez, e nunca antes: a rodada que este empurrao dispara
    // precisa dela.
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
