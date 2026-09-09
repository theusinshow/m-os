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
    /// Entradas da fila que ja nao tinham trabalho: a entidade virou linha em
    /// algum momento e ninguem apagou o bilhete.
    ///
    /// Nao e erro nem conquista — e faxina. Aparece no numero para o log poder
    /// explicar uma fila que encolheu sem nada ter sido materializado.
    pub limpas: usize,
    /// Partes de mensagem que foram substituidas e continuavam segurando a vaga
    /// da substituta. Ver [`SqliteStorage::soltar_lugar_disputado`].
    pub desocupadas: usize,
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
    /// **O lugar esta ocupado.** Unicidade ou chave primaria: ja existe uma
    /// linha equivalente ali. Nada que chegue depois muda isso, e a entidade
    /// SAI DA FILA.
    ///
    /// Chave estrangeira nao entra aqui — ela e a primeira metade, e nao a
    /// segunda. Ver `ErrorCode::Conflict`.
    ///
    /// Tratar as duas como a mesma coisa era um defeito real, e ele estava
    /// vivo: seis `message_part` de 04/09/2026 batiam em
    /// `UNIQUE constraint failed: message_parts.message_id, message_parts.seq`
    /// e voltavam para a fila a cada abertura do app. A varredura reimprimia
    /// seis linhas de erro por abertura para um trabalho que nunca poderia
    /// terminar.
    ///
    /// **Correcao de 09/09/2026, com o banco da VPS na mao.** A nota anterior
    /// dizia que eram "ids duplicados da mesma parte logica, nascidos em dois
    /// aparelhos", e que "o conteudo estava na tela o tempo todo". As duas
    /// coisas estavam erradas. Eram a parte VELHA e a que a substituiu — o
    /// `finish_message` troca as partes por outras, com ids novos, no mesmo
    /// `(message_id, seq)` — e o que estava na tela era o texto ANTIGO. Sair da
    /// fila calava o log e congelava o erro.
    ///
    /// Por isso [`Self::soltar_lugar_disputado`] roda ANTES: ela libera a vaga,
    /// e ai o abandono volta a significar o que a separacao queria dizer.
    ///
    /// O caso que esta separacao aceita perder, e vale estar escrito: se um dia
    /// a linha ocupante for apagada DE VERDADE, a abandonada nao volta sozinha
    /// — ela ja saiu da fila. O apagamento no M/OS e logico (`trashed` mantem a
    /// linha) em toda tabela menos `message_parts`, e la e a varredura acima que
    /// responde.
    ///
    /// # A faxina que vem antes
    ///
    /// A fila so era limpa quando a varredura PROCESSAVA a entidade. Uma que
    /// virasse linha por outro caminho — a rodada seguinte de sync, por
    /// exemplo — deixava o bilhete para tras: trabalho ja feito, guardado para
    /// sempre. Havia um assim no banco do dono, de 05/09/2026.
    ///
    /// Entao a varredura comeca soltando da fila tudo que nao esta entre os
    /// candidatos. Nao ha o que perder nisso: candidato E a definicao de
    /// "existe na sombra e nao existe na tabela", que e a unica coisa que esta
    /// funcao sabe fazer. O que nao e candidato ou ja tem linha, ou nao tem
    /// sombra — e nos dois casos o bilhete nao aponta para trabalho nenhum.
    pub fn reparar_materializacao(&self) -> Result<Reparo, CoreError> {
        // Antes de tudo: liberar a vaga que duas partes disputam. Sem isso a
        // segunda seria abandonada com o texto VELHO congelado na tela.
        let mut reparo = Reparo {
            desocupadas: self.soltar_lugar_disputado()?,
            ..Reparo::default()
        };

        let candidatos = crate::sync_projecao::ProjecaoSqlite::entidades_sem_linha(self)?;
        reparo.examinadas = candidatos.len();
        reparo.limpas = self.soltar_fila_sem_trabalho(&candidatos)?;
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

    /// Solta da fila tudo que nao esta entre os candidatos.
    ///
    /// Devolve quantos bilhetes foram soltos. Ver a nota de
    /// [`Self::reparar_materializacao`] para o porque de isso ser seguro.
    fn soltar_fila_sem_trabalho(
        &self,
        candidatos: &[(String, uuid::Uuid)],
    ) -> Result<usize, CoreError> {
        let conexao = self.escrita()?;
        let na_fila: Vec<(String, String)> = {
            let mut consulta = conexao
                .prepare("SELECT entity_kind, entity_id FROM sync_pendentes")
                .map_err(crate::map_sql_error)?;
            let linhas = consulta
                .query_map([], |linha| Ok((linha.get(0)?, linha.get(1)?)))
                .map_err(crate::map_sql_error)?;
            let mut achados = Vec::new();
            for linha in linhas {
                achados.push(linha.map_err(crate::map_sql_error)?);
            }
            achados
        };

        let mut soltos = 0;
        for (kind, id) in na_fila {
            let ainda_tem_trabalho = candidatos
                .iter()
                .any(|(candidato, alvo)| candidato == &kind && alvo.to_string() == id);
            if ainda_tem_trabalho {
                continue;
            }
            conexao
                .execute(
                    "DELETE FROM sync_pendentes WHERE entity_kind = ?1 AND entity_id = ?2",
                    rusqlite::params![kind, id],
                )
                .map_err(crate::map_sql_error)?;
            soltos += 1;
        }
        Ok(soltos)
    }

    /// Libera o `(message_id, seq)` que duas partes de mensagem disputam.
    ///
    /// # Por que existe, e por que so para `message_part`
    ///
    /// `finish_message` troca as partes de uma mensagem: apaga as que estao la e
    /// insere outras, com ids novos, no MESMO `(message_id, seq)`. Ate o
    /// conserto em `conversation_repository.rs` a troca nao emitia `Delete`,
    /// entao a parte velha continuava viva na sombra — e como a 0010 exige
    /// `UNIQUE (message_id, seq)`, ela segurava a vaga com o conteudo ANTIGO
    /// enquanto a substituta nunca virava linha em aparelho nenhum.
    ///
    /// Aquele conserto impede que isso volte a acontecer. Ele nao desfaz o que
    /// ja aconteceu: seis pares assim estavam vivos na VPS em 09/09/2026, e a
    /// rodada parava neles a cada minuto.
    ///
    /// # Quem ganha a vaga, e por que nao e o HLC
    ///
    /// O ID, e nao o instante. O `backfill` inicial reemite toda linha
    /// existente com um instante NOVO — na VPS a parte velha ficou com `wallMs`
    /// maior que a substituta, porque era ela que tinha linha na hora do
    /// backfill. Ordenar por HLC ali escolheria exatamente o texto errado.
    ///
    /// `MessagePartId` e UUIDv7, e o substituto e sempre cunhado depois do que
    /// ele substitui. A ordem dos ids sobrevive ao backfill, e os dois
    /// aparelhos chegam a ela sem se falarem — que e a propriedade que o
    /// `SYNC.md` pede de todo desempate.
    ///
    /// # Por que `Delete`, e nao mudanca de campo
    ///
    /// A parte velha nao foi arquivada: ela deixou de existir quando a resposta
    /// fechou. Quem guarda o que foi dito e a mensagem, e ela continua inteira.
    fn soltar_lugar_disputado(&self) -> Result<usize, CoreError> {
        use std::collections::BTreeMap;

        let conexao = self.escrita()?;

        // (message_id, seq) -> ids vivos que o reivindicam.
        let mut vagas: BTreeMap<(String, i64), Vec<uuid::Uuid>> = BTreeMap::new();
        {
            let mut consulta = conexao
                .prepare(
                    "SELECT entity_id, estado FROM sync_state \
                     WHERE entity_kind = 'message_part'",
                )
                .map_err(crate::map_sql_error)?;
            let linhas = consulta
                .query_map([], |linha| {
                    Ok((linha.get::<_, String>(0)?, linha.get::<_, String>(1)?))
                })
                .map_err(crate::map_sql_error)?;
            for linha in linhas {
                let (id, estado) = linha.map_err(crate::map_sql_error)?;
                let Ok(id) = uuid::Uuid::parse_str(&id) else {
                    continue;
                };
                let Ok(estado) = serde_json::from_str::<mos_sync::EstadoDaEntidade>(&estado) else {
                    continue;
                };
                // Ja apagada nao disputa nada.
                if estado.deleted_at.is_some() {
                    continue;
                }
                let (Some(mensagem), Some(seq)) = (
                    estado
                        .campos
                        .get("messageId")
                        .and_then(|campo| campo.valor.as_str())
                        .map(str::to_owned),
                    estado
                        .campos
                        .get("seq")
                        .and_then(|campo| campo.valor.as_i64()),
                ) else {
                    continue;
                };
                vagas.entry((mensagem, seq)).or_default().push(id);
            }
        }

        let mut desocupadas = 0;
        for (_, mut donos) in vagas {
            if donos.len() < 2 {
                continue;
            }
            donos.sort_unstable();
            // O maior id fica com a vaga; o resto foi substituido.
            donos.pop();
            for substituida in donos {
                let transacao = conexao
                    .unchecked_transaction()
                    .map_err(crate::map_sql_error)?;
                transacao
                    .execute(
                        "DELETE FROM message_parts WHERE id = ?1",
                        rusqlite::params![substituida.to_string()],
                    )
                    .map_err(crate::map_sql_error)?;
                self.emitir(
                    &transacao,
                    mos_sync::EntityRef::new("message_part", substituida),
                    mos_sync::OpBody::Delete,
                )?;
                transacao.commit().map_err(crate::map_sql_error)?;
                desocupadas += 1;
            }
        }
        Ok(desocupadas)
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
        use mos_core::NewTask;

        let (storage, _guarda) = storage();

        // Uma Task que perde a linha mas mantem a sombra: candidata legitima.
        // A materializacao dela falha porque o Project que ela cita nunca
        // chegou — e essa e a falha que o tempo resolve.
        let tarefa = NewTask::create("Revisar a prancha", "", None).unwrap();
        let id = tarefa.id;
        storage.create_task(tarefa).unwrap();

        {
            let conexao = storage.escrita().unwrap();
            let estado: String = conexao
                .query_row(
                    "SELECT estado FROM sync_state WHERE entity_kind = 'task' AND entity_id = ?1",
                    rusqlite::params![id.to_string()],
                    |linha| linha.get(0),
                )
                .unwrap();
            let mut json: serde_json::Value = serde_json::from_str(&estado).unwrap();
            let carimbo = json["campos"]["title"]["at"].clone();
            // Um Project que nao existe: a chave estrangeira recusa, e a peca
            // que falta pode chegar na proxima rodada.
            json["campos"]["projectId"] = serde_json::json!({
                "valor": uuid::Uuid::now_v7().to_string(),
                "at": carimbo,
            });
            conexao
                .execute(
                    "UPDATE sync_state SET estado = ?1 \
                     WHERE entity_kind = 'task' AND entity_id = ?2",
                    rusqlite::params![json.to_string(), id.to_string()],
                )
                .unwrap();
            conexao
                .execute(
                    "DELETE FROM tasks WHERE id = ?1",
                    rusqlite::params![id.to_string()],
                )
                .unwrap();
        }

        let reparo = storage.reparar_materializacao().unwrap();

        assert_eq!(reparo.reparadas, 0);
        assert!(
            reparo.abandonadas.is_empty(),
            "falta de dependencia nao e recusa do banco: {:?}",
            reparo.abandonadas
        );
        assert_eq!(reparo.falharam.len(), 1, "{reparo:?}");
    }

    /// **A faxina.** Um bilhete na fila para algo que ja virou linha.
    ///
    /// Acontecia de verdade: a entidade materializava por outro caminho — a
    /// rodada seguinte de sync —, e a fila so era limpa quando a varredura
    /// PROCESSAVA aquela entidade. Como ela ja tinha linha, nunca virava
    /// candidata, e o bilhete ficava para sempre. Havia um assim no banco do
    /// dono, de 05/09/2026.
    #[test]
    fn bilhete_de_trabalho_ja_feito_e_solto_da_fila() {
        let (storage, _guarda) = storage();

        let projeto = NewProject::create("Quiosque", "", "").unwrap();
        let id = projeto.id;
        storage.create_project(projeto).unwrap();

        // A linha existe e a sombra tambem: nao ha nada a materializar. Mesmo
        // assim, um bilhete ficou para tras.
        storage
            .escrita()
            .unwrap()
            .execute(
                "INSERT INTO sync_pendentes \
                 (entity_kind, entity_id, tentativas, ultimo_erro, atualizado_em) \
                 VALUES ('project', ?1, 1, 'materializacao adiada', '2026-09-05T00:00:00Z')",
                rusqlite::params![id.to_string()],
            )
            .unwrap();

        let reparo = storage.reparar_materializacao().unwrap();

        assert_eq!(reparo.limpas, 1, "{reparo:?}");
        assert_eq!(reparo.reparadas, 0);
        assert!(reparo.falharam.is_empty());

        let sobrou: i64 = storage
            .connection
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM sync_pendentes", [], |linha| {
                linha.get(0)
            })
            .unwrap();
        assert_eq!(sobrou, 0);

        // E o Project continua inteiro: faxina nao encosta em dado.
        let vivo: i64 = storage
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM projects WHERE id = ?1",
                rusqlite::params![id.to_string()],
                |linha| linha.get(0),
            )
            .unwrap();
        assert_eq!(vivo, 1);
    }

    /// **O outro lado do mesmo defeito, medido na VPS em 2026-09-09.**
    ///
    /// Seis `message_part` batiam em `UNIQUE constraint failed` e nunca viravam
    /// linha. O teste acima os tira da fila para o log parar de repetir — mas
    /// tirar da fila nao era a resposta inteira, e a nota dele estava errada
    /// sobre a causa: nao eram "ids duplicados da mesma parte logica". Eram a
    /// parte VELHA e a que a substituiu.
    ///
    /// `finish_message` apaga as partes e insere outras, com ids novos, no mesmo
    /// `(message_id, seq)`. Antes do conserto em `conversation_repository.rs` a
    /// velha nao ganhava `Delete`, entao ela segurava a vaga com o conteudo
    /// ANTIGO e a nova ficava invisivel para sempre. Abandonar sem liberar a
    /// vaga congela o texto errado na tela.
    ///
    /// Quem desempata e o ID, e nao o HLC. O `backfill` inicial recarimba toda
    /// linha existente com instante novo — medido na VPS: a parte velha ficou
    /// com `wallMs` MAIOR que a que a substituiu. O `MessagePartId` e UUIDv7, e
    /// o substituto e sempre cunhado depois: a ordem dos ids sobrevive ao
    /// backfill, e os dois aparelhos chegam a ela sem se falarem.
    #[test]
    fn a_parte_substituida_solta_o_lugar_para_a_que_a_substituiu() {
        use mos_core::ConversationRepository;

        let (storage, _guarda) = storage();

        let conversa = storage
            .create_conversation(mos_core::NewConversation::create())
            .unwrap();
        let mensagem = storage
            .append_message(mos_core::NewMessage::user(conversa.id, "e a laje?").unwrap())
            .unwrap();
        let velha = mensagem.parts[0].id.as_uuid();

        // A que substituiu: id maior, mesmo lugar, conteudo novo — e sem linha,
        // que e o retrato de quem chegou pelo sync e nao coube.
        let nova = uuid::Uuid::now_v7();
        assert!(nova > velha, "o teste depende da ordem dos UUIDv7");
        {
            let conexao = storage.escrita().unwrap();
            let estado: String = conexao
                .query_row(
                    "SELECT estado FROM sync_state                      WHERE entity_kind = 'message_part' AND entity_id = ?1",
                    rusqlite::params![velha.to_string()],
                    |linha| linha.get(0),
                )
                .unwrap();
            let mut json: serde_json::Value = serde_json::from_str(&estado).unwrap();
            json["campos"]["payload"]["valor"] =
                serde_json::json!(r#"{"text":"a laje ficou 12 mil"}"#);
            json["campos"]["searchText"]["valor"] = serde_json::json!("a laje ficou 12 mil");
            conexao
                .execute(
                    "INSERT INTO sync_state (entity_kind, entity_id, estado, updated_at)                      VALUES ('message_part', ?1, ?2, ?3)",
                    rusqlite::params![
                        nova.to_string(),
                        json.to_string(),
                        "2026-09-09T13:27:00Z"
                    ],
                )
                .unwrap();
        }

        let reparo = storage.reparar_materializacao().unwrap();

        assert!(
            reparo.abandonadas.is_empty(),
            "abandonar congela o texto velho na tela: {:?}",
            reparo.abandonadas
        );
        assert!(reparo.falharam.is_empty(), "falhou: {:?}", reparo.falharam);

        let dono_do_lugar = storage.messages(conversa.id).unwrap()[0].parts[0].id.as_uuid();
        assert_eq!(
            dono_do_lugar, nova,
            "a parte velha continuou segurando a vaga"
        );

        // E a velha some da sombra tambem: senao a proxima varredura a traz de
        // volta e as duas voltam a disputar. `deletedAt` e um Hlc, e nao um
        // texto — vivo e exatamente `null`.
        let sombra: String = storage
            .connection
            .lock()
            .unwrap()
            .query_row(
                "SELECT estado FROM sync_state                  WHERE entity_kind = 'message_part' AND entity_id = ?1",
                rusqlite::params![velha.to_string()],
                |linha| linha.get(0),
            )
            .unwrap();
        let sombra: mos_sync::EstadoDaEntidade = serde_json::from_str(&sombra).unwrap();
        assert!(
            sombra.deleted_at.is_some(),
            "a parte velha continuou viva na sombra: o proximo sync a traria de volta"
        );
    }
}
