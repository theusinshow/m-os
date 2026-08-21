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
| ADR-035 | Desfazer arquiva, nunca apaga | Accepted |
| ADR-036 | O rail vai a nove, e Tempo ganha endereço | Accepted |
| ADR-037 | O M/OS observa nomes de programa, e nada além disso | Accepted |
| ADR-038 | O rail vai a dez, e Apps sai para o Calendário entrar | Accepted |
| ADR-039 | O rail vai a onze, e Finance entra sem tirar ninguém | Accepted |
| ADR-040 | A ponta arredondada entra compensada, e a moldura entra só na Home | Accepted |
| ADR-041 | Argos, a face do estado, e por que ela não é AI slop | Accepted |
| ADR-042 | O lembrete assenta, e o sódio passa a nomear em vez de contornar | Accepted |
| ADR-043 | O M/OS pode iniciar com o Windows, e o registro é quem manda | Accepted |
| ADR-044 | O rail vai a doze, e Reuniões entra sem tirar ninguém | Accepted |
| ADR-045 | O rail volta a oito, e o recém-chegado nasce no leque | Accepted |
| ADR-046 | Todo drop vira Capture primeiro, e a entidade vem depois | Accepted |
| ADR-047 | A detecção de reunião observa o microfone, e nunca o conteúdo | Accepted |
| ADR-050 | A página de Tempo passa a se chamar CronoCAD, e leva a marca junto | Accepted |
| ADR-048 | Argos ganha corpo, e o orçamento de movimento abre uma exceção nomeada | Accepted |
| ADR-051 | O Hermes opera o M/OS, e a busca acontece antes do envio | Accepted |

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
**Revisada pela ADR-040** em 2026-08-18: a condicao "promover startup automatico depende de necessidade observada" foi cumprida, e o startup entra como opcao desligada por padrao. O resto desta ADR continua valendo.

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

## ADR-038 — O rail vai a dez, e Apps sai para o Calendário entrar

**Data:** 2026-08-17
**Status:** aceito, por decisão do proprietário do produto
**Revisa:** ADR-036, ADR-031

### Contexto

A ADR-036 levou o rail a nove e manteve a regra: *"o décimo destino exige retirar
um"*. Este é o décimo, e esta ADR existe para que ele seja troca e não omissão.

O pedido: um calendário interativo. A primeira fase mostra o que aconteceu em
cada dia — sessões, Tasks, Captures e programas abertos —, e a segunda traz
prazos, que é o que responde a pergunta que o `ROADMAP` fixou como critério de
sucesso da Fase 4: *"o que eu preciso lembrar hoje?"*.

### Decisão

**Apps sai do rail. Calendário entra. O teto passa a ser dez.**

Dois motivos que se somam:

1. **Evidência.** O banco do usuário tem **zero apps cadastrados**. Sair não tira
   nada dele hoje.
2. **O critério da própria ADR-036**, que definiu item de rail como *"algo de que
   depende a renda ou a memória do usuário, não algo que ele usa com
   frequência"*. Um lançador de aplicativos não é renda nem memória — é
   conveniência. Library **é** memória, Inbox é a entrada dela, Workspaces é a
   lente sobre tudo, e Tempo é de onde sai a renda.

### Por que não Workspaces

Foi considerado e descartado **por evidência registrada**. A ADR-031 conta que
Workspaces já foi rebaixado uma vez e o resultado foi ficar *"invisível para quem
não conhece o Command, até ser promovido de volta"*. Repetir um experimento
sabendo o resultado não é rigor, é teimosia.

### A porta que teve de ser construída junto

Tirar Apps do rail quase repetiu exatamente aquela falha, por um motivo que só
apareceu na implementação: o Command **não lista destinos** — ele busca
entidades. Com zero apps cadastrados, a busca não acharia nada, e a página de
Apps ficaria inalcançável **justamente para criar o primeiro**.

Por isso o widget APPS da Home ganhou um botão `Gerenciar`, na mesma mudança e
não depois. A diferença para o caso de Workspaces é essa: lá o item sumiu e a
porta alternativa era hipotética; aqui a porta foi construída antes de a antiga
fechar.

### Consequências

- o teto vira dez, e a regra de troca continua: o décimo primeiro exige retirar
  um;
- Apps segue alcançável por três caminhos — o botão do widget na Home, a busca do
  Command quando houver app cadastrado, e a tela de Workspaces;
- a evidência usada ("zero apps") mede **conteúdo, não frequência**. O M/OS não
  registra clique de navegação, então este foi o melhor sinal disponível e não o
  ideal. A decisão foi tomada com essa ressalva dita ao proprietário;
- é reversível pelo mesmo caminho que trouxe Workspaces de volta: se Apps fizer
  falta, uma ADR nova o repõe;
- o risco assumido continua sendo o da ADR-036, agora um degrau maior: cada
  revisão do teto torna a próxima mais fácil. A defesa não é o número, é a
  exigência de ADR para mexer nele — e esta é a segunda em dois dias, o que é um
  sinal a observar.

## ADR-039 — O rail vai a onze, e Finance entra sem tirar ninguém

**Data:** 2026-08-17
**Status:** aceito, por decisão do proprietário do produto
**Revisa:** ADR-038, ADR-036, ADR-031

### Contexto

A ADR-038 levou o rail a dez e manteve a regra herdada da ADR-036: o próximo
destino exige retirar um. M-Finance é hoje um app web separado
(`apps/m-finance`, Next.js/Postgres/Supabase, deploy em produção), alcançável
pelo M/OS apenas através do App Registry, que abre o navegador padrão do
Windows e tira o usuário da janela do M/OS. A Feature A (ver
`docs/superpowers/specs/2026-08-17-m-finance-embed-design.md`) embute essa
mesma URL num iframe dentro de uma página nativa do M/OS, e o pedido natural é
um destino de rail para ela — Finance não é uma tela secundária, é onde o
usuário vê e mexe no próprio dinheiro.

### Decisão

**Finance entra no rail. Nada sai. O teto passa a ser onze.**

O critério já fixado pela ADR-036 é "algo de que depende a renda ou a memória
do usuário, não algo que ele usa com frequência". Finance passa nesse critério
com folga maior que Apps passava — Apps foi removido justamente por ser só
conveniência (ADR-038), e contas, vencimentos e faturas são renda de forma
direta, não uma leitura extensiva do critério.

Diferente da troca Apps→Calendário da ADR-038, aqui nada precisa sair: o rail
ainda comporta um décimo primeiro item sem ficar ilegível nas larguras
suportadas (840×600 em diante, conforme os lotes de UI/UX já validados), e não
há um destino de menor evidência para substituir sem repetir o experimento já
descartado com Workspaces (ADR-031).

Finance entra no grupo `TRABALHO`, depois de Calendário, antes do grupo
`MEMÓRIA` (Library).

### Consequências

- o teto vira onze, e a regra de troca continua valendo para o próximo pedido:
  o décimo segundo exige retirar um, ou uma ADR nova que justifique não
  retirar, como esta fez;
- o App Registry continua tendo a entrada `m-finance` como está — o rail é um
  caminho adicional, não uma substituição;
- esta ADR não reabre nem contradiz a ADR-032 (M-Finance continua Next.js,
  Postgres e Vercel, rodando exatamente como hoje; só o lugar onde a mesma URL
  é exibida muda).

## ADR-040 — A ponta arredondada entra compensada, e a moldura entra só na Home

**Data:** 2026-08-17
**Status:** aceito, por decisão do proprietário do produto
**Revisa:** ADR-034

### Contexto

O proprietário trouxe `https://amicro.vercel.app/mono-charts` como referência
para os widgets: 30 visualizadores monocromáticos construídos sobre geometria
arredondada, cada um num card com superfície aninhada e rodapé de metas.

A ADR-034 fixou o contrário em dois pontos. Primeiro, "ponta reta, sempre",
com a justificativa de que "cap arredondado mente sobre o valor em anéis
pequenos". Segundo, a Home nunca teve moldura de card — o `Panel` é rótulo e
ar, e a nota em `Surface.tsx` registra que "card é a resposta preguiçosa".

Os dois pontos foram reafirmados pelo proprietário depois de a colisão ser
apontada.

### Decisão

**1. A ponta passa a ser arredondada, com compensação aritmética.**

A regra antiga estava certa sobre o problema e o resolvia proibindo. A nova
resolve compensando: desenha-se `L' = max(ε, L − espessura)`, de modo que a
extensão *pintada* — que o cap estende em meia espessura por ponta — volte a
ser exatamente `L`.

O erro que a regra antiga evitava, medido nos tamanhos da própria família:
2,3 pontos percentuais no anel de 88px, 3,4 no de 44px e 6,9 no de 14px. É o
último que justifica a proibição ter sido escrita, e é ele que a compensação
zera.

O limite fica declarado em vez de escondido: abaixo de uma espessura de traço,
`L'` cai no piso e o cap pinta um disco. Ali o anel **para de medir e passa a
afirmar presença** — "existe algo, menor que o menor traço que este anel sabe
desenhar". Zero continua não desenhando nada.

E uma distinção que a ADR-034 não precisava fazer, porque não havia retângulos
na família: **`rx` não mente, `linecap` mente**. O canto arredondado de um
`rect` arredonda para dentro da geometria e a barra mantém a altura exata do
valor; a ponta arredondada de um traço estende para fora. Só a segunda é
compensada, e é por isso que as formas retangulares novas não precisam de
correção nenhuma.

**2. A moldura de card entra, e só na Home.**

Os 15 widgets da Home ganham moldura, superfície aninhada para a forma e
rodapé de metas. A reversão da posição anti-cardização vale **apenas nesse
escopo**: o `Panel` sem moldura continua sendo a resposta em Settings, no
Inspector de Workspaces e no Tempo, e a nota do `Surface.tsx` segue valendo
para o resto do sistema. A regra é escopada a `.home-grid .widget` justamente
para não poder vazar.

**3. O raio ganha charter novo, sem mexer no padrão.**

`--radius-widget: 12px` para a moldura externa e `--radius-lg: 8px` — que era
reservado a "somente app icon e overlay grande" — liberado também para a
superfície aninhada de widget. `--radius: 3px` continua valendo para botão,
campo, linha e todo o resto. Subir o raio global foi considerado e recusado:
vazaria a maciez para o sistema inteiro sem ninguém ter pedido.

### O que foi recusado

**A paleta monocromática da referência.** O sódio continua reservado para
carga e agora/hoje continuam traço branco de 2px. Metade do charme da
referência vem do cinza puro, e a recusa precisa estar escrita para que quem
reabrir o assunto encontre uma decisão em vez de supor esquecimento.

**As formas sem domínio** — candlestick, Sankey, pirâmide, scatter, donut de
quatro fatias. A razão é a da própria ADR-034: "um anel bonito preenchido com
número inventado é pior que a ausência".

### Consequências

- a família de widgets ganha uma terceira classe ao lado do anel e da
  densidade: as formas de plot (`Bars`, `Stack`, `Bullet`, `Spark`), todas
  sobre dado que já estava na tela;
- o `Bullet` resolve uma limitação que estava escrita no código do
  `BudgetRing` — o anel parava em cheio e o estouro da meta só existia no
  texto;
- a compensação vira responsabilidade de um módulo puro e testado, e não de
  cada chamador;
- o risco assumido: uma linguagem visual macia é mais fácil de esticar para
  onde não foi decidida. A defesa é o escopo `.home-grid`, que faz o
  vazamento exigir uma edição deliberada em vez de acontecer por herança.

## ADR-041 — Argos, a face do estado, e por que ela não é AI slop

**Data:** 2026-08-18
**Status:** aceito, por decisão do proprietário do produto
**Revisa:** nada. Convive com `UX-PRINCIPLES.md` §16 e `HERMES-PREMIUM-CHAT.md` §7.6
pela distinção fixada abaixo.
**Revisada por:** ADR-048, nas condições de tamanho e de laço. A condição 2 —
cada pose é um fato — continua valendo integralmente.

### Contexto

O proprietário trouxe `https://bloub.vercel.app/` como referência: um gerador de
avatar SVG animado, com estados que espelham situações de sistema — pensando,
alerta, notificação, espera.

Dois documentos encostam nisso. O `UX-PRINCIPLES.md` §16 manda evitar
"orbes decorativos" e "interfaces que parecem demos de IA". O
`HERMES-PREMIUM-CHAT.md` §7.6 é literal: "Proibido: bolha de chat, avatar, orb,
sparkle, gradiente 'AI', glow permanente, partícula, spinner grande".

### Decisão

**Entra uma criatura no shell, chamada Argos, sob três condições que a tornam o
oposto de enfeite.**

**1. Ela não é avatar do assistente, é a face do M/OS.** Argos tem pose para
boot, para operação em curso e para cronômetro correndo, tanto quanto tem para o
Hermes gerando. Um avatar de IA não teria pose para "o banco está abrindo". É
essa amplitude que o separa do que o §7.6 proíbe — e ele nunca aparece dentro da
superfície do Hermes, onde a proibição é literal.

**2. Cada pose é um fato, e nenhuma existe sem sinal.** É a mesma doutrina que a
ADR-034 aplicou ao movimento e que o próprio §7.6 enuncia: o efeito É o estado,
nunca enfeite. Onde não há sinal, não há pose — por isso Argos não dorme (ver
abaixo) e não tem ponto de notificação.

**3. Ela não tem loop nem piscada ociosa.** O orçamento de movimento da ADR-034
dá um loop por tela, e o sistema já gastou o dele na barra inclinada do
`Symbol.tsx`, que é declarada "o único spinner do sistema". Uma piscada periódica
não carregaria dado nenhum — seria exatamente o enfeite que o §16 proíbe. O
movimento de Argos é a transição entre poses, que só acontece porque um fato
mudou.

### O desenho do bloub foi recusado

O autor da referência escreve que "The MIT licence covers the code in this
repository, not the design it imitates", e junto: "Not affiliated with, endorsed
by or connected to x.ai". O blob dele é uma reimplementação do mascote da x.ai.
Vestir o M/OS com a cara de outro produto seria cair no "interfaces que parecem
demos de IA" do §16 por um caminho mais curto ainda.

Da referência foram adotados o mecanismo e a doutrina: uma silhueta preenchida
com a expressão inteira nos olhos, estados como dado, e um motor puro sem relógio
— o mesmo padrão de `plotGeometry.ts`.

A silhueta é um quadrado de cantos macios, herdando a família geométrica do
símbolo do rail — sem herdar a marca: nunca campo sódio, nunca a barra dentro.

### Argos só escuta

Ele assina o barramento `hermes-event` e **nunca responde**, em particular nunca
chama `hermes.approve`. A ADR-024 fixou que Hermes é superfície, não segundo
agente; e o próprio `hermes.ts` registra o problema que a endereçagem de evento
veio corrigir — "sem isto, duas superfícies assinando o mesmo barramento dividiam
a mesma resposta entre si". Um bicho que respondesse seria um terceiro
respondente.

### O nome

Argos Panoptes é o vigia de cem olhos: uma criatura cuja única característica é o
olhar, que é literalmente o caso aqui — toda a expressão são duas cápsulas. A
repartição com Hermes é exata: **Hermes fala, Argos olha**, e o nome carrega a
restrição de segurança acima.

E o mito já contém o escopo: Argos é o que não dorme. O nosso também não — não
por esquecimento, mas porque o sinal de inatividade real mora no Rust, no monitor
da ADR-037, e fingir sono com um temporizador de renderer seria inventar o dado.

### Consequências

- a topbar ganha um segundo espelho de estado ao lado do que já existe;
- o dia em que a ADR-037 alimentar o sono, Argos dorme pela primeira vez, e isso
  é uma nota de release;
- o risco assumido: uma criatura é mais fácil de esticar do que uma barra. A
  defesa é esta ADR — qualquer pose nova exige um sinal que já exista, e qualquer
  pose sem sinal é a mudança silenciosa que `UI-UX-REFINEMENT.md` §15 proíbe.

## ADR-042 — O lembrete assenta, e o sódio passa a nomear em vez de contornar

**Data:** 2026-08-18
**Status:** aceito, por decisão do proprietário do produto
**Revisa:** nada. Aplica ADR-034 e o `UX-PRINCIPLES` §16 a uma janela que estava
fora dos dois.

### Contexto

A moldura do lembrete parecia "falhada". Não era: o monitor está a 100%, sem
escala fracionária, então não havia fringing de subpixel a consertar.

O defeito era outro. O lembrete é a **única superfície flutuante do M/OS sem
elevação** — `--shadow-overlay` é usado pelo Command, pelo drawer, pelo diálogo
e pelo menu, e não por ele. Um retângulo com um fio de 1px e nada mais não
parece pousado sobre o CAD; parece recortado.

E a moldura inteira em `--signal-ink` contradizia o que a própria janela declara
em código: *"ela não rouba o foco"*. Contornar tudo em sódio é o gesto mais alto
que o design system tem.

### Decisão

**1. A janela passa a ser transparente.** É o que permite sombra e raio
existirem: uma janela `decorations: false` opaca não projeta nada para fora do
próprio retângulo, e um raio revelaria o fundo dela. `"transparent": true` em
`tauri.conf.json`, e a janela cresce de 400×232 para 420×252 para a sombra caber
dentro dela.

**2. A sombra é mais curta que `--shadow-overlay`, de propósito.** Nos outros
overlays a sombra mora dentro do app; aqui ela mora dentro da JANELA, e cada
pixel dela é um pixel transparente que engole clique do que estiver embaixo — o
Tauri não faz click-through por região. Com `0 20px 48px` o anel morto seria de
24px nas laterais e 44px embaixo, sobre o desenho de quem está trabalhando. Com
`0 6px 16px` fica em 10 e 16, e a janela ainda assenta.

Este é o caso em que seguir o token cegamente custaria mais do que ganha, e é
por isso que a exceção está escrita aqui em vez de resolvida no CSS em silêncio.

**3. O sódio deixa de contornar e passa a nomear.** A moldura vira
`--border-strong` com `--radius`, que é a receita que `.command-surface`, o
drawer e o menu já usam. O sódio migra para o rótulo `M/OS · TEMPO`: numa tela
tomada pelo CAD, ele diz de quem é aquela janela.

Isso é a diferença que o §16 usa para separar sinal de slop — o sódio passa a
carregar **informação**, e não contorno decorativo. E `--signal-ink` existe
exatamente para isto: o comentário dele em `tokens.css` registra que "âmbar puro
é ilegível como texto no claro".

**4. O gradiente passa UMA vez, na entrada.** O rótulo recebe uma passada de
brilho e congela. O orçamento de movimento da ADR-034 dá um loop por tela e
exige que movimento carregue dado; um brilho perpétuo gastaria esse loop num
gesto que não diz nada — o "shimmer" que a própria ADR-034 nomeia. Uma passada
na entrada é o que ela autoriza: *"anima na entrada e congela no valor final"*.

Dois guardas que o efeito exige, e sem os quais ele quebra de verdade: em
`forced-colors` o preenchimento volta a sólido, senão o texto some (recorte por
gradiente com preenchimento transparente não deixa nada visível); e o gradiente
usa `repeat`, senão as letras somem uma a uma conforme a passada avança.

### O conserto que apareceu no caminho

**A janela do lembrete nunca recebia o tema.** `data-theme` é escrito no
`documentElement` do `DesktopApp`, que é outra janela e portanto outro
documento. Esta caía no `:root` e seguia escura mesmo com o M/OS no claro. Agora
ela lê o tema na abertura e a cada lembrete — a cada, porque a janela sobrevive
entre eles, escondida e não destruída.

### Consequências

- a janela cresce 20×20 px, e o anel transparente engole clique nessa faixa;
- o lembrete passa a se parecer com o resto do M/OS, em vez de ser a exceção;
- o risco assumido: transparência em janela Tauri no Windows tem histórico de
  artefato com composição desligada. Se aparecer, o caminho de volta é remover
  `"transparent"` e a sombra, mantendo raio e o rótulo em sódio — o desenho não
  depende da sombra para funcionar.
---

## ADR-043 — O M/OS pode iniciar com o Windows, e o registro é quem manda

**Data:** 2026-08-18
**Status:** aceito, por decisão do proprietário do produto
**Revisa:** ADR-016
**Complementa:** `ATTENTION-SYSTEM.md` (decisão D-5)

### Contexto

A ADR-016 fechou o lifecycle do processo desktop: fechar a janela esconde no
tray, `Quit` encerra, e *"startup com Windows não entra na v0.1"*. Ela também
registrou a condição de revisão, e é ela que este documento cumpre:

> promover startup automático depende de necessidade observada.

A necessidade apareceu. O Attention System promete, na §1.1 do documento dele,
que **nenhum Reminder é perdido em silêncio**. Um Reminder só dispara com o
processo vivo — o agendador mora no backend, e o backend morre junto com o app.
Sem iniciar com o sistema, a promessa passa a ter uma condição escondida:

> nenhum Reminder é perdido em silêncio, *desde que você tenha aberto o M/OS
> depois do login*.

Isso não é uma promessa mais fraca. É outra promessa. Um lembrete para as 9h da
manhã, criado na véspera, não dispara se a pessoa liga a máquina às 8h50 e só
abre o M/OS às 11h — e o pior é que o sistema teria funcionado exatamente como
foi construído, sem nada quebrar, sem nada avisar.

O Attention System já trata esse caso: a reconciliação da abertura marca o
Reminder como `missed` e mostra há quanto tempo. Mas "perdido há duas horas" é
o que se faz quando o aviso falhou, não o que se promete como funcionamento
normal.

### As alternativas, e por que a escolhida

**Tarefa agendada no logon.** O M/OS já tem esse padrão em casa: o túnel do
Hermes usa uma tarefa `AtLogOn` registrada por `scripts/install-hermes-tunnel.ps1`.
Ela funciona, mas exige um script de instalação separado, aparece num lugar que
o usuário não associa ao aplicativo, e some do radar quando ele procurar onde
desligar. Serve para uma peça de infraestrutura; não serve para uma preferência
do produto.

**Processo de segundo plano separado.** Sobreviveria ao app fechado, mas
significaria dois processos disputando o mesmo SQLite. Contradiz a ADR-005
(monólito modular) e a ADR-008, que fez do banco local a fonte de verdade única.
Rejeitada.

**Plugin oficial `tauri-plugin-autostart`.** Escolhida. O que ele faz de fato,
verificado na fonte da dependência e não na documentação: por baixo usa
`auto-launch 0.5`, que no Windows grava em
`HKEY_CURRENT_USER\SOFTWARE\Microsoft\Windows\CurrentVersion\Run`. Chave por
usuário: **não pede elevação**, o que preserva a promessa da ADR-016 de que o
M/OS não pede admin, e não escreve fora do perfil do usuário.

### Decisão

**O M/OS pode iniciar com o Windows, por opção explícita, desligada por padrão.**

Duas preferências em Settings, ambas começando desligadas:

```text
Iniciar com o Windows        [ ]
Iniciar minimizado           [ ]
```

E três regras que decidem como isso se comporta.

**O registro é a fonte de verdade, e não uma configuração nossa.**

O `auto-launch` também escreve em
`SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run` —
a chave que o **Gerenciador de Tarefas** usa na aba Inicializar. Ou seja: o
Windows dá ao usuário um interruptor para este recurso fora do nosso aplicativo,
e ele pode desligar por lá sem nos avisar.

Portanto o toggle **lê `is_enabled()` a cada vez que a tela abre**, em vez de
espelhar um booleano nosso. Guardar a preferência num arquivo próprio criaria
duas fontes de verdade que divergem no primeiro clique feito no Gerenciador de
Tarefas — e a tela passaria a afirmar "ligado" sobre algo desligado.

Isso é a mesma regra que a ADR-016 já aplicou ao atalho global: *"a interface
nunca afirma que o atalho está disponível depois de `Quit`"*. Interface que
afirma capacidade que não tem é pior que interface sem a capacidade.

**Desligado por padrão, e por decisão.**

Um aplicativo que se instala na inicialização sem ser convidado é um aplicativo
que decidiu pelo dono da máquina. A `VISION.md` §14 diz que o M/OS existe para
reduzir carga mental, e um programa a mais subindo no logon sem pedido é carga.
Quem quiser a confiabilidade completa liga; quem não ligar recebe um sistema que
funciona com o app aberto, e a superfície não promete mais que isso.

O espírito é o mesmo da ADR-037: *"observação que não pode ser desligada é
vigilância, mesmo quando o observado é o dono da máquina."* Inicialização que
não pode ser desligada é da mesma família.

**`Iniciar minimizado` depende de `Iniciar com o Windows`.**

Sozinha ela não significa nada, e o M/OS já sabe nascer sem janela visível — é o
que a ADR-016 estabeleceu ao separar fechar de encerrar. O argumento vai por
linha de comando, que o plugin suporta (`.arg()`), e o `setup` decide não
mostrar a janela principal quando ele está presente.

### Consequências

- a promessa do Attention System deixa de ter condição escondida **para quem
  ligar**, e continua condicionada para quem não ligar. A superfície precisa
  dizer isso, e não sugerir confiabilidade que depende de uma opção desligada;
- o M/OS passa a poder aparecer na aba Inicializar do Gerenciador de Tarefas,
  onde o usuário pode desligá-lo sem abrir o aplicativo. Isso é bom e é
  deliberado — não vamos tentar reverter o que ele decidir por lá;
- o toggle nunca guarda estado próprio: ele pergunta ao sistema. Um PR que
  introduza um booleano espelhando isso reintroduz a divergência que esta ADR
  existe para evitar;
- a ADR-016 continua valendo em tudo o mais: fechar esconde no tray, `Quit`
  encerra, e o atalho global só funciona com o processo vivo;
- desinstalar o M/OS deve remover a entrada do registro. Um programa que some do
  disco e continua listado na inicialização deixa lixo que o usuário não sabe de
  onde veio;
- esta ADR autoriza a preferência, não o P1 inteiro. Canal de notificação do
  Windows, tray com "Próximo" e ações no toast continuam sendo trabalho próprio,
  descrito no `ATTENTION-SYSTEM.md` §11 e §34.

## ADR-044 — O rail vai a doze, e Reuniões entra sem tirar ninguém

**Data:** 2026-08-19
**Status:** aceito, por decisão do proprietário do produto
**Revisa:** ADR-039, ADR-038, ADR-036, ADR-031

### Contexto

A ADR-039 levou o rail a onze e manteve a regra herdada: *"o décimo segundo
exige retirar um, ou uma ADR nova que justifique não retirar, como esta fez"*.

Este é o décimo segundo, e esta ADR existe para que ele seja revisão e não
omissão.

O pedido: o Meeting Agent (`MEETING-AGENT.md`) — gravar uma reunião, transcrever
localmente, analisar com o Hermes e transformar o que ficou combinado em Task e
Reminder.

### Decisão

**Doze destinos. Reuniões entra sem ninguém sair, e o teto passa a ser doze** —
com a mesma regra de troca valendo daqui em diante.

O critério fixado pela ADR-036 é *"algo de que depende a renda ou a **memória** do
usuário, não algo que ele usa com frequência"*. Reuniões passa por ele pela
segunda metade, e de forma literal: **uma reunião gravada é memória.** É o único
lugar do M/OS onde uma hora de conversa — decisões, prazos, compromissos — deixa
de existir se ninguém a guardar.

E ela passa também pela primeira metade, por consequência: as decisões que saem
de uma reunião comercial sustentam trabalho faturado.

### Por que não as alternativas

**Dentro de Calendário** foi considerado com seriedade. Calendário já é o destino
temporal e já mostra o que aconteceu em cada dia, e uma reunião é exatamente
isso. Mas Calendário responde *"o que aconteceu naquele dia?"*, e Reuniões
responde *"o que ficou combinado?"* — perguntas diferentes, com objetos
diferentes. Enterrar a lista de reuniões atrás de uma navegação de mês faria a
segunda pergunta custar três cliques.

**Sem ícone no rail, alcançável só pelo `Ctrl+K`** está descartado por evidência,
e não por preferência. A ADR-031 registra que esse experimento já foi feito com
Workspaces e falhou: o item ficou *"invisível para quem não conhece o Command"*
até ser promovido de volta. Repetir sabendo o resultado não é rigor.

**Retirar um dos onze** foi a saída da ADR-038, e ali havia um candidato claro —
Apps era conveniência. Hoje não há: os onze passam pelo critério de renda ou
memória, e remover qualquer um seria trocar uma violação de contagem por uma
perda real.

### Consequências

- o teto vira doze, e a regra de troca continua: o décimo terceiro exige retirar
  um, ou outra ADR;
- Reuniões entra no grupo `TRABALHO`, depois de Finance e antes de `MEMÓRIA`.
  Ela pertence ao mesmo eixo de Tempo e Calendário — o que aconteceu, e quando;
- `mos-design-system.md` continua dizendo "máximo 6". A distância entre o
  documento e o produto passa a ser de seis, e ela está registrada aqui e nas
  quatro ADRs anteriores em vez de descoberta depois;
- **o risco assumido cresce, e vale nomeá-lo de novo:** cada revisão do teto
  torna a próxima mais fácil, e é assim que um rail vira depósito. A defesa nunca
  foi o número — é a exigência de uma ADR para mexer nele, e o fato de que
  escrever esta obrigou a comparar Reuniões com Calendário antes de aceitar.

### O que esta decisão NÃO autoriza

Ela autoriza o destino, e não a superfície. A barra de gravação vive no shell —
fora do rail e fora da página — porque a promessa de que **nunca se grava sem
indicação visível** (`MEETING-AGENT.md` §17.2) não pode depender de qual tela
está aberta. Isso é uma peça de shell, como o Argos e o estado de sistema, e não
um décimo terceiro destino.


## ADR-045 — O rail volta a oito, e o recém-chegado nasce no leque

**Data:** 2026-08-19
**Status:** aceito, por decisão do proprietário do produto
**Revisa:** ADR-044, ADR-039, ADR-038, ADR-036, ADR-031

### Contexto

O teto do rail foi de seis a oito (ADR-031), nove (ADR-036), dez (ADR-038), onze
(ADR-039) e doze (ADR-044) — **cinco revisões em pouco mais de duas semanas**.
Cada uma argumentou bem o seu caso. Nenhuma segurou o conjunto.

O padrão é o diagnóstico: o teto era um **número**, e um número só sabe dizer
não. Quando o destino seguinte tinha mérito — e todos tinham —, a única saída
era subir o teto de novo. A ADR-038 chegou a exigir uma troca, e a troca saiu
por evidência de desuso (*"o banco do usuário tem zero apps cadastrados"*); a
039 e a 044 entraram sem tirar ninguém, cada uma com a sua justificativa.

Enquanto isso o grupo TRABALHO acumulou **sete dos onze** destinos. Sete itens
sob um rótulo é uma lista, não um grupo — o rótulo para de informar, que é
exatamente o serviço que ele deveria prestar.

### Decisão

**O rail volta a oito. Calendário, Finance e Reuniões saem para o leque. E a
regra deixa de ser um número:**

> Destino novo **nasce no leque**. Ele sobe ao rail quando provar ser renda ou
> memória — o critério que a ADR-036 já tinha escrito e que continua valendo.

O leque é o degrau que faltava entre "existe" e "mora no rail". Antes dele só
havia dois estados: rail ou Ctrl+K. E o Ctrl+K é *recall* — exige saber o nome e
digitá-lo —, então cair nele era virar invisível para quem não conhece o
vocabulário. É a falha que a ADR-031 registrou quando Workspaces foi rebaixado:
ele ficou *"invisível para quem não conhece o Command, até ser promovido de
volta"*.

Os grupos passam a usar o vocabulário que a própria ADR-038 fixou — *"Library é
memória, Inbox é a entrada dela, Workspaces é a lente sobre tudo, e Tempo é de
onde sai a renda"*:

| grupo | destinos |
|---|---|
| GERAL | Home, Hermes |
| TRABALHO | Tasks, Projects, Tempo, Workspaces |
| MEMÓRIA | Inbox, Library |

### Por que Workspaces não saiu

Foi considerado e descartado **por evidência registrada**, e não por gosto. A
ADR-031 conta que ele já foi rebaixado uma vez e o que aconteceu. Repetir um
experimento cujo resultado está escrito é o que essas decisões existem para
evitar.

### Consequências, incluindo as que doem

**Reuniões sai do rail com dois dias de vida.** Ela nasceu no commit `bafbfb5`,
em 19/08, e nunca chegou a formar hábito — tirá-la agora é decidir **sem
evidência de uso**, o oposto do que a ADR-038 fez com Apps, que saiu porque o
banco provava zero cadastros. A mitigação tem duas camadas: ela nasce fixada no
leque, e a **barra de gravação continua na topbar**, que é onde mora a promessa
da §17.2 do `MEETING-AGENT.md` — a indicação de que o microfone está aberto
nunca dependeu do rail.

**Cinco pétalas é teto, e não ponto de partida.** Um leque que cresce vira um
segundo rail, e aí o problema apenas mudou de lugar. O número está travado no
código (`SLOTS`, em `lequePetalas.ts`) com um teste que garante que o ângulo de
um slot não muda conforme os outros são preenchidos — porque é a estabilidade
dos ângulos, e não o desenho, que faz o leque valer a pena.

**As portas entram junto com a saída.** Os três destinos que saíram ganham botão
no widget AÇÕES da Home no mesmo commit. É a dívida que a ADR-038 registrou ao
tirar Apps: sem a porta nova, *"a pagina ficaria inalcancavel"*.

**O que esta ADR não consegue prever:** se o leque de fato substitui o rail no
uso diário. Isso só aparece depois de uma semana de uso real. Se não substituir,
o caminho de volta é promover ao rail o que estiver sendo mais tocado no leque —
e não subir o teto de novo.

---

## ADR-046 — Todo drop vira Capture primeiro, e a entidade vem depois

**Data:** 2026-08-19
**Status:** aceito
**Contexto:** Universal Drop Zone (`TECHNICAL-FOUNDATION-V0.3-UNIVERSAL-DROP.md`)
**Revisa:** ADR-021 (acrescenta o tipo `file` ao Resource)

### A pergunta

Um PDF arrastado para dentro da janela: o que ele é?

A resposta óbvia é *Resource*. Ele vai para a Library, tem título, tem motivo,
tem lifecycle. O caminho curto seria criar o Resource direto — copiar o arquivo,
inserir a linha, pronto.

### Por que o caminho curto está errado

Porque ele decide antes de preservar.

Criar o Resource exige saber o título, o tipo e — se o drop aconteceu dentro de
um Project — a relação. São três perguntas, e cada uma pode falhar: o parser não
abre, o hash não bate, a extração explode, a relação aponta para um Project
arquivado. Num pipeline que cria a entidade no fim, **qualquer uma dessas falhas
custa o arquivo**, porque não existe nada gravado antes dela.

E o M/OS já tem o registro certo para "algo entrou e ainda não foi decidido". Ele
se chama Capture, existe desde a v0.1, tem a durabilidade do `synchronous=FULL`
(ADR-017), aparece na Inbox, entra na Search e sabe derivar outras entidades
preservando proveniência (ADR-004, ADR-018).

### A decisão

**Todo drop grava uma Capture antes do primeiro byte de conteúdo.**

```text
DROP → Capture (commit)  →  bytes no disco (commit)  →  Resource + relações (commit)
        ↑                    ↑                          ↑
        nada se perde        nada se perde              enriquecimento
        a partir daqui       a partir daqui             pode falhar
```

A Capture nasce na Inbox dizendo o nome do que chegou. Quando o Resource é
criado, ele nasce com `source_capture_id` apontando para ela — e o mesmo código
que já processava Capture → Task marca a Capture como `processed`, tirando-a da
Inbox. **Nenhuma linha nova de regra de proveniência foi escrita**; o caminho já
existia.

O efeito é que as invariantes do briefing deixam de ser cuidado e viram
consequência:

| invariante | por que ela vale |
|---|---|
| o original não se perde | a Capture commitou antes dos bytes; os bytes commitaram antes da entidade |
| IA não é necessária para salvar | não há IA em nenhum dos três commits |
| tipo desconhecido ainda entra | `DetectedKind::Unknown` é resultado, não erro |
| falha não destrói a captura | a Capture fica na Inbox, com o nome do arquivo |
| a Inbox é a rede de segurança | ela já era, e continua sendo, sem segunda lista |

### Três consequências que exigiram mexer no schema

**1. `captures.source_kind` aceita `'drop'`.** Registrar um arquivo arrastado como
se tivesse sido digitado na Home apagaria exatamente o fato que a ADR-004 existe
para guardar. A migration 0023 recria a tabela — e, diferente da 0007, `captures`
**tem filhas**: `tasks` e `resources` apontam para ela. O procedimento é o
documentado pelo SQLite (`foreign_keys=OFF` + `legacy_alter_table=ON` em volta da
transação), e o `migrate()` roda `foreign_key_check` depois. Um teste sobe um v22
povoado com Task derivada, Resource derivado e vínculo de Workspace, e prende as
duas pontas: nada se perde, nada fica órfão.

**2. `resources.kind` aceita `'file'`.** Um arquivo preservado não é site, não é
biblioteca e não é nota. Ele não tem `url`: o caminho do original mora na linha de
ingestão, endereçado pelo hash do conteúdo. Guardar o caminho no Resource também
criaria duas verdades sobre onde o arquivo está, e um dia elas discordariam.

**3. `resource_projects` passa a existir.** É cópia estrutural de
`resource_workspaces` (0009), N-para-N pelo mesmo motivo: o mesmo memorial pode
servir a dois Projects. Sem ela, o caso B dos critérios de aceite — *soltar dentro
de um Project e ficar relacionado* — não teria onde ser gravado. Isso **não** abre
a exceção que a ADR-012 fecha: continua não havendo grafo genérico, apenas uma
tabela de par para uma relação concreta e usada.

### A tabela `ingestions` não é uma segunda Library

Ela guarda o que a Capture e o Resource não sabem dizer: hash, MIME, tamanho,
caminho do original, estado da leitura de conteúdo, o contexto da tela no
instante do drop e o que a ingestão acrescentou de relação. Nada é procurado nela
— a Library lê Resources, a Inbox lê Captures, a Search lê os índices. Ela é a
**memória do pipeline**, e é o que permite responder depois "de onde isso veio" e
"por que isto foi relacionado àquilo".

### O que a lente de Workspace compra, e o que ela custa

A confiança para relacionar sozinho está calibrada assim:

| sinal | confiança | ação |
|---|---:|---|
| Project aberto na tela | 0.95 | relaciona |
| Task aberta (o Project dela) | 0.90 | relaciona |
| lente de Workspace ativa | 0.80 | relaciona |
| nome do arquivo cita um Project | 0.60 | **sugere** |
| nada | 0 | não inventa |

A terceira linha é a discutível, e ela foi decidida contra o instinto. Uma lente
de Workspace é um sinal mais fraco que um Project aberto — mas **a Library filtra
por ela por padrão**. Um Resource sem o vínculo nasce invisível exatamente na tela
onde a pessoa está parada. Errar aqui custa uma relação a mais, visível e
desfazível; não vincular custa o item sumir na hora em que ele deveria aparecer.

A quarta linha é a que o instinto pediria para promover — *"NexoDoc-pricing.pdf,
óbvio que é do NexoDoc"* — e ela ficou em sugestão de propósito. Nome de arquivo
é convenção pessoal, não declaração; relacionar sozinho por causa dele seria o
sistema inventando contexto, que é o que o §20 do briefing proíbe.

### O que ficou de fora, e por quê

**Não existe diálogo de "onde isto pertence?".** Ele estava no briefing, e o
pipeline o tornou desnecessário: preservar primeiro + Inbox como rede + desfazer
no recibo cobrem os três casos em que a pergunta apareceria. Perguntar seria
cobrar uma decisão no momento exato em que o produto promete não cobrar (§4 do
`VISION.md`).

**A duplicata não pergunta.** Ela relaciona o contexto novo ao Resource antigo e
diz o que fez, com desfazer. O desfazer remove **apenas o que aquela ingestão
acrescentou** — duas colunas booleanas na linha existem só para isso, porque
remover uma relação que já estava lá seria destruir contexto alheio a pretexto de
corrigir.

**Sem OCR e sem embeddings.** Um PDF escaneado registra `extraction_state =
'empty'`, e isso não é uma falha: é a fila de trabalho do OCR no dia em que ele
existir.

## ADR-047 — A detecção de reunião observa o microfone, e nunca o conteúdo

**Data:** 2026-08-19
**Status:** aceito, por decisão do proprietário do produto
**Revisa:** ADR-037

### Contexto

O Meeting Agent grava, transcreve e analisa — mas só se alguém lembrar de abrir o
M/OS e clicar. O pedido do proprietário foi o que o Notion faz: uma janelinha que
aparece sobre o Meet oferecendo gravar.

A escolha inicial dele foi **detectar por título de janela**, e ela foi tomada a
partir de uma tabela que eu apresentei — na qual eu classifiquei "microfone em
uso" como a opção mais cara. **Estava errado.** A pergunta dele — *"como o Notion
faz?"* — desfez o erro. A conta oficial do Notion é explícita:

> "On desktop, Notion can detect that a meeting app is active and show this
> prompt to start AI Meeting Notes. **It doesn't read your browser content** or
> listen to audio unless you actively start notes."

### Decisão

**O M/OS observa qual programa está com o microfone aberto.** A fronteira da
ADR-037 vai de *"nomes de programa, e nada além disso"* para *"nomes de programa,
e qual programa está com o microfone aberto"*.

O dado vem do `ConsentStore` do Windows, por leitura de registro — sem hook, sem
injeção, sem captura. Dois campos atravessam: **quem** e **desde quando**.

**O que esta ADR explicitamente NÃO autoriza**, e é isso que mantém a fronteira
estreita: ler título de janela, ler conteúdo de aba, escutar o áudio. Saber que o
Chrome abriu o microfone não diz com quem se fala nem sobre o quê.

### Por que microfone, e não título

Título de janela expõe **conteúdo** — "Orçamento Vila Nova — Chrome", "Demissão
do Fulano.docx — Word". Microfone expõe uma **capacidade**.

E há uma razão melhor que privacidade: o microfone detecta o **fato certo**. Uma
aba do Meet aberta não é uma reunião; um microfone aberto é. Título exigiria uma
lista de padrões — Meet, Zoom, Teams, e o que se esquecesse —, e disparia com aba
aberta sem reunião.

### Consequências, incluindo a que dói

**Ligada de fábrica.** Isso significa que a fronteira é atravessada **com aviso e
não com pedido**. A ADR-037 desenhou a fronteira justamente para que atravessá-la
fosse difícil e visível, e ligar por padrão atravessa por decisão do produto e não
da pessoa. Foi escolha do proprietário, com o trade-off na mesa, e o argumento a
favor é o mesmo do Notion: uma feature que exige ser descoberta não serve a quem
não a descobre.

O toggle é a mitigação, e ele mora em **Settings → REUNIÕES**, não em Avançado. O
texto ao lado dele diz o que a feature não faz, porque é isso que a pessoa precisa
para decidir.

**A oferta nunca vira gravação sozinha.** Continua valendo o *"observação não vira
hora sozinha"* da ADR-037: a janela oferece, e alguém clica.

**Três exclusões que o código impõe:** o próprio `mos-desktop.exe` — ele abre o
microfone quando grava, e sem a exclusão o detector se veria gravando e ofereceria
gravar; qualquer processo enquanto já há gravação em curso; e processo silenciado
pelo "não neste app".

**O que esta ADR não consegue prever:** se vinte segundos de espera é cedo, tarde
ou irritante. O número foi escolhido para separar reunião de teste de som, sem
evidência de uso. Reveja depois de uma semana; se for irritante, o caminho é subir
a espera, e não desligar a feature.

---

## ADR-048 — A voz entra pelo campo que já existe, e o silêncio não vira Task

**Status:** aceita · 2026-08-19 · `feat/voice-inbox`

### Contexto

A Fase 7 do `ROADMAP.md` pede uma coisa só: *"o usuário consegue capturar uma
ideia sem precisar parar para digitar ou navegar"*. A auditoria antes do código
achou quatro coisas prontas e uma armadilha.

Prontas: a janela `quick-capture` já é um overlay de 640px com atalho global; o
componente dela já carregava quatro traços de amplitude comentados como
*"apagados até a voz existir (fase adiada)"*; `mos-audio` já captura microfone
por WASAPI com recuperação de queda e RMS; e `mos-transcribe` já implementa a
porta `TranscriptionProvider`, com o whisper instalado na máquina.

A armadilha é a de 19/08 no Meeting Agent: **o whisper preenche silêncio com
texto inventado**. Um canal quase mudo transcreveu `"Legenda por Sônia Ruberti"`,
e o nome inventado chegou ao resumo do Hermes. Numa reunião isso é ruído. Numa
Voice Inbox, seria uma Task nascida de uma tecla encostada.

E o que não existia **no ponto de partida**: `Universal Drop Zone` e
`ingestion pipeline` só apareciam em `docs/`, então a branch nasceu apoiada em
`Capture` mais `tasks.source_capture_id`, que era o pipeline de ingestão real.

**A master os construiu enquanto isso** (ADR-046), e o merge reconciliou em vez
de deixar dois sistemas. O veredicto foi que `Ingestion` e `VoiceNote` são
irmãos e não duplicatas: `Ingestion` é de ARQUIVO — mime, sha256, duplicata,
extração, contagem de páginas —, e nada disso significa coisa alguma para sete
segundos de fala, assim como pico de energia e transcrição não significam nada
para um PDF. O que os dois compartilham é o princípio e o ponto de encontro: a
Capture nasce primeiro, cada um com a sua origem. O que era duplicata de
verdade — `ProjectHint`, o par (id, nome) que as duas superfícies usam para
perguntar "a que Project isto pertence?" — foi eliminado: a voz passou a usar o
do planejador de relações do drop.

### Decisão

**Voz não é um modo, é uma forma de digitar** — o `mos-design-system.md` §Voz já
dizia isso, e a implementação obedece: nenhuma janela nova, nenhum ícone de
microfone, a mesma barra `/`, o mesmo campo. Segurar para falar, não alternar.

1. **`Ctrl+Alt+G`, segurado.** O padrão foi MEDIDO e não escolhido: a primeira
   opção (`Ctrl+Alt+Space`, para ficar na família do `Ctrl+Shift+Space`) não
   registra nesta máquina — `RegisterHotKey` devolve 1409. Um padrão que não
   registra é uma feature que não existe.
2. **Dois pisos antes de transcrever, e um filtro depois.** Duração mínima de
   400 ms, pico de RMS mínimo de 120 em 1000, e a família de créditos de legenda
   recusada no domínio. Recusado, **nada é persistido** — nem linha, nem byte.
3. **`voice_notes` guarda o áudio até existir texto.** `Capture.content` é
   `NOT NULL` não-vazio e o domínio não reescreve conteúdo; uma Capture não pode
   nascer antes da transcrição sem inventar conteúdo falso. Mesmo desenho de
   `Meeting`, e pela mesma razão.
4. **O áudio é apagado assim que a Capture existe**, e sobrevive exatamente
   enquanto a informação ainda não foi preservada. Sem enum de retenção: oito
   segundos de "comprar café" não têm valor de reescuta.
5. **A leitura da fala é determinística.** `mos_core::voice` lê data natural em
   pt-BR, Project e intenção sem rede e sem IA. O Hermes não participa desta
   feature. Confiança alta age sozinha com Desfazer; média oferece por ⏎ com a
   Capture já salva atrás; baixa fica na Inbox. **Marcador de hesitação vence
   verbo e data juntos** — "talvez eu devesse" não autoriza nada.
6. **A migration 0025 recria `captures`** para admitir mais uma origem —
   SQLite não altera `CHECK`, e widening custa uma recriação por vez. Ela segue
   o procedimento que a 0023 já tinha estabelecido: `foreign_keys=OFF` e
   `legacy_alter_table=ON` fora da transação, `RENAME` em vez de swap, e o
   `verify_foreign_keys` do `migrate()` conferindo depois. Nasceu 0022 na branch
   e virou 0025 no merge, porque a master chegou ao 24 primeiro.

### O portão de abertura, que não era desta feature

Rodar o app derrubou o M/OS duas vezes, em comandos que já existiam:

```text
mos_desktop_lib::attention::attention_count -> AppHandle::state::<AppState>
panicked: state() called before manage() for AppState
thread caused non-unwinding panic. aborting.
```

O Tauri cria as janelas declaradas em `tauri.conf.json` **antes** de chamar o
`setup`, e a webview já emite IPC enquanto o banco abre. `state()` nesse instante
não devolve erro: aborta o processo. São 84 pontos que chamam `state()` à mão.

A guarda entrou **no `invoke_handler`**, num lugar só: nenhum comando roda antes
de o app estar pronto. Cobre os 84 e o comando que alguém escrever amanhã.

O defeito é pré-existente, mas entra nesta ADR porque a 0025 roda exatamente
nessa janela na primeira abertura depois desta versão — e a alarga.

### Consequências

- a voz vira uma origem de `Capture` (`source = voice`), e nada mais: aparece na
  Inbox, no Search e no Calendário pelos caminhos que já existiam;
- `Waiting For` continua não existindo, então "João disse que vai mandar o
  orçamento sexta" vira Capture. É o comportamento correto, e não uma falha;
- o `blur` deixou de encerrar gravação. Ele era guarda de microfone e virou o
  oposto: quem fala pelo atalho global está, por definição, em outro programa;
- `mos-audio` ganhou `start_mic`. A ausência do loopback é a decisão: gravar o
  que sai pelos alto-falantes enquanto alguém dita um lembrete capturaria a
  reunião aberta atrás.

## ADR-048 — Argos ganha corpo, e o orçamento de movimento abre uma exceção nomeada

**Data:** 2026-08-19
**Status:** aceito, por decisão do proprietário do produto
**Revisa:** ADR-041 (condições 1 e 3) e o orçamento de movimento da ADR-034.
Abre exceção nomeada contra `UX-PRINCIPLES.md` §16 e `HERMES-PREMIUM-CHAT.md` §7.6.

### Contexto

O proprietário pediu Argos em 3D, grande, ancorado no canto inferior direito.

Quatro documentos proíbem isso na letra: a condição 3 da ADR-041 ("não tem loop
nem piscada ociosa"), o orçamento da ADR-034 ("um loop por tela", já gasto na
barra do `Symbol.tsx`), o §7.6 ("avatar, orb, glow permanente, spinner grande") e
o §16 ("orbes decorativos", "interfaces que parecem demos de IA").

Não há leitura em que o pedido não contradiga os quatro.

### Decisão

**A exceção é concedida, e é nominal: ela vale para Argos e para mais nada.**

**1. O orçamento passa a ser "um loop por tela, mais o Argos".** Não "dois
loops": qualquer terceiro laço continua precisando de ADR própria.

**2. A exceção é paga com pausas, e elas são parte da decisão.** Sem foco não
desenha. Minimizado não desenha. Oculto não desenha. E sob
`prefers-reduced-motion` não há laço nenhum — um quadro por pose, e congela.
Quem pediu menos movimento continua recebendo exatamente o que a ADR-034
prometia.

**3. A condição 2 da ADR-041 não é tocada.** Cada pose continua sendo um fato
ligado a um sinal que já existe, `poseFor` não muda uma linha, e as seis poses
continuam sendo seis. É este pilar, e não o tamanho, que separava a criatura do
enfeite — e é ele que sobrevive.

**4. Há piso.** Onde o WebGL não sobe, o SVG de 22px desenha as mesmas seis
poses. O bicho novo não pode custar a face do estado a ninguém.

### O `aria-hidden` cai

A ADR-041 escondia Argos dos leitores de tela com o argumento de que "os mesmos
fatos já são anunciados em texto pelo estado de sistema ao lado". Saindo da
topbar, não há mais nada ao lado — o argumento morre com a mudança de endereço. E
virando controle que abre o Centro de Atenção, esconder deixa de ser opção.

Argos passa a ser um botão com nome próprio, e o nome diz o **fato**, nunca a
expressão: "Estado do sistema: aguardando sua aprovação", e não "arregalado".

### A silhueta, e o risco assumido

A ADR-041 recusou o blob da referência porque ele é reimplementação do mascote da
x.ai. **Aquele problema era de propriedade, não de geometria** — um blob de
desenho próprio não o herda.

Mas continua perto da silhueta que o §16 alerta. Isso é risco assumido pelo
proprietário, com o conflito à vista, e está escrito aqui para que ninguém, daqui
a seis meses, precise adivinhar se foi descuido.

### O ruído é soma de senos, e não simplex de terceiro

O corpo deforma por três senos em frequências primas entre si, escritos no
próprio vertex shader. Um simplex de biblioteca daria um corpo mais orgânico, e
a 72px a diferença não se vê — mas traria GLSL de outra licença para dentro do
repo, que é exatamente o cuidado que fez esta família de decisões recusar o
desenho da referência.

### Consequências

- a topbar perde o segundo espelho de estado que a ADR-041 lhe deu, e fica com o
  texto e o spinner;
- o renderer ganha `three` por import dinâmico, num chunk separado do `index`:
  quem não monta o bicho não paga o download;
- o risco que a ADR-041 nomeou — "uma criatura é mais fácil de esticar do que uma
  barra" — cresce com o tamanho. A defesa continua sendo a mesma: qualquer pose
  nova exige um sinal que já exista;
- o dia em que a bateria incomodar, o primeiro corte é a taxa de quadros em
  `desperto`, que ocupa 90% do tempo — e não desligar o resto.

### Adendo de 2026-08-21 — o corpo passa a 96px, e o número sai do código

O dono do produto pediu Argos maior. O corpo vai de **72px para 96px**, e o
texto acima que fala em "72px" deve ser lido como o valor original, não como o
valor corrente.

O que muda de forma mais durável que o número é onde ele mora. Eram **dois
lugares** que podiam divergir em silêncio — o atributo do `<canvas>` em
`Argos.tsx` e a regra `.argos-canvas` no CSS. Agora existe `--argos-corpo` em
`tokens.css`, e quem precisa reservar espaço para o bicho no layout lê a mesma
fonte em vez de repetir a conta: foi assim que o rodapé do Hermes passou a
acompanhar o crescimento sozinho.

Quem manda no tamanho **renderizado** continua sendo o CSS: `argosScene.ts`
chama `setSize(clientWidth, clientHeight)`, então o atributo do canvas só vale
no quadro entre a montagem e a subida da cena WebGL.

O adendo também corrige um defeito que estava escondido desde a decisão
original: o piso sem WebGL saía com os **22px do `viewBox`**, e não com o
tamanho do corpo. Numa máquina com driver velho, VM ou sessão remota —
exatamente o caso que o piso existe para cobrir — Argos aparecia três vezes
menor que o normal. As duas versões passam a ocupar a mesma caixa.

A frase da decisão que dizia que "a 72px a diferença não se vê" continua
valendo como registro do que foi medido na época; a 96px ela não foi medida de
novo, e se alguém for trocar os três senos por simplex, é aqui que a conta
precisa ser refeita.

## ADR-050 — A página de Tempo passa a se chamar CronoCAD, e leva a marca junto

**Data:** 2026-08-20
**Status:** aceito, por decisão do proprietário do produto
**Revisa:** ADR-036, no nome do destino. O argumento dela para a página existir
— "o usuário fatura por hora, então tempo rastreado é o registro de onde sai a
renda dele" — continua valendo integralmente.

### Contexto

A ADR-032 decidiu que o CronoCAD viraria superfície nativa do M/OS, e ele virou:
a página de Tempo tem as mesmas seis telas, com os mesmos nomes, e o
`TempoPage.tsx` diz por quê — *"trocar 'Painel' por 'Visão geral' obrigaria quem
usou o app por meses a reaprender onde as coisas estão"*.

O mesmo argumento nunca foi aplicado ao **nome do destino no rail**. Ele ficou
"Tempo", que é como o M/OS chama a matéria, e não como o dono chama a
ferramenta. Quem usou o CronoCAD por meses procura "CronoCAD".

### Decisão

**O destino passa a se chamar CronoCAD, e ganha a marca dele no rail.**

`Tempo` descrevia a matéria; `CronoCAD` nomeia a ferramenta que a pessoa
procura. É a mesma doutrina que já tinha preservado os nomes das seis telas,
aplicada um nível acima.

**O identificador da página segue `"tempo"`.** Ele não aparece em tela nenhuma, e
renomeá-lo tocaria roteamento, leque, Command e widget da Home para trocar uma
string que ninguém lê. Nome de código não é nome de produto.

### O ícone é a única exceção a "preenchido = currentColor"

A marca do CronoCAD é um C dentro de um quadrado colorido, e a cor faz parte do
que o dono reconhece. Duas consequências:

**1. Sai em sódio, e não no vermelho de origem.** Quem manda na cor é este
design system. Vermelho aqui seria o segundo sinal que a ADR-034 não autoriza.

**2. Só no estado ATIVO.** Sódio fixo faria o destino parecer permanentemente
selecionado, e o rail perderia o sinal de onde a pessoa está — que é o único
trabalho que a cor tem no resto daquela fila. Inativo, ele é traço como os
outros sete.

**O C é buraco, e não recorte.** No estado cheio ele leva a cor do fundo, como o
olho do Argos — um setor de anel, caminho simples e fechado. É o que a regra de
construção do `Icon.tsx` manda usar no lugar de furo com `fill-rule`, que é onde
ícone desenhado sem poder olhar para a tela costuma quebrar.

### Consequências

- o rail ganha o primeiro destino com cor própria; qualquer segundo exige ADR,
  ou a fila vira uma paleta e o sódio para de significar "você está aqui";
- a trilha de contexto e a topbar passam a dizer CRONOCAD;
- o leque e o seletor de pétala acompanham, senão o mesmo destino teria dois
  nomes dependendo de onde é aberto;
- o CronoCAD avulso continua existindo em `apps/cronocad`. Enquanto os dois
  existirem com o mesmo nome, o dono tem duas portas para a mesma atividade e
  dois bancos que já divergiram — isto está registrado aqui porque é dívida, e
  não desenho.

---

## ADR-051 — O Hermes opera o M/OS, e a busca acontece antes do envio

**Aceita em:** 2026-08-20.

### Contexto

A ADR-024 estabeleceu que o Hermes é superfície e não segundo agente. A ADR-028
escolheu injeção de contexto como caminho de leitura, e registrou a consequência
com todas as letras: *"o agente não consegue pedir mais dados no meio do turno:
o contexto é fixo no envio"*. A SPEC-ACOES-ENTRE-APPS deu a ele um catálogo de
ações. Cada peça estava certa; juntas, elas ainda produziam um chatbot.

O caso que expôs o buraco:

> *"Criar lembrete para hoje de noite às 20:30 para enviar tipos de bases
> faltantes para o Victor, task já cadastrada no kanban."*

O que chegava à VPS era essa frase e mais nada. Quatro ausências, e nenhuma
delas era do modelo:

1. **Ninguém dizia onde ele estava.** "Kanban" era, para um agente que também
   atende WhatsApp, um conceito de metodologia — não a coluna de Tasks aberta na
   tela naquele segundo.
2. **Ninguém dizia que horas eram.** "Hoje às 20:30" não é uma data até alguém
   dizer que dia é hoje, e em que fuso.
3. **Ninguém dizia que a Task existia.** O contexto só carregava o que o usuário
   anexasse à mão com `@`, e quem escreve "a task do Victor" não anexa nada.
4. **Não havia ação de lembrete.** O catálogo tinha nove ações e nenhuma
   agendava. A única coisa que o modelo podia propor sobre aquela frase era
   `mos.task.create` — a duplicata exata que o usuário disse para não criar.

### Decisão

**O M/OS pesquisa a própria base antes de enviar, e manda o resultado junto.**

Ao submeter, o M/OS extrai os termos da frase — descartando conectivos, verbos
de comando e o vocabulário do próprio produto —, roda uma varredura por termo no
FTS local e prefixa ao prompt um bloco de candidatos com `kind`, id curto,
rótulo e um distintivo. Junto descem a identidade operacional, a data e hora com
fuso, e a tela aberta com o Project e a Task que estão nela.

A varredura é **por termo**, e não uma consulta só. O FTS do M/OS junta termos
com `AND`, o que é certo para a caixa de busca e errado aqui: exigir que uma
Task contenha "enviar", "tipos", "bases", "faltantes" e "victor" ao mesmo tempo
não acharia a Task que existe. Uma busca por termo dá semântica de OU, e a
contagem de termos que bateram vira o ranking.

**Referências passam a resolver em degraus.** Id inteiro, prefixo de id, título
exato, começo de título, pedaço de título — o primeiro degrau que acerta decide,
e só a ambiguidade dentro do degrau que acertou vira pergunta.

**Um segundo salto, e só um.** Quando a busca automática não basta, o modelo
pode pedir uma busca escrevendo um bloco ```mos-query```; o M/OS executa
localmente e reenvia o resultado pelo mesmo socket.

### Por que isto não reabre a ADR-028

A ADR-028 adiou o MCP local porque expor um servidor da máquina à VPS muda a
superfície de ataque que `ARCHITECTURE.md` §15.2 não cobre. Nada disso acontece
aqui: **o M/OS continua sendo quem fala primeiro em todo salto.** Não há porta
nova, não há inversão de túnel, e nenhum dado sai sem o M/OS escolher o que sai.
O que a ADR-028 chamou de limitação — "o agente não consegue pedir mais dados" —
deixa de doer não porque o agente ganhou um canal, mas porque **o M/OS parou de
esperar que o usuário fizesse a busca por ele**.

O custo é honesto e é um só: o turno inteiro roda duas vezes quando há salto.
Por isso o teto é um. Cada salto é um `prompt.submit` sobre um túnel SSH até uma
VPS, e o custo aparece como silêncio na tela.

### Consequências

- o preâmbulo desce em toda mensagem e é o maior custo fixo de token do chat;
  cada palavra dele é paga em toda conversa, e o catálogo de ações precisa
  continuar cabendo numa linha por ação;
- o bloco de candidatos **sai da máquina**. Não são dados pessoais em bloco, mas
  são títulos de Task e nomes de Project. Isso entra no registro da ADR-027 como
  o resto: a busca vira UMA parte `context_ref` com origem automática, e os
  nomes do que foi ficam em `fields`. Um chip por candidato seria honesto e
  ilegível — doze chips numa mensagem sem anexo esconderiam os anexos de verdade;
- o catálogo ganha quatro ações — lembrete com vínculo, resolução de lembrete,
  Capture em Task e troca de Project —, e `ReminderSource::Hermes`, que existia
  desde o P0 e nunca tinha sido escrito, passa a ser;
- o contexto ambiente da tela sai do `VoiceRuntime` e vira `surface.rs`, fonte
  única para a voz e para o Hermes. Duas cópias dariam dois lugares para a tela
  publicar e um dia para elas divergirem;
- o rastro do que cada ação tocou é gravado dentro da própria proposta
  (`ActionAudit`), e não numa tabela nova: a conversa já guardava a ação crua, o
  instante e a conversa — faltavam a entidade resolvida e o estado anterior;
- **ambiguidade continua sendo pergunta.** O trabalho aqui foi tirar a pergunta
  do caminho comum, não tirá-la do sistema. Agir sobre a Task errada é pior que
  perguntar qual delas.

### Revisar quando

O salto único não bastar para trabalho composto de verdade — "arquiva essas
captures e me mostra o que sobrou" —, ou quando o preâmbulo passar a competir
por espaço com a conversa. Os dois casos apontam para a ADR de MCP local que a
ADR-028 já previu, e aí ela terá o caso concreto que faltava.
