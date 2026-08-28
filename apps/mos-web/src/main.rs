//! O M/OS de bolso.
//!
//! Uma porta, e nao um segundo M/OS — ver o `Cargo.toml`.
//!
//! # A porta, e por que ela e uma feature
//!
//! `passkey` liga a porta de verdade (WebAuthn). Ela e opcional porque o
//! `webauthn-rs` depende de OpenSSL, que nao existe na maquina de
//! desenvolvimento — e como dependencia obrigatoria ela impediria de compilar
//! todo o RESTO: servidor, sync, superficie. Com feature, o resto anda aqui e a
//! porta e conferida onde OpenSSL existe.
//!
//! `porta-aberta` nao autentica nada e serve para desenvolver. O binario
//! RECUSA a subir com ela ligada se o bind nao for localhost — uma porta de
//! desenvolvimento que chega em producao e como a maioria dos vazamentos
//! comeca.

use std::net::SocketAddr;

use mos_web::api;
use mos_web::avisos;
use mos_web::estado::{Estado, Hub, Porta, Push};
use mos_web::push::Vapid;
use mos_web::sync;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Um subcomando, e nao uma geracao automatica na subida: um servidor que
    // sorteia a chave VAPID ao subir e um servidor que fica mudo a cada
    // reinicio, porque as assinaturas do aparelho ficam presas a chave antiga.
    // Ela nasce uma vez, a mao, e vai para o arquivo de ambiente.
    if std::env::args().any(|arg| arg == "--gerar-vapid") {
        let (privada, publica) = Vapid::gerar();
        println!("MOS_WEB_VAPID_PRIVADA={privada}");
        println!("# a publica e derivada da privada; esta linha e so para conferencia");
        println!("# publica={publica}");
        return Ok(());
    }

    let bind = std::env::var("MOS_WEB_BIND").unwrap_or_else(|_| String::from("127.0.0.1"));
    let porta: u16 = std::env::var("MOS_WEB_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(9130);

    // A porta existe quando ha uma origem publica e um convite. As duas juntas,
    // e nao uma ou outra: sem origem o WebAuthn nao tem a que amarrar a
    // credencial, e sem convite qualquer um que achasse a URL viraria o dono.
    let porta_de_entrada = match (
        std::env::var("MOS_WEB_ORIGEM"),
        std::env::var("MOS_WEB_INVITE"),
    ) {
        (Ok(origem), Ok(convite)) if !origem.is_empty() && !convite.is_empty() => Some(Porta {
            banco: std::env::var("MOS_WEB_PORTA_DB").unwrap_or_else(|_| String::from("porta.db")),
            origem,
            convite,
        }),
        _ => None,
    };

    if let Err(motivo) = conferir_a_porta(&bind, porta_de_entrada.is_some()) {
        eprintln!(
            "
[web] RECUSADO A SUBIR

{motivo}
"
        );
        std::process::exit(1);
    }

    let banco = std::env::var("MOS_WEB_DB").unwrap_or_else(|_| String::from("mos-web.db"));
    let backups = std::env::var("MOS_WEB_BACKUPS").unwrap_or_else(|_| String::from("backups"));

    // Hub opcional: sem ele o `mos-web` funciona sozinho, e isso e util para
    // desenvolver. Em producao, um `mos-web` sem hub e um caderno separado com
    // cara de M/OS — por isso ele DIZ, alto, quando sobe assim.
    let hub = match (std::env::var("MOS_WEB_HUB"), std::env::var("MOS_WEB_TOKEN")) {
        (Ok(url), Ok(token)) if !url.is_empty() && !token.is_empty() => Some(Hub { url, token }),
        _ => {
            eprintln!(
                "[web] AVISO: sem MOS_WEB_HUB/MOS_WEB_TOKEN. Nada sai deste aparelho nem \
                 chega nele."
            );
            None
        }
    };

    // O push tambem e opcional, e pela mesma logica do hub: sem chave o
    // `mos-web` sobe igual, so que mudo — e a tela DIZ isso, em vez de mostrar
    // um botao que nao faz nada.
    let push = match std::env::var("MOS_WEB_VAPID_PRIVADA") {
        Ok(privada) if !privada.is_empty() => Some(Push {
            banco: std::env::var("MOS_WEB_PUSH_DB").unwrap_or_else(|_| String::from("push.db")),
            privada,
            contato: std::env::var("MOS_WEB_VAPID_CONTATO")
                .unwrap_or_else(|_| String::from("mailto:mos@localhost")),
        }),
        _ => None,
    };

    let estado = Estado::abrir(&banco, &backups, hub, push, porta_de_entrada)?;

    if let Some(hub) = &estado.hub {
        sync::iniciar(
            std::sync::Arc::clone(&estado.storage),
            std::sync::Arc::clone(hub),
            estado
                .push
                .as_ref()
                .map(|push| std::sync::Arc::clone(&push.avisador)),
            std::sync::Arc::clone(&estado.vez),
        );
    }

    // O laco dos lembretes. Separado do sync de proposito: um lembrete vence na
    // hora dele mesmo que a rede esteja fora, e amarrar o aviso a uma rodada de
    // sync bem-sucedida faria o celular ficar quieto justamente no dia em que a
    // VPS estivesse com problema de rede.
    if let Some(push) = &estado.push {
        avisos::iniciar(
            std::sync::Arc::clone(&push.avisador),
            std::sync::Arc::clone(&estado.attention),
        );
    }

    let rotas = api::servidor(estado);
    let endereco: SocketAddr = format!("{bind}:{porta}").parse()?;
    println!("[web] {banco}, ouvindo {endereco}");

    let ouvinte = tokio::net::TcpListener::bind(endereco).await?;
    axum::serve(ouvinte, rotas)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            println!("[web] encerrando.");
        })
        .await?;
    Ok(())
}

/// A porta interna esta LIGADA?
///
/// Nao e o mesmo que "compilada", e a distincao ja custou caro: por semanas o
/// `auth.rs` existiu, compilou com a feature `passkey` e nao estava montado em
/// rota nenhuma. Um guardiao que confiasse na feature teria dado a resposta mais
/// perigosa que existe — "estou protegido" — para um servidor aberto.
///
/// Hoje a cerimonia esta montada (`api::servidor` faz o `merge`) e o guardiao
/// esta montado (`porta::guarda`), e as duas coisas tem teste. Entao a feature
/// passou a ser resposta honesta — mas so ela: sem `MOS_WEB_INVITE` nao ha porta
/// configurada, e e por isso que quem chama passa `porta_configurada` tambem.
const PORTA_INTERNA_LIGADA: bool = cfg!(feature = "passkey");

/// Recusa combinacoes que nao deveriam existir.
///
/// # O buraco que o bind sozinho nao ve
///
/// A versao anterior perguntava so uma coisa: o bind e local? Atras de um proxy
/// reverso — que e exatamente como isto roda na VPS — o bind E local, o
/// guardiao passa, e o M/OS inteiro fica publico sem porta nenhuma. O sinal que
/// faltava nao esta no bind: esta em EXISTIR um endereco publico.
///
/// Entao `MOS_WEB_ORIGEM` passa a ser a declaracao de que este servidor e
/// alcancavel de fora. Com ela, alguma porta precisa existir: a interna
/// (compilada E configurada) ou uma externa que o operador afirma ter posto na
/// frente (`MOS_WEB_PORTA_EXTERNA=1` — Basic Auth no Caddy, mTLS, o que for).
///
/// Afirmar e o ponto. Um servidor nao consegue enxergar o proxy que esta na
/// frente dele; o que ele consegue e exigir que alguem tenha pensado no assunto
/// e escrito a resposta. "Eu configuro depois" e como toda porta aberta comeca.
fn conferir_a_porta(bind: &str, porta_configurada: bool) -> Result<(), String> {
    let local = bind == "127.0.0.1" || bind == "localhost" || bind == "::1";
    let publicado = std::env::var("MOS_WEB_ORIGEM")
        .map(|origem| !origem.trim().is_empty())
        .unwrap_or(false);
    let porta_externa = std::env::var("MOS_WEB_PORTA_EXTERNA")
        .map(|valor| valor == "1")
        .unwrap_or(false);

    // As DUAS coisas: compilada e configurada. Com a feature ligada e sem
    // `MOS_WEB_INVITE`, `Estado::abrir` nao monta sessao nenhuma, o guardiao
    // fica inerte e a API abre inteira — exatamente o caso que este guardiao
    // existe para nao deixar acontecer em silencio.
    let interna = PORTA_INTERNA_LIGADA && porta_configurada;

    if publicado && !interna && !porta_externa {
        // Linhas num array, e nao uma string com `\` no fim de cada linha: a
        // continuacao de string do Rust nao come a indentacao de forma
        // confiavel, e o resultado sai com um degrau de espacos no meio de uma
        // mensagem que precisa ser lida com pressa.
        return Err([
            "MOS_WEB_ORIGEM esta definida, entao este servidor e alcancavel de fora —",
            "e nao ha porta nenhuma na frente dele.",
            "",
            "Para a porta interna: compile com `--features passkey` e defina",
            "MOS_WEB_INVITE. Para uma externa: ponha autenticacao no proxy (Basic Auth",
            "no Caddy, por exemplo) e declare isso com MOS_WEB_PORTA_EXTERNA=1.",
            "",
            "Atras desta URL esta o seu M/OS inteiro.",
        ]
        .join(
            "
",
        ));
    }

    if publicado && porta_externa {
        eprintln!(
            "[web] publicado em modo PORTA EXTERNA: quem autentica e o proxy, e nao este binario."
        );
    }

    #[cfg(feature = "porta-aberta")]
    if !local {
        return Err(format!(
            "Compilado com `porta-aberta` e ouvindo em {bind}. Isso publica o M/OS \
             sem autenticacao nenhuma. Recompile com `--features passkey`."
        ));
    }

    #[cfg(not(any(feature = "passkey", feature = "porta-aberta")))]
    if !local {
        return Err(format!(
            "Sem porta compilada e ouvindo em {bind}. Recompile com `--features passkey`."
        ));
    }

    #[cfg(not(feature = "passkey"))]
    if local && !publicado {
        eprintln!("[web] AVISO: sem autenticacao. So vale porque o bind e local.");
    }

    Ok(())
}
