//! As relações do Knowledge Graph, contra banco de verdade.
//!
//! O §24 pede que relação sincronize como entidade de primeira classe. Este
//! arquivo prova as duas decisões que fazem isso funcionar, e nomeia o que
//! aconteceria sem cada uma.

use mos_core::{
    NewProject, NewResource, NewWorkspace, ResourceKind, ResourceRepository, WorkRepository,
};
use mos_storage_sqlite::SqliteStorage;
use mos_sync::{aplicar, DeviceRepository, EstadoDaEntidade, OpBody, OutboxRepository, Relacao};

fn com_sync(nome: &str) -> (tempfile::TempDir, SqliteStorage) {
    let dir = tempfile::tempdir().unwrap();
    let arquivo = dir.path().join("mos.db");
    let backups = dir.path().join("backups");
    std::fs::create_dir_all(&backups).unwrap();
    let storage = SqliteStorage::open(&arquivo, &backups).unwrap();
    let device = storage.este_dispositivo(nome, "windows", "0.3.0").unwrap();
    storage.habilitar_sync(device.id).unwrap();
    (dir, storage)
}

fn recurso_e_projeto(storage: &SqliteStorage) -> (mos_core::ResourceId, mos_core::ProjectId) {
    let recurso = storage
        .create_resource(
            NewResource::create(
                ResourceKind::Site,
                "Allplan",
                "https://allplan.com",
                "",
                None,
            )
            .unwrap(),
        )
        .unwrap();
    let projeto = storage
        .create_project(NewProject::create("043", "", "").unwrap())
        .unwrap();
    (recurso.id, projeto.id)
}

fn relacao_emitida(storage: &SqliteStorage) -> mos_sync::Op {
    storage
        .pendentes(50)
        .unwrap()
        .into_iter()
        .rfind(|op| op.entity.kind.as_str() == "relation")
        .expect("vincular precisa emitir uma relacao")
}

#[test]
fn vincular_emite_a_relacao_como_entidade() {
    let (_dir, storage) = com_sync("PC");
    let (recurso, projeto) = recurso_e_projeto(&storage);
    storage
        .set_resource_project(recurso, projeto, true)
        .unwrap();

    let relacao = relacao_emitida(&storage);
    match &relacao.body {
        OpBody::Update { fields } => {
            assert_eq!(fields["kind"], serde_json::json!("resourceProject"));
            assert_eq!(fields["linked"], serde_json::json!(true));
            // O id e um hash e nao diz nada. Quem recebe precisa saber O QUE
            // foi ligado sem ter visto a relacao antes.
            assert_eq!(fields["from"], serde_json::json!(recurso.to_string()));
            assert_eq!(fields["to"], serde_json::json!(projeto.to_string()));
        }
        outro => panic!("relacao precisa ser Update, veio {outro:?}"),
    }
}

#[test]
fn desvincular_nunca_emite_delete() {
    // A razao: `Delete` no motor tem semantica de "apagar ganha de editar", que
    // esta certa para uma Task e errada para um interruptor. Com `Delete`,
    // desvincular as 10:00 venceria revincular as 10:05 para sempre.
    let (_dir, storage) = com_sync("PC");
    let (recurso, projeto) = recurso_e_projeto(&storage);
    storage
        .set_resource_project(recurso, projeto, true)
        .unwrap();
    storage
        .set_resource_project(recurso, projeto, false)
        .unwrap();

    let ops = storage.pendentes(50).unwrap();
    assert!(
        !ops.iter().any(|op| matches!(op.body, OpBody::Delete)),
        "desvincular e alternar um campo, e nunca apagar"
    );
    match &relacao_emitida(&storage).body {
        OpBody::Update { fields } => assert_eq!(fields["linked"], serde_json::json!(false)),
        outro => panic!("veio {outro:?}"),
    }
}

#[test]
fn o_id_emitido_e_o_id_derivado_do_par() {
    // O CASO QUE JUSTIFICA O ID DERIVADO. Se cada dispositivo sorteasse um id,
    // dois dispositivos ligando o mesmo par criariam DUAS relacoes para o mesmo
    // vinculo, e desfazer uma deixaria a outra de pe — o Resource continuaria
    // ligado ao Project sem ninguem entender por que.
    //
    // Aqui se prova o elo que o banco tem a ver: o que o adaptador EMITE e
    // exatamente o id que `Relacao::id()` calcula a partir do par. Qualquer
    // outro dispositivo que conheca as duas pontas chega ao mesmo numero sem
    // conversar com este — e essa parte, que nao depende de banco, esta provada
    // em `mos-sync`.
    let (_dir, storage) = com_sync("PC");
    let (recurso, projeto) = recurso_e_projeto(&storage);
    storage
        .set_resource_project(recurso, projeto, true)
        .unwrap();

    let emitida = relacao_emitida(&storage);
    let calculada = Relacao::nova("resourceProject", recurso.as_uuid(), projeto.as_uuid());
    assert_eq!(emitida.entity.id, calculada.id());

    // E ligar de novo nao cria outra: a segunda operacao fala da MESMA
    // entidade, entao reconciliar deixa um vinculo so.
    storage
        .set_resource_project(recurso, projeto, true)
        .unwrap();
    let de_novo = relacao_emitida(&storage);
    assert_eq!(de_novo.entity.id, emitida.entity.id);

    let r = aplicar(EstadoDaEntidade::default(), &[emitida, de_novo]);
    assert_eq!(r.estado.campo("linked").unwrap(), &serde_json::json!(true));
    assert!(r.conflitos.is_empty(), "concordar nao e conflito");
}

#[test]
fn desvincular_num_e_revincular_no_outro_termina_vinculado() {
    // O ultimo gesto vence, que e o que um interruptor deve fazer. Com `Delete`
    // o vinculo nunca mais voltaria.
    let (_dir, pc) = com_sync("PC");
    let (recurso, projeto) = recurso_e_projeto(&pc);

    pc.set_resource_project(recurso, projeto, false).unwrap();
    let desligar = relacao_emitida(&pc);

    let relacao = Relacao::nova("resourceProject", recurso.as_uuid(), projeto.as_uuid());
    let religar = mos_sync::Op::new(
        uuid::Uuid::now_v7(),
        relacao.entidade(),
        relacao.alternar(true),
        // Cinco minutos depois, e do outro dispositivo.
        mos_sync::Hlc::new(
            desligar.at.wall_ms + 300_000,
            0,
            mos_sync::DeviceId(uuid::Uuid::from_u128(999)),
        ),
    );

    let r = aplicar(EstadoDaEntidade::default(), &[desligar, religar]);
    assert_eq!(
        r.estado.campo("linked").unwrap(),
        &serde_json::json!(true),
        "revincular depois precisa vencer"
    );
    assert!(
        r.estado.visivel(),
        "a relacao nunca e apagada, so alternada"
    );
}

#[test]
fn workspace_tambem_vira_relacao() {
    let (_dir, storage) = com_sync("PC");
    let recurso = storage
        .create_resource(
            NewResource::create(ResourceKind::Site, "x", "https://x.com", "", None).unwrap(),
        )
        .unwrap();
    let workspace = storage
        .create_workspace(NewWorkspace::create("Engenharia", "").unwrap())
        .unwrap();
    storage
        .set_resource_workspace(recurso.id, workspace.id, true)
        .unwrap();

    match &relacao_emitida(&storage).body {
        OpBody::Update { fields } => {
            assert_eq!(fields["kind"], serde_json::json!("resourceWorkspace"))
        }
        outro => panic!("veio {outro:?}"),
    }
}

#[test]
fn project_no_workspace_tambem_vira_relacao() {
    let (_dir, storage) = com_sync("PC");
    let projeto = storage
        .create_project(NewProject::create("043", "", "").unwrap())
        .unwrap();
    let workspace = storage
        .create_workspace(NewWorkspace::create("Engenharia", "").unwrap())
        .unwrap();
    storage
        .set_project_workspace(projeto.id, workspace.id, true)
        .unwrap();

    match &relacao_emitida(&storage).body {
        OpBody::Update { fields } => {
            assert_eq!(fields["kind"], serde_json::json!("projectWorkspace"));
            assert_eq!(fields["from"], serde_json::json!(projeto.id.to_string()));
        }
        outro => panic!("veio {outro:?}"),
    }
}

#[test]
fn tipos_diferentes_de_vinculo_nao_colidem() {
    // O tipo faz parte da identidade. Sem isso, ligar as MESMAS duas pontas por
    // dois motivos diferentes colidiria num vinculo so, e desfazer um desfaria
    // o outro.
    let a = Relacao::nova(
        "resourceProject",
        uuid::Uuid::from_u128(1),
        uuid::Uuid::from_u128(2),
    );
    let b = Relacao::nova(
        "resourceWorkspace",
        uuid::Uuid::from_u128(1),
        uuid::Uuid::from_u128(2),
    );
    assert_ne!(a.id(), b.id());
}
