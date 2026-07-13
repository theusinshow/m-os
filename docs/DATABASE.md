# DATABASE.md — CronoCAD

Banco **SQLite** local, acessado via `tauri-plugin-sql`, criado apenas por
**migrations versionadas** (`src-tauri/migrations/`). Fonte da verdade do
dominio.

## Convencoes

- **ids**: `TEXT` (UUID) gerados pela aplicacao.
- **timestamps**: `TEXT` ISO 8601 em UTC (`YYYY-MM-DDTHH:MM:SSZ`).
- **dinheiro**: `INTEGER` em centavos.
- **duracoes**: `INTEGER` em segundos.
- **booleanos**: `INTEGER` (0/1).
- Regras de valores usam `CHECK` para status/enums.

## Tabelas

### clients
Dados de clientes. `archived_at` para arquivamento (soft).
Campos: `id`, `name`, `company_name`, `email`, `phone`, `notes`,
`created_at`, `updated_at`, `archived_at`.

### projects
Projetos com valor/hora e status.
Campos: `id`, `client_id` (FK -> clients, `ON DELETE SET NULL`), `name`, `code`,
`description`, `hourly_rate_cents`, `status`
(`active|paused|completed|archived`), `color`, `created_at`, `updated_at`,
`archived_at`.
Indices: `idx_projects_client`, `idx_projects_status`.

### time_entries
Sessoes de trabalho concluidas. Soft delete via `deleted_at`.
Campos: `id`, `project_id` (FK -> projects, `ON DELETE CASCADE`), `started_at`,
`ended_at`, `duration_seconds`, `idle_seconds`, `description`, `activity_type`
(`drawing|detailing|revision|meeting|study|other`), `billable`,
`hourly_rate_snapshot_cents`, `source` (`timer|manual|reconstructed`),
`created_at`, `updated_at`, `deleted_at`.
Indices: `idx_time_entries_project`, `idx_time_entries_started`,
`idx_time_entries_deleted`.
Regra: `hourly_rate_snapshot_cents` preserva o valor/hora do momento da sessao.

### active_timer
Estado do **unico** cronometro ativo. Persistido a cada transicao para permitir
recuperacao.
Campos: `id`, `singleton` (UNIQUE, `CHECK = 1`), `project_id` (FK -> projects,
`ON DELETE CASCADE`), `started_at`, `last_resumed_at`, `accumulated_seconds`,
`status` (`running|paused`), `description`, `activity_type`, `created_at`,
`updated_at`.
**Integridade:** a coluna `singleton UNIQUE CHECK(singleton = 1)` garante, no
nivel do banco, no maximo um registro ativo.

### monitored_apps
Executaveis monitorados (configuraveis).
Campos: `id`, `display_name`, `process_name` (UNIQUE), `enabled`,
`remind_on_open`, `remind_on_close`, `created_at`, `updated_at`.
Seed inicial: AutoCAD, Revit, SketchUp, Eberick, QiBuilder (nem todos estarao
instalados).

### activity_events
Eventos detectados para reconstrucao do dia e auditoria.
Campos: `id`, `event_type` (`app_opened|app_closed|idle_started|idle_ended|
timer_started|timer_paused|timer_resumed|timer_stopped`), `process_name`,
`detected_at`, `metadata_json`, `processed`, `created_at`.
Indices: `idx_activity_events_detected`, `idx_activity_events_type`,
`idx_activity_events_processed`.

### settings
Linha unica (`id = 1`) com colunas explicitas e tipadas. Valores iniciais
recomendados (secao 8): `idle_threshold_minutes = 10`,
`process_check_interval_seconds = 5`, `rounding_enabled = 0`,
`rounding_mode = 'nearest'`, `minimize_to_tray = 1`, `close_to_tray = 1`,
`currency = 'BRL'`, `locale = 'pt-BR'`, etc.

## Relacionamentos

```
clients 1 ── N projects 1 ── N time_entries
                       1 ── 0..1 active_timer
monitored_apps (independente)   activity_events (independente)
settings (linha unica)
```

## Migrations

- Arquivos: `src-tauri/migrations/NNNN_descricao.sql` (versao crescente).
- Registro: `src-tauri/src/database/mod.rs` (`migrations()`), aplicadas na
  inicializacao pelo plugin.
- **Nunca** editar migration ja aplicada em producao; sempre criar nova.
- Atual: `0001_initial_schema.sql` (versao 1).

## Regras de integridade

- Um cronometro ativo no maximo (constraint acima).
- Preservar sempre o tempo real; arredondamento/desconto so em
  visualizacao/cobranca.
- Exclusoes preferencialmente por soft delete (`deleted_at`), com confirmacao.
- Timestamps sempre em UTC; conversao para exibicao ocorre no frontend.
