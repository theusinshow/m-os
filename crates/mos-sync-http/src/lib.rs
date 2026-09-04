//! O `Transport` sobre HTTP: o outro lado do hub, visto de dentro do M/OS.
//!
//! # Por que bloqueante
//!
//! `Transport::push` e `Transport::pull` sao funcoes sincronas, e isso e
//! decisao do motor: `sincronizar()` e um laco sequencial que empurra, puxa e
//! reconcilia, e tornar isso assincrono contaminaria `mos-sync` inteiro com um
//! runtime que o iPhone teria que carregar junto.
//!
//! A consequencia e uma regra de uso, e ela nao e negociavel:
//!
//! > **Chame `sincronizar()` fora do runtime assincrono.** No Tauri, dentro de
//! > `spawn_blocking`. Chamar `reqwest::blocking` de dentro de um worker do
//! > tokio derruba o processo com "cannot block the current thread from within
//! > a runtime" — e derruba na hora, nao intermitentemente.
//!
//! # O que viaja
//!
//! Exatamente o que o `Transport` define, e nada a mais. Nenhum cabecalho
//! carrega identidade de dispositivo: quem e o dispositivo ja esta dentro do
//! HLC de cada operacao, e repetir isso no envelope criaria uma segunda fonte
//! da verdade para a mesma pergunta.

use std::time::Duration;

use mos_sync::{Lote, Op, Resultado, SyncError, Transport};
use serde::{Deserialize, Serialize};

/// Quanto tempo esperar antes de desistir de uma rodada.
///
/// Trinta segundos, e nao cinco: a primeira sincronizacao de um dispositivo
/// novo puxa lotes cheios, e um teto curto transformaria "sincronizacao inicial
/// lenta" em "sincronizacao inicial impossivel". O motor ja e resiliente a
/// falha — o que ele nao contorna e desistir cedo demais toda vez.
const TIMEOUT: Duration = Duration::from_secs(30);

pub struct HttpTransport {
    base: String,
    token: String,
    cliente: reqwest::blocking::Client,
}

impl HttpTransport {
    pub fn novo(base: impl Into<String>, token: impl Into<String>) -> Resultado<Self> {
        let cliente = reqwest::blocking::Client::builder()
            .timeout(TIMEOUT)
            .build()
            .map_err(|causa| SyncError::novo(format!("cliente HTTP: {causa}"), false))?;
        Ok(Self {
            // Sem barra no fim, sempre: `base` + "/sync/push" com barra dupla
            // funciona na maioria dos servidores e falha em alguns proxies, o
            // que vira um bug que so aparece em producao.
            base: base.into().trim_end_matches('/').to_owned(),
            token: token.into(),
            cliente,
        })
    }
}

/// O erro como o servidor conta.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ErroJson {
    mensagem: String,
    retriavel: bool,
}

/// Traduz a resposta em erro do motor, preservando `retriavel`.
///
/// Essa distincao e o que separa backoff de desistencia. Uma falha de rede
/// passa numa nova tentativa; credencial errada e contrato incompativel nunca
/// vao passar por insistencia — e insistir neles, no celular, e bateria gasta
/// para receber o mesmo nao.
fn erro_da_resposta(resposta: reqwest::blocking::Response) -> SyncError {
    let status = resposta.status();
    let retriavel_por_status =
        status.is_server_error() || status == reqwest::StatusCode::REQUEST_TIMEOUT;
    match resposta.json::<ErroJson>() {
        Ok(corpo) => SyncError::novo(corpo.mensagem, corpo.retriavel),
        // Sem corpo entendivel: o status ainda diz o suficiente para escolher
        // entre tentar de novo e parar.
        Err(_) => SyncError::novo(format!("O hub respondeu {status}."), retriavel_por_status),
    }
}

/// Falha de transporte: DNS, recusa de conexao, timeout, TLS.
///
/// Sempre retriavel. O motor conta a tentativa por operacao, e e essa contagem
/// que alimenta o backoff e o "esta travada ha quanto tempo" do diagnostico.
fn erro_de_rede(causa: reqwest::Error) -> SyncError {
    SyncError::novo(format!("Sem alcancar o hub: {causa}"), true)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PushResposta {
    aceitas: Vec<uuid::Uuid>,
}

impl Transport for HttpTransport {
    fn push(&self, contrato: u32, ops: &[Op]) -> Resultado<Vec<uuid::Uuid>> {
        // Lote vazio nao vira chamada. O motor ja evita, mas quem transporta
        // nao deve depender de quem chama para nao gastar uma ida a rede — no
        // celular, uma ida a toa e radio ligado a toa.
        if ops.is_empty() {
            return Ok(Vec::new());
        }

        let resposta = self
            .cliente
            .post(format!("{}/sync/push", self.base))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({ "contrato": contrato, "ops": ops }))
            .send()
            .map_err(erro_de_rede)?;

        if !resposta.status().is_success() {
            return Err(erro_da_resposta(resposta));
        }

        let corpo: PushResposta = resposta.json().map_err(|causa| {
            // Resposta ilegivel nao e retriavel: o servidor respondeu, e
            // respondeu algo que este cliente nao entende. Insistir daria o
            // mesmo texto.
            SyncError::novo(format!("Resposta do hub ilegivel: {causa}"), false)
        })?;
        Ok(corpo.aceitas)
    }

    fn pull(&self, contrato: u32, cursor: &str, limite: usize) -> Resultado<Lote> {
        let resposta = self
            .cliente
            .get(format!("{}/sync/pull", self.base))
            .bearer_auth(&self.token)
            .query(&[
                ("contrato", contrato.to_string()),
                ("cursor", cursor.to_owned()),
                ("limite", limite.to_string()),
            ])
            .send()
            .map_err(erro_de_rede)?;

        if !resposta.status().is_success() {
            return Err(erro_da_resposta(resposta));
        }

        resposta
            .json::<Lote>()
            .map_err(|causa| SyncError::novo(format!("Lote do hub ilegivel: {causa}"), false))
    }
}

// --------------------------------------------------------------- a malha

/// O que este aparelho diz de si ao hub.
///
/// Emprestado, e nao dono: quem chama ja tem as quatro coisas na mao, e clonar
/// para anunciar seria alocar por batida sem motivo.
pub struct Anuncio<'a> {
    pub id: &'a str,
    pub nome: &'a str,
    pub plataforma: &'a str,
    pub versao: &'a str,
    pub contrato: u32,
    /// O retrato deste aparelho, por familia. Vazio e resposta valida: um
    /// aparelho que nao conseguiu calcular ainda precisa aparecer na malha.
    pub manifesto: &'a [FamiliaNoAnuncio],
}

/// Uma familia no manifesto, como ela viaja.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FamiliaNoAnuncio {
    pub familia: String,
    pub contagem: usize,
    pub hash: String,
}

/// Um aparelho, como o hub o conhece.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AparelhoNaMalha {
    pub id: String,
    pub nome: String,
    pub plataforma: String,
    pub versao: String,
    pub contrato: u32,
    /// Contagem e hash por familia, como o hub recebeu na ultima batida.
    ///
    /// `default` porque um aparelho que ainda nao atualizou responde sem o
    /// campo, e isso nao pode quebrar a leitura da malha.
    #[serde(default)]
    pub manifesto: Vec<FamiliaNoAnuncio>,
    /// RFC3339, na hora do SERVIDOR — relogio de cliente errado e comum, e um
    /// "visto ha tres dias" que na verdade foi agora manda a investigacao para
    /// o lado errado.
    pub visto_em: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MalhaResposta {
    aparelhos: Vec<AparelhoNaMalha>,
}

/// A batida e a lista vivem FORA do trait `Transport`, e de proposito.
///
/// O trait espelha o que o motor precisa: `push` e `pull`. Identidade de
/// aparelho nao e assunto do motor — enfia-la la dentro obrigaria toda
/// implementacao futura (um transporte de teste, um por arquivo) a fingir que
/// sabe o que e uma versao de app.
impl HttpTransport {
    /// Diz ao hub quem e este aparelho.
    pub fn anunciar(&self, anuncio: &Anuncio<'_>) -> Resultado<()> {
        let resposta = self
            .cliente
            .post(format!("{}/sync/aparelho", self.base))
            .bearer_auth(&self.token)
            .json(&serde_json::json!({
                "id": anuncio.id,
                "nome": anuncio.nome,
                "plataforma": anuncio.plataforma,
                "versao": anuncio.versao,
                "contrato": anuncio.contrato,
                "manifesto": anuncio.manifesto,
            }))
            .send()
            .map_err(erro_de_rede)?;
        if !resposta.status().is_success() {
            return Err(erro_da_resposta(resposta));
        }
        Ok(())
    }

    /// Quem mais esta na malha.
    pub fn malha(&self) -> Resultado<Vec<AparelhoNaMalha>> {
        let resposta = self
            .cliente
            .get(format!("{}/sync/aparelhos", self.base))
            .bearer_auth(&self.token)
            .send()
            .map_err(erro_de_rede)?;
        if !resposta.status().is_success() {
            return Err(erro_da_resposta(resposta));
        }
        let corpo: MalhaResposta = resposta
            .json()
            .map_err(|causa| SyncError::novo(format!("Malha ilegivel: {causa}"), false))?;
        Ok(corpo.aparelhos)
    }
}
