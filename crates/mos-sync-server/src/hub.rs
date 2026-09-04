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

            -- O retrato de cada aparelho, por familia.
            --
            -- Trocada inteira a cada batida, e nao acumulada: familia que sumiu
            -- do aparelho tem que sumir do hub, ou a tela mostraria conteudo que
            -- ele nao tem mais.
            CREATE TABLE IF NOT EXISTS manifestos (
                dispositivo TEXT NOT NULL,
                familia     TEXT NOT NULL,
                contagem    INTEGER NOT NULL,
                hash        TEXT NOT NULL,
                visto_em    TEXT NOT NULL,
                PRIMARY KEY (dispositivo, familia)
            );

            -- Quem esta na malha.
            --
            -- Separada do log de proposito: o log e o contrato, e esta tabela e
            -- metadado operacional. Apagar `aparelhos` inteira nao perde uma
            -- unica operacao — perde so a resposta para "quem sao voces", que os
            -- proprios aparelhos reconstroem na batida seguinte.
            CREATE TABLE IF NOT EXISTS aparelhos (
                id          TEXT PRIMARY KEY,
                nome        TEXT NOT NULL,
                plataforma  TEXT NOT NULL,
                versao      TEXT NOT NULL,
                contrato    INTEGER NOT NULL,
                visto_em    TEXT NOT NULL
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

/// O que um aparelho diz de si.
///
/// Nao e regra, e metadado: o hub grava e devolve, sem decidir nada com isso —
/// nenhuma operacao e recusada por versao, nenhum cliente e bloqueado. A
/// pergunta "quem esta na malha, e em que versao" nao tinha onde ser
/// respondida, e responde-la custou uma manha de investigacao em 02/09/2026.
#[derive(Debug, Clone)]
pub struct AparelhoRegistrado {
    /// O mesmo `DeviceId` que assina as operacoes: e ele que liga esta linha ao
    /// que aparece no log.
    pub id: String,
    pub nome: String,
    pub plataforma: String,
    pub versao: String,
    pub contrato: u32,
    pub visto_em: String,
}

impl Hub {
    /// A batida de um aparelho.
    ///
    /// `visto_em` e a hora do SERVIDOR, e nao a que o cliente mandou: relogio de
    /// cliente errado e comum, e um "visto ha tres dias" que na verdade foi
    /// agora manda a investigacao para o lado errado.
    pub fn registrar_aparelho(
        &mut self,
        aparelho: &AparelhoRegistrado,
        visto_em: &str,
    ) -> Resultado<()> {
        self.conexao.execute(
            "INSERT INTO aparelhos (id, nome, plataforma, versao, contrato, visto_em) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT(id) DO UPDATE SET nome = ?2, plataforma = ?3, versao = ?4, \
             contrato = ?5, visto_em = ?6",
            rusqlite::params![
                aparelho.id,
                aparelho.nome,
                aparelho.plataforma,
                aparelho.versao,
                aparelho.contrato,
                visto_em,
            ],
        )?;
        Ok(())
    }

    /// Quem esta na malha, o mais recente primeiro.
    pub fn aparelhos(&self) -> Resultado<Vec<AparelhoRegistrado>> {
        let mut consulta = self.conexao.prepare(
            "SELECT id, nome, plataforma, versao, contrato, visto_em FROM aparelhos \
             ORDER BY visto_em DESC",
        )?;
        let linhas = consulta.query_map([], |linha| {
            Ok(AparelhoRegistrado {
                id: linha.get(0)?,
                nome: linha.get(1)?,
                plataforma: linha.get(2)?,
                versao: linha.get(3)?,
                contrato: linha.get::<_, i64>(4)? as u32,
                visto_em: linha.get(5)?,
            })
        })?;
        let mut aparelhos = Vec::new();
        for linha in linhas {
            aparelhos.push(linha?);
        }
        Ok(aparelhos)
    }
}

/// Uma familia, como o hub a guarda.
#[derive(Debug, Clone)]
pub struct FamiliaDoManifesto {
    pub familia: String,
    pub contagem: i64,
    pub hash: String,
}

impl Hub {
    /// Troca o manifesto inteiro do aparelho.
    ///
    /// Apaga e regrava numa transacao: familia que sumiu do aparelho tem que
    /// sumir do hub, e um `INSERT ... ON CONFLICT` deixaria a antiga para tras
    /// — a tela passaria a mostrar conteudo que aquele aparelho nao tem mais.
    pub fn guardar_manifesto(
        &mut self,
        dispositivo: &str,
        familias: &[FamiliaDoManifesto],
        visto_em: &str,
    ) -> Resultado<()> {
        let transacao = self.conexao.unchecked_transaction()?;
        transacao.execute(
            "DELETE FROM manifestos WHERE dispositivo = ?1",
            rusqlite::params![dispositivo],
        )?;
        for familia in familias {
            transacao.execute(
                "INSERT INTO manifestos (dispositivo, familia, contagem, hash, visto_em)                  VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    dispositivo,
                    familia.familia,
                    familia.contagem,
                    familia.hash,
                    visto_em
                ],
            )?;
        }
        transacao.commit()?;
        Ok(())
    }

    /// O manifesto de um aparelho.
    pub fn manifesto_de(&self, dispositivo: &str) -> Resultado<Vec<FamiliaDoManifesto>> {
        let mut consulta = self.conexao.prepare(
            "SELECT familia, contagem, hash FROM manifestos WHERE dispositivo = ?1              ORDER BY familia",
        )?;
        let linhas = consulta.query_map(rusqlite::params![dispositivo], |linha| {
            Ok(FamiliaDoManifesto {
                familia: linha.get(0)?,
                contagem: linha.get(1)?,
                hash: linha.get(2)?,
            })
        })?;
        let mut familias = Vec::new();
        for linha in linhas {
            familias.push(linha?);
        }
        Ok(familias)
    }
}

#[cfg(test)]
mod tests {
    use super::{AparelhoRegistrado, FamiliaDoManifesto, Hub};

    fn aparelho() -> AparelhoRegistrado {
        AparelhoRegistrado {
            id: "01a0279d-18e1-78c2-991f-9e894e7214be".into(),
            nome: "DESKTOP-634TJR1".into(),
            plataforma: "windows".into(),
            versao: "0.3.5".into(),
            contrato: 1,
            visto_em: String::new(),
        }
    }

    #[test]
    fn o_hub_guarda_quem_e_cada_aparelho() {
        let mut hub = Hub::em_memoria().unwrap();
        hub.registrar_aparelho(&aparelho(), "2026-09-03T12:00:00Z")
            .unwrap();

        let lista = hub.aparelhos().unwrap();
        assert_eq!(lista.len(), 1);
        assert_eq!(lista[0].nome, "DESKTOP-634TJR1");
        assert_eq!(lista[0].versao, "0.3.5");
        // A hora e a do SERVIDOR: a que o cliente mandou nao entra.
        assert_eq!(lista[0].visto_em, "2026-09-03T12:00:00Z");
    }

    #[test]
    fn a_batida_seguinte_atualiza_em_vez_de_duplicar() {
        let mut hub = Hub::em_memoria().unwrap();
        let mut aparelho = aparelho();
        hub.registrar_aparelho(&aparelho, "2026-09-03T12:00:00Z")
            .unwrap();
        aparelho.versao = "0.3.6".into();
        hub.registrar_aparelho(&aparelho, "2026-09-03T12:30:00Z")
            .unwrap();

        let lista = hub.aparelhos().unwrap();
        assert_eq!(lista.len(), 1, "a batida duplicou o aparelho");
        assert_eq!(lista[0].versao, "0.3.6");
        assert_eq!(lista[0].visto_em, "2026-09-03T12:30:00Z");
    }

    #[test]
    fn o_manifesto_e_trocado_inteiro_a_cada_batida() {
        let mut hub = Hub::em_memoria().unwrap();
        let dispositivo = "01a0279d-18e1-78c2-991f-9e894e7214be";

        hub.guardar_manifesto(
            dispositivo,
            &[
                FamiliaDoManifesto {
                    familia: "task".into(),
                    contagem: 17,
                    hash: "aa".into(),
                },
                FamiliaDoManifesto {
                    familia: "project".into(),
                    contagem: 10,
                    hash: "bb".into(),
                },
            ],
            "2026-09-04T12:00:00Z",
        )
        .unwrap();

        // A batida seguinte nao tem mais `project`: ele precisa SUMIR do hub,
        // ou a tela mostraria uma familia que o aparelho nao tem mais.
        hub.guardar_manifesto(
            dispositivo,
            &[FamiliaDoManifesto {
                familia: "task".into(),
                contagem: 19,
                hash: "cc".into(),
            }],
            "2026-09-04T12:05:00Z",
        )
        .unwrap();

        let guardado = hub.manifesto_de(dispositivo).unwrap();
        assert_eq!(guardado.len(), 1, "a familia antiga ficou para tras");
        assert_eq!(guardado[0].familia, "task");
        assert_eq!(guardado[0].contagem, 19);
        assert_eq!(guardado[0].hash, "cc");
    }
}
