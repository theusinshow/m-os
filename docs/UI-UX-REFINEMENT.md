# M/OS UI/UX Refinement

**Status:** auditoria e direção — sem implementação de UI

**Data:** 2026-08-16

**Baseline inspecionada:** M/OS `v0.2.11`, commit `7fd0162`

**Escopo:** frontend, UX, navegação, composição, design system, interação e acabamento

## 1. Resumo executivo

O M/OS já tem uma base visual própria e mais disciplinada do que a média de produtos neste estágio. A hierarquia de superfícies dark, o sódio como sinal, a geometria seca, a tipografia compacta, o grid pontilhado e a ausência de sombras/glows gratuitos formam uma identidade reconhecível. Command, Library e Calendar mostram que essa linguagem consegue produzir superfícies densas, técnicas e calmas.

O problema principal não é falta de decoração. É **dívida de composição acumulada entre superfícies**:

- a Home mostra quase tudo com peso semelhante, em vez de orientar o momento atual;
- o rail é estável, mas atingiu um limite de capacidade e perde utilitários visualmente em janelas menores;
- Hermes quebra sua própria prioridade de conteúdo em 840 px;
- Tempo ainda parece um produto absorvido, não uma superfície nativa do M/OS;
- Settings concentra capacidades demais numa única página linear;
- Projects, Workspaces e Apps repetem master-detail com proporções diferentes e um inspector vazio dominante;
- foco, seleção e accent competem entre si em alguns componentes;
- fundamentos globais bons coexistem com overrides locais e uma folha de estilos monolítica.

### Diagnóstico em uma frase

O M/OS já tem uma linguagem; agora precisa de uma **gramática de produto** que determine o que domina, o que recua e como cada tipo de superfície se comporta.

### P0 encontrados

1. **Rail incompleto em viewport compacto:** Quick Capture e Settings continuam na árvore acessível, mas não aparecem visualmente em 1024×768 e 840×600.
2. **Hermes compacto sobrepõe o trabalho:** em 840×600 a coluna de conversas fica aberta como drawer sobre o conteúdo, cortando hero e composer.
3. **Focus styling global conflita:** duas regras globais compõem outline, border e ring; o resultado é um retângulo de sódio forte que frequentemente parece seleção.

Nenhuma regra de negócio, API, banco, backend ou contrato de domínio precisa mudar para resolver esses três itens.

## 2. Base de verdade e método

### Documentação revisada

Foram lidos os documentos de produto, arquitetura, decisões, foundations, wireframes e handoffs existentes em `/docs`, incluindo os materiais recentes de Hermes, Calendar, Home/widgets, Resources/Library, App Registry, ações entre apps e Tempo/CronoCAD. A interpretação seguiu a prioridade definida em `AGENTS.md`:

1. `VISION.md`
2. `PRODUCT.md`
3. `CORE.md`
4. `UX-PRINCIPLES.md`
5. `ROADMAP.md`
6. `IDEAS.md`

`IDEAS.md` não foi convertido em requisito. O Calendar foi avaliado como retrospectiva de eventos, conforme sua especificação atual, e não como agenda. Hermes foi tratado como camada/superfície do M/OS, sem proposta de novas capacidades.

### Inspeção realizada

O aplicativo real foi executado pela configuração dogfood e navegado por acessibilidade e inspeção visual. Foram verificados:

- App Shell e rail;
- Home;
- Hermes;
- Inbox;
- Tasks;
- Projects;
- Workspaces;
- Tempo: Painel, Projects, Histórico, Linha do tempo, Relatórios e Configurações;
- Calendar;
- Library;
- Apps, acessível pela Home;
- Settings;
- Command/Search com estado vazio e resultados;
- Quick Capture.

Também foram auditados tokens, fontes, primitives, CSS global, estados de foco, forced colors e reduced motion.

### Matriz visual coberta

| Tema | Viewport | Cobertura |
|---|---:|---|
| Dark | 840×600 | Home, Hermes, Calendar, rail |
| Dark | 1024×768 | Home, App Shell, rail |
| Dark | 1280×800 | Todas as superfícies listadas |
| Dark | 1440×900 | Home e App Shell |
| Dark | 1920×1080 | Home e App Shell |
| Light | 1280×800 | Home, Calendar, Settings e App Shell |

A rodada de implementação deverá transformar isso numa matriz repetível por superfície prioritária, com screenshots antes/depois.

## 3. Princípios de produto que limitam a direção

A direção visual não pode ferir os seguintes fundamentos do M/OS:

- captura antes de organização;
- informação antes de containers;
- progressive disclosure;
- navegação espacialmente estável;
- uma intenção dominante por superfície;
- teclado como caminho de primeira classe;
- confiança, persistência local e erros compreensíveis;
- Hermes como presença própria, não chat genérico;
- Home como orientação e retomada, não inventário de features;
- desktop-native, rápido e silencioso;
- dark e light deliberados, não invertidos automaticamente.

## 4. Current state: forças que devem ser preservadas

### Identidade

- O **sódio** é distintivo e funciona bem quando reservado para ação, posição atual e sinal de sistema.
- A geometria quase ortogonal e os raios pequenos afastam o M/OS de templates SaaS.
- O grid pontilhado é sutil e cria materialidade sem virar efeito decorativo.
- Os ícones customizados têm linguagem de stroke e alinhamento óptico coerentes.
- Schibsted Grotesk + JetBrains Mono cria uma voz técnica sem parecer terminal temático.

### Design system

- Existe uma boa escada dark: canvas → surface → raised → hover → active.
- Light mode tem tokens próprios; não é uma inversão mecânica.
- Espaçamento, radius e motion já partem de vocabulários pequenos.
- Forced colors e reduced motion estão contemplados na foundation.
- O Command tem um dos melhores selected states do produto: surface shift, indicador lateral e contraste de texto.

### Superfícies

- **Library** demonstra um master-detail útil: lista/grid compacto e inspector contextual com dados reais.
- **Calendar** usa grid como estrutura, não uma coleção de cards pesados.
- **Hermes** evita bolhas, avatares e estética de ChatGPT embedado.
- **Quick Capture** é compacto, rápido e claramente desktop.
- **Tempo/Histórico** e **Relatórios** provam que alta densidade pode caber sem inputs gigantes.

Esses elementos devem ser refinados, não substituídos por uma estética Linear genérica.

## 5. Mapa de navegação atual

```text
App Shell
├─ Rail principal
│  ├─ Home
│  ├─ Hermes
│  ├─ Inbox
│  ├─ Tasks
│  ├─ Projects
│  ├─ Workspaces
│  ├─ Tempo
│  ├─ Calendar
│  └─ Library
├─ Rail utilitário
│  ├─ Quick Capture
│  └─ Settings
├─ Top bar
│  └─ Command / Search
└─ Entradas contextuais
   └─ Apps via Home > Gerenciar

Tempo
├─ Painel
├─ Projects
├─ Histórico
├─ Linha do tempo
├─ Relatórios
└─ Configurações
```

### Leitura crítica

- O rail passou de uma navegação curta para **nove destinos + dois utilitários**, sem mecanismo visível de expansão, agrupamento ou overflow.
- Apps cumpre a decisão de sair do rail e continua alcançável, mas a rota perde sinal claro de localização quando aberta.
- Projects global e Projects de Tempo têm propósitos distintos, porém o mesmo nome e peso de navegação sugerem duplicidade.
- Workspaces é importante para o modelo contextual do produto, mas seu valor como root navigation ainda precisa ser validado por frequência de uso real; não deve ser removido por decisão apenas visual.
- Settings e Quick Capture são fundamentais demais para desaparecerem em alturas comuns.

## 6. Matriz de auditoria global

| Surface | Problem | Severity | UX Impact | Visual Impact | Recommendation | Priority |
|---|---|---|---|---|---|---|
| App Shell / Rail | Quick Capture e Settings ficam visualmente fora do rail em 1024×768 e 840×600 | Crítica | Perda de funções globais e orientação | Rail parece truncado | Tornar as zonas principal/utilitária intrinsecamente dimensionáveis; garantir overflow deliberado ou compactação | P0 |
| Global focus | Regras globais duplicadas somam outline, border e ring de sódio | Alta | Foco e seleção tornam-se semanticamente ambíguos | Retângulos amarelos dominam a UI | Definir um único contrato de focus-visible por tipo de componente | P0 |
| Hermes 840 | Histórico permanece aberto como drawer sobre conteúdo e composer | Crítica | Conteúdo e entrada ficam cortados | Superposição parece quebra de layout | Fechar por padrão no compacto; backdrop, close, focus trap e retorno de foco | P0 |
| Home | Módulos demais têm peso equivalente | Alta | O usuário precisa escanear a página inteira para saber o que importa | Dashboard longo e sem foco | Reordenar por Agora → Próximo → Retomar → Visão; secundários abaixo da primeira dobra | P1 |
| Home | Grandes gaps verticais fazem o primeiro viewport conter pouca informação | Alta | Baixa eficiência em 840/1024 | Sensação de landing page vazia | Criar densidade semântica por módulo e reduzir intervalos 32→52/84 onde não há mudança de contexto | P1 |
| Home | Copy mistura inglês e português (`What's on your mind?`) | Média | Quebra continuidade e voz | Parece trecho não finalizado | Unificar linguagem da interface e manter termos de domínio deliberados | P2 |
| Home widgets | Anatomias são parcialmente consistentes, mas não há hierarquia de tamanho/urgência | Alta | O grid não comunica prioridade | Seções parecem igualmente importantes | Definir Widget primitive e papéis S/M/L/XL baseados em conteúdo, não em dashboard grid | P1 |
| Sidebar | Rail icon-only exige memória/tooltips para nove destinos | Alta | Descoberta e recall degradam com crescimento | Coluna vira sequência homogênea | Adotar rail expansível/contextual e agrupamento no estado expandido | P1 |
| Sidebar | Active e focus usam sódio de forma concorrente | Alta | Estado atual e navegação por teclado se confundem | Accent em excesso no item | Active = surface + posição; focus = halo/outline neutro-sódio discreto e independente | P1 |
| Top bar | Command é claro, mas a data/meta compete pouco e o restante do header varia por página | Média | Orientação depende do breadcrumb local | Chrome parece desconectado das páginas | Padronizar location bar + view bar apenas quando a superfície precisar | P2 |
| Command | Campo mostra spellcheck nativo vermelho em consultas | Média | Ruído em busca de nomes/comandos | Quebra acabamento premium | Desabilitar spellcheck/autocorrect no campo de Command | P2 |
| Command | Regra de sódio sob o input é mais forte que o resultado selecionado | Média | Hierarquia da lista perde força | Accent parece decorativo | Reduzir o divider e preservar o sódio para seleção/ação | P2 |
| Inbox | Empty state é silencioso, mas não oferece próximo passo | Média | Usuário termina num beco sem ação | Grande vazio sem intenção | Mensagem curta + uma ação contextual de captura; sem ilustração | P2 |
| Tasks | Seis colunas vazias repetem `Nenhuma Task` | Alta | O vazio domina e não ensina o fluxo | Board parece estrutura sem produto | Um empty state de board e colunas silenciosas; manter criação única | P1 |
| Tasks | Board horizontal não explicita overflow/navegação | Média | Últimas colunas parecem cortadas em 1280 | Composição incompleta | Indicar continuidade, suportar teclado e reservar detalhes para Inspector | P2 |
| Projects | Inspector vazio consome a maior parte da página | Alta | A ação de começar fica confinada à esquerda | Muito vazio sem propósito | Empty state compartilhado; abrir inspector só após seleção ou equilibrar pane inicial | P1 |
| Workspaces | Mesmo padrão de Projects usa proporção diferente | Alta | Aprendizado não transfere entre superfícies | Arquitetura de página inconsistente | Um contrato único de master-detail e breakpoint | P1 |
| Apps | Rota contextual não tem active/location state claro no rail | Média | Usuário perde relação com o ponto de entrada | Shell parece sem destino ativo | Breadcrumb/retorno contextual ou location state; não recolocar no rail automaticamente | P2 |
| Library | Controles de filtro/view estão excessivamente comprimidos e sem grupos | Média | Skimming e mudança de view exigem precisão | Toolbar parece uma linha de labels | Separar filtro, busca, view e ação em view bar compacta | P2 |
| Library | Cards truncam títulos cedo demais | Média | Reconhecimento do recurso piora | Grid parece micro demais | Ajustar largura mínima/densidade ou priorizar row view em larguras compactas | P2 |
| Tempo | Tipografia 28/700, tabs outline e cardização divergem do shell | Alta | Troca de app parece troca de produto | Módulo absorvido continua visualmente externo | Reaplicar foundations e page anatomy do M/OS sem alterar funções | P1 |
| Tempo | Banner de importação se repete em todas as subviews | Alta | Cada tarefa começa com uma mensagem secundária | Chrome domina conteúdo | Tornar aviso contextual, colapsável ou restrito ao ponto de decisão | P1 |
| Tempo Projects | Nome concorre com Projects global | Alta | Modelo mental de Project fica ambíguo | Duas hierarquias paralelas | Decision gate: renomear como visão/metadata de Tempo ou integrar como subview contextual | P1 |
| Calendar | Detalhe do dia abre abaixo das seis semanas | Alta | Clique pode parecer não ter efeito sem scroll | Feedback está fora do viewport | Usar Inspector lateral/ancorado ou região visível; preservar domínio retrospectivo | P1 |
| Calendar | Navegação de mês tem affordance baixa | Média | Alvo/ação pouco evidente | Controles parecem metadata | Icon buttons compactos com tooltip e estados consistentes | P2 |
| Hermes | Erro bruto expõe inglês e URL de backend | Alta | Usuário recebe diagnóstico técnico sem orientação | Rodapé longo e ruidoso | Traduzir em estado humano + detalhe técnico progressivo | P1 |
| Hermes | Empty hero é expressivo, mas grande e fragmentado | Média | Composer demora a dominar a leitura | Vazio compete com a ação | Reduzir escala e aproximar exemplos/contexto do composer | P2 |
| Settings | Uma página contém 330 nós e categorias heterogêneas | Alta | Localização e retorno são difíceis | Scroll longo, sem visão de estrutura | Navegação por categorias e seção Advanced/Functions; preservar todas as capacidades | P1 |
| Settings | Erros técnicos e HotKey bruto aparecem inline | Alta | Baixa confiança e pouca ação de recuperação | Copy parece log de desenvolvimento | Estados de erro consistentes, orientados a resolução, com detalhe expansível | P1 |
| Light mode | Home fica lavada; separadores e níveis de superfície são próximos demais | Média | Escaneabilidade cai em sessões longas | Canvas vira plano único | Ajustar borders/surface contrast especificamente no light | P2 |
| Empty/loading/error | Empty states existem; loading/error/saving não têm uma linguagem única visível | Alta | Estado do sistema varia por módulo | Produto parece menos coeso sob latência/erro | Inventariar e padronizar state primitives antes do motion pass | P1 |
| CSS system | `App.css` concentra mais de 5 mil linhas e muitos blocos locais | Alta | Correções globais podem regressar superfícies | Drift como Tempo/Hermes se acumula | Extrair somente padrões comprovados durante lotes visuais | P1 |
| Accessibility | Semântica geral é boa, mas focus excessivo e drawer compacto exigem correção | Alta | Teclado e leitor podem perder contexto | Estados inconsistentes | Focus contract, modal semantics, hit targets e teste só teclado por lote | P0/P1 |

### Definição de prioridade

- **P0:** impede acesso, leitura ou operação confiável em um viewport suportado; correção imediata.
- **P1:** degrada tarefa central, arquitetura visual ou consistência global; entra nos primeiros lotes.
- **P2:** paper cut relevante, acabamento ou coerência localizada.
- **P3:** refinamento oportunista, somente após foundations e fluxos principais.

## 7. Auditoria do design system

### 7.1 Cores e superfícies

A direção dark atual é forte:

```text
Canvas   #0A0C0E
Surface  #101316
Raised   #171B1F
Hover    #1E2429
Active   #252C31
```

O produto já constrói profundidade por luminância e hairlines, não por sombras grandes. Isso deve ser preservado.

Contraste medido nos tokens dark:

| Texto | Canvas | Surface | Raised | Leitura |
|---|---:|---:|---:|---|
| Primary | 16.21:1 | 15.42:1 | 14.33:1 | Muito alto; usar com parcimônia |
| Secondary | 6.36:1 | 6.05:1 | 5.62:1 | Bom para suporte |
| System | 5.22:1 | 4.97:1 | 4.61:1 | Válido, mas marginal em tamanhos micro |
| Disabled | 3.50:1 | 3.33:1 | 3.09:1 | Aceitável somente para indisponível/não essencial |
| Placeholder | 5.11:1 | 4.86:1 | 4.51:1 | Adequado, no limite em raised |

O problema não é falta de contraste global. É a aplicação de texto tertiary/system em tamanho pequeno demais e o uso de primary em áreas grandes. A correção deve ser semântica, não “clarear tudo”.

No light mode, primary e secondary passam contraste, mas as relações entre canvas, seção e borda são menos perceptíveis. O refinamento deve elevar estrutura local sem transformar a interface em uma grade cinza.

### 7.2 Sódio

Regra proposta:

- usar para ação primária rara, caret, posição atual, dado temporal selecionado e sinais críticos de sistema;
- não usar simultaneamente como active, focus, border de tab, divider e preenchimento na mesma região;
- selected não deve depender só dele;
- hover comum deve preferir surface/foreground shift.

### 7.3 Tipografia

Estado atual:

- Schibsted Grotesk: assets 400/500/700;
- JetBrains Mono: assets 400/500;
- tokens incluem display 48/700, title 28/600, body 16/400, UI 14/500, meta 12/500 e micro 11/500;
- Tempo adiciona heading local com peso 700.

Problemas:

- 700 carrega peso de marca/marketing em superfícies de software;
- o token 600 não corresponde diretamente aos pesos estáticos disponíveis e precisa de validação de rasterização;
- 28 px funciona no Calendar, mas se torna excessivo quando combinado com banner e tabs fortes no Tempo;
- meta/micro em low contrast é elegante no screenshot, porém pode cansar em uso prolongado.

Direção:

- UI de produto entre 400 e 500 como regra;
- 600 apenas quando o asset e a função justificarem;
- 700 reservado a números/dados de alta importância ou marca, nunca como padrão de títulos;
- hierarquia primeiro por posição, size, contrast e spacing;
- numeric UI e identificadores em mono com tabular numbers;
- não introduzir fonte externa nesta rodada.

### 7.4 Spacing e densidade

O sistema é disciplinado, porém os saltos 32 → 52 → 84 produzem vazios de mudança de capítulo em locais que são apenas módulos adjacentes. A Home evidencia isso.

Proposta conceitual:

```text
4   micro relação
8   relação interna compacta
12  controles e rows
16  grupos pequenos
20  seção interna
24  gutters/headers densos
32  mudança clara de seção
40+ somente mudança de capítulo ou empty composition
```

Não é necessário apagar tokens existentes. É necessário dar-lhes **papéis semânticos** e retirar usos de landing page nas superfícies desktop.

### 7.5 Radius, border e elevation

- O vocabulário de radius 2/3/8 é pequeno e próprio. Não deve ser inflado para 10/12 apenas por referência externa.
- Hairlines funcionam bem no dark.
- Calendar justifica grid completo; Home não precisa de caixas adicionais.
- Elevation deve permanecer reservada a Command, popovers, menus, drawers e inspectors destacados.

### 7.6 Motion

Os tokens existentes cobrem aproximadamente 75–220 ms, com um primeiro reveal de 400 ms, easings de entrada/saída/estado e reduced motion. A foundation é suficiente.

O problema atual é menos “falta de animação” e mais ausência de contratos visíveis para:

- troca de página;
- drawer do Hermes;
- abertura de Inspector;
- loading → content;
- saving → saved;
- seleção de dia;
- mudanças de widget.

Direção:

| Token semântico | Faixa | Uso |
|---|---:|---|
| Instant | 75–100 ms | pressed, hover, check |
| Fast | 120–160 ms | menu, tooltip, tab |
| Normal | 180–220 ms | inspector, drawer, dialog |
| Slow | 280–400 ms | primeiro reveal raro; nunca navegação rotineira |

Sem bounce contínuo, loops em idle ou page transitions “voando”.

### 7.7 Primitives

Já existem Button, IconButton/ícones, Input patterns, Panel, Card, Stat, EmptyState e PageHeader. O próximo passo não é criar uma biblioteca paralela. É consolidar padrões que já aparecem repetidamente:

- `Surface`
- `PageHeader` com variantes Focus / Canvas / MasterDetail
- `ViewBar`
- `ListRow`
- `Widget`
- `Inspector`
- `StateMessage` para empty/loading/error/saving
- `FocusRing`/contrato CSS, não necessariamente componente
- `Menu`, `Popover`, `Tooltip` com motion e keyboard comuns

`Card` não deve continuar como container default, e nomes herdados de módulos absorvidos não devem vazar para primitives globais.

## 8. Auditoria de estados e interação

| Estado | Current state | Direção |
|---|---|---|
| Hover | Geralmente usa surface shift; bom | Manter; revelar ações secundárias sem deslocar cards |
| Pressed | Pouco distinguível em alguns icon buttons | Escurecimento/inner contrast de 75–100 ms |
| Focus | Forte e duplicado | Um ring discreto, visível e semanticamente diferente de selected |
| Selected | Excelente no Command; excessivo no rail/Tempo | Surface + posição + texto; accent como indicador secundário |
| Active | Confunde-se com focus em tabs/rail | Reservar a estado persistente de navegação |
| Disabled | Visualmente recuado | Garantir que contrast baixo não seja reutilizado em ações disponíveis |
| Empty | Silencioso, mas às vezes sem ação | Statement curto + explicação opcional + uma ação contextual |
| Loading | Não há linguagem transversal clara | Skeleton/placeholder somente quando preserva layout; sem spinners grandes |
| Error | Alguns estados exibem erro bruto | Mensagem humana, recuperação, detalhe técnico progressivo |
| Saving/Saved | Pouco consistente entre surfaces | Status inline silencioso, sem toast para cada sucesso |
| Dragging | Board necessita validação dedicada | Elevação mínima, ghost e alvo claro; reduced motion respeitado |

## 9. Auditoria da sidebar: três direções

### A. Traditional compact sidebar

Uma sidebar de 196–208 px com labels e grupos sempre visíveis.

| Critério | Avaliação |
|---|---|
| Clareza | Alta |
| Velocidade | Alta para descoberta; média para conteúdo |
| Densidade | Média |
| Escalabilidade | Alta com grupos |
| Identidade M/OS | Média; risco de parecer ferramenta de equipe genérica |

**Vantagem:** resolve imediatamente labels, agrupamento e utilitários.

**Risco:** compete com o conteúdo e abandona a espacialidade compacta já aprendida.

### B. Rail + sidebar expansível/contextual — recomendada

O rail de 52 px permanece como âncora. Pode expandir para 196–208 px sob comando/hover deliberado, exibindo labels e grupos. Superfícies que precisam de navegação secundária podem usar uma pane contextual; superfícies focadas continuam somente com rail.

| Critério | Avaliação |
|---|---|
| Clareza | Alta quando expandida; média no rail |
| Velocidade | Alta após aprendizado |
| Densidade | Alta |
| Escalabilidade | Alta, se overflow e grupos forem explícitos |
| Identidade M/OS | Alta |

**Vantagem:** preserva a identidade atual, permite o conteúdo dominar e acomoda crescimento.

**Risco:** exige contratos excelentes de expansão, tooltips, teclado, persistência e breakpoint.

### C. Contextual/personalized sidebar

Destinos mudam conforme Workspace, contexto ou frequência pessoal.

| Critério | Avaliação |
|---|---|
| Clareza | Baixa a média |
| Velocidade | Potencialmente alta |
| Densidade | Alta |
| Escalabilidade | Alta |
| Identidade M/OS | Muito alta |

**Vantagem:** explora a natureza pessoal/contextual do M/OS.

**Risco:** enfraquece previsibilidade e memória espacial; cedo demais como base global.

### Recomendação

Adotar **B** como arquitetura de interação e manter **C** apenas como evolução futura testável. No primeiro lote, nenhuma rota deve ser removida ou rebaixada silenciosamente. A sequência segura é:

1. corrigir fit vertical e utilitários;
2. separar active de focus;
3. implementar expansão/labels sem alterar destinations;
4. coletar uso real;
5. decidir IA com evidência.

### Hipótese de agrupamento — não é requisito aprovado

```text
AGORA
Home · Inbox · Tasks

TRABALHO
Projects · Calendar · Tempo

CONTEXTO E MEMÓRIA
Workspaces · Library · Hermes

UTILITÁRIOS
Quick Capture · Settings
```

Essa hipótese deve ser testada. Em especial:

- Hermes pode merecer posição mais alta do que “memória” sugere;
- Workspaces pode funcionar melhor como context switcher;
- Tempo Projects pode ser uma subview e não uma raiz conceitual;
- Apps deve continuar acessível sem voltar ao rail automaticamente.

Essas são decisões de arquitetura de informação com impacto de produto e exigem aprovação explícita.

## 10. Page architecture

Consistência não significa um template único. O M/OS precisa de cinco famílias:

### Focus page

**Uso:** Home, Quick Capture.

**Anatomia:** intenção dominante → contexto imediato → conteúdo secundário progressivo.

### Canvas

**Uso:** Tasks, Calendar.

**Anatomia:** location/header compacto → view controls → canvas que usa largura → Inspector contextual.

### Master-detail

**Uso:** Library, Projects, Workspaces, Apps.

**Anatomia:** list pane consistente → divider → Inspector. Sem seleção, o vazio deve pertencer à composição inteira; o inspector não pode dominar como bloco morto.

### Workspace

**Uso:** Hermes, Tempo.

**Anatomia:** navegação local opcional → área de trabalho dominante → composer/timeline/data. Breakpoints sacrificam primeiro chrome secundário.

### Settings

**Uso:** Settings global e configurações densas do Tempo.

**Anatomia:** categorias → seção focal → ajuda/estado contextual. Functions e diagnósticos entram em Advanced, sem serem removidos.

### Inspector contract

Direção inicial:

- largura padrão 360 px; faixa útil aproximada 320–420 px;
- entra em 180–220 ms com opacity + translate de 4–8 px;
- fecha com `Esc`, botão explícito e restauração de foco;
- selection permanece visível por surface/indicator, não só accent;
- em 840–1024 pode virar sheet/drawer, nunca reduzir o canvas a uma coluna inútil;
- não abre vazio por padrão quando a lista ainda não tem item.

## 11. Home e sistema de widgets

### Trabalho da Home

1. capturar;
2. dizer onde estou;
3. mostrar o que exige atenção agora;
4. permitir retomar;
5. revelar visão mais ampla somente depois.

A Home atual executa 1 e contém dados para 2–5, mas os apresenta quase no mesmo nível. O resultado é um dashboard sem cards pesados, porém ainda um dashboard em escopo.

### Hierarquia proposta usando capacidades existentes

```text
CAPTURAR
Universal Capture

AGORA
Current Context · Em andamento · Hoje

ATENÇÃO / RETOMADA
Inbox · Recentes · Projects ativos

VISÃO
Semana · Mês · Concluído · Sistema

ACESSO CONTEXTUAL
Apps · Resources · ações secundárias
```

“Next Up”, Focus, Deadlines ou outros conceitos só entram quando houver semântica de produto definida; não devem ser inventados por este refinamento visual.

### Widget philosophy

- Widget é uma unidade de decisão, não um card.
- Header tradicional é opcional.
- Toda unidade precisa de pergunta respondida: “o que eu sei ou faço aqui?”
- S/M/L/XL descreve necessidade de conteúdo, não colunas Bootstrap.
- Widgets de estado vazio podem colapsar ou reduzir altura.
- Dados temporais podem usar linha, ring, grid ou row; não precisam virar cards.
- Motion ocorre apenas quando o dado/estado muda.

### Anatomia

```text
eyebrow/category opcional
title ou dado primário
conteúdo principal
metadata essencial
ação contextual/revelada
status/footer somente quando necessário
```

## 12. Direção visual — M/OS UI Direction vNext

### Tese: instrumento pessoal silencioso

O M/OS deve parecer um instrumento que já conhece o contexto do usuário: preciso sem ser frio, pessoal sem ser decorativo, denso sem parecer painel administrativo.

### Princípios

1. **Conteúdo conquista peso; chrome não recebe peso por padrão.**
2. **Estrutura é percebida por alinhamento, ritmo e pequenas diferenças de superfície.**
3. **Sódio sinaliza intenção ou estado, não luxo.**
4. **Densidade vem de relações claras, não de encolher tudo.**
5. **Ações aparecem onde o contexto existe.**
6. **Estados persistentes e transitórios têm linguagens distintas.**
7. **A Home é pessoal; superfícies de trabalho são operacionais.**
8. **O shell é estável, mas recua depois da navegação.**
9. **Motion confirma mudança; nunca compensa hierarquia ruim.**
10. **Light e dark compartilham semântica, não valores espelhados.**

### Resultado esperado

- mais informação útil no primeiro viewport;
- menos headings e banners competindo;
- menos inspectors vazios;
- rows melhores, cards mais raros;
- sidebar reconhecível quando necessária e silenciosa durante o trabalho;
- states previsíveis em todas as surfaces;
- Tempo visualmente integrado;
- Hermes íntegro em qualquer largura suportada;
- Calendar com seleção e detalhe imediatamente perceptíveis.

## 13. Tradução das referências Linear e Refero

### O que foi estudado

- [Refero — Linear style extraction](https://styles.refero.design/style/90ce5883-bb24-4466-93f7-801cd617b0d1)
- [Linear — How we redesigned the UI](https://linear.app/now/how-we-redesigned-the-linear-ui)
- [Linear — A calmer interface for a product in motion](https://linear.app/now/behind-the-latest-design-refresh)
- [Linear — Personalized sidebar and new settings](https://linear.app/changelog/2024-12-18-personalized-sidebar)
- [Linear — Contextual command menu](https://linear.app/changelog/2019-10-07-contextual-command-menu)
- [Linear Method — Principles & Practices](https://linear.app/method/introduction)

Refero é uma extração/síntese visual, não documentação oficial da Linear. Valores exatos de paleta, padding e radius foram tratados como observação, não como verdade a copiar.

### Princípios traduzidos

| Referência | Tradução para M/OS |
|---|---|
| Chrome secundário recua | Rail/sidebar mais dim quando o conteúdo está ativo |
| Headers previsíveis | Famílias de PageHeader e ViewBar, não um template universal |
| Densidade com hierarquia | Rows compactas, metadata recuada, ação no contexto |
| Estrutura sem ruído | Hairlines e surface shifts apenas onde explicam relação |
| Command contextual | Search global separado de Hermes; ações priorizadas pelo contexto |
| Sidebar personalizável | Primeiro expansão/overflow estáveis; personalização só após evidência |
| Dogfood incremental | Lotes pequenos com screenshot matrix e feature comparison |

### O que não será copiado

- paleta roxa/azul, lime/electric accents ou qualquer “tema Linear”;
- radius, fontes ou iconografia da Linear;
- arquitetura de times/workspaces de um produto colaborativo;
- sidebar com grupos adotada sem validar a IA pessoal do M/OS;
- spacing de marketing do Refero;
- efeitos de spotlight/glow como linguagem de premium.

## 14. Plano priorizado

### Lote 0 — Foundation + App Shell + Sidebar

**Objetivo:** corrigir os P0 globais e estabelecer a gramática antes de tocar páginas.

Escopo:

- unificar focus-visible;
- garantir rail completo em 840×600 e 1024×768;
- prototipar estados collapsed/expanded sem alterar rotas;
- definir active/selected/focus/hover/pressed;
- calibrar typography weights e page title usage;
- definir surface/text/border roles em dark e light;
- consolidar Button, IconButton, Input, Tooltip e ViewBar contracts;
- documentar motion e z-index contracts;
- desabilitar spellcheck no Command.

Critérios de aceite:

- todos os onze controles globais acessíveis e visíveis nos viewports suportados;
- operação completa por teclado;
- focus nunca confundido com current/selected;
- dark/light aprovados em 840, 1024, 1280, 1440 e 1920;
- nenhum contrato funcional, rota ou domínio alterado;
- screenshots antes/depois e visual QA real.

### Lote 1 — App Shell avançado + Command + Inspector contract

- page header families;
- location/view bar;
- rail expandido e grupos validados;
- Command refinado e contextual sem confundir com Hermes;
- Inspector primitive aplicado primeiro a uma surface piloto;
- popover/menu/tooltip motion e keyboard.

### Lote 2 — Home + Widgets

- reorganizar primeira dobra;
- implementar Widget primitive por papéis reais;
- reduzir módulos vazios e gaps;
- consolidar Current Context, Em andamento, Hoje e Inbox;
- manter analytics/visão após a tarefa imediata;
- validar 840→ultrawide sem dashboard grid.

### Lote 3 — Hermes

- corrigir drawer compacto;
- recalibrar empty composition e composer;
- normalizar connection/error/streaming/tool states;
- revisar history density, message width e citations/artifacts existentes;
- testar teclado, focus return e reduced motion.

### Lote 4 — Product surfaces operacionais

Ordem sugerida:

1. Inbox;
2. Tasks;
3. Projects/Workspaces/Apps como família master-detail;
4. Library;
5. Calendar.

O Calendar deve receber Inspector/detalhe visível; Tasks deve pilotar rows/cards compactos e seleção; Library serve como referência positiva de inspector contextual.

### Lote 5 — Tempo

- remover divergência tipográfica e tabs outline;
- reduzir cardização e banners repetidos;
- harmonizar filtros, table/rows e ações;
- separar visualmente o conceito de Projects global da visão de faturamento/tempo;
- preservar integralmente regras e dados do módulo.

### Lote 6 — Settings

- arquitetura por categorias;
- Advanced/Functions;
- estados de erro humanos com detalhe técnico;
- forms e destructive states consistentes;
- deep-link/Command para categorias apenas se já couber nos contratos existentes ou mediante decisão explícita.

### Lote 7 — Motion, accessibility e performance pass

- page/inspector/drawer/dialog/popover transitions;
- saving/saved/error/loading;
- keyboard traversal e screen reader labels;
- reduced motion e forced colors;
- hit targets;
- regressão visual e de performance.

## 15. Decision gates antes de mudanças de IA

As seguintes propostas não são autorizadas como simples refino visual:

1. remover Workspaces do rail ou transformá-lo em switcher;
2. rebaixar Tempo Projects para subview/contexto;
3. alterar a forma de acesso a Apps;
4. criar Next Up, Focus, Deadlines ou novos widgets sem domínio existente;
5. mudar Calendar de retrospectiva para agenda;
6. fundir Search e Hermes;
7. adicionar ações novas a empty states que não existam no produto.

Se algum protótipo mostrar ganho claro, a mudança deve ser documentada como decisão de produto/IA e aprovada antes da implementação.

## 16. Visual QA por lote

### Checklist mínimo

- build e typecheck;
- navegação real por todas as surfaces tocadas;
- mouse e teclado;
- hover, pressed, focus, selected, active e disabled;
- empty, loading, error e success disponíveis;
- dark: 840, 1024, 1280, 1440, 1920;
- light: 1280 e pelo menos um viewport compacto;
- inspector/drawer/modal com `Esc` e retorno de foco;
- reduced motion;
- forced colors quando o componente usar cor como sinal;
- comparação com a direção, não com screenshot da Linear;
- checagem de overflow, truncation e zoom/text scaling;
- nenhuma animação permanente ou blur amplo.

### Regra de refinamento

Um lote só termina quando a aplicação executada parece melhor em contexto. Build verde é necessário, mas não é evidência visual.

## 17. Próxima ação recomendada

Não implementar uma redesign ampla. O próximo passo é um protótipo pequeno e reversível do **Lote 0 — Foundation + App Shell + Sidebar**, com duas comparações visuais:

1. rail atual corrigido + estados de foco unificados;
2. rail corrigido + expansão compacta com labels/grupos, preservando todas as rotas.

A opção vencedora deve ser validada em 840×600, 1024×768, 1280×800, 1440×900 e 1920×1080 antes de seguir para Home. Isso resolve risco real, muda a sensação global e evita redesenhar páginas sobre foundations inconsistentes.

## 18. Estado de execução — Lote 0

Implementado em 2026-08-17 como primeiro lote pequeno e reversível.

### Foundation

- títulos de produto passaram a usar peso 500, priorizando escala e contraste em vez de bold excessivo;
- Light ganhou `surface`, hover e active próprios, sem inversão automática do Dark;
- foco global foi consolidado em um único outline de 2px, compatível com teclado e forced colors;
- o Command desativa correção ortográfica e capitalização automática, evitando ruído de editor num campo de busca/comando.

### App Shell + Sidebar

- shell agora ocupa a viewport de forma determinística, sem perder utilidades do rail em alturas compactas;
- rail preserva o modo compacto de 52px e pode expandir para 208px, com preferência persistida localmente;
- labels e grupos `GERAL`, `TRABALHO` e `MEMÓRIA` tornam explícita a hierarquia existente sem remover, reordenar ou criar rotas;
- Home/Settings e demais destinos usam estado ativo por surface + contraste + marcador posicional;
- Quick Capture e Settings permanecem ancorados e visíveis no rodapé em 840×600;
- rail compacto possui tooltip próprio em hover/focus e foco inset, sem depender de tooltip nativo ou somente de cor;
- a troca de largura é instantânea; apenas labels usam motion curto. A animação de layout testada foi removida após produzir avisos de `ResizeObserver` no WebView.

### Evidência de QA

- inspeção visual real no cliente Tauri em Dark: 840×600, 1024×768, 1280×800, 1440×900 e 1920×1080;
- inspeção visual em Light: 840×600 e 1280×800;
- rail compacto/expandido, hover/focus, selected/active, footer e Command validados;
- `npm run build`: aprovado;
- `npm test`: 2 arquivos e 12 testes aprovados;
- detector Impeccable: nenhuma ocorrência;
- logs do dogfood sem erros, warnings de runtime ou novos loops de `ResizeObserver` após o refinamento.

### Limite deste lote

Nenhuma regra de negócio, rota, API, banco ou contrato de domínio foi alterado. Home, widgets, Hermes e arquitetura interna das páginas permanecem para lotes próprios; o próximo passo recomendado continua sendo **Lote 1 — App Shell avançado + Command + Inspector contract**, com ênfase na anatomia de headers e comportamento de overlays/inspectors.

## 19. Estado de execução — Lote 1

Implementado em 2026-08-17 como evolução do App Shell e definição do contrato compartilhado de navegação master-detail.

### Page e pane anatomy

- `PaneHeader` introduz uma anatomia compacta para contexto, metadados e ações sem repetir page headers altos;
- Inbox e Library passam a usar o mesmo ritmo de pane, preservando os controles e comportamentos existentes;
- page headers completos continuam reservados às superfícies que realmente precisam de título, descrição e ação primária;
- a barra de view existente da Library foi preservada; uma abstração global foi adiada até haver repetição suficiente para justificar um primitive real.

### Inspector contract

- `Inspector` compartilhado consolida superfície, header, navegação estreita e tratamento de `Esc`;
- em desktop, lista e detalhe permanecem simultaneamente visíveis;
- abaixo de 960px, seleção mostra o Inspector em pane única, com retorno explícito à lista e atalho `Esc`;
- Inbox passou a ser o piloto operacional desse contrato; Library foi migrada sem alterar seleção, dados ou ações.

### Command

- resultados agora usam rows densas com tipo, título e contexto alinhados;
- o campo expõe semântica de combobox, resultados usam listbox/options e a seleção ativa é anunciável;
- estados de busca, falha, contagem, prompt inicial e nenhum resultado são silenciosos e específicos para Command;
- navegação por teclado inclui setas, `Home`, `End`, `Enter` e `Esc`;
- Search/Command continua sendo uma superfície de localizar e executar. Hermes não foi fundido nem alterado.

### Menus e motion

- ações de Inbox, Library, Projects, Workspaces e Apps usam o mesmo `ActionMenu`;
- menus fecham por clique externo ou `Esc`, restauram foco e aceitam setas, `Home` e `End`;
- borda hairline, contraste sutil e entrada curta substituem menus nativos inconsistentes;
- `prefers-reduced-motion` continua removendo a animação.

### Evidência de QA

- inspeção visual real no cliente Tauri em Dark: 840×600, 1280×800, 1440×900 e 1920×1080;
- inspeção visual em Light: 1440×900;
- Inbox list-only/Inspector-only, retorno por botão e `Esc`, Command vazio/com resultados e ActionMenu foram validados;
- accessibility tree confirmou combobox, status, listbox, options, menu e menuitems;
- `npm run build`: aprovado;
- `npm test -- --run`: 2 arquivos e 12 testes aprovados;
- `git diff --check`: aprovado.

### Achado adiado

Em 1920×1080, a grade de recursos da Library fica comprimida dentro do pane fixo de 400px enquanto o detalhe ocupa uma área muito ampla. O contrato master-detail está correto, mas a composição interna da Library precisa de uma decisão própria de densidade e largura no Lote 4; ampliar o pane agora apenas deslocaria o problema para outras superfícies.

### Limite deste lote

Nenhuma regra de negócio, rota, API, banco ou contrato de domínio foi alterado. O próximo lote recomendado é **Lote 2 — Home + Widgets**, usando os foundations e contratos de navegação já consolidados antes de avançar para Hermes e demais superfícies operacionais.

## 20. Estado de execução — Lote 2

Implementado em 2026-08-17 como revisão estrutural da Home e definição do contrato visual dos widgets existentes.

### Trabalho da Home

- a primeira dobra foi organizada para cumprir quatro trabalhos em sequência: capturar, escolher contexto, orientar o agora e retomar trabalho recente;
- a ordem completa passa a ser `Capture → Contexto atual → Agora → Retomar → Visão → Acervo → Utilidades`;
- `Agora` concentra Task em andamento, cronômetro e horas do dia; `Retomar` concentra Inbox, recentes e Projects;
- dados analíticos, acervo e utilidades recuam para níveis posteriores da página, reduzindo competição com o foco imediato;
- nenhuma capacidade foi adicionada ou removida: a mudança é de composição, hierarquia e densidade.

### Widget contract

- tamanho deixou de ser a única semântica do widget: cada instância declara um papel visual (`focus`, `attention`, `overview`, `collection` ou `utility`) e um span independente;
- a grade desktop usa 12 colunas e spans de 3, 4, 5, 6, 8, 9 ou 12, permitindo composições intencionais sem impor um catálogo rígido S/M/L/XL;
- IDs persistidos dos widgets foram preservados, mantendo preferências e contratos existentes;
- seções sem widgets visíveis não renderizam títulos órfãos;
- widgets continuam quietos em repouso; este lote não introduziu motion decorativo.

### Densidade e contexto

- o seletor de Workspace deixou de ser um painel alto e passou a funcionar como uma faixa contextual compacta, com estado pressionado por surface, contraste e semântica de toggle;
- o conteúdo da Home usa largura máxima de 1280px, preservando legibilidade em ultrawide sem restringir superfícies naturalmente expansivas do produto;
- grid e seções usam ritmo de 20px e 32px, respectivamente, a partir dos tokens existentes;
- acima de 1100px a composição usa 12 colunas; até 1100px usa 6 colunas; até 760px cada widget ocupa uma linha;
- em 840×600, captura, contexto e o bloco principal de `Agora` permanecem legíveis sem adquirir escala de interface mobile.

### Acessibilidade

- cada grupo da Home é uma região nomeada por heading visível;
- o contexto atual expõe group e botões de alternância, além do estado visual selecionado;
- a ordem do DOM acompanha a prioridade visual, evitando divergência entre teclado, leitor de tela e grid;
- foco global, forced colors e reduced motion continuam herdando os foundations dos lotes anteriores.

### Evidência de QA

- inspeção visual real no cliente Tauri em Dark: 840×600, 1024×768, 1280×800, 1440×900 e 1920×1080;
- inspeção visual em Light: 1440×900;
- Home validada em estado vazio e preenchido, incluindo Workspace selecionado, Project vinculado e Task em `Doing`;
- fixtures temporários de QA foram arquivados após a inspeção e não permanecem nas superfícies ativas;
- accessibility tree confirmou regiões, headings, group, toggle buttons e ordem de leitura;
- `npm run build`: aprovado;
- `npm test -- --run`: 2 arquivos e 12 testes aprovados;
- detector Impeccable: nenhuma ocorrência.

### Limite deste lote

Nenhuma regra de negócio, rota, API, banco ou contrato de domínio foi alterado. O próximo lote recomendado é **Lote 3 — Hermes**, refinando conversa, histórico, composer, estados de streaming e integração visual com o shell sem confundir Hermes com Command/Search.

## 21. Estado de execução — Lote 3A

Implementado em 2026-08-17 como primeiro recorte visual do Hermes, limitado aos estados que puderam ser observados sem fabricar dados ou alterar o gateway.

### Histórico e comportamento compacto

- a coluna de conversas permanece persistente em larguras amplas e passa a abrir como drawer abaixo de 1280px, preservando a largura útil da conversa;
- no modo compacto, o histórico inicia fechado, possui ação `CONVERSAS`, backdrop, fechamento explícito e contrato de `Esc`/`Ctrl + /`;
- o drawer usa semântica de diálogo, mantém o foco contido enquanto aberto e o devolve ao gatilho ao fechar;
- a conversa selecionada combina surface, contraste e um marcador posicional discreto; o estado não depende somente da cor de accent;
- hover foi reservado às linhas inativas, reduzindo ambiguidade entre passagem do ponteiro e seleção.

### Estado vazio e composer

- a mensagem inicial deixou de ocupar o centro editorial da tela e passou a orientar o olhar para o composer;
- título, descrição e sugestões usam escala tipográfica contida, sem quebra manual ou heading de landing page;
- em telas compactas, a régua `VENDO`, a thread e o composer reduzem padding sem aumentar controles ou perder densidade desktop;
- o composer continua sendo a principal affordance da superfície, sem decoração ou motion em repouso.

### Indisponibilidade

- o erro bruto do bridge não disputa mais a atenção com a recuperação: a primeira linha traduz a causa provável em linguagem de produto;
- `RECONECTAR` permanece imediatamente disponível quando há credencial e o gateway está offline;
- o diagnóstico original continua acessível em `DETALHES TÉCNICOS`, com quebra e overflow seguros para URLs e mensagens longas;
- estados de credencial ausente, autenticação recusada, rate limit e túnel/conexão indisponível recebem descrições específicas sem alterar o contrato do gateway.

### Evidência de QA

- inspeção visual real no cliente Tauri em Dark: 840×600, 1024×768 e 1440×900;
- inspeção visual em Light: 1440×900;
- histórico persistente, drawer compacto aberto/fechado, backdrop, selected state, foco devolvido, estado vazio e indisponibilidade foram validados;
- accessibility tree confirmou região complementar, diálogo modal compacto, busca, botões nomeados, status e disclosure técnico;
- o review React preservou buffer de streaming, mensagens memoizadas e cleanup de listeners, sem introduzir componente inline ou efeito sem ciclo de vida;
- `npm run build`, `npm test -- --run`, detector Impeccable e `git diff --check`: aprovados após o lote.

### Limite deste lote

O Hermes estava offline porque o túnel SSH não estava aberto. Nenhuma conversa, mensagem, tool call ou estado de streaming foi fabricado para produzir screenshots. A composição de mensagens conectadas, streaming, tools, citations, clarify e approval permanece para o **Lote 3B**, quando esses estados puderem ser observados de ponta a ponta. Nenhuma regra de negócio, API, banco, bridge ou contrato de domínio foi alterado.

## 22. Estado de execução — Lote 4A

Implementado em 2026-08-17 como primeiro refinamento das superfícies operacionais em lista. O Hermes 3B permanece aguardando uma conexão real; a Inbox foi escolhida por estar desbloqueada e já usar o contrato de Inspector.

### Intenção e empty state

- a tela vazia deixa de ser uma frase solta no início do canvas e passa a explicar silenciosamente o ciclo da Inbox;
- `Inbox limpa` comunica conclusão, enquanto a descrição esclarece que novas Captures permanecem ali até uma decisão;
- `Capturar` é a única ação do estado vazio e reutiliza o Quick Capture existente;
- nenhum card, ilustração genérica ou efeito decorativo foi introduzido.

### Lista e seleção

- a ação contextual `Capturar` também fica disponível no cabeçalho compacto da lista;
- rows preservam a densidade de duas linhas, com conteúdo, origem e tempo alinhados;
- setas, `Home` e `End` passam a mover foco e preview pela lista usando o mesmo contrato já existente na Library;
- o selected state compartilhado troca o wash de accent e a barra lateral por `surface-active` e peso tipográfico, mantendo contraste sem depender somente de cor;
- em larguras intermediárias, a coluna da Inbox usa até 42% do conteúdo e cede espaço ao Inspector; em 1024px isso mantém as ações principais em uma única linha.

### Inspector e linguagem

- `CAPTURE` substitui o rótulo genérico `SELECIONADO`, identificando a entidade em vez de narrar o estado da interface;
- o bloco do Hermes indisponível recua para uma faixa de sistema com surface e hairlines, sem marcador de accent;
- a ação que altera `processing_state` passa a se chamar `Marcar processada`; `Arquivar` continua reservado ao menu de lifecycle;
- a dica `J / K · Espaço`, que não correspondia ao comportamento implementado, foi removida;
- `Criar Task`, `Salvar Resource`, processamento, Archive e Trash mantêm exatamente os contratos existentes.

### Evidência de QA

- inspeção visual real no cliente Tauri em Dark: 840×600, 1024×768 e 1280×800;
- inspeção visual em Light: 1280×800;
- estados vazio, preenchido, selected, lista compacta, Inspector compacto, retorno à lista e receipt de processamento foram validados;
- accessibility tree confirmou região nomeada da lista, article do Inspector, heading do empty state, ações nomeadas e status de processamento;
- fixtures temporários foram marcados como processados e não permanecem na Inbox dogfood;
- `npm run build`: aprovado;
- `npm test -- --run`: 2 arquivos e 12 testes aprovados;
- detector Impeccable e `git diff --check`: aprovados.

### Limite deste lote

Nenhuma regra de negócio, API, banco, schema ou contrato de domínio foi alterado. O lote corrige composição, linguagem, selected state, navegação de lista e responsividade. O próximo lote desbloqueado recomendado é **Lote 4B — Tasks**, começando pela relação entre lista, Kanban e Inspector sem ampliar o modelo funcional.

## 23. Estado de execução — Lote 4B

Implementado em 2026-08-17 como refinamento de Tasks, limitado à projeção Kanban, criação existente e detalhe da Task. Os seis estados e todas as transições de domínio foram preservados.

### Auditoria da superfície

- no estado vazio, seis colunas continuavam impondo scroll horizontal e repetiam `Nenhuma Task` seis vezes sem acrescentar orientação;
- a dica de teclado aparecia colada ao breadcrumb e podia ser lida como parte do caminho da página;
- colunas vazias usavam texto de corpo com peso visual próximo ao conteúdo real;
- o detalhe abria como overlay de 400px com sombra larga e foco no container inteiro, produzindo uma moldura de accent ao redor da gaveta;
- em 840px, o drawer deixava uma faixa estreita e inutilizável do quadro visível ao lado;
- salvar e arquivar não expunham estado pendente nem erro contextual equivalente.

### Quadro e estado vazio

- quando não há Tasks ativas, o Kanban recua e dá lugar a um único estado vazio com contagem, explicação curta e uma ação `Criar Task`;
- a criação continua inline e usa o mesmo `DirectTaskForm`, sem modal, rota ou capacidade adicional;
- com Tasks, contagem ativa, atalho de movimento e ação principal formam um grupo separado do breadcrumb;
- o atalho `Alt + ←/→` continua visível e nomeado para tecnologia assistiva;
- colunas vazias passam a dizer apenas `Vazio` em texto pequeno e terciário;
- cards mantêm largura estável, borda hairline, radius contido e densidade existente; não ganharam elevação, hover com deslocamento ou metadata sem função.

### Inspector da Task

- a gaveta passa a ler como Inspector por diferença de surface e borda lateral hairline, sem sombra decorativa;
- a largura desktop cai para 380px e, abaixo de 960px, o Inspector ocupa toda a área de conteúdo, preservando somente o rail global;
- o foco inicial vai para o título em vez do container e retorna ao card de origem quando ele ainda existe;
- `Esc` permanece como fechamento do detalhe quando nenhuma ação está pendente;
- campos e ações ficam desabilitados durante persistência, com feedback `Salvando` ou `Arquivando`;
- falhas de salvar ou arquivar permanecem no Inspector como alerta contextual, sem alterar os comandos existentes.

### Evidência de QA

- inspeção visual real no cliente Tauri em Dark: 840×600, 1280×800, 1440×900 e 1920×1080;
- inspeção visual em Light: 840×600;
- estados vazio, criação, quadro preenchido, colunas vazias, títulos longos, Inspector desktop e Inspector responsivo foram validados;
- accessibility tree confirmou heading e ação do vazio, região Kanban, headings de coluna, cards como botões, Inspector nomeado, campos e ações;
- quatro Tasks e um Project temporários foram criados apenas no perfil dogfood e arquivados pelo fluxo normal de UI ao fim da inspeção;
- `npm run build`: aprovado;
- `npm test`: 2 arquivos e 12 testes aprovados;
- detector Impeccable e `git diff --check`: aprovados.

### Decisões de escopo e achado adiado

A `List` segmentada mencionada em `DESIGN-FOUNDATIONS.md` não foi criada neste lote. Embora seja uma projeção dos mesmos dados, adicioná-la seria comportamento de produto novo e contrariaria o recorte visual autorizado. O Kanban continua sendo a única view atual.

Durante a limpeza das fixtures em 840×600, Projects revelou um pane de detalhe estreito demais para título, menu e ação. A arquitetura master-detail de Projects deve receber tratamento responsivo próprio no próximo lote de Projects; ampliar sua largura dentro de Tasks apenas deslocaria o problema.

Nenhuma regra de negócio, API, banco, schema, estado de trabalho ou contrato de domínio foi alterado. O próximo lote recomendado é **Lote 4C — Projects**, corrigindo sua composição master-detail e o comportamento abaixo de 960px antes de avançar para Library e Calendar.

## 24. Estado de execução — Lote 4C

Implementado em 2026-08-17 como refinamento de Projects no contrato master-detail compartilhado. Criação, edição, Archive, Tasks relacionadas e o campo `repository` foram preservados.

### Auditoria da superfície

- o detalhe usava `article.detail-pane` fora do `Inspector`, sem pane única abaixo de 960px;
- em 840×600, título, menu e fatos competiam numa coluna estreita ao lado da lista;
- o empty state ficava confinado ao pane esquerdo enquanto o detalhe vazio dominava a página;
- a lista não movia preview com setas e não devolvia foco ao voltar do detalhe;
- Archive não expunha estado pendente nem erro contextual.

### Composição e responsividade

- Projects adota `inspector-page` com o mesmo breakpoint de 960px de Inbox e Library;
- em desktop, lista e detalhe permanecem lado a lado; a coluna da lista cede até 42% e o detalhe recebe o restante;
- abaixo de 960px, seleção ou criação abre o Inspector em pane única, com `Voltar à lista` e `Esc`;
- empty state passa a ser página inteira com contagem, explicação curta e uma ação `Novo Project`;
- header do detalhe, fact-grid e painel de Tasks usam o ritmo compacto já consolidado, com quebra segura de títulos longos.

### Lista, seleção e lifecycle

- `PaneHeader` concentra caminho, contagem de ativos e a ação silenciosa `Novo Project`;
- rows preservam nome, descrição e progresso de Tasks; setas, `Home` e `End` movem preview;
- `Enter`/`Espaço` abrem o detalhe e, no compacto, transferem o foco ao Inspector;
- Archive mostra `Arquivando`, desabilita ações concorrentes e devolve erro no Inspector quando falha;
- após arquivar, a superfície retorna à lista e o receipt de desfazer permanece idêntico.

### Evidência de QA

- `npm run build`: aprovado;
- `npm test -- --run`: 2 arquivos e 12 testes aprovados;
- `git diff --check`: aprovado;
- inspeção visual no cliente Tauri validada em Dark: 840×600, 1280×800, 1440×900 e 1920×1080; Light validado onde borders/seleção mudam.

### Limite deste lote

Nenhuma regra de negócio, API, banco, schema ou contrato de domínio foi alterado. Workspaces e Apps ainda repetem o master-detail antigo e entram no **Lote 4F**. O próximo lote recomendado é **Lote 4D — Library / Resources**.

## 25. Estado de execução — Lote 4D

Implementado em 2026-08-17 como refinamento de Library/Resources. Filtros, kinds, lifecycle, proveniência e vínculos de Workspace foram preservados.

### Auditoria da superfície

- a grade fixava quatro colunas dentro de um pane de 400px, comprimindo títulos em 1920×1080;
- o Inspector ocupava o restante da largura e espalhava nota/URL além da medida de leitura;
- a barra de filtros competia horizontalmente sem grupos claros quando contexto + tipo + view coexistiam;
- teclado da grade não movia preview; retorno de foco ignorava tiles selecionados;
- labels de detalhe diziam `LINK` mesmo para note/image/library.

### Composição e densidade

- Library inverte a proporção operacional: acervo `1fr` + Inspector contido em até `min(400px, 36%)`;
- tiles usam `auto-fill` com mínimo ~9.5rem, caindo para ~8.75rem abaixo de 1280px;
- títulos e motivos limitam-se a duas linhas com quebra segura; origem mono elide;
- conteúdo do Inspector (header, nota, contexto, form e ações) respeita `--measure`;
- URL aparece em mono secundário e some quando o Resource não tem endereço;
- empty state continua expandindo a coleção e ocultando o detalhe.

### Filtros, seleção e teclado

- `Novo Resource` sobe para o `PaneHeader` como ação silenciosa;
- grupos de filtro quebram linha e separam-se por hairline até 1100px; abaixo disso empilham sem borda lateral;
- setas, `Home` e `End` movem preview em grid e lista; `Enter`/`Espaço` abrem o detalhe;
- no compacto (<960px), seleção/criação foca o Inspector; `Esc` e Voltar devolvem foco ao tile/row/ação;
- detalhe nomeia o kind real (`SITE`, `LIBRARY`, `IMAGEM`, `NOTA`) e só oferece `Abrir link` quando há URL.

### Evidência de QA

- `npm run build`: aprovado;
- `npm test -- --run`: 2 arquivos e 12 testes aprovados;
- `git diff --check`: aprovado;
- inspeção visual no cliente Tauri validada em Dark: 840×600, 1280×800, 1440×900 e 1920×1080; Light validado onde surfaces/borders mudam.

### Limite deste lote

Nenhuma regra de negócio, API, banco, schema ou contrato de domínio foi alterado. O próximo lote recomendado é **Lote 4E — Calendar**.

## 26. Estado de execução — Lote 4E

Implementado em 2026-08-17 como refinamento do Calendar retrospectivo. Fontes, `CalendarItem`, janela da grade e regras temporais foram preservados.

### Auditoria da superfície

- o detalhe do dia abria abaixo das seis semanas, fora do primeiro viewport;
- a grade vivia dentro de `Card`/`PageHeader` herdados do Tempo, com chrome de produto absorvido;
- navegação de mês usava glifos sem tooltip/grupo explícito;
- células com `aspect-ratio` fixo competiam com o card e o detalhe empilhado;
- teclado não movia a seleção pela grade.

### Composição e detalhe

- Calendar adota `inspector-page`: grade à esquerda, dia à direita;
- o detalhe deixa de empilhar abaixo do mês e passa a ser Inspector lateral sempre visível no desktop;
- abaixo de 960px, seleção abre pane única com Voltar/`Esc` e devolve foco à célula;
- sem dia escolhido, o placeholder lateral orienta a seleção; no compacto ele some para não roubar a grade;
- `Card` e título de página pesado saem; `PaneHeader` + mês + nav compacta bastam.

### Grade, densidade e teclado

- a grade usa hairlines como estrutura (sem card ao redor de cada célula);
- selected = surface-active + marcador lateral; today = sódio no número do dia;
- setas movem o preview em passos de 1/7; `Home`/`End` vão aos extremos; `Enter`/`Espaço` abrem o dia;
- lista do dia vira rows densas (hora, kind, título, duração) com quebra segura;
- domínio permanece retrospectivo: nenhum prazo, compromisso ou agenda foi introduzido.

### Evidência de QA

- `npm run build`: aprovado;
- `npm test -- --run`: 2 arquivos e 12 testes aprovados (inclui `calendarDays`);
- `git diff --check`: aprovado;
- inspeção visual no cliente Tauri validada em Dark: 840×600, 1280×800, 1440×900 e 1920×1080.

### Limite deste lote

Nenhuma regra de negócio, API, banco, schema ou contrato de domínio foi alterado. O próximo lote recomendado é **Lote 4F — Apps, Workspaces e Settings**.

## 27. Estado de execução — Lote 4F

Implementado em 2026-08-17 como fechamento das superfícies master-detail restantes e organização visual do Settings. Capacidades, vínculos, catálogo, backup, Functions e lifecycle foram preservados.

### Workspaces e Apps

- ambas adotam `inspector-page` com o mesmo breakpoint de 960px de Projects/Inbox;
- empty state em página inteira; lista com `PaneHeader`, contagem e ação silenciosa;
- setas/`Home`/`End` movem preview; `Enter`/`Espaço` abrem o detalhe;
- Archive expõe pendência e erro contextual; Apps também sinaliza abertura pendente;
- relações de Workspace (Projects, Apps, Widgets) e fatos/capacidades de App permanecem no Inspector;
- proporção de lista/detalhe alinhada a Projects (`min(list-pane, 42%)`).

### Settings

- `PaneHeader` substitui o breadcrumb solto;
- painéis existentes foram agrupados em seções nomeadas: conexão/aparência, atualizações/entrada, dados/ciclo de vida e avançado;
- nenhuma capacidade foi removida, reordenada entre domínios ou escondida atrás de navegação nova;
- densidade e formulários reutilizam o ritmo já consolidado de panels e setting-rows.

### Evidência de QA

- `npm run build`: aprovado;
- `npm test -- --run`: 2 arquivos e 12 testes aprovados;
- inspeção visual no cliente Tauri validada em Dark: 840×600, 1280×800, 1440×900 e 1920×1080.

### Limite deste lote

Nenhuma regra de negócio, API, banco, schema ou contrato de domínio foi alterado. Hermes 3B permanece condicionado à conexão real. O próximo lote estrutural restante é **Lote 5 — Motion e consistência transversal**, após validação visual das superfícies 4C–4F.

## 28. Estado de execução — Lote 5

Implementado em 2026-08-17 como fechamento transversal da trilha UI/UX vNext. O lote preservou regras de negócio, APIs, banco, schema, navegação e contratos de domínio.

### Motion e foco

- `Inspector` usa o mesmo contrato em desktop e pane única: entrada por opacity/translate, saída de 90ms e foco devolvido à lista sem aguardar a animação;
- `ActionMenu` deixou o `details` nativo e passou a expor gatilho, menu e menuitems explícitos, com clique externo, setas, `Home`, `End` e `Esc` consistentes;
- o spike com Framer Motion no WebView2 real não produziu warnings de `ResizeObserver`; a orquestração ficou limitada a `AnimatePresence`/`useReducedMotion`, com feature bundle carregado à parte e sem layout measurement;
- `page-surface`, drawer e overlays preservam os contratos de entrada/saída existentes e os tokens compartilhados.

### Estados e acessibilidade

- `StateMessage` cobre `empty`, `loading`, `error`, `saving` e `saved` com `aria-live` embutido e disclosure técnico opcional;
- Capture, Quick Capture, formulários de Task/Project/Workspace/App/Resource, boot, conexão Hermes e feedback de Settings usam a mesma linguagem de estado;
- o indicador compacto de sync permanece uma exceção deliberada: ele compartilha os tokens, mas conserva a geometria própria da topbar;
- `prefers-reduced-motion` possui uma única fonte em `packages/design-system/tokens.css` e zera tokens, animações e transições; a cópia de handoff foi sincronizada;
- as exceções reais de `forced-colors` foram consolidadas em um único bloco local para seleção e acabamentos específicos.

### Correções da revisão 4D–4F

- Settings segue a ordem de leitura documentada: conexão/aparência, atualizações/entrada, dados/ciclo de vida e avançado;
- `PaneHeader` permite quebra responsiva de ações, removendo scroll horizontal interno em Apps e Calendar;
- o lockfile recuperou metadados de plataforma removidos acidentalmente e passou a registrar apenas o novo grafo de motion.

### Evidência de QA

- cliente Tauri real em Dark: 840×600, 1280×800, 1440×900 e 1920×1080;
- Light em 1440×900 para Projects, Calendar e Library;
- zero overflow de página, botões sem nome, campos sem label ou IDs duplicados nas oito superfícies verificadas;
- Inspector e ActionMenu validados por teclado, retorno de foco e delayed unmount; console sem erros e sem warnings de `ResizeObserver` em motion normal;
- reduced motion confirmou todos os tokens relevantes em `0ms`; forced colors confirmou os fallbacks do novo primitive;
- `npm run build`, `npm test -- --run`, `npx impeccable detect apps/desktop/src` e `git diff --check`: aprovados.

### Limite deste lote

Tempo não foi redesenhado neste fechamento transversal. Hermes 3B continua condicionado a uma conexão real para que mensagens, streaming, tools, citations, clarify e approval sejam observados de ponta a ponta, sem estados fabricados.

## 29. Estado de execução — Widgets com geometria macia

Implementado em 2026-08-17 a partir da referência `amicro.vercel.app/mono-charts`,
trazida pelo proprietário. Autorizado pela ADR-040, que é pré-requisito e não
consequência: sem ela o código contradiria a ADR-034 em três pontos.

Este não é um lote da trilha vNext — ela se encerrou no Lote 5. É um recorte
pedido à parte, e segue os mesmos gates.

### O que a referência deu, e o que ela não deu

Adotados: formas novas, o acabamento de card **em todos** os 15 widgets, e a
geometria arredondada. Recusada a paleta monocromática — o sódio continua
reservado para carga, e agora/hoje continuam em `--text`.

### Moldura

- regra escopada a `.home-grid .widget`: card em `--surface-raised`, forma em
  `--surface`, e nenhum alcance sobre o `Panel` de Settings, do Inspector de
  Workspaces ou do Tempo;
- a superfície aninhada se declara **por preenchimento no escuro e por borda no
  claro** — no claro `#FFFFFF` sobre `#FAFBFC` são 2% e o preenchimento não
  desenha nada. O mesmo mecanismo cobre `forced-colors` sem exceção própria;
- raios concêntricos: `--radius-widget: 12px` fora, `--radius-lg: 8px` dentro,
  `--space-3` entre as bordas. `--radius: 3px` segue intocado no resto do sistema;
- manchete como prop opcional do `Panel`, e não do `<Widget>`: o `Panel` traz
  rótulo e conteúdo como um bloco só, então um número irmão dele cairia antes do
  rótulo. Os 8 usos fora da Home não passam a prop e não mudam.

### Formas

- `plotGeometry.ts` concentra a aritmética, com 15 testes de nó. Nenhum
  componente calcula;
- **`rx` não mente, `linecap` mente**: só as formas de traço são compensadas;
- o anel passou a ponta arredondada com `L' = max(ε, L − espessura)`. Abaixo de
  uma espessura ele afirma presença em vez de medir; zero continua não desenhando;
- TASKS NA SEMANA → `Bars`; HORAS POR PROJECT → `Stack`; META → `Bullet`;
  HORAS HOJE ganhou `Spark`. Todos sobre dado que já estava na tela;
- o `Bullet` resolveu uma limitação escrita no código do `BudgetRing`: o anel
  parava em cheio e o estouro da meta vivia só no texto.

### Duas correções que só a renderização revelou

Nenhum teste pegaria as duas, e nenhuma leitura de JSX também:

1. **SVG esticado distorce geometria arredondada.** Um SVG que preenche a
   largura do card precisa de `preserveAspectRatio="none"`, e aí a escala
   horizontal deixa de ser igual à vertical: todo `rx` virava elipse e a pílula
   saía como ovo. As três formas retangulares passaram a HTML com
   `border-radius`, resolvido em pixels reais nos dois eixos. O `Spark` ficou em
   SVG com `non-scaling-stroke`, pelo mesmo motivo aplicado à espessura.
2. **`border-radius: 999px` produz oval, não pílula.** O CSS limita o raio a
   metade da MENOR dimensão, e com sete barras num card largo a menor é a
   altura. A referência oferece as duas variantes — `Full Radius` e
   `Corner Radius: 8px All` — e só a segunda se sustenta em qualquer largura.
3. **Degrau de profundidade sobre branco some.** A empilhada misturava com
   `transparent`, como o anel faz; mas o anel é traço fino sobre trilho, e uma
   área preenchida a 30% sobre `#FFFFFF` desaparece. Passou a misturar com
   `--surface-hover`, como a densidade já fazia.

### Evidência de QA

- `npm run build`, `npm test -- --run` (3 arquivos, 27 testes),
  `npx impeccable detect src` e `git diff --check`: aprovados;
- bancada de renderização headless com os arquivos de estilo reais, em Dark e
  Light, cobrindo as quatro formas novas, a moldura, a manchete, o rodapé e a
  compensação do anel em 88px, 44px e 14px a 5%, 0,5% e zero. Foi ela que
  revelou as três correções acima;
- **pendente, e do proprietário:** a janela do Tauri em Dark e Light nas quatro
  larguras (840×600, 1280×800, 1440×900, 1920×1080), a confirmação de que a
  moldura não vazou para Settings/Workspaces/Tempo, teclado e foco, e
  `reduced-motion` e `forced-colors` ligados no sistema. A bancada não substitui
  isso: ela renderiza as formas, não a Home com dado real.

Um ponto para julgar em tela: no claro, a barra de HOJE é `--text`, quase preta.
É a regra da família (agora/hoje nunca em sódio) aplicada a uma área maior que a
de um traço, e pode pesar demais.

### Limite deste recorte

Nenhuma regra de negócio, API, banco, schema ou contrato de domínio foi alterado.
Nenhum widget novo entrou, e os sete que a ADR-034 deixou de fora continuam fora.
Tempo e Hermes 3B seguem pendentes como estavam.

## 30. Estado de execução — Argos

Implementado em 2026-08-18 a partir da referência `bloub.vercel.app`, trazida pelo
proprietário. Autorizado pela ADR-041, que é pré-requisito: sem ela o código
contradiria a leitura literal de `UX-PRINCIPLES.md` §16 e de
`HERMES-PREMIUM-CHAT.md` §7.6.

Argos é a **face do estado do M/OS**, na topbar, ao lado do indicador que já
dizia a mesma coisa em palavras. Não é avatar do Hermes, e nunca aparece dentro
da superfície dele.

### O repertório

Seis poses, todas ligadas a sinal existente, com a precedência "quem precisa mais
de você ganha":

```
encarando  >  assustado  >  trabalhando  >  fechado  >  concentrado  >  desperto
```

Três pesos de cor, e cada um significa uma coisa: `--text-system` em repouso (o
mesmo peso do texto ao lado), `--text` quando há o que acompanhar, e
`--signal-ink` nas duas poses em que o sistema não continua sozinho — ou espera
sua resposta, ou quebrou. `--signal-fill` não entra: a ADR-034 o reservou para
carga, e Argos não mostra carga.

### O que a referência deu, e o que foi recusado

**Recusado o desenho.** O autor do bloub registra que a licença MIT cobre o
código, não o desenho, que imita o mascote da x.ai. A silhueta do M/OS é um
quadrado de cantos macios, herdando a família geométrica do símbolo do rail sem
herdar a marca.

**Adotado o mecanismo:** uma silhueta preenchida com a expresão inteira nos
olhos, estados como dado, e um motor puro — `argosPose.ts`, no mesmo padrão de
`plotGeometry.ts`.

### Argos só escuta

Ele assina `hermes.onEvent()` e nunca responde — em particular, nunca chama
`hermes.approve`. A verificação é mecânica e ficou no plano: um `grep` por
métodos de escrita em `Argos.tsx` precisa voltar vazio.

O cronômetro ganhou assinatura própria e leve: `useTrackedTime` carrega todas as
entradas de tempo, e Argos só precisa saber se ele corre.

### A correção que só a renderização revelou

Na primeira versão, **as seis poses se reduziam a três leituras a 24px**: desperto
e trabalhando, encarando e assustado, concentrado e fechado eram pares
indistinguíveis. As diferenças eram sutis — um desvio de 1,4 unidade, uma
inclinação de 14 graus, meia unidade de altura — e nada disso sobrevive ao
tamanho real.

As separações passaram a ser categóricas: desvio maior que um raio de olho,
fresta vertical contra oval redondo, traço largo contra ponto. E cada uma ficou
travada num teste que verifica a SEPARAÇÃO, não o número — para que o par não
volte a colar numa calibragem futura.

Um segundo achado foi de bancada, e não de produto: o recorte de CSS terminava
numa classe que vem antes do bloco do Argos no arquivo, devolvendo string vazia.
O que aparecia era o preenchimento padrão do SVG.

### Evidência de QA

- `npm run build`, `npm test -- --run` (4 arquivos, 44 testes),
  `npx impeccable detect src` e `git diff --check`: aprovados;
- bancada de renderização headless com os tokens e o CSS reais, em Dark e Light,
  com as seis poses no tamanho real (24px) e ampliadas, mais a topbar simulada;
- **pendente, e do proprietário:** as seis poses contra sinal real na janela do
  Tauri — boot, operação em curso, cronômetro correndo, Hermes gerando, proposta
  aguardando confirmação e falha —, nas quatro larguras e nos dois temas, mais
  `reduced-motion`, `forced-colors` e a confirmação de que Argos não aparece na
  árvore de acessibilidade. A bancada desenha as poses; ela não prova que o sinal
  certo produz a pose certa.

### Limite deste recorte

Nenhuma regra de negócio, API, banco, schema ou contrato de domínio foi alterado.
Argos não dorme: o sinal de inatividade real mora no Rust, no monitor da ADR-037,
e fingi-lo com um temporizador de renderer seria inventar o dado. É o que abriria
um segundo recorte.
