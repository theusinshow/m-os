//! As fronteiras do motor de sincronizacao com o mundo.
//!
//! Mesma disciplina de `mos-core::ports`: o dominio declara o que precisa, e
//! quem guarda decide como. E isso que permite o mesmo motor rodar sobre o
//! SQLite do desktop e sobre o SQLite do iPhone sem uma linha diferente.

use crate::{Conflito, Device, DeviceId, Hlc, Op};

/// Erro de armazenamento, visto pelo motor.
///
/// Proprio, e nao o `CoreError` de `mos-core`: este crate nao depende do core,
/// e nao deve passar a depender so para reaproveitar um tipo de erro. Quem
/// implementa a porta converte na fronteira, que e o lugar de converter.
#[derive(Debug, thiserror::Error)]
#[error("{mensagem}")]
pub struct SyncError {
    pub mensagem: String,
    /// Se tentar de novo tem chance de dar certo. Disco cheio tem; JSON
    /// corrompido nao.
    pub retriavel: bool,
}

impl SyncError {
    pub fn novo(mensagem: impl Into<String>, retriavel: bool) -> Self {
        Self {
            mensagem: mensagem.into(),
            retriavel,
        }
    }
}

pub type Resultado<T> = Result<T, SyncError>;

/// Quem sou eu, e quem mais existe.
pub trait DeviceRepository: Send + Sync {
    /// O dispositivo desta instalacao, criando na primeira vez.
    ///
    /// Idempotente de proposito: e chamado em toda abertura do app, e abrir o
    /// M/OS duas vezes nao pode criar dois dispositivos.
    fn este_dispositivo(&self, nome: &str, plataforma: &str, versao: &str)
        -> Resultado<Device>;

    /// Todos os conhecidos, este primeiro.
    fn listar(&self) -> Resultado<Vec<Device>>;

    /// Marca o instante da ultima sincronizacao concluida.
    fn marcar_sync(&self, id: DeviceId, quando: &str) -> Resultado<()>;
}

/// A fila do que saiu daqui e ainda nao foi confirmado.
pub trait OutboxRepository: Send + Sync {
    /// Enfileira. Reenfileirar a MESMA operacao nao duplica — a chave e o id
    /// dela, que nasceu na origem.
    fn enfileirar(&self, op: &Op) -> Resultado<()>;

    /// As proximas a enviar, em ordem de instante.
    fn pendentes(&self, limite: usize) -> Resultado<Vec<Op>>;

    /// O outro lado confirmou. Sai da fila.
    fn confirmar(&self, ids: &[uuid::Uuid]) -> Resultado<()>;

    /// Falhou. Guarda o motivo e conta a tentativa, para o backoff e para o
    /// diagnostico que o §39 pede.
    fn falhou(&self, id: uuid::Uuid, motivo: &str) -> Resultado<()>;

    /// Quantas esperam. E o que a interface le para dizer "alteracoes
    /// pendentes" (§40) sem varrer a fila inteira.
    fn quantidade_pendente(&self) -> Resultado<usize>;
}

/// Onde os conflitos ficam guardados ate alguem olhar.
pub trait ConflictRepository: Send + Sync {
    fn registrar(&self, entity_kind: &str, entity_id: uuid::Uuid, conflitos: &[Conflito])
        -> Resultado<()>;
    fn abertos(&self, limite: usize) -> Resultado<Vec<Conflito>>;
}

/// O relogio logico, entre execucoes.
pub trait ClockRepository: Send + Sync {
    fn carregar(&self) -> Resultado<Option<Hlc>>;
    fn guardar(&self, agora: Hlc) -> Resultado<()>;
    /// Ate onde este dispositivo ja puxou. Vazio significa "nunca puxou", e e
    /// o que dispara a sincronizacao inicial em vez da incremental.
    fn cursor(&self) -> Resultado<String>;
    fn guardar_cursor(&self, cursor: &str) -> Resultado<()>;
}
