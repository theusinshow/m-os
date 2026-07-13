# CronoCAD

Rastreador de horas **local-first** para desenhistas e projetistas de CAD.
Aplicativo desktop para Windows que reduz o esquecimento de registrar o
trabalho: cronometro por projeto, deteccao de programas CAD, deteccao de
inatividade, reconstrucao aproximada do dia e relatorios de horas/valores.

> Status: **MVP completo (Fases 0-8)**. Cronometro com recuperacao,
> monitoramento de programas, inatividade, historico/relatorios com CSV e
> reconstrucao do dia. Guia de uso em [docs/USAGE.md](docs/USAGE.md);
> planejamento em [docs/ROADMAP.md](docs/ROADMAP.md).

## Stack

Tauri 2 · React 18 · TypeScript · Vite 6 · SQLite (`tauri-plugin-sql`) ·
Tailwind CSS 3 · React Router · Zustand · Zod · lucide-react · Vitest ·
testes nativos Rust.

## Pre-requisitos

- **Node.js** 18+ e **npm**.
- **Rust** (stable, toolchain `x86_64-pc-windows-msvc`) — instale via
  [rustup](https://rustup.rs).
- **Microsoft C++ Build Tools** (workload "Desktop development with C++",
  inclui o linker MSVC e o Windows SDK).
- **WebView2 Runtime** (ja presente no Windows 11).

## Desenvolvimento

```bash
npm install          # instala dependencias do frontend
npm run tauri:dev    # roda o app desktop (Vite + backend Tauri, com recarga)
```

Somente frontend (UI no navegador, sem backend):

```bash
npm run dev          # http://localhost:1420
```

## Validacao

```bash
npm run typecheck    # checagem de tipos (TS estrito)
npm run lint         # ESLint (0 warnings)
npm run test         # Vitest (regras de dominio + render)
npm run build        # build de producao do frontend

# backend (em src-tauri/)
cargo test           # regras de dominio do cronometro
cargo fmt            # formatacao
cargo clippy         # lint Rust
```

## Build do instalador (Windows)

```bash
npm run tauri:build  # gera o executavel e o instalador NSIS
```

Saida em `src-tauri/target/release/` e o instalador em
`src-tauri/target/release/bundle/nsis/`.

## Estrutura

```
src/                 Frontend React
  app/               App, router, rotas, providers
  components/        layout/ e ui/ (primitivos reutilizaveis)
  features/          dashboard, projects, history, reports, settings, timer
  lib/               regras puras (duracao, arredondamento, dinheiro, formato)
  services/          ponte Tauri + dados temporarios (fase atual)
  stores/            estado de UI (Zustand)
  types/             tipos de dominio e contratos de eventos
  styles/            tokens.css (unica fonte de valores visuais) + global.css
  config/            nome do app e padroes
src-tauri/           Backend Rust (Tauri 2)
  src/               commands, domain, database, monitoring, notifications,
                     tray, state
  migrations/        SQL versionado
  capabilities/      permissoes minimas
docs/                PRODUCT, ARCHITECTURE, DATABASE, UX-FLOWS, ROADMAP, USAGE
CLAUDE.md            guia de convencoes e regras criticas
```

## Privacidade

O app **nao** faz captura de tela, keylogging, leitura de arquivos CAD nem
telemetria. Registra apenas metadados de tempo/processo necessarios ao controle
de horas. Todos os dados ficam **localmente** em SQLite; nada depende de
internet.

## Licenca

Projeto privado. Definir licenca conforme necessidade.
