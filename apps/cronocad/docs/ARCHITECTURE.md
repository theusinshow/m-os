# ARCHITECTURE.md — CronoCAD

## Componentes

```
+-------------------------------------------------------------+
|  Frontend (WebView) — React + TS + Tailwind                 |
|                                                             |
|  apresentacao ── estado de UI (Zustand) ── regras (src/lib) |
|        |                                                    |
|        | comandos Tauri (tipados) / eventos Tauri (tipados) |
+--------|----------------------------------------------------+
         v
+-------------------------------------------------------------+
|  Backend (Rust) — Tauri 2                                   |
|                                                             |
|  commands ── domain ── database (SQLite via plugin-sql)     |
|      |          |          ^                                |
|      |          |          |                                |
|  monitoring   notifications tray   state                    |
|  (Windows)     (nativas)   (bandeja)                        |
+-------------------------------------------------------------+
         |
         v
   SQLite local (app data dir) — fonte persistente da verdade
```

## Responsabilidades

| Camada | Pasta | Responsabilidade |
|---|---|---|
| Apresentacao | `src/components`, `src/features/*` | Render, layout, telas |
| Estado de UI | `src/stores` | Tema e estado derivado (nao autoritativo) |
| Regras (front) | `src/lib` | Formatacao, arredondamento, duracao, dinheiro |
| Servicos (front) | `src/services` | Ponte com Tauri; dados temporarios (fase atual) |
| Comandos | `src-tauri/src/commands` | API especifica e validada para o frontend |
| Dominio (back) | `src-tauri/src/domain` | Regras centrais do cronometro (autoritativo) |
| Persistencia | `src-tauri/src/database`, `migrations/` | Esquema e migrations |
| Monitoramento | `src-tauri/src/monitoring` | Deteccao de processos (Windows) |
| Notificacoes | `src-tauri/src/notifications` | Avisos nativos com cooldown |
| Bandeja | `src-tauri/src/tray` | Menu e ciclo de vida da janela |
| Estado (back) | `src-tauri/src/state` | Handles de servicos e cache de estado |

## Comunicacao frontend/backend

- **Comandos** (`invoke`): frontend chama funcoes Rust especificas e tipadas.
  Nenhum SQL arbitrario e exposto (seguranca — secao 19). Wrapper tipado em
  `src/services/tauri.ts`, que degrada com seguranca no navegador.
- **Eventos** (`listen`): backend emite eventos tipados
  (`timer-state-changed`, `monitored-app-opened`, `monitored-app-closed`,
  `idle-started`, `idle-ended`, `database-updated`). Contratos em
  `src/types/events.ts`. Evita-se polling excessivo no frontend.

## Persistencia

SQLite local via `tauri-plugin-sql`, resolvido no diretorio de dados do app
(`sqlite:cronocad.sqlite`). Esquema criado **somente** por migrations
versionadas registradas em `database::migrations()`. Detalhes em
`docs/DATABASE.md`.

### Acesso ao banco sem expor SQL ao frontend (secao 19)

O plugin oficial conecta e migra o banco no **startup** via `preload` em
`tauri.conf.json` (`plugins.sql.preload`). Os comandos do backend obtem o
**mesmo** pool `sqlx` do plugin por `database::pool()` (que le o estado
`DbInstances` e casa a variante `DbPool::Sqlite`) e executam consultas
**especificas e tipadas** na camada `repository` (valores sempre via `bind`,
nunca concatenacao). O frontend chama apenas comandos nomeados
(`list_projects`, `create_client`, …) — **nenhum** SQL arbitrario e exposto, e
nao ha pool duplicado. As permissoes de `sql:*` **nao** sao concedidas ao
webview em `capabilities/default.json`.

Fluxo de escrita/leitura:
`UI -> store (Zustand) -> service (invoke) -> command (valida) -> repository (sqlx) -> SQLite`.

## Monitoramento (implementado — secao 10)

`monitoring::run` roda em tarefa `tokio` propria (nao bloqueia) e, a cada
`process_check_interval_seconds`, le os processos via **`sysinfo`** (apenas
nomes — secao 6; sem `shell`). Compara com os `monitored_apps` habilitados,
calcula transicoes com `diff_transitions` (funcao pura testada) sem repetir
eventos, registra `app_opened`/`app_closed` em `activity_events`, emite
`monitored-app-opened`/`closed` e dispara notificacoes nativas com **cooldown**
e supressao "nao lembrar hoje". Continua ativo com a janela na bandeja; respeita
`process_monitoring_enabled` (pode ser desligado) e encerra de forma limpa no
`RunEvent::Exit` (`MonitorShared::stop`). O estado em memoria (snapshot anterior,
cooldowns, supressoes) fica em `MonitorShared` (mutex), atualizado tambem pelo
comando `suppress_app_reminder_today`.

O frontend decide a acao: `monitored-app-opened` sem cronometro ativo abre o
lembrete de projeto; `monitored-app-closed` com cronometro ativo abre o lembrete
de encerrar/manter/pausar. Nunca vincula projeto nem encerra silenciosamente.

## Inatividade (implementado — secao 11)

`idle::run` roda em tarefa `tokio` propria e amostra, a cada poucos segundos, o
tempo desde a ultima entrada via **`GetLastInputInfo`** (Windows) — apenas o
tempo ocioso, nunca teclas/coordenadas/conteudo (secao 6/11). `classify` (funcao
pura testada) decide as transicoes. Ao cruzar o limite (`idle_threshold_minutes`)
registra `idle_started`; ao retornar a atividade registra `idle_ended` e emite o
evento correspondente com a duracao inativa. O frontend, havendo um cronometro
ativo, abre o lembrete manter/descontar/editar. O desconto e aplicado por
`discount_idle`, que soma em `active_timer.idle_seconds` (migration 0002); ao
encerrar, esse valor (limitado a duracao bruta) vai para `time_entries.idle_seconds`.
Nunca desconta automaticamente. Respeita `idle_detection_enabled` e encerra de
forma limpa no `RunEvent::Exit`.

## Recuperacao e relogio do sistema

- **Estado do cronometro** e persistido em `active_timer` a cada transicao. Ao
  abrir, se houver cronometro em execucao, o app calcula o periodo transcorrido
  e apresenta modal de recuperacao (manter/editar/descartar) — nunca decide
  silenciosamente.
- **Duracao** e calculada no backend a partir de timestamps persistidos, nao de
  contador do frontend. `elapsed_seconds` trata delta negativo (relogio para
  tras) como zero, de modo que ajustes do relogio nunca reduzem tempo ja
  acumulado. Estrategia documentada tambem em `CLAUDE.md` (regra 4).

## Distincoes de tempo

- **Bruta**: intervalo total inicio->fim.
- **Inativa**: periodo sem atividade detectada.
- **Liquida**: bruta menos inativa descontada.
- **Faturavel**: liquida quando `billable = true`, senao 0.
- **Arredondada**: aplicada apenas na visualizacao/cobranca; nunca persistida.

## Decisoes tecnicas

- **Tailwind 3 + tokens CSS** (em vez de v4): setup estavel e previsivel; tokens
  centralizados como unica fonte de verdade, preparados para modo claro.
- **Tabela `settings` de linha unica** com colunas explicitas (tipagem direta)
  em vez de chave-valor.
- **`active_timer.singleton UNIQUE CHECK(=1)`**: garante um cronometro ativo no
  nivel do banco, alem das regras de dominio.
- **Regras de dominio duplicadas** (TS e Rust) com testes espelhados: o backend
  e autoritativo; o frontend replica para exibir sem round-trip a cada segundo.
- **Fechar-para-bandeja** intercepta `CloseRequested` e esconde a janela,
  mantendo o app ativo; "Sair completamente" encerra de fato.
