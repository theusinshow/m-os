//! A varredura que faz aparecer o que chegou e nao apareceu.
//!
//! # Por que ela existe
//!
//! `sync_state` e a sombra de tudo que sincroniza — inclusive do que nasce
//! local, porque `emitir` chama `absorver_local` na mesma transacao
//! (`sync_emit.rs`). Entao uma entidade que esta na sombra e nao esta na tabela
//! de dominio significa uma coisa so: ela chegou e nao virou linha.
//!
//! Isso acontecia e ninguem via. A fila de pendentes era memoria, o cursor
//! avancava assim mesmo, e a abertura seguinte nao sabia que havia o que
//! consertar. A varredura fecha o buraco olhando o BANCO, e nao a fila — e e
//! por isso que ela conserta tambem os bancos que ja estao nesse estado hoje,
//! sem ninguem rodar diagnostico.
//!
//! # Por que na abertura, e nao a cada rodada
//!
//! Rodar a cada sincronizacao custaria uma varredura por rodada para achar,
//! quase sempre, nada. Na abertura ela custa uma vez e cobre o caso que
//! importa: o app que fechou com pendencia e voltou.

use mos_core::CoreError;
use serde::Serialize;

use crate::SqliteStorage;

/// O que a varredura encontrou.
#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reparo {
    /// Entidades na sombra que nao tinham linha.
    pub examinadas: usize,
    /// Quantas viraram linha agora.
    pub reparadas: usize,
    /// As que continuaram sem virar, com o motivo. Elas dependem de algo que
    /// nao chegou, e a mensagem e o que permite descobrir o que.
    pub falharam: Vec<String>,
}

impl SqliteStorage {
    /// Materializa o que esta na sombra e nao esta na tabela.
    pub fn reparar_materializacao(&self) -> Result<Reparo, CoreError> {
        let mut reparo = Reparo::default();

        let candidatos = crate::sync_projecao::ProjecaoSqlite::entidades_sem_linha(self)?;
        reparo.examinadas = candidatos.len();
        if candidatos.is_empty() {
            return Ok(reparo);
        }

        // Varias passadas, porque a dependencia pode estar entre os proprios
        // candidatos: a hora precisa do projeto, e os dois podem ter sumido.
        // Para quando uma passada inteira nao consegue nada — ponto fixo, e nao
        // um numero de tentativas, pela mesma razao de `resolver_pendentes`: a
        // profundidade da arvore e do esquema, e nao deste laco.
        //
        // A mensagem do erro fica FORA da fila e so entra em `falharam` no fim:
        // guardada na fila, ela faria duas tentativas do mesmo item parecerem
        // itens diferentes na comparacao de tamanho.
        let mut fila = candidatos;
        let mut motivos: Vec<String> = Vec::new();
        loop {
            let tentativa = std::mem::take(&mut fila);
            let antes = tentativa.len();
            motivos.clear();
            for (kind, id) in tentativa {
                match crate::sync_projecao::ProjecaoSqlite::materializar_avulso(self, &kind, id) {
                    Ok(()) => {
                        reparo.reparadas += 1;
                        self.esquecer_pendente(&kind, id);
                    }
                    Err(causa) => {
                        motivos.push(format!("{kind} {id}: {causa}"));
                        fila.push((kind, id));
                    }
                }
            }
            if fila.len() == antes || fila.is_empty() {
                break;
            }
        }

        reparo.falharam = motivos;
        Ok(reparo)
    }

    /// Tira da fila o que acabou de virar linha.
    fn esquecer_pendente(&self, kind: &str, id: uuid::Uuid) {
        let Ok(conexao) = self.connection.lock() else {
            return;
        };
        let _ = conexao.execute(
            "DELETE FROM sync_pendentes WHERE entity_kind = ?1 AND entity_id = ?2",
            rusqlite::params![kind, id.to_string()],
        );
    }
}

#[cfg(test)]
mod tests {
    use mos_core::{NewProject, WorkRepository};
    use mos_sync::DeviceRepository;

    use crate::SqliteStorage;

    fn storage() -> (SqliteStorage, tempfile::TempDir) {
        let pasta = tempfile::tempdir().unwrap();
        let storage =
            SqliteStorage::open(pasta.path().join("mos.db"), pasta.path().join("backups")).unwrap();
        let dispositivo = storage
            .este_dispositivo("teste", "windows", "0.0.0")
            .unwrap();
        storage.habilitar_sync(dispositivo.id).unwrap();
        (storage, pasta)
    }

    /// O estado em que um banco pode estar HOJE: a entidade existe no banco de
    /// sincronizacao e nao existe na tabela que a tela le.
    ///
    /// A varredura tem que achar isso sozinha, sem depender da fila de
    /// pendentes — os bancos que ja estao assim perderam a fila quando o app
    /// fechou, e e justamente por isso que ela existe.
    #[test]
    fn a_varredura_materializa_o_que_ficou_para_tras() {
        let (storage, _guarda) = storage();

        let projeto = NewProject::create("Rancho Queimado", "", "").unwrap();
        let id = projeto.id;
        storage.create_project(projeto).unwrap();

        // Apaga SO a linha de dominio, deixando o estado do sync intacto: e o
        // retrato exato de "chegou e nao virou linha".
        storage
            .escrita()
            .unwrap()
            .execute(
                "DELETE FROM projects WHERE id = ?1",
                rusqlite::params![id.to_string()],
            )
            .unwrap();

        let reparo = storage.reparar_materializacao().unwrap();

        assert_eq!(reparo.reparadas, 1, "a varredura nao reconstruiu a linha");
        assert!(reparo.falharam.is_empty(), "falhou: {:?}", reparo.falharam);
        let voltou: i64 = storage
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM projects WHERE id = ?1",
                rusqlite::params![id.to_string()],
                |linha| linha.get(0),
            )
            .unwrap();
        assert_eq!(voltou, 1);
    }

    /// Banco saudavel: a varredura passa e nao mexe em nada.
    #[test]
    fn um_banco_alinhado_nao_e_tocado() {
        let (storage, _guarda) = storage();
        storage
            .create_project(NewProject::create("Quiosque", "", "").unwrap())
            .unwrap();

        let reparo = storage.reparar_materializacao().unwrap();
        assert_eq!(reparo.examinadas, 0);
        assert_eq!(reparo.reparadas, 0);
        assert!(reparo.falharam.is_empty());
    }
}
