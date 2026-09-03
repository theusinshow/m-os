//! A guarda que impede o proximo CronoCAD.
//!
//! # O defeito que ela existe para tornar impossivel
//!
//! O CronoCAD gravava horas desde sempre e nunca emitiu uma operacao. Nao deu
//! erro, nao apareceu em log, nao quebrou teste — vinte e duas horas de trabalho
//! existiam num PC e nao no outro, e a unica forma de descobrir foi procurar.
//!
//! A causa nao foi distracao: nada no codigo obrigava alguem a decidir. Criar
//! uma tabela e escrever nela sao dois atos completos por si; sincroniza-la e um
//! terceiro que ninguem e forcado a considerar.
//!
//! # Como ela funciona
//!
//! Toda tabela do schema tem que aparecer em UMA das duas listas abaixo. Uma
//! tabela nova nao classificada derruba o teste, e a mensagem diz o que fazer.
//! A decisao continua sendo humana — o teste so recusa que ela seja esquecida.
//!
//! Nao ha lista "certa": `usage_requisicao` fora do sync esta tao correto quanto
//! `time_entries` dentro. O que estava errado era nao ter escolhido.

/// Tabelas cujo conteudo atravessa entre aparelhos.
///
/// Estar aqui obriga a existir entrada em `mapa_de` e emissao no repositorio.
/// Compilada sempre, mesmo so lida por teste: ela e o REGISTRO da decisao, e um
/// registro que some do binario de release e um registro que ninguem encontra
/// quando for procurar por que uma tabela sincroniza.
#[allow(dead_code)]
pub(crate) const SINCRONIZAVEIS: &[&str] = &[
    "tasks",
    "projects",
    "workspaces",
    "captures",
    "resources",
    "reminders",
    "academic_semesters",
    "academic_subjects",
    "academic_assignments",
    "academic_exams",
    "academic_study_sessions",
    "academic_subject_resources",
    "time_entries",
    "project_tracking",
    "conversations",
    "messages",
    "message_parts",
    "clients",
    "tracking_settings",
    "academic_provider_subject_facts",
    "daily_sessions",
    "daily_objectives",
    "daily_reflections",
    "weekly_reviews",
    // Tabelas de juncao: viajam como `relation`, e nao como tipo proprio.
    "resource_projects",
    "resource_workspaces",
    "project_workspaces",
];

/// Tabelas que ficam na maquina, com o motivo ao lado.
///
/// O motivo e obrigatorio: "nao sincroniza" sem porque e o mesmo esquecimento
/// de antes, so que por escrito.
#[allow(dead_code)]
pub(crate) const LOCAIS: &[(&str, &str)] = &[
    // --- a maquinaria do proprio sync ---
    ("sync_outbox", "a fila deste aparelho; replica-la faria cada PC reenviar o trabalho do outro em laco"),
    ("sync_state", "a sombra que guarda instante por campo, reconstruida a partir das operacoes"),
    ("sync_clock", "o relogio logico e o cursor DESTE dispositivo"),
    ("sync_conflicts", "o que este aparelho viu conflitar, para esta tela"),
    ("devices", "quem e cada instalacao; `is_this_device` nao faz sentido replicado"),
    // --- telemetria: descreve o que aconteceu NESTA maquina ---
    ("usage_requisicao", "consumo de API desta maquina; misturado, o relatorio somaria as duas como uma"),
    ("usage_janela", "idem"),
    ("usage_fonte", "idem"),
    ("activity_events", "que app abriu e quando, nesta maquina"),
    // --- o que descreve a maquina ---
    ("apps", "os aplicativos instalados AQUI"),
    ("app_workspaces", "junção de uma tabela local: ligar um app DESTA maquina a um workspace so vale aqui"),
    ("app_metadata", "idem"),
    ("monitored_apps", "o que esta maquina vigia"),
    ("active_timer", "cronometro em curso; replicado, dois PCs disputariam um so"),
    // --- o que guarda ARQUIVO em disco, e o hub so carrega campos ---
    ("meetings", "o audio mora em `audio_dir`; a linha sem o arquivo aponta para o nada"),
    ("meeting_segments", "idem"),
    ("meeting_analyses", "idem"),
    ("meeting_insights", "idem"),
    ("meeting_evidence", "idem"),
    ("meeting_transcript_index", "idem"),
    ("voice_notes", "idem"),
    ("ingestions", "o arquivo mora em `stored_path`"),
    // --- sessao e credencial ---
    ("academic_provider_state", "a sessao do Univirtus, com credencial implicita"),
    // --- layout: as duas telas sao diferentes ---
    ("radial_pins", "arranjo desta tela"),
    ("workspace_widget_layout", "idem"),
    ("workspace_hidden_widgets", "idem"),
    ("attention_notifications", "notificacao ja entregue nesta maquina"),
    // --- ainda nao decidido, e por isso listado ---
    ("academic_external_refs", "contabilidade do provedor NESTA maquina: `first_synced_at` e `last_synced_at` sao fatos locais, e as entidades que ela aponta ja viajam pelos ids proprios. Limite conhecido: se o Univirtus for conectado nos DOIS PCs, cada um importaria com ids proprios e criaria duplicata"),
    ("academic_material_urls", "cache datado de uma `temporary_url` do provedor: o endereco expira, e mandar um link vencido para o outro PC e pior que nao mandar"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SqliteStorage;

    /// Tabelas internas do FTS5 e do proprio SQLite, que ninguem classifica.
    fn e_ruido(nome: &str) -> bool {
        nome.starts_with("sqlite_")
            || nome.contains("_search")
            || nome.ends_with("_config")
            || nome.ends_with("_data")
            || nome.ends_with("_docsize")
            || nome.ends_with("_idx")
            || nome.ends_with("_content")
    }

    /// A cobertura da geracao 2, copiada a mao.
    ///
    /// A duplicacao E o mecanismo, e nao descuido: mudar `SINCRONIZAVEIS` sem
    /// tocar aqui quebra este teste, e a mensagem manda subir `GERACAO_ATUAL`.
    /// Sem isso a cobertura cresce em silencio, quem ja passou pelo backfill
    /// nunca re-emite o que foi incluido, e o dado velho fica parado num PC so
    /// — que foi exatamente o que aconteceu entre a geracao 1 e a v0.3.4.
    const COBERTURA_DA_GERACAO_2: &[&str] = &[
        "tasks",
        "projects",
        "workspaces",
        "captures",
        "resources",
        "reminders",
        "academic_semesters",
        "academic_subjects",
        "academic_assignments",
        "academic_exams",
        "academic_study_sessions",
        "academic_subject_resources",
        "time_entries",
        "project_tracking",
        "conversations",
        "messages",
        "message_parts",
        "clients",
        "tracking_settings",
        "academic_provider_subject_facts",
        "daily_sessions",
        "daily_objectives",
        "daily_reflections",
        "weekly_reviews",
        // Tabelas de juncao: viajam como `relation`, e nao como tipo proprio.
        "resource_projects",
        "resource_workspaces",
        "project_workspaces",
    ];

    #[test]
    fn mudar_a_cobertura_obriga_a_subir_a_geracao() {
        assert_eq!(
            SINCRONIZAVEIS, COBERTURA_DA_GERACAO_2,
            "A cobertura mudou. Suba `GERACAO_ATUAL` em `sync_backfill.rs` e              escreva aqui a lista nova, como `COBERTURA_DA_GERACAO_N` — sem              isso, quem ja passou pelo backfill nunca re-emite o que voce              acabou de incluir."
        );
        assert_eq!(
            crate::sync_backfill::GERACAO_ATUAL,
            2,
            "a geracao subiu; atualize a copia da cobertura neste teste"
        );
    }

    #[test]
    fn toda_tabela_do_schema_esta_classificada() {
        let pasta = tempfile::tempdir().unwrap();
        let storage =
            SqliteStorage::open(pasta.path().join("mos.db"), pasta.path().join("backups")).unwrap();
        let conexao = storage.connection.lock().unwrap();
        let mut consulta = conexao
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .unwrap();
        let tabelas: Vec<String> = consulta
            .query_map([], |linha| linha.get::<_, String>(0))
            .unwrap()
            .map(|linha| linha.unwrap())
            .filter(|nome| !e_ruido(nome))
            .collect();

        let nao_classificadas: Vec<&String> = tabelas
            .iter()
            .filter(|nome| {
                !SINCRONIZAVEIS.contains(&nome.as_str())
                    && !LOCAIS.iter().any(|(local, _)| local == nome)
            })
            .collect();

        assert!(
            nao_classificadas.is_empty(),
            "tabela sem decisao de sincronizacao: {nao_classificadas:?}\n\n\
             Toda tabela precisa estar em SINCRONIZAVEIS (e ai ganhar entrada em \
             `mapa_de` e emissao no repositorio) ou em LOCAIS (e ai dizer POR QUE \
             fica na maquina). Foi assim que o CronoCAD gravou 22 horas que nunca \
             sairam do PC: ninguem foi obrigado a escolher."
        );
    }

    /// Estar em SINCRONIZAVEIS sem entrada no mapa e prometer e nao entregar: a
    /// operacao viaja, o outro lado guarda o estado e nao materializa linha
    /// nenhuma.
    #[test]
    fn toda_tabela_sincronizavel_sabe_virar_linha() {
        let sem_mapa: Vec<&&str> = SINCRONIZAVEIS
            .iter()
            .filter(|tabela| !crate::sync_projecao::tem_mapa_para_tabela(tabela))
            .collect();

        assert!(
            sem_mapa.is_empty(),
            "tabela marcada como sincronizavel mas sem entrada em `mapa_de`: \
             {sem_mapa:?}\n\nA operacao viajaria e o outro lado guardaria o estado \
             sem nunca virar linha na tela."
        );
    }

    /// Uma tabela nao pode estar nas duas listas.
    #[test]
    fn nenhuma_tabela_esta_dos_dois_lados() {
        let nos_dois: Vec<&&str> = SINCRONIZAVEIS
            .iter()
            .filter(|tabela| LOCAIS.iter().any(|(local, _)| local == *tabela))
            .collect();
        assert!(
            nos_dois.is_empty(),
            "tabela em SINCRONIZAVEIS e em LOCAIS: {nos_dois:?}"
        );
    }
}
