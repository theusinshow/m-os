//! O que o servidor carrega: um M/OS inteiro, pequeno.
//!
//! # Ele e um dispositivo, e nao um proxy
//!
//! A tentacao obvia seria o `mos-web` repassar as chamadas para o desktop, ou
//! ler o banco do hub direto. As duas quebram o desenho: a primeira faz o
//! celular parar de funcionar quando o PC esta desligado — que e justamente
//! quando se tem uma ideia na rua —, e a segunda transformaria o hub em
//! autoridade, coisa que o `SYNC.md` recusa em toda linha.
//!
//! Entao ele tem banco proprio, `mos-core` proprio e identidade propria de
//! dispositivo. Capturar aqui grava aqui e enfileira uma operacao, igualzinho ao
//! desktop. O que o hub faz e o que ele ja fazia: guardar em ordem e devolver.

use std::sync::Arc;

use mos_core::{AttentionService, CaptureService, SystemClock, WorkService};
use mos_storage_sqlite::SqliteStorage;
use mos_sync::DeviceRepository;

use crate::assinaturas::Assinaturas;
use crate::avisos::Avisador;
use crate::porta::Sessoes;
use crate::push::Vapid;

#[derive(Debug, thiserror::Error)]
pub enum SubidaError {
    #[error("{0}")]
    Configuracao(String),
    #[error("banco: {0}")]
    Banco(String),
}

/// Tudo o que as rotas precisam.
#[derive(Clone)]
pub struct Estado {
    pub storage: Arc<SqliteStorage>,
    pub captures: Arc<CaptureService>,
    pub work: Arc<WorkService>,
    /// Os lembretes, que e o que decide quando o celular vibra.
    pub attention: Arc<AttentionService>,
    /// Onde o hub esta, e com que segredo falar com ele. Vazio desliga o sync —
    /// e o `mos-web` continua funcionando, so que sozinho.
    pub hub: Option<Arc<Hub>>,
    /// As sessoes da porta. Vazio quando nao ha porta — e ai `conferir_a_porta`,
    /// no `main.rs`, ja recusou subir publicado.
    pub sessoes: Option<Arc<Sessoes>>,
    /// A cerimonia WebAuthn. So existe com a feature compilada — sem ela o
    /// binario sobe, o guardiao continua fechando a API, e nao ha como abrir uma
    /// sessao. E o estado correto: recusar todo mundo e melhor que deixar passar
    /// todo mundo.
    #[cfg(feature = "passkey")]
    pub webauthn: Option<Arc<crate::auth::Porta>>,
    /// Vazio quando nao ha chave VAPID configurada. O `mos-web` sobe igual e a
    /// tela DIZ que nao ha notificacao — um botao que existe e nao funciona e
    /// pior que um botao ausente.
    pub push: Option<PushLigado>,
}

pub struct Hub {
    pub url: String,
    pub token: String,
}

/// O que a porta precisa para existir.
pub struct Porta {
    /// O arquivo das credenciais e sessoes. Separado do banco de dominio pela
    /// mesma razao do push — ver `porta.rs`.
    pub banco: String,
    /// A URL exata que o navegador ve. O WebAuthn amarra a credencial a ela.
    pub origem: String,
    /// O convite. Passkey autentica quem ja e conhecido; ele nao decide quem
    /// passa a ser.
    pub convite: String,
}

/// O que o push precisa para existir.
pub struct Push {
    /// O arquivo das assinaturas. Separado do banco de dominio de proposito —
    /// ver `assinaturas.rs`.
    pub banco: String,
    /// A chave privada VAPID, base64url. Nasce do `--gerar-vapid`.
    pub privada: String,
    /// `mailto:` ou `https:`. A RFC 8292 exige, e a Apple recusa sem.
    pub contato: String,
}

/// O push, ja montado.
#[derive(Clone)]
pub struct PushLigado {
    pub assinaturas: Arc<Assinaturas>,
    pub avisador: Arc<Avisador>,
    /// A publica, que a tela passa para `pushManager.subscribe`.
    pub chave_publica: String,
}

impl Estado {
    pub fn abrir(
        banco: &str,
        backups: &str,
        hub: Option<Hub>,
        push: Option<Push>,
        porta: Option<Porta>,
    ) -> Result<Self, SubidaError> {
        let storage = Arc::new(
            SqliteStorage::open(banco, backups)
                .map_err(|causa| SubidaError::Banco(causa.message))?,
        );

        // Identidade propria, e o sync ligado. Sem `habilitar_sync` o M/OS
        // funciona igual e nao emite nada — e um `mos-web` que nao emite e um
        // caderno separado com cara de M/OS.
        let device = storage
            .este_dispositivo("M/OS Web", "web", env!("CARGO_PKG_VERSION"))
            .map_err(|causa| SubidaError::Banco(causa.mensagem))?;
        storage
            .habilitar_sync(device.id)
            .map_err(|causa| SubidaError::Banco(causa.message))?;

        // A passagem unica do que ja existia antes do sync.
        //
        // Nao derruba a subida se falhar: o `mos-web` sem backfill continua um
        // M/OS inteiro, e recusar-se a subir por causa dela deixaria o dono sem
        // a superficie de bolso para consertar uma coisa que ele nem viu.
        match storage.backfill_do_sync() {
            Ok(0) => {}
            Ok(quantas) => eprintln!("[sync] backfill inicial: {quantas} operacoes"),
            Err(causa) => eprintln!("[sync] backfill nao passou: {}", causa.message),
        }

        let sessoes = match &porta {
            Some(porta) => Some(Arc::new(
                Sessoes::abrir(&porta.banco)
                    .map_err(|causa| SubidaError::Banco(causa.to_string()))?,
            )),
            None => None,
        };
        // A cerimonia e montada aqui, e nao na primeira requisicao: uma origem
        // invalida tem que derrubar a subida, e nao virar um login que falha na
        // primeira vez que alguem precisar dele.
        #[cfg(feature = "passkey")]
        let webauthn = match &porta {
            Some(porta) => Some(Arc::new(
                crate::auth::Porta::nova(&porta.origem, porta.convite.clone())
                    .map_err(SubidaError::Configuracao)?,
            )),
            None => None,
        };
        // Sem a feature, `porta` ja fez o que tinha para fazer: dizer onde fica
        // o banco das sessoes.
        let _ = &porta;

        // Montado ANTES de responder qualquer rota: uma chave VAPID invalida
        // tem que derrubar a subida, e nao virar um push que falha calado meses
        // depois, na primeira vez que alguem precisar dele.
        let push = match push {
            Some(push) => {
                let vapid = Vapid::nova(&push.privada, &push.contato)
                    .map_err(|causa| SubidaError::Configuracao(causa.to_string()))?;
                let assinaturas = Arc::new(
                    Assinaturas::abrir(&push.banco)
                        .map_err(|causa| SubidaError::Banco(causa.to_string()))?,
                );
                let vapid = Arc::new(vapid);
                Some(PushLigado {
                    chave_publica: vapid.publica_base64(),
                    avisador: Arc::new(Avisador::novo(Arc::clone(&assinaturas), vapid)),
                    assinaturas,
                })
            }
            None => {
                eprintln!(
                    "[web] AVISO: sem MOS_WEB_VAPID_PRIVADA. Nenhuma notificacao sai deste servidor."
                );
                None
            }
        };

        Ok(Self {
            captures: Arc::new(CaptureService::new(Arc::clone(&storage) as Arc<_>)),
            work: Arc::new(WorkService::new(Arc::clone(&storage) as Arc<_>)),
            attention: Arc::new(AttentionService::new(
                Arc::clone(&storage) as Arc<_>,
                Arc::new(SystemClock),
            )),
            storage,
            hub: hub.map(Arc::new),
            sessoes,
            #[cfg(feature = "passkey")]
            webauthn,
            push,
        })
    }
}
