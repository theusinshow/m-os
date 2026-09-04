//! O manifesto: o que este aparelho tem, em contagem e hash.
//!
//! # Por que sobre `sync_state`, e nao sobre as tabelas
//!
//! A projecao carimba `updated_at` com a hora de QUEM APLICOU. Dois aparelhos
//! com exatamente o mesmo conteudo tem bytes diferentes nas tabelas de dominio,
//! e um hash calculado ali acusaria divergencia onde nao ha nenhuma — o pior
//! defeito possivel numa ferramenta cujo proposito e dizer se estao iguais.
//!
//! `sync_state` guarda campo, valor e instante HLC. O instante nasce no
//! aparelho que ESCREVEU, e viaja junto: depois da convergencia ele e o mesmo
//! dos dois lados. E o unico lugar do banco onde "mesmo conteudo" significa
//! "mesmos bytes".
//!
//! # O que ele nao prova
//!
//! Que a entidade aparece na tela. Sombra igual com tabela vazia e exatamente o
//! defeito que a varredura de reparo conserta — por isso as duas coisas nascem
//! juntas nesta fase.

use mos_core::CoreError;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{map_sql_error, SqliteStorage};

/// Uma familia no manifesto.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinhaDoManifesto {
    pub familia: String,
    pub contagem: usize,
    /// SHA-256 dos pares `(id, estado)` ordenados por id, em hexadecimal.
    pub hash: String,
}

impl SqliteStorage {
    /// O retrato deste aparelho, por familia.
    ///
    /// Ordenado por id dentro de cada familia: sem ordem estavel, o mesmo
    /// conteudo daria hashes diferentes conforme o SQLite devolvesse as linhas.
    ///
    /// Familia sem nenhuma entidade nao entra — uma lista com vinte e seis zeros
    /// esconde as tres linhas que importam.
    pub fn manifesto(&self) -> Result<Vec<LinhaDoManifesto>, CoreError> {
        let conexao = self.connection.lock().map_err(crate::map_lock_error)?;
        let mut consulta = conexao
            .prepare(
                "SELECT entity_kind, entity_id, estado FROM sync_state \
                 ORDER BY entity_kind, entity_id",
            )
            .map_err(map_sql_error)?;
        let linhas = consulta
            .query_map([], |linha| {
                Ok((
                    linha.get::<_, String>(0)?,
                    linha.get::<_, String>(1)?,
                    linha.get::<_, String>(2)?,
                ))
            })
            .map_err(map_sql_error)?;

        let mut familias: Vec<LinhaDoManifesto> = Vec::new();
        let mut atual: Option<(String, usize, Sha256)> = None;
        for linha in linhas {
            let (kind, id, estado) = linha.map_err(map_sql_error)?;
            let trocou = atual.as_ref().is_none_or(|(k, _, _)| k != &kind);
            if trocou {
                if let Some((familia, contagem, hasher)) = atual.take() {
                    familias.push(LinhaDoManifesto {
                        familia,
                        contagem,
                        hash: format!("{:x}", hasher.finalize()),
                    });
                }
                atual = Some((kind, 0, Sha256::new()));
            }
            if let Some((_, contagem, hasher)) = atual.as_mut() {
                *contagem += 1;
                // Separadores entre id e estado, e entre linhas: sem eles, dois
                // conteudos diferentes poderiam concatenar nos mesmos bytes.
                hasher.update(id.as_bytes());
                hasher.update(b"\0");
                hasher.update(estado.as_bytes());
                hasher.update(b"\n");
            }
        }
        if let Some((familia, contagem, hasher)) = atual.take() {
            familias.push(LinhaDoManifesto {
                familia,
                contagem,
                hash: format!("{:x}", hasher.finalize()),
            });
        }
        Ok(familias)
    }
}

#[cfg(test)]
mod tests {
    use mos_core::{NewProject, WorkRepository};
    use mos_sync::DeviceRepository;

    use crate::sync_projecao::ProjecaoSqlite;
    use crate::SqliteStorage;

    fn storage(nome: &str) -> (SqliteStorage, tempfile::TempDir) {
        let pasta = tempfile::tempdir().unwrap();
        let storage =
            SqliteStorage::open(pasta.path().join("mos.db"), pasta.path().join("backups")).unwrap();
        let dispositivo = storage.este_dispositivo(nome, "windows", "0.0.0").unwrap();
        storage.habilitar_sync(dispositivo.id).unwrap();
        (storage, pasta)
    }

    /// A prova de que o hash serve: a MESMA entidade, criada de um lado e
    /// recebida do outro pelo caminho do sync, tem que dar o mesmo hash.
    ///
    /// Se o calculo fosse sobre as tabelas de dominio, este teste falharia — a
    /// projecao carimba `updated_at` com a hora de quem aplicou.
    #[test]
    fn dois_bancos_com_o_mesmo_conteudo_dao_o_mesmo_hash() {
        let (origem, _g1) = storage("origem");
        let (destino, _g2) = storage("destino");

        origem
            .create_project(NewProject::create("Rancho Queimado", "", "").unwrap())
            .unwrap();

        let ops: Vec<mos_sync::Op> = {
            let conexao = origem.connection.lock().unwrap();
            let mut consulta = conexao
                .prepare("SELECT payload FROM sync_outbox WHERE entity_kind = 'project'")
                .unwrap();
            let linhas = consulta
                .query_map([], |linha| linha.get::<_, String>(0))
                .unwrap();
            linhas
                .map(|payload| serde_json::from_str(&payload.unwrap()).unwrap())
                .collect()
        };

        {
            let mut projecao = ProjecaoSqlite::nova(&destino);
            for op in &ops {
                let base = mos_sync::Projecao::estado_de(&projecao, op);
                let estado = mos_sync::aplicar(base, std::slice::from_ref(op)).estado;
                mos_sync::Projecao::guardar(&mut projecao, op, &estado).unwrap();
            }
            assert!(projecao.resolver_pendentes().is_empty());
        }

        let hash_de = |s: &SqliteStorage| {
            s.manifesto()
                .unwrap()
                .into_iter()
                .find(|linha| linha.familia == "project")
                .expect("sem project no manifesto")
        };
        let da_origem = hash_de(&origem);
        let do_destino = hash_de(&destino);

        assert_eq!(da_origem.contagem, 1);
        assert_eq!(
            da_origem.hash, do_destino.hash,
            "mesmo conteudo, hash diferente: o manifesto acusaria divergencia inexistente"
        );
    }

    /// Conteudo diferente tem que dar hash diferente, ou o manifesto nao prova
    /// nada.
    #[test]
    fn conteudo_diferente_muda_o_hash() {
        let (um, _g1) = storage("um");
        let (outro, _g2) = storage("outro");
        um.create_project(NewProject::create("Rancho", "", "").unwrap())
            .unwrap();
        outro
            .create_project(NewProject::create("Quiosque", "", "").unwrap())
            .unwrap();

        let hash_de = |s: &SqliteStorage| {
            s.manifesto()
                .unwrap()
                .into_iter()
                .find(|linha| linha.familia == "project")
                .unwrap()
                .hash
        };
        assert_ne!(hash_de(&um), hash_de(&outro));
    }

    /// Familia vazia nao aparece.
    #[test]
    fn familia_vazia_nao_aparece() {
        let (storage, _guarda) = storage("vazio");
        assert!(storage.manifesto().unwrap().is_empty());
    }
}
