//! Web Push: o aviso que chega com o app fechado.
//!
//! # Por que isto e escrito a mao
//!
//! Ver o `Cargo.toml`: o crate `web-push` traria uma segunda versao de `http`
//! para a arvore e um OpenSSL que esta maquina nao tem. O que torna a troca
//! segura nao e coragem — e que a RFC 8291 publica vetores de teste completos.
//! Os testes no fim deste arquivo cifram a mensagem da RFC com as chaves da RFC
//! e conferem o corpo byte a byte. Um push que nao chega nunca diz por que; sem
//! esses vetores, so o iPhone diria — e tarde.
//!
//! # As duas metades
//!
//! **Quem envia** (RFC 8292, VAPID): um JWT ES256 assinado com a chave privada
//! do servidor, dizendo para qual origem ele vai e ate quando vale. E o que
//! impede qualquer um que descubra o endpoint de mandar notificacao no seu
//! nome.
//!
//! **O que vai dentro** (RFC 8291): o conteudo e cifrado com uma chave que so o
//! aparelho consegue derivar — ECDH entre uma chave efemera do servidor e a
//! chave publica do aparelho, misturada ao segredo de autenticacao. O servico
//! de push da Apple encaminha o pacote sem poder ler uma palavra dele.
//!
//! Isso tem uma consequencia que vale dizer alto: **o texto da notificacao nao
//! passa em claro por servidor nenhum**, nem pelo da Apple.

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes128Gcm, Nonce};
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use hkdf::Hkdf;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::{PublicKey, SecretKey};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

/// Tamanho de registro anunciado no cabecalho do corpo cifrado.
///
/// 4096 e o valor do exemplo da RFC e o teto que todo servico de push aceita.
/// Como o M/OS manda uma notificacao por vez e ela cabe folgada num registro,
/// isto nunca precisa variar — o formato admite varios registros, e nao usar
/// isso e o que mantem a cifra em vinte linhas em vez de duzentas.
const RS: u32 = 4096;

/// Quanto tempo o servico de push guarda a mensagem se o aparelho estiver
/// desligado.
///
/// Doze horas. Um lembrete que venceu de manha ainda faz sentido a tarde; um de
/// ontem, nao — e uma notificacao que chega velha ensina a pessoa a ignorar
/// notificacao.
const TTL: u32 = 43_200;

/// Por quanto tempo o JWT vale.
///
/// A RFC 8292 permite ate 24h. Doze e o mesmo teto do TTL, e um JWT com validade
/// longa e um JWT que continua servindo depois de vazar.
const VALIDADE_JWT: i64 = 43_200;

#[derive(Debug, thiserror::Error)]
pub enum PushError {
    #[error("chave: {0}")]
    Chave(String),
    #[error("cifra: {0}")]
    Cifra(String),
    #[error("endpoint: {0}")]
    Endpoint(String),
    #[error("rede: {0}")]
    Rede(String),
    #[error("o servico de push recusou: {codigo} {corpo}")]
    Recusado { codigo: u16, corpo: String },
}

/// Uma assinatura de push, exatamente como o navegador a entrega.
///
/// Os tres campos vem do `PushSubscription.toJSON()` e nao sao interpretados em
/// lugar nenhum: `endpoint` e uma URL do servico de push do fabricante (a Apple,
/// no iPhone), e as duas chaves so servem para derivar o segredo desta
/// mensagem.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Assinatura {
    pub endpoint: String,
    /// A chave publica do aparelho — P-256 em forma nao comprimida, 65 bytes,
    /// base64url.
    pub p256dh: String,
    /// O segredo de autenticacao do aparelho — 16 bytes, base64url.
    pub auth: String,
}

/// O que aconteceu com um envio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entrega {
    /// O servico aceitou. Isso NAO garante que a pessoa viu — garante que a
    /// mensagem saiu daqui e foi aceita para entrega.
    Aceita,
    /// A assinatura morreu (404/410): o app foi desinstalado, ou o navegador a
    /// revogou. Quem chama deve apaga-la — insistir num endpoint morto e gastar
    /// rede toda vez para receber o mesmo erro.
    Morta,
}

/// As chaves VAPID do servidor.
///
/// Elas nascem uma vez e vivem no `/etc/mos-web.env`. **Troca-las mata todas as
/// assinaturas**: o aparelho assinou com a chave publica antiga e o servico de
/// push recusa qualquer coisa assinada pela nova. O sintoma e o pior possivel —
/// tudo parece funcionar e nada chega —, e por isso isto esta escrito aqui, no
/// README e no runbook.
pub struct Vapid {
    privada: SigningKey,
    /// A publica em forma nao comprimida (65 bytes), derivada da privada. Vai no
    /// cabecalho de cada envio e e a mesma que a tela usa para assinar.
    publica: Vec<u8>,
    /// Quem procurar se este servidor se comportar mal. A RFC 8292 exige
    /// `mailto:` ou `https:` — a Apple recusa o envio sem isso.
    contato: String,
}

/// `Debug` escrito a mao, e nao derivado.
///
/// Derivar imprimiria a chave privada em qualquer log que formate este tipo — e
/// um segredo que aparece uma vez num log e um segredo que vazou. Aqui so sai o
/// que ja e publico.
impl std::fmt::Debug for Vapid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vapid")
            .field("publica", &self.publica_base64())
            .field("contato", &self.contato)
            .field("privada", &"<oculta>")
            .finish()
    }
}

impl Vapid {
    /// Sorteia um par novo. Devolve `(privada, publica)` em base64url.
    ///
    /// Usado uma vez, pelo `--gerar-vapid`, e nunca em tempo de execucao: um
    /// servidor que sorteia chave ao subir e um servidor que fica mudo a cada
    /// reinicio.
    pub fn gerar() -> (String, String) {
        let privada = SecretKey::random(&mut OsRng);
        let publica = privada.public_key().to_encoded_point(false);
        (
            B64.encode(privada.to_bytes()),
            B64.encode(publica.as_bytes()),
        )
    }

    pub fn nova(privada_b64: &str, contato: &str) -> Result<Self, PushError> {
        if !contato.starts_with("mailto:") && !contato.starts_with("https:") {
            return Err(PushError::Chave(format!(
                "o contato VAPID precisa comecar com `mailto:` ou `https:` — a \
                 RFC 8292 exige, e o servico de push recusa sem isso. Veio: {contato}"
            )));
        }
        let bytes = B64
            .decode(privada_b64)
            .map_err(|causa| PushError::Chave(format!("privada nao e base64url: {causa}")))?;
        let segredo = SecretKey::from_slice(&bytes)
            .map_err(|causa| PushError::Chave(format!("privada nao e uma chave P-256: {causa}")))?;
        let publica = segredo
            .public_key()
            .to_encoded_point(false)
            .as_bytes()
            .to_vec();
        Ok(Self {
            privada: SigningKey::from(&segredo),
            publica,
            contato: contato.to_string(),
        })
    }

    /// A publica em base64url — o que a tela passa para `pushManager.subscribe`.
    pub fn publica_base64(&self) -> String {
        B64.encode(&self.publica)
    }

    /// O cabecalho `Authorization` para um endpoint.
    ///
    /// A audiencia e a ORIGEM do endpoint (esquema + host), e nao a URL
    /// completa: o caminho identifica o aparelho, e assinar por aparelho faria o
    /// mesmo JWT nao servir para o proximo.
    fn autorizacao(&self, endpoint: &str, agora: i64) -> Result<String, PushError> {
        let origem = origem_de(endpoint)?;
        let cabecalho = B64.encode(br#"{"typ":"JWT","alg":"ES256"}"#);
        let alegacoes = B64.encode(
            serde_json::json!({
                "aud": origem,
                "exp": agora + VALIDADE_JWT,
                "sub": self.contato,
            })
            .to_string(),
        );
        let assinado = format!("{cabecalho}.{alegacoes}");
        // `Signature::to_bytes` devolve r||s cru (64 bytes), que e o que o JOSE
        // pede. A codificacao DER do ECDSA — a que a maioria das bibliotecas
        // devolve por padrao — seria recusada sem explicacao.
        let assinatura: Signature = self.privada.sign(assinado.as_bytes());
        let jwt = format!("{assinado}.{}", B64.encode(assinatura.to_bytes()));
        Ok(format!("vapid t={jwt}, k={}", self.publica_base64()))
    }
}

/// A origem de uma URL: esquema + host, sem caminho.
fn origem_de(endpoint: &str) -> Result<String, PushError> {
    let (esquema, resto) = endpoint
        .split_once("://")
        .ok_or_else(|| PushError::Endpoint(format!("sem esquema: {endpoint}")))?;
    let host = resto.split('/').next().unwrap_or_default();
    if host.is_empty() {
        return Err(PushError::Endpoint(format!("sem host: {endpoint}")));
    }
    Ok(format!("{esquema}://{host}"))
}

/// Cifra uma mensagem para uma assinatura, no formato `aes128gcm`.
///
/// `salt` e `efemera` entram por parametro em vez de serem sorteados aqui — e a
/// unica razao pela qual os vetores da RFC podem ser reproduzidos byte a byte.
/// Quem usa de verdade chama [`cifrar`], que sorteia os dois.
fn cifrar_com(
    assinatura: &Assinatura,
    texto: &[u8],
    salt: &[u8; 16],
    efemera: &SecretKey,
) -> Result<Vec<u8>, PushError> {
    let ua_publica_bytes = B64
        .decode(&assinatura.p256dh)
        .map_err(|causa| PushError::Chave(format!("p256dh nao e base64url: {causa}")))?;
    let auth = B64
        .decode(&assinatura.auth)
        .map_err(|causa| PushError::Chave(format!("auth nao e base64url: {causa}")))?;
    let ua_publica = PublicKey::from_sec1_bytes(&ua_publica_bytes)
        .map_err(|causa| PushError::Chave(format!("p256dh nao e um ponto P-256: {causa}")))?;

    let as_publica = efemera.public_key().to_encoded_point(false);
    let as_publica = as_publica.as_bytes();

    // O segredo compartilhado: a coordenada x do ponto ECDH. Os dois lados
    // chegam a ela sem nunca a transmitirem.
    let compartilhado =
        p256::ecdh::diffie_hellman(efemera.to_nonzero_scalar(), ua_publica.as_affine());

    // RFC 8291 §3.3: o segredo do ECDH e o segredo de autenticacao viram uma
    // chave so. O `key_info` amarra a derivacao as DUAS chaves publicas — e o
    // que impede reaproveitar um pacote cifrado para outro aparelho.
    let mut key_info = Vec::with_capacity(14 + 65 + 65);
    key_info.extend_from_slice(b"WebPush: info\0");
    key_info.extend_from_slice(&ua_publica_bytes);
    key_info.extend_from_slice(as_publica);

    let mut ikm = [0u8; 32];
    Hkdf::<Sha256>::new(Some(&auth), compartilhado.raw_secret_bytes())
        .expand(&key_info, &mut ikm)
        .map_err(|causa| PushError::Cifra(format!("IKM: {causa}")))?;

    // RFC 8188 §2.2: daqui para baixo e o `aes128gcm` generico, e o salt e o que
    // torna cada mensagem unica mesmo com as mesmas chaves.
    let derivada = Hkdf::<Sha256>::new(Some(salt), &ikm);
    let mut cek = [0u8; 16];
    derivada
        .expand(b"Content-Encoding: aes128gcm\0", &mut cek)
        .map_err(|causa| PushError::Cifra(format!("CEK: {causa}")))?;
    let mut nonce = [0u8; 12];
    derivada
        .expand(b"Content-Encoding: nonce\0", &mut nonce)
        .map_err(|causa| PushError::Cifra(format!("nonce: {causa}")))?;

    // O 0x02 e o delimitador de "ultimo registro". Com 0x01 o aparelho fica
    // esperando um registro que nunca vem e descarta a mensagem em silencio.
    let mut conteudo = texto.to_vec();
    conteudo.push(0x02);

    let cifrado = Aes128Gcm::new_from_slice(&cek)
        .map_err(|causa| PushError::Cifra(format!("chave AES: {causa}")))?
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &conteudo,
                aad: b"",
            },
        )
        .map_err(|causa| PushError::Cifra(format!("AES-GCM: {causa}")))?;

    // O cabecalho do corpo, RFC 8188 §2.1: salt, tamanho de registro, tamanho da
    // chave, a chave publica efemera. Ele vai EM CLARO — e assim que o aparelho
    // sabe com que chave decifrar.
    let mut corpo = Vec::with_capacity(21 + as_publica.len() + cifrado.len());
    corpo.extend_from_slice(salt);
    corpo.extend_from_slice(&RS.to_be_bytes());
    corpo.push(as_publica.len() as u8);
    corpo.extend_from_slice(as_publica);
    corpo.extend_from_slice(&cifrado);
    Ok(corpo)
}

/// Cifra uma mensagem, sorteando salt e chave efemera.
pub fn cifrar(assinatura: &Assinatura, texto: &[u8]) -> Result<Vec<u8>, PushError> {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    cifrar_com(assinatura, texto, &salt, &SecretKey::random(&mut OsRng))
}

/// Manda uma notificacao.
///
/// Bloqueante, como o transporte do sync e pela mesma razao: quem chama ja esta
/// dentro de `spawn_blocking`. Chamar isto de dentro de um worker do tokio
/// derruba o processo na hora.
pub fn enviar(
    cliente: &reqwest::blocking::Client,
    vapid: &Vapid,
    assinatura: &Assinatura,
    texto: &[u8],
    agora: i64,
) -> Result<Entrega, PushError> {
    let corpo = cifrar(assinatura, texto)?;
    let resposta = cliente
        .post(&assinatura.endpoint)
        .header("TTL", TTL.to_string())
        .header("Content-Encoding", "aes128gcm")
        .header("Content-Type", "application/octet-stream")
        .header(
            "Authorization",
            vapid.autorizacao(&assinatura.endpoint, agora)?,
        )
        .body(corpo)
        .send()
        .map_err(|causa| PushError::Rede(causa.to_string()))?;

    let codigo = resposta.status().as_u16();
    match codigo {
        // 201 e o certo; 200 e 202 aparecem em alguns servicos.
        200..=299 => Ok(Entrega::Aceita),
        // A assinatura acabou. Nao e erro — e informacao, e quem chama apaga.
        404 | 410 => Ok(Entrega::Morta),
        _ => Err(PushError::Recusado {
            codigo,
            corpo: resposta.text().unwrap_or_default(),
        }),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    // Os valores da RFC 8291 §5. Reproduzidos aqui e nao gerados: o objetivo do
    // teste e provar que ESTA implementacao concorda com a especificacao, e um
    // vetor derivado do proprio codigo provaria apenas que ele concorda consigo
    // mesmo.
    const TEXTO: &[u8] = b"When I grow up, I want to be a watermelon";
    const UA_PUBLICA: &str =
        "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4";
    const AUTH: &str = "BTBZMqHH6r4Tts7J_aSIgg";
    const AS_PRIVADA: &str = "yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw";
    const SALT: &str = "DGv6ra1nlYgDCS1FRnbzlw";
    const CORPO_ESPERADO: &str = "DGv6ra1nlYgDCS1FRnbzlwAAEABBBP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A_yl95bQpu6cVPTpK4Mqgkf1CXztLVBSt2Ks3oZwbuwXPXLWyouBWLVWGNWQexSgSxsj_Qulcy4a-fN";

    fn assinatura_da_rfc() -> Assinatura {
        Assinatura {
            endpoint: String::from(
                "https://push.example.net/push/JzLQ3raZJfFBR0aqvOMsLrt54w4rJUsV",
            ),
            p256dh: String::from(UA_PUBLICA),
            auth: String::from(AUTH),
        }
    }

    /// O teste que decide se este arquivo esta certo.
    ///
    /// Com as chaves, o salt e a chave efemera da RFC, o corpo cifrado tem que
    /// sair identico ao publicado — byte a byte, cabecalho e ciphertext.
    #[test]
    fn o_corpo_cifrado_bate_com_o_vetor_da_rfc_8291() {
        let salt: [u8; 16] = B64.decode(SALT).unwrap().try_into().unwrap();
        let efemera = SecretKey::from_slice(&B64.decode(AS_PRIVADA).unwrap()).unwrap();

        let corpo = cifrar_com(&assinatura_da_rfc(), TEXTO, &salt, &efemera).unwrap();

        assert_eq!(B64.encode(&corpo), CORPO_ESPERADO);
    }

    /// O cabecalho em claro do corpo, conferido campo a campo.
    ///
    /// Existe separado porque, quando o teste acima falha, ele diz ONDE: um
    /// tamanho de registro errado e um erro completamente diferente de uma
    /// chave derivada errada, e um `assert_eq` de 145 bytes nao distingue os
    /// dois.
    #[test]
    fn o_cabecalho_do_corpo_tem_a_forma_da_rfc_8188() {
        let salt: [u8; 16] = B64.decode(SALT).unwrap().try_into().unwrap();
        let efemera = SecretKey::from_slice(&B64.decode(AS_PRIVADA).unwrap()).unwrap();

        let corpo = cifrar_com(&assinatura_da_rfc(), TEXTO, &salt, &efemera).unwrap();

        assert_eq!(&corpo[..16], &salt, "o salt vai em claro, no comeco");
        assert_eq!(
            u32::from_be_bytes(corpo[16..20].try_into().unwrap()),
            RS,
            "tamanho de registro"
        );
        assert_eq!(corpo[20], 65, "uma chave P-256 nao comprimida tem 65 bytes");
        assert_eq!(corpo[21], 0x04, "e comeca com 0x04");
        assert_eq!(
            corpo.len(),
            21 + 65 + TEXTO.len() + 1 + 16,
            "cabecalho + chave + texto + delimitador + tag do GCM"
        );
    }

    /// Sem salt fixo, duas cifras da mesma mensagem tem que sair diferentes.
    ///
    /// Se saissem iguais, quem observa a rede saberia que a mesma notificacao
    /// foi repetida — e num aparelho pessoal isso ja e informacao demais.
    #[test]
    fn duas_cifras_da_mesma_mensagem_nao_se_repetem() {
        let assinatura = assinatura_da_rfc();
        let uma = cifrar(&assinatura, TEXTO).unwrap();
        let outra = cifrar(&assinatura, TEXTO).unwrap();
        assert_ne!(uma, outra);
        assert_eq!(uma.len(), outra.len());
    }

    #[test]
    fn a_chave_publica_vapid_vem_da_privada() {
        let (privada, publica) = Vapid::gerar();
        let vapid = Vapid::nova(&privada, "mailto:eu@exemplo.com").unwrap();
        assert_eq!(vapid.publica_base64(), publica);
        assert_eq!(B64.decode(&publica).unwrap().len(), 65);
    }

    /// O JWT tem que ser verificavel com a chave publica que viaja ao lado dele
    /// — que e exatamente o que o servico de push faz ao receber.
    #[test]
    fn o_jwt_se_verifica_com_a_chave_publica_anunciada() {
        use p256::ecdsa::signature::Verifier;
        use p256::ecdsa::VerifyingKey;

        let (privada, _) = Vapid::gerar();
        let vapid = Vapid::nova(&privada, "mailto:eu@exemplo.com").unwrap();
        let cabecalho = vapid
            .autorizacao("https://web.push.apple.com/abc/def", 1_700_000_000)
            .unwrap();

        let (t, k) = cabecalho
            .trim_start_matches("vapid ")
            .split_once(", ")
            .expect("o cabecalho tem t= e k=");
        let jwt = t.trim_start_matches("t=");
        let chave = k.trim_start_matches("k=");

        let (assinado, assinatura) = jwt.rsplit_once('.').unwrap();
        let publica = VerifyingKey::from_sec1_bytes(&B64.decode(chave).unwrap()).unwrap();
        let assinatura = Signature::from_slice(&B64.decode(assinatura).unwrap()).unwrap();
        publica
            .verify(assinado.as_bytes(), &assinatura)
            .expect("o servico de push confere isto, e recusa se nao bater");

        let alegacoes: serde_json::Value =
            serde_json::from_slice(&B64.decode(assinado.split_once('.').unwrap().1).unwrap())
                .unwrap();
        assert_eq!(
            alegacoes["aud"], "https://web.push.apple.com",
            "a audiencia e a ORIGEM, sem o caminho que identifica o aparelho"
        );
        assert_eq!(alegacoes["exp"], 1_700_000_000i64 + VALIDADE_JWT);
        assert_eq!(alegacoes["sub"], "mailto:eu@exemplo.com");
    }

    /// A Apple recusa um `sub` que nao seja `mailto:` ou `https:`, e o erro dela
    /// nao explica isso. Recusar aqui transforma um mistrerio em producao numa
    /// mensagem na subida.
    #[test]
    fn um_contato_vapid_invalido_e_recusado_na_subida() {
        let (privada, _) = Vapid::gerar();
        let erro = Vapid::nova(&privada, "eu@exemplo.com").unwrap_err();
        assert!(erro.to_string().contains("mailto:"), "{erro}");
    }

    #[test]
    fn a_origem_ignora_o_caminho() {
        assert_eq!(
            origem_de("https://web.push.apple.com/abc?x=1").unwrap(),
            "https://web.push.apple.com"
        );
        assert!(origem_de("web.push.apple.com/abc").is_err());
    }
}
