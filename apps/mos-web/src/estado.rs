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

use mos_core::{CaptureService, WorkService};
use mos_storage_sqlite::SqliteStorage;
use mos_sync::DeviceRepository;

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
    /// Onde o hub esta, e com que segredo falar com ele. Vazio desliga o sync —
    /// e o `mos-web` continua funcionando, so que sozinho.
    pub hub: Option<Arc<Hub>>,
}

pub struct Hub {
    pub url: String,
    pub token: String,
}

impl Estado {
    pub fn abrir(banco: &str, backups: &str, hub: Option<Hub>) -> Result<Self, SubidaError> {
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

        Ok(Self {
            captures: Arc::new(CaptureService::new(Arc::clone(&storage) as Arc<_>)),
            work: Arc::new(WorkService::new(Arc::clone(&storage) as Arc<_>)),
            storage,
            hub: hub.map(Arc::new),
        })
    }
}
