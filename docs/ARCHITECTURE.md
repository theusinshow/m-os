# M/OS — Technical Architecture

## 1. Status

**Status:** aprovada para a fundação técnica da v0.1

**Data:** 2026-08-13

**Escopo:** fundação técnica, sem implementação de produto

Esta arquitetura é subordinada a `VISION.md`, `PRODUCT.md`, `CORE.md` e `UX-PRINCIPLES.md`.

Seu objetivo é sustentar a evolução do M/OS sem tentar implementar antecipadamente sua visão final.

Foi revisada de forma independente, aprovada pelo usuário e confirmada pelo spike registrado em `TECHNICAL-SPIKE-DESKTOP-SHELL.md`. Decisões de cloud, sync e iOS continuam deliberadamente abertas ou adiadas.

## 2. Decisões de contexto confirmadas

- plataforma mobile futura: iOS;
- primeira entrega: foco total em Windows desktop;
- não haverá um produto offline-first completo nem sync sofisticado na primeira entrega;
- o núcleo desktop local deve permitir capturar e consultar sem depender da cloud;
- iOS poderá exigir conexão quando for introduzido;
- cloud é aceitável para autenticação, backup, sincronização e integrações;
- self-hosting não é requisito;
- sensibilidade inicial dos dados é razoável, sem exigência regulatória especial;
- Capture preserva origem e mantém proveniência das entidades derivadas.

## 3. Drivers arquiteturais

Em ordem de prioridade:

1. não perder Captures;
2. abrir e responder rapidamente;
3. comportar-se como programa Windows real;
4. permanecer operável localmente sem login ou cloud;
5. oferecer busca confiável;
6. preservar clareza do Core;
7. permitir evolução posterior para cloud, iOS, integrações e Hermes;
8. evitar infraestrutura e abstrações empresariais prematuras.

## 4. Arquitetura proposta

O M/OS começará como um **monólito modular desktop, local-first no sentido de autoridade local**, composto por:

```text
┌──────────────────────────────────────────────────────┐
│ Windows Desktop App                                  │
│                                                      │
│  React + TypeScript UI                               │
│             │ typed commands / events                │
│             ▼                                        │
│  Tauri application boundary                          │
│             │                                        │
│             ▼                                        │
│  Rust application + domain modules                   │
│             │                                        │
│       ┌─────┴──────────┐                             │
│       ▼                ▼                             │
│  SQLite + FTS5    Windows adapters                    │
│                   shortcut / tray / window / launch   │
└──────────────────────────────────────────────────────┘

Cloud services: ausentes na v0.1 e adicionados por portas explícitas.
iOS: cliente posterior consumindo contratos de aplicação/cloud, não o banco local do desktop.
```

## 5. Stack recomendada

### 5.1 Desktop shell: Tauri 2

Tauri 2 é a recomendação inicial porque oferece:

- aplicativo instalável e janelas próprias no Windows;
- acesso a tray e atalhos globais;
- fronteira explícita entre WebView e comandos nativos;
- uso do WebView2 já presente em versões modernas do Windows;
- bundle menor que uma aplicação que distribui seu próprio Chromium;
- backend Rust adequado para persistência, sistema operacional e integrações;
- possibilidade de manter a UI visual altamente customizada.

A escolha não significa utilizar Tauri para o iOS. Compartilhar shell ou UI não é um objetivo arquitetural.

### 5.2 Renderer: React + TypeScript

Responsabilidades:

- apresentação;
- estado efêmero de interface;
- keyboard navigation;
- feedback imediato;
- projeções visuais;
- chamadas a comandos tipados.

O renderer não deve:

- executar SQL;
- conhecer paths privados de persistência;
- armazenar tokens em `localStorage`;
- implementar regras de domínio críticas;
- acessar livremente filesystem ou rede.

React foi escolhido como proposta por adequação a uma aplicação com múltiplas projeções densas, navegação por teclado, design system próprio e necessidade de testes de componentes. Não foi escolhido por popularidade.

Comparação do renderer:

| Alternativa | Vantagens | Custos | Avaliação |
|---|---|---|---|
| React + TypeScript | composição explícita; ecossistema sólido de acessibilidade e testes; adequado a múltiplas superfícies | runtime e gestão de estado exigem disciplina | Proposto |
| Svelte | menor boilerplate e boa performance percebida | nova camada de competências; benefício pouco relevante diante do backend Rust | Alternativa válida se o spike revelar vantagem real |
| Vanilla/Custom Elements | mínimo de dependências | custo alto para estado, foco, acessibilidade e consistência em uma aplicação crescente | Rejeitado para o produto completo |
| UI inteiramente Rust | uma linguagem no core | ecossistema e refinamento visual menos adequados ao objetivo atual | Rejeitado agora |

### 5.3 Core nativo: Rust

Responsabilidades:

- comandos da aplicação;
- invariantes do domínio;
- transações;
- persistência;
- migrações;
- indexação de Search;
- adaptadores do Windows;
- backup e exportação;
- sync e integrações futuras.

O Rust Core deve ser uma biblioteca testável sem inicializar uma janela Tauri.

### 5.4 Persistência: SQLite

SQLite será a fonte de verdade do desktop.

Razões:

- transações locais;
- arquivo único administrável;
- maturidade;
- migrações versionadas;
- FTS5 para busca textual;
- ausência de serviço local separado;
- compatibilidade com backup e exportação.

O acesso será feito no backend Rust. O plugin SQL exposto diretamente ao renderer não será usado como camada de aplicação.

### 5.5 Search: SQLite FTS5

A primeira busca será lexical e local.

Ela deve indexar:

- Capture;
- Task;
- Project;
- contexto necessário para identificar o resultado.

O índice é reconstruível e não contém estado exclusivo.

Busca vetorial, embeddings e semantic search estão fora da fundação inicial.

## 6. Comparação de alternativas desktop

| Alternativa | Vantagens | Custos e riscos | Decisão |
|---|---|---|---|
| WinUI 3 + C# | UI nativa; APIs Windows diretas; UI Automation e packaging do ecossistema Microsoft | customização e iteração visual concentradas em XAML; lock-in Windows; lacunas de tooling precisam ser avaliadas | Reserva se o spike Tauri falhar em comportamento nativo ou acessibilidade |
| Tauri 2 + web UI + Rust | shell leve; UI flexível; shortcut/tray; separação de privilégios; SQLite local | WebView2; duas linguagens; plugins, IME, UI Automation, foco e packaging exigem QA contínuo | Aceito para a v0.1 após o spike |
| Electron + web UI | Ecossistema maduro; APIs desktop estáveis; depuração simples | distribui Chromium; maior consumo de memória e armazenamento; desktop-only | Alternativa de contingência |
| PWA/browser | desenvolvimento simples | viola programa desktop real; captura global e tray dependem de workarounds; cloud tende a virar requisito | Rejeitada |
| .NET MAUI | C# compartilhado entre Windows e iOS | compartilhamento não elimina código nativo de extensions; desktop-first ficaria condicionado à abstração cross-platform | Não recomendado para a primeira fase |

## 7. Scorecard e spike obrigatórios

Tauri não seria aceito apenas por demonstrar viabilidade. A decisão usou o seguinte scorecard:

| Critério | Peso | Gate obrigatório |
|---|---:|---|
| Quick Capture, foco, tray e lifecycle do processo | 25 | Sim |
| teclado, Narrator, UI Automation, high contrast, scaling e IME | 20 | Sim |
| durabilidade e integração SQLite | 15 | Sim |
| performance, memória e tempo de abertura | 15 | Não |
| velocidade de desenvolvimento e manutenção em duas linguagens | 10 | Não |
| packaging, assinatura e atualização | 10 | Sim |
| capacidade de executar least privilege | 5 | Sim |

Critério de aceitação:

- todos os gates obrigatórios passam;
- score total mínimo de 75/100;
- nenhum defeito conhecido compromete Capture, acessibilidade ou confiança;
- custos de Rust, WebView2 e duas linguagens são registrados com honestidade;
- WinUI 3 é avaliado para qualquer gate que Tauri não cumpra.

Cada critério recebe nota inteira de 0 a 5:

- `0`: não testado ou não funciona;
- `1`: falha estrutural, sem workaround aceitável;
- `2`: funciona parcialmente, com risco alto;
- `3`: atende o mínimo, com limitações documentadas;
- `4`: atende bem o uso alvo;
- `5`: atende integralmente e sem risco relevante conhecido.

O valor ponderado é `peso × nota ÷ 5`. Um gate obrigatório só passa com nota mínima 4.

Evidência obrigatória por critério:

- procedimento reproduzível;
- ambiente, versão e configuração;
- resultado esperado e observado;
- medidas quando aplicáveis;
- screenshots, logs redigidos ou output de ferramenta;
- defeitos e workarounds;
- nota justificada;
- comparação WinUI 3 quando Tauri obtiver menos de 4 em gate obrigatório.

O resultado está registrado em `TECHNICAL-SPIKE-DESKTOP-SHELL.md`. A ADR-006 foi aceita por evidência dos gates, não por percepção informal ou apenas pelo score agregado.

Para passar de proposta para aceita, a decisão exigiu que um spike descartável provasse no Windows alvo:

1. iniciar uma aplicação single-instance;
2. registrar atalho global configurável;
3. abrir Quick Capture sobre outro programa;
4. focar o campo sem roubar estado indevidamente;
5. persistir Capture em transação SQLite;
6. fechar e devolver foco ao programa anterior;
7. operar via tray;
8. diferenciar fechar janela, esconder no tray e encerrar processo;
9. detectar conflito de atalho e permitir reconfiguração;
10. empacotar, instalar e desinstalar sem perder dados pessoais;
11. validar Narrator, UI Automation, high contrast, IME, teclado, scaling e múltiplos monitores;
12. medir memória, cold start, warm open e tempo do atalho até o input;
13. validar WebView2 bootstrap e comportamento sem runtime adequado;
14. documentar requisitos de assinatura e distribuição.

O spike não deve conter UI ou regras definitivas de produto.

### 7.1 Lifecycle do processo desktop

Na primeira entrega:

- abrir M/OS inicia um único processo;
- fechar a janela principal esconde a interface e mantém o processo no tray;
- `Quit` no menu do tray encerra o processo;
- o atalho global funciona somente enquanto o processo está ativo;
- a interface nunca afirma que o atalho está disponível depois de `Quit`;
- iniciar automaticamente com Windows está fora da v0.1 até ser promovido do `IDEAS.md` por necessidade observada.

## 8. Fronteiras modulares

### Capture

- criar Capture;
- consultar Capture;
- listar Inbox;
- processar, arquivar e restaurar;
- manter proveniência.

### Projects

- criar e editar Project;
- consultar contexto básico;
- arquivar e restaurar.

### Tasks

- criar e editar Task;
- mudar estado;
- relacionar a Project;
- concluir e restaurar.

### Search

- manter índice derivado;
- consultar múltiplos tipos;
- abrir resultado em sua origem.

### Desktop

- janela principal;
- Quick Capture;
- tray;
- global shortcuts;
- single instance;
- abertura de URLs e programas por políticas explícitas.

### Storage

- conexão SQLite;
- transações;
- migrations;
- backup;
- exportação;
- integridade.

### Future boundaries

- Workspaces;
- Resources;
- App Registry;
- Time;
- Sync;
- Integrations;
- Hermes.

Essas fronteiras futuras podem possuir interfaces mínimas de arquitetura quando necessário, mas não módulos vazios, tabelas ou serviços antecipados.

## 9. Camadas e dependências

```text
UI
  -> Application Commands / Queries
      -> Domain
      -> Ports
          -> SQLite adapters
          -> Windows adapters
          -> future Cloud adapters
```

Regras:

- Domain não depende de Tauri, React, SQLite ou cloud;
- Application coordena casos de uso e transações;
- adapters implementam persistência e sistema operacional;
- UI não importa adapters;
- eventos internos não substituem chamadas diretas quando não existe desacoplamento real a ganhar.

Não serão criados microserviços, message broker ou service mesh.

## 10. Comandos e queries

A fronteira Tauri deve expor casos de uso, não CRUD genérico.

Exemplos:

```text
capture.create(input)
capture.process_as_task(capture_id, task_input)
capture.archive(capture_id)
inbox.list(query)
task.change_status(task_id, status)
project.create(input)
search.query(text)
```

Comandos retornam resultados tipados e erros compreensíveis. Operações que mudam várias entidades utilizam uma transação.

Hermes futuro chamará essa mesma fronteira de aplicação por um adapter autorizado.

## 11. Persistência e integridade

### 11.1 Estrutura

- database no diretório de dados da aplicação;
- migrations numeradas e somente para frente;
- foreign keys habilitadas;
- `journal_mode=WAL`;
- `synchronous=FULL`;
- `foreign_keys=ON` em todas as conexões;
- `busy_timeout` curto e medido, sem mascarar contenção persistente;
- valores de PRAGMA lidos e verificados na abertura;
- backup consistente antes de migration destrutiva;
- índices medidos a partir de queries reais.

`synchronous=FULL` em WAL adiciona sincronização do WAL depois de cada commit e é a baseline de durabilidade para Capture. Uma mudança para `NORMAL` exige nova ADR, medição e revisão explícita da promessa de confiança.

Checkpoint deve ocorrer fora do caminho crítico de Capture, durante idle, fechamento limpo ou backup. A política final será medida no spike; durabilidade não pode depender de copiar arquivos WAL manualmente.

### 11.2 Capture commit

Uma Capture só aparece como salva após `COMMIT` bem-sucedido.

O fluxo é:

```text
input -> validate minimally -> begin transaction -> insert -> commit -> feedback
```

Na v0.1, alteração de entidade e atualização da projeção FTS ocorrem na mesma transação SQLite. Se a indexação falhar, o comando inteiro falha e não produz feedback de sucesso.

O índice continua reconstruível. Na abertura, uma verificação barata de versão/contagem detecta necessidade de rebuild; uma rotina explícita de integridade cobre divergências mais profundas. Rebuild nunca altera as entidades fonte.

### 11.3 Contrato de durabilidade

Depois do feedback `Saved`, a v0.1 deve preservar a Capture diante de:

- crash do renderer;
- encerramento forçado do processo;
- crash do processo nativo;
- reinício inesperado do Windows;
- perda de energia, dentro das garantias oferecidas pelo filesystem e hardware ao `fsync`.

O sistema não deve confirmar salvamento quando ocorrer:

- disco cheio;
- database bloqueado além do timeout;
- erro de I/O;
- falha de constraint;
- transação não confirmada.

Corrupção preexistente ou falha física do dispositivo não podem ser resolvidas apenas pelo commit. Na abertura, o produto deve detectar erro de integridade, interromper writes e orientar recuperação a partir de backup sem sobrescrever a cópia danificada.

Testes de fault injection devem cobrir process kill antes e depois do commit, disk full, lock contention, abertura de database corrompido e restore do último backup válido. Teste de queda de energia real pode ser substituído por garantias documentadas do SQLite mais teste controlado de reinício do sistema.

### 11.4 Migrações

Toda migration deve possuir:

- teste sobre database vazio;
- teste de upgrade da versão anterior;
- verificação de integridade;
- estratégia de backup;
- documentação quando houver perda ou transformação de informação.

## 12. Performance e experiência

Budgets iniciais, a validar no hardware alvo:

- feedback de Capture após commit local: imperceptível em uso normal;
- Quick Capture quente: pronta para digitação em até 300 ms;
- busca local comum: primeiros resultados em até 100 ms no volume pessoal esperado;
- digitação nunca bloqueada por indexação ou I/O não essencial;
- animações repetitivas curtas e canceláveis;
- nenhuma requisição de rede no caminho crítico da Capture v0.1.

Esses números são budgets de engenharia, não garantias de produto até medição.

## 13. Cloud e sincronização futura

### 13.1 Papel da cloud

A cloud poderá fornecer:

- autenticação;
- cópia remota e backup;
- sincronização;
- armazenamento de anexos;
- endpoints para iOS;
- integrações externas;
- execução futura de Hermes.

Ela não será necessária para iniciar o desktop, capturar, organizar ou buscar dados locais.

### 13.2 Estratégia

Quando iOS ou backup remoto entrar no roadmap técnico:

```text
Desktop SQLite
    -> change journal / push
Cloud API + Postgres
    -> pull changes
iOS client (online)
```

Características propostas:

- IDs globais gerados no cliente;
- Captures append-first;
- sem CRDT genérico na primeira implementação de sync.

Revisões, change journal, tombstones, cursores, idempotência e conflito serão definidos juntos em uma ADR de sync. Antecipar apenas parte desse contrato seria mais perigoso que adiá-lo.

Quando iOS for introduzido, a cloud deverá ordenar e mediar o dataset compartilhado. O SQLite permanece a fonte operacional local do desktop, mas deixa de ser a única autoridade do ecossistema. Essa mudança de autoridade exige migration e ADR próprias.

O produto não promete modo offline completo em múltiplos dispositivos. O desktop continua local por construção; o iOS poderá exigir rede.

### 13.3 Cloud gerenciada

Um backend gerenciado baseado em Postgres, Auth e object storage é adequado ao produto pessoal. Supabase é o candidato inicial porque reúne essas capacidades e possui SDK Swift.

Essa é uma escolha de fornecedor adiada. O domínio não deve importar SDKs de cloud; um adapter e uma API estreita devem proteger o Core do fornecedor.

Alternativas:

| Abordagem | Avaliação |
|---|---|
| Supabase gerenciado | boa relação entre Auth, Postgres, Storage e operação reduzida; candidato preferencial |
| API própria + Postgres gerenciado | maior controle; mais deploy, auth, observabilidade e manutenção |
| Firebase/Firestore | sync e mobile maduros; modelo documental distante do SQLite relacional e maior acoplamento |
| CloudKit | boa integração iOS; inadequado como centro de um desktop Windows |
| self-hosting | controle máximo; custo operacional sem justificativa para o escopo pessoal |

## 14. iOS futuro

O iOS será um companion e não uma versão comprimida do desktop.

Recomendação:

- SwiftUI para o aplicativo;
- Share Extension nativa;
- App Group e shared container para handoff durável entre Share Extension e aplicativo;
- Keychain para credenciais;
- cliente online da API cloud;
- superfícies prioritárias: Capture, Share, Inbox, consulta rápida e, depois, voz.

`Online-only` significa que consulta e sync podem exigir rede. Não significa captura volátil. A Share Extension só confirma sucesso depois de receber ack remoto ou persistir uma entrega pendente no shared container. Upload em background deve utilizar configuração compatível com App Group.

Não haverá obrigação de compartilhar componentes visuais ou código de domínio com o desktop. Serão compartilhados:

- linguagem do domínio;
- contratos da API;
- schemas serializados;
- casos de uso;
- testes de contrato.

## 15. Segurança e privacidade

### 15.1 Baseline local

- database no perfil do usuário do Windows;
- confiar inicialmente na proteção de conta do sistema e criptografia de disco disponível;
- sem criptografia própria do database na v0.1;
- sem login obrigatório para uso local;
- logs sem conteúdo de Captures por padrão;
- exports e backups tratados como dados privados.

### 15.2 Threat model inicial

O baseline pretende proteger contra:

- acesso casual por outra conta padrão do Windows;
- vazamento acidental por logs;
- interceptação de rede quando cloud existir;
- uso indevido de tokens fora do escopo concedido;
- perda lógica recuperável por backup.

O baseline depende de:

- conta Windows bloqueada quando o usuário se afasta;
- ACLs do perfil de usuário;
- BitLocker ou proteção equivalente para dados em repouso contra perda física;
- sistema operacional e dependências atualizados.

O baseline não protege contra:

- malware ou administrador local no dispositivo desbloqueado;
- extração física quando criptografia de disco está desativada;
- conteúdo exposto voluntariamente em export não criptografado;
- comprometimento da conta cloud futura.

A decisão final de não criptografar o database permanece proposta até confirmar BitLocker, permissões dos arquivos, comportamento de backups e exports e procedimentos de recovery.

### 15.3 Tauri

- frontend empacotado localmente;
- Content Security Policy restritiva;
- sem execução de scripts remotos;
- capabilities por janela;
- Quick Capture recebe apenas permissões necessárias;
- acesso a filesystem e rede bloqueado por padrão no renderer;
- comandos Rust validam input e autorização de escopo;
- URLs externas abrem no navegador do sistema.

### 15.4 Cloud e integrações

- TLS;
- tokens em Windows Credential Manager e iOS Keychain;
- menor escopo possível por integração;
- Row Level Security quando aplicável;
- secrets de serviço nunca distribuídos ao cliente;
- revogação e remoção de credenciais;
- ação externa destrutiva exige confirmação e registro.

A integração futura com dados financeiros exige nova avaliação de ameaça antes de expor dados do M-Finance à cloud ou Hermes.

## 16. Backup, exportação e propriedade

Antes de sincronização cloud, a arquitetura deve permitir:

- backup consistente do database local;
- restauração validada;
- exportação legível e versionada dos dados principais;
- identificação da versão do schema;
- recuperação depois de migration malsucedida.

Copiar apenas o arquivo principal enquanto WAL está ativo não é uma estratégia de backup válida. O backup deve usar mecanismo consistente do SQLite ou checkpoint controlado.

Política proposta para v0.1a:

- snapshot interno consistente antes de toda migration;
- snapshot diário após a primeira alteração do dia;
- retenção dos últimos sete snapshots diários;
- backup manual em arquivo `.mos-backup` escolhido pelo usuário;
- manifest com versão do schema e checksums;
- restore sempre cria antes um safety backup do estado atual;
- restore substitui o dataset local inteiro; merge não faz parte da v0.1;
- export JSON versionado é distinto de backup e não promete reimportação;
- UI avisa que backup e export podem conter conteúdo pessoal em texto claro.

A rotina deve produzir o backup pela API de backup do SQLite ou mecanismo equivalente consistente, nunca por cópia ingênua dos arquivos ativos.

## 17. Testes arquiteturais

### Domain

- lifecycle de Capture;
- proveniência;
- transições válidas de Task;
- regras de Archive e Trash.

### Persistence

- atomicidade;
- foreign keys;
- migrations;
- rebuild de Search;
- backup e restore.

### Desktop

- global shortcut;
- focus e fechamento de Quick Capture;
- single instance;
- tray;
- scaling e teclado;
- instalação limpa e upgrade.

### Fault model

- renderer crash;
- process kill;
- native crash;
- restart inesperado;
- disk full;
- lock contention;
- database corrompido na abertura;
- backup inválido e restore válido.

### Contract

- comandos tipados entre renderer e Rust;
- API cloud futura;
- serialização compatível entre desktop e iOS.

## 18. Observabilidade

Na v0.1:

- logs locais estruturados;
- níveis de log;
- correlation ID por comando;
- redaction de conteúdo pessoal;
- diagnóstico exportável voluntariamente;
- nenhum analytics remoto por padrão.

Métricas técnicas locais podem medir tempo de abertura, Capture e Search sem registrar conteúdo.

## 19. Riscos e controles

| Risco | Controle |
|---|---|
| Tauri não alcançar comportamento esperado de Quick Capture | spike obrigatório e fallback WinUI/Electron |
| regras de domínio vazarem para UI | comandos de aplicação e testes no Rust Core |
| perda de Capture | commit local atômico, backup e testes de falha |
| FTS ficar inconsistente | índice reconstruível e rotina de verificação |
| schema antecipar visão final | somente relações usadas, migrations incrementais |
| cloud contaminar Core | ports/adapters e fornecedor adiado |
| sync virar projeto independente | só iniciar junto de caso de uso iOS/backup definido |
| UI parecer site em janela | design foundations próprias e validação desktop real |
| Rust aumentar custo inicial | Core pequeno, módulos diretos e sem framework interno |
| confirmação de Capture prometer durabilidade indefinida | fault model explícito, WAL + FULL e backup consistente |

## 20. Itens explicitamente não adotados

- microservices;
- banco de grafos;
- event sourcing completo;
- CQRS distribuído;
- CRDT genérico;
- vector database;
- plugin marketplace;
- execução dinâmica de código de terceiros;
- Kubernetes;
- self-hosting obrigatório;
- monorepo com todos os Apps independentes;
- autenticação antes de existir cloud;
- abstração cross-platform de UI.

## 21. Referências técnicas consultadas

- [Tauri — Prerequisites](https://v2.tauri.app/start/prerequisites/)
- [Tauri — Global Shortcut](https://v2.tauri.app/reference/javascript/global-shortcut/)
- [Tauri — System Tray](https://v2.tauri.app/learn/system-tray/)
- [Tauri — Capabilities](https://v2.tauri.app/security/capabilities/)
- [Tauri — SQL](https://v2.tauri.app/plugin/sql/)
- [Tauri — Windows Installer](https://v2.tauri.app/distribute/windows-installer/)
- [Microsoft — Windows App SDK](https://learn.microsoft.com/en-us/windows/apps/windows-app-sdk/)
- [Electron — globalShortcut](https://www.electronjs.org/docs/latest/api/global-shortcut)
- [Electron — Tray](https://www.electronjs.org/docs/latest/api/tray/)
- [SQLite — FTS5](https://www.sqlite.org/fts5.html)
- [SQLite — Write-Ahead Logging](https://www.sqlite.org/wal.html)
- [SQLite — PRAGMA synchronous](https://sqlite.org/pragma.html#pragma_synchronous)
- [Apple — Configuring App Groups](https://developer.apple.com/documentation/xcode/configuring-app-groups/)
- [Apple — App Extension Scenarios](https://developer.apple.com/library/archive/documentation/General/Conceptual/ExtensibilityPG/ExtensionScenarios.html)
- [Supabase — Auth Architecture](https://supabase.com/docs/guides/auth/architecture)
- [Supabase — Storage](https://supabase.com/docs/guides/storage)

## 22. Gate para implementação

Gate concluído em 2026-08-13:

- [x] revisão independente deste documento e de `CORE-FOUNDATION.md`;
- [x] decisão explícita sobre as críticas da revisão;
- [x] aprovação do escopo em `V0.1-SCOPE.md`;
- [x] design foundations com fluxos, navegação, estados e accessibility baseline;
- [x] spike técnico com score 81/100 e todos os gates obrigatórios aprovados;
- [x] ADRs da fundação local aceitas.

Está autorizado iniciar a fundação técnica de `v0.1a`. Isso não autoriza cloud, sync, mobile, integrações, Workspaces, App Registry ou outros itens fora do corte aprovado.
