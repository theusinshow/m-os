//! O hub: guarda em ordem, devolve a partir de um cursor, aceita reenvio sem
//! duplicar.
//!
//! # O que ele NAO faz
//!
//! Nao reconcilia, nao projeta, nao conhece Task nem Project. O `HubLocal` do
//! teste `sync_two_devices.rs` foi escrito de proposito como "o menor que
//! satisfaz o contrato", com a observacao de que um hub esperto esconderia
//! defeito do motor. Isto aqui e a mesma coisa, com um disco embaixo: a unica
//! diferenca entre os dois e que este sobrevive a um reinicio.
//!
//! # Por que o log e append-only
//!
//! O cliente pede "o que mudou desde o cursor", e a resposta so e estavel se
//! nada for reescrito no meio. Uma operacao nunca e alterada nem removida: o
//! `OpBody::Delete` do dominio ja e apagamento LOGICO, e apagar a linha do log
//! deixaria um dispositivo offline sem saber que algo sumiu — uma linha ausente
//! e indistinguivel de uma que nunca chegou.

use std::path::Path;

use mos_sync::{Lote, Op};
use rusqlite::{Connection, OptionalExtension};

#[derive(Debug, thiserror::Error)]
pub enum HubError {
    #[error("banco do hub: {0}")]
    Banco(#[from] rusqlite::Error),
    #[error("operacao ilegivel: {0}")]
    Payload(#[from] serde_json::Error),
}

pub type Resultado<T> = Result<T, HubError>;

/// O log de operacoes, em SQLite.
pub struct Hub {
    conexao: Connection,
}

impl Hub {
    /// Abre (ou cria) o log no caminho dado.
    pub fn abrir(caminho: impl AsRef<Path>) -> Resultado<Self> {
        let conexao = Connection::open(caminho)?;
        Self::preparar(conexao)
    }

    /// Um hub em memoria, para teste.
    pub fn em_memoria() -> Resultado<Self> {
        Self::preparar(Connection::open_in_memory()?)
    }

    fn preparar(conexao: Connection) -> Resultado<Self> {
        // WAL e `synchronous = FULL` pela mesma razao do desktop: o hub e a
        // unica copia de uma operacao entre o momento em que um dispositivo a
        // confirma e o momento em que o outro a busca. Perder essa janela num
        // corte de energia perderia trabalho que o cliente ja considera salvo.
        conexao.pragma_update(None, "journal_mode", "WAL")?;
        conexao.pragma_update(None, "synchronous", "FULL")?;
        conexao.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS sync_log (
                -- O cursor. `AUTOINCREMENT` e nao rowid puro: sem ele o SQLite
                -- reaproveita o id de uma linha removida, e um cursor que anda
                -- para tras faria um dispositivo receber duas vezes o que ja
                -- tinha. Nada e removido aqui hoje, mas o cursor e uma promessa
                -- externa e nao deve depender disso continuar verdade.
                seq        INTEGER PRIMARY KEY AUTOINCREMENT,
                -- A chave de idempotencia, nascida na origem.
                op_id      TEXT NOT NULL UNIQUE,
                device     TEXT NOT NULL,
                payload    TEXT NOT NULL,
                recebido_em TEXT NOT NULL
            );
            "#,
        )?;
        Ok(Self { conexao })
    }

    /// Guarda o que ainda nao conhece e CONFIRMA tudo.
    ///
    /// Aceitar e diferente de guardar. Uma operacao que o hub ja tem e
    /// confirmada do mesmo jeito — e exatamente isso que faz o retry do cliente
    /// ser seguro: ele tira da fila o que foi confirmado, e reenviar dez vezes
    /// precisa dar o mesmo resultado de enviar uma.
    pub fn push(&mut self, ops: &[Op], recebido_em: &str) -> Resultado<Vec<uuid::Uuid>> {
        // Uma transacao para o lote inteiro: metade de um lote gravada deixaria
        // o cursor no meio de uma unidade que o cliente enviou junta.
        let transacao = self.conexao.transaction()?;
        let mut aceitas = Vec::with_capacity(ops.len());
        {
            let mut inserir = transacao.prepare(
                "INSERT OR IGNORE INTO sync_log (op_id, device, payload, recebido_em)
                 VALUES (?1, ?2, ?3, ?4)",
            )?;
            for op in ops {
                inserir.execute((
                    op.id.to_string(),
                    op.device().to_string(),
                    serde_json::to_string(op)?,
                    recebido_em,
                ))?;
                aceitas.push(op.id);
            }
        }
        transacao.commit()?;
        Ok(aceitas)
    }

    /// Devolve o que entrou depois do cursor.
    ///
    /// O cursor e opaco para o cliente — ele so guarda e devolve. Vazio
    /// significa "nunca puxei", e e o que dispara a sincronizacao inicial.
    pub fn pull(&self, cursor: &str, limite: usize) -> Resultado<Lote> {
        let de: i64 = cursor.parse().unwrap_or(0);
        let mut consulta = self
            .conexao
            .prepare("SELECT seq, payload FROM sync_log WHERE seq > ?1 ORDER BY seq LIMIT ?2")?;
        let mut ops = Vec::new();
        let mut ultimo = de;
        let linhas = consulta.query_map((de, limite as i64), |linha| {
            Ok((linha.get::<_, i64>(0)?, linha.get::<_, String>(1)?))
        })?;
        for linha in linhas {
            let (seq, payload) = linha?;
            ops.push(serde_json::from_str::<Op>(&payload)?);
            ultimo = seq;
        }

        // `tem_mais` sai de uma pergunta ao banco, e nao de `ops.len() ==
        // limite`. O atalho erra justamente no caso em que o lote acaba exato:
        // o cliente veria "tem mais", pediria de novo e receberia vazio. Uma
        // rodada inteira desperdicada, e no iPhone isso e radio ligado a toa.
        let tem_mais = self
            .conexao
            .query_row(
                "SELECT 1 FROM sync_log WHERE seq > ?1 LIMIT 1",
                [ultimo],
                |_| Ok(()),
            )
            .optional()?
            .is_some();

        Ok(Lote {
            ops,
            proximo_cursor: ultimo.to_string(),
            tem_mais,
        })
    }

    /// Quantas operacoes o hub guarda. Diagnostico, nao contrato.
    pub fn total(&self) -> Resultado<usize> {
        let total: i64 = self
            .conexao
            .query_row("SELECT COUNT(*) FROM sync_log", [], |linha| linha.get(0))?;
        Ok(total as usize)
    }
}
