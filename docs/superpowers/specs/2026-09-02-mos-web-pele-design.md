# A pele do bolso — carta de navegação, marca e movimento no `mos-web`

Data: 2026-09-02
Estado: aprovado, aguardando implementação
Entrega: 1 de 4 (ver "O caminho inteiro", no fim)

## O problema

O `mos-web` funciona. Ele captura, mostra a inbox, mexe em tasks e entrega
lembretes por notificação — e isso atravessa o sync e chega nos dois PCs. O que
ele não faz é **parecer** o M/OS, e não mostra quase nada do que já sabe.

Três defeitos concretos, e nenhum deles é gosto:

1. **O ícone não é a marca.** `ui/public/icone.svg` é um "M" desenhado à mão,
   campo escuro com traço âmbar. A marca do M/OS é a barra — paralelogramo
   sólido escuro em campo sódio (`BRIEF-SISTEMA-DE-LOGOS.md`). Na tela de início
   do iPhone, ao lado dos outros aplicativos, o M/OS de bolso não se identifica
   como sendo do mesmo sistema.
2. **Quatro páginas de mesmo peso.** Capturar, Inbox, Tasks e Lembretes ocupam a
   barra inteira, sem panorama nenhum. O app abre no compositor e nunca responde
   "como estão as coisas".
3. **O banco sabe muito mais do que a tela mostra.** Desde a cobertura total do
   sync (v0.3.4), o `mos-web` recebe horas do CronoCAD, projetos, acadêmico,
   diário, revisão semanal e conversas do Hermes. Tudo isso está no banco da VPS
   agora, sem uma tela sequer.

## O objetivo

Abrir o M/OS de bolso e reconhecer o sistema: a mesma marca, a mesma paleta, o
mesmo vocabulário — e um panorama antes das listas. Esta entrega para na
fronteira do que o servidor já responde; ela **não** acrescenta rota.

## A fronteira, que é uma decisão e não uma deriva

`apps/mos-web/README.md` diz que o bolso é "uma **porta** para o M/OS, e não um
segundo M/OS", e que crescer até o desktop "seria uma decisão, não uma deriva".

Esta é a decisão, escrita: **o bolso ganha panorama e consulta; o trabalho de
verdade continua no desktop.** O critério que separa os dois é o CAD aberto — o
que se faz com o desenho na frente fica no PC. Ver quanto se trabalhou, o que
vence hoje e o que caiu na inbox é justamente o que se pergunta longe da mesa.

O README será atualizado junto com a implementação, para a fronteira nova
aparecer onde a antiga estava escrita.

## 1. A marca

O brief fecha o assunto, e três pontos dele são entrada e não escolha:

- **A barra é a marca**: paralelogramo sólido `#0A0C0E` em campo sódio
  `#E7C24E`. Nenhuma cor de marca nova entra.
- **Ângulo corrigido por escala, nunca um SVG escalado.** viewBox `0 0 64 64`,
  centroide (32,32):

  | Tamanhos | Ângulo | `polygon` |
  | --- | --- | --- |
  | 1024 · 512 · 256 · 128 | 22° | `38,8 53,8 26,56 11,56` |
  | 64 · 48 | 18° | `40,10 54,10 24,54 10,54` |
  | 32 · 24 · 16 | 14° | `42,12 56,12 22,52 8,52` |

- **Raio ≈ 18% do lado**: 512→92px, 192→35px, 180→32px, 64→11px, 32→6px.

Arquivos trocados: `ui/public/icone.svg`, `icone-maskable.svg`, `icone-180.png`,
`icone-192.png`, `icone-512.png`, e as cópias em `static/`. O maskable mantém a
barra dentro do círculo de segurança (80% do lado), porque o Android recorta.

A barra também reaparece **dentro** do app, como o brief manda: filete vertical
no limiar do compositor, e marcador do item ativo nas listas.

## 2. A carta de navegação

Barra inferior de cinco destinos:

```
⌂ Home   ✎ Capturar   ▤ Inbox   ☑ Tasks   ⋯ Mais
```

- **Home** é o hub: cartões de estado que levam às páginas.
- **Mais** é uma página-índice, e **não** uma gaveta. Hoje lista Lembretes e
  Ajustes (notificação, sessão, sync). Amanhã recebe Horas, O dia e Acadêmico.

Por que índice e não gaveta: gaveta esconde o novo atrás de um gesto que
ninguém descobre sozinho, e o preço aparece como "o app não tem isso".

Por que Lembretes sai da barra: ele é destino de notificação — chega-se nele
pelo aviso que tocou, não por varredura. A barra guarda os cinco alvos que o
polegar procura sem motivo externo.

**Sem router.** A página continua sendo estado do React, como hoje. Um router
traria histórico e URL para um app que roda em tela cheia na Tela de Início,
onde nenhum dos dois aparece.

## 3. A Home

Nasce nesta entrega com o que a API já responde — `/api/estado`, `/api/inbox`,
`/api/tasks`, `/api/lembretes`:

| Cartão | O que diz | Leva para |
| --- | --- | --- |
| Sync | pendentes na fila, e quando foi a última rodada | Mais |
| Hoje | lembretes que vencem hoje, e o que já passou | Lembretes |
| Inbox | quantas capturas esperam processamento | Inbox |
| Tasks | abertas, e quantas em andamento | Tasks |
| Última captura | o texto, e há quanto tempo | Inbox |

Horas do CronoCAD e acadêmico entram na **entrega 2**, com a rota de resumo.
Cartão que ainda não tem dado não aparece — um cartão vazio prometendo conteúdo
é pior que a ausência dele.

## 4. A pele

Mesma paleta, mesma tipografia. A riqueza vem de camada, não de cor nova:

- **Cartão**: `--surface-raised`, borda `--border`, sombra baixa
  (`0 1px 2px rgba(0,0,0,.4)`), raio `--radius-lg`.
- **Gradiente de topo** na Home: sódio a 6% desvanecendo em 120px. É o único
  gradiente do app.
- **Números protagonistas**: Schibsted 500, `font-variant-numeric: tabular-nums`,
  32px nos cartões. O número é o conteúdo; o rótulo é legenda.
- **Rótulo de seção**: mono 11px, `letter-spacing: .08em`, `--text-system`.
- **Nada é hover.** A regra já está no topo de `estilo.css` e continua valendo: o
  feedback de toque mora em `:active`.

## 5. O movimento

O brief é taxativo, e isso poda metade das escolhas: **a barra dá meia-volta
(180°), é o único movimento da marca e o único spinner do sistema — não existe
círculo girando, não existem três pontos.**

| Momento | O que acontece | Duração |
| --- | --- | --- |
| Troca de página | desliza 12px no sentido da barra + fade | 140ms |
| Item concluído | risca, esmaece e recolhe a altura | 200ms |
| Captura enviada | o texto sobe 8px e some; a barra do limiar dá meia-volta | 240ms |
| Lista que chega | entrada escalonada, 30ms por item, no máximo 6 | 30ms × n |
| Esperando o servidor | a barra do topo dá meia-volta, em laço | 600ms/volta |

Tudo dentro de `@media (prefers-reduced-motion: reduce)` vira corte seco — sem
deslize, sem escalonamento, sem laço. Não é acessibilidade decorativa: quem
liga essa opção costuma ligá-la por enjoo de movimento.

Sem biblioteca de animação. CSS e a Web Animations API bastam para todos os
cinco casos, e um pacote a mais num app servido por uma VPS pequena é peso que
o usuário paga no 4G.

## 6. O código

`App.tsx` tem 666 linhas e faz navegação, estado, quatro telas e a folha de
agendamento. Ele vira:

```
ui/src/
  App.tsx              casca: estado compartilhado, navegação, recados
  paginas/
    Home.tsx  Capturar.tsx  Inbox.tsx  Tasks.tsx  Lembretes.tsx  Mais.tsx
  componentes/
    Cartao.tsx  Lista.tsx  Vazio.tsx  Barra.tsx  Marca.tsx
  estilo.css           tokens e base
  telas.css            componentes e páginas
```

O estado (`capturas`, `tasks`, `lembretes`, `estado`, `avisos`) continua na
casca e desce por props. Sem contexto e sem store: são cinco valores e uma
função de recarga, e a indireção custaria mais do que resolve.

`Porta.tsx`, `Quando.tsx`, `api.ts`, `instantes.ts`, `notificacoes.ts` e
`cerimonia.ts` não mudam.

## 7. Como isto é conferido

Duas metades, porque nenhuma sozinha responde.

**Comportamento**, em vitest, como `instantes.test.ts` já faz: qual página o
cartão abre, o que a Home mostra quando cada lista está vazia, e que a marca
escolhe o polígono certo para cada tamanho.

**Aparência**, numa bancada. O `mos-web` de verdade exige sessão — `/app/*` sem
passkey cai na porta —, então fotografar o app real custa uma cerimônia inteira
antes de cada olhada. A bancada é `ui/bancada.html`, uma segunda entrada do Vite
que monta cada página com dados falsos e o CSS real, e existe **só em
desenvolvimento**: `vite build` continua com uma entrada só, e nada dela vai
para o binário.

As fotos saem do navegador dirigido pelo agente, contra o Vite: as seis páginas,
em 390px e 430px de largura, com e sem `prefers-reduced-motion`. **Nenhuma
dependência nova entra em `ui/package.json`** — o CI roda `npm ci` nessa pasta
para montar a PWA, e um Playwright ali significaria baixar navegador em todo
build por uma foto que o agente já sabe tirar.

Isso não substitui abrir no iPhone — substitui **não olhar**.

## O caminho inteiro

Esta é a entrega 1. As outras três já estão decididas em ordem, e cada uma terá
seu próprio spec:

2. **A rota de resumo** — `/api/panorama` com horas da semana, valor, o que vence
   hoje e o acadêmico próximo; a Home ganha os cartões que faltam.
3. **Consultar** — páginas de Horas (por projeto, com valor), O dia e Acadêmico,
   sobre dado que o sync já traz.
4. **Criar e registrar** — a camada de escrita: task e lembrete com mais campos,
   lançar hora trabalhada, diário do dia.

A ordem existe por uma razão: a navegação e a linguagem visual precisam estar
decididas antes das páginas novas, ou cada página nova reabriria as duas.
