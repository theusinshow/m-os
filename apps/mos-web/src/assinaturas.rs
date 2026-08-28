//! Onde as assinaturas de push moram — e por que num banco separado.
//!
//! # Isto nao e uma entidade do M/OS
//!
//! Uma assinatura de push nao e Task, nem Capture, nem Resource: e uma
//! propriedade DESTE aparelho, com validade curta e sem valor em nenhum outro.
//! Guarda-la no banco de dominio teria duas consequencias, e as duas ruins:
//!
//! 1. **Ela sincronizaria.** O endpoint do seu iPhone apareceria no banco do PC
//!    e no do hub, sem que nada nesses dois lugares tivesse o que fazer com ele.
//!    Um segredo que viaja para onde nao serve e um segredo a mais para vazar.
//! 2. **Ela entraria na cadeia de migrations** que o desktop tambem executa —
//!    o desktop ganharia uma tabela vazia para sempre, e uma migration a mais
//!    para dar errado numa maquina que nao tem push.
//!
//! Por isso: arquivo proprio, `push.db`, com duas tabelas e nenhuma pretensao.
//!
//! # A segunda tabela existe por um motivo especifico
//!
//! `avisados` guarda o que ja foi notificado. Sem ela, um lembrete vencido as
//! 9h viraria uma notificacao as 9h, outra as 9h01, outra as 9h02 — o laco roda
//! a cada minuto e o lembrete continua vencido ate voce resolve-lo. Notificacao
//! repetida nao e insistencia util: e o que ensina a desligar notificacao.

use rusqlite::Connection;

use crate::push::Assinatura;

#[derive(Debug, thiserror::Error)]
#[error("push.db: {0}")]
pub struct BancoError(String);

fn erro(causa: rusqlite::Error) -> BancoError {
    BancoError(causa.to_string())
}

pub struct Assinaturas {
    conexao: std::sync::Mutex<Connection>,
}

impl Assinaturas {
    pub fn abrir(caminho: &str) -> Result<Self, BancoError> {
        let conexao = Connection::open(caminho).map_err(erro)?;
        // `endpoint` e a chave primaria de proposito: reassinar no mesmo
        // aparelho devolve o MESMO endpoint, e sem essa restricao cada vez que
        // voce tocasse "ativar" nasceria uma linha nova — e a mesma notificacao
        // chegaria duas, tres, quatro vezes.
        conexao
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS assinaturas (
                     endpoint  TEXT PRIMARY KEY,
                     p256dh    TEXT NOT NULL,
                     auth      TEXT NOT NULL,
                     criada_em INTEGER NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS avisados (
                     chave TEXT PRIMARY KEY,
                     em    INTEGER NOT NULL
                 );",
            )
            .map_err(erro)?;
        Ok(Self {
            conexao: std::sync::Mutex::new(conexao),
        })
    }

    /// Grava uma assinatura, ou atualiza as chaves se o endpoint ja existir.
    ///
    /// O navegador pode rodar as chaves mantendo o endpoint. Ignorar o conflito
    /// deixaria o servidor cifrando com uma chave que o aparelho nao tem mais —
    /// e o sintoma disso e silencio, nao erro.
    pub fn salvar(&self, assinatura: &Assinatura, agora_ms: i64) -> Result<(), BancoError> {
        self.conexao
            .lock()
            .expect("mutex do push.db")
            .execute(
                "INSERT INTO assinaturas (endpoint, p256dh, auth, criada_em)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(endpoint) DO UPDATE SET p256dh = ?2, auth = ?3",
                rusqlite::params![
                    assinatura.endpoint,
                    assinatura.p256dh,
                    assinatura.auth,
                    agora_ms
                ],
            )
            .map_err(erro)?;
        Ok(())
    }

    pub fn todas(&self) -> Result<Vec<Assinatura>, BancoError> {
        let conexao = self.conexao.lock().expect("mutex do push.db");
        let mut consulta = conexao
            .prepare("SELECT endpoint, p256dh, auth FROM assinaturas ORDER BY criada_em")
            .map_err(erro)?;
        let linhas = consulta
            .query_map([], |linha| {
                Ok(Assinatura {
                    endpoint: linha.get(0)?,
                    p256dh: linha.get(1)?,
                    auth: linha.get(2)?,
                })
            })
            .map_err(erro)?;
        linhas.collect::<Result<Vec<_>, _>>().map_err(erro)
    }

    /// Apaga uma assinatura morta.
    ///
    /// Chamado quando o servico de push responde 404 ou 410. Insistir num
    /// endpoint que o fabricante ja declarou morto e uma ida a rede por minuto
    /// para receber sempre o mesmo nao.
    pub fn remover(&self, endpoint: &str) -> Result<(), BancoError> {
        self.conexao
            .lock()
            .expect("mutex do push.db")
            .execute("DELETE FROM assinaturas WHERE endpoint = ?1", [endpoint])
            .map_err(erro)?;
        Ok(())
    }

    pub fn quantas(&self) -> Result<usize, BancoError> {
        let conexao = self.conexao.lock().expect("mutex do push.db");
        let total: i64 = conexao
            .query_row("SELECT COUNT(*) FROM assinaturas", [], |linha| linha.get(0))
            .map_err(erro)?;
        Ok(total as usize)
    }

    /// Marca uma chave como avisada. Devolve `true` se foi a PRIMEIRA vez.
    ///
    /// Consulta e escrita na mesma chamada de proposito: separadas, duas
    /// passadas do laco poderiam ler "ainda nao avisou" antes de qualquer uma
    /// escrever, e a notificacao sairia duas vezes. `INSERT ... ON CONFLICT DO
    /// NOTHING` resolve isso no banco, que e onde a corrida acontece.
    pub fn avisar_uma_vez(&self, chave: &str, agora_ms: i64) -> Result<bool, BancoError> {
        let mudou = self
            .conexao
            .lock()
            .expect("mutex do push.db")
            .execute(
                "INSERT INTO avisados (chave, em) VALUES (?1, ?2)
                 ON CONFLICT(chave) DO NOTHING",
                rusqlite::params![chave, agora_ms],
            )
            .map_err(erro)?;
        Ok(mudou == 1)
    }

    /// Esquece avisos velhos.
    ///
    /// Sem isto a tabela cresce para sempre. Trinta dias e bem mais que o tempo
    /// em que um lembrete vencido ainda importa, e curto o bastante para o
    /// arquivo nao virar um problema silencioso daqui a um ano.
    pub fn esquecer_antes_de(&self, limite_ms: i64) -> Result<usize, BancoError> {
        let apagados = self
            .conexao
            .lock()
            .expect("mutex do push.db")
            .execute("DELETE FROM avisados WHERE em < ?1", [limite_ms])
            .map_err(erro)?;
        Ok(apagados)
    }

    /// So para teste: se uma chave ja foi avisada, sem marcar nada.
    #[cfg(test)]
    fn ja_avisou(&self, chave: &str) -> Result<bool, BancoError> {
        use rusqlite::OptionalExtension;
        let conexao = self.conexao.lock().expect("mutex do push.db");
        let achou: Option<i64> = conexao
            .query_row("SELECT 1 FROM avisados WHERE chave = ?1", [chave], |l| {
                l.get(0)
            })
            .optional()
            .map_err(erro)?;
        Ok(achou.is_some())
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn assinatura(endpoint: &str) -> Assinatura {
        Assinatura {
            endpoint: String::from(endpoint),
            p256dh: String::from("BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4"),
            auth: String::from("BTBZMqHH6r4Tts7J_aSIgg"),
        }
    }

    fn banco() -> Assinaturas {
        Assinaturas::abrir(":memory:").unwrap()
    }

    #[test]
    fn guarda_e_devolve() {
        let banco = banco();
        banco
            .salvar(&assinatura("https://push.exemplo/a"), 1)
            .unwrap();
        assert_eq!(
            banco.todas().unwrap(),
            vec![assinatura("https://push.exemplo/a")]
        );
    }

    /// Tocar "ativar notificacoes" duas vezes nao pode virar duas notificacoes.
    #[test]
    fn reassinar_o_mesmo_aparelho_nao_duplica() {
        let banco = banco();
        banco
            .salvar(&assinatura("https://push.exemplo/a"), 1)
            .unwrap();
        banco
            .salvar(&assinatura("https://push.exemplo/a"), 2)
            .unwrap();
        assert_eq!(banco.quantas().unwrap(), 1);
    }

    /// Chaves novas no mesmo endpoint tem que SUBSTITUIR as velhas — cifrar com
    /// a chave antiga produz silencio, nao erro.
    #[test]
    fn chaves_rodadas_substituem_as_antigas() {
        let banco = banco();
        banco
            .salvar(&assinatura("https://push.exemplo/a"), 1)
            .unwrap();
        let mut nova = assinatura("https://push.exemplo/a");
        nova.auth = String::from("outroSegredoDiferente");
        banco.salvar(&nova, 2).unwrap();
        assert_eq!(banco.todas().unwrap(), vec![nova]);
    }

    #[test]
    fn remover_apaga_a_assinatura_morta() {
        let banco = banco();
        banco
            .salvar(&assinatura("https://push.exemplo/a"), 1)
            .unwrap();
        banco
            .salvar(&assinatura("https://push.exemplo/b"), 1)
            .unwrap();
        banco.remover("https://push.exemplo/a").unwrap();
        assert_eq!(
            banco.todas().unwrap(),
            vec![assinatura("https://push.exemplo/b")]
        );
    }

    /// O teste que impede a notificacao por minuto.
    #[test]
    fn o_mesmo_aviso_so_sai_uma_vez() {
        let banco = banco();
        assert!(banco.avisar_uma_vez("lembrete:abc:1000", 1).unwrap());
        assert!(!banco.avisar_uma_vez("lembrete:abc:1000", 2).unwrap());
        assert!(banco.ja_avisou("lembrete:abc:1000").unwrap());
    }

    /// Um lembrete que se repete vence de novo — e a nova ocorrencia tem que
    /// avisar, porque a chave carrega o vencimento.
    #[test]
    fn a_proxima_ocorrencia_avisa_de_novo() {
        let banco = banco();
        assert!(banco.avisar_uma_vez("lembrete:abc:1000", 1).unwrap());
        assert!(banco.avisar_uma_vez("lembrete:abc:2000", 2).unwrap());
    }

    #[test]
    fn avisos_velhos_sao_esquecidos() {
        let banco = banco();
        banco.avisar_uma_vez("velho", 100).unwrap();
        banco.avisar_uma_vez("novo", 900).unwrap();
        assert_eq!(banco.esquecer_antes_de(500).unwrap(), 1);
        assert!(!banco.ja_avisou("velho").unwrap());
        assert!(banco.ja_avisou("novo").unwrap());
    }
}
