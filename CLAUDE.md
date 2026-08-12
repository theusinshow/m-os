# CLAUDE.md — CronoCAD

Guia para agentes e pessoas que trabalham neste repositorio. Leia antes de
alterar codigo.

## Visao do projeto

CronoCAD e um aplicativo **desktop Windows, local-first**, que rastreia horas de
trabalho de desenhistas/projetistas de CAD. O objetivo central e **reduzir o
esquecimento de registrar o trabalho**, por meio de cronometro por projeto,
deteccao de programas CAD abertos, deteccao de inatividade, reconstrucao
aproximada do dia e relatorios de horas/valores para cobranca.

Pergunta que guia cada decisao de produto:
> Isso reduz a possibilidade de o usuario esquecer de registrar seu trabalho?

O app **nao** e ferramenta de vigilancia: sem captura de tela, keylogging,
leitura de arquivos CAD ou telemetria sem consentimento.

## Stack

- **Tauri 2** (backend Rust) + **React 18** + **TypeScript** (estrito) + **Vite 6**
- **SQLite** via **`tauri-plugin-sql`** (plugin oficial)
- **Tailwind CSS 3** (tokens em `src/styles/tokens.css`)
- **React Router 6**, **Zustand** (estado de UI), **Zod** (validacao)
- **lucide-react** (icones)
- Testes: **Vitest** (frontend) + testes nativos do **Rust** (`cargo test`)
- Gerenciador de pacotes: **npm**

Nao usar Electron. Nao adicionar dependencias sem necessidade real.

## Comandos

Frontend (raiz):

| Comando | Acao |
|---|---|
| `npm install` | Instala dependencias |
| `npm run dev` | Vite dev server (porta 1420) |
| `npm run build` | `tsc --noEmit` + build de producao do frontend |
| `npm run typecheck` | Checagem de tipos |
| `npm run lint` / `lint:fix` | ESLint (0 warnings) |
| `npm run format` | Prettier |
| `npm run test` | Vitest (uma passada) |
| `npm run tauri:dev` | Roda o app desktop (Tauri) |
| `npm run tauri:build` | Gera o instalador Windows (NSIS) |

Backend (em `src-tauri/`):

| Comando | Acao |
|---|---|
| `cargo build` | Compila o backend |
| `cargo test` | Testes das regras de dominio |
| `cargo fmt` / `cargo clippy` | Formatacao / lint Rust |

## Arquitetura (resumo)

Separacao clara de responsabilidades (detalhes em `docs/ARCHITECTURE.md`):

- **Apresentacao** — `src/components`, `src/features/*/**Page.tsx`
- **Estado de UI** — `src/stores` (Zustand); **nao** e a fonte da verdade
- **Regras de dominio** — `src/lib` (frontend) e `src-tauri/src/domain` (backend)
- **Persistencia** — SQLite + migrations (`src-tauri/migrations`)
- **Integracao com o SO** — `src-tauri/src/monitoring`, `.../tray`
- **Comandos Tauri** — `src-tauri/src/commands`
- **Relatorios** — `src/features/reports`

O **banco e a fonte persistente da verdade**. O frontend mantem apenas estado
derivado para renderizacao.

## Convencoes

### TypeScript
- Modo estrito; **proibido `any`** (ESLint erra).
- Tipos de dominio em `src/types/domain.ts`; validar dados externos (Zod).
- Componentes pequenos e focados; separar visual de regra de negocio.
- **Nao** executar SQL dentro de componentes React.
- Import alias `@/` -> `src/`.

### Rust
- Tratar erros explicitamente; **sem `unwrap()`** em caminhos de producao.
- Retornar `AppError` (serializa como string legivel) para o frontend.
- Separar comandos Tauri dos servicos; usar structs tipadas.
- Documentar codigo especifico do Windows; loops de monitoramento devem poder
  ser encerrados corretamente.

### Banco
- **Sempre** via migrations versionadas (`src-tauri/migrations/NNNN_*.sql`).
  Nunca criar tabelas informalmente na inicializacao.
- Indices para consultas frequentes; timestamps consistentes (ISO 8601 UTC).
- Preservar historico; usar soft delete (`deleted_at`) quando adequado.

## Regras criticas (nao quebrar)

1. **Confiabilidade dos registros acima de tudo.** Nunca sacrificar
   confiabilidade por estetica/animacao.
2. **No maximo um cronometro ativo.** Garantido no banco (coluna `singleton`
   unica em `active_timer`) e nas regras de dominio.
3. **Persistir estado do cronometro imediatamente** a cada transicao
   (iniciar/pausar/continuar/encerrar) para permitir recuperacao apos
   fechamento inesperado.
4. **Calcular duracao no backend a partir de timestamps persistidos**, nunca so
   por contador do frontend. Robustez a mudanca do relogio: delta negativo
   nunca reduz tempo acumulado (`domain::timer::elapsed_seconds`).
5. **O banco preserva sempre o tempo real.** Arredondamento e desconto de
   inatividade sao aplicados apenas na visualizacao/cobranca, nunca
   sobrescrevendo o valor original.
6. **`hourly_rate_snapshot_cents`** preserva o valor/hora no momento da sessao;
   alterar o valor atual do projeto nao altera sessoes anteriores.
7. **Valores monetarios em centavos (inteiros).** Duracoes em segundos.
8. **Nunca encerrar/descartar tempo silenciosamente.** Recuperacao,
   inatividade e fechamento de CAD sempre pedem decisao ao usuario.
9. **Seguranca Tauri:** permissoes minimas (`capabilities/default.json`); **sem
   SQL arbitrario exposto ao frontend**; comandos especificos e validados.
10. **Nome do produto centralizado** em `src/config/app.ts` (`APP.name`).

## Processo de migrations

1. Crie `src-tauri/migrations/NNNN_descricao.sql` com versao crescente.
2. Registre em `src-tauri/src/database/mod.rs` (`migrations()`), com
   `version` = NNNN e `include_str!` do arquivo.
3. **Nunca** edite uma migration ja aplicada em producao; crie uma nova.

## Padroes de testes

- Regras de dominio testadas dos dois lados: `src/lib/*.test.ts` (Vitest) e
  `src-tauri/src/domain/timer.rs` (`#[cfg(test)]`).
- Cobrir: duracao, pausa/retomada, encerramento, recuperacao, arredondamento,
  calculo monetario, desconto de inatividade, unico cronometro ativo, snapshot
  de valor/hora, horarios invalidos, sessao atravessando meia-noite, filtros.
- Checklist manual em `docs/UX-FLOWS.md`.

## Acesso ao banco (backend)

- O plugin conecta/migra no startup via `preload` (`tauri.conf.json`).
- Comandos usam `database::pool(&db)` para pegar o pool `sqlx` compartilhado e
  chamam a camada `repository` (consultas tipadas, valores via `bind`).
- Nunca conceder `sql:*` ao webview nem montar SQL no frontend. Fluxo:
  `store -> service (invoke) -> command (valida) -> repository -> SQLite`.

## Estado atual

**MVP completo (Fases 0-8).** Cadastros; motor do cronometro com recuperacao;
monitoramento de processos; inatividade; historico/relatorios (manual, edicao,
soft delete/restore, filtros, arredondamento na visualizacao, CSV via dialogo
nativo, impressao); e **reconstrucao do dia** (linha do tempo real +
`source = reconstructed`). Sem mocks. Alem do MVP: metas de horas por projeto,
inicio em 1 clique, fatura por cliente em PDF, ajuste percentual, emissor nas
configuracoes, modo claro persistente, onboarding, **edicao/exclusao de sessao
direto do Painel** e **aviso "Conferir?"** em sessoes de cronometro acima de 8h
ou que atravessam a madrugada (`src/lib/suspiciousEntry.ts`). Migrations
atuais: 0001 a 0005. Guia do usuario em `docs/USAGE.md`. Ver `docs/ROADMAP.md`.
