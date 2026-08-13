# M/OS — UX Principles

## 1. Propósito deste documento

Este documento define os princípios de experiência e interface que devem orientar o M/OS durante todo o seu desenvolvimento.

Ele não determina componentes específicos, biblioteca visual, framework ou implementação técnica.

Seu objetivo é responder:

> **Como deve ser usar o M/OS?**

O M/OS não é apenas mais um software utilizado durante o dia.

Ele pretende ocupar uma posição muito mais próxima do usuário:

**ser uma extensão da memória, da organização e da capacidade de agir.**

Por isso, cada decisão de UX deve considerar não apenas eficiência operacional, mas também:

- carga mental;
- velocidade;
- confiança;
- previsibilidade;
- clareza;
- contexto;
- continuidade;
- prazer de uso.

---

## 2. Ambição de experiência

O M/OS deve transmitir a sensação de um sistema:

- pessoal;
- extremamente bem construído;
- rápido;
- silencioso;
- inteligente;
- confiável;
- sofisticado;
- técnico;
- organizado;
- consistente.

A interface deve parecer projetada especificamente para sua função.

Não deve parecer um template de dashboard adaptado para outro propósito.

---

## 3. Princípio mestre

> **O M/OS deve exigir menos organização mental do que a quantidade de organização mental que remove.**

Se utilizar uma funcionalidade exige mais esforço do que simplesmente lembrar da informação por conta própria, essa experiência falhou.

O sistema existe para reduzir carga cognitiva.

Nunca para transferir a burocracia da cabeça do usuário para uma interface.

---

## 4. Capturar é mais importante que classificar

Quando uma informação surge, o objetivo imediato é preservá-la.

O usuário não deve ser obrigado a decidir naquele momento:

- qual categoria;
- qual projeto;
- qual prioridade;
- qual tag;
- qual workspace;
- qual status;
- qual estrutura.

Capturar deve vir primeiro.

Contextualizar pode acontecer depois.

Exemplo ideal:

> "Preciso testar essa biblioteca no M-Finance."

Enter.

Fim.

A informação está segura.

---

## 5. Fricção proporcional à intenção

A quantidade de interação exigida deve ser proporcional à complexidade da ação.

Salvar uma ideia:

**quase nenhuma fricção.**

Criar uma tarefa:

**pouca fricção.**

Editar informações importantes:

**fricção moderada.**

Executar ações externas ou potencialmente destrutivas:

**confirmação clara.**

O M/OS não deve utilizar o mesmo nível de cerimônia para todas as ações.

---

## 6. Capture first

A entrada universal é um dos elementos mais importantes do produto.

Independentemente de onde o usuário esteja, deve ser fácil responder à pergunta:

> **What's on your mind?**

A experiência pode aceitar futuramente:

- texto;
- voz;
- links;
- imagens;
- arquivos;
- conteúdo compartilhado.

O método de entrada pode variar.

O conceito deve permanecer constante.

---

## 7. Entrada natural antes de formulários

Sempre que possível, o M/OS deve permitir expressar intenção em linguagem natural.

Em vez de:

```text
Título:
Projeto:
Tipo:
Data:
Prioridade:
Status:
```

o usuário poderia escrever:

> Refatorar a navbar da Escadas Minarum amanhã.

O sistema pode interpretar o restante posteriormente.

Formulários continuam úteis para edição precisa.

Eles não devem ser obrigatoriamente a porta de entrada.

---

## 8. Progressive disclosure

A interface deve mostrar inicialmente apenas o que é necessário.

Complexidade adicional aparece conforme o usuário precisar dela.

Uma Task pode inicialmente mostrar apenas:

- título;
- projeto;
- estado;
- prazo relevante.

Informações como:

- GitHub Issue;
- branch;
- relacionamento;
- histórico;
- metadata;
- automações;

podem aparecer mediante expansão.

O usuário não deve ser obrigado a processar toda a estrutura interna do sistema o tempo inteiro.

---

## 9. Complexidade interna não deve virar complexidade visual

O M/OS poderá futuramente possuir muitas relações:

```text
Task
→ Project
→ Workspace
→ GitHub
→ Reminder
→ Resource
→ Hermes
```

Isso não significa que todas elas devem aparecer simultaneamente.

Uma arquitetura rica pode produzir uma interface simples.

Na verdade, esse deve ser o objetivo.

---

## 10. Informação antes de containers

Evitar construir interfaces baseadas exclusivamente em:

**card dentro de card dentro de card.**

A unidade visual principal deve ser a informação.

Containers devem ser usados quando ajudam a:

- estabelecer agrupamento;
- separar contexto;
- indicar interatividade;
- criar hierarquia.

Não simplesmente porque dashboards normalmente possuem cards.

---

## 11. Evitar Dashboard Syndrome

A Home não deve virar um painel contendo vinte métricas apenas porque existem dados disponíveis.

A Home deve responder principalmente:

> O que importa agora?

Possíveis informações:

- entrada universal;
- tarefas relevantes;
- Inbox;
- lembretes;
- atividade atual;
- projetos recentes;
- contexto temporal.

Tudo que não ajuda diretamente essa pergunta deve justificar sua presença.

---

## 12. Hierarquia visual extrema

O usuário deve conseguir compreender a prioridade de uma tela antes mesmo de lê-la completamente.

Toda tela deve possuir:

### Elemento primário

Aquilo que aquela tela existe para permitir.

### Elementos secundários

Informações e ações que ajudam a função principal.

### Elementos terciários

Metadata, opções avançadas e contexto adicional.

Se tudo chama atenção, nada possui prioridade.

---

## 13. Uma tela, uma intenção dominante

Toda superfície importante deve possuir uma intenção claramente dominante.

Exemplos:

**Home**

Entender o momento atual.

**Inbox**

Processar coisas capturadas.

**Project**

Entender e agir dentro de um contexto.

**Kanban**

Visualizar e alterar o estado do trabalho.

**Library**

Encontrar e explorar coisas preservadas.

**Apps**

Encontrar e abrir ferramentas.

Isso não significa limitar funcionalidades.

Significa preservar clareza.

---

## 14. Quiet UI

O M/OS deve possuir uma interface visualmente silenciosa.

O produto armazenará muita informação.

A interface não deve competir com ela.

Evitar uso excessivo de:

- cores;
- gradientes;
- sombras;
- borders;
- badges;
- ícones;
- animações;
- containers;
- decoração.

A estética deve vir principalmente de:

- proporção;
- tipografia;
- spacing;
- alinhamento;
- grid;
- ritmo;
- contraste;
- movimento cuidadosamente utilizado.

---

## 15. Premium não significa ornamentado

O M/OS deve possuir acabamento visual premium.

Isso não significa adicionar efeitos.

Uma interface premium pode ser extremamente simples.

Qualidade deve ser percebida através de:

- decisões consistentes;
- excelente tipografia;
- espaçamento preciso;
- microinterações;
- transições suaves;
- estados bem desenhados;
- componentes refinados;
- atenção aos detalhes.

O usuário deve perceber cuidado antes de perceber decoração.

---

## 16. Evitar AI Slop

O M/OS não deve possuir elementos visuais apenas porque são associados a produtos de inteligência artificial.

Evitar automaticamente:

- gradientes roxo/azul sem justificativa;
- glow excessivo;
- orbes decorativos;
- estrelas mágicas em todas as ações;
- textos excessivamente explicativos;
- cards genéricos de assistente;
- animações de partículas;
- glassmorphism gratuito;
- interfaces que parecem demos de IA.

Hermes deve parecer parte do sistema.

Não um produto diferente colocado dentro dele.

---

## 17. Hermes não deve sequestrar a interface

A presença de inteligência artificial não significa transformar tudo em chat.

O M/OS continuará possuindo:

- Projects;
- Tasks;
- Library;
- Kanban;
- Apps;
- Search;
- Inbox.

Hermes complementa essas interfaces.

Não precisa substituí-las.

Em algumas situações, interação visual tradicional será mais eficiente.

Em outras, linguagem natural será superior.

O sistema deve permitir ambas.

---

## 18. Hermes como camada, não destino

O usuário não deveria necessariamente precisar navegar até:

`Menu → AI → Chat`

para utilizar Hermes.

A inteligência poderá aparecer contextualmente:

- Universal Capture;
- Command interface;
- ações;
- busca;
- voz;
- processamento da Inbox;
- sugestões relevantes.

Hermes deve existir onde sua presença reduz trabalho.

---

## 19. IA deve explicar ações

Quando Hermes interpretar uma solicitação, deve ser possível entender o que foi compreendido.

Exemplo:

> Refatorar navbar da Minarum amanhã.

O sistema poderá indicar discretamente:

```text
Task        Refatorar navbar
Project     Escadas Minarum
Date        Amanhã
```

O usuário mantém controle sobre o significado atribuído à entrada.

---

## 20. Autonomia proporcional ao risco

A inteligência deve ser agressivamente útil em ações reversíveis e conservadora em ações com consequências maiores.

Exemplo:

### Baixo risco

Salvar uma ideia.

Pode ocorrer imediatamente.

### Médio risco

Criar uma tarefa.

Pode ocorrer imediatamente com feedback.

### Maior risco

Criar Issue pública, alterar dados externos ou realizar operações destrutivas.

Pode exigir confirmação.

O objetivo é autonomia sem perda de confiança.

---

## 21. Undo antes de confirmation overload

Sempre que uma ação puder ser facilmente revertida, preferir:

> Executar → informar → permitir Undo

em vez de:

> Tem certeza?

para cada pequena operação.

Confirmações constantes tornam o sistema cansativo e ensinam o usuário a clicar sem ler.

---

## 22. Feedback imediato

Toda interação deve produzir resposta perceptível.

Ao capturar algo:

> Saved to Inbox

Ao criar tarefa:

> Task created

Ao mover item:

movimento visual claro.

Ao executar algo com Hermes:

estado perceptível da operação.

O usuário nunca deve ficar se perguntando:

> Funcionou?

---

## 23. Estados são parte do design

Todo componente relevante deve considerar:

- default;
- hover;
- focus;
- pressed;
- selected;
- loading;
- success;
- warning;
- error;
- disabled;
- empty.

Uma interface não está finalizada quando apenas seu estado ideal foi desenhado.

---

## 24. Empty states devem ensinar

Telas vazias não devem parecer quebradas.

Uma Inbox vazia pode comunicar:

> Nothing on your mind right now.

Um Project sem Tasks pode oferecer diretamente:

> Add first task

Estados vazios devem explicar silenciosamente o propósito daquele espaço.

---

## 25. Erros devem preservar confiança

Mensagens de erro devem informar:

1. o que aconteceu;
2. se alguma informação foi perdida;
3. o que o usuário pode fazer.

Evitar mensagens genéricas como:

> Something went wrong.

Sempre que houver informação útil disponível.

---

## 26. Search como infraestrutura

Busca não deve ser uma feature secundária.

Conforme o M/OS crescer, encontrar informação será tão importante quanto organizá-la.

Search deve atravessar diferentes tipos:

- Projects;
- Tasks;
- Apps;
- Resources;
- Links;
- Captures;
- Notes.

O usuário não deve precisar lembrar em qual módulo colocou algo.

---

## 27. Command interface

O M/OS deve considerar uma interface rápida de comandos.

Conceitualmente:

`Ctrl + K`

ou outro atalho apropriado.

Ela poderá permitir:

- encontrar coisas;
- abrir Apps;
- navegar;
- executar ações;
- criar itens;
- conversar com Hermes.

O objetivo é permitir que usuários experientes operem o M/OS sem depender constantemente da navegação visual.

---

## 28. Keyboard-first no desktop

O desktop deve ser extremamente eficiente com teclado.

Ações frequentes devem possuir atalhos.

Exemplos conceituais:

- Universal Capture;
- Search;
- Command interface;
- criar Task;
- navegar;
- fechar overlays;
- confirmar;
- editar.

O mouse continua plenamente suportado.

Mas o teclado deve permitir velocidade superior para quem desejar.

---

## 29. Quick Capture global

No desktop, o M/OS deverá considerar uma forma de captura que funcione mesmo quando a janela principal não está aberta.

Exemplo conceitual:

`Ctrl + Shift + Space`

↓

```text
What's on your mind?
```

Capturar.

Fechar.

Continuar o que estava fazendo.

Isso é mais importante para o conceito do produto do que possuir dezenas de telas.

---

## 30. Desktop é o centro operacional

A experiência desktop é onde o usuário deve poder:

- organizar;
- relacionar;
- pesquisar profundamente;
- trabalhar com Kanban;
- administrar Projects;
- gerenciar Library;
- acessar Apps;
- usar integrações;
- utilizar Hermes;
- compreender o sistema.

O desktop pode apresentar maior densidade de informação.

Sem sacrificar clareza.

---

## 31. Mobile não é desktop reduzido

O mobile deve ser projetado de acordo com seu contexto de uso.

Seu objetivo prioritário é:

**capturar, consultar e agir rapidamente.**

Não simplesmente comprimir a interface desktop para uma tela menor.

---

## 32. Mobile capture-first

As ações mais acessíveis no celular devem ser relacionadas a:

- capturar;
- voz;
- compartilhar;
- consultar Today;
- Inbox;
- lembretes;
- Hermes;
- tarefas rápidas.

A interface deve considerar situações em que o usuário:

- está andando;
- possui apenas uma mão disponível;
- está alternando entre aplicativos;
- quer gastar poucos segundos.

---

## 33. Voice-first quando fizer sentido

Voz é especialmente relevante porque pensamentos nem sempre surgem quando digitar é conveniente.

A interação deve ser:

1. iniciar;
2. falar;
3. confirmar visualmente que foi capturado;
4. continuar.

Evitar fluxos longos antes ou depois da gravação.

---

## 34. Compartilhar deve ser captura

No celular, compartilhar conteúdo para o M/OS deve ser tratado como extensão da Universal Capture.

Exemplo:

Instagram / Browser / GitHub

↓

Share

↓

M/OS

↓

Adicionar contexto opcional

↓

Saved

O sistema não deve exigir abrir manualmente o M/OS, navegar até Library e cadastrar URL.

---

## 35. Continuidade entre dispositivos

O usuário não deve sentir que possui dois M/OS diferentes.

Algo capturado no celular deve naturalmente aparecer no desktop.

Algo organizado no desktop deve estar disponível para consulta no celular.

A experiência pode ser diferente.

A informação deve ser a mesma.

---

## 36. Context over navigation

Sempre que possível, o M/OS deve usar contexto para diminuir navegação.

Se o usuário está dentro de:

**Project → Escadas Minarum**

e cria uma Task, o sistema já possui forte indício de qual Project relacionar.

Não deveria perguntar novamente aquilo que já sabe.

---

## 37. Preserve context

Ao navegar entre informações, o usuário não deve perder desnecessariamente seu estado anterior.

Exemplos:

- filtros;
- posição;
- projeto aberto;
- seleção;
- busca;
- contexto de trabalho.

O M/OS deve evitar a sensação de precisar “se localizar novamente” o tempo inteiro.

---

## 38. Context switching deve ser barato

Workspaces existem parcialmente para ajudar na mudança de contexto.

Ao entrar em:

**Web Design**

o usuário deve encontrar rapidamente:

- Projects relevantes;
- Tasks;
- Apps;
- Resources;
- atividade recente.

Isso reduz o custo mental de perguntar:

> Onde estavam as coisas que uso para trabalhar nisso?

---

## 39. Workspaces não podem virar silos

Apesar de fornecerem contexto, Workspaces não devem prender informação.

Search deve atravessar o sistema.

Um Resource pode ser relevante em diferentes contextos.

Uma Task pode aparecer em múltiplas visualizações.

O M/OS é um cérebro conectado, não uma coleção de pastas isoladas.

---

## 40. Relações devem ser visíveis sem serem barulhentas

O usuário deve conseguir perceber relações importantes.

Exemplo:

```text
Refatorar Navbar

Escadas Minarum
GitHub #42
Tomorrow
```

Sem precisar abrir uma visualização complexa de grafo.

Relações são poderosas.

Sua representação não precisa ser complicada.

---

## 41. Navigation architecture deve permanecer previsível

O usuário precisa desenvolver memória espacial do M/OS.

Elementos principais não devem mudar constantemente de lugar.

A navegação precisa ser:

- previsível;
- estável;
- consistente.

Novas features não devem constantemente reorganizar toda a estrutura.

---

## 42. Recency é um sinal importante

Como o M/OS acompanha trabalho diário, itens recentes possuem alto valor contextual.

A interface poderá considerar:

- Projects recentes;
- Apps recentes;
- Resources recentes;
- Captures recentes;
- atividade recente.

Isso reduz necessidade de busca para coisas que o usuário acabou de utilizar.

---

## 43. Favorites devem ser permitidos

Recência não substitui preferência.

Apps, Projects ou Resources muito utilizados podem possuir mecanismos simples de acesso persistente.

O sistema deve equilibrar:

- recente;
- favorito;
- relevante.

---

## 44. Densidade deve ser deliberada

M/OS não precisa escolher entre:

> interface extremamente vazia

e

> dashboard abarrotado.

Densidade deve variar conforme a tarefa.

Home:

densidade moderada.

Kanban:

maior densidade.

Quick Capture:

densidade mínima.

Project overview:

densidade contextual.

Library:

densidade voltada à exploração.

---

## 45. Typography is interface

Tipografia deve possuir papel estrutural.

Ela deve comunicar:

- hierarquia;
- contexto;
- estados;
- metadata;
- ação.

Não utilizar dez tamanhos e pesos diferentes sem necessidade.

Um sistema tipográfico consistente pode substituir diversos elementos gráficos.

---

## 46. Grid e spacing devem criar identidade

A identidade visual do M/OS deve ser percebida também através de:

- grid;
- alinhamentos;
- ritmo vertical;
- espaçamento;
- proporções.

Spacing não é apenas acabamento.

É parte da arquitetura da informação.

---

## 47. Ícones devem comunicar, não decorar

Utilizar ícones quando:

- aumentam reconhecimento;
- economizam espaço;
- representam ações conhecidas;
- ajudam scanning.

Evitar ícones simplesmente para tornar listas visualmente interessantes.

Quando uma ação não for universalmente compreendida, ícone sozinho pode ser insuficiente.

---

## 48. Cor deve possuir função

Cor deve principalmente comunicar:

- estado;
- prioridade;
- seleção;
- atenção;
- feedback;
- identidade pontual.

Não criar dezenas de cores para categorias arbitrárias.

Quanto mais restrito o sistema cromático, maior o significado de cada uso.

---

## 49. Motion deve explicar mudança

Animação deve possuir função.

Ela pode comunicar:

- origem;
- destino;
- mudança de estado;
- hierarquia;
- relação espacial;
- continuidade.

Exemplo:

Mover uma Task entre colunas deve comunicar claramente sua nova posição.

Abrir Quick Capture deve parecer instantâneo e relacionado ao contexto atual.

Motion não deve existir apenas para impressionar.

---

## 50. Motion precisa ser rápido

O M/OS será utilizado muitas vezes ao dia.

Animações excessivamente longas rapidamente se tornam irritantes.

A sensação desejada é:

**fluida, não cinematográfica.**

Momentos especiais podem utilizar transições mais elaboradas.

Operações repetitivas devem priorizar velocidade.

---

## 51. Performance percebida faz parte da UX

O M/OS deve parecer responsivo.

Sempre que possível:

- responder imediatamente;
- mostrar estados intermediários;
- utilizar otimistic UI quando seguro;
- evitar bloqueios desnecessários;
- preservar conteúdo enquanto algo carrega.

Uma bela interface lenta contradiz o conceito do produto.

---

## 52. Offline e falhas de conexão devem ser consideradas

Como o M/OS pretende funcionar como memória externa, confiança é crítica.

Quando possível, problemas de rede não devem causar sensação de:

> Minha ideia sumiu?

Capturas são especialmente sensíveis.

A experiência deve priorizar preservação da informação.

---

## 53. Trust is a feature

O usuário precisa desenvolver confiança de que:

> Se coloquei no M/OS, está lá.

Isso exige:

- persistência confiável;
- feedback;
- recuperação;
- previsibilidade;
- clareza sobre sincronização;
- tratamento cuidadoso de erros.

Sem confiança, o usuário voltará a manter coisas na própria cabeça.

---

## 54. Destructive actions devem ser inequívocas

Excluir algo permanentemente deve parecer diferente de:

- arquivar;
- concluir;
- remover relação;
- esconder.

O produto deve evitar consequências inesperadas.

Quando apropriado, preferir:

- Archive;
- Trash;
- Undo.

---

## 55. Accessibility não é acabamento

A interface deve ser projetada considerando:

- contraste;
- navegação por teclado;
- foco visível;
- tamanho de targets;
- semântica;
- leitura;
- redução de movimento;
- estados compreensíveis sem depender apenas de cor.

Uma interface tecnicamente sofisticada que possui baixa acessibilidade não está bem resolvida.

---

## 56. Touch targets no mobile

Elementos utilizados frequentemente no celular devem possuir áreas confortáveis para interação.

Principalmente:

- Capture;
- Voice;
- completar Task;
- abrir item;
- navegação;
- ações rápidas.

Precisão visual não pode sacrificar usabilidade física.

---

## 57. Responsive significa reconsiderar, não encolher

Layouts desktop não devem simplesmente diminuir proporcionalmente.

Em telas menores:

- prioridades mudam;
- elementos podem desaparecer;
- ações podem mudar de posição;
- navegação pode mudar;
- informação pode ser apresentada progressivamente.

Responsive design deve preservar intenção, não geometria.

---

## 58. Data belongs to the user

O M/OS armazena pensamentos, projetos, links e informações pessoais.

A interface deve reforçar sensação de propriedade.

O usuário deve conseguir compreender:

- o que está salvo;
- onde está;
- quando foi criado;
- quando foi alterado;
- relações importantes.

O sistema não deve parecer uma caixa-preta.

---

## 59. Inteligência não pode esconder informação

Automação pode organizar dados.

Ela não deve tornar impossível entender onde foram parar.

Se Hermes classificar algo automaticamente, o usuário deve conseguir encontrar e alterar essa classificação.

---

## 60. Defaults fortes, configuração opcional

M/OS deve funcionar bem sem exigir uma sessão inicial de configuração.

O produto deve possuir bons defaults.

Customização pode existir posteriormente para usuários que realmente precisam.

Evitar transformar flexibilidade em obrigação de configurar tudo.

---

## 61. Personalização deve aumentar utilidade

O fato de M/OS ser pessoal permite customização.

Mas personalização deve responder a necessidades reais.

Exemplos potencialmente úteis:

- favoritos;
- atalhos;
- ordenação;
- Workspaces;
- visualizações preferidas.

Evitar customização puramente cosmética como substituto para uma identidade visual bem definida.

---

## 62. Consistency over novelty

Quando um padrão já existe, reutilizá-lo.

Novos padrões de interação só devem ser criados quando resolvem algo que os existentes não resolvem.

Uma aplicação grande precisa de consistência mais do que precisa de interfaces diferentes em cada página.

---

## 63. Familiar where useful, distinctive where valuable

O M/OS não precisa reinventar:

- fechar;
- voltar;
- buscar;
- selecionar;
- editar;
- arrastar;
- abrir menu.

Padrões conhecidos reduzem aprendizagem.

A originalidade deve aparecer onde realmente melhora a experiência:

- Universal Capture;
- relação entre informações;
- Hermes;
- Workspaces;
- App ecosystem;
- navegação contextual.

---

## 64. No dead ends

Sempre que possível, telas devem permitir continuar uma ação relevante.

Project sem Tasks:

→ criar Task.

Busca sem resultado:

→ criar ou capturar.

Library vazia:

→ adicionar Resource.

Inbox vazia:

→ Capture.

O usuário não deve chegar frequentemente a superfícies sem próxima ação possível.

---

## 65. Reduce repetitive work

Se o usuário repetir constantemente uma sequência, isso pode indicar oportunidade de melhoria.

Exemplo:

1. criar Task;
2. selecionar sempre o mesmo Project;
3. criar Reminder;
4. abrir GitHub;
5. criar Issue.

No futuro isso pode virar uma única intenção interpretada pelo M/OS.

A automação deve nascer da repetição real.

Não da vontade de automatizar tudo.

---

## 66. Features devem justificar presença

Toda nova funcionalidade deve responder pelo menos uma destas perguntas:

### Ela reduz algo que preciso lembrar?

### Ela facilita encontrar algo?

### Ela conecta informações hoje separadas?

### Ela reduz etapas para executar algo?

### Ela melhora significativamente compreensão ou confiança?

Se não responder nenhuma, provavelmente não pertence ao Core.

---

## 67. Beauty is functional

No M/OS, estética não é superficial.

Uma interface bem resolvida:

- reduz ruído;
- melhora hierarquia;
- aumenta prazer de uso;
- facilita leitura;
- aumenta confiança;
- incentiva continuidade.

Como será um software utilizado constantemente, qualidade visual influencia diretamente sua adoção.

---

## 68. Design quality bar

Nenhuma tela deve ser considerada final apenas porque:

> funciona.

Para ser considerada resolvida, deve também possuir:

- hierarquia clara;
- spacing consistente;
- estados completos;
- boa navegação;
- comportamento responsivo;
- acessibilidade;
- feedback;
- empty states;
- tratamento de erro;
- coerência com o sistema visual;
- ausência de complexidade desnecessária.

---

## 69. Critério de simplicidade

Simplicidade não significa remover funcionalidades.

Significa reduzir a quantidade de conceitos que o usuário precisa processar simultaneamente.

Uma feature complexa pode possuir UX simples se o sistema fizer corretamente o trabalho de abstração.

---

## 70. Critério para interfaces novas

Antes de criar uma nova página, modal ou painel, perguntar:

> Essa informação realmente precisa de uma nova superfície?

Talvez possa existir:

- dentro do contexto atual;
- através da Search;
- via Command;
- em um drawer;
- como detalhe progressivo.

Evitar crescimento infinito da navegação.

---

## 71. Critério para modais

Modais devem ser utilizados quando existe necessidade real de interromper temporariamente o contexto.

Não utilizar modal automaticamente para toda criação ou edição.

Quick Capture, edição inline, command interfaces ou superfícies dedicadas podem ser melhores dependendo da situação.

---

## 72. Critério para sidebars

Sidebar é uma ferramenta de navegação.

Não deve se tornar depósito de todas as features existentes.

Itens principais precisam permanecer poucos e estáveis.

Features secundárias podem ser acessadas por:

- Search;
- Command;
- contexto;
- menus;
- Workspaces.

---

## 73. Critério para cards

Antes de criar um card, perguntar:

> Por que essa informação precisa de um container próprio?

Se a resposta for apenas:

> porque fica bonito

provavelmente não precisa.

---

## 74. Critério para badges

Badges devem indicar informação compacta que realmente ajuda scanning.

Exemplos:

- status;
- prioridade excepcional;
- origem;
- estado externo.

Não transformar toda metadata em badge.

---

## 75. Critério para AI interactions

Antes de adicionar Hermes a uma experiência, perguntar:

> A IA reduz trabalho aqui ou apenas adiciona uma etapa diferente?

Se navegar ou clicar for mais rápido, utilizar interface tradicional.

Se interpretar linguagem, relacionar informações ou executar múltiplas operações reduzir esforço, Hermes faz sentido.

---

## 76. Critério para automação

Automação boa é:

- previsível;
- observável;
- reversível quando possível;
- contextual;
- realmente econômica em esforço.

Automação ruim é:

- surpreendente;
- invisível;
- difícil de desfazer;
- excessivamente autônoma;
- criada sem necessidade real.

---

## 77. Design system

O M/OS deve eventualmente possuir um Design System próprio.

Ele deverá definir pelo menos:

- typography;
- spacing;
- grid;
- colors;
- radius;
- borders;
- elevation;
- motion;
- iconography;
- controls;
- states;
- responsive behavior;
- accessibility rules.

O objetivo não é criar documentação por documentação.

É impedir inconsistências conforme o produto crescer.

---

## 78. Componentes não devem determinar design

Bibliotecas de componentes podem acelerar desenvolvimento.

Elas não devem definir a aparência final do produto.

Componentes externos devem ser tratados como primitives quando necessário.

A identidade do M/OS deve vir das decisões próprias do produto.

---

## 79. Visual direction

A direção visual desejada é:

**minimalista**

sem ser vazia.

**premium**

sem ser luxuosa artificialmente.

**técnica**

sem parecer ferramenta industrial.

**moderna**

sem depender de tendências passageiras.

**pessoal**

sem parecer informal.

**sofisticada**

sem perder velocidade.

---

## 80. Personalidade visual

Se o M/OS fosse descrito como um objeto físico, deveria parecer:

- preciso;
- silencioso;
- durável;
- extremamente bem acabado;
- criado para uso diário;
- sem elementos supérfluos.

Mais próximo de uma ferramenta profissional cuidadosamente projetada do que de uma landing page tentando impressionar.

---

## 81. Design should disappear

Durante o uso normal, a interface não deve constantemente chamar atenção para si própria.

O usuário deve pensar sobre:

- sua ideia;
- seu projeto;
- sua tarefa;
- sua referência.

Não sobre como utilizar o software.

Os detalhes de design podem ser percebidos quando observados.

Mas durante o trabalho, devem desaparecer em favor do conteúdo.

---

## 82. Momentos de personalidade

Uma interface silenciosa não significa ausência total de personalidade.

Momentos específicos podem possuir tratamento especial:

- primeira abertura;
- Inbox zerada;
- conclusão relevante;
- transições entre contextos;
- Hermes;
- Quick Capture.

Esses momentos devem ser raros o suficiente para manter valor.

---

## 83. Construir para uso diário

Cada decisão deve considerar que determinada interação pode ser realizada centenas ou milhares de vezes.

Uma animação bonita na primeira vez pode ser irritante na centésima.

Um clique extra pequeno pode representar centenas de interrupções ao longo do ano.

O M/OS deve melhorar com familiaridade.

---

## 84. Beginner friendly, expert fast

Uma pessoa deve conseguir compreender visualmente o sistema sem decorar comandos.

Ao mesmo tempo, conforme o usuário aprende:

- atalhos;
- command interface;
- captura global;
- busca;
- Hermes;

a velocidade operacional deve aumentar significativamente.

---

## 85. Measure UX by interruption

Um critério importante para o M/OS será:

> Quanto o sistema me interrompe para eu conseguir registrar, encontrar ou executar aquilo que quero?

Quanto menor a interrupção, melhor.

Isso é particularmente importante para Capture.

---

## 86. Measure UX by retrieval confidence

Outra medida fundamental:

> Quando lembro que salvei alguma coisa, acredito que conseguirei encontrá-la?

Se a resposta deixar de ser sim, organização, Search ou relações precisam melhorar.

---

## 87. Measure UX by cognitive residue

Depois de registrar algo, o usuário ainda continua pensando:

> Será que eu vou lembrar disso?

Se sim, o M/OS ainda não conquistou confiança suficiente.

A experiência ideal permite mentalmente encerrar aquela preocupação.

---

## 88. Measure UX by unnecessary decisions

Contabilizar decisões que o sistema exige sem necessidade.

Exemplo ruim:

Para salvar um link:

1. escolher Workspace;
2. escolher categoria;
3. escolher Project;
4. escolher tags;
5. escolher tipo;
6. confirmar.

Exemplo melhor:

1. compartilhar;
2. salvar.

Contexto adicional pode vir depois.

---

## 89. North Star Interaction

Uma das interações que melhor representa a visão do M/OS é:

O usuário está fazendo outra coisa.

Tem um pensamento.

Aciona M/OS.

Escreve ou fala.

A informação é preservada.

O usuário continua exatamente de onde parou.

Todo o processo exige apenas alguns segundos.

---

## 90. North Star Experience

A experiência de longo prazo deve chegar ao ponto em que o usuário pense:

> **Não preciso manter isso na cabeça. Está no M/OS.**

E quando precisar novamente:

> **Não preciso lembrar onde coloquei. O M/OS encontra.**

E quando quiser agir:

> **Não preciso percorrer cinco ferramentas. O M/OS sabe o contexto.**

---

## 91. Teste final para qualquer experiência

Antes de aprovar uma tela ou fluxo, perguntar:

### Clareza
Entendo imediatamente o que posso fazer?

### Fricção
Existe algum passo que poderia desaparecer?

### Contexto
O sistema está perguntando algo que já deveria saber?

### Hierarquia
Minha atenção vai primeiro para o lugar correto?

### Feedback
Consigo saber o que aconteceu?

### Confiança
Tenho medo de perder informação ou executar algo errado?

### Velocidade
Isso continuará agradável depois de centenas de usos?

### Consistência
Esse comportamento se parece com o restante do M/OS?

### Visual
A interface parece deliberadamente projetada ou apenas montada?

### Propósito
Isso reduz carga mental?

Se uma experiência falhar significativamente nesses critérios, ela ainda não está pronta.

---

## 92. Regra final

> **A melhor interface do M/OS não é aquela que mostra tudo que o sistema consegue fazer.**

> **É aquela que permite fazer o que importa com o menor esforço mental possível, sem sacrificar controle, clareza ou qualidade visual.**

O M/OS deve ser complexo por dentro para poder ser simples por fora.
