# M/OS — Design System Spec v0.5

Especificação de implementação. Escrita para ser lida por um agente de código.
Se algo aqui conflitar com uma decisão de implementação, **esta especificação ganha**.

Fonte de valores: `mos-tokens.css`. Nenhum componente declara cor, tamanho, spacing, radius ou duração próprios.

---

## 1. Premissa

M/OS é um sistema operacional pessoal: uma camada entre pensar e fazer. Capture → Organize → Connect → Act.

Duas frases governam toda decisão visual:

- **Quiet interface. Powerful system.** A interface é neutra em 95% da superfície e inconfundível nos 5% que o usuário toca cem vezes por dia.
- **Informação antes do container.** O elemento mais importante da tela é o dado, não a caixa em volta dele.

Território visual: **Console** — neutros frios, camada mono explícita para dado de sistema, a barra `/` como sintaxe.

---

## 2. Tipografia

| Papel | Fonte | Tamanho / peso | Tracking |
|---|---|---|---|
| Display | Schibsted Grotesk | 48 / 700 | −0.034em |
| Title | Schibsted Grotesk | 28 / 700 | −0.024em |
| Capture | Schibsted Grotesk | 21 / 400 | −0.022em |
| Body | Schibsted Grotesk | 15 / 400 | −0.008em |
| UI | Schibsted Grotesk | 14 / 400 | −0.008em |
| Small | Schibsted Grotesk | 13 / 400 | 0 |
| Meta | JetBrains Mono | 11 / 400 | +0.05em |
| Micro | JetBrains Mono | 9 / 400 CAPS | +0.14em |

Três pesos apenas: 400, 500, 700. **600 não existe no sistema.**

### Fronteira da mono — regra dura

A mono marca **dado de sistema**, nunca conteúdo.

- **Sim:** timestamp, caminho de contexto, id, atalho de teclado, contagem, progresso, tipo de resultado, código.
- **Nunca:** título, nome de Project, texto capturado pelo usuário, rótulo de botão, mensagem de erro, item de navegação, qualquer frase.
- Tamanho máximo 12px fora de blocos de código.

Se a mono vazar para conteúdo, o produto vira terminal. Esta é a violação mais fácil de cometer e a mais caras de corrigir.

---

## 3. Cor

Um único sinal: **âmbar-sódio**. Ele significa *ativo, selecionado, em foco, ou pedindo ação*.

### Dois papéis do sinal

- `--signal-fill` — preenchimento. Idêntico nos dois modos (`#E7C24E`).
- `--signal-ink` — tinta (texto, ícone, barra fina). `#E7C24E` no dark, `#8A6A12` no light. Âmbar puro sobre branco é ilegível.

### Onde o sódio pode aparecer
focus ring · marcador de seleção (barra de 2px) · a barra de contexto `/` · progresso · autoria de Hermes · botão primário · contagem que exige ação.

### Onde não pode
hover · ícone inativo · texto corrido · borda de panel · gradiente ou glow · estado de sucesso · qualquer decoração.

### Warning não tem cor
Warning = ícone + frase + borda neutra (`--text-system`). O âmbar pertence ao accent. Nenhum estado do sistema depende apenas de cor.

### Superfícies
Três níveis, nunca quatro: `--canvas` → `--surface` → `--surface-raised`.
- **Dark:** separa por luminância crescente; borda quase invisível.
- **Light:** separa por borda com peso real; superfície fica mais branca que o canvas.
- Sombra existe **somente** em overlay flutuante (`--shadow-overlay`).

---

## 4. Geometria

- **Radius:** 3px em tudo; 2px em elementos de 14–20px; 8px só em app icon e overlay grande.
- **Spacing:** apenas 4, 8, 12, 20, 32, 52, 84. Dentro de linha: 8 e 12. Entre blocos: 32 e 52.
- **Stroke de ícone:** 1.5 em 24px · 1.25 em 20px · 1 em 16px. Cada tamanho é um desenho próprio — **nunca escalar o SVG**. Terminais retos (butt cap). `filled` significa uma única coisa: destino ativo na navegação.
- **Shell:** rail 52 · sidebar 232 · drawer 400 · margem de conteúdo 56 (32 abaixo de 1280px).
- **Overlays:** largura fixa (Capture 640 · Command 720 · Dialog 440 · Menu 216), posicionados a **34% da altura da tela**, nunca centralizados verticalmente.

---

## 5. A sintaxe da barra `/`

A barra é marca, caminho, comando e limiar — sempre em `--font-system` e `--signal-ink`.

1. **Caminho de contexto:** `WEB-DESIGN / MINARUM / BOARD` em mono maiúscula no topo da tela, último segmento em `--text`. Substitui título de página.
2. **Limiar de entrada:** todo campo onde algo entra no sistema começa com `/` — Capture, Command, Search, ditado.
3. **Comando:** digitar `/` no campo transforma texto em comando. Sem paleta separada, sem modo.
4. **Autoria do sistema:** barra vertical de 2px em sódio marca tudo que o sistema produziu (interpretação, síntese) — e é o mesmo marcador da seleção.
5. **Transição de contexto:** na troca de Workspace a barra percorre 20px e o conteúdo faz cross-fade (220ms).

**Nunca:** dentro de conteúdo do usuário · duas barras com funções diferentes na mesma linha · como divisor decorativo · em qualquer cor que não seja signal-ink.

---

## 6. Componentes

### 6.1 Button

| Variante | Fundo | Texto | Borda |
|---|---|---|---|
| primary | `--signal-fill` | `--on-signal` | none |
| secondary | `--surface-hover` | `--text` | none |
| outline | transparent | `--text` | 1px `--border-strong` |
| ghost | transparent | `--text-secondary` | none |
| destrutivo | transparent | `--danger` | 1px `--danger` a 40% |

- Alturas: sm 28 / md 36 / lg 44 (lg só em touch). Padding horizontal 12 / 16 / 20.
- Peso 500, verbo no infinitivo, **sem ícone acompanhando texto**, sem caixa-alta.
- Hover: luminância +6%, nunca troca de cor. Press: −10%, sem deslocar o elemento.
- Focus: halo de 3px em `--signal-ring` (idêntico em todo componente).
- Disabled: `--surface-raised` + `--text-disabled`, sem cursor.
- **Um único primário por superfície.**
- Não existe: gradiente, sombra, pill, spinner interno, ícone+texto.

### 6.2 Input / field

- Altura 38, padding horizontal 13, radius 3, borda 1px `--border`.
- Label em Micro mono **acima** do campo, nunca dentro.
- Estados: vazio (`--text-placeholder`) · preenchido (borda `--border-strong`) · focus (borda `--signal-ink` + halo) · erro (borda `--danger` + ícone + frase abaixo) · bloqueado (`--surface-raised`).
- Textarea cresce até 6 linhas e depois rola; nunca redimensiona pelo canto.
- **Capture é a exceção deliberada:** sem caixa, sem borda — apenas a barra `/`, o texto em 21px e uma linha de base.

### 6.3 Controls

- **Checkbox** 14px, radius 2. Marcado = fundo `--signal-fill` + check em `--on-signal`. Estado parcial = traço horizontal.
- **Radio** 14px, círculo — a única forma redonda do sistema (a convenção vale mais que a coerência geométrica).
- **Switch** 30×16, retangular (não pill), thumb 12×12 radius 1. Só para preferências que valem imediatamente; nunca dentro de formulário com Salvar.
- **Segmented** para modo de visualização; **tabs** para seções de conteúdo, com indicador = barra de 2px em sódio.
- Alvo de toque real de 24px (44 em touch) mesmo com o desenho em 14.

### 6.4 Panel — o único container

Três variantes:

1. **Nu** (padrão, ~80% do produto): sem borda e sem fundo. Um fio de 1px no topo e um rótulo em Micro mono.
2. **Delimitado:** borda 1px + `--surface`, padding 18/20. Só quando o conteúdo é *de outra natureza* que o entorno (interpretação do sistema, código, preview).
3. **Elevado:** `--surface-raised` + `--shadow-overlay`. Único lugar com sombra: overlay, menu, dialog, drawer.

**Card no sentido usual não existe.** Teste antes de criar container: ele estabelece agrupamento, interação, hierarquia, separação ou contexto? Se a resposta for "fica mais bonito", não entra.

Proibido: panel dentro de panel delimitado · sombra em conteúdo estático · borda com cor de accent · grade de métricas em cards · panel com menos de duas informações · quarto nível de superfície.

### 6.5 Row — a unidade mais repetida

Estrutura: marcador opcional · conteúdo em grotesk · metadata em mono à direita.

- Alturas: 34 padrão · 30 denso · 44 touch · 56 com segunda linha.
- Separador de 1px entre rows, **nunca gap**.
- Hover: fundo `--surface`, sem borda e sem deslocamento. Ações secundárias aparecem à direita só no hover.
- Selecionado: `--signal-wash` + barra de 2px em `--signal-fill` na borda esquerda.
- Concluído: opacidade 0.42 + line-through discreto.
- Em densidade alta, a metadata desaparece e só volta no item ativo/hover/selecionado.
- Não existe: row com borda própria, avatar, thumbnail, tag colorida, três linhas de texto, ícone de arraste permanente.

### 6.6 Navigation

- **Rail 52px**, só ícones, **máximo 6 destinos**, símbolo no topo (não clicável), `filled` = ativo.
- **Sidebar 232px** expandida no hover do rail ou por atalho: rótulos, Workspace no topo, contagem em mono — em sódio somente quando exige decisão.
- **Contexto:** a linha com o caminho em barra. Substitui breadcrumb e título.
- **Menu:** itens de 30px, atalho sempre visível à direita, um único separador permitido, destrutivo por último em `--danger`, **sem ícones**.

### 6.7 Overlays

- **Capture** 640 · **Command** 720 · **Dialog** 440 · **Drawer** 400 (direita, sem backdrop escurecido — o contexto continua legível) · **Menu** 216.
- Entrada: opacidade + 4px de Y em `--dur-enter`/`--ease-enter`. Saída: `--dur-exit`/`--ease-exit`. Assimetria é deliberada.
- **Dialog existe somente para ação irreversível.** Todo o resto usa ação direta + desfazer.
- Não existe: modal sobre modal, overlay em tela cheia, popover com seta, backdrop escuro em drawer.

### 6.8 Feedback

- **Recibo:** canto inferior esquerdo, mono micro, 1,6s, sem botão de fechar. Substitui toast.
- **Desfazer:** ações em lote ganham janela de 8s com `⌘Z` — substitui confirmação.
- **Warning:** ícone + frase + borda `--text-system`. Sem cor.
- **Erro inline:** ícone + frase específica em `--danger`. Nunca "algo deu errado".
- **Carregando:** blocos estáticos em três luminâncias, **sem pulsar**, e só acima de 300ms.
- **Empty state:** uma frase, sem ilustração e sem ícone. Três tipos: conquista ("Inbox limpa."), busca vazia (mostra o termo), primeiro uso (explica a superfície em uma frase).
- Não existe: toast no topo, spinner em ação rápida, skeleton animado, alerta de sucesso, som, badge vermelha.

---

## 7. Motion

| Evento | Duração | Easing | Propriedade |
|---|---|---|---|
| Hover, press | 60–90ms | linear | cor, opacidade |
| Check, select | 120–160ms | `--ease-state` | fill |
| Overlay entra | 140–180ms | `--ease-enter` | opacity + 4px Y |
| Overlay sai | 80–100ms | `--ease-exit` | opacity |
| Item se move (FLIP) | 180ms | `--ease-state` | transform |
| Troca de Workspace | 220ms | `--ease-enter` | barra 20px X + cross-fade |
| Primeira abertura | ≤400ms | `--ease-enter` | 1x por sessão |
| Reduced motion | 0–80ms | linear | só opacity |

Regra acima das outras: **nada que acontece muitas vezes por dia pode passar de 200ms.** Zero bounce, zero overshoot, zero stagger em lista longa. Se alguém percebe a animação, ela está longa.

Motion comunica origem e destino: um Capture que entra na Inbox sai do campo e chega na contagem — o número muda no fim do percurso, não no clique.

---

## 8. Hermes (camada de inteligência)

Escriba e bibliotecário. Personalidade no comportamento, **zero na representação**.

- Presença = barra de 2px em `--signal-fill` marcando o que o sistema produziu.
- **Nunca bloqueia:** Enter salva na hora, com ou sem interpretação pronta. Se a interpretação demora mais de ~200ms, ela chega depois, na Inbox.
- **Sempre reversível:** todo campo interpretado é editável com Tab e desfazível com `⌘Z`.
- Bibliotecário mostra **síntese + as fontes reais** (itens clicáveis com data), nunca só a resposta.
- Não existe: primeira pessoa, saudação, bolha, avatar, animação de "pensando", janela chamada "Assistente", sugestão não solicitada dentro do fluxo de captura.

### Voz
Voz **não é um modo, é uma forma de digitar**. Desktop: segurar `⌥` enquanto fala (não alternar). Mobile: toque para começar, toque para terminar.
Estados: repouso (três traços apagados) → ouvindo (traços em sódio reagindo à amplitude + timer) → transcrevendo (palavras provisórias em `--text-system`, confirmadas em `--text`) → interpretado (idêntico ao Capture digitado) → falhou (linguagem de warning, campo continua utilizável).
Soltar a tecla encerra a captação, **não a ação** — salvar continua sendo Enter. Nunca salvar áudio como resultado: o que fica guardado é texto.

---

## 9. Superfícies do produto e densidade

| Superfície | Densidade | Nota |
|---|---|---|
| Quick Capture | muito baixa | overlay de 640 sobre outro software; nunca tela cheia |
| Home | baixa | Capture na primeira posição; Today, Inbox, em andamento |
| Projects | moderada | Row de 56 com progresso |
| Kanban | alta | Row de 30; metadata só no ativo/hover; corta em "+ N mais" acima de ~20 por coluna; nunca scroll dentro de coluna dentro de página |
| Search / Command | alta | scanning; tipo do resultado em mono à esquerda |
| Library | editorial | licença de **escala**, não de linguagem: imagem 4:3, coluna 320, gutter 20, título 14 + uma linha de mono. Herda todo o resto sem exceção |

Mobile é companion, não desktop comprimido: capturar, Today, Inbox, compartilhar de outro app, voz. **Não faz** Kanban, Library visual, organização de Projects, Workspaces. Sem bottom tab bar — hierarquia tipográfica, alvo mínimo 44px, linhas de 52px.

---

## 10. Acessibilidade

- Nenhum estado depende apenas de cor (warning e erro sempre com ícone + frase).
- Foco visível em ordem lógica, com o mesmo anel em todo componente.
- Contraste mínimo: texto de sistema em ~3.4:1; nunca usar cinza abaixo de `--text-system` para informação legível.
- `prefers-reduced-motion`: só opacidade, zero translate, zero FLIP — e o produto continua completo.
- Todo fluxo principal (navegar, capturar, mover no board, buscar) operável sem mouse. Setas navegam, `⌥←→` move de coluna, espaço seleciona múltiplos.

---

## 11. Checklist antes de considerar um componente pronto

1. Cinco estados existem: repouso, hover, focus, ativo, bloqueado.
2. Funciona nos dois modos, sem inversão automática.
3. Operável por teclado, foco visível em ordem lógica.
4. Nenhum estado depende apenas de cor.
5. Sobrevive a texto 3× maior e a conteúdo vazio.
6. Íntegro com reduced motion e sem blur.
7. Usa **somente** valores dos tokens.

---

## 12. Ordem de implementação

1. **Tokens** (`mos-tokens.css`) — cor nos dois modos, tipografia, spacing, radius, stroke, durações. Nada de componente antes disso.
2. **Capture e Command** — são o produto. Se ficarem bons, o resto tem referência.
3. **Row e Panel** — cobrem Home, Inbox, Projects, Library e Search com duas peças.
4. **Navigation e overlays** — rail, linha de contexto, drawer, menu.
5. **Controls e feedback** — só aparecem em formulário e em erro; por isso vêm por último.

---

## 13. Lista negra (o que nunca deve aparecer no código)

Glassmorphism · blur decorativo · gradiente em superfície ou botão · glow · neon · partículas · sparkles em ação de IA · orbe de assistente · cérebro ou rede neural como ícone · card dentro de card · grade de métricas sem necessidade · badge colorida · toast no topo · spinner em ação rápida · skeleton animado · sombra para separar conteúdo · pill · caixa-alta em botão · ícone + texto em botão · dois primários na mesma tela · emoji na interface · animação acima de 200ms em ação recorrente · qualquer cor de accent que não seja o sódio.
