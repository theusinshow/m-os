# M/OS — Core Foundation

## 1. Status e propósito

**Status:** aprovado para a fundação da v0.1

**Data:** 2026-08-13

Este documento refina o modelo conceitual descrito em `CORE.md` sem substituí-lo.

Seu objetivo é definir:

- limites entre conceitos;
- invariantes do domínio;
- ciclos de vida;
- relações essenciais;
- decisões que podem orientar o modelo de dados;
- pontos que continuam deliberadamente abertos.

Este documento não define schema SQL final nem autoriza implementação.

## 2. Princípios normativos

As seguintes regras são invariantes do produto:

1. Uma informação pode entrar no M/OS sem classificação.
2. Capturar e organizar são operações distintas.
3. A persistência de uma Capture deve acontecer antes de qualquer interpretação opcional.
4. Processar uma Capture não pode apagar sua origem.
5. Uma entidade derivada não é uma mutação de tipo da Capture.
6. Uma mesma informação não deve ser duplicada para aparecer em visualizações diferentes.
7. Kanban, Inbox, Library, Home e Search são visualizações ou projeções, não entidades do domínio.
8. Hermes não faz parte do Core e não é necessário para o Core funcionar.
9. Relações devem acrescentar contexto sem exigir classificação completa no momento da captura.
10. Exclusão de uma relação não pode excluir implicitamente as entidades relacionadas.

## 3. Linguagem do domínio

### 3.1 Capture

Registro de algo que entrou no cérebro digital.

Uma Capture representa a preservação da entrada original, não uma conclusão sobre o que ela significa.

Campos conceituais mínimos:

- identidade global;
- conteúdo original;
- origem;
- instante de captura;
- estado de processamento;
- estado de lifecycle;
- metadata específica da origem, quando existir;
- instante de arquivamento ou descarte, quando aplicável.

Origens inicialmente previstas:

- `desktop-main`;
- `desktop-quick-capture`;
- importação manual futura;
- iOS e share extension futuros;
- voz e integrações futuras.

O conteúdo de origem deve continuar disponível depois do processamento.

### 3.2 Inbox

Inbox é a projeção das Captures que aguardam uma decisão.

Não deve existir como container proprietário de dados nem como segundo tipo de item.

Conceitualmente:

```text
Inbox = Captures cujo processing_state é inbox
        e lifecycle_state é active
```

Mover algo para fora da Inbox altera seu estado de processamento. Não altera sua identidade e não apaga a Capture.

### 3.3 Project

Contexto de algo que está sendo desenvolvido, mantido ou acompanhado.

Project não é limitado a software e não deve absorver responsabilidades de Workspace, App ou Task.

### 3.4 Task

Ação que precisa ser realizada.

Uma Task pode:

- ser criada diretamente;
- ser derivada de uma ou mais Captures;
- existir sem Project;
- possuir no máximo um Project na v0.1;
- aparecer em diferentes visualizações sem ser duplicada.

Estados, a partir da v0.3:

- `inbox`;
- `backlog`;
- `planned`;
- `doing`;
- `review`;
- `done`.

A ordem acima é a ordem das colunas do Kanban.

**Histórico e revisão.** A v0.1 propôs apenas `backlog`, `doing` e `done`, com duas justificativas: `Planned` dependia de uma semântica temporal ainda inexistente, `Review` dependia de um fluxo de trabalho ainda não validado, e o termo `inbox` foi recusado para evitar colisão com a Inbox de Captures.

As duas primeiras justificativas continuam corretas — e a decisão de incluir os estados assume que **coluna de Kanban é visualização, não semântica**. Uma Task em `planned` não promete nenhum comportamento temporal, e `review` não dispara fluxo algum. Se um dia essa semântica existir, ela se apoia em estados que já estarão persistidos, em vez de exigir migração de dados.

**A colisão de `inbox` é real e permanece.** `Task.state = "inbox"` **não** é a Inbox de Captures: Capture tem `processing_state`, Task tem `work_state`. São conceitos distintos que compartilham o nome porque o design usa INBOX como rótulo da primeira coluna. A mitigação é nomenclatura e documentação, não renomeação — mudar o rótulo quebraria o design fechado. O aviso está no enum (`work.rs`) e na migration (`0007_v03_design.sql`).

Isto revoga a recomendação de `ARCHITECTURE-REVIEW.md` de remover `Planned`.

Task também possui `lifecycle_state` separado do estado de trabalho:

- `active`;
- `archived`;
- `trashed`.

Transições: **livres entre quaisquer estados**. O Kanban permite arrastar um card para qualquer coluna, e restringir transições aqui quebraria o gesto sem proteger nada. `completed_at` continua exclusivo de `done`: entrar carimba, sair limpa.

```text
inbox <-> backlog <-> planned <-> doing <-> review <-> done

active <-> archived
active <-> trashed
```

Ao entrar em `done`, `completed_at` é preenchido. Reabrir uma Task move explicitamente para `backlog` e remove `completed_at`. Arquivar ou restaurar não altera o estado de trabalho.

Arquivar Project não arquiva suas Tasks. Enviar Project para Trash é bloqueado enquanto existirem Tasks relacionadas fora de Trash; o usuário precisa primeiro remover as relações ou tratar essas Tasks.

### 3.5 Workspace

Perspectiva ampla sobre informações relacionadas a um contexto de atividade.

Workspace não é uma pasta nem uma fronteira de acesso.

No modelo conceitual:

- um Project pode estar relacionado a mais de um Workspace;
- um App pode estar relacionado a mais de um Workspace;
- Search atravessa todos os Workspaces;
- ausência de Workspace nunca impede recuperação.

Workspaces não fazem parte da primeira entrega vertical, embora o modelo de Project não deva impedir sua introdução posterior.

### 3.6 Resource

Informação preservada por seu valor de consulta futura.

Link, imagem, arquivo, site, biblioteca e referência são formas de Resource, não entidades concorrentes por padrão.

Resource pertence à fase de memória e não precisa ser implementado na v0.1.

### 3.7 App

Ferramenta conhecida pelo ecossistema M/OS.

App representa algo que pode ser encontrado e aberto. Project representa algo que está sendo desenvolvido ou acompanhado.

Um software próprio pode existir simultaneamente como:

```text
Project: M-Finance em desenvolvimento
App: M-Finance disponível para uso
```

São registros distintos ligados por relação explícita.

App Registry é a coleção pesquisável de Apps, não uma entidade adicional.

### 3.8 Reminder

Intenção de chamar atenção em determinado momento ou condição.

Reminder é diferente de:

- prazo de Task;
- data planejada;
- evento de calendário;
- notificação já entregue.

O modelo temporal completo permanece fora da v0.1.

### 3.9 Integration

Capacidade de comunicação com sistema externo.

Integration pertence à arquitetura de aplicação, não ao núcleo de dados pessoais. Ela pode expor capacidades como:

- `access`;
- `read`;
- `write`;
- `automate`.

Um App pode existir sem Integration. Uma Integration pode atender mais de um contexto ou App.

### 3.10 Hermes

Camada futura que consulta e executa comandos sobre o Core.

Hermes:

- não escreve diretamente na persistência;
- não cria semântica paralela ao domínio;
- usa os mesmos comandos disponíveis para as interfaces tradicionais;
- deve informar interpretação, confiança e efeitos;
- deve respeitar confirmação, auditabilidade e Undo conforme o risco.

## 4. Proveniência de Capture

### 4.1 Regra central

Processar uma Capture cria entidades e relações. Não converte o registro original in-place.

Exemplo:

```text
Capture
"Refatorar navbar da Minarum amanhã"
        |
        +-- derivou --> Task "Refatorar navbar"
        |                  |
        |                  +-- Project: Escadas Minarum
        |
        +-- permanece consultável como origem
```

### 4.2 Cardinalidade

- uma Capture pode derivar zero, uma ou várias entidades;
- uma entidade pode receber contexto de uma ou várias Captures;
- remover uma derivação não remove nenhum dos lados;
- excluir ou arquivar uma Capture não exclui entidades derivadas;
- excluir uma entidade derivada não exclui sua Capture de origem.

### 4.3 Processamento e lifecycle

Capture possui duas dimensões independentes.

`processing_state` registra se uma decisão de organização foi tomada:

- `inbox`: aguarda decisão;
- `processed`: uma decisão explícita foi tomada.

`lifecycle_state` registra retenção e visibilidade operacional:

- `active`: participa das superfícies normais;
- `archived`: preservada, fora das superfícies ativas;
- `trashed`: aguardando exclusão definitiva.

As dimensões não apagam uma à outra. Uma Capture processada e arquivada volta a ser processada ao ser restaurada. Uma Capture não processada enviada para Trash volta para Inbox ao ser restaurada.

Transições iniciais:

```text
processing_state: inbox -> processed
                  processed -> inbox   somente por Undo explícito

lifecycle_state:  active <-> archived
                  active <-> trashed
```

Uma Capture pode ser marcada como `processed` sem gerar outra entidade. Isso representa decisões como:

- manter como nota simples;
- reconhecer que não exige ação;
- preservar apenas para busca.

### 4.4 Atomicidade

Ao processar uma Capture em Task, a criação da Task, a relação de proveniência e a mudança de estado da Capture devem ocorrer na mesma transação.

Se qualquer etapa falhar, nenhuma delas deve ser apresentada como concluída.

## 5. Identidade, tempo e revisão

Todas as entidades persistentes devem possuir identificadores globais gerados no cliente.

O modelo deve manter pelo menos:

- `created_at`;
- `updated_at`;
- `archived_at`, quando aplicável;
- `deleted_at`, para tombstone ou lixeira;
- versão local opcional somente quando necessária para concorrência otimista dentro do desktop.

Versão de sincronização, ordenação entre dispositivos e identidade de tombstones serão definidas apenas na ADR futura de sync.

Datas técnicas devem ser armazenadas em UTC. A interpretação de datas naturais e a apresentação devem respeitar timezone e locale do usuário.

## 6. Matriz inicial de relações

| Origem | Destino | Cardinalidade conceitual | v0.1 |
|---|---|---:|---|
| Capture | Task | muitos para muitos | Sim, começando por uma origem comum |
| Task | Project | zero ou um Project por Task | Sim |
| Project | Workspace | muitos para muitos | Não |
| App | Workspace | muitos para muitos | Não |
| Project | App | muitos para muitos | Não |
| Project | Resource | muitos para muitos | Não |
| Task | Resource | muitos para muitos | Não |
| Entity | Reminder | um para muitos | Não |
| Project | Repository | inicialmente zero ou um | Não |

A implementação inicial deve usar relações específicas e integridade referencial normal.

Não será criada uma tabela genérica de grafo para antecipar `Everything linkable`. Um mecanismo genérico só será considerado quando relações reais não couberem de forma sustentável no modelo explícito.

## 7. Projeções

### Inbox

Captures não processadas, ordenadas por captura e com processamento rápido.

### Kanban

Tasks agrupadas por estado. Mover uma Task altera a Task existente.

### Search

Índice derivado e reconstruível sobre conteúdo pesquisável. O índice não é fonte de verdade.

Por padrão, Search agrupa Capture e entidades derivadas em um único resultado contextual:

- quando existe uma única entidade ativa derivada, ela aparece como resultado primário e a Capture como origem subordinada;
- quando existem várias entidades derivadas, o resultado primário é um grupo de proveniência identificado pela Capture, com cada derivado como destino separado;
- texto que existe apenas na Capture continua contribuindo para o match;
- quando uma Capture não possui derivação, ela aparece como resultado primário;
- filtros específicos podem expor origens separadamente no futuro.

Esse agrupamento evita dois resultados quase idênticos sem esconder proveniência.

Na v0.1b, a interface permite derivar no máximo uma Task por Capture. A cardinalidade conceitual continua muitos para muitos para não confundir essa limitação de interface com uma invariável permanente.

### Home

Composição contextual. Não possui dados próprios e não deve virar depósito de módulos.

### Library

Exploração de Resources. Não duplica Resources.

## 8. Archive, Trash e exclusão

Arquivar, concluir e excluir são operações diferentes.

- `Archive` remove de superfícies ativas, mantendo busca e relações.
- `Done` representa conclusão de Task.
- `Trash` permite recuperação antes de exclusão definitiva.
- exclusão definitiva deve ser explícita e não cascatear para entidades relacionadas sem uma regra específica e visível.

Na v0.1, Captures e Tasks devem preferir Archive ou Trash a exclusão imediata. Essas operações alteram `lifecycle_state` e preservam `processing_state` ou estado de trabalho.

## 9. Search document

Search deve indexar uma projeção uniforme, sem exigir uma superentidade no domínio.

Exemplo conceitual:

```text
SearchDocument
- entity_type
- entity_id
- title
- body
- context
- updated_at
```

O documento pode ser reconstruído a partir das entidades. Um hit resolve para um `SearchResultGroup`: selecionar o resultado primário abre a entidade derivada única ou o grupo; selecionar um derivado abre aquele derivado; selecionar a origem abre a Capture.

## 10. Decisões adiadas

Permanecem deliberadamente abertas:

- modelo completo de Resources e anexos;
- taxonomia de Library;
- datas, prazos, agenda e Reminders;
- relação de uma Task com múltiplos Projects;
- relações arbitrárias entre entidades;
- histórico detalhado de alterações;
- semântica de aliases;
- representação de Notes e Ideas além de Capture preservada;
- deduplicação;
- busca semântica;
- permissões de Hermes;
- integração GitHub.

Essas decisões não devem contaminar o schema da v0.1 sem caso de uso atual.

## 11. Questões para revisão independente

1. Preservar Capture e entidade derivada cria ruído de busca ou dupla contagem?
2. O agrupamento de Search preserva contexto suficiente quando uma Capture deriva várias entidades?
3. A cardinalidade inicial Task → Project é restritiva demais?
4. Relações específicas continuarão sustentáveis até a fase Context?
5. Quais invariantes precisam ser reforçadas no banco além da camada de aplicação?
