# Argos — a face do estado do M/OS — Design

**Status:** aprovado para plano de implementação

**Data:** 2026-08-17

**Baseline:** M/OS `v0.2.11` + ADR-040 (geometria macia dos widgets). Shell em `apps/desktop/src/App.tsx`, cliente do Hermes em `hermes.ts`, símbolo do sistema em `Symbol.tsx`.

**Origem:** referência trazida pelo proprietário — `https://bloub.vercel.app/`, um gerador de avatar SVG animado com oito formas, vinte expressões e uma linha do tempo de estados. Da referência foi adotado o **mecanismo** e recusado o **desenho**, pelas razões de §3.

**Revisa:** nada. Convive com `UX-PRINCIPLES.md` §16 e `HERMES-PREMIUM-CHAT.md` §7.6 por meio da distinção que a ADR-041 fixa.

## 1. Objetivo

Dar ao M/OS uma presença que mostre, em qualquer tela, o que o sistema está fazendo agora — usando uma criatura cujas poses são fatos, e não enfeite.

Argos é a **face do M/OS**, não do Hermes. Ele mostra boot, ocupado, cronômetro e conexão tanto quanto mostra o assistente trabalhando. É essa amplitude que o separa de um avatar de IA.

## 2. Escopo

**Dentro:**
- `argosPose.ts` — módulo puro com a precedência de poses e a geometria dos olhos, coberto por testes de nó;
- `Argos.tsx` — o desenho, ~22px, na topbar, ao lado do estado de sistema;
- seis poses, todas ligadas a sinal existente hoje;
- ADR-041, registrando por que isto não contradiz o §16 e o §7.6.

**Fora:**
- **dormir por inatividade real** — o sinal mora no Rust, no monitor da ADR-037; ver §8;
- qualquer forma de acúmulo, histórico ou "saúde" do bicho (decidido pelo proprietário: espelho do agora);
- notificação, badge ou ponto — o M/OS não tem domínio de notificação;
- interação: Argos não é clicável nesta versão;
- personalização de forma e cor pelo usuário, que é o que o bloub oferece e o M/OS não precisa;
- qualquer presença de Argos **dentro da superfície do Hermes**, onde avatar é proibido por requisito herdado.

## 3. O que a referência deu, e o que foi recusado

**Recusado: o desenho.** O autor do bloub escreve que *"The MIT licence covers the code in this repository, not the design it imitates"*, e junto: *"Not affiliated with, endorsed by or connected to x.ai"*. O blob dele é uma reimplementação do mascote da x.ai. Copiá-lo seria vestir o M/OS com a cara de outro produto — e cair exatamente no *"interfaces que parecem demos de IA"* do §16. Ele também é Vue 3, então reuso direto de componente nunca esteve na mesa.

**Adotado: o mecanismo.** Três ideias, todas livres:

1. **uma silhueta preenchida, e a expressão inteira nos olhos.** As vinte expressões do bloub são a mesma forma com olhos diferentes;
2. **estados como dado**, e não como animação escrita à mão uma a uma;
3. **motor puro e sem relógio** — `engine.sample(t)` é função do tempo. É o mesmo padrão do `plotGeometry.ts`: a matemática num módulo testável, o componente só desenhando.

## 4. A criatura

### 4.1 Silhueta

**Quadrado de cantos macios, não círculo.** O M/OS já tem geometria própria: o símbolo do rail é um squircle de raio 5px em campo sódio com a barra inclinada dentro. Argos herda a **família geométrica** — corpo de ~22px com `--radius-lg` (8px), a maciez que a ADR-040 acabou de estabelecer — e **não** herda a marca: nunca leva fundo sódio nem a barra dentro, porque isso transformaria o logotipo em bichinho.

Não ser um círculo também é o que o afasta visualmente do blob da referência.

### 4.2 Olhos

Duas cápsulas, e a expressão inteira é a variação de `{ x, y, rx, ry, tilt }` nelas. A 22px isso não é simplificação: é a única coisa que se lê.

### 4.3 Cor — três pesos, cada um com significado

| Peso | Token | Quando | Significa |
|---|---|---|---|
| Repouso | `--text-system` | nada acontecendo | o mesmo peso do texto de estado ao lado; Argos não pesa mais que a topbar |
| Atento | `--text` | o sistema está fazendo algo | clareou porque há o que acompanhar |
| Chamando | `--signal-ink` | o sistema precisa de você | o único caso em que ele puxa o olho |

`--signal-fill` **não entra**: a ADR-034 o reservou para carga, e Argos não mostra carga. `--signal-ink` é a tinta legível que a topbar já usa (`.system-state .mos-symbol`), então o terceiro peso não inventa cor nenhuma.

## 5. O repertório e a precedência

| Pose | Olhos | Peso (§4.3) | Sinal | Origem |
|---|---|---|---|---|
| **Desperto** | abertos, neutros | repouso | nada acontecendo | ausência dos demais |
| **Trabalhando** | deslocados para o lado, pálpebra baixa | atento | Hermes gerando ou executando ferramenta | `delta`, `reasoning`, `tool{running:true}` |
| **Encarando** | arregalados, fixos no centro | **chamando** | o sistema precisa de você | `approval`, `clarify` |
| **Concentrado** | semicerrados | atento | cronômetro correndo | `timer.status === "running"` |
| **De olhos fechados** | duas linhas | atento | boot ou operação em curso | `bootState === "loading"`, `busy` |
| **Assustado** | arregalados, deslocados | **chamando** | falha | `bootState === "error"`, `failed` |

Os dois pesos `chamando` são as duas poses em que o sistema não continua sozinho: ou ele espera sua resposta, ou ele quebrou.

**Conexão offline não assusta Argos.** Hermes nunca configurado é `offline`, e é um estado perfeitamente normal — se ele contasse como falha, Argos ficaria permanentemente aterrorizado em qualquer instalação sem gateway. A conexão já é dita em texto pelo SystemHealth, e por isso não entra em `ArgosSignals`.

**Precedência — quem precisa mais de você ganha:**

```
encarando  >  assustado  >  trabalhando  >  de olhos fechados  >  concentrado  >  desperto
```

`complete` devolve ao repouso. A precedência é função pura e é onde ela vira verificável (§6.1).

## 6. Arquitetura

### 6.1 `argosPose.ts` — puro e testado

Mesmo padrão do `plotGeometry.ts`, e pelo mesmo motivo: `vitest.config.ts` roda só funções puras em ambiente de nó, e é aqui que mora o que pode mentir.

- `type ArgosSignals = { hermes: "idle" | "working" | "waiting" | "failed"; busy: boolean; boot: "loading" | "ready" | "error"; timerRunning: boolean }`
- `poseFor(signals: ArgosSignals): ArgosPose` — a precedência de §5;
- `type Eye = { x: number; y: number; rx: number; ry: number; tilt: number }`
- `eyesFor(pose: ArgosPose): { left: Eye; right: Eye }` — a geometria.

### 6.2 `Argos.tsx` — só desenha

Assina `hermes.onEvent()`, que já é global. Não assina `onState`: a conexão não entra no repertório (§5). Recebe `busy`, `bootState` e `timerRunning` como props. Nenhuma decisão mora aqui.

O estado de streaming e de aprovação **não precisa ser levantado do `HermesPage`**: `hermes.onEvent()` entrega `TurnEvent` no barramento `hermes-event` para quem assinar.

### 6.3 A restrição de segurança

**Argos só escuta. Nunca responde.** Em particular, nunca chama `hermes.approve`.

Isto não é zelo abstrato: o comentário do próprio `hermes.ts` registra o problema que a endereçagem de evento veio corrigir — *"sem isto, duas superfícies assinando o mesmo barramento dividiam a mesma resposta entre si"*. E a ADR-024 fixou que Hermes é superfície, não segundo agente. Um bicho que respondesse seria um terceiro respondente.

### 6.4 Onde mora

Topbar, à direita, ao lado do bloco `.system-state` — que já é, hoje, um espelho de estado do sistema naquele exato lugar. A adição é coerente com o que já está ali, em vez de nova.

CSS em `App.css`, e **não** em `packages/design-system/widgets.css`: aquele arquivo é compartilhado com quatro apps do monorepo, e Argos é cromo do shell do M/OS.

## 7. Movimento e acessibilidade

**Argos não tem loop, e isso é regra.** A ADR-034 dá um loop por tela e o sistema já gastou o dele: a barra inclinada girando quando está ocupado — que o `Symbol.tsx` declara ser *"o único spinner do sistema. Não existe círculo, não existem três pontos."*

Disso decorrem duas coisas:

- **não há piscada ociosa.** Uma piscada periódica não carrega dado; é enfeite, e enfeite é o que o §16 proíbe. O movimento de Argos é a *transição entre poses*, que só acontece porque um fato mudou;
- **nenhuma pose pode girar nem usar três tempos**, sob pena de virar um segundo indicador de ocupado competindo com o símbolo.

Com `prefers-reduced-motion`, a pose troca sem interpolar — a fonte única em `tokens.css` já zera os tokens de duração.

**`aria-hidden="true"`.** Os mesmos fatos já são anunciados em texto pelo estado de sistema ao lado e pela página do Hermes. Argos é redundante por construção, e é isso que o torna seguro de esconder: um leitor de tela não deve ouvir a mesma coisa duas vezes.

Em `forced-colors`, os três pesos caem nos tokens que o modo já remapeia.

## 8. O que foi recusado

**Dormir.** É a pose mais bonita da ideia, e o sinal dela — inatividade real — mora no Rust, no monitor da ADR-037. Fingir sono com um temporizador de renderer seria inventar o dado, que é o que a ADR-034 chama de pior que a ausência. Fica registrado como a razão de existir um segundo recorte, com um módulo de humor no núcleo emitindo evento.

**Histórico, saúde e acúmulo.** Decisão do proprietário: Argos é espelho do agora. Uma "saúde" exigiria inventar pesos que ninguém definiu — quantos dias de Inbox valem tristeza.

**Notificação e badge.** O M/OS não tem domínio de notificação.

**Personalização de forma e cor.** É o produto do bloub, não uma necessidade do M/OS.

**O desenho do bloub.** Ver §3.

## 9. Gates de QA

- `npm run build`, `npm test -- --run`, `npx impeccable detect src`, `git diff --check`;
- Dark e Light, nas quatro larguras (840×600, 1280×800, 1440×900, 1920×1080) — a topbar muda de densidade nas menores;
- as seis poses verificadas contra sinal real: boot, conexão caída, uma conversa do Hermes com aprovação pendente, e o cronômetro correndo;
- `reduced-motion` confirmando troca de pose sem interpolação;
- `forced-colors` confirmando os três pesos;
- árvore de acessibilidade confirmando que Argos **não** aparece nela.

A verificação em tela é do proprietário: a janela do Tauri não é legível pelo agente.

## 10. ADR-041 — o que ela decide

**Que uma criatura no shell não é AI slop, e sob quais condições.**

O `UX-PRINCIPLES.md` §16 proíbe elementos que existam *"apenas porque são associados a produtos de inteligência artificial"* — e lista "orbes decorativos". O `HERMES-PREMIUM-CHAT.md` §7.6 proíbe "avatar" e "orb" **na superfície do Hermes**, e completa: *"React Bits só entra se o efeito **for** o estado — nunca como enfeite."*

Argos passa por três razões que a ADR precisa registrar:

1. **não fica na superfície do Hermes**, onde a proibição é literal;
2. **não é avatar do assistente.** É a face do M/OS: mostra boot, ocupado e cronômetro tanto quanto mostra o Hermes. Um avatar de IA não teria pose para "o banco está abrindo";
3. **cada pose é um fato, e nenhuma existe sem sinal.** É a mesma doutrina que a ADR-034 já aplicou ao movimento — o efeito É o estado.

A ADR registra também:

- **a recusa do desenho do bloub** e sua razão de linhagem (§3);
- **a ausência de loop e de piscada ociosa** como consequência do orçamento de movimento, e não como economia;
- **a restrição de que Argos nunca responde** ao barramento (§6.3);
- **o nome.** Argos Panoptes é o vigia de cem olhos: uma criatura cuja única característica é o olhar. A repartição com Hermes é exata — Hermes fala, Argos olha — e reforça no nome a restrição de segurança. E o mito já contém o escopo: Argos é o que não dorme, e o nosso também não, até que o sinal de inatividade exista.
