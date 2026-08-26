//! A porta: passkey para entrar, convite para poder entrar a primeira vez.
//!
//! # Por que passkey, e o que ele NAO resolve
//!
//! Atras desta URL esta o cerebro inteiro do dono, e ela vai ser publica —
//! passkey tira a senha do caminho: o Face ID do iPhone assina um desafio, e
//! nao ha nada digitavel para vazar, reusar ou phishar.
//!
//! O que ele nao resolve e **quem pode se cadastrar**. Passkey autentica quem ja
//! e conhecido; ele nao decide quem passa a ser. Sem uma trava, a primeira
//! pessoa que achasse a URL viraria a dona da casa. Por isso todo registro —
//! inclusive o primeiro — exige o `MOS_WEB_INVITE`.
//!
//! # Sessao sem criptografia propria
//!
//! O cookie carrega um token ALEATORIO e opaco, e quem resolve o que ele
//! significa e uma tabela. Assinar um cookie eu mesmo seria inventar cripto num
//! lugar onde errar custa a casa toda; um valor sorteado de 32 bytes nao tem
//! nada para forjar — ou ele esta na tabela, ou nao esta.
//!
//! O token e guardado como HASH. Um vazamento do banco de sessoes nao deve
//! entregar sessoes vivas, pela mesma razao que uma senha nao se guarda em
//! claro.

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use webauthn_rs::prelude::*;

/// Quanto tempo uma sessao vale sem uso.
///
/// Trinta dias: o celular fica no bolso e a captura precisa ser instantanea —
/// uma sessao que expira toda semana transformaria "tirar da cabeca agora" em
/// "autenticar primeiro", que e o atrito que este app existe para remover. O
/// aparelho ja e protegido pelo proprio desbloqueio.
const SESSAO_DIAS: i64 = 30;

const COOKIE: &str = "mos_web_sessao";

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("{0}")]
    Recusado(String),
    #[error("{0}")]
    Interno(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, mensagem) = match &self {
            // 401 e nao 403: o cliente pode conseguir se apresentar de novo.
            AuthError::Recusado(m) => (StatusCode::UNAUTHORIZED, m.clone()),
            AuthError::Interno(m) => {
                // A causa crua fica no servidor. Ela nomeia tabela e caminho, e
                // isso e informacao de dentro.
                eprintln!("[web] auth: {m}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    String::from("Nao foi possivel concluir agora."),
                )
            }
        };
        (status, Json(serde_json::json!({ "erro": mensagem }))).into_response()
    }
}

type Resultado<T> = Result<T, AuthError>;

fn interno(causa: impl std::fmt::Display) -> AuthError {
    AuthError::Interno(causa.to_string())
}

/// As tabelas da porta, separadas do banco de dominio.
///
/// Arquivo proprio de proposito: credencial e sessao nao sao entidades do M/OS,
/// nao sincronizam e nao devem viajar para dispositivo nenhum. Misturar as duas
/// coisas poria chave publica de passkey dentro do banco que o backup exporta e
/// o sync carrega.
pub fn preparar(conexao: &Connection) -> Result<(), rusqlite::Error> {
    conexao.execute_batch(
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

        -- O estado intermediario do WebAuthn: o desafio que o navegador tem que
        -- assinar. Vive no servidor, e nao no cliente, porque quem verifica a
        -- resposta precisa ter guardado a pergunta — devolver o desafio para o
        -- cliente guardar seria deixa-lo escolher a pergunta.
        CREATE TABLE IF NOT EXISTS desafios (
            id        TEXT PRIMARY KEY NOT NULL,
            estado    TEXT NOT NULL,
            criado_em TEXT NOT NULL
        );
        "#,
    )
}

pub struct Porta {
    pub webauthn: Webauthn,
    pub convite: String,
}

impl Porta {
    /// `origem` e a URL exata que o navegador ve, com esquema e porta.
    ///
    /// O WebAuthn amarra a credencial a ela: uma passkey criada em
    /// `https://mos.exemplo` nao funciona em `https://outro.exemplo`, e e isso
    /// que torna phishing impossivel. O preco e que trocar de dominio invalida
    /// as credenciais — por isso o endereco e configuracao, e nao um palpite.
    pub fn nova(origem: &str, convite: String) -> Result<Self, String> {
        let url = Url::parse(origem).map_err(|causa| format!("Origem invalida: {causa}"))?;
        let rp_id = url
            .host_str()
            .ok_or_else(|| String::from("A origem precisa ter host."))?
            .to_owned();
        let webauthn = WebauthnBuilder::new(&rp_id, &url)
            .map_err(|causa| format!("WebAuthn: {causa}"))?
            .rp_name("M/OS")
            .build()
            .map_err(|causa| format!("WebAuthn: {causa}"))?;
        Ok(Self { webauthn, convite })
    }
}

fn agora() -> time::OffsetDateTime {
    time::OffsetDateTime::now_utc()
}

fn iso(momento: time::OffsetDateTime) -> String {
    momento
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

/// Bytes aleatorios do sistema. Sessao sorteada com gerador previsivel e sessao
/// adivinhavel.
fn sorteio(bytes: usize) -> Resultado<String> {
    let mut buffer = vec![0u8; bytes];
    getrandom::fill(&mut buffer).map_err(|causa| interno(format!("sem entropia: {causa}")))?;
    Ok(buffer.iter().map(|b| format!("{b:02x}")).collect())
}

/// SHA-256 do token, em hex.
///
/// O token do cookie e guardado assim, e nunca em claro: um vazamento da tabela
/// de sessoes nao deve entregar sessoes vivas, pela mesma razao que uma senha
/// nao se guarda legivel.
fn hash(token: &str) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;
    let digest: [u8; 32] = Sha256::digest(token.as_bytes()).into();
    digest.iter().fold(String::new(), |mut saida, byte| {
        let _ = write!(saida, "{byte:02x}");
        saida
    })
}

// ------------------------------------------------------------------- rotas

#[derive(Deserialize)]
pub struct InicioRegistro {
    /// O convite. Todo registro exige, inclusive o primeiro.
    pub convite: String,
    /// Como este aparelho vai se chamar na lista. "iPhone", "PC de casa".
    pub apelido: String,
}

#[derive(Serialize)]
pub struct DesafioResposta {
    pub desafio: String,
    pub opcoes: serde_json::Value,
}

/// Compara em tempo constante. Comparar segredo com `==` vaza o tamanho do
/// prefixo certo pelo tempo de resposta.
fn confere_convite(recebido: &str, esperado: &str) -> bool {
    let (a, b) = (recebido.as_bytes(), esperado.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub struct Estado {
    pub porta: Arc<Porta>,
    pub banco: Arc<std::sync::Mutex<Connection>>,
}

impl Clone for Estado {
    fn clone(&self) -> Self {
        Self {
            porta: Arc::clone(&self.porta),
            banco: Arc::clone(&self.banco),
        }
    }
}

pub async fn registro_inicio(
    State(estado): State<Estado>,
    Json(pedido): Json<InicioRegistro>,
) -> Resultado<Json<DesafioResposta>> {
    if !confere_convite(&pedido.convite, &estado.porta.convite) {
        return Err(AuthError::Recusado("Convite invalido.".into()));
    }
    let apelido = pedido.apelido.trim();
    if apelido.is_empty() {
        return Err(AuthError::Recusado("Diga como chamar este aparelho.".into()));
    }

    let banco = estado.banco.lock().map_err(|_| interno("banco ocupado"))?;
    // Um usuario so: o M/OS tem um dono. O id e estavel para que uma passkey
    // nova entre como MAIS UM aparelho do mesmo dono, e nao como outra pessoa.
    let usuario = usuario_unico(&banco)?;
    let ja_registradas = credenciais_existentes(&banco)?;

    let (opcoes, estado_registro) = estado
        .porta
        .webauthn
        .start_passkey_registration(usuario, "m-os", "M/OS", Some(ja_registradas))
        .map_err(interno)?;

    let desafio = guardar_desafio(&banco, &estado_registro)?;
    Ok(Json(DesafioResposta {
        desafio,
        opcoes: serde_json::to_value(opcoes).map_err(interno)?,
    }))
}

#[derive(Deserialize)]
pub struct FimRegistro {
    pub desafio: String,
    pub apelido: String,
    pub credencial: RegisterPublicKeyCredential,
}

pub async fn registro_fim(
    State(estado): State<Estado>,
    Json(pedido): Json<FimRegistro>,
) -> Resultado<Json<serde_json::Value>> {
    let banco = estado.banco.lock().map_err(|_| interno("banco ocupado"))?;
    let estado_registro: PasskeyRegistration = tomar_desafio(&banco, &pedido.desafio)?;
    let passkey = estado
        .porta
        .webauthn
        .finish_passkey_registration(&pedido.credencial, &estado_registro)
        .map_err(|causa| AuthError::Recusado(format!("Passkey recusada: {causa}")))?;

    banco
        .execute(
            "INSERT INTO credenciais (id, apelido, passkey, criada_em) VALUES (?1, ?2, ?3, ?4)",
            params![
                uuid::Uuid::now_v7().to_string(),
                pedido.apelido.trim(),
                serde_json::to_string(&passkey).map_err(interno)?,
                iso(agora()),
            ],
        )
        .map_err(interno)?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn login_inicio(
    State(estado): State<Estado>,
) -> Resultado<Json<DesafioResposta>> {
    let banco = estado.banco.lock().map_err(|_| interno("banco ocupado"))?;
    let passkeys = todas_as_passkeys(&banco)?;
    if passkeys.is_empty() {
        return Err(AuthError::Recusado(
            "Nenhum aparelho registrado ainda.".into(),
        ));
    }
    let (opcoes, estado_login) = estado
        .porta
        .webauthn
        .start_passkey_authentication(&passkeys)
        .map_err(interno)?;
    let desafio = guardar_desafio(&banco, &estado_login)?;
    Ok(Json(DesafioResposta {
        desafio,
        opcoes: serde_json::to_value(opcoes).map_err(interno)?,
    }))
}

#[derive(Deserialize)]
pub struct FimLogin {
    pub desafio: String,
    pub credencial: PublicKeyCredential,
}

pub async fn login_fim(
    State(estado): State<Estado>,
    jar: CookieJar,
    Json(pedido): Json<FimLogin>,
) -> Resultado<(CookieJar, Json<serde_json::Value>)> {
    let banco = estado.banco.lock().map_err(|_| interno("banco ocupado"))?;
    let estado_login: PasskeyAuthentication = tomar_desafio(&banco, &pedido.desafio)?;
    let autenticado = estado
        .porta
        .webauthn
        .finish_passkey_authentication(&pedido.credencial, &estado_login)
        .map_err(|causa| AuthError::Recusado(format!("Nao reconheci: {causa}")))?;

    let credencial_id = format!("{:?}", autenticado.cred_id());
    let token = sorteio(32)?;
    let expira = agora() + time::Duration::days(SESSAO_DIAS);
    banco
        .execute(
            "INSERT INTO sessoes (hash, credencial, criada_em, expira_em) VALUES (?1, ?2, ?3, ?4)",
            params![hash(&token), credencial_id, iso(agora()), iso(expira)],
        )
        .map_err(interno)?;

    // `Secure` + `HttpOnly` + `SameSite=Strict`: o cookie nunca sai em HTTP, o
    // JavaScript da pagina nao o le (entao um XSS nao o rouba), e ele nao
    // acompanha requisicao vinda de outro site.
    let cookie = Cookie::build((COOKIE, token))
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .max_age(time::Duration::days(SESSAO_DIAS))
        .build();

    Ok((jar.add(cookie), Json(serde_json::json!({ "ok": true }))))
}

/// Quem esta pedindo, se e que alguem esta.
pub fn sessao_valida(banco: &Connection, jar: &CookieJar) -> bool {
    let Some(token) = jar.get(COOKIE).map(|c| c.value().to_owned()) else {
        return false;
    };
    let encontrada: Option<String> = banco
        .query_row(
            "SELECT expira_em FROM sessoes WHERE hash = ?1",
            params![hash(&token)],
            |linha| linha.get(0),
        )
        .optional()
        .ok()
        .flatten();
    let Some(expira) = encontrada else {
        return false;
    };
    time::OffsetDateTime::parse(&expira, &time::format_description::well_known::Rfc3339)
        .map(|quando| quando > agora())
        .unwrap_or(false)
}

// ------------------------------------------------------------------ apoio

fn usuario_unico(banco: &Connection) -> Resultado<Uuid> {
    // Um dono, um id, estavel entre registros: uma passkey nova precisa entrar
    // como mais um aparelho do MESMO dono. Id sorteado a cada registro faria o
    // segundo aparelho virar outra pessoa.
    let existente: Option<String> = banco
        .query_row(
            "SELECT passkey FROM credenciais LIMIT 1",
            [],
            |linha| linha.get(0),
        )
        .optional()
        .map_err(interno)?;
    match existente {
        Some(json) => {
            let passkey: Passkey = serde_json::from_str(&json).map_err(interno)?;
            Ok(Uuid::from_bytes(*passkey.cred_id().as_ref().first_chunk::<16>().unwrap_or(&[0; 16])))
        }
        None => Ok(Uuid::now_v7()),
    }
}

fn credenciais_existentes(banco: &Connection) -> Resultado<Vec<CredentialID>> {
    Ok(todas_as_passkeys(banco)?
        .iter()
        .map(|p| p.cred_id().clone())
        .collect())
}

fn todas_as_passkeys(banco: &Connection) -> Resultado<Vec<Passkey>> {
    let mut consulta = banco
        .prepare("SELECT passkey FROM credenciais")
        .map_err(interno)?;
    let linhas = consulta
        .query_map([], |linha| linha.get::<_, String>(0))
        .map_err(interno)?;
    let mut saida = Vec::new();
    for linha in linhas {
        let json = linha.map_err(interno)?;
        saida.push(serde_json::from_str(&json).map_err(interno)?);
    }
    Ok(saida)
}

fn guardar_desafio<T: Serialize>(banco: &Connection, estado: &T) -> Resultado<String> {
    let id = sorteio(16)?;
    banco
        .execute(
            "INSERT INTO desafios (id, estado, criado_em) VALUES (?1, ?2, ?3)",
            params![
                id,
                serde_json::to_string(estado).map_err(interno)?,
                iso(agora())
            ],
        )
        .map_err(interno)?;
    Ok(id)
}

/// Le o desafio e o APAGA.
///
/// Uso unico: um desafio reaproveitavel deixaria uma resposta capturada valer
/// duas vezes, que e exatamente o replay que o desafio existe para impedir.
fn tomar_desafio<T: for<'de> Deserialize<'de>>(banco: &Connection, id: &str) -> Resultado<T> {
    let json: Option<String> = banco
        .query_row(
            "SELECT estado FROM desafios WHERE id = ?1",
            params![id],
            |linha| linha.get(0),
        )
        .optional()
        .map_err(interno)?;
    let json = json.ok_or_else(|| AuthError::Recusado("Desafio expirado.".into()))?;
    banco
        .execute("DELETE FROM desafios WHERE id = ?1", params![id])
        .map_err(interno)?;
    serde_json::from_str(&json).map_err(interno)
}
