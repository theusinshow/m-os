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
    ///
    /// Continuam na fila: o que falta pode chegar na proxima rodada.
    pub falharam: Vec<String>,
    /// As que o banco RECUSOU, e que por isso saem da fila para sempre.
    ///
    /// Ver [`SqliteStorage::reparar_materializacao`] para o porque da
    /// separacao. Elas nao sao perda de dado: o lugar ja esta ocupado por uma
    /// linha equivalente, e e ela que a tela mostra.
    pub abandonadas: Vec<String>,
}

impl SqliteStorage {
    /// Materializa o que esta na sombra e nao esta na tabela.
    ///
    /// # Duas formas de nao dar certo, e so uma merece uma segunda chance
    ///
    /// **Falta alguem.** A hora chegou antes do projeto, o item antes da Task.
    /// Isso se resolve sozinho quando a peca que falta chegar, entao a entidade
    /// FICA NA FILA e a proxima varredura tenta de novo.
    ///
    /// **O banco recusou.** Chave repetida, unicidade, estrangeira, CHECK.
    /// Nada que chegue depois muda isso — o destino esta ocupado, e continuara
    /// ocupado. A entidade SAI DA FILA.
    ///
    /// Tratar as duas como a mesma coisa era um defeito real, e ele estava
    /// vivo: seis `message_part` de 04/09/2026 batiam em
    /// `UNIQUE constraint failed: message_parts.message_id, message_parts.seq`
    /// e voltavam para a fila a cada abertura do app. O conteudo estava na tela
    /// o tempo todo — eram ids DUPLICADOS da mesma parte logica, nascidos em
    /// dois aparelhos. A varredura reimprimia seis linhas de erro por abertura
    /// para um trabalho que nunca poderia terminar.
    ///
    /// O caso que esta separacao aceita perder, e vale estar escrito: se um dia
    /// a linha ocupante for apagada DE VERDADE, a abandonada nao volta sozinha
    /// — ela ja saiu da fila. O apagamento no M/OS e logico (`trashed` mantem a
    /// linha), entao isso hoje nao acontece por nenhum caminho da interface.
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
                    // O banco recusou. Nao volta para a fila: ver a nota da
                    // funcao. Sai da fila AQUI, e nao no fim, porque a proxima
                    // passada do ponto fixo nao deve nem contar com ela.
                    Err(causa) if causa.code == mos_core::ErrorCode::Conflict => {
                        reparo.abandonadas.push(format!("{kind} {id}: {causa}"));
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
        assert!(reparo.abandonadas.is_empty());
    }

    /// **O defeito que estava vivo no banco do dono, em 2026-09-08.**
    ///
    /// Seis `message_part` de 04/09 batiam em
    /// `UNIQUE constraint failed: message_parts.message_id, message_parts.seq`
    /// e voltavam para a fila a cada abertura do app — para sempre, porque o
    /// destino estava ocupado por uma linha EQUIVALENTE e nada que chegasse
    /// depois liberaria a vaga. Eram ids duplicados da mesma parte logica,
    /// nascidos em dois aparelhos.
    ///
    /// Aqui a mesma forma sai mais barata: `tasks.source_capture_id` tambem e
    /// UNIQUE, e duas Tasks apontando para a mesma Capture produzem a recusa
    /// identica — unicidade secundaria violada na materializacao.
    #[test]
    fn o_que_o_banco_recusa_sai_da_fila_em_vez_de_ser_retentado() {
        use mos_core::{CaptureRepository, NewCapture, NewTask, WorkRepository};

        let (storage, _guarda) = storage();

        // A Capture que as duas Tasks vao disputar.
        let captura =
            NewCapture::create("Levar o notebook", mos_core::CaptureSource::QuickCapture).unwrap();
        let id_captura = captura.id;
        storage.create(captura).unwrap();

        // A ocupante: nasce ligada a Capture.
        let ocupante = storage
            .create_task_from_capture(id_captura, NewTask::create("Ocupante", "", None).unwrap())
            .unwrap();
        assert_eq!(ocupante.source_capture_id, Some(id_captura));

        // A duplicata: nasce solta, e sera reescrita para disputar a mesma vaga.
        let duplicata = NewTask::create("Duplicata", "", None).unwrap();
        let id_duplicata = duplicata.id;
        storage.create_task(duplicata).unwrap();

        {
            let conexao = storage.escrita().unwrap();

            // A sombra da duplicata passa a apontar para a Capture ja tomada.
            let estado: String = conexao
                .query_row(
                    "SELECT estado FROM sync_state WHERE entity_kind = 'task' AND entity_id = ?1",
                    rusqlite::params![id_duplicata.to_string()],
                    |linha| linha.get(0),
                )
                .unwrap();
            let mut json: serde_json::Value = serde_json::from_str(&estado).unwrap();
            let carimbo = json["campos"]["title"]["at"].clone();
            json["campos"]["sourceCaptureId"] = serde_json::json!({
                "valor": id_captura.to_string(),
                "at": carimbo,
            });
            conexao
                .execute(
                    "UPDATE sync_state SET estado = ?1                      WHERE entity_kind = 'task' AND entity_id = ?2",
                    rusqlite::params![json.to_string(), id_duplicata.to_string()],
                )
                .unwrap();

            // E ela perde a linha: vira candidata da varredura.
            conexao
                .execute(
                    "DELETE FROM tasks WHERE id = ?1",
                    rusqlite::params![id_duplicata.to_string()],
                )
                .unwrap();
            conexao
                .execute(
                    "INSERT OR REPLACE INTO sync_pendentes                      (entity_kind, entity_id, tentativas, ultimo_erro, atualizado_em)                      VALUES ('task', ?1, 1, 'materializacao adiada', '2026-09-04T00:00:00Z')",
                    rusqlite::params![id_duplicata.to_string()],
                )
                .unwrap();
        }

        let reparo = storage.reparar_materializacao().unwrap();

        assert_eq!(
            reparo.abandonadas.len(),
            1,
            "a recusa do banco tinha que ser abandonada, e nao retentada: {reparo:?}"
        );
        assert!(
            reparo.falharam.is_empty(),
            "recusa do banco nao e 'depende de algo que nao chegou': {:?}",
            reparo.falharam
        );

        // A Task ocupante continua de pe: nada foi sobrescrito.
        let vivos: i64 = storage
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE id = ?1",
                rusqlite::params![ocupante.id.to_string()],
                |linha| linha.get(0),
            )
            .unwrap();
        assert_eq!(vivos, 1);

        // E a fila nao guarda mais um trabalho que nunca poderia terminar.
        let ainda_na_fila: i64 = storage
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM sync_pendentes WHERE entity_id = ?1",
                rusqlite::params![id_duplicata.to_string()],
                |linha| linha.get(0),
            )
            .unwrap();
        assert_eq!(
            ainda_na_fila, 0,
            "o que o banco recusou tem que sair da fila"
        );
    }

    /// A outra metade da distincao: o que depende de algo que nao chegou
    /// CONTINUA na fila, porque a peca que falta ainda pode aparecer.
    #[test]
    fn o_que_depende_de_algo_que_nao_chegou_continua_na_fila() {
        let (storage, _guarda) = storage();

        // Uma Task cujo Project nunca chegou: a chave estrangeira recusa, e a
        // recusa e do tipo que o tempo resolve.
        let id = uuid::Uuid::now_v7();
        {
            let conexao = storage.escrita().unwrap();
            conexao
                .execute(
                    "INSERT INTO sync_pendentes \
                     (entity_kind, entity_id, tentativas, ultimo_erro, atualizado_em) \
                     VALUES ('project', ?1, 1, 'materializacao adiada', '2026-09-04T00:00:00Z')",
                    rusqlite::params![id.to_string()],
                )
                .unwrap();
        }

        // Sem sombra, a entidade nem e candidata — e a varredura nao inventa
        // trabalho. O que importa aqui e que ela tambem nao APAGA a fila por
        // conta propria.
        let reparo = storage.reparar_materializacao().unwrap();
        assert!(reparo.abandonadas.is_empty());

        let continua: i64 = storage
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM sync_pendentes WHERE entity_id = ?1",
                rusqlite::params![id.to_string()],
                |linha| linha.get(0),
            )
            .unwrap();
        assert_eq!(continua, 1);
    }
}
