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

`Design System/handoff/mos-tokens.css` é a fonte única de tokens do cliente
desktop. Componentes usam primitivas próprias, SVGs próprios e fontes locais.
Bibliotecas genéricas de UI ou ícones não entram sem uma necessidade concreta.

### Consequências

- dark, light e forced colors compartilham o mesmo contrato semântico;
- valores visuais deixam de ser espalhados pelos componentes;
- Capture e Command preservam as geometrias normativas do produto;
- extensões de token devem ser explícitas e documentadas.
