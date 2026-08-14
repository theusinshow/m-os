# M/OS — Core

## 1. Objetivo

O Core representa os conceitos fundamentais que existem dentro do M/OS.

Ele não define como essas informações serão armazenadas tecnicamente.

Seu objetivo é estabelecer uma linguagem comum para o produto.

Os principais conceitos identificados até o momento são:

- Capture
- Inbox
- Project
- Task
- Workspace
- Resource
- Library
- App
- Reminder
- Integration
- Hermes

Esses conceitos formam a base sobre a qual o restante do produto poderá ser construído.

---

## 2. Princípio Fundamental

O M/OS deve permitir que informações entrem no sistema antes mesmo que o usuário saiba exatamente o que elas são.

Por isso existe uma distinção importante entre:

**capturar informação**

e

**organizar informação.**

Uma informação não precisa nascer como Task, Resource ou Project.

Ela pode simplesmente entrar como uma captura.

Depois, o usuário ou futuramente o Hermes poderá determinar o que fazer com ela.

---

## 3. Capture

Capture representa qualquer coisa que o usuário deseja retirar da cabeça ou guardar rapidamente.

Exemplos:

> Preciso corrigir a navbar.

> Ideia para o M-Finance.

> Gostei desse site.

> Pesquisar essa biblioteca depois.

> Me lembra de falar com fulano.

Uma Capture pode ter origem em:

- texto;
- voz;
- link;
- imagem;
- arquivo;
- compartilhamento externo;
- outras formas futuras de entrada.

Capture deve exigir o mínimo possível de informação obrigatória.

Seu objetivo é velocidade.

---

## 4. Capture não significa classificação

Ao capturar algo, o usuário não deve ser obrigado a decidir imediatamente se aquilo é:

- tarefa;
- ideia;
- referência;
- lembrete;
- recurso;
- nota.

Pode simplesmente registrar.

Exemplo:

> testar Motion nessa Hero

Isso já é suficiente para entrar no sistema.

Posteriormente essa informação poderá ganhar contexto.

---

## 5. Inbox

Inbox representa informações que entraram no M/OS e ainda precisam ou podem ser processadas.

É uma área temporária de confiança.

O usuário deve poder pensar:

> Coloquei no M/OS. Depois eu vejo.

Um item na Inbox poderá posteriormente:

- permanecer como nota;
- virar Task;
- virar Resource;
- ser relacionado a Project;
- receber Reminder;
- entrar na Library;
- ser arquivado;
- ser descartado.

---

## 6. Processamento da Inbox

Processar a Inbox significa transformar informação bruta em informação contextualizada.

Exemplo:

Capture:

> Refatorar navbar da Minarum amanhã.

Após processamento:

**Task**

Refatorar navbar

**Project**

Escadas Minarum

**Date**

Amanhã

O usuário não deveria precisar realizar manualmente todas essas etapas quando o sistema for capaz de inferi-las com segurança.

---

## 7. Project

Project representa algo que está sendo desenvolvido, construído, mantido ou acompanhado.

Exemplos:

- M/OS;
- M-Finance;
- ChronoCAD;
- Escadas Minarum;
- projetos profissionais;
- projetos pessoais.

Project é um dos principais elementos de contexto do M/OS.

---

## 8. Project como contexto

Um Project poderá reunir ou relacionar:

- Tasks;
- Captures;
- Resources;
- Links;
- Apps;
- GitHub;
- Reminders;
- referências;
- arquivos;
- informações futuras.

Isso permite que diferentes informações relacionadas ao mesmo trabalho sejam encontradas a partir de um único contexto.

---

## 9. Project não significa necessariamente software

Apesar de muitos projetos atuais serem softwares ou sites, Project não deve conceitualmente ser limitado a desenvolvimento.

Um Project pode representar qualquer iniciativa relevante que possua começo, evolução ou objetivo.

A implementação futura poderá decidir como diferentes tipos de projetos são tratados.

---

## 10. Task

Task representa algo que precisa ser feito.

Exemplo:

> Refatorar navbar.

Uma Task pode existir sozinha ou possuir contexto.

Possíveis relações:

- Project;
- Workspace;
- Reminder;
- GitHub;
- data;
- prioridade;
- outras Tasks;
- recursos relacionados.

---

## 11. Task e Capture são diferentes

Uma Capture representa:

> Algo entrou na minha cabeça e eu quero guardar.

Uma Task representa:

> Existe uma ação que precisa ser realizada.

Nem toda Capture precisa virar Task.

Exemplo:

> Gostei muito da tipografia desse site.

Isso provavelmente é uma referência, não uma tarefa.

---

## 12. Estado das Tasks

Tasks possuem estados para representar seu progresso.

Desde a v0.3 os seis conceitos abaixo estão implementados, nesta ordem — que é também a ordem das colunas do Kanban. Ver `CORE-FOUNDATION.md` para as regras de transição e para a nota sobre a colisão entre `inbox` e a Inbox de Captures.

Conceitos:

- Inbox;
- Backlog;
- Planned;
- Doing;
- Review;
- Done.

Esses estados permitirão diferentes representações, incluindo Kanban.

---

## 13. Kanban

Kanban não é um objeto fundamental separado do Core.

Kanban é uma forma de visualizar e manipular Tasks.

Isso significa que a mesma Task poderá aparecer:

- dentro de um Project;
- no Kanban;
- na Home;
- em Today;
- em uma busca;
- em uma agenda.

Sem criar cópias diferentes da mesma tarefa.

---

## 14. Workspace

Workspace representa um contexto amplo de atividade.

Exemplos imaginados:

- Web Design;
- Engenharia;
- Finance;
- Learning.

Workspace ajuda a reduzir ruído e apresentar informações relevantes para determinado contexto.

---

## 15. Workspace e Project

Um Workspace pode conter ou relacionar diferentes Projects.

Exemplo:

**Web Design**

→ Escadas Minarum  
→ Coded by M  
→ outros projetos de sites

Outro exemplo:

**Engineering**

→ projetos técnicos  
→ ferramentas relacionadas  
→ atividades profissionais

A relação final poderá ser mais flexível, mas conceitualmente Workspace representa um agrupamento maior que Project.

---

## 16. Workspace e Apps

Apps também poderão estar associados a Workspaces.

Exemplo:

**Web Design**

- Screenshot Tool
- Figma
- GitHub
- outras ferramentas

**Engineering**

- ChronoCAD
- NexoDoc
- outras ferramentas

Isso permite que entrar em um Workspace também signifique acessar rapidamente as ferramentas daquele contexto.

---

## 17. Resource

Resource representa algo que vale a pena guardar e recuperar posteriormente.

Exemplos:

- link;
- site;
- biblioteca;
- referência;
- imagem;
- ferramenta;
- artigo;
- documentação;
- arquivo;
- inspiração.

Resources formam parte da memória de longo prazo do M/OS.

---

## 18. Resource e Capture

Um Resource pode começar como Capture.

Exemplo:

O usuário envia:

> https://motion.dev

e escreve:

> Gostei dessa biblioteca para animações.

Inicialmente isso pode entrar na Inbox.

Depois poderá ser processado como Resource.

---

## 19. Library

Library é uma forma de navegar e organizar Resources.

Assim como Kanban é uma visão de Tasks, Library deve ser entendida principalmente como uma forma de acessar conhecimento e referências armazenadas.

Ela poderá possuir agrupamentos como:

- Web Design;
- Development;
- Libraries;
- Inspiration;
- Tools;
- Typography;
- Motion;
- outras categorias.

A estrutura final ainda deverá ser definida.

---

## 20. Por que o usuário salvou algo?

Uma preocupação importante da Library é preservar contexto.

Salvar apenas:

`https://algum-site.com`

não é suficiente.

O M/OS deve permitir preservar:

> Por que isso chamou minha atenção?

Exemplo:

**Site**

algum-site.com

**Nota**

Gostei da forma como a Hero faz transição para a próxima seção.

Isso aumenta muito o valor da informação quando ela for encontrada meses depois.

---

## 21. App

App representa um software ou ferramenta acessível através do ecossistema M/OS.

Pode ser:

- software criado pelo usuário;
- aplicação web;
- programa desktop;
- ferramenta externa;
- serviço utilizado frequentemente.

Exemplos:

- M-Finance;
- ChronoCAD;
- Screenshot Tool;
- NexoDoc;
- Figma;
- outras ferramentas.

---

## 22. App Registry

O conjunto de Apps conhecidos pelo M/OS forma conceitualmente um App Registry.

O objetivo é evitar depender da memória para lembrar:

- quais ferramentas existem;
- para que servem;
- onde estão;
- como acessá-las.

Cada App poderá possuir contexto próprio e relações com outros elementos do M/OS.

---

## 23. App e Project

App e Project são conceitos diferentes.

Exemplo:

**App**

Screenshot Tool

é uma ferramenta.

Enquanto:

**Project**

Escadas Minarum

é algo sendo desenvolvido.

O Project pode utilizar determinado App.

Um App também pode ser ele próprio resultado de um Project.

Exemplo:

M-Finance pode existir simultaneamente como:

- projeto em desenvolvimento;
- aplicativo disponível para uso.

Essa distinção deve ser preservada conceitualmente.

---

## 24. Apps independentes

O M/OS não precisa possuir internamente o código de todos os Apps.

Um App pode ser totalmente independente.

O M/OS precisa apenas conhecer sua existência e como acessá-lo.

Integrações mais profundas são opcionais.

---

## 25. Reminder

Reminder representa a intenção de ser lembrado sobre algo em determinado momento ou condição.

Exemplos:

> Me lembra disso amanhã.

> Preciso fazer isso sexta.

> Me lembra daqui a uma semana.

Reminder pode estar relacionado a:

- Task;
- Project;
- Capture;
- outras informações.

---

## 26. Reminder e Task

Reminder e Task não são necessariamente a mesma coisa.

Task:

> Refatorar navbar.

Reminder:

> Me lembrar amanhã de olhar essa tarefa.

Essa separação permite que o sistema represente tanto trabalho quanto atenção temporal.

---

## 27. Time Context

O tempo será um contexto importante no M/OS.

Informações poderão eventualmente estar relacionadas a conceitos como:

- hoje;
- amanhã;
- determinada data;
- prazo;
- lembrete;
- agenda.

A representação técnica disso será definida posteriormente.

---

## 28. Integration

Integration representa uma conexão entre o M/OS e um sistema externo ou outro App.

Exemplos:

- GitHub;
- calendário;
- ChronoCAD;
- M-Finance;
- Hermes;
- ferramentas futuras.

Uma integração pode possuir diferentes capacidades.

---

## 29. Níveis conceituais de integração

### Access

M/OS sabe como abrir a ferramenta.

### Read

M/OS consegue consultar determinadas informações.

### Write

M/OS consegue criar ou alterar determinadas informações.

### Automate

M/OS consegue executar fluxos envolvendo aquela ferramenta.

Nem toda integração precisa oferecer todas as capacidades.

---

## 30. GitHub como Integration

GitHub é um exemplo importante.

Um Project poderá possuir relação com um repositório.

Isso permite que uma Task eventualmente seja relacionada a:

- Issue;
- branch;
- Pull Request;
- outras informações de desenvolvimento.

Exemplo:

**Project**

Escadas Minarum

↓

**Repository**

escadas-minarum

↓

**Task**

Refatorar navbar

↓

**GitHub Issue**

Issue relacionada

A estrutura técnica será definida posteriormente.

---

## 31. Hermes

Hermes possui papel diferente dos demais conceitos.

Ele é imaginado como uma camada de inteligência capaz de interpretar e operar os elementos do Core.

Hermes não deve ser considerado o próprio Core.

O M/OS precisa continuar funcionando mesmo sem inteligência artificial.

---

## 32. Hermes e Capture

Uma das funções mais importantes do Hermes será interpretar capturas.

Entrada:

> Amanhã tenho que refatorar a navbar da Minarum.

Possível interpretação:

- intenção: Task;
- conteúdo: Refatorar navbar;
- Project: Escadas Minarum;
- Time Context: amanhã.

Isso reduz o trabalho manual de organização.

---

## 33. Hermes e ações

Além de interpretar informações, Hermes poderá futuramente solicitar ou executar ações disponíveis no sistema.

Exemplos:

> Salva isso.

> Cria uma tarefa.

> Abre o ChronoCAD.

> Coloca isso no projeto M-Finance.

> Me lembra amanhã.

> Cria uma Issue dessa tarefa.

O Core deve existir independentemente dessas ações para que Hermes opere sobre conceitos estáveis.

---

## 34. Search

Search deve atravessar diferentes conceitos do Core.

Uma pesquisa não deveria exigir que o usuário saiba previamente se está procurando:

- Task;
- Resource;
- Project;
- App;
- Capture.

O usuário simplesmente procura.

Exemplo:

> motion

O sistema poderá encontrar:

- biblioteca Motion;
- uma referência que utiliza Motion;
- uma Task sobre Motion;
- uma nota mencionando Motion.

---

## 35. Relações são fundamentais

O valor do M/OS não estará apenas nos objetos armazenados.

Estará principalmente nas relações entre eles.

Exemplo:

```text
Workspace
Web Design
    │
    └── Project
        Escadas Minarum
            │
            ├── Tasks
            │
            ├── Resources
            │
            ├── Repository
            │
            └── Apps
```

Outro exemplo:

```text
Capture
"Refatorar navbar amanhã"
        │
        ↓
Task
Refatorar navbar
        │
        ├── Project → Escadas Minarum
        ├── Reminder → Amanhã
        └── GitHub → Issue
```

---

## 36. Informação única, múltiplas visualizações

Um princípio importante do Core é evitar duplicação conceitual.

Uma Task não deve precisar ser recriada para aparecer em diferentes lugares.

A mesma Task poderá aparecer:

```text
Project
   ↓
Task
   ↑
Kanban

Today
   ↓
Task

Search
   ↓
Task
```

São diferentes formas de acessar a mesma informação.

---

## 37. Contexto progressivo

Informações podem começar simples e ganhar contexto ao longo do tempo.

Exemplo:

### Momento 1

> testar essa biblioteca

### Momento 2

Resource:

Biblioteca X

### Momento 3

Relacionada a:

Web Design

### Momento 4

Relacionada também a:

Escadas Minarum

O M/OS não deve exigir contexto completo desde o primeiro momento.

---

## 38. Relações imaginadas atualmente

Em nível conceitual:

```text
M/OS
│
├── Workspace
│   ├── Project
│   │   ├── Task
│   │   ├── Resource
│   │   ├── App
│   │   └── Integration
│   │
│   ├── App
│   └── Resource
│
├── Inbox
│   └── Capture
│
├── Library
│   └── Resource
│
├── Kanban
│   └── Task
│
└── Reminder
    └── Related Context
```

Essa representação é conceitual e não deve ser interpretada automaticamente como estrutura de banco de dados.

---

## 39. Core mínimo

Independentemente das funcionalidades futuras, os conceitos mais fundamentais identificados são:

**Capture**

Algo entrou no cérebro digital.

**Project**

Algo está sendo desenvolvido ou acompanhado.

**Task**

Algo precisa ser feito.

**Workspace**

Em qual contexto amplo isso existe.

**Resource**

Algo merece ser lembrado ou consultado.

**App**

Uma ferramenta disponível no ecossistema.

Esses conceitos formam o núcleo inicial do produto.

---

## 40. Objetivo do Core

O Core deve permitir que o M/OS responda progressivamente a perguntas como:

> O que eu pensei?

> O que preciso fazer?

> Em que estou trabalhando?

> Onde está aquilo que salvei?

> Qual ferramenta eu uso para isso?

> O que pertence a esse projeto?

> Quando preciso lembrar disso?

> Como essas informações estão relacionadas?

Se o sistema consegue responder essas perguntas de forma confiável, ele começa a cumprir sua função como cérebro digital.
