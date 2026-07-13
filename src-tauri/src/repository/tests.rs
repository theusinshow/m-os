//! Testes de persistencia e validacao (secao 22).
//!
//! Rodam contra um SQLite em memoria com a migration real aplicada. O pool usa
//! uma unica conexao para que o banco em memoria seja compartilhado entre as
//! consultas.

use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};

use crate::database::{MIGRATION_0001, MIGRATION_0002, MIGRATION_0003, MIGRATION_0004};
use crate::models::{
    validate_status, ClientInput, EntryUpdateInput, ManualEntryInput, ProjectInput,
    StartTimerInput,
};
use crate::repository::{clients, new_id, now_iso, projects, time_entries, timer};

async fn setup() -> Pool<Sqlite> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("conectar sqlite em memoria");
    for migration in [MIGRATION_0001, MIGRATION_0002, MIGRATION_0003, MIGRATION_0004] {
        sqlx::raw_sql(migration)
            .execute(&pool)
            .await
            .expect("aplicar migration");
    }
    pool
}

fn client_input(name: &str) -> ClientInput {
    ClientInput {
        name: name.to_string(),
        company_name: None,
        email: None,
        phone: None,
        notes: None,
    }
}

fn project_input(name: &str, rate: i64) -> ProjectInput {
    ProjectInput {
        client_id: None,
        name: name.to_string(),
        code: None,
        description: None,
        hourly_rate_cents: rate,
        budget_minutes: None,
        color: None,
    }
}

#[tokio::test]
async fn totais_por_projeto_somam_sessoes() {
    let pool = setup().await;
    let project = projects::create(&pool, project_input("Proj", 6000).validate().unwrap())
        .await
        .unwrap();
    time_entries::create_manual(
        &pool,
        manual_input(&project.id, "2026-07-11T08:00:00Z", "2026-07-11T09:00:00Z")
            .validate()
            .unwrap(),
    )
    .await
    .unwrap();
    let totals = time_entries::totals_by_project(&pool).await.unwrap();
    let total = totals.iter().find(|t| t.project_id == project.id).unwrap();
    assert_eq!(total.seconds, 3600);
}

#[tokio::test]
async fn cria_e_lista_cliente() {
    let pool = setup().await;
    let created = clients::create(&pool, client_input("Meridiano").validate().unwrap())
        .await
        .unwrap();
    assert_eq!(created.name, "Meridiano");
    assert!(created.archived_at.is_none());

    let all = clients::list(&pool, false).await.unwrap();
    assert_eq!(all.len(), 1);

    let fetched = clients::get(&pool, &created.id).await.unwrap();
    assert_eq!(fetched.id, created.id);
}

#[tokio::test]
async fn arquivar_cliente_o_oculta_da_lista_padrao() {
    let pool = setup().await;
    let c = clients::create(&pool, client_input("Temp").validate().unwrap())
        .await
        .unwrap();
    clients::archive(&pool, &c.id).await.unwrap();

    assert_eq!(clients::list(&pool, false).await.unwrap().len(), 0);
    assert_eq!(clients::list(&pool, true).await.unwrap().len(), 1);
}

#[tokio::test]
async fn atualiza_cliente_persistindo() {
    let pool = setup().await;
    let c = clients::create(&pool, client_input("Antigo").validate().unwrap())
        .await
        .unwrap();
    let mut input = client_input("Novo Nome");
    input.email = Some("contato@exemplo.com".to_string());
    let updated = clients::update(&pool, &c.id, input.validate().unwrap())
        .await
        .unwrap();
    assert_eq!(updated.name, "Novo Nome");
    assert_eq!(updated.email.as_deref(), Some("contato@exemplo.com"));
}

#[tokio::test]
async fn cria_projeto_preserva_valor_hora() {
    let pool = setup().await;
    let p = projects::create(&pool, project_input("Projeto A", 9000).validate().unwrap())
        .await
        .unwrap();
    assert_eq!(p.hourly_rate_cents, 9000);
    assert_eq!(p.status, "active");
}

#[tokio::test]
async fn arquivar_projeto_o_oculta_e_marca_archived_at() {
    let pool = setup().await;
    let p = projects::create(&pool, project_input("Projeto B", 10000).validate().unwrap())
        .await
        .unwrap();
    let archived = projects::set_status(&pool, &p.id, "archived").await.unwrap();
    assert_eq!(archived.status, "archived");
    assert!(archived.archived_at.is_some());
    assert_eq!(projects::list(&pool, false).await.unwrap().len(), 0);
    assert_eq!(projects::list(&pool, true).await.unwrap().len(), 1);
}

#[tokio::test]
async fn concluir_e_reativar_projeto_ajusta_archived_at() {
    let pool = setup().await;
    let p = projects::create(&pool, project_input("Projeto C", 8000).validate().unwrap())
        .await
        .unwrap();
    let completed = projects::set_status(&pool, &p.id, "completed").await.unwrap();
    assert_eq!(completed.status, "completed");
    assert!(completed.archived_at.is_none());

    let archived = projects::set_status(&pool, &p.id, "archived").await.unwrap();
    assert!(archived.archived_at.is_some());
    let reactivated = projects::set_status(&pool, &p.id, "active").await.unwrap();
    assert!(reactivated.archived_at.is_none());
}

#[test]
fn validacao_rejeita_entradas_invalidas() {
    assert!(client_input("   ").validate().is_err());
    let mut with_bad_email = client_input("Ok");
    with_bad_email.email = Some("sem-arroba".to_string());
    assert!(with_bad_email.validate().is_err());

    assert!(project_input("  ", 1000).validate().is_err());
    assert!(project_input("Ok", -1).validate().is_err());

    assert!(validate_status("active").is_ok());
    assert!(validate_status("invalido").is_err());
}

fn start_input(project_id: &str) -> StartTimerInput {
    StartTimerInput {
        project_id: project_id.to_string(),
        activity_type: "drawing".to_string(),
        description: Some("sessao".to_string()),
    }
}

#[tokio::test]
async fn cronometro_ciclo_start_pause_resume_stop() {
    let pool = setup().await;
    let project = projects::create(&pool, project_input("Proj", 9000).validate().unwrap())
        .await
        .unwrap();

    // start
    let started = timer::start(&pool, start_input(&project.id).validate().unwrap())
        .await
        .unwrap();
    assert_eq!(started.status, "running");
    assert_eq!(started.accumulated_seconds, 0);
    assert!(timer::active(&pool).await.unwrap().is_some());

    // pause -> paused
    let paused = timer::pause(&pool).await.unwrap();
    assert_eq!(paused.status, "paused");

    // resume -> running
    let resumed = timer::resume(&pool).await.unwrap();
    assert_eq!(resumed.status, "running");

    // stop -> cria time_entry e remove o active_timer
    let entry = timer::stop(&pool).await.unwrap();
    assert_eq!(entry.project_id, project.id);
    assert_eq!(entry.source, "timer");
    assert!(entry.duration_seconds >= 0);
    assert!(timer::active(&pool).await.unwrap().is_none());
}

#[tokio::test]
async fn start_falha_quando_ja_existe_ativo() {
    let pool = setup().await;
    let project = projects::create(&pool, project_input("Proj", 5000).validate().unwrap())
        .await
        .unwrap();
    timer::start(&pool, start_input(&project.id).validate().unwrap())
        .await
        .unwrap();
    let second = timer::start(&pool, start_input(&project.id).validate().unwrap()).await;
    assert!(second.is_err(), "nao deve iniciar um segundo cronometro");
}

#[tokio::test]
async fn stop_congela_snapshot_do_valor_hora() {
    // A sessao gravada preserva o valor/hora; alterar o projeto depois nao a muda.
    let pool = setup().await;
    let project = projects::create(&pool, project_input("Proj", 9000).validate().unwrap())
        .await
        .unwrap();
    timer::start(&pool, start_input(&project.id).validate().unwrap())
        .await
        .unwrap();
    let entry = timer::stop(&pool).await.unwrap();
    assert_eq!(entry.hourly_rate_snapshot_cents, 9000);

    // Muda o valor/hora do projeto.
    projects::update(
        &pool,
        &project.id,
        project_input("Proj", 12000).validate().unwrap(),
    )
    .await
    .unwrap();

    // A sessao ja gravada permanece com o snapshot antigo.
    let recent = time_entries::list_recent(&pool, 10, false).await.unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].hourly_rate_snapshot_cents, 9000);
}

#[tokio::test]
async fn desconto_de_inatividade_vai_para_a_sessao_ao_encerrar() {
    let pool = setup().await;
    let project = projects::create(&pool, project_input("Proj", 6000).validate().unwrap())
        .await
        .unwrap();
    timer::start(&pool, start_input(&project.id).validate().unwrap())
        .await
        .unwrap();

    // Usuario decide descontar 300s (5 min) de inatividade.
    let updated = timer::add_idle(&pool, 300).await.unwrap();
    assert_eq!(updated.idle_seconds, 300);

    let entry = timer::stop(&pool).await.unwrap();
    // O inativo e limitado a duracao bruta (aqui ~0), preservando net >= 0.
    assert!(entry.idle_seconds <= entry.duration_seconds);
    assert!(entry.idle_seconds >= 0);
}

#[tokio::test]
async fn add_idle_rejeita_valor_negativo() {
    let pool = setup().await;
    let project = projects::create(&pool, project_input("Proj", 6000).validate().unwrap())
        .await
        .unwrap();
    timer::start(&pool, start_input(&project.id).validate().unwrap())
        .await
        .unwrap();
    assert!(timer::add_idle(&pool, -10).await.is_err());
}

#[tokio::test]
async fn descartar_remove_cronometro_sem_criar_sessao() {
    let pool = setup().await;
    let project = projects::create(&pool, project_input("Proj", 7000).validate().unwrap())
        .await
        .unwrap();
    timer::start(&pool, start_input(&project.id).validate().unwrap())
        .await
        .unwrap();
    timer::discard(&pool).await.unwrap();
    assert!(timer::active(&pool).await.unwrap().is_none());
    assert_eq!(time_entries::list_recent(&pool, 10, false).await.unwrap().len(), 0);
}

#[tokio::test]
async fn pause_e_resume_sem_cronometro_retornam_erro() {
    let pool = setup().await;
    assert!(timer::pause(&pool).await.is_err());
    assert!(timer::resume(&pool).await.is_err());
    assert!(timer::stop(&pool).await.is_err());
}

fn manual_input(project_id: &str, start: &str, end: &str) -> ManualEntryInput {
    ManualEntryInput {
        project_id: project_id.to_string(),
        started_at: start.to_string(),
        ended_at: end.to_string(),
        description: Some("manual".to_string()),
        activity_type: "drawing".to_string(),
        billable: true,
        idle_seconds: 0,
        source: None,
    }
}

#[tokio::test]
async fn sessao_reconstruida_usa_source_reconstructed() {
    let pool = setup().await;
    let project = projects::create(&pool, project_input("Proj", 6000).validate().unwrap())
        .await
        .unwrap();
    let mut input = manual_input(
        &project.id,
        "2026-07-11T08:00:00Z",
        "2026-07-11T09:00:00Z",
    );
    input.source = Some("reconstructed".to_string());
    let entry = time_entries::create_manual(&pool, input.validate().unwrap())
        .await
        .unwrap();
    assert_eq!(entry.source, "reconstructed");
}

#[tokio::test]
async fn sessao_manual_atravessa_meia_noite() {
    let pool = setup().await;
    let project = projects::create(&pool, project_input("Proj", 6000).validate().unwrap())
        .await
        .unwrap();
    // 23:30 -> 00:30 do dia seguinte = 1h.
    let input = manual_input(
        &project.id,
        "2026-07-11T23:30:00Z",
        "2026-07-12T00:30:00Z",
    )
    .validate()
    .unwrap();
    let entry = time_entries::create_manual(&pool, input).await.unwrap();
    assert_eq!(entry.duration_seconds, 3600);
    assert_eq!(entry.source, "manual");
}

#[test]
fn horario_final_antes_do_inicial_e_invalido() {
    let input = manual_input("p", "2026-07-11T10:00:00Z", "2026-07-11T09:00:00Z");
    assert!(input.validate().is_err());
    let bad = manual_input("p", "nao-e-data", "2026-07-11T09:00:00Z");
    assert!(bad.validate().is_err());
}

#[tokio::test]
async fn edita_sessao_recalculando_duracao() {
    let pool = setup().await;
    let project = projects::create(&pool, project_input("Proj", 6000).validate().unwrap())
        .await
        .unwrap();
    let entry = time_entries::create_manual(
        &pool,
        manual_input(&project.id, "2026-07-11T08:00:00Z", "2026-07-11T09:00:00Z")
            .validate()
            .unwrap(),
    )
    .await
    .unwrap();

    let update = EntryUpdateInput {
        started_at: "2026-07-11T08:00:00Z".to_string(),
        ended_at: "2026-07-11T10:00:00Z".to_string(),
        description: Some("editada".to_string()),
        activity_type: "revision".to_string(),
        billable: false,
        idle_seconds: 0,
    }
    .validate()
    .unwrap();
    let updated = time_entries::update(&pool, &entry.id, update).await.unwrap();
    assert_eq!(updated.duration_seconds, 7200);
    assert_eq!(updated.activity_type, "revision");
    assert!(!updated.billable);
}

#[tokio::test]
async fn soft_delete_e_restore_de_sessao() {
    let pool = setup().await;
    let project = projects::create(&pool, project_input("Proj", 6000).validate().unwrap())
        .await
        .unwrap();
    let entry = time_entries::create_manual(
        &pool,
        manual_input(&project.id, "2026-07-11T08:00:00Z", "2026-07-11T09:00:00Z")
            .validate()
            .unwrap(),
    )
    .await
    .unwrap();

    time_entries::soft_delete(&pool, &entry.id).await.unwrap();
    assert_eq!(time_entries::list_recent(&pool, 10, false).await.unwrap().len(), 0);
    assert_eq!(time_entries::list_recent(&pool, 10, true).await.unwrap().len(), 1);

    let restored = time_entries::restore(&pool, &entry.id).await.unwrap();
    assert!(restored.deleted_at.is_none());
    assert_eq!(time_entries::list_recent(&pool, 10, false).await.unwrap().len(), 1);
}

async fn insert_active_timer(
    pool: &Pool<Sqlite>,
    project_id: &str,
) -> Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error> {
    let now = now_iso();
    sqlx::query(
        "INSERT INTO active_timer \
         (id, project_id, started_at, last_resumed_at, accumulated_seconds, \
          status, activity_type, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?3, 0, 'running', 'drawing', ?3, ?3)",
    )
    .bind(new_id())
    .bind(project_id)
    .bind(now)
    .execute(pool)
    .await
}

#[tokio::test]
async fn banco_impede_dois_cronometros_ativos() {
    // Regra critica: a constraint UNIQUE/CHECK em active_timer.singleton impede
    // um segundo registro ativo, alem das regras de dominio.
    let pool = setup().await;
    let project = projects::create(&pool, project_input("Proj", 5000).validate().unwrap())
        .await
        .unwrap();

    insert_active_timer(&pool, &project.id)
        .await
        .expect("primeiro cronometro ativo");
    let second = insert_active_timer(&pool, &project.id).await;
    assert!(
        second.is_err(),
        "o banco deveria impedir um segundo cronometro ativo"
    );
}
