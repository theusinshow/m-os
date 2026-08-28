//! O caminho inteiro da notificacao, com o aparelho fazendo o papel dele.
//!
//! # Por que este teste decifra
//!
//! Um teste que so conferisse "saiu um POST" passaria com o corpo cifrado
//! errado, com a chave errada e com o texto errado. Todos esses defeitos tem o
//! MESMO sintoma no mundo real: o iPhone recebe o pacote, nao consegue abrir, e
//! descarta em silencio. Nada aparece na tela e nada aparece no log do
//! servidor, que recebeu 201 e considera o trabalho feito.
//!
//! Entao aqui o teste faz o papel do aparelho de verdade: gera o proprio par de
//! chaves P-256 e o proprio segredo de autenticacao, assina com eles, e depois
//! **abre o que chegou**. Se o texto decifrado bate, o iPhone tambem abriria.
//!
//! O servico de push da Apple e substituido por um `axum` local — o que o M/OS
//! precisa provar e o que ele MANDA, e o que a Apple faz com isso e problema
//! dela.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes128Gcm, Nonce};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::Router;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use hkdf::Hkdf;
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::{PublicKey, SecretKey};
use rand_core::{OsRng, RngCore};
use sha2::Sha256;

/// O que o servico de push falso guardou.
#[derive(Clone, Debug)]
struct Recebido {
    cabecalhos: HeaderMap,
    corpo: Vec<u8>,
}

#[derive(Clone)]
struct ServicoFalso {
    recebidos: Arc<Mutex<Vec<Recebido>>>,
    /// O que responder. `201` e o normal; `410` e a assinatura morta.
    resposta: StatusCode,
}

async fn receber_push(
    State(servico): State<ServicoFalso>,
    cabecalhos: HeaderMap,
    corpo: axum::body::Bytes,
) -> StatusCode {
    servico.recebidos.lock().unwrap().push(Recebido {
        cabecalhos,
        corpo: corpo.to_vec(),
    });
    servico.resposta
}

/// Sobe o servico de push falso e devolve (endereco, o que ele recebeu).
async fn servico_de_push(resposta: StatusCode) -> (SocketAddr, Arc<Mutex<Vec<Recebido>>>) {
    let recebidos = Arc::new(Mutex::new(Vec::new()));
    let servico = ServicoFalso {
        recebidos: Arc::clone(&recebidos),
        resposta,
    };
    let rotas = Router::new()
        .route("/push/{id}", post(receber_push))
        .with_state(servico);
    let ouvinte = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endereco = ouvinte.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(ouvinte, rotas).await.unwrap();
    });
    (endereco, recebidos)
}

/// Um aparelho: o par de chaves e o segredo que o navegador geraria.
struct Aparelho {
    privada: SecretKey,
    publica_b64: String,
    auth: [u8; 16],
}

impl Aparelho {
    fn novo() -> Self {
        let privada = SecretKey::random(&mut OsRng);
        let publica_b64 = B64.encode(privada.public_key().to_encoded_point(false).as_bytes());
        let mut auth = [0u8; 16];
        OsRng.fill_bytes(&mut auth);
        Self {
            privada,
            publica_b64,
            auth,
        }
    }

    fn assinatura(&self, push: SocketAddr) -> serde_json::Value {
        serde_json::json!({
            "endpoint": format!("http://{push}/push/este-aparelho"),
            "p256dh": self.publica_b64,
            "auth": B64.encode(self.auth),
        })
    }

    /// O que o service worker do iPhone faria ao receber o pacote.
    ///
    /// E a operacao inversa de `push::cifrar`, derivada da RFC 8291 de forma
    /// independente — se as duas concordarem por engano, elas concordam com a
    /// especificacao, porque foi dela que as duas sairam.
    fn decifrar(&self, corpo: &[u8]) -> String {
        let salt = &corpo[..16];
        let tamanho_da_chave = corpo[20] as usize;
        let as_publica = &corpo[21..21 + tamanho_da_chave];
        let cifrado = &corpo[21 + tamanho_da_chave..];

        let compartilhado = p256::ecdh::diffie_hellman(
            self.privada.to_nonzero_scalar(),
            PublicKey::from_sec1_bytes(as_publica).unwrap().as_affine(),
        );

        let mut key_info = Vec::new();
        key_info.extend_from_slice(b"WebPush: info\0");
        key_info.extend_from_slice(&B64.decode(&self.publica_b64).unwrap());
        key_info.extend_from_slice(as_publica);

        let mut ikm = [0u8; 32];
        Hkdf::<Sha256>::new(Some(&self.auth), compartilhado.raw_secret_bytes())
            .expand(&key_info, &mut ikm)
            .unwrap();

        let derivada = Hkdf::<Sha256>::new(Some(salt), &ikm);
        let mut cek = [0u8; 16];
        derivada
            .expand(b"Content-Encoding: aes128gcm\0", &mut cek)
            .unwrap();
        let mut nonce = [0u8; 12];
        derivada
            .expand(b"Content-Encoding: nonce\0", &mut nonce)
            .unwrap();

        let mut aberto = Aes128Gcm::new_from_slice(&cek)
            .unwrap()
            .decrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: cifrado,
                    aad: b"",
                },
            )
            .expect("o aparelho tem que conseguir abrir o que o servidor cifrou");
        // O ultimo byte e o delimitador de registro.
        assert_eq!(aberto.pop(), Some(0x02), "delimitador de ultimo registro");
        String::from_utf8(aberto).unwrap()
    }
}

/// Sobe um `mos-web` de verdade, com push ligado e sem hub.
async fn servir_web(pasta: &std::path::Path) -> SocketAddr {
    let backups = pasta.join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let (privada, _) = mos_web::push::Vapid::gerar();

    let estado = mos_web::estado::Estado::abrir(
        pasta.join("web.db").to_str().unwrap(),
        backups.to_str().unwrap(),
        // Sem hub: o que este arquivo prova e a notificacao, e um hub aqui so
        // acrescentaria uma rodada de sync sem para onde ir.
        None,
        Some(mos_web::estado::Push {
            banco: pasta.join("push.db").to_str().unwrap().to_string(),
            privada,
            contato: String::from("mailto:eu@exemplo.com"),
        }),
        None,
    )
    .unwrap();

    let ouvinte = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endereco = ouvinte.local_addr().unwrap();
    let rotas = mos_web::api::rotas().with_state(estado);
    tokio::spawn(async move {
        axum::serve(ouvinte, rotas).await.unwrap();
    });
    endereco
}

/// O teste que responde a unica pergunta que importa: chega, e da para ler?
#[tokio::test]
async fn a_notificacao_sai_cifrada_e_o_aparelho_consegue_abrir() {
    let pasta = tempfile::tempdir().unwrap();
    let (push, recebidos) = servico_de_push(StatusCode::CREATED).await;
    let web = servir_web(pasta.path()).await;
    let aparelho = Aparelho::novo();
    let cliente = reqwest::Client::new();

    // 1. A tela assina.
    let resposta = cliente
        .post(format!("http://{web}/api/push/assinar"))
        .json(&aparelho.assinatura(push))
        .send()
        .await
        .unwrap();
    assert_eq!(resposta.status(), 200);

    // 2. O botao "testar agora".
    let resposta: serde_json::Value = cliente
        .post(format!("http://{web}/api/push/testar"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(resposta["enviadas"], 1, "o servidor achou que mandou uma");

    // 3. O servico de push recebeu — com os cabecalhos que a Apple exige.
    let recebidos = recebidos.lock().unwrap().clone();
    assert_eq!(recebidos.len(), 1);
    let recebido = &recebidos[0];
    assert_eq!(
        recebido.cabecalhos["content-encoding"], "aes128gcm",
        "sem este cabecalho o servico nao sabe o que esta encaminhando"
    );
    assert!(recebido.cabecalhos.contains_key("ttl"), "TTL e obrigatorio");
    let autorizacao = recebido.cabecalhos["authorization"].to_str().unwrap();
    assert!(autorizacao.starts_with("vapid t="), "{autorizacao}");
    assert!(autorizacao.contains(", k="), "a chave publica viaja junto");

    // 4. E o aparelho consegue abrir o que veio.
    let texto = aparelho.decifrar(&recebido.corpo);
    let aviso: serde_json::Value = serde_json::from_str(&texto).unwrap();
    assert_eq!(aviso["titulo"], "M/OS");
    assert!(
        aviso["corpo"].as_str().unwrap().contains("funciona"),
        "{aviso}"
    );
}

/// Quando o fabricante diz que a assinatura morreu, ela some daqui.
///
/// Sem isto, um app desinstalado viraria uma ida a rede por minuto, para sempre,
/// para receber sempre o mesmo 410.
#[tokio::test]
async fn a_assinatura_morta_e_esquecida() {
    let pasta = tempfile::tempdir().unwrap();
    let (push, _) = servico_de_push(StatusCode::GONE).await;
    let web = servir_web(pasta.path()).await;
    let aparelho = Aparelho::novo();
    let cliente = reqwest::Client::new();

    cliente
        .post(format!("http://{web}/api/push/assinar"))
        .json(&aparelho.assinatura(push))
        .send()
        .await
        .unwrap();

    let estado: serde_json::Value = cliente
        .get(format!("http://{web}/api/estado"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(estado["aparelhosAvisados"], 1, "assinou");

    let resposta: serde_json::Value = cliente
        .post(format!("http://{web}/api/push/testar"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(resposta["enviadas"], 0, "nenhuma foi aceita");

    let estado: serde_json::Value = cliente
        .get(format!("http://{web}/api/estado"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(estado["aparelhosAvisados"], 0, "e a morta foi removida");
}

/// Assinar duas vezes do mesmo aparelho nao pode virar duas notificacoes.
#[tokio::test]
async fn assinar_de_novo_nao_duplica_a_notificacao() {
    let pasta = tempfile::tempdir().unwrap();
    let (push, recebidos) = servico_de_push(StatusCode::CREATED).await;
    let web = servir_web(pasta.path()).await;
    let aparelho = Aparelho::novo();
    let cliente = reqwest::Client::new();

    for _ in 0..3 {
        cliente
            .post(format!("http://{web}/api/push/assinar"))
            .json(&aparelho.assinatura(push))
            .send()
            .await
            .unwrap();
    }

    cliente
        .post(format!("http://{web}/api/push/testar"))
        .send()
        .await
        .unwrap();

    assert_eq!(recebidos.lock().unwrap().len(), 1);
}

/// Sem chave VAPID o servidor sobe igual — e diz que nao notifica, em vez de
/// oferecer um botao que nao faz nada.
#[tokio::test]
async fn sem_chave_vapid_a_tela_sabe_que_nao_ha_push() {
    let pasta = tempfile::tempdir().unwrap();
    let backups = pasta.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let estado = mos_web::estado::Estado::abrir(
        pasta.path().join("web.db").to_str().unwrap(),
        backups.to_str().unwrap(),
        None,
        None,
        None,
    )
    .unwrap();
    let ouvinte = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let web = ouvinte.local_addr().unwrap();
    let rotas = mos_web::api::rotas().with_state(estado);
    tokio::spawn(async move {
        axum::serve(ouvinte, rotas).await.unwrap();
    });

    let cliente = reqwest::Client::new();
    let estado: serde_json::Value = cliente
        .get(format!("http://{web}/api/estado"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(estado["chavePush"].is_null(), "sem chave, a tela sabe");

    let resposta = cliente
        .post(format!("http://{web}/api/push/testar"))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resposta.status(),
        501,
        "501 e nao 500: nao ha defeito, ha configuracao ausente"
    );
}
