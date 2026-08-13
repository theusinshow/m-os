# M/OS — Desktop Shell Technical Spike

## 1. Status

**Status:** concluído — Tauri 2 aceito para a fundação da v0.1

**Data:** 2026-08-13

**Escopo:** experimento descartável; nenhum código deste diretório é código de produto

**Implementação:** `spikes/desktop-shell`

## 2. Objetivo

Testar os riscos que poderiam invalidar Tauri 2 como shell Windows antes de iniciar o M/OS:

- Quick Capture independente da janela principal;
- atalho global, foco, tray e single-instance;
- persistência SQLite durável e busca FTS5;
- acessibilidade e comportamento de teclado;
- instalação, desinstalação e preservação de dados;
- custo de startup, memória e manutenção em duas linguagens;
- least privilege na fronteira entre WebView e Rust.

O spike não implementa Inbox, Projects, Tasks, proveniência, backup ou qualquer outro fluxo definitivo do produto.

## 3. Ambiente

| Item | Valor observado |
|---|---|
| Sistema | Windows 11 Pro x64, build 10.0.26200 |
| WebView2 Runtime | 147.0.3912.98 |
| Tauri | 2.11.5 |
| Rust | 1.97.0, target `x86_64-pc-windows-msvc` |
| Node.js | 24.15.0 |
| npm | 11.12.1 |
| SQLite | 3.53.2, compilado via `rusqlite` bundled |
| Renderer | React 19 + TypeScript + Vite |

## 4. Matriz de evidências

| Capacidade | Procedimento | Resultado observado | Estado |
|---|---|---|---|
| Single-instance | iniciar o executável release e iniciar uma segunda cópia | segunda cópia encerrou e reabriu a janela do processo original | Passou |
| Quick Capture | acionar `Ctrl+Shift+Space` com outro aplicativo ativo | janela separada abriu sempre no topo e recebeu entrada | Passou |
| Tray | fechar janela principal; abrir ícones ocultos; acionar `Captura rapida` no menu | processo permaneceu ativo e o item do tray abriu Quick Capture | Passou |
| Lifecycle | fechar janela principal, consultar processo e depois executar novamente | fechar escondeu a janela; processo continuou responsivo; segunda execução revelou a instância | Passou |
| Atalho configurável | informar atalho inválido `Not+A+Shortcut` | falha foi exibida e `Ctrl+Shift+Space` foi registrado novamente | Passou |
| Commit local | salvar Capture pela Quick Capture | confirmação após commit em 1–2 ms nos testes interativos | Passou |
| Atomicidade FTS | remover a tabela FTS durante teste e tentar salvar | comando falhou e a linha de Capture foi revertida | Passou |
| Busca local | salvar e consultar termos pela tela e pelos testes Rust | resultado retornado pelo FTS5 sem rede | Passou |
| Unicode | persistir e consultar `ação`; inserir `Ação rápida no iOS 日本語` pela UI Automation | conteúdo preservado; teste automatizado de diacríticos passou | Passou |
| UI Automation | inspecionar as duas janelas com Orca computer-use | headings, regions, buttons, inputs, labels, status e listas foram expostos semanticamente | Passou |
| Teclado | revisar tab order e handlers de Enter, Shift+Enter e Escape; operar menu do tray por teclado | contratos presentes e menu do tray acionável; automação de foco do provider impediu uma medição estável do Enter na textarea | Passou com ressalva de QA |
| High contrast | usar controles semânticos e CSS `forced-colors: active` | tokens deixam de impor cores e retornam às system colors | Passou para arquitetura; QA visual permanece |
| Scaling e monitores | abrir e mover foco entre monitores com escalas diferentes durante UI Automation | janelas permaneceram utilizáveis; screenshots foram capturados em escalas 0,85 e 1,0 | Passou |
| CSP/capabilities | executar release com CSP explícita, sem opener e capability restrita às duas janelas | IPC necessário funcionou; nenhum acesso de rede ou filesystem foi exposto ao renderer | Passou |
| Packaging | gerar MSI e NSIS, instalar NSIS silenciosamente e iniciar instalado | build e instalação concluíram com exit code 0; instalado abriu em 270 ms | Passou |
| Preservação no uninstall | criar sentinela e banco no app-data; desinstalar silenciosamente | diretório do programa removido; banco e sentinela preservados | Passou |

### 4.1 Testes automatizados

`cargo test` executa quatro testes:

1. SQLite abre em `WAL` e `synchronous=FULL`;
2. Capture e FTS são confirmados na mesma transação;
3. Unicode e diacríticos são preservados e pesquisáveis;
4. falha de indexação reverte a Capture.

Resultado final: **4 passed, 0 failed**.

`npm run build` executou TypeScript e build de produção sem erros.

## 5. Medidas

| Medida | Resultado |
|---|---:|
| Cold start release até janela visível | 455 ms |
| Cold start do executável instalado | 270 ms |
| Reabertura warm via segunda instância | 61 ms |
| Working set após startup | 25,7 MiB |
| Private memory após startup | 4,1 MiB |
| Commit interativo de Capture | 1–2 ms |
| Executável release | 10,45 MiB |
| Instalador MSI | 3,83 MiB |
| Instalador NSIS | 2,63 MiB |
| Build release limpo observado | 78 s |
| Build release incremental observado | 27 s |

Esses números caracterizam a máquina de desenvolvimento, não um SLA. A telemetria local da v0.1 deve repetir as medidas em uso real sem registrar conteúdo.

## 6. Scorecard

Escala e pesos seguem `ARCHITECTURE.md`.

| Critério | Peso | Nota | Pontos | Gate | Justificativa |
|---|---:|---:|---:|---|---|
| Quick Capture, foco, tray e lifecycle | 25 | 4 | 20 | Passou | fluxo completo, single-instance, tray e rollback do atalho funcionaram no Windows alvo |
| Teclado, acessibilidade, contraste, scaling e IME | 20 | 4 | 16 | Passou | UI Automation semântica, teclado de tray, Unicode, forced colors e múltiplas escalas validados; smoke auditivo de Narrator permanece em QA |
| Durabilidade e integração SQLite | 15 | 5 | 15 | Passou | WAL/FULL verificado, confirmação pós-commit e rollback transacional sob falha de FTS |
| Performance, memória e abertura | 15 | 4 | 12 | Não | cold start subsegundo e working set de 25,7 MiB; não há baseline em hardware de baixa especificação |
| Desenvolvimento e manutenção em duas linguagens | 10 | 3 | 6 | Não | fronteira é clara, mas builds Rust e contratos Rust/TypeScript adicionam custo real |
| Packaging, assinatura e atualização | 10 | 4 | 8 | Passou | MSI/NSIS, install, launch e uninstall passaram; assinatura e updater pertencem à fundação de release |
| Least privilege | 5 | 4 | 4 | Passou | renderer sem SQL, rede ou opener; comandos Rust explícitos, CSP e capability por janela |
| **Total** | **100** |  | **81** | **Passou** | mínimo exigido: 75 |

Todos os gates obrigatórios receberam nota mínima 4. WinUI 3 não precisa de spike comparativo agora.

## 7. Decisão

Tauri 2 é aceito como shell Windows da v0.1.

Também ficam aceitos para a fundação inicial:

- Rust para domínio, aplicação, persistência e adapters Windows;
- React/TypeScript para renderer;
- SQLite como fonte operacional local;
- FTS5 como busca textual inicial;
- processo residente no tray enquanto o atalho estiver ativo;
- WAL com `synchronous=FULL` e confirmação somente após commit.

O código do spike não será promovido por cópia. A fundação técnica do produto deve recriar apenas os padrões aprovados, com módulos, migrations, contratos gerados e testes próprios.

## 8. Limitações e gates posteriores

Não bloqueiam o início da fundação técnica, mas bloqueiam release público:

- smoke auditivo manual com Narrator e navegação completa só por teclado;
- QA visual com Windows High Contrast real, 125%, 150% e 200%;
- teste com IME japonês real, além da inserção Unicode por UI Automation;
- código assinado e estratégia de reputação SmartScreen;
- política de updater, rollback de versão e migrations compatíveis;
- comportamento quando WebView2 não existe ou está corrompido;
- fault injection de processo, disco cheio, lock contention e banco corrompido;
- benchmark em hardware Windows de baixa especificação.

## 9. Reproduzir

```powershell
cd C:\Dev\pessoal\m-os\spikes\desktop-shell
npm install
npm run build

cd src-tauri
cargo fmt --check
cargo test

cd ..
npm run tauri dev
npm run tauri build
```

Os bundles são gerados em `src-tauri/target/release/bundle`. O diretório `target` é ignorado pelo Git.
