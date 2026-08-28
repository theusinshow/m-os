//! Quem entra: a sessão, o cookie e o guardião das rotas.
//!
//! # Por que este módulo NÃO está atrás da feature `passkey`
//!
//! Porque ele é a metade que decide quem passa, e essa metade não pode ser
//! verificável só onde OpenSSL existe.
//!
//! O `auth.rs` — a cerimônia WebAuthn — depende do `webauthn-rs`, que depende de
//! OpenSSL, que não existe na máquina de desenvolvimento. Enquanto ela era o
//! módulo inteiro, a pergunta que mais importa — *uma requisição sem sessão é
//! recusada?* — só podia ser respondida no CI. E não foi: o `auth.rs` ficou
//! escrito, compilando no CI, e **não montado em rota nenhuma** por semanas. Um
//! `cargo check` verde não sabe a diferença entre um guardião montado e um
//! guardião esquecido numa gaveta.
//!
//! Aqui não há WebAuthn. Há tabela, cookie e um `middleware` — e um teste que
//! bate na rota sem cookie e exige 401. Isso roda no Windows, em dois segundos.
//!
//! # O que a cerimônia entrega para cá
//!
//! O `auth.rs` prova que o dono é o dono, e chama [`Sessoes::criar`]. Deste
//! ponto em diante, quem manda é o cookie — e o cookie é um valor **sorteado e
//! opaco**, guardado como hash.
//!
//! Assinar um cookie eu mesmo seria inventar criptografia num lugar onde errar
//! custa a casa inteira. Um valor sorteado de 32 bytes não tem nada para forjar:
//! ou ele está na tabela, ou não está. E guardar o hash em vez do valor é a
//! mesma disciplina de uma senha — um vazamento da tabela não deve entregar
//! sessões vivas.

use std::sync::{Arc, Mutex};

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use rusqlite::{params, Connection, OptionalExtension};

/// Quanto tempo uma sessão vale.
///
/// Trinta dias: o celular fica no bolso e a captura precisa ser instantânea —
/// uma sessão que expira toda semana transformaria "tirar da cabeça agora" em
/// "autenticar primeiro", que é o atrito que esta superfície existe para
/// remover. O aparelho já é protegido pelo próprio desbloqueio.
pub const SESSAO_DIAS: i64 = 30;

pub const COOKIE: &str = "mos_web_sessao";

/// As rotas que o guardião deixa passar sem sessão.
///
/// Só a própria porta. Se esta lista crescer, cada item novo é uma rota pública
/// a mais — e vale escrever aqui por que ela precisa ser.
const LIVRES: &[&str] = &["/api/porta/"];

#[derive(Debug, thiserror::Error)]
#[error("porta.db: {0}")]
pub struct PortaError(String);

fn erro(causa: rusqlite::Error) -> PortaError {
    PortaError(causa.to_string())
}

/// O banco da porta.
///
/// Arquivo próprio, como o do push e pela mesma razão: credencial e sessão não
/// são entidades do M/OS, não sincronizam e não devem viajar para dispositivo
/// nenhum. Misturá-las ao banco de domínio poria chave pública de passkey dentro
/// do que o backup exporta e o sync carrega.
pub struct Sessoes {
    conexao: Arc<Mutex<Connection>>,
}

impl Sessoes {
    pub fn abrir(caminho: &str) -> Result<Self, PortaError> {
        let conexao = Connection::open(caminho).map_err(erro)?;
        conexao
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS credenciais (
                    id          TEXT PRIMARY KEY NOT NULL,
                    apelido     TEXT NOT NULL,
                    passkey     TEXT NOT NULL,
                    criada_em   TEXT NOT NULL,
                    usada_em    TEXT
                );

                CREATE TABLE IF NOT EXISTS sessoes (
                    -- O hash do token, nunca o token.
                    hash        TEXT PRIMARY KEY NOT NULL,
                    credencial  TEXT NOT NULL,
                    criada_em   TEXT NOT NULL,
                    expira_em   TEXT NOT NULL
                );

                -- O desafio que o navegador tem que assinar. Vive no servidor
                -- porque quem verifica a resposta precisa ter guardado a
                -- pergunta: devolver o desafio para o cliente guardar seria
                -- deixá-lo escolher a pergunta.
                CREATE TABLE IF NOT EXISTS desafios (
                    id        TEXT PRIMARY KEY NOT NULL,
                    estado    TEXT NOT NULL,
                    criado_em TEXT NOT NULL
                );
                "#,
            )
            .map_err(erro)?;
        Ok(Self {
            conexao: Arc::new(Mutex::new(conexao)),
        })
    }

    /// A conexão, para a cerimônia do `auth.rs` mexer nas credenciais.
    pub fn conexao(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.conexao)
    }

    /// Algum aparelho já foi registrado?
    ///
    /// A tela pergunta isto ANTES de qualquer coisa: sem nenhuma credencial, o
    /// que ela mostra é "registrar este aparelho" e não "entrar" — e um botão de
    /// entrar que não tem com o que comparar falha com uma mensagem que não
    /// explica nada.
    pub fn ha_credencial(&self) -> Result<bool, PortaError> {
        let conexao = self.conexao.lock().expect("mutex da porta");
        let achou: Option<i64> = conexao
            .query_row("SELECT 1 FROM credenciais LIMIT 1", [], |linha| {
                linha.get(0)
            })
            .optional()
            .map_err(erro)?;
        Ok(achou.is_some())
    }

    /// Abre uma sessão e devolve o token que vai no cookie.
    ///
    /// Só o `auth.rs` chama, e só depois de a passkey ter sido verificada.
    pub fn criar(
        &self,
        credencial: &str,
        agora: time::OffsetDateTime,
    ) -> Result<String, PortaError> {
        let token = sorteio(32)?;
        let expira = agora + time::Duration::days(SESSAO_DIAS);
        self.conexao
            .lock()
            .expect("mutex da porta")
            .execute(
                "INSERT INTO sessoes (hash, credencial, criada_em, expira_em) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![hash(&token), credencial, iso(agora), iso(expira)],
            )
            .map_err(erro)?;
        Ok(token)
    }

    /// A sessão do cookie vale agora?
    pub fn valida(&self, jar: &CookieJar, agora: time::OffsetDateTime) -> bool {
        let Some(token) = jar.get(COOKIE).map(|c| c.value().to_owned()) else {
            return false;
        };
        let conexao = self.conexao.lock().expect("mutex da porta");
        let expira: Option<String> = conexao
            .query_row(
                "SELECT expira_em FROM sessoes WHERE hash = ?1",
                params![hash(&token)],
                |linha| linha.get(0),
            )
            .optional()
            .ok()
            .flatten();
        let Some(expira) = expira else {
            return false;
        };
        time::OffsetDateTime::parse(&expira, &time::format_description::well_known::Rfc3339)
            .map(|quando| quando > agora)
            .unwrap_or(false)
    }

    /// Apaga a sessão deste cookie. É o "sair".
    pub fn encerrar(&self, jar: &CookieJar) -> Result<(), PortaError> {
        if let Some(token) = jar.get(COOKIE).map(|c| c.value().to_owned()) {
            self.conexao
                .lock()
                .expect("mutex da porta")
                .execute("DELETE FROM sessoes WHERE hash = ?1", params![hash(&token)])
                .map_err(erro)?;
        }
        Ok(())
    }

    /// Varre o que já venceu.
    pub fn limpar_expiradas(&self, agora: time::OffsetDateTime) -> Result<usize, PortaError> {
        self.conexao
            .lock()
            .expect("mutex da porta")
            .execute(
                "DELETE FROM sessoes WHERE expira_em < ?1",
                params![iso(agora)],
            )
            .map_err(erro)
    }
}

/// O cookie da sessão.
///
/// `Secure` + `HttpOnly` + `SameSite=Strict`: ele nunca sai em HTTP, o
/// JavaScript da página não o lê (então um XSS não o rouba), e não acompanha
/// requisição vinda de outro site.
pub fn cookie_de(token: String) -> Cookie<'static> {
    Cookie::build((COOKIE, token))
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .max_age(time::Duration::days(SESSAO_DIAS))
        .build()
}

/// O cookie que apaga o cookie.
pub fn cookie_vazio() -> Cookie<'static> {
    Cookie::build((COOKIE, ""))
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .max_age(time::Duration::seconds(0))
        .build()
}

/// O guardião.
///
/// # Por que ele decide pelo CAMINHO, e não por estar montado num sub-router
///
/// Porque um sub-router protegido é uma decisão que se perde: alguém acrescenta
/// uma rota no lugar errado e ela nasce pública, sem nada indicando isso. Aqui a
/// regra é uma só, escrita num lugar só — **tudo sob `/api` exige sessão, menos
/// o que está em [`LIVRES`]** —, e uma rota nova nasce protegida por omissão.
///
/// O que NÃO passa por aqui é a página: ela precisa carregar para poder mostrar
/// a tela de entrar. Ela não expõe dado nenhum — o dado está atrás da API.
pub async fn guarda(
    State(sessoes): State<Option<Arc<Sessoes>>>,
    jar: CookieJar,
    pedido: Request,
    proximo: Next,
) -> Response {
    let caminho = pedido.uri().path().to_owned();

    // Sem porta configurada, o `mos-web` é o de desenvolvimento em localhost —
    // e `conferir_a_porta`, no `main.rs`, já recusou subir assim publicado.
    let Some(sessoes) = sessoes else {
        return proximo.run(pedido).await;
    };

    let protegida =
        caminho.starts_with("/api/") && !LIVRES.iter().any(|livre| caminho.starts_with(livre));

    if protegida && !sessoes.valida(&jar, time::OffsetDateTime::now_utc()) {
        // 401 e não 403: o cliente pode se apresentar. A tela lê este status
        // para trocar o app pela tela de entrar.
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "erro": "Entre para continuar." })),
        )
            .into_response();
    }

    proximo.run(pedido).await
}

// -------------------------------------------------------------------- apoio

fn iso(momento: time::OffsetDateTime) -> String {
    momento
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

/// Bytes aleatórios do sistema. Sessão sorteada com gerador previsível é sessão
/// adivinhável.
pub fn sorteio(bytes: usize) -> Result<String, PortaError> {
    let mut buffer = vec![0u8; bytes];
    getrandom::fill(&mut buffer).map_err(|causa| PortaError(format!("sem entropia: {causa}")))?;
    Ok(buffer.iter().map(|b| format!("{b:02x}")).collect())
}

/// SHA-256 em hex.
pub fn hash(token: &str) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;
    let digest: [u8; 32] = Sha256::digest(token.as_bytes()).into();
    digest.iter().fold(String::new(), |mut saida, byte| {
        let _ = write!(saida, "{byte:02x}");
        saida
    })
}

#[cfg(test)]
mod testes {
    use super::*;

    fn instante(segundos: i64) -> time::OffsetDateTime {
        time::OffsetDateTime::from_unix_timestamp(segundos).unwrap()
    }

    fn jar_com(token: &str) -> CookieJar {
        CookieJar::new().add(Cookie::new(COOKIE, token.to_owned()))
    }

    #[test]
    fn uma_sessao_recem_criada_vale() {
        let sessoes = Sessoes::abrir(":memory:").unwrap();
        let token = sessoes.criar("cred-1", instante(1_000_000)).unwrap();
        assert!(sessoes.valida(&jar_com(&token), instante(1_000_100)));
    }

    #[test]
    fn sem_cookie_nao_vale() {
        let sessoes = Sessoes::abrir(":memory:").unwrap();
        sessoes.criar("cred-1", instante(1_000_000)).unwrap();
        assert!(!sessoes.valida(&CookieJar::new(), instante(1_000_100)));
    }

    /// Um token inventado não pode valer. É o ataque mais óbvio que existe
    /// contra um cookie opaco, e o teste que prova que ele não funciona.
    #[test]
    fn um_token_inventado_nao_vale() {
        let sessoes = Sessoes::abrir(":memory:").unwrap();
        sessoes.criar("cred-1", instante(1_000_000)).unwrap();
        assert!(!sessoes.valida(&jar_com("deadbeef"), instante(1_000_100)));
    }

    #[test]
    fn depois_de_trinta_dias_nao_vale_mais() {
        let sessoes = Sessoes::abrir(":memory:").unwrap();
        let token = sessoes.criar("cred-1", instante(0)).unwrap();
        let um_dia = 24 * 60 * 60;
        assert!(sessoes.valida(&jar_com(&token), instante(29 * um_dia)));
        assert!(!sessoes.valida(&jar_com(&token), instante(31 * um_dia)));
    }

    /// O token não pode estar legível na tabela: um vazamento do arquivo não
    /// deve entregar sessões vivas.
    #[test]
    fn o_token_nao_e_guardado_em_claro() {
        let sessoes = Sessoes::abrir(":memory:").unwrap();
        let token = sessoes.criar("cred-1", instante(1_000_000)).unwrap();
        let conexao = sessoes.conexao();
        let guardado: String = conexao
            .lock()
            .unwrap()
            .query_row("SELECT hash FROM sessoes", [], |l| l.get(0))
            .unwrap();
        assert_ne!(guardado, token);
        assert_eq!(guardado, hash(&token));
    }

    #[test]
    fn encerrar_invalida_na_hora() {
        let sessoes = Sessoes::abrir(":memory:").unwrap();
        let token = sessoes.criar("cred-1", instante(1_000_000)).unwrap();
        let jar = jar_com(&token);
        sessoes.encerrar(&jar).unwrap();
        assert!(!sessoes.valida(&jar, instante(1_000_100)));
    }

    #[test]
    fn a_faxina_leva_so_o_que_venceu() {
        let sessoes = Sessoes::abrir(":memory:").unwrap();
        let velho = sessoes.criar("cred-1", instante(0)).unwrap();
        let novo = sessoes
            .criar("cred-2", instante(60 * 24 * 60 * 60))
            .unwrap();
        let agora = instante(45 * 24 * 60 * 60);
        assert_eq!(sessoes.limpar_expiradas(agora).unwrap(), 1);
        assert!(!sessoes.valida(&jar_com(&velho), agora));
        assert!(sessoes.valida(&jar_com(&novo), agora));
    }

    #[test]
    fn o_cookie_da_sessao_e_secure_httponly_e_strict() {
        let cookie = cookie_de(String::from("abc"));
        assert!(cookie.secure().unwrap_or(false), "Secure");
        assert!(cookie.http_only().unwrap_or(false), "HttpOnly");
        assert_eq!(cookie.same_site(), Some(SameSite::Strict));
    }

    #[test]
    fn sem_credencial_registrada_a_tela_sabe() {
        let sessoes = Sessoes::abrir(":memory:").unwrap();
        assert!(!sessoes.ha_credencial().unwrap());
        sessoes
            .conexao()
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO credenciais (id, apelido, passkey, criada_em) \
                 VALUES ('1', 'iPhone', '{}', '2026-01-01T00:00:00Z')",
                [],
            )
            .unwrap();
        assert!(sessoes.ha_credencial().unwrap());
    }
}
