//! O corpus da busca (F5 do MOS-OVERHAUL-PLAN, ADR-081).
//!
//! Uma massa sintética parecida com o uso real — projetos de engenharia,
//! Captures faladas, Tasks com espera e checklist — e perguntas com a resposta
//! esperada, escritas ANTES de mexer no ranking. Mede presença no top 5.
//!
//! O número mínimo abaixo é o piso depois da melhoria, fixado a partir do
//! baseline medido (ver ADR-081), e não escolhido depois do resultado para
//! declarar sucesso. Consultas exatas não podem regredir: elas têm piso próprio
//! de 100%.

use mos_core::{
    CaptureRepository, CaptureSource, EditTask, NewCapture, NewChecklistItem, NewProject, NewTask,
    SearchItem, SearchRequest, WorkRepository,
};
use mos_storage_sqlite::SqliteStorage;

struct Massa {
    storage: SqliteStorage,
    _dir: tempfile::TempDir,
}

fn chave(item: &SearchItem) -> String {
    match item {
        SearchItem::Capture { capture, .. } => format!("capture:{}", capture.content),
        SearchItem::Task { task, .. } => format!("task:{}", task.title),
        SearchItem::Project { project } => format!("project:{}", project.name),
        SearchItem::Workspace { workspace } => format!("workspace:{}", workspace.name),
        outro => format!("{outro:?}"),
    }
}

fn massa() -> Massa {
    let dir = tempfile::tempdir().unwrap();
    let backups = dir.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let storage = SqliteStorage::open(dir.path().join("mos.db"), &backups).unwrap();

    let jab = storage
        .create_project(
            NewProject::create(
                "Jaboticatuba",
                "Galpão industrial, fundações e drenagem",
                "",
            )
            .unwrap(),
        )
        .unwrap();
    let rat = storage
        .create_project(
            NewProject::create("Ratones", "Residência unifamiliar em concreto armado", "").unwrap(),
        )
        .unwrap();
    storage
        .create_project(
            NewProject::create("Caixa Econômica", "Reforma da agência centro", "").unwrap(),
        )
        .unwrap();

    let tarefas: &[(&str, &str, Option<mos_core::ProjectId>, &str, &[&str])] = &[
        (
            "Enviar bases para o Victor",
            "Plantas de fundação revisadas",
            Some(jab.id),
            "Victor",
            &["Exportar DWG", "Conferir cotas"],
        ),
        (
            "Revisar memorial de cálculo",
            "Memorial da caixa de drenagem pluvial",
            Some(jab.id),
            "",
            &["Revisar armaduras"],
        ),
        (
            "Dimensionar caixa de drenagem",
            "Caixa de passagem com tampa de concreto",
            Some(jab.id),
            "",
            &[],
        ),
        (
            "Modelar escadas no Allplan",
            "Escada helicoidal do bloco B",
            Some(rat.id),
            "",
            &["Importar arquitetura"],
        ),
        (
            "Orçamento de aço",
            "Quantitativo de armaduras das lajes",
            Some(rat.id),
            "Fornecedor Gerdau",
            &[],
        ),
        ("Pagar boleto do CREA", "Anuidade", None, "", &[]),
        (
            "Estudar para prova de Cálculo III",
            "Integrais triplas e jacobiano",
            None,
            "",
            &["Lista 4", "Lista 5"],
        ),
        (
            "Ligar para a prefeitura",
            "Alvará de construção do galpão",
            Some(jab.id),
            "Prefeitura",
            &[],
        ),
        (
            "Detalhar sapatas",
            "Sapatas isoladas do eixo 3",
            Some(jab.id),
            "",
            &["Armadura de arranque"],
        ),
        (
            "Atualizar cronograma da obra",
            "Cronograma físico-financeiro",
            Some(rat.id),
            "",
            &[],
        ),
        (
            "Responder e-mail da Caixa",
            "Documentação da reforma",
            None,
            "Gerente da agência",
            &[],
        ),
        ("Comprar notebook novo", "", None, "", &[]),
    ];
    for (titulo, descricao, projeto, espera, passos) in tarefas {
        let task = storage
            .create_task(NewTask::create(titulo, descricao, *projeto).unwrap())
            .unwrap();
        if !espera.is_empty() {
            let mut edit = EditTask::from_task(&task);
            edit.waiting_for = (*espera).to_owned();
            storage.update_task(task.id, edit).unwrap();
        }
        for passo in *passos {
            storage
                .add_checklist_item(NewChecklistItem::create(task.id, passo).unwrap())
                .unwrap();
        }
    }
    for fala in [
        "o Victor pediu as bases atualizadas até sexta",
        "lembrar de verificar a impermeabilização da caixa d'água",
        "ideia: usar blocos de concreto celular no muro",
        "decidido na reunião de Jaboticatuba: fundação em estacas hélice",
        "guardei o manual do Allplan na pasta de referências",
        "semana que vem visitar a obra da Ratones",
        "pesquisar software de orçamento",
    ] {
        storage
            .create(NewCapture::create(fala, CaptureSource::QuickCapture).unwrap())
            .unwrap();
    }
    Massa { storage, _dir: dir }
}

/// (consulta, esperado no top 5, exata?). `None` = a resposta certa é não achar.
const CORPUS: &[(&str, Option<&str>, bool)] = &[
    // exatas — não podem regredir
    (
        "Enviar bases",
        Some("task:Enviar bases para o Victor"),
        true,
    ),
    (
        "memorial de cálculo",
        Some("task:Revisar memorial de cálculo"),
        true,
    ),
    ("Allplan", Some("task:Modelar escadas no Allplan"), true),
    ("boleto", Some("task:Pagar boleto do CREA"), true),
    ("Jaboticatuba", Some("project:Jaboticatuba"), true),
    ("Ratones", Some("project:Ratones"), true),
    ("sapatas", Some("task:Detalhar sapatas"), true),
    (
        "cronograma",
        Some("task:Atualizar cronograma da obra"),
        true,
    ),
    ("prefeitura", Some("task:Ligar para a prefeitura"), true),
    ("notebook", Some("task:Comprar notebook novo"), true),
    ("drenagem", Some("task:Dimensionar caixa de drenagem"), true),
    (
        "concreto celular",
        Some("capture:ideia: usar blocos de concreto celular no muro"),
        true,
    ),
    (
        "Exportar DWG",
        Some("task:Enviar bases para o Victor"),
        true,
    ),
    (
        "Lista 4",
        Some("task:Estudar para prova de Cálculo III"),
        true,
    ),
    // acento, caixa e flexão
    ("calculo", Some("task:Revisar memorial de cálculo"), false),
    (
        "CAIXA DE DRENAGEM",
        Some("task:Dimensionar caixa de drenagem"),
        false,
    ),
    ("orcamento aco", Some("task:Orçamento de aço"), false),
    ("escada", Some("task:Modelar escadas no Allplan"), false),
    ("sapata", Some("task:Detalhar sapatas"), false),
    ("armadura", Some("task:Orçamento de aço"), false),
    ("fundações", Some("project:Jaboticatuba"), false),
    (
        "reuniões de Jaboticatuba",
        Some("capture:decidido na reunião de Jaboticatuba: fundação em estacas hélice"),
        false,
    ),
    (
        "provas",
        Some("task:Estudar para prova de Cálculo III"),
        false,
    ),
    // frases vagas, como se pergunta de verdade
    (
        "aquele negócio das bases que o Victor pediu",
        Some("task:Enviar bases para o Victor"),
        false,
    ),
    (
        "reunião da caixa de drenagem",
        Some("task:Dimensionar caixa de drenagem"),
        false,
    ),
    (
        "arquivo que guardei sobre Allplan",
        Some("capture:guardei o manual do Allplan na pasta de referências"),
        false,
    ),
    (
        "decisão da reunião de Jaboticatuba",
        Some("capture:decidido na reunião de Jaboticatuba: fundação em estacas hélice"),
        false,
    ),
    (
        "o que eu tinha que pagar do crea",
        Some("task:Pagar boleto do CREA"),
        false,
    ),
    (
        "aquela ideia do muro",
        Some("capture:ideia: usar blocos de concreto celular no muro"),
        false,
    ),
    (
        "a escada do bloco b",
        Some("task:Modelar escadas no Allplan"),
        false,
    ),
    (
        "quando visitar a obra da ratones",
        Some("capture:semana que vem visitar a obra da Ratones"),
        false,
    ),
    (
        "coisa da impermeabilização",
        Some("capture:lembrar de verificar a impermeabilização da caixa d'água"),
        false,
    ),
    (
        "o alvará do galpão",
        Some("task:Ligar para a prefeitura"),
        false,
    ),
    (
        "quantitativo das lajes",
        Some("task:Orçamento de aço"),
        false,
    ),
    (
        "o email da caixa economica",
        Some("task:Responder e-mail da Caixa"),
        false,
    ),
    (
        "estacas hélice",
        Some("capture:decidido na reunião de Jaboticatuba: fundação em estacas hélice"),
        false,
    ),
    (
        "integrais triplas",
        Some("task:Estudar para prova de Cálculo III"),
        false,
    ),
    // erro de digitação
    ("alplan", Some("task:Modelar escadas no Allplan"), false),
    ("jaboticatubba", Some("project:Jaboticatuba"), false),
    ("memoriall", Some("task:Revisar memorial de cálculo"), false),
    // não encontrado é a resposta certa
    ("contrato de aluguel", None, false),
    ("viagem para Portugal", None, false),
];

fn busca(storage: &SqliteStorage, consulta: &str) -> Vec<String> {
    storage
        .search_all(SearchRequest {
            query: consulta.to_owned(),
            include_archived: false,
            limit: 5,
        })
        .unwrap()
        .iter()
        .map(chave)
        .collect()
}

#[test]
fn corpus_de_busca_mede_o_top_5() {
    let m = massa();
    let mut acertos = 0;
    let mut exatas = 0;
    let mut exatas_ok = 0;
    let mut vazias_ok = 0;
    let mut vazias = 0;
    let mut falhas = Vec::new();
    for (consulta, esperado, exata) in CORPUS {
        let achados = busca(&m.storage, consulta);
        let ok = match esperado {
            Some(alvo) => achados.iter().any(|a| a == alvo),
            // "Não encontrado" admite resultado fraco, mas não um cheio de ruído.
            None => achados.len() <= 1,
        };
        if esperado.is_none() {
            vazias += 1;
            vazias_ok += usize::from(ok);
        }
        if *exata {
            exatas += 1;
            exatas_ok += usize::from(ok);
        }
        if ok {
            acertos += 1;
        } else {
            falhas.push(format!("{consulta:?} → {achados:?}"));
        }
    }
    let total = CORPUS.len();
    eprintln!(
        "[corpus] top5: {acertos}/{total} · exatas {exatas_ok}/{exatas} · nao-encontrado {vazias_ok}/{vazias}"
    );
    for falha in &falhas {
        eprintln!("[corpus] falhou: {falha}");
    }
    assert!(total >= 40);
    assert_eq!(exatas_ok, exatas, "consulta exata regrediu: {falhas:#?}");
    assert_eq!(
        vazias_ok, vazias,
        "'não encontrado' virou ruído: {falhas:#?}"
    );
    assert!(
        acertos >= PISO,
        "top5 {acertos}/{total} abaixo do piso {PISO}"
    );
}

/// Ver ADR-081: baseline 27/42 (AND estrito); depois da mudança 41/42. O piso
/// deixa folga de um para ajuste de ranking, e não mais.
const PISO: usize = 40;
