# M/OS

M/OS e um sistema pessoal desktop-first para capturar, organizar e reencontrar
contexto com baixa friccao. A implementacao atual e o corte local `v0.1` para
Windows, do pensamento capturado ate a acao organizada.

## Estado atual

- aplicativo nativo Tauri para Windows;
- Quick Capture global, janela principal, single instance e tray;
- SQLite local com WAL, `synchronous=FULL` e FTS5;
- Inbox, Archive e Trash recuperaveis;
- Projects e Tasks com proveniencia explicita de Capture;
- Kanban simples com Backlog, Doing e Done;
- Search unificada e agrupada para Captures, Tasks e Projects;
- backup `.mos-backup`, restore validado e safety backup;
- export JSON legivel e versionado;
- design system proprio com temas dark e light;
- nenhuma rede no caminho de captura ou consulta.

Cloud, sincronizacao e iOS nao fazem parte deste marco.

## Executar

Pre-requisitos: Node.js, Rust e dependencias de desenvolvimento do Tauri 2 para
Windows.

```powershell
cd apps\desktop
npm ci
npm run tauri dev
```

## Verificar

```powershell
cargo test --workspace
cd apps\desktop
npm run build
npm run tauri build
```

O pacote Windows local gera um instalador NSIS por usuario, sem admin por
padrao, em `target\release\bundle\nsis`. O binario em `target\release` tambem
pode ser usado como artefato portatil em maquinas que ja tenham WebView2.

O workflow `Package Windows` publica o instalador NSIS e o executavel portatil
por acionamento manual ou tag `v*`.

As decisoes de produto e arquitetura ficam em [`docs`](docs). Leia
`AGENTS.md` e a documentacao de produto antes de ampliar o escopo.
