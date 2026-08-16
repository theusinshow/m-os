# M/OS — Architecture Decision Log

## 1. Propósito

Este documento registra decisões arquiteturais, seus motivos, consequências e gatilhos de revisão.

Estados possíveis:

- `Accepted`: decisão confirmada;
- `Proposed`: recomendação aguardando revisão;
- `Deferred`: decisão intencionalmente adiada;
- `Rejected`: alternativa analisada e não escolhida;
- `Superseded`: substituída por decisão posterior.

## 2. Índice

| ID | Decisão | Estado |
|---|---|---|
| ADR-001 | Desktop Windows é a primeira plataforma | Accepted |
| ADR-002 | Desktop local funciona sem cloud ou login | Accepted |
| ADR-003 | iOS será companion online posterior | Accepted |
| ADR-004 | Capture preserva origem e proveniência | Accepted |
| ADR-005 | Monólito modular | Accepted |
| ADR-006 | Tauri 2 como shell desktop | Accepted |
| ADR-007 | Rust Core e React/TypeScript renderer | Accepted |
| ADR-008 | SQLite como fonte de verdade desktop | Accepted |
| ADR-009 | FTS5 para Search inicial | Accepted |
| ADR-010 | Sync e cloud fora da v0.1 | Accepted |
| ADR-011 | Backend cloud gerenciado e provider-neutral | Proposed |
| ADR-012 | Sem abstração genérica de grafo ou plugin | Accepted |
| ADR-013 | Segurança proporcional à sensibilidade inicial | Proposed |
| ADR-014 | v0.1 é um corte vertical da Fase Brain | Accepted |
| ADR-015 | Estado de processamento separado do lifecycle | Accepted |
| ADR-016 | Quick Capture depende do processo no tray | Accepted |
| ADR-017 | WAL com synchronous FULL | Accepted |
| ADR-018 | Proveniência explícita de Capture para Task | Accepted |
| ADR-019 | Design system versionado como contrato do renderer | Accepted |
| ADR-020 | Origem de App é metadata, não alvo de abertura | Accepted |
| ADR-021 | Resource começa por Link e preserva contexto | Accepted |
| ADR-022 | O design v0.7 sobrepõe o handoff onde ele supõe back-end inexistente | Accepted |
| ADR-023 | Coluna de Kanban é visualização, não semântica | Accepted |
| ADR-024 | Hermes é superfície, não segundo agente | Accepted |
| ADR-025 | A conversa do Hermes é persistida localmente pelo M/OS | Accepted |
| ADR-026 | Markdown é renderizado como elementos React, nunca como HTML | Accepted |
| ADR-027 | Nada sai para o Hermes sem chip visível e registro do que foi enviado | Accepted |
| ADR-028 | A leitura do M/OS pelo Hermes começa por injeção de contexto | Accepted |
| ADR-029 | Não existem modos de conversa; o Hermes continua dono do reasoning | Accepted |
| ADR-030 | A superfície Hermes adota a direção Marginália | Accepted |
| ADR-031 | O rail carrega oito destinos, e o teto de seis vira regra de crescimento | Accepted |
| ADR-032 | Os Apps próprios entram no monorepo, com profundidade decidida por app | Accepted |
| ADR-033 | A unificação troca o valor por trás do nome, não o componente | Accepted |
| ADR-034 | A família de widgets entra pela geometria, e só onde há dado | Accepted |

## ADR-001 — Desktop Windows é a primeira plataforma

**Estado:** Accepted

### Contexto

M/OS é desktop-first e deve se comportar como um programa real. O usuário confirmou foco total no desktop para a primeira entrega.

### Decisão

Arquitetura, design foundations e primeira implementação terão Windows como único alvo.

### Consequências

- requisitos de Windows podem ser tratados como capacidades principais;
- mobile não condiciona escolhas da primeira entrega;
- código compartilhado com iOS não é critério de sucesso;
- outras plataformas desktop não são suportadas inicialmente.

### Revisar quando

O fluxo desktop `Capture → Inbox → Task → Project → Search` estiver confiável.

## ADR-002 — Desktop local funciona sem cloud ou login

**Estado:** Accepted

### Contexto

Cloud é aceitável, mas não pode ser requisito para capturar ou consultar o núcleo local.

### Decisão

O desktop possui persistência local autoritativa e inicia sem autenticação.

Não será comercializado um modo offline completo; ausência transitória de internet simplesmente não interfere no núcleo desktop local.

### Consequências

- Capture não faz request de rede no caminho crítico;
- cloud pode ser adicionada sem bloquear uso local;
- account identity e local identity precisam ser reconciliadas no futuro;
- backup local é necessário antes do backup cloud.

## ADR-003 — iOS será companion online posterior

**Estado:** Accepted

### Contexto

A plataforma mobile escolhida é iOS, mas não pertence à primeira entrega.

### Decisão

iOS será projetado posteriormente para Capture, Share e consulta rápida. Poderá exigir conexão com a cloud.

### Consequências

- não será implementado sync agora;
- app e Share Extension poderão ser nativos;
- não haverá obrigação de compartilhar UI com Windows;
- contratos e linguagem do domínio serão compartilhados.

## ADR-004 — Capture preserva origem e proveniência

**Estado:** Accepted

### Contexto

Classificar informação não deve destruir a evidência do que foi capturado.

### Decisão

Processar Capture cria entidades derivadas e relações. A Capture original permanece.

### Consequências

- conversão in-place de tipo é proibida;
- transações precisam criar derivação atomicamente;
- Search deve evitar apresentar duplicação confusa;
- exclusões não cascateiam entre origem e derivado.

## ADR-005 — Monólito modular

**Estado:** Accepted

**Aceita em:** 2026-08-13, como fronteira de implantação da v0.1.

### Contexto

O produto possui domínio amplo, mas apenas um usuário e uma primeira entrega local.

### Decisão

Começar com um processo desktop e módulos internos explícitos.

### Alternativas rejeitadas agora

- microservices;
- serviços locais separados;
- event bus distribuído;
- backend obrigatório.

### Consequências

- deploy e depuração simples;
- transações locais diretas;
- fronteiras precisam ser mantidas por dependências e testes, não por rede;
- um módulo só será extraído quando existir necessidade operacional real.

## ADR-006 — Tauri 2 como shell desktop

**Estado:** Accepted

**Aceita em:** 2026-08-13, após score 81/100 e aprovação de todos os gates do spike.

### Contexto

O produto precisa de tray, global shortcut, Quick Capture, instalação Windows e UI visualmente própria.

### Decisão

Usar Tauri 2. O condicionamento foi satisfeito pelo resultado registrado em `TECHNICAL-SPIKE-DESKTOP-SHELL.md`.

### Alternativas

- WinUI 3 como fallback para falhas de comportamento nativo;
- Electron como fallback de maturidade/ecossistema;
- PWA rejeitada.

### Consequências

- WebView2 renderiza a interface;
- APIs privilegiadas ficam em Rust;
- capabilities e CSP fazem parte da segurança;
- o spike pode rejeitar a decisão sem perda de código de produto;
- score mínimo e gates obrigatórios estão definidos em `ARCHITECTURE.md`;
- falha em acessibilidade, lifecycle, packaging ou durabilidade exige avaliar WinUI 3.

## ADR-007 — Rust Core e React/TypeScript renderer

**Estado:** Accepted

**Aceita em:** 2026-08-13, após o spike validar a fronteira Rust/React.

### Decisão

React/TypeScript responde pela interface. Rust responde por domínio, aplicação, persistência e sistema operacional.

### Consequências

- regras críticas são testáveis fora da UI;
- renderer não acessa SQL diretamente;
- contratos Tauri precisam ser tipados e versionáveis;
- existem duas linguagens, com fronteira intencional.

## ADR-008 — SQLite como fonte de verdade desktop

**Estado:** Accepted

**Aceita em:** 2026-08-13, após os testes de durabilidade e packaging do spike.

### Decisão

Persistir o Core em SQLite dentro do perfil local da aplicação.

### Consequências

- migrations e backup tornam-se responsabilidades do produto;
- não existe servidor local;
- captura pode ser atômica;
- modelo cloud posterior será uma réplica/representação, não acesso remoto ao arquivo SQLite.

## ADR-009 — FTS5 para Search inicial

**Estado:** Accepted

**Aceita em:** 2026-08-13, após busca local e atomicidade FTS serem comprovadas no spike.

### Decisão

Search v0.1 utiliza projeção FTS5 reconstruível. Capture e entidade derivada aparecem agrupadas por padrão, com a entidade ativa como resultado primário e Capture como origem subordinada.

### Consequências

- busca funciona sem cloud;
- índice precisa de rotina de rebuild e verificação;
- semantic search é adiada;
- relevância será validada com dados reais;
- conteúdo exclusivo da Capture continua contribuindo para o match sem criar duplicação visual.

## ADR-010 — Sync e cloud fora da v0.1

**Estado:** Accepted

**Aceita em:** 2026-08-13, como limite explícito do corte v0.1.

### Contexto

Não existe cliente iOS na primeira entrega e o desktop funciona localmente.

### Decisão

Não implementar autenticação, cloud, change journal remoto ou resolução de conflitos na v0.1.

O schema deve usar IDs globais. Revisões distribuídas, tombstones, cursores e conflito serão decididos juntos apenas quando existir um caso de uso real de sync.

### Consequências

- primeira entrega testa valor do Core;
- backup local não pode ser adiado junto com cloud;
- sync será desenhado contra casos reais de iOS e backup.

## ADR-011 — Backend cloud gerenciado e provider-neutral

**Estado:** Proposed

### Decisão

Quando necessário, adotar serviço gerenciado com Postgres, Auth e object storage. Supabase é o primeiro candidato, mas permanece atrás de ports/adapters e contratos próprios.

### Consequências

- operação reduzida;
- self-hosting não é obrigação;
- Core não importa SDK do fornecedor;
- escolha final requer prova de backup, exportação, custo e integração Swift.

## ADR-012 — Sem abstração genérica de grafo ou plugin

**Estado:** Accepted

**Aceita em:** 2026-08-13, como controle deliberado contra overengineering.

### Decisão

Usar relações explícitas e integrações internas até que casos reais exijam extensibilidade dinâmica.

### Consequências

- integridade referencial mais simples;
- menor superfície de segurança;
- `Everything linkable` permanece direção futura;
- novos tipos exigirão migration explícita no início.

## ADR-013 — Segurança proporcional à sensibilidade inicial

**Estado:** Proposed

### Contexto

O usuário classificou a sensibilidade inicial como razoável.

### Decisão proposta

Confiar inicialmente no perfil do Windows e na proteção de disco do sistema, sem criptografia própria do SQLite.

Aplicar least privilege, CSP, redaction de logs e armazenamento seguro de tokens quando eles existirem.

A decisão só pode ser aceita depois de validar o threat model, o estado do BitLocker, ACLs, backups, exports e recovery na máquina alvo.

### Consequências

- menor complexidade de recovery e busca;
- dispositivo desbloqueado continua sendo parte do threat model;
- integração financeira ou dados mais sensíveis exigem nova ADR.

## ADR-014 — v0.1 é um corte vertical da Fase Brain

**Estado:** Accepted

**Aceita em:** 2026-08-13, após revisão independente e aprovação do usuário.

### Contexto

A Fase Brain completa contém mais superfícies do que uma primeira entrega segura deve assumir.

### Decisão

Entregar primeiro:

```text
v0.1a  Capture → Inbox → Search de Captures → backup/restore
v0.1b  Capture → Task → Project → Search agrupada
v0.1c  Kanban simples → hardening e release v0.1
```

Workspaces e App Registry continuam na Fase Brain, mas após validação desse fluxo.

### Consequências

- primeira versão produz valor mensurável;
- Roadmap não é alterado;
- fase e release deixam de ser tratados como sinônimos;
- integrações e mobile não bloqueiam a aprendizagem inicial;
- dogfooding começa antes de Projects e Kanban estarem prontos.

## ADR-015 — Estado de processamento separado do lifecycle

**Estado:** Accepted

**Aceita em:** 2026-08-13, como parte do modelo mínimo do Core v0.1.

### Contexto

`inbox/processed` descrevem uma decisão sobre organização. `active/archived/trashed` descrevem retenção e visibilidade. Misturá-los torna Restore e Undo ambíguos.

### Decisão

Capture utiliza:

- `processing_state`: `inbox | processed`;
- `lifecycle_state`: `active | archived | trashed`.

Task utiliza estado de trabalho separado de `lifecycle_state`.

### Consequências

- restaurar preserva a decisão anterior de processamento;
- Inbox possui query inequívoca;
- Archive e Trash não alteram significado de Task ou Capture;
- transições podem ser testadas separadamente.

## ADR-016 — Quick Capture depende do processo no tray

**Estado:** Accepted

**Aceita em:** 2026-08-13, após lifecycle e tray serem validados no spike.

### Contexto

Atalho global só pode ser atendido enquanto o processo está ativo.

### Decisão

Fechar a janela principal esconde no tray. `Quit` encerra o processo e desativa o atalho. Startup com Windows não entra na v0.1.

### Consequências

- comportamento é previsível e comunicável;
- o usuário precisa iniciar M/OS após login;
- conflito de atalho exige feedback e configuração;
- promover startup automático depende de necessidade observada.

## ADR-017 — WAL com synchronous FULL

**Estado:** Accepted

**Aceita em:** 2026-08-13, após verificação dos pragmas, commit e rollback no spike.

### Contexto

O feedback `Saved` precisa ter significado testável diante de crash do processo, reinício do sistema e perda de energia.

### Decisão

SQLite utiliza `journal_mode=WAL` e `synchronous=FULL` como baseline. A aplicação verifica os valores na abertura e confirma Capture somente depois de commit bem-sucedido.

### Consequências

- existe custo adicional de sync por commit;
- performance deve ser medida no spike;
- mudar para `NORMAL` exige nova ADR;
- fault injection, backup consistente e detecção de corrupção são obrigatórios.

## ADR-018 — Proveniência explícita de Capture para Task

**Estado:** Accepted

**Aceita em:** 2026-08-13, como requisito de produto confirmado pelo usuário.

### Contexto

Transformar uma Capture não pode apagar nem reescrever o pensamento original.
Search também não deve apresentar Capture e Task derivada como resultados sem
relação aparente.

### Decisão

Task possui uma relação opcional e explícita `source_capture_id`. Na v0.1b ela é
única, permitindo no máximo uma Task derivada por Capture. A conversão cria a
Task e marca a Capture como processada na mesma transação.

### Consequências

- a origem continua consultável e imutável;
- Search pode agrupar origem, derivação e Project;
- falha na conversão não deixa estado parcial;
- múltiplas derivações exigirão nova decisão e migration.

## ADR-019 — Design system versionado como contrato do renderer

**Estado:** Accepted

**Aceita em:** 2026-08-13, após handoff do design system pelo usuário.

### Decisão

`Design System/design_handoff_frontend/mos-tokens.css` é a fonte única de tokens do cliente
desktop. Componentes usam primitivas próprias, SVGs próprios e fontes locais.
Bibliotecas genéricas de UI ou ícones não entram sem uma necessidade concreta.

### Consequências

- dark, light e forced colors compartilham o mesmo contrato semântico;
- valores visuais deixam de ser espalhados pelos componentes;
- Capture e Command preservam as geometrias normativas do produto;
- extensões de token devem ser explícitas e documentadas.

## ADR-020 — Origem de App é metadata, não alvo de abertura

**Estado:** Accepted

**Aceita em:** 2026-08-13, para introduzir o catálogo pessoal de Apps sem antecipar a Integration GitHub.

### Contexto

CronoCAD, M Finance e Coded Atlas possuem repositórios conhecidos, mas nem todo repositório representa um alvo operacional. CronoCAD, por exemplo, é desktop e não possui release publicada.

### Decisão

`RegisteredApp` preserva `source_url` separadamente de `launch_kind` e `launch_target`. O catálogo conhecido pelo Core pode cadastrar e enriquecer Apps de forma explícita e idempotente, sem sobrescrever um alvo já escolhido pelo usuário.

### Consequências

- o repositório de origem permanece encontrável sem ser executado como se fosse o App;
- Apps web podem possuir URL de uso e origem distintas;
- Apps desktop podem ser conhecidos antes de um executável local ser configurado;
- o corte não cria autenticação nem integração runtime com GitHub;
- associação Project → Repository continua adiada para sua fase própria.

## ADR-021 — Resource começa por Link e preserva contexto

**Estado:** Accepted

**Aceita em:** 2026-08-13, como primeiro corte vertical da Fase Memory.

### Contexto

Resource é conceitualmente amplo, mas implementar antecipadamente imagens, arquivos, artigos, bibliotecas e taxonomias criaria abstrações sem uso observado. O primeiro caso concreto é guardar uma URL junto do motivo pelo qual ela merece ser lembrada.

### Decisão

Introduzir `Resource` com o tipo concreto `link`, contendo título, URL, nota contextual, lifecycle e proveniência opcional de Capture. Library é uma projeção em lista/detalhe, não uma entidade ou container. A conversão de Capture é atômica e não apaga a origem. Links ativos são abertos somente pelo backend nativo depois de nova validação de esquema.

### Consequências

- a Fase Memory começa com um caso pequeno e útil;
- título vazio pode usar a URL para preservar baixa fricção;
- metadata remota, arquivos, tags e relações ficam adiados;
- o schema pode receber novos tipos apenas quando casos concretos forem aprovados;
- título, URL e nota participam da Search unificada;
- encontrar um Resource no Command abre seu detalhe, não executa a URL;
- Archive e Trash permanecem recuperáveis sem exclusão definitiva.

---

## ADR-022 — O design v0.7 sobrepõe o handoff onde ele supõe um back-end que não existe

**Aceita em:** 2026-08-14, na aplicação da camada visual v0.3.

### Contexto

O pacote `Design System/design_handoff_frontend/` fecha a linguagem visual e afirma, no seu README: *"Nenhuma chamada de `api.ts` muda de assinatura. Nenhuma tabela muda."*

Essa afirmação foi escrita supondo que o back-end já suportava o desenho. Ele não suportava. O Kanban desenhado tem seis colunas contra três estados persistidos; a Library filtra por tipo de Resource contra um único tipo `link`; o painel de App declara quatro capacidades que não existiam; o pane de Project exibe um repositório sem campo correspondente.

### Decisão

Criar no back-end o que o desenho exige, aceitando que assinaturas de `api.ts` e o schema mudem. O README continua valendo integralmente em tudo que é visual — ele perde apenas nesta questão específica, e por instrução explícita do proprietário do produto.

### Consequências

- migration 0007 leva o schema de 6 para 7;
- `CORE-FOUNDATION.md` foi corrigido junto, em vez de ficar contradizendo o código;
- a regra "nenhuma dependência nova, nenhum valor fora dos tokens" permanece intacta e verificada;
- futuros handoffs devem declarar o que exigem do back-end, em vez de assumir suporte.

---

## ADR-023 — Coluna de Kanban é visualização, não semântica

**Aceita em:** 2026-08-14.

### Contexto

`CORE-FOUNDATION.md` excluiu `Planned` e `Review` da v0.1 com justificativas corretas: `Planned` depende de semântica temporal inexistente, `Review` de um fluxo não validado. `ARCHITECTURE-REVIEW.md` recomendou remover `Planned`. O design v0.7 desenhou seis colunas.

### Decisão

Persistir os seis estados. Uma Task em `planned` não promete comportamento temporal e `review` não dispara fluxo algum — são rótulos de posição, e a semântica, se vier, se apoiará em dados já persistidos em vez de exigir migração.

O valor `inbox` é mantido apesar da colisão de nome com a Inbox de Captures, porque mudar o rótulo quebraria o design fechado. Capture tem `processing_state`; Task tem `work_state`. O aviso está no enum, na migration e em `CORE-FOUNDATION.md`.

### Consequências

- revoga a recomendação de `ARCHITECTURE-REVIEW.md` sobre `Planned`;
- transições passam a ser livres entre estados, porque o Kanban permite arrastar para qualquer coluna;
- a colisão de nome exige disciplina de leitura e está documentada nos três lugares onde alguém tropeçaria.

---

## ADR-024 — Hermes é superfície, não segundo agente

**Aceita em:** 2026-08-14, na investigação da integração.

### Contexto

Já existe um Hermes (`NousResearch/Hermes-Agent`) rodando em VPS, usado por WhatsApp e pelo dashboard próprio. A alternativa seria o M/OS construir a própria camada de IA.

### Decisão

O M/OS é mais uma superfície do Hermes existente. Ele mantém interface, UX, contexto local e sessão própria; o Hermes mantém modelo, reasoning, skills, tools e execução agentic. O acesso é por túnel SSH para `127.0.0.1:9119`, sem expor porta nova. O contrato factual está em `HERMES-GATEWAY-CONTRACT.md`.

A ponte vive em crate próprio, sem dependência de `mos-storage-sqlite` — "Hermes nunca escreve no SQLite" passa a ser impossibilidade de compilação em vez de regra a lembrar.

### Consequências

- nenhuma skill é duplicada: `/api/ws` delega ao mesmo dispatcher da TUI e do WhatsApp;
- sessão do M/OS é separada do WhatsApp, mas o agente é o mesmo;
- `CORE-FOUNDATION.md:33` já estabelecia que Hermes não faz parte do Core; o crate separado torna isso estrutural;
- ações do M/OS a partir do Hermes (fases 3+) passarão pela camada de aplicação, nunca direto no banco.

---

## ADR-025 — A conversa do Hermes é persistida localmente pelo M/OS

**Aceita em:** 2026-08-15, na evolução para AI Workspace.

### Contexto

Até aqui o M/OS não guardava nada da conversa: o histórico vive no `state.db` da VPS e a
thread existia apenas na memória do componente React. A auditoria em
`HERMES-PREMIUM-CHAT.md` §1.2 mostrou três consequências disso.

O `session_id`, que a Spec B mandava guardar, vivia num `Mutex` de memória de processo.
Nada o escrevia em disco, então `session.resume` — implementado e testado — nunca rodava
entre aberturas do app. Cada abertura criava sessão nova na VPS.

Sem conversa local não existe lista, busca, rename, branch nem qualquer ação sobre uma
mensagem específica, porque não existe mensagem: existia um triplo de strings por turno.

### Decisão

O M/OS passa a persistir conversa, mensagens e partes de mensagem em SQLite local, em
`mos-core` + `mos-storage-sqlite`. A VPS continua sendo dona do histórico do agente; o
M/OS guarda a sua projeção e o vínculo (`hermes_session_id`).

`mos-hermes` **não** ganha acesso ao banco. Ele continua sem `mos-storage-sqlite` e sem
`mos-core`. A tradução entre `Outcome` e `MessagePart` acontece no orquestrador do
desktop, que é o único lugar onde ponte e domínio se encontram.

Três tabelas, não nove: `Conversation`, `Message`, `MessagePart`. Anexo, artifact, citação
e execução de ferramenta entram como `kind` de parte, com payload JSON validado pelo
domínio, e só viram tabela própria quando precisarem de lifecycle ou consulta própria.

### Alternativas rejeitadas

- **Continuar sem persistência local.** Mantém a VPS como fonte única, mas impede toda a
  fase P0 e deixa `session.resume` morto.
- **Nove entidades desde o início**, como no desenho original do AI Workspace. Contraria
  ADR-012 e a prática que criou `Resource` só quando havia caso concreto (ADR-021).
  `MessagePart` sozinho já preserva a capacidade de promover cada uma depois.

### Consequências

- migration 0010 leva o schema de 9 para 10;
- backup e export passam a conter conversas — `ARCHITECTURE.md` §16 já avisa que ambos
  podem conter dado pessoal em texto claro, e o aviso agora cobre mais coisa;
- apenas partes `text` entram no FTS5; reasoning e payload de ferramenta ficam fora;
- persistência acontece por mensagem, não por delta: um `INSERT` por token sob
  `synchronous=FULL` (ADR-017) seria um fsync por token.

---

## ADR-026 — Markdown é renderizado como elementos React, nunca como HTML

**Aceita em:** 2026-08-15.

### Contexto

A resposta do Hermes era exibida como `<p>{texto}</p>`. Bloco de código, tabela e lista
chegavam com as marcações visíveis. Renderizar Markdown é requisito de P0, e introduz a
primeira superfície do M/OS que interpreta conteúdo vindo de fora da máquina.

A auditoria recomendou biblioteca, com o argumento de que escrever parser e sanitizador à
mão num caminho de conteúdo remoto é a troca errada. **A implementação mudou essa
conclusão, e o motivo está registrado aqui em vez de ficar implícito no código.**

O perigo não está em interpretar Markdown. Está em produzir HTML e depois tentar limpá-lo.
Um renderer que emite elementos React e nunca uma string de HTML não tem sanitizador
porque não tem o que sanitizar: o React escapa texto por construção, e sem
`dangerouslySetInnerHTML` não existe caminho de injeção. Uma biblioteca madura é segura
pelo mesmo motivo, não por um motivo melhor.

Removido o argumento de segurança, sobram os de custo. `react-markdown` com `remark-gfm`
traz dezenas de pacotes transitivos para uma aplicação cujas dependências de runtime hoje
são React, a API do Tauri e duas fontes. E o realce de sintaxe com tema pronto entrega uma
paleta de dez cores para um design system que restringe cor a função
(`UX-PRINCIPLES.md` §48) e mantém a cor primária abaixo de 10% da interface.

### Decisão

Renderer próprio, em `apps/desktop/src/markdown.tsx`, emitindo apenas elementos React.
Proibido `dangerouslySetInnerHTML` no projeto inteiro; proibido HTML cru vindo do modelo.

Realce de sintaxe próprio, com três classes semânticas — comentário, literal e
palavra-chave — usando tokens existentes. Não há tema de dez cores.

O subconjunto suportado é o que uma conversa técnica usa: heading, ênfase, código inline,
link, lista ordenada e não ordenada com aninhamento, citação, bloco cercado com linguagem,
tabela, régua e parágrafo.

### Consequências

- nenhuma dependência nova de runtime, e ADR-019 continua intacta;
- cerca aberta durante o streaming é tratada deliberadamente como bloco de código aberto,
  em vez de piscar a cada token — comportamento que a biblioteca não daria de graça;
- Markdown fora do subconjunto aparece como texto, nunca como marcação quebrada;
- link externo abre pelo backend nativo, como `open_resource` já faz;
- imagem remota não é carregada: a CSP de `ARCHITECTURE.md` §15.3 bloqueia, e a UI explica
  em vez de mostrar quebrado;
- se o subconjunto se mostrar insuficiente no uso real, adotar biblioteca continua
  possível — a fronteira é uma função só.

---

## ADR-027 — Nada sai para o Hermes sem chip visível e registro do que foi enviado

**Aceita em:** 2026-08-15.

### Contexto

`ARCHITECTURE.md` §15.2 modela o M/OS como dados locais no perfil do Windows. O baseline
não cobre envio deliberado de conteúdo pessoal a uma VPS. Hoje isso já acontece em pequena
escala — o que o usuário digita vai para o Hermes — mas anexar contexto do M/OS
(Projects, Tasks, Captures, Resources) muda o volume e a sensibilidade de ordem.

A menção por `@` existente agravava o problema por outro lado: ela parecia anexar contexto
e não anexava nada. Substituía o texto pelo nome do Project e mandava a string. O usuário
acreditava ter dado contexto ao Hermes sem ter dado.

### Decisão

Nenhum dado do M/OS atravessa a ponte sem um chip visível na composição, removível antes
do envio. Vale para contexto explícito (o usuário pediu) e automático (o sistema ofereceu),
que se distinguem por peso tipográfico e rótulo, nunca só por cor.

Cada mensagem persiste uma parte `context_ref` com o que **efetivamente** foi enviado —
entidade, campos e tamanho. A pergunta "o que exatamente foi para a VPS?" precisa ter
resposta depois do envio, não só antes.

Contexto automático nasce desligado e só é ligado por ação do usuário.

### Consequências

- o chip deixa de ser decoração e passa a ser o controle de um limite de confiança;
- `UX-PRINCIPLES.md` §59 ("inteligência não pode esconder informação") fica verificável;
- o registro tem custo de espaço, aceito: ele é a evidência;
- exportar ou fazer backup passa a incluir esse registro, o que é desejável.

---

## ADR-028 — A leitura do M/OS pelo Hermes começa por injeção de contexto

**Aceita em:** 2026-08-15.

### Contexto

`mos_search`, `mos_get_context` e as demais leituras precisam de um caminho pelo qual o
agente alcance o M/OS. O protocolo WebSocket do gateway não expõe registro de ferramenta
do lado do cliente — verificado em `tui_gateway/server.py`. Existem três caminhos, e eles
não são equivalentes.

O checkout do Hermes contém `mcp_serve.py` e `optional-mcps/`, então um MCP server local
consumido pelo agente é tecnicamente plausível. Mas o túnel hoje vai do M/OS para a VPS, e
esse caminho exige o inverso: a VPS alcançando a máquina do usuário.

### Decisão

Começar por **injeção de contexto**: o M/OS monta um bloco estruturado e orçado a partir
dos seus próprios serviços de leitura e o prefixa ao prompt, com os chips da ADR-027.

MCP server local fica adiado e exige ADR própria antes de qualquer código, porque expor um
servidor local à VPS é uma mudança de superfície de ataque que `ARCHITECTURE.md` §15.2 não
cobre. Fork do gateway está rejeitado.

### Consequências

- "Jarvis conhece meu M/OS" fica possível sem mudar topologia de rede nem threat model;
- o agente não consegue pedir mais dados no meio do turno: o contexto é fixo no envio, e
  essa limitação é real e conhecida;
- o Context Service precisa orçar o que envia, porque não há segunda chance;
- se a limitação incomodar no uso real, a ADR de MCP tem um caso concreto para justificar.

---

## ADR-029 — Não existem modos de conversa; o Hermes continua dono do reasoning

**Aceita em:** 2026-08-15.

### Contexto

A superfície trazia três modos, `ASK`, `ACT` e `ORGANIZE`, com dois desabilitados. O
desenho do AI Workspace propôs quatro: `Fast`, `Think`, `Research` e `Act`.

Avaliação de lastro real: `Fast` não tem controle de latência que o M/OS possa usar;
`Think` só existiria via `agent.reasoning_effort`, que ADR-024 atribui explicitamente ao
Hermes; `Act` é redundante, porque o agente já escolhe ferramenta sozinho; `Research` tem
lastro — a skill existe no checkout — mas a forma dos seus eventos não foi verificada.

### Decisão

Remover o seletor de modos e não colocar nada no lugar. O Hermes escolhe skill e esforço
sozinho, como ADR-024 estabeleceu.

`Research` poderá voltar em P2 como **tipo de execução** com progresso e fontes, não como
chave de modo — e só depois de os eventos da skill serem observados.

Expor controle de esforço exigirá emendar ADR-024 e não é feito por decisão de UI.

### Consequências

- some um controle que ensinava errado: dois de três modos não faziam nada;
- `Tab` volta a mover foco estrutural, como `DESIGN-FOUNDATIONS.md` §12 exige — ele estava
  sequestrado para trocar modo dentro de um campo de texto;
- o composer fica com uma intenção só, coerente com `UX-PRINCIPLES.md` §13;
- se o usuário quiser controle de esforço, existe caminho, e ele passa por ADR.

### Emenda de 2026-08-15 — o slot fica, o ciclo não

`M-OS Hermes - Design Direction v1` §07 chegou depois desta decisão e especifica os quatro
modos como texto mono à direita do campo, trocados por `Tab` **no campo vazio** (§18). A
frase que resolve a tensão está no próprio documento: *"o modo é a única promessa que o
sistema faz sobre o que vai acontecer com seus dados"*.

Sendo uma promessa sobre dados, ela não pode ser uma etiqueta decorativa. Hoje existe uma
promessa verdadeira e verificável, e ela é garantida pela arquitetura em vez de por
intenção: `mos-hermes` não compila com acesso ao banco, e não existe nenhuma ferramenta de
escrita no M/OS. O composer passa a exibir essa promessa — **`NÃO ESCREVE`** — no slot e na
tipografia que o design define.

O ciclo de quatro entra quando houver mais de uma promessa para fazer, o que acontece em
P4 com `ACT`. `Tab` continua livre até lá: sem segundo modo, não há o que alternar, e a
tensão entre `Tab`-troca-modo e `Tab`-move-foco fica para ser resolvida com um caso real.

Isto preserva o layout, o vocabulário e a intenção do design sem exibir três promessas que
o sistema não pode cumprir.

---

## ADR-030 — A superfície Hermes adota a direção Marginália

**Aceita em:** 2026-08-15, com o handoff de `M-OS Hermes - Design Direction v1`.

### Contexto

O documento testou duas gramáticas de thread e escolheu **A, Marginália**: um gutter de
108px à esquerda da coluna de leitura de 62ch. O princípio é uma separação de papéis, não
uma preferência de layout: *tudo que o sistema faz — buscar, ler, citar, executar — mora na
margem; tudo que ele diz mora na coluna de leitura.*

A implementação anterior misturava os dois: execução de ferramenta era renderizada como
linha dentro da prosa, empurrando a resposta para baixo a cada passo.

### Decisão

Adotar Marginália como a estrutura da superfície, com as decisões que a acompanham:

- **thread** — gutter 108px + prosa 62ch fixos; 26px entre turnos, sem separador;
- **reconhecimento tipográfico** — pergunta em 21px, resposta em 15px. Sem bolha, avatar,
  orbe ou gradiente;
- **composer Trilho** (§04, decisão A) — sem caixa, régua superior, barra de sódio de 6px
  à esquerda como único elemento que muda por estado; enviar só ganha preenchimento com
  conteúdo válido; parar substitui enviar no mesmo lugar;
- **tool activity na margem** (§08, decisão A) — passos durante, recibo de uma linha
  depois, detalhe técnico nunca despejado na thread;
- **chip de contexto** (§06) — borda sólida é manual, tracejada é automático. Sem cor
  extra, sem ícone, sem legenda, o que também satisfaz "nada só por cor";
- **ordem de sacrifício responsivo** (§19) — conversas, depois inspector, depois margem. A
  coluna de leitura nunca é sacrificada, e em ultrawide sobra canvas em vez de linha longa;
- **`Esc` interrompe de qualquer foco** dentro do Hermes (§20), nunca dependendo de
  alcançar um botão.

A paleta do documento já é a de `mos-tokens.css`. Nenhum literal de cor foi introduzido.

### Consequências

- ferramenta deixa de empurrar a prosa, que era o defeito estrutural da versão anterior;
- o inspector (plano 4) fica desenhado e **não implementado** em P0: fontes, memória e
  artifact dividem esse painel e nenhum deles existe ainda;
- `Ctrl+/` alterna a coluna de conversas, e `Ctrl+N` cria conversa;
- a lista agrupa por tempo, porque conversas não são arquivos — a maioria morre no mesmo
  dia e o que sobrevive delas são Tasks e Resources.

---

## ADR-031 — O rail carrega oito destinos, e o teto de seis vira regra de crescimento

**Aceita em:** 2026-08-15, na revisão de layout.
**Revisada pela ADR-036** em 2026-08-16: o teto passou a nove com a entrada de Tempo. A
regra de troca continua valendo a partir dali.

### Contexto

`mos-design-system.md` estabelece: *"Rail 52px, só ícones, máximo 6 destinos"*. O rail tem
oito — Home, Hermes, Inbox, Tasks, Projects, Workspaces, Library, Apps — mais Quick Capture
e Settings no rodapé.

A revisão levantou isso como violação. Ela merece decisão registrada em vez de correção
automática, porque as duas saídas têm custo real.

### Decisão

Manter os oito, e reinterpretar o número como **teto de crescimento**, não como contagem
histórica: a partir daqui, um destino novo no rail exige retirar um.

O motivo é o propósito da regra. Ela existe para o rail não virar depósito de features — e
os oito não são features, são os substantivos centrais do produto: `VISION.md` §5 organiza
o M/OS em CAPTURE, ORGANIZE, CONNECT e ACT, e cada item do rail é uma dessas superfícies.
Um rail com oito conceitos estáveis não é o problema que a regra descreve.

A alternativa — rebaixar Library e Apps para o `Ctrl+K` — já foi testada com Workspaces e
falhou. O comentário que acompanha a entrada de Workspaces em `App.tsx` registra o
resultado: o item ficou *"invisível para quem não conhece o Command"* até ser promovido ao
rail. Repetir isso com Library e Apps trocaria uma violação de contagem por uma perda de
descoberta.

Mexer em navegação também tem custo próprio: `UX-PRINCIPLES.md` §41 pede que elementos
principais não mudem de lugar, porque o usuário desenvolve memória espacial do sistema.

### Consequências

- a regra deixa de ser descumprida em silêncio e passa a ter um limite operante;
- o próximo destino candidato ao rail chega junto da pergunta "qual sai?";
- se o rail crescer para nove sem essa troca, a decisão aqui foi ignorada, e não revisada;
- Quick Capture e Settings continuam fora da contagem: eles não são destinos de conteúdo, e
  o rodapé do rail é uma zona própria.

---

## ADR-032 — Os Apps próprios entram no monorepo, com profundidade decidida por app

**Aceita em:** 2026-08-15, por decisão do proprietário do produto.

### Contexto

`ARCHITECTURE.md` §20 lista **"monorepo com todos os Apps independentes"** entre os itens
explicitamente não adotados, e `PRODUCT.md` §12 estabelece que um App pode continuar
completamente independente do código do M/OS. Esta decisão reverte esse ponto, a pedido do
proprietário, com o objetivo de ter **um lugar só para editar**.

Levantamento dos três primeiros Apps:

| App | Stack | Dados | Arquivos TS |
|---|---|---|---:|
| CronoCAD | Tauri 2 + React + Vite | SQLite local (`plugin-sql`) | 98 |
| M-Finance | Next.js + Drizzle | Postgres remoto (`DATABASE_URL`) | 165 |
| Coded Atlas | Next.js 15 + React 19 | filesystem do servidor + rotas `/api` | 91 |

Os três não são o mesmo problema, e tratá-los como se fossem é o erro que esta ADR evita.

### Decisão

**O código dos três passa a viver em `apps/`, importado com histórico** (`git subtree`), e
os repositórios de origem são arquivados. Existe uma fonte de verdade.

**A profundidade da integração é decidida por App, pela natureza dele:**

- **CronoCAD → superfície nativa do M/OS.** Ele já é Tauri 2 com SQLite local: mesma shell,
  mesma linguagem, mesmo modelo de persistência. Absorvê-lo não custa nada que ele já não
  pague, e ele é a base natural da Fase 4 (Time) do `ROADMAP.md`.
- **M-Finance → continua web.** O valor dele inclui abrir pelo celular, e `ROADMAP.md`
  §18.2 já dizia, antes desta conversa: *"Não transformar o M/OS em duplicação do
  M-Finance"* — o que ele pede é resumo, acesso rápido e consulta pelo Hermes. Absorvê-lo
  exigiria ou rede no caminho crítico, contra o driver 4 da `ARCHITECTURE.md`, ou migrar
  Postgres para SQLite e perder o acesso móvel.
- **Coded Atlas → continua web.** Ele depende de runtime Node para gerar assets e mexer no
  filesystem por rotas `/api`. Dentro do renderer do Tauri isso não roda; exigiria um
  sidecar Node ou reescrever o pipeline em Rust, e nenhum dos dois se paga agora.

Os três compartilham o design system do M/OS. Isso é o que cumpre "combinar com o M/OS"
sem exigir que os três virem a mesma coisa.

### Consequências

- `ARCHITECTURE.md` §20 perde o item de monorepo; os demais continuam valendo;
- `PRODUCT.md` §13 (Atalho → Contexto → Integração → Automação) continua descrevendo bem a
  relação: CronoCAD sobe para Integração, os outros dois ficam em Contexto;
- o histórico de cada App é preservado, então `git log` continua respondendo por que uma
  linha existe;
- trabalho não publicado nos checkouts de origem precisa ser resgatado **antes** do
  arquivamento — a importação do Coded Atlas já trouxe um delta que vivia só na árvore de
  trabalho;
- o M/OS passa a ter apps web no repositório sem ter, ainda, workspace npm; isso entra
  quando houver dependência compartilhada de verdade, e não antes.

---

## ADR-033 — A unificação troca o valor por trás do nome, não o componente

**Aceita em:** 2026-08-15, na unificação do design system (ADR-032, fase 2).

### Contexto

Os três Apps chegaram ao monorepo com vocabulários e paletas próprios:

| App | Tailwind | Vocabulário | Identidade |
|---|---|---|---|
| CronoCAD | 3, via `tailwind.config.js` | `--color-surface`, `--color-accent` | fundo esverdeado, sinal vermelho, Panchang + Satoshi, raio 0 |
| M-Finance | 4, via `@theme` | `--color-background-card`, `--color-accent` | verde profundo, acento vermelho, gradientes e grade |
| Coded Atlas | 4, via `@theme` | `--color-base`, `--color-line` | neutros frios em hue 265, acento cobre |

Reescrever componente a componente seriam ~350 arquivos, e cada um deles uma chance de
errar sem poder conferir na tela.

### Decisão

Um pacote, `packages/design-system`, com três arquivos: `tokens.css` (a fundação do M/OS),
`aliases.css` (a tradução dos vocabulários) e `all.css` (os dois, na ordem certa).

**A unificação acontece em `aliases.css`, e ela não repinta componente nenhum.** Ela
redefine o que cada nome VALE. O código de cada App continua dizendo `bg-surface`,
`bg-background-card`, `text-accent` — e passa a desenhar as superfícies do M/OS. É a
diferença entre repintar a sala e trocar a lâmpada.

Nos dois Apps em Tailwind 4 o tema usa `@theme inline`, e não `@theme`: com `inline`, as
utilidades apontam para o valor da variável em vez de copiá-lo, então o tema segue os
tokens em vez de congelar uma cópia no build.

### O limite: aparência é unificada, medida não

O CronoCAD tem escala de espaçamento própria, linear de 4 em 4. A do M/OS é
4·8·12·20·32·52·84, pensada para a densidade das superfícies dele. Deixar o import vencer
mudaria `--space-4` de 16px para 20px e `--space-6` de 24px para 52px — todo o layout do
App, de uma vez, sem ninguém ter pedido.

Ele retoma a própria escala depois do import. **Cor, tipografia e raio vêm do M/OS; medida
continua sendo de cada App.** Unificar aparência não é unificar layout, e tratar as duas
como a mesma coisa é como esta unificação quebraria.

### A exceção autorizada ao "nenhuma cor hardcoded"

Recharts e widgets de canvas/SVG não leem custom property. `apps/m-finance/lib/ui/colors.ts`
e o canvas de `triangle-field.tsx` carregam hex literal — e é deliberado que a exceção
viva em dois lugares nomeados, em vez de espalhada.

A paleta categórica de gráfico virou **rampa de claridade**, não sequência de matizes: o
sistema tem um acento só, e inventar cinco matizes devolveria pela porta do gráfico a
paleta que o design system recusa na interface. Diferenciar por luminosidade também
sobrevive ao daltonismo, o que matiz não faz.

### Consequências

- os tokens passam a ser lidos de `packages/design-system`; a pasta `Design System/`
  continua sendo o arquivo da entrega do designer, e é dela que um handoff novo é
  transposto para o pacote — isto estende ADR-019 sem contrariá-la;
- `apps/cronocad/src/styles/tokens.css` fica no repositório de propósito: ele registra a
  identidade que o App tinha sozinho, e explica por que as classes se chamam o que se
  chamam;
- decoração saiu junto — gradiente de fundo, brilho de acento e grade ambiente no
  M-Finance, contra o que `UX-PRINCIPLES` §14 e a lista do design system já pediam;
- `apps/m-finance/tailwind.config.ts` foi removido: era Tailwind 3 vestigial, que nada
  referenciava, carregando a paleta antiga — quem editasse ali não veria efeito nenhum;
- o que NÃO foi feito: revisão tela a tela. Componente que fixou cor ou espaçamento fora
  do vocabulário continua fora, e só aparece olhando cada superfície.

---

## ADR-034 — A família de widgets entra pela geometria, e só onde há dado

**Aceita em:** 2026-08-15, com o handoff `M-OS Widgets - Visuais e Animados v0.1`.

### Contexto

A Home era uma grade de listas: tudo texto alinhado à esquerda. O desenho propõe doze
widgets que trocam parte disso por geometria — anéis, arcos e densidade — para que a
leitura aconteça em meio segundo, antes de qualquer leitura de palavra.

Duas famílias sustentam os doze: **o anel**, que mostra a proporção de uma coisa só, e a
**densidade**, que mostra tempo como área ocupada e nunca como tabela de horas.

### Decisão

As duas famílias entram como linguagem compartilhada, em
`packages/design-system/widgets.css`, e não como componente de uma tela. Elas ficam
disponíveis para as quatro aplicações do monorepo pelo mesmo import dos tokens.

O orçamento de movimento do desenho vira regra: um loop por tela, movimento que carrega
dado, cascata de 40ms com teto de oito, e `reduced-motion` que nasce no valor final em vez
de degradar.

Três regras de desenho que o código impõe em vez de confiar em quem usa: ponta reta
sempre, porque cap arredondado mente sobre o valor em anéis pequenos; início às 12h no
sentido horário; e **zero não desenha nada**, porque um traço de comprimento zero com
sub-pixel vira um ponto solto de sódio.

E uma regra de cor que atravessa as duas famílias: **o sódio é reservado para carga**.
Agora e hoje são sempre um traço branco de 2px — no arco, nas colunas e na grade do mês.

### O corte: cinco dos doze não foram construídos

| Widget | Dado que ele exige | Existe? |
|---|---|---|
| W03 Week Rings | Task concluída por dia | sim |
| W07 Densidade | atividade por dia | sim |
| Progresso | Task concluída sobre total | sim |
| Inbox | proporção envelhecendo | sim |
| W01 Focus · W02 Capacity | tempo rastreado por projeto | **não** — CronoCAD, ADR-032 fase 3 |
| W04 Next Up · W06 Day Arc | calendário e lembretes | **não** — Fase 4 do ROADMAP |
| W05 Habits | domínio de hábitos | **não** — não existe |

Os sete que ficaram de fora não são um atraso de implementação: são widgets que só podem
existir depois que o dado existir. Um anel bonito preenchido com número inventado é pior
que a ausência — ele ensina a confiar numa medida que o sistema não tem.

### Duas adaptações declaradas

A densidade do mês mede **atividade registrada** — Task criada, Task concluída, Capture — e
não "eventos", que viriam de um calendário inexistente. O rótulo diz "registros" em vez de
fingir agenda.

E ela não aplica o passado a 45% que o desenho pede. Aquilo faz sentido no arco do dia,
onde o passado é contexto e o próximo evento é o assunto; na grade do mês o passado É o
conteúdo, e apagá-lo apagaria o widget.

### Consequências

- a Home ganha três widgets novos e o da Inbox troca o número cru de 48px por anel;
- `--space-*` e a escala de cor continuam sendo os únicos valores usados: nenhuma cor nova
  entrou, e os degraus de profundidade são o mesmo sódio rebaixado a 55% e 30%;
- os widgets restantes chegam junto do dado — Focus e Capacity na absorção do CronoCAD,
  Next Up e Day Arc na Fase 4;
- a linguagem está disponível para CronoCAD, M-Finance e Coded Atlas, que a importam junto
  dos tokens; aplicá-la lá é trabalho de cada superfície, não deste corte.

---

## ADR-035 — Desfazer arquiva, nunca apaga

**Data:** 2026-08-15
**Status:** aceito
**Contexto:** fase 2 de `SPEC-ACOES-ENTRE-APPS.md` — o Hermes executa, e o usuário precisa
de caminho de volta.

### A pergunta

Qual é o inverso de "criar"? A resposta intuitiva é apagar: se a Task não devia existir,
que ela deixe de existir. Foi por aí que comecei.

### Por que apagar está errado aqui

Duas coisas, encontradas no próprio código, apontam para o mesmo lado.

`ports.rs` declara que a exclusão definitiva **recusa o que ainda está ativo**, e o
comentário diz o motivo: a regra existe "para que nenhum apagamento aconteça por engano no
meio do uso normal". Uma Task criada há três segundos está ativa. Para apagá-la eu teria
que arquivá-la primeiro e então apagar — ou seja, contornar deliberadamente uma guarda
escrita para impedir exatamente aquilo.

E todo Undo que o M/OS já oferece é **restauração de estado**: `moveToInbox`, `restore`,
`setResourceArchived(false)`. Nenhum remove. Um desfazer que destrói seria o único caminho
sem volta do aplicativo — e estaria sendo oferecido no momento exato em que o usuário
acabou de dizer que errou, que é o pior momento possível para uma ação irreversível.

### A decisão

O inverso de criar é **arquivar**. O inverso de mover é voltar ao estado anterior, que por
isso é lido antes da mudança e não depois.

### Onde o Undo vive

Na janela de cinco segundos do recibo, como no resto do app — não no cartão da conversa.
Um botão "desfazer" permanente numa ação de semana passada seria surpresa, não segurança.

O cartão guarda o desfecho de forma permanente, e é por isso que o recibo só aparece
quando há caminho de volta: sem Undo ele não acrescentaria nada que a conversa já não diga.

### Consequências

- desfazer deixa registro arquivado em vez de sumir com o dado, e isso é a intenção;
- a exclusão definitiva continua exigindo a decisão explícita do usuário, pelo caminho que
  já existia em Settings;
- ações futuras sem inverso honesto declaram `undo: None` — e a ausência fica visível no
  código em vez de virar esquecimento;
- a forma do `UndoStep` atravessa a ponte escrita à mão dos dois lados, e um teste prende
  os nomes: renomear uma variante quebraria o desfazer justamente dentro dos cinco
  segundos em que ele importa.

---

## ADR-036 — O rail vai a nove, e Tempo ganha endereço

**Data:** 2026-08-16
**Status:** aceito, por decisão do proprietário do produto
**Revisa:** ADR-031

### Contexto

A ADR-031 manteve oito destinos e transformou o número em **teto de crescimento**: *"a partir
daqui, um destino novo no rail exige retirar um"*. Ela também antecipou este momento: *"se o
rail crescer para nove sem essa troca, a decisão aqui foi ignorada, e não revisada"*.

Este é o nono, e esta ADR existe para que ele seja revisão e não omissão.

O pedido: o CronoCAD deixa de ser aplicativo à parte e vira página do M/OS, alcançável por
ícone no rail, com o tempo trabalhado atravessando para a Home.

### Decisão

**Nove destinos.** Tempo entra sem ninguém sair, e o teto passa a ser nove — com a mesma
regra de troca valendo daqui em diante.

O argumento que sustenta os oito continua sustentando o nono, e é o único que importa: a
regra existe para o rail não virar depósito de *features*, e os itens do rail não são
features, são os substantivos centrais do produto.

Tempo é um deles, e por uma razão que nenhum dos outros oito tem: **o usuário fatura por
hora.** Tempo rastreado não é uma conveniência do sistema — é o registro do qual sai a
renda dele. Uma informação nesse nível não vive atrás de um `Ctrl+K`.

### Por que não as alternativas

**Trocar um dos oito** foi descartado pela própria evidência da ADR-031: a troca já foi
testada com Workspaces e falhou. O item ficou *"invisível para quem não conhece o Command"*
até ser promovido de volta. Repetir o experimento sabendo o resultado não é rigor, é
teimosia.

**Tempo dentro de Projects** resolveria metade. A hora sempre pertence a um Project, então
a aba por projeto faz sentido — e vai existir de qualquer forma. Mas o histórico global, os
totais do mês e o que o CronoCAD chama de relatório não são de projeto nenhum: são a visão
por cima. Sem endereço próprio, essa visão fica sem casa.

### Consequências

- o teto vira nove, e a regra de troca continua: o décimo destino exige retirar um;
- a justificativa passa a ser explícita e verificável — um destino só entra se for
  substantivo central, e "substantivo central" agora tem um caso de referência: algo de que
  depende a renda ou a memória do usuário, não algo que ele usa com frequência;
- `mos-design-system.md` continua dizendo "máximo 6". A distância entre o documento e o
  produto é de três, e ela está registrada aqui e na ADR-031 em vez de descoberta depois;
- o risco assumido é real: cada revisão do teto torna a próxima mais fácil, e é assim que
  um rail vira depósito. A defesa não é o número, é a exigência de ADR para mexer nele.

## ADR-037 — O M/OS observa nomes de programa, e nada além disso

**Data:** 2026-08-16
**Status:** aceito
**Complementa:** ADR-032, ADR-036

### Contexto

A absorção do CronoCAD (ADR-032) terminou com a peça que fazia o aplicativo original ganhar
o adjetivo que ele usava para se descrever: ele **percebe** que você começou a trabalhar. Um
laço compara os processos em execução com uma lista de programas cadastrados, e mede há
quanto tempo ninguém toca no teclado ou no mouse.

Isso é, por construção, um recurso de monitoramento — a mesma família de código que sustenta
software de vigilância de funcionário. A diferença entre os dois não está na intenção de quem
escreve, e sim em onde a fronteira é desenhada e em quão difícil é atravessá-la depois.

### Decisão

**Só duas coisas são lidas:** o nome do executável dos processos em execução, e o número de
segundos desde o último evento de teclado ou mouse.

Explicitamente fora, e não por esquecimento:

- título de janela — diria em qual arquivo, em qual cliente, em qual assunto;
- linha de comando do processo — diria caminhos completos;
- conteúdo de arquivo, de qualquer tipo;
- captura de tela, de qualquer frequência;
- telemetria: nada do que é observado sai da máquina.

E a regra que já vinha do CronoCAD, agora com laço de verdade por trás dela: **observação não
vira hora sozinha.** O evento é gravado, a Linha do Tempo mostra o vão, e quem decide se
aquilo foi trabalho é a pessoa. Nenhum caminho do código cria uma sessão a partir de um
evento observado sem passar por um clique.

### Por que a fronteira é a API, e não a política

A escolha das funções chamadas é o que sustenta a promessa. `GetLastInputInfo` devolve um
número — não há como extrair dele o que foi digitado. A enumeração de processos do `sysinfo`
com `features = ["system"]` traz o nome; ler o título da janela exigiria `GetWindowText`, que
é uma chamada nova, visível em revisão, e que teria de ser escrita de propósito.

Uma política escrita num documento é respeitada por quem lê o documento. Uma fronteira
mantida pela API é respeitada por quem não leu — e é a segunda que resiste ao tempo.

O texto na tela de Configurações diz exatamente isto, em português, onde o usuário pode
conferir. Uma promessa de privacidade que só existe no código é uma promessa que o usuário
não pode cobrar.

### Consequências

- o monitoramento pode ser desligado inteiro, e o lembrete separadamente. Observação que não
  pode ser desligada é vigilância, mesmo quando o observado é o dono da máquina;
- qualquer PR que introduza leitura de título de janela, de linha de comando ou de tela
  contradiz esta ADR e exige revisá-la — não basta achar que "é só um detalhe a mais";
- fora do Windows o laço não mede inatividade e devolve ausência, em vez de fingir zero.
  Zero significaria "acabou de mexer", e a Linha do Tempo passaria a nunca ver inatividade —
  um número inventado é pior que um número faltando;
- o custo assumido: um laço acordando a cada poucos segundos. Ele roda em tarefa própria,
  nunca no fio da interface, e o intervalo é configurável com piso de um segundo.
