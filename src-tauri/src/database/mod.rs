//! Camada de banco de dados.
//!
//! Registra as migrations versionadas do SQLite e expoe acesso ao pool `sqlx`.
//! As tabelas nunca sao criadas de forma informal na inicializacao: toda
//! alteracao de esquema passa por uma migration numerada em `migrations/`
//! (secoes 8 e 20).
//!
//! Acesso ao banco (secao 19): o plugin oficial conecta e migra o banco no
//! startup (via `preload` em `tauri.conf.json`). Os comandos do backend obtem o
//! **mesmo** pool por `pool()` e executam consultas especificas e tipadas com
//! `sqlx`. Nenhum SQL arbitrario e exposto ao frontend.

use sqlx::{Pool, Sqlite};
use tauri_plugin_sql::{DbInstances, DbPool, Migration, MigrationKind};

use crate::error::AppError;

/// URL do banco local. O plugin resolve caminhos relativos para o diretorio de
/// dados do aplicativo, mantendo o dado fora do repositorio e por usuario.
pub const DB_URL: &str = "sqlite:cronocad.sqlite";

/// SQL das migrations, tambem usados nos testes de persistencia.
pub const MIGRATION_0001: &str = include_str!("../../migrations/0001_initial_schema.sql");
pub const MIGRATION_0002: &str =
    include_str!("../../migrations/0002_active_timer_idle.sql");
pub const MIGRATION_0003: &str = include_str!("../../migrations/0003_project_budget.sql");
pub const MIGRATION_0004: &str = include_str!("../../migrations/0004_issuer_settings.sql");

/// Lista ordenada de migrations. Acrescente novas entradas com versao crescente;
/// nunca edite uma migration ja aplicada em producao.
pub fn migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            description: "esquema inicial",
            sql: MIGRATION_0001,
            kind: MigrationKind::Up,
        },
        Migration {
            version: 2,
            description: "tempo inativo no cronometro ativo",
            sql: MIGRATION_0002,
            kind: MigrationKind::Up,
        },
        Migration {
            version: 3,
            description: "meta de horas por projeto",
            sql: MIGRATION_0003,
            kind: MigrationKind::Up,
        },
        Migration {
            version: 4,
            description: "dados do emissor para faturas",
            sql: MIGRATION_0004,
            kind: MigrationKind::Up,
        },
    ]
}

/// Obtem um clone do pool SQLite gerenciado pelo plugin (barato — o pool e
/// interno a um `Arc`). Falha de forma legivel se o pool nao estiver pronto.
pub async fn pool(instances: &DbInstances) -> Result<Pool<Sqlite>, AppError> {
    let map = instances.0.read().await;
    match map.get(DB_URL) {
        Some(DbPool::Sqlite(p)) => Ok(p.clone()),
        _ => Err(AppError::Database(
            "pool SQLite indisponivel (banco nao carregado)".to_string(),
        )),
    }
}
