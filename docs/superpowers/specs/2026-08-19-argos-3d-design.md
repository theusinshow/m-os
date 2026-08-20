# Argos ganha corpo — o bicho em 3D no canto — Design

**Status:** aprovado para plano de implementação

**Data:** 2026-08-19

**Baseline:** M/OS `v0.3.0`. Argos vive hoje em `apps/desktop/src/Argos.tsx` e `argosPose.ts`, montado na topbar em `App.tsx:3269`, com 22px e sem laço.

**Origem:** pedido do proprietário — *"deixe ele em 3d, ancorado no canto direito inferior bem maior"*. Na conversa que produziu esta spec, o proprietário escolheu, entre alternativas apresentadas: WebGL real (e não volume desenhado em SVG), corpo de **blob orgânico** (e não squircle extrudado), **~72px**, **saindo da topbar**, **migrando de canto** quando disputado, e **clicável**.

**Revisa:** ADR-041 (condições de tamanho e de laço) e o orçamento de movimento da ADR-034. A ADR-048, escrita junto com a implementação, é quem formaliza a revisão. Abre exceção nomeada contra `UX-PRINCIPLES.md` §16 e `HERMES-PREMIUM-CHAT.md` §7.6.

## 1. Objetivo

Dar ao Argos presença física: um corpo tridimensional, com volume e deformação, grande o bastante para ser lido de relance, ancorado no canto inferior direito.

O que **não** muda: ele continua sendo a face do estado do M/OS, e cada pose continua sendo um fato ligado a um sinal que já existe. A ADR-041 tinha três pilares; esta spec derruba dois (tamanho, ausência de laço) e mantém o terceiro intacto, que é o que impede a criatura de virar enfeite.

## 2. Escopo

**Dentro:**

- `argosPose.ts` cresce com `sceneParamsFor(pose)`, puro e testado;
- `argosCorner.ts` novo — a decisão de canto como função pura, testada;
- `argosScene.ts` novo — dono do WebGL, sem React;
- `Argos.tsx` vira casca: monta o canvas, empurra pose e ponteiro, trata o clique;
- saída do `<Argos>` da topbar (`App.tsx:3269`);
- o SVG de hoje sobrevive como **fallback** de WebGL indisponível;
- ADR-048, registrando a revisão e a exceção.

**Fora:**

- `.attention-toast`, `.drop-panel` e `.receipt` não são tocados — quem cede é sempre o Argos;
- `Symbol.tsx` não é tocado: a barra inclinada continua sendo o spinner do sistema;
- opção de desligar ou redimensionar o Argos — a 72px ele não exige uma; se exigir, é outra entrega;
- poses novas. As seis continuam sendo seis, pela razão de §5;
- qualquer presença de Argos dentro da superfície do Hermes, onde avatar segue proibido;
- dormir por inatividade real — continua dependendo do monitor da ADR-037, como a spec de 2026-08-17 já registrou.

## 3. O conflito, declarado antes de ser resolvido

Quatro documentos do repo proíbem, na letra, o que esta spec constrói:

| Documento | O que diz |
|---|---|
| ADR-041, condição 3 | *"Ela não tem loop nem piscada ociosa."* |
| ADR-034 (`DECISIONS.md:1056`) | *"um loop por tela, movimento que carrega [dado]"* — e o loop já foi gasto no `Symbol.tsx` |
| `HERMES-PREMIUM-CHAT.md` §7.6 | proíbe *"avatar, orb, glow permanente, partícula, spinner grande"* |
| `UX-PRINCIPLES.md` §16 | proíbe *"orbes decorativos"* e *"interfaces que parecem demos de IA"* |

Um canvas WebGL é um laço permanente por definição, e um blob de 72px é literalmente um orbe. Não há leitura em que isto não contradiga os quatro.

**A decisão é do proprietário, tomada com o conflito à vista.** Todas essas ADRs foram aceitas "por decisão do proprietário do produto", e é a mesma autoridade que as revisa. O que esta spec recusa é revogá-las em silêncio — o `UI-UX-REFINEMENT.md` §15 chama isso de mudança silenciosa, e é a falha que a ADR-048 existe para não cometer.

Sobre a forma: a ADR-041 recusou o blob da referência porque ele é reimplementação do mascote da x.ai, e vestir o M/OS com a cara de outro produto seria cair no §16 por um caminho curto. **Aquele problema era de propriedade, não de geometria.** Um blob de desenho próprio não o herda. Mas continua perto da silhueta que o §16 alerta, e isso é risco assumido — está escrito aqui para que ninguém, daqui a seis meses, precise adivinhar se foi descuido.

## 4. Arquitetura

O cérebro puro e a casca fina, que é o padrão que o repo já pratica em `plotGeometry.ts` e no próprio `argosPose.ts`. O WebGL fica numa caixa que o React não abre.

| Módulo | Papel | Testado |
|---|---|---|
| `argosPose.ts` *(cresce)* | `poseFor` como hoje, mais `sceneParamsFor(pose)` → `{ deformacao, velocidade, abertura, recuo }` | sim, nó |
| `argosCorner.ts` *(novo)* | `cantoPara({ direitaOcupada, esquerdaOcupada })` → `"direita" \| "esquerda" \| "oculto"` | sim, nó |
| `argosScene.ts` *(novo)* | canvas, câmera, luz, malha, laço. Expõe `mount / setPose / setPointer / pause / resume / dispose` | não — ver §10 |
| `Argos.tsx` *(vira casca)* | monta o canvas, empurra pose e ponteiro, trata clique e fallback | pela bancada |

`useArgosPose` fica exatamente como está: ele já assina Hermes e cronômetro e devolve pose, e nada nesta mudança toca em sinal.

**O corpo** sai de uma esfera com deslocamento no vertex shader — ruído simplex modulado pela pose. Deformar é mudar dois uniforms, não trocar geometria. É o que torna as seis poses uma interpolação e não seis malhas.

**O `three` entra por `import()` dinâmico.** O bundle do renderer hoje é 511KB; o `three` tree-shaken acrescenta cerca de 150KB, ~30%. O `UX-PRINCIPLES.md` §51 diz que "uma bela interface lenta contradiz o conceito do produto" — carregar sob demanda é o que mantém o boot no tamanho que ele tem hoje.

## 5. As poses continuam sendo fatos

Este é o pilar da ADR-041 que **não** se toca. Cada pose existe porque um sinal existe; nenhuma pose nasce de um temporizador. O que muda é a expressão, não a precedência — `poseFor` não é alterada.

| Pose | Sinal (inalterado) | Corpo | Olhos |
|---|---|---|---|
| `desperto` | nada acontecendo | respiração lenta, deformação mínima | abertos |
| `concentrado` | cronômetro correndo | comprimido, deformação baixa | semicerrados |
| `trabalhando` | Hermes streamando ou em tool | ativo, giro lento | atentos |
| `fechado` | boot carregando, ou app ocupado | contraído | fechados |
| `encarando` | Hermes pede aprovação | parado, encara | arregalados |
| `assustado` | boot falhou, ou Hermes falhou | agitado, recua | arregalados, tremendo |

A cor vem de `getComputedStyle` sobre os tokens, lida na montagem e relida na troca de tema. Nenhuma cor literal no shader: o design system continua sendo a fonte, e `--signal-fill` continua fora, reservado a carga pela ADR-034.

## 6. O canto, e a terceira posição

O canto inferior direito é disputado por `.attention-toast` (`App.css:6678`) e `.drop-panel` (`App.css:7817`), ambos em `right: var(--space-4); bottom: var(--space-4)`. O inferior esquerdo é disputado por `.receipt` (`App.css:4419`). Por isso a migração precisa de três estados, e não de dois:

| Direita ocupada | Esquerda ocupada | Argos |
|---|---|---|
| não | — | **direita** — o padrão |
| sim | não | **esquerda** |
| sim | sim | **oculto**, e volta quando vagar |

Toast e recibo são transitórios, então `oculto` é raro e curto. **Argos sempre cede:** ele nunca cobre um aviso do sistema, porque o aviso carrega dado que ele não carrega.

**A ocupação vem do estado do shell, não de medição do DOM.** O `App.tsx` já sabe quando cada superfície está aberta — `delivered` para o toast, `undo` para o recibo, e o estado da Drop Zone para o painel. `argosCorner.ts` recebe dois booleanos derivados desses, e nada lê geometria de tela. Medir o DOM traria o problema que a `verificacao-em-tela` já conhece: leitura em cache e decisão sobre dado velho.

O deslocamento entre cantos usa os tokens de duração e easing existentes. Sob `prefers-reduced-motion` ele teleporta em vez de deslizar.

## 7. O laço, que é onde a disciplina volta

Revogar a condição 3 sem pôr nada no lugar é como o Argos vira exatamente o enfeite que a ADR-041 previa. As contrapartidas são parte da decisão, não otimização posterior:

- **pausa** quando a janela perde foco, quando minimiza, e quando o Argos está `oculto`;
- **`prefers-reduced-motion` não tem laço nenhum** — um quadro por pose, e congela. A promessa da ADR-034 continua inteira para quem pediu menos movimento, e é o que impede a exceção de virar regra;
- `devicePixelRatio` capado em 2; `powerPreference: "low-power"`;
- **se o WebGL não inicializar** — driver velho, VM, sessão remota — cai para o SVG de hoje, no mesmo canto, com as mesmas poses. Por isso o desenho SVG **não é apagado**: ele muda de endereço e vira o piso.

O fallback não é zelo excessivo: o M/OS é um programa de Windows que roda em WebView2, e uma tela preta no canto seria pior do que não ter bicho nenhum.

## 8. Olhar, clique e acessibilidade

Os olhos seguem o cursor dentro de um limite angular, com o ponteiro coalescido por `requestAnimationFrame`. Desligado sob `prefers-reduced-motion`.

**O clique abre o Centro de Atenção** — `setAttentionOpen(true)`, o mesmo alvo que o `AttentionToast` já usa. É a escolha coerente: Argos é a face do estado, e clicar na face abre o detalhe do estado.

Isso tem uma consequência que precisa estar escrita. A ADR-041 justificou o `aria-hidden` assim: *"os mesmos fatos já são anunciados em texto pelo estado de sistema ao lado"*. Saindo da topbar, não há mais nada ao lado — a justificativa morre junto com a mudança de endereço. E virando controle, esconder deixa de ser opção.

Então:

- o canvas passa a viver dentro de um `<button>`;
- com nome acessível que diz a pose em palavras — *"Estado do sistema: em repouso"*, *"Estado do sistema: aguardando sua aprovação"*;
- alcançável por teclado, com anel de foco vindo dos tokens;
- o `<canvas>` em si segue `aria-hidden`: quem fala é o botão.

## 9. Topbar

`<Argos>` sai do `div.system-state` em `App.tsx:3269`. Ficam o `MosSymbol` girando, o rótulo `SINCRONIZANDO` e o `pageMeta`. Não abre buraco: a div já trata o caso vazio, porque o spinner só existe quando `busy`.

## 10. Testes

Seguindo a doutrina que o `SETUP-MAQUINA.md` §4 fixou por necessidade — a lógica mora onde o teste roda:

- `argosPose.test.ts` cresce com `sceneParamsFor` nas seis poses, incluindo os limites de `deformacao` e `abertura`;
- `argosCorner.test.ts`, novo, cobre a tabela-verdade de §6 inteira, incluindo a volta de `oculto`;
- `argosScene.ts` **não** ganha teste de unidade. WebGL em headless mente mais do que prova, e um teste que não protege nada é pior que nenhum;
- cobertura visual pela bancada Playwright com `tokens.css` e `App.css` reais, nos dois temas e nas larguras de quebra;
- e a foto da janela real via `capturar-janela.ps1`, porque a bancada reproduz a marcação à mão e pode divergir do que o React gera.

## 11. ADR-048

Escrita junto com a implementação, e ela é entregável, não nota de rodapé:

- revisa a ADR-041 nas condições de tamanho e de laço, e a marca como "Revisada por ADR-048";
- emenda o orçamento da ADR-034: um loop por tela **mais** o Argos, sob as pausas de §7;
- registra a exceção contra `HERMES-PREMIUM-CHAT.md` §7.6 e `UX-PRINCIPLES.md` §16, com o argumento de §3;
- registra o risco assumido da silhueta, com o nome de quem o assumiu.

## 12. Riscos

**O bicho come a bateria.** Mitigado pelas pausas de §7, mas não eliminado: 72px de WebGL a 60Hz custa mais que zero. Se incomodar na prática, o próximo passo é reduzir a taxa em `desperto` — a pose que ocupa 90% do tempo — e não desligar o resto.

**A criatura é mais fácil de esticar do que uma barra.** É o risco que a própria ADR-041 nomeou, e ele cresce com o tamanho: um bicho de 72px pede acessório, e acessório pede pose sem sinal. A defesa continua sendo a mesma — qualquer pose nova exige um sinal que já exista.

**O canto vira disputado por um quarto elemento.** A tabela de §6 tem três estados porque hoje há três disputantes. Um quarto exige revisitar `argosCorner.ts`, e é por isso que ele é função pura e testada, e não uma sequência de `if` dentro do componente.
