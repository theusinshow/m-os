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
//! # Onde este arquivo termina
//!
//! Ele prova que o dono e o dono, e entrega o resultado para o `porta.rs`, que
//! cria a sessao e o cookie. A divisao existe porque a metade que decide QUEM
//! PASSA precisa ser testavel onde este arquivo nao compila — ver o topo do
//! `porta.rs`.

use std::sync::{Arc, Mutex};

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use axum_extra::extract::cookie::CookieJar;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use webauthn_rs::prelude::*;

use crate::porta::{self, Sessoes};

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

/// As tabelas moram no `porta.rs`, que nao depende de OpenSSL.
///
/// A divisao nao e arrumacao: a metade que decide QUEM PASSA tem que ser
/// testavel onde este arquivo nao compila. Ver o topo de `porta.rs`.

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
    pub sessoes: Arc<Sessoes>,
    pub banco: Arc<Mutex<Connection>>,
}

impl Clone for Estado {
    fn clone(&self) -> Self {
        Self {
            porta: Arc::clone(&self.porta),
            sessoes: Arc::clone(&self.sessoes),
            banco: Arc::clone(&self.banco),
        }
    }
}

/// A cerimonia, como rotas.
///
/// Todas sob `/api/porta/`, que e o unico prefixo que o guardiao do `porta.rs`
/// deixa passar sem sessao — e tem que ser assim: quem ainda nao entrou nao tem
/// como pedir para entrar por uma rota que exige ter entrado.
pub fn rotas(porta: Arc<Porta>, sessoes: Arc<Sessoes>) -> Router {
    let estado = Estado {
        porta,
        banco: sessoes.conexao(),
        sessoes,
    };
    Router::new()
        .route("/api/porta/registro/inicio", post(registro_inicio))
        .route("/api/porta/registro/fim", post(registro_fim))
        .route("/api/porta/login/inicio", post(login_inicio))
        .route("/api/porta/login/fim", post(login_fim))
        .with_state(estado)
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
        return Err(AuthError::Recusado(
            "Diga como chamar este aparelho.".into(),
        ));
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

/// Registrar TAMBEM abre a sessao.
///
/// A versao anterior so guardava a credencial, e a tela chamava o login em
/// seguida — duas cerimonias, dois Face ID, e a segunda sem gesto do usuario
/// atras dela. No iOS isso e um `NotAllowedError` calado: o Safari exige que a
/// chamada do WebAuthn saia junto com o toque, e uma ida ao servidor no meio
/// gasta essa permissao.
///
/// Alem disso a segunda cerimonia era redundante por definicao: quem acabou de
/// provar que tem a chave nao precisa provar de novo dois segundos depois.
pub async fn registro_fim(
    State(estado): State<Estado>,
    jar: CookieJar,
    Json(pedido): Json<FimRegistro>,
) -> Resultado<(CookieJar, Json<serde_json::Value>)> {
    let banco = estado.banco.lock().map_err(|_| interno("banco ocupado"))?;
    let estado_registro: PasskeyRegistration = tomar_desafio(&banco, &pedido.desafio)?;
    let passkey = estado
        .porta
        .webauthn
        .finish_passkey_registration(&pedido.credencial, &estado_registro)
        .map_err(|causa| AuthError::Recusado(format!("Passkey recusada: {causa}")))?;

    let credencial_id = uuid::Uuid::now_v7().to_string();
    banco
        .execute(
            "INSERT INTO credenciais (id, apelido, passkey, criada_em) VALUES (?1, ?2, ?3, ?4)",
            params![
                credencial_id,
                pedido.apelido.trim(),
                serde_json::to_string(&passkey).map_err(interno)?,
                iso(agora()),
            ],
        )
        .map_err(interno)?;

    // Solto ANTES: `criar` toma o mesmo mutex.
    drop(banco);
    let token = estado
        .sessoes
        .criar(&credencial_id, agora())
        .map_err(interno)?;

    Ok((
        jar.add(porta::cookie_de(token)),
        Json(serde_json::json!({ "ok": true })),
    ))
}

pub async fn login_inicio(State(estado): State<Estado>) -> Resultado<Json<DesafioResposta>> {
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
    // A sessao e o cookie sao do `porta.rs`. Este arquivo prova quem e o dono; o
    // que acontece depois disso e a mesma coisa para qualquer forma de provar.
    //
    // O `banco` e solto ANTES: `criar` toma o mesmo mutex, e mante-lo aqui
    // travaria o processo contra si mesmo.
    drop(banco);
    let token = estado
        .sessoes
        .criar(&credencial_id, agora())
        .map_err(interno)?;

    Ok((
        jar.add(porta::cookie_de(token)),
        Json(serde_json::json!({ "ok": true })),
    ))
}

// ------------------------------------------------------------------ apoio

fn usuario_unico(banco: &Connection) -> Resultado<Uuid> {
    // Um dono, um id, estavel entre registros: uma passkey nova precisa entrar
    // como mais um aparelho do MESMO dono. Id sorteado a cada registro faria o
    // segundo aparelho virar outra pessoa.
    let existente: Option<String> = banco
        .query_row("SELECT passkey FROM credenciais LIMIT 1", [], |linha| {
            linha.get(0)
        })
        .optional()
        .map_err(interno)?;
    match existente {
        Some(json) => {
            let passkey: Passkey = serde_json::from_str(&json).map_err(interno)?;
            Ok(Uuid::from_bytes(
                *passkey
                    .cred_id()
                    .as_ref()
                    .first_chunk::<16>()
                    .unwrap_or(&[0; 16]),
            ))
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
    let id = porta::sorteio(16).map_err(interno)?;
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
