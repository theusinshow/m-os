# M/OS — Product

## 1. Definição do Produto

M/OS é um sistema pessoal criado para funcionar como:

- cérebro digital;
- central de organização;
- hub de softwares;
- central de projetos;
- espaço para captura de ideias;
- biblioteca pessoal;
- gerenciador de tarefas;
- ponto de acesso às ferramentas utilizadas no dia a dia;
- futura interface para interação com o Hermes.

O objetivo não é substituir todas as ferramentas existentes.

O objetivo é criar uma camada central entre o usuário e tudo aquilo que ele cria, utiliza, precisa lembrar ou pretende fazer.

---

## 2. Usuário

Inicialmente, o M/OS será um produto pessoal.

Existe um único usuário principal.

Isso permite que o produto seja construído especificamente ao redor da forma como esse usuário trabalha, pensa e organiza informações, sem necessidade inicial de generalizar comportamentos para diferentes tipos de usuários.

---

## 3. Experiência Principal

A experiência deve partir de uma necessidade simples:

> Tenho algo na cabeça. Quero tirar isso da minha cabeça agora.

Ao abrir o M/OS, deve existir uma entrada de texto em posição de destaque.

Essa entrada poderá futuramente também aceitar voz.

O usuário poderá escrever naturalmente:

> Tenho que refatorar a navbar do projeto Escadas Minarum.

Ou:

> Guarda esse site nas minhas referências de Web Design.

Ou:

> Tive uma ideia para o M-Finance.

Ou:

> Me lembra de fazer isso amanhã.

O objetivo futuro é permitir que o M/OS compreenda essas solicitações através do Hermes e execute as ações correspondentes.

---

## 4. Home

A Home deve funcionar como o ponto de entrada do cérebro digital.

Ela poderá reunir informações como:

- entrada rápida de texto;
- acesso à voz;
- tarefas do dia;
- lembretes;
- Inbox;
- projetos recentes;
- aplicativos utilizados recentemente;
- atividades importantes;
- acesso ao Hermes.

A Home não deve se transformar em um dashboard sobrecarregado.

Seu objetivo principal é responder:

> O que está acontecendo e o que preciso fazer?

---

## 5. Universal Capture

Capturar informação deve ser uma função fundamental do produto.

O usuário deve conseguir registrar rapidamente:

- ideias;
- pensamentos;
- tarefas;
- notas;
- links;
- imagens;
- referências;
- lembretes;
- arquivos;
- informações capturadas por voz.

No momento da captura, organizar a informação deve ser opcional.

O usuário pode simplesmente registrar:

> testar aquela biblioteca depois

e continuar o que estava fazendo.

A informação entra na Inbox e pode ser processada posteriormente.

---

## 6. Inbox

A Inbox funciona como a memória temporária do sistema.

Tudo aquilo que ainda não foi organizado pode aparecer nela.

Um item posteriormente poderá ser:

- transformado em tarefa;
- associado a um projeto;
- salvo como referência;
- enviado para uma biblioteca;
- transformado em lembrete;
- classificado como ideia;
- arquivado;
- descartado.

O objetivo é permitir captura rápida sem exigir decisões imediatas.

---

## 7. Projects

Projetos representam coisas que estão sendo construídas, mantidas ou desenvolvidas.

Exemplos:

- M-Finance;
- ChronoCAD;
- Escadas Minarum;
- outros softwares;
- projetos profissionais;
- projetos pessoais.

Um projeto poderá reunir informações relacionadas como:

- tarefas;
- notas;
- referências;
- links;
- aplicativos;
- repositório GitHub;
- arquivos;
- lembretes;
- atividades.

O projeto funciona como um contexto dentro do cérebro digital.

---

## 8. Tasks

O M/OS deve possuir seu próprio sistema de tarefas.

Uma tarefa poderá existir independentemente ou estar relacionada a:

- projeto;
- workspace;
- data;
- prioridade;
- GitHub;
- lembrete;
- outras informações.

As tarefas poderão ser visualizadas de diferentes maneiras.

Uma delas será através de Kanban.

---

## 9. Kanban

O Kanban será uma representação visual das tarefas.

Seu objetivo não é transformar o M/OS em uma ferramenta de gestão empresarial.

Ele deve ser simples e adequado à organização pessoal.

Exemplos de estados possíveis:

- Inbox;
- Backlog;
- Planned;
- Doing;
- Review;
- Done.

A definição final dos estados poderá ser feita durante o design do produto.

---

## 10. Workspaces

Workspaces representam diferentes contextos.

Exemplos inicialmente imaginados:

### Web Design

Pode reunir:

- projetos de sites;
- ferramentas;
- referências;
- bibliotecas;
- GitHub;
- screenshots;
- recursos;
- tarefas.

### Engenharia

Pode reunir:

- projetos;
- ChronoCAD;
- NexoDoc;
- ferramentas;
- tarefas;
- referências técnicas.

### Finance

Pode reunir:

- M-Finance;
- informações relacionadas às finanças.

### Learning

Pode reunir:

- materiais;
- referências;
- ferramentas;
- projetos de aprendizado.

Workspaces funcionam como diferentes perspectivas sobre as informações do M/OS.

---

## 11. App Hub

Uma das funções centrais do M/OS será servir como hub para softwares e ferramentas.

O usuário cria diversos softwares para resolver necessidades específicas.

O M/OS deve fornecer um local central para encontrá-los.

Exemplos:

- M-Finance;
- ChronoCAD;
- ferramenta de screenshots;
- NexoDoc;
- outras ferramentas futuras.

Cada aplicativo poderá possuir informações como:

- nome;
- descrição;
- categoria;
- workspace;
- ícone;
- endereço;
- repositório;
- status.

---

## 12. Aplicativos Independentes

Os softwares cadastrados no M/OS não precisam necessariamente fazer parte do mesmo código.

Um aplicativo pode continuar completamente independente.

O M/OS pode simplesmente:

1. conhecê-lo;
2. exibi-lo;
3. permitir abri-lo.

Posteriormente, alguns aplicativos poderão possuir integrações mais profundas.

---

## 13. Níveis de Integração

Um aplicativo poderá ter diferentes níveis de relacionamento com o M/OS.

### Atalho

O M/OS apenas permite abrir o aplicativo.

### Contexto

O M/OS conhece informações relacionadas ao aplicativo ou projeto.

### Integração

O aplicativo fornece dados para o M/OS.

### Automação

O M/OS ou Hermes consegue executar determinadas ações no aplicativo.

Nem todos os aplicativos precisam atingir todos os níveis.

---

## 14. Library

O M/OS deve possuir uma biblioteca pessoal para guardar coisas que o usuário deseja encontrar novamente.

Exemplos:

- sites interessantes;
- bibliotecas de desenvolvimento;
- referências de Web Design;
- ferramentas;
- recursos;
- inspirações;
- componentes;
- links;
- imagens;
- materiais.

Um exemplo de uso:

O usuário encontra um site com uma Hero interessante.

Ele compartilha o link com o M/OS e registra:

> Gostei principalmente da Hero.

Posteriormente poderá encontrar essa referência pesquisando por Hero, Web Design ou outros relacionamentos.

---

## 15. Links

Salvar links deve ser extremamente simples.

O usuário não deve depender dos favoritos do navegador para armazenar tudo aquilo que considera útil.

Um link poderá possuir:

- título;
- endereço;
- comentário pessoal;
- tags;
- workspace;
- projeto relacionado;
- categoria.

O objetivo é lembrar não apenas do endereço, mas também:

> Por que eu salvei isso?

---

## 16. GitHub

Projetos de desenvolvimento poderão ser associados aos respectivos repositórios GitHub.

Exemplo:

Projeto:

**Escadas Minarum**

Repositório:

**escadas-minarum**

Com essa relação criada, tarefas do M/OS poderão futuramente ser relacionadas a elementos do GitHub.

Exemplos:

- Issues;
- branches;
- commits;
- Pull Requests.

Nem toda tarefa precisa existir no GitHub.

A vinculação deve ocorrer quando fizer sentido.

---

## 17. Exemplo de GitHub + Kanban

O usuário registra:

> Refatorar navbar da Escadas Minarum.

O M/OS poderá criar uma tarefa.

Posteriormente, essa tarefa poderá ser associada ao repositório correspondente.

Uma tarefa técnica poderá eventualmente possuir:

- projeto;
- repositório;
- Issue;
- branch;
- Pull Request;
- status.

Isso permite acompanhar desenvolvimento sem transformar todas as tarefas pessoais em Issues.

---

## 18. Hermes

Hermes é imaginado como a futura camada inteligente do M/OS.

Sua função não será simplesmente responder perguntas.

Ele deverá conseguir compreender o contexto armazenado no sistema.

O usuário poderá escrever ou falar naturalmente.

Exemplo:

> Coloca na agenda e no Kanban que tenho que refatorar a navbar do projeto Escadas Minarum e me lembra depois.

O Hermes poderá interpretar:

- intenção: criar tarefa;
- tarefa: refatorar navbar;
- projeto: Escadas Minarum;
- repositório: projeto associado;
- necessidade de lembrete;
- necessidade de entrada na agenda.

Depois poderá executar as ações disponíveis no M/OS.

---

## 19. Hermes como Interface

A longo prazo, o usuário não deveria precisar navegar pelo sistema para realizar todas as operações.

Poderá simplesmente solicitar:

> Cria uma tarefa.

> Guarda isso.

> Salva esse link.

> Abre o ChronoCAD.

> Qual era aquela biblioteca que salvei?

> O que tenho para fazer hoje?

> Me lembra disso amanhã.

> Cria uma Issue dessa tarefa.

> Mostra meus projetos de Web Design.

Hermes funcionará como uma interface de linguagem natural sobre o ecossistema.

---

## 20. Confirmações

A autonomia do Hermes deve variar conforme o risco da ação.

Ações simples podem eventualmente ocorrer diretamente.

Exemplo:

> Guarda essa ideia.

Ações com efeitos externos ou mais relevantes podem exigir confirmação.

Exemplo:

> Criar Issue no GitHub.

O sistema poderá mostrar previamente as ações que pretende executar.

---

## 21. Voz

O M/OS deverá futuramente permitir interação por voz.

Isso é especialmente relevante para captura rápida no celular.

Exemplo:

> Tive uma ideia para o M-Finance. Na tela do cartão poderia mostrar quanto da fatura já compromete o próximo mês. Guarda isso como ideia do M-Finance.

A intenção futura é que o sistema não apenas transcreva a mensagem.

Ele deve conseguir interpretar:

- tipo: ideia;
- projeto: M-Finance;
- conteúdo: ideia registrada.

---

## 22. Desktop

O produto principal deve possuir experiência de aplicativo desktop.

A intenção é evitar a sensação de:

> abrir o navegador para entrar no meu cérebro.

O M/OS deve poder ser iniciado como qualquer outro programa do computador.

O desktop será a experiência mais completa do produto.

---

## 23. Mobile

O M/OS também deve possuir acesso fácil pelo celular.

O mobile terá especial importância para momentos em que pensamentos e ideias surgem longe do computador.

Possíveis funções prioritárias:

- captura;
- voz;
- Inbox;
- tarefas;
- agenda;
- lembretes;
- Hermes;
- compartilhamento de links.

A experiência mobile pode ser propositalmente mais simples que a desktop.

---

## 24. Quick Capture

No desktop, existe a ideia de futuramente disponibilizar um atalho global.

Exemplo conceitual:

`Ctrl + Shift + Space`

Uma pequena interface poderia aparecer sobre qualquer software:

> What's on your mind?

O usuário registra a informação e continua trabalhando.

O M/OS não deve exigir que ele interrompa seu fluxo apenas para registrar algo.

---

## 25. Compartilhamento Mobile

No celular, o M/OS deverá futuramente poder receber conteúdo compartilhado de outros aplicativos.

Exemplo:

O usuário encontra um site interessante.

Seleciona:

**Compartilhar → M/OS**

Adiciona:

> Referência de Hero para Web Design.

O conteúdo entra no cérebro digital.

---

## 26. Search

O M/OS deve possuir busca global.

O usuário poderá pesquisar por algo mesmo sem lembrar onde aquilo foi armazenado.

Exemplos:

> motion

> navbar

> Minarum

> biblioteca de animação

> screenshots

A busca poderá retornar diferentes tipos de conteúdo relacionados.

---

## 27. ChronoCAD

ChronoCAD é um software independente utilizado para controle de tempo de projetos.

Inicialmente, o M/OS poderá apenas fornecer acesso ao aplicativo.

Futuramente poderá existir integração.

Possibilidades imaginadas:

- projeto atual;
- tempo trabalhado hoje;
- horas por projeto;
- iniciar atividade;
- relacionar tempo a uma tarefa.

Essas possibilidades não são requisitos iniciais.

---

## 28. M-Finance

M-Finance continuará sendo um software independente.

Inicialmente poderá aparecer no App Hub.

Futuramente poderá fornecer informações ao M/OS.

Possibilidades imaginadas:

- informações financeiras resumidas;
- acesso rápido;
- relacionamento com projetos;
- consultas através do Hermes.

Essas possibilidades não são requisitos iniciais.

---

## 29. Ferramenta de Screenshots

Existe uma ferramenta utilizada para capturar imagens e screenshots de sites e projetos.

Ela poderá fazer parte do Workspace de Web Design.

O M/OS poderá inicialmente funcionar como acesso rápido para essa ferramenta.

Futuramente, referências geradas por ela poderão potencialmente ser relacionadas a projetos ou à Library.

---

## 30. Agenda e Lembretes

Tarefas e informações poderão futuramente possuir relação temporal.

Exemplos:

> Fazer amanhã.

> Me lembra disso daqui a uma semana.

> Tenho que resolver isso sexta.

O M/OS deverá conseguir representar essas intenções através de agenda, datas e lembretes.

A implementação final ainda não está definida.

---

## 31. Princípio de Organização

O usuário não deve precisar organizar perfeitamente o sistema para que ele seja útil.

Idealmente:

**capturar → continuar a vida → organizar quando necessário.**

Com inteligência suficiente, parte dessa organização poderá futuramente ser realizada automaticamente.

---

## 32. Princípio de Evolução

O M/OS não deve tentar implementar imediatamente todas as possibilidades descritas neste documento.

O produto deve crescer conforme necessidades reais aparecem durante seu uso.

Uma funcionalidade imaginada não é automaticamente uma funcionalidade aprovada.

Este documento registra a visão do produto.

O Roadmap decidirá quando — ou se — cada capacidade será construída.

---

## 33. Experiência Desejada

O objetivo final é que o usuário desenvolva confiança suficiente no sistema para pensar:

> Se for importante, está no M/OS.

Isso significa que o sistema precisa ser:

- rápido para capturar;
- fácil de consultar;
- confiável;
- acessível;
- pesquisável;
- conectado;
- pouco burocrático.

---

## 34. Produto em uma frase

> **M/OS é meu cérebro digital e a interface central entre aquilo que penso, aquilo que construo e aquilo que preciso fazer.**
