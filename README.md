# M/OS

M/OS e um sistema pessoal desktop-first para capturar, organizar e reencontrar
contexto com baixa friccao. A implementacao atual e a candidata local `v0.2.0`
para Windows, da captura e acao organizada ate Resources reencontraveis.

## Estado atual

- aplicativo nativo Tauri para Windows;
- Quick Capture global, janela principal, single instance e tray;
- SQLite local com WAL, `synchronous=FULL` e FTS5;
- Inbox, Archive e Trash recuperaveis;
- Projects e Tasks com proveniencia explicita de Capture;
- Daily Session: iniciar e encerrar o dia, objetivos com peso e desfecho,
  carry-over e historico (ver `docs/DAILY-SESSION.md`);
- Weekly Review: o fecho da semana em narrativa, sem placar de produtividade;
- Obsolescência: o que está parado há tempo demais, com limiar por coluna do
  Kanban (ver `DECISIONS.md`, ADR-056);
- M/Academic: semestre, disciplinas, provas, entregas, notas e estudo,
  integrados a Tasks e ao Calendário (ver `docs/ACADEMIC.md`);
- Kanban simples com Backlog, Doing e Done;
- Workspaces como contexto compartilhado entre Projects e Apps;
- App Registry local com abertura controlada de URLs e paths;
- catalogo idempotente de Apps proprios, com origem GitHub separada do alvo de abertura;
- Resources link-first com Library, nota contextual e proveniencia de Capture;
- Universal Drop Zone: arquivo, URL ou texto solto sobre a janela vira Capture
  antes de qualquer processamento, e Resource depois;
- Search unificada para Captures, Tasks, Projects, Workspaces, Apps, Resources e
  Functions, alcancando tambem o texto extraido de arquivos soltos;
- Functions de baixo risco roteadas pelo Command para os fluxos existentes;
- backup `.mos-backup`, restore validado e safety backup;
- export JSON legivel e versionado;
- atualizacao manual assinada pelo Tauri Updater;
- faixa de uso na borda da tela: quanto da janela de 5h do Claude Code ja foi,
  medido contra o proprio pico observado, com lingueta para recolher e item no
  tray para desligar (ver `DECISIONS.md`, ADR-059 e ADR-060);
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
