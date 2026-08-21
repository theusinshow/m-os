//! O motor de sincronizacao do M/OS, sem plataforma nenhuma dentro.
//!
//! # O que este crate e
//!
//! As regras de como duas historias viram uma: o relogio que ordena eventos
//! entre dispositivos sem servidor arbitro, a operacao que viaja, a
//! reconciliacao por campo e a identidade de cada instalacao.
//!
//! # O que este crate NAO e
//!
//! Nao persiste, nao fala rede e nao conhece SQLite, HTTP nem Tauri. Quem
//! guarda a fila e o adaptador de armazenamento; quem transporta e o cliente de
//! sync. Essa fronteira e o motivo de o crate existir separado: ele precisa
//! compilar **identico** no Windows e no iOS, e a unica forma de garantir isso
//! e nao ter como escrever codigo de plataforma aqui dentro.
//!
//! E a mesma disciplina que `mos-core` ja seguia — e foi ela que fez esta
//! missao ser possivel sem reescrever o M/OS.
//!
//! # O contrato
//!
//! `CONTRACT_VERSION` acompanha o formato do que viaja entre dispositivos, e
//! nao a versao do aplicativo. Ele sobe quando o formato muda de um jeito que
//! um cliente antigo nao entende; ver `docs/SYNC.md`.

mod clock;
mod device;
mod engine;
mod merge;
mod op;
mod ports;

pub use clock::{Hlc, HlcClock};
pub use engine::{
    carregar_relogio, erro_de_contrato, instante_de, sincronizar, Deposito, Lote, Projecao, Rodada,
    Transport,
};
pub use device::{Device, DeviceId, Platform};
pub use merge::{aplicar, CampoResolvido, Conflito, EstadoDaEntidade, Reconciliacao};
pub use op::{EntityKind, EntityRef, Op, OpBody};
pub use ports::{
    ClockRepository, ConflictRepository, DeviceRepository, OutboxRepository, Resultado, SyncError,
};

/// A versao do formato que viaja entre dispositivos.
///
/// **Nao e a versao do app.** Desktop 0.9 e iPhone 0.7 podem falar o contrato 1
/// sem problema, e e exatamente isso que os §27, §73 e §74 exigem: a App Store
/// nao publica quando o desktop publica, e o sistema precisa continuar de pe com
/// as duas versoes convivendo.
pub const CONTRACT_VERSION: u32 = 1;

/// O menor contrato que este cliente ainda aceita receber.
///
/// Ficar igual a `CONTRACT_VERSION` seria dizer "so falo com quem esta na minha
/// versao", e ai a primeira atualizacao quebraria o outro lado.
pub const MIN_CONTRACT_VERSION: u32 = 1;

/// Se um cliente que fala `outro` consegue conversar com este.
pub fn contrato_compativel(outro: u32) -> bool {
    outro >= MIN_CONTRACT_VERSION && outro <= CONTRACT_VERSION
}

#[cfg(test)]
mod tests;
