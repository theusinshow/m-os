//! A passagem unica do que ja existia antes do sync.
//!
//! # Por que ele precisa existir
//!
//! A operacao so nasce na mesma transacao da mudanca (`sync_emit.rs`). Isso e
//! deliberado e esta certo — e tambem significa que ligar a sincronizacao nao
//! move NADA do que ja estava no banco. Um M/OS com dois anos de historico
//! ligaria o sync e mandaria para o outro aparelho apenas o que fosse tocado
//! dali em diante.
//!
//! # Por que ele nao e uma migration `.sql`
//!
//! As migrations deste crate sao SQL puro, e SQL nao gera UUID nem sabe o
//! relogio logico. Mais fundo que isso: o backfill precisa do HLC, e o HLC so
//! existe depois de `habilitar_sync` — antes disso o dispositivo nem tem
//! identidade. Uma migration roda cedo demais para este trabalho.
//!
//! # Por que ele reusa o `Mapa`
//!
//! O mapa ja diz, por tipo, qual tabela e quais colunas viajam. Escrever aqui
//! uma segunda lista das mesmas colunas seria a TERCEIRA copia da mesma verdade
//! — e a divergencia entre duas copias e exatamente o defeito que esta spec
//! veio consertar. O que o backfill le e o que a emissao emitiria.

use mos_core::CoreError;
use rusqlite::{params, OptionalExtension};

use crate::{map_sql_error, SqliteStorage};

/// A geracao da cobertura ja aplicada neste banco.
const MARCA: &str = "sync_backfill_geracao";

/// A marca de antes, quando isto era um booleano.
///
/// Quem a tem passou pela cobertura de doze tipos, e portanto esta na geracao 1
/// — e precisa re-emitir. Ler a marca antiga em vez de ignora-la e o que faz os
/// bancos que ja existem atravessarem sem ninguem apagar nada a mao.
const MARCA_ANTIGA: &str = "sync_backfill_v1";

/// A geracao da cobertura ATUAL.
///
/// Sobe quando `sync_cobertura.rs` passa a incluir tipos que antes nao
/// atravessavam — e e isso que faz o backfill rodar de novo em quem ja tinha
/// passado por ele. A geracao 1 cobria doze tipos; a 2 cobre vinte e seis.
///
/// Um teste em `sync_cobertura.rs` falha se a lista mudar sem este numero
/// subir. Sem ele, a cobertura cresce em silencio e o dado velho fica parado
/// num PC so — que foi exatamente o que aconteceu entre a v1 e a v0.3.4.
pub(crate) const GERACAO_ATUAL: u32 = 2;

/// Os tipos, em ORDEM DE DEPENDENCIA.
///
/// A ordem nao e estetica. O destino aplica as operacoes e tenta materializar; o
/// `project_tracking` que chegasse antes do `project` bateria na chave
/// estrangeira, e a linha seria recusada — o mesmo estrago que
/// `sync_projecao.rs` descreve para `academic_subjects.semester_id`.
///
/// A projecao tem retentativa para o que chega fora de ordem, mas depender dela
/// aqui seria escolher o caminho ruim de proposito quando o certo custa uma
/// lista ordenada.
const ORDEM: &[&str] = &[
    // Sem pai nenhum.
    "tracking_settings",
    "client",
    "workspace",
    "project",
    "capture",
    "resource",
    "task",
    "reminder",
    // Dependem de project.
    "project_tracking",
    "time_entry",
    // O diario.
    "daily_session",
    "daily_objective",
    "daily_reflection",
    "weekly_review",
    // A conversa: cada um depende do anterior.
    "conversation",
    "message",
    "message_part",
    // O academico, do semestre para baixo.
    "academic_semester",
    "academic_subject",
    "academic_assignment",
    "academic_exam",
    "academic_study_session",
    "academic_provider_subject_fact",
];

impl SqliteStorage {
    /// Enfileira, uma vez so, tudo que ja existe neste banco.
    ///
    /// Devolve quantas operacoes foram enfileiradas. Zero significa "ja tinha
    /// passado" ou "banco vazio", e as duas sao respostas boas.
    ///
    /// Idempotente pela marca em `app_metadata`, gravada na MESMA transacao das
    /// operacoes: uma queda no meio nao deixa metade enfileirada e marcada como
    /// feita — ou passou inteiro, ou nao passou.
    pub fn backfill_do_sync(&self) -> Result<usize, CoreError> {
        if !self.sync_ligado() {
            return Err(crate::sync_emit::erro_de_sync(
                "O backfill precisa da sincronizacao ligada: sem o relogio deste \
                 dispositivo nao ha instante para carimbar as operacoes.",
            ));
        }

        let connection = self.escrita()?;
        let transacao = connection.unchecked_transaction().map_err(map_sql_error)?;

        // A geracao ja aplicada. Sem marca nenhuma o banco esta na geracao 0 e
        // passa inteiro; com a marca ANTIGA ele esta na 1, e re-emite o que a
        // cobertura passou a incluir desde entao.
        let gravada: Option<String> = transacao
            .query_row(
                "SELECT value FROM app_metadata WHERE key = ?1",
                params![MARCA],
                |linha| linha.get(0),
            )
            .optional()
            .map_err(map_sql_error)?;

        let geracao: u32 = match gravada.and_then(|valor| valor.parse().ok()) {
            Some(numero) => numero,
            None => {
                let antiga: bool = transacao
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM app_metadata WHERE key = ?1)",
                        params![MARCA_ANTIGA],
                        |linha| linha.get(0),
                    )
                    .map_err(map_sql_error)?;
                u32::from(antiga)
            }
        };

        if geracao >= GERACAO_ATUAL {
            return Ok(0);
        }

        let mut enfileiradas = 0;
        for kind in ORDEM {
            enfileiradas += crate::sync_projecao::enfileirar_tabela(self, &transacao, kind)?;
        }

        // A geracao, e nao a contagem: quantas linhas passaram e curiosidade, e
        // qual cobertura ja passou e a pergunta que a proxima abertura faz.
        transacao
            .execute(
                "INSERT INTO app_metadata (key, value) VALUES (?1, ?2) \
                 ON CONFLICT(key) DO UPDATE SET value = ?2",
                params![MARCA, GERACAO_ATUAL.to_string()],
            )
            .map_err(map_sql_error)?;
        transacao.commit().map_err(map_sql_error)?;
        Ok(enfileiradas)
    }
}

#[cfg(test)]
mod tests {
    use mos_core::{NewProject, NewTimeEntry, TimeTrackingRepository, WorkRepository};
    use mos_sync::DeviceRepository;

    use crate::SqliteStorage;

    fn storage() -> (SqliteStorage, tempfile::TempDir) {
        let pasta = tempfile::tempdir().unwrap();
        let storage =
            SqliteStorage::open(pasta.path().join("mos.db"), pasta.path().join("backups")).unwrap();
        (storage, pasta)
    }

    fn ligar(storage: &SqliteStorage) {
        let dispositivo = storage
            .este_dispositivo("teste", "windows", "0.0.0")
            .unwrap();
        storage.habilitar_sync(dispositivo.id).unwrap();
    }

    fn ops_na_fila(storage: &SqliteStorage) -> Vec<(String, i64)> {
        let conexao = storage.connection.lock().unwrap();
        let mut consulta = conexao
            .prepare("SELECT entity_kind, COUNT(*) FROM sync_outbox GROUP BY entity_kind")
            .unwrap();
        let linhas = consulta
            .query_map([], |linha| {
                Ok((linha.get::<_, String>(0)?, linha.get::<_, i64>(1)?))
            })
            .unwrap();
        linhas.map(|linha| linha.unwrap()).collect()
    }

    /// O caso real: o M/OS existia antes do sync, e o dono liga agora.
    #[test]
    fn o_que_existia_antes_do_sync_entra_na_fila() {
        let (storage, _guarda) = storage();

        // Escrito com o sync DESLIGADO — nada emite, que e o estado de qualquer
        // banco anterior a esta feature.
        let projeto = NewProject::create("Rancho Queimado", "", "").unwrap();
        let id_projeto = projeto.id;
        storage.create_project(projeto).unwrap();
        storage
            .create_time_entry(NewTimeEntry {
                project_id: id_projeto,
                started_at: time::OffsetDateTime::now_utc(),
                ended_at: None,
                duration_seconds: 3_600,
                idle_seconds: 0,
                description: String::from("desenho"),
                activity_type: mos_core::ActivityType::Drawing,
                billable: true,
                hourly_rate_snapshot_cents: 12_000,
                source: mos_core::EntrySource::Timer,
            })
            .unwrap();
        assert!(
            ops_na_fila(&storage).is_empty(),
            "sync desligado nao devia ter emitido nada"
        );

        ligar(&storage);
        let enfileiradas = storage.backfill_do_sync().unwrap();

        let fila = ops_na_fila(&storage);
        assert!(enfileiradas >= 2, "o backfill enfileirou so {enfileiradas}");
        assert!(
            fila.iter().any(|(kind, _)| kind == "project"),
            "o projeto que ja existia ficou de fora: {fila:?}"
        );
        assert!(
            fila.iter().any(|(kind, _)| kind == "time_entry"),
            "a hora que ja existia ficou de fora: {fila:?}"
        );
    }

    /// Rodar duas vezes nao pode duplicar: a segunda passada acharia as mesmas
    /// linhas e enfileiraria tudo de novo, e o outro aparelho veria cada
    /// entidade nascer duas vezes.
    #[test]
    fn rodar_o_backfill_de_novo_nao_acrescenta_nada() {
        let (storage, _guarda) = storage();
        let projeto = NewProject::create("Rancho Queimado", "", "").unwrap();
        storage.create_project(projeto).unwrap();
        ligar(&storage);

        let primeira = storage.backfill_do_sync().unwrap();
        let fila_depois_da_primeira = ops_na_fila(&storage);
        let segunda = storage.backfill_do_sync().unwrap();

        assert!(primeira > 0, "a primeira passada nao enfileirou nada");
        assert_eq!(segunda, 0, "a segunda passada enfileirou {segunda}");
        assert_eq!(
            ops_na_fila(&storage),
            fila_depois_da_primeira,
            "a fila mudou entre as duas passadas"
        );
    }

    /// A armadilha que ja disparou uma vez.
    ///
    /// A marca era um booleano. Quando a cobertura cresceu de 12 para 26 tipos,
    /// quem ja tinha passado pelo backfill NUNCA re-emitiu o que passou a ser
    /// sincronizavel — e o dado velho ficou parado num PC so.
    #[test]
    fn geracao_menor_faz_o_backfill_rodar_de_novo() {
        let (storage, _guarda) = storage();
        let projeto = NewProject::create("Rancho Queimado", "", "").unwrap();
        storage.create_project(projeto).unwrap();
        ligar(&storage);

        assert!(
            storage.backfill_do_sync().unwrap() > 0,
            "a primeira passagem nao emitiu"
        );
        assert_eq!(
            storage.backfill_do_sync().unwrap(),
            0,
            "passou duas vezes na mesma geracao"
        );

        // O aparelho que parou na geracao anterior.
        storage
            .escrita()
            .unwrap()
            .execute(
                "INSERT INTO app_metadata (key, value) VALUES (?1, '1') \
                 ON CONFLICT(key) DO UPDATE SET value = '1'",
                rusqlite::params![super::MARCA],
            )
            .unwrap();

        assert!(
            storage.backfill_do_sync().unwrap() > 0,
            "a geracao nova nao re-emitiu: o dado velho ficaria parado"
        );
    }

    /// Quem vinha da marca antiga entra como geracao 1, e portanto re-emite.
    #[test]
    fn a_marca_antiga_conta_como_geracao_um() {
        let (storage, _guarda) = storage();
        let projeto = NewProject::create("Rancho Queimado", "", "").unwrap();
        storage.create_project(projeto).unwrap();
        ligar(&storage);
        storage
            .escrita()
            .unwrap()
            .execute(
                "INSERT INTO app_metadata (key, value) VALUES (?1, '140')",
                rusqlite::params![super::MARCA_ANTIGA],
            )
            .unwrap();

        assert!(
            storage.backfill_do_sync().unwrap() > 0,
            "quem tinha a marca antiga precisa re-emitir na geracao 2"
        );
    }

    /// Sem relogio nao ha instante para carimbar, e uma operacao sem instante
    /// nao se ordena contra as outras. Falhar dizendo isso e melhor que
    /// enfileirar algo que o merge nao sabe posicionar.
    #[test]
    fn o_backfill_recusa_rodar_com_o_sync_desligado() {
        let (storage, _guarda) = storage();
        let erro = storage.backfill_do_sync().unwrap_err();
        assert!(
            erro.message.contains("sincronizacao ligada"),
            "erro inesperado: {}",
            erro.message
        );
    }
}

/// Ensaio contra um banco REAL, fora da suite.
///
/// Ignorado por padrao: ele depende de um arquivo que so existe na maquina de
/// quem roda. Existe porque banco de teste tem tres linhas e banco de verdade
/// tem historico — colunas nulas, entidades orfas, dados de versoes antigas do
/// esquema. Um backfill que passa nos dois e um backfill testado.
///
/// ```text
/// MOS_BANCO_DE_PROVA=C:\caminho\m-os.db cargo test -p mos-storage-sqlite \
///     --lib ensaio_contra_banco_real -- --ignored --nocapture
/// ```
#[cfg(test)]
#[test]
#[ignore = "precisa de um banco real apontado por MOS_BANCO_DE_PROVA"]
fn ensaio_contra_banco_real() {
    use mos_sync::DeviceRepository;

    let Ok(caminho) = std::env::var("MOS_BANCO_DE_PROVA") else {
        panic!("aponte MOS_BANCO_DE_PROVA para uma COPIA do banco");
    };
    let banco = std::path::PathBuf::from(caminho);
    let backups = banco.parent().unwrap().join("backups");
    let storage = crate::SqliteStorage::open(banco, backups).unwrap();

    let antes: i64 = {
        let conexao = storage.connection.lock().unwrap();
        conexao
            .query_row("SELECT COUNT(*) FROM sync_outbox", [], |l| l.get(0))
            .unwrap()
    };

    let dispositivo = storage
        .este_dispositivo("prova", "windows", "0.0.0")
        .unwrap();
    storage.habilitar_sync(dispositivo.id).unwrap();
    let enfileiradas = storage.backfill_do_sync().unwrap();

    let (linhas, ops): (Vec<(String, i64)>, Vec<mos_sync::Op>) = {
        let conexao = storage.connection.lock().unwrap();
        let mut consulta = conexao
            .prepare(
                "SELECT entity_kind, COUNT(*) FROM sync_outbox \
                 WHERE status = 'pending' GROUP BY entity_kind ORDER BY 2 DESC",
            )
            .unwrap();
        let contagem: Vec<(String, i64)> = consulta
            .query_map([], |l| Ok((l.get(0)?, l.get(1)?)))
            .unwrap()
            .map(|l| l.unwrap())
            .collect();
        let mut cargas = conexao
            .prepare("SELECT payload FROM sync_outbox WHERE status = 'pending'")
            .unwrap();
        let operacoes: Vec<mos_sync::Op> = cargas
            .query_map([], |l| l.get::<_, String>(0))
            .unwrap()
            .map(|p| serde_json::from_str(&p.unwrap()).unwrap())
            .collect();
        (contagem, operacoes)
    };

    println!("\n=== BACKFILL CONTRA O BANCO REAL ===");
    println!("fila antes: {antes}");
    println!("enfileiradas agora: {enfileiradas}");
    println!("--- pendentes por tipo ---");
    for (tipo, quantas) in &linhas {
        println!("  {tipo:34} {quantas:>5}");
    }
    assert!(enfileiradas > 0, "o backfill nao enfileirou nada");

    // A prova que importa: as operacoes viram LINHA num banco vazio, que e o
    // que o outro PC faz ao puxar. Contar a fila prova que saiu; so isto prova
    // que chegou.
    let vazio = tempfile::tempdir().unwrap();
    let outro =
        crate::SqliteStorage::open(vazio.path().join("mos.db"), vazio.path().join("backups"))
            .unwrap();
    let seu_id = outro.este_dispositivo("outro", "windows", "0.0.0").unwrap();
    outro.habilitar_sync(seu_id.id).unwrap();

    {
        use mos_sync::Projecao;
        let mut projecao = crate::sync_projecao::ProjecaoSqlite::nova(&outro);
        for op in &ops {
            let base = projecao.estado_de(op);
            let estado = mos_sync::aplicar(base, std::slice::from_ref(op)).estado;
            projecao.guardar(op, &estado).unwrap();
        }
        let faltas = projecao.resolver_pendentes();
        assert!(faltas.is_empty(), "nao materializou: {faltas:?}");
    }

    println!("--- linhas no banco VAZIO depois de aplicar ---");
    let conexao = outro.connection.lock().unwrap();
    let mut total = 0i64;
    for tabela in crate::sync_cobertura::SINCRONIZAVEIS {
        let n: i64 = conexao
            .query_row(&format!("SELECT COUNT(*) FROM {tabela}"), [], |l| l.get(0))
            .unwrap();
        if n > 0 {
            println!("  {tabela:34} {n:>5}");
            total += n;
        }
    }
    println!("  {:34} {total:>5}", "TOTAL");
    assert!(total > 100, "chegaram so {total} linhas do outro lado");
}
