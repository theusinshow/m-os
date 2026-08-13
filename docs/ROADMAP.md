# M/OS — Roadmap

## 1. Objetivo deste documento

Este documento organiza a evolução conceitual do M/OS.

Ele não define:

- arquitetura técnica;
- stack;
- banco de dados;
- estrutura de código;
- APIs;
- implementação;
- cronograma em dias ou semanas.

Essas decisões serão feitas posteriormente.

O objetivo aqui é responder:

> **O que precisa existir primeiro para que o M/OS comece a cumprir sua função?**

E, principalmente:

> **O que não precisa existir ainda?**

---

## 2. Regra do Roadmap

Uma ideia presente em `PRODUCT.md` ou `IDEAS.md` não significa automaticamente que ela pertence à próxima versão.

O produto deve evoluir de acordo com necessidades reais percebidas durante o uso.

A ordem deve priorizar:

1. captura;
2. confiança;
3. recuperação;
4. organização;
5. contexto;
6. integração;
7. inteligência;
8. automação.

---

## 3. Princípio de construção

O M/OS deve se tornar útil antes de se tornar inteligente.

Primeiro:

> consigo confiar nele para guardar o que está na minha cabeça.

Depois:

> consigo organizar e encontrar aquilo.

Depois:

> consigo conectar meus projetos e ferramentas.

Só então:

> o sistema começa a interpretar e agir por mim.

---

## 4. Fase 0 — Fundação

### Objetivo

Transformar a visão do M/OS em produto suficientemente definido para começar a construção.

### Deve existir

- Vision;
- Product Definition;
- Core Conceptual;
- UX Principles;
- Roadmap;
- Ideas;
- arquitetura técnica;
- modelo de dados;
- decisões sobre desktop;
- estratégia de sincronização;
- estratégia mobile;
- design direction inicial;
- escopo da primeira versão.

Os primeiros seis documentos são definidos antes da arquitetura.

Os demais poderão ser elaborados posteriormente com auxílio do Codex.

---

## 5. Fase 1 — Brain

### Objetivo

Fazer o M/OS cumprir sua função mais básica:

> **tirar coisas da cabeça e confiar que elas estão salvas.**

Esta é a primeira versão que deve produzir valor real.

### 5.1 Universal Capture

Deve existir uma maneira central de registrar rapidamente uma informação.

O usuário deve poder escrever algo e salvar imediatamente.

Exemplo:

> testar Motion no projeto M-Finance

Sem necessidade obrigatória de:

- escolher categoria;
- escolher projeto;
- escolher prioridade;
- escolher tags;
- definir status.

### 5.2 Inbox

Toda captura ainda não organizada deve possuir um lugar confiável.

A Inbox deve permitir:

- visualizar Captures;
- processar Captures;
- transformar em outros elementos;
- arquivar;
- descartar.

### 5.3 Projects

O usuário deve conseguir criar e consultar Projects.

Um Project deve permitir reunir contexto relacionado ao trabalho.

Nesta fase, não precisa possuir todas as integrações futuras.

### 5.4 Tasks

O M/OS deve possuir Tasks próprias.

O usuário deve conseguir:

- criar;
- visualizar;
- editar;
- concluir;
- relacionar a um Project;
- alterar estado.

### 5.5 Kanban

Tasks devem possuir uma visualização Kanban simples.

O objetivo é fornecer visão espacial do trabalho.

Não construir um sistema de gestão empresarial completo.

### 5.6 Workspaces

O usuário deve conseguir organizar contextos amplos.

Exemplos iniciais:

- Web Design;
- Engenharia;
- Finance;
- Learning.

O escopo exato poderá ser reduzido caso a primeira versão não necessite de todos.

### 5.7 App Registry

O usuário deve conseguir cadastrar ferramentas e softwares utilizados.

Exemplos:

- M-Finance;
- ChronoCAD;
- Screenshot Tool;
- NexoDoc;
- Figma;
- GitHub;
- outras ferramentas.

Inicialmente, um App pode funcionar apenas como acesso organizado.

### 5.8 Search básica

O usuário deve conseguir encontrar elementos importantes do sistema sem depender exclusivamente da navegação.

A busca inicial poderá focar nos elementos já existentes nesta versão.

---

## 6. Critério de sucesso da Fase 1

A primeira fase pode ser considerada útil quando este fluxo funcionar de maneira confiável:

### No celular ou computador

Surge uma ideia.

↓

O usuário registra rapidamente.

↓

A informação entra na Inbox.

↓

Mais tarde, no computador, o usuário encontra essa informação.

↓

Transforma em Task.

↓

Relaciona ao Project correto.

↓

Visualiza no Kanban.

Quando esse fluxo funcionar bem, o M/OS já começou a cumprir sua função como cérebro digital.

---

## 7. Fase 2 — Memory

### Objetivo

Fazer o M/OS guardar não apenas ações, mas também conhecimento e referências.

### 7.1 Resources

Introduzir ou aprofundar o conceito de Resource.

Permitir guardar:

- links;
- referências;
- ferramentas;
- bibliotecas;
- inspirações;
- imagens;
- materiais;
- documentação.

### 7.2 Library

Criar uma forma adequada de explorar Resources.

Especialmente importante para contextos como Web Design.

Exemplos:

- sites de referência;
- bibliotecas favoritas;
- ferramentas;
- tipografias;
- Motion;
- UI;
- componentes;
- inspiração visual.

### 7.3 Contexto de salvamento

Permitir preservar:

> Por que eu salvei isso?

Exemplo:

**Link**

motion.dev

**Nota**

Gostei dessa biblioteca para animações de interface.

### 7.4 Busca ampliada

Search passa a atravessar também:

- Resources;
- Library;
- links;
- notas;
- referências.

---

## 8. Critério de sucesso da Fase 2

O usuário deve poder pensar:

> Eu lembro que salvei algo sobre isso no M/OS.

E conseguir encontrá-lo sem precisar lembrar:

- a pasta;
- o módulo;
- a categoria exata;
- quando salvou.

---

## 9. Fase 3 — Context

### Objetivo

Conectar melhor as diferentes partes do cérebro digital.

### 9.1 Relações entre informações

Aprofundar relações como:

```text
Task
→ Project
→ Workspace
→ Resource
→ App
```

Sem transformar a interface em um sistema complexo de gerenciamento de relações.

### 9.2 Project Context

Projects passam a funcionar como verdadeiros centros de contexto.

Possibilidades:

- Tasks;
- Resources;
- links;
- Apps;
- arquivos;
- atividade recente;
- integrações.

### 9.3 Context-aware creation

Quando o usuário criar algo dentro de determinado contexto, o M/OS deve reutilizar informações que já conhece.

Exemplo:

Dentro de:

**Project → Escadas Minarum**

Criar:

> Refatorar navbar

O sistema não deveria exigir selecionar novamente Escadas Minarum.

### 9.4 Recents e Favorites

Introduzir acesso rápido baseado em:

- recência;
- frequência;
- favoritos.

Especialmente para:

- Projects;
- Apps;
- Resources.

---

## 10. Fase 4 — Time

### Objetivo

Adicionar dimensão temporal ao cérebro digital.

### 10.1 Reminders

Permitir criar lembretes relacionados a:

- Tasks;
- Projects;
- Captures;
- outros itens.

### 10.2 Today

Criar uma visão clara do que importa no dia atual.

Pode reunir:

- Tasks;
- prazos;
- Reminders;
- eventos relevantes;
- trabalho em andamento.

### 10.3 Agenda

Permitir visualizar compromissos e elementos temporais.

A forma final ainda não está definida.

### 10.4 Calendar Integration

Avaliar integração com calendário externo.

O objetivo não é necessariamente substituir o calendário existente.

Pode ser suficiente conectá-lo.

---

## 11. Critério de sucesso da Fase 4

O usuário deve conseguir confiar no M/OS para responder:

> O que eu preciso lembrar hoje?

e:

> O que está chegando?

---

## 12. Fase 5 — GitHub

### Objetivo

Conectar o fluxo pessoal de Tasks ao fluxo real de desenvolvimento.

### 12.1 Repository Association

Permitir relacionar Project a um repositório GitHub.

Exemplo:

**Escadas Minarum**

↓

**Repository**

escadas-minarum

### 12.2 Task ↔ GitHub

Permitir que uma Task técnica possa, quando necessário, possuir relação com GitHub.

Possibilidades:

- Issue;
- branch;
- Pull Request.

### 12.3 Criação opcional de Issue

Uma Task não deve virar Issue automaticamente.

O usuário poderá decidir quando determinada Task merece existir também no GitHub.

### 12.4 Estado externo

O M/OS poderá futuramente refletir informações relevantes do GitHub.

Exemplo:

```text
Task
Refatorar Navbar

GitHub Issue #42
PR #51
Merged
```

---

## 13. Critério de sucesso da Fase 5

O usuário não precisa mais manter mentalmente separados:

> tarefa que estou fazendo

e

> trabalho correspondente no GitHub.

O M/OS conhece a relação.

---

## 14. Fase 6 — Hermes

### Objetivo

Transformar Hermes em camada inteligente sobre um sistema que já funciona.

Hermes não deve entrar antes de existir informação suficiente para que ele seja realmente útil.

### 14.1 Conversa básica

Permitir interação com Hermes dentro do M/OS.

Mas a experiência não deve parar em um chat isolado.

### 14.2 Hermes + Capture

Hermes poderá interpretar entradas naturais.

Exemplo:

> Amanhã preciso refatorar a navbar da Minarum.

Possível interpretação:

```text
Task: Refatorar Navbar
Project: Escadas Minarum
Date: Amanhã
```

### 14.3 Hermes + Search

Permitir consultas como:

> Onde está aquela biblioteca de animação que salvei?

> Qual era aquele site cuja Hero eu gostei?

### 14.4 Hermes + Projects

Permitir consultas como:

> O que falta fazer no M-Finance?

> Quais tarefas estão abertas na Minarum?

### 14.5 Hermes + Actions

Permitir progressivamente ações como:

> Cria uma Task.

> Guarda isso como referência.

> Me lembra amanhã.

> Abre o ChronoCAD.

As permissões e confirmações devem seguir os princípios definidos em UX.

---

## 15. Critério de sucesso da Fase 6

Hermes deixa de ser um chatbot.

Ele passa a compreender e operar o contexto do M/OS.

---

## 16. Fase 7 — Voice

### Objetivo

Permitir que pensar em voz alta seja suficiente para capturar ou solicitar algo.

### 16.1 Captura por voz

O usuário pode falar.

O sistema registra.

### 16.2 Interpretação de voz

Com Hermes, o objetivo futuro é interpretar intenção.

Exemplo:

> Guarda como ideia do M-Finance mostrar quanto da fatura atual compromete o próximo mês.

### 16.3 Voice no Mobile

O celular deve ser uma das principais superfícies para esse uso.

---

## 17. Critério de sucesso da Fase 7

O usuário consegue capturar uma ideia sem precisar parar para digitar ou navegar.

---

## 18. Fase 8 — External Apps

### Objetivo

Aprofundar a ideia do M/OS como hub do ecossistema pessoal.

### 18.1 ChronoCAD

Começar por integração somente se houver necessidade real.

Possibilidades já imaginadas:

- horas trabalhadas;
- atividade atual;
- tempo por Project;
- iniciar trabalho a partir de Task.

### 18.2 M-Finance

Possibilidades já imaginadas:

- consultas;
- resumos;
- informações importantes;
- acesso através do Hermes.

Não transformar o M/OS em duplicação do M-Finance.

### 18.3 Screenshot Tool

Possibilidades:

- acesso pelo Workspace Web Design;
- referências relacionadas a Projects;
- envio de imagens para Library.

### 18.4 Outros Apps

Novos softwares poderão entrar progressivamente no ecossistema.

O objetivo é que o App Registry permita esse crescimento sem alterações fundamentais no Core.

---

## 19. Fase 9 — Cross-App Intelligence

### Objetivo

Permitir que informações de diferentes áreas sejam relacionadas de maneira útil.

Exemplos futuros:

> Quanto tempo trabalhei nesse projeto?

> Quanto recebi por esse projeto?

> Qual foi meu valor/hora real?

> Qual projeto está consumindo mais tempo?

> Quais Projects possuem tarefas paradas?

> O que está ocupando minha semana?

Esta fase depende de integrações anteriores estarem maduras.

---

## 20. Fase 10 — Personal Operating System

### Objetivo

Chegar à visão completa imaginada para o produto.

O usuário poderá simplesmente perguntar:

> O que eu preciso fazer agora?

E o M/OS poderá considerar informações como:

- Tasks;
- Projects;
- prazos;
- Reminders;
- agenda;
- contexto atual;
- atividade;
- integrações.

Outro exemplo:

> Quero trabalhar em Web Design.

O M/OS poderá apresentar:

- Projects relevantes;
- Tasks;
- Apps;
- Resources;
- contexto recente.

---

## 21. O que não deve bloquear a primeira versão

Os seguintes elementos são importantes para a visão de longo prazo, mas não devem impedir a primeira versão de existir:

- Hermes;
- voz;
- GitHub avançado;
- calendário avançado;
- M-Finance integrado;
- ChronoCAD integrado;
- automações;
- inteligência cross-app;
- classificação automática;
- múltiplos dispositivos extremamente sofisticados;
- métricas avançadas.

---

## 22. Risco principal

O maior risco do M/OS é:

> tentar construir o sistema final antes de possuir um sistema utilizável.

O produto possui naturalmente potencial para crescer indefinidamente.

Por isso, o escopo de cada etapa deve ser protegido.

---

## 23. Regra para adicionar algo ao Roadmap

Antes de promover uma ideia do `IDEAS.md` para o Roadmap, perguntar:

1. Estou sentindo falta disso utilizando o M/OS?
2. Isso resolve um problema recorrente?
3. Isso reduz carga mental?
4. Existe uma versão mais simples do problema?
5. Precisa ser construído agora?
6. Ele depende de algo que ainda não existe?

Somente então decidir se entra em uma próxima fase.

---

## 24. Norte do Roadmap

A evolução desejada é:

```text
CAPTURE
   ↓
REMEMBER
   ↓
ORGANIZE
   ↓
CONNECT
   ↓
FIND
   ↓
ACT
   ↓
ASSIST
   ↓
AUTOMATE
```

Cada camada deve ser confiável antes que a próxima tente esconder sua complexidade.
