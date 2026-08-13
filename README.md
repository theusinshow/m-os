# M/OS

M/OS e um sistema pessoal desktop-first para capturar, organizar e reencontrar
contexto com baixa friccao. A implementacao atual e a fundacao `v0.1a` para
Windows, deliberadamente limitada a Capture, Inbox e Search locais.

## Estado atual

- aplicativo nativo Tauri para Windows;
- Quick Capture global, janela principal, single instance e tray;
- SQLite local com WAL, `synchronous=FULL` e FTS5;
- Inbox, Search, Archive e Trash de Captures;
- backup `.mos-backup`, restore validado e safety backup;
- nenhuma rede no caminho de captura ou consulta.

Tasks, Projects, Kanban, cloud e iOS nao fazem parte deste marco.

## Executar

Pre-requisitos: Node.js, Rust e dependencias de desenvolvimento do Tauri 2 para
Windows.

```powershell
cd apps\desktop
npm install
npm run tauri dev
```

## Verificar

```powershell
cargo test --workspace
cd apps\desktop
npm run build
```

As decisoes de produto e arquitetura ficam em [`docs`](docs). Leia
`AGENTS.md` e a documentacao de produto antes de ampliar o escopo.
