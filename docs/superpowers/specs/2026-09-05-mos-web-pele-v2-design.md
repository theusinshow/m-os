# M/OS de bolso v1.0 — a pele que se vê

**Origem:** artboards `M-OS de Bolso v1.0 - Artboards.dc.html`, no Claude Design
(projeto "Exploração de identidade e direção de arte"). Cinco telas desenhadas:
Home cheia, Home vazia, Home arrumando, Agenda lista, Agenda mês. As demais
telas — Horas, Acadêmico, Fazer, Capturar, Lembretes, Mais — não têm artboard e
são extensão da mesma linguagem.

**A queixa que originou tudo**, nas palavras do dono: o app é *pobre de
informação*, *feio e sem personalidade*, e *parado*. Não citou navegação; a
navegação entra assim mesmo, porque o calendário estar inalcançável não é gosto.

## 1. A tipografia nunca carregou

O CSS pedia `Schibsted Grotesk` e `JetBrains Mono` e **nenhum arquivo de fonte
existia no projeto** — sem `@font-face`, sem link, sem nada em `public/`. O
iPhone caía em Helvetica e no mono do sistema desde o primeiro dia. Boa parte da
queixa de "sem personalidade" era isso: a tipografia que o app pensava ter nunca
esteve na tela.

Agora as duas famílias são **servidas pelo próprio binário**
(`ui/public/fontes/*.woff2`, subconjunto latino, variáveis, 108 KB somados). Não
por CDN: o app abre atrás de autenticação numa VPS, e uma fonte de terceiro seria
uma requisição a mais, um vazamento de quem abriu o app e uma dependência que o
modo avião derruba. Elas entram no `rust-embed` como o resto da PWA.

- **Bricolage Grotesque** substitui a Schibsted no conteúdo. Mesma família
  neutra, com caráter nos pesos altos.
- **JetBrains Mono** deixa de ser só rótulo e vira **o corpo do dado**. Dígito
  tabular alinhado coluna a coluna é a única razão de pôr números lado a lado.

## 2. Duas profundidades, não uma

Entra `--surface-card: #14181B` entre o canvas e o card levantado, e a hairline
`--border-hair: #262E33`. A paleta não muda: sódio segue sendo o único acento com
direito de puxar o olho, e o verde do "em dia" segue dessaturado.

Raio do card vai de 8px para 12px. É a única mudança de forma.

## 3. A barra ganha a Agenda e perde o Capturar

De `Home · Capturar · Inbox · Tasks · Mais` para:

```
HOME    AGENDA    [ ✎ ]    FAZER    MAIS
```

Três decisões dentro disso:

- **Capturar vira o botão central em sódio**, 52px. É a razão de existir do app;
  um destino igual aos outros o punha em pé de igualdade com "Mais".
- **Inbox e Tasks fundem em FAZER**, uma tela com as capturas por triar em cima e
  as tasks embaixo. O badge soma as duas (3 capturas + 2 tasks = 5). São a mesma
  pergunta — *o que está aberto?* — e ocupavam dois dos cinco lugares.
- **A Agenda entra na barra.** Ela existia e só era alcançável por dentro de
  "Mais": quem não soubesse que existe nunca descobriria.

## 4. Densidade: cada cartão diz mais sem dizer outra coisa

O que muda na Home, e por quê:

- **Cabeçalho com o dia** (`Quarta, 9 de set` + `SEM 37`). A Home abria em
  números sem dizer de quando eles são.
- **HORAS ganha sparkline da semana** — sete barras, a de hoje em sódio, as
  futuras em traço apagado. Responde *onde foi o tempo* antes de ler o número.
- **TASKS ganha barra de progresso** — fração das feitas sobre o total.
- **ÚLTIMA CAPTURA ganha o "há 12 min"**, que é o que decide se ela ainda importa.
- **ACADÊMICO ganha a distância** (`· 2d`) junto do título.

A regra que sobrevive intacta: **cartão sem o que dizer não aparece**, e o SYNC é
a única exceção — "em dia" é a resposta à pergunta que traz alguém ao app na rua.

## 5. Agenda: lista e mês

A lista ganha uma **faixa de dias rolável** no topo (nove dias, com ponto por
dia: cinza para registro, sódio para o que cobra). O que já passou continua em
traço apagado.

O **mês** é novo: grade de sete colunas, até três pontos por dia, anel de sódio
no dia escolhido, e a lista daquele dia embaixo. Tocar num dia troca a lista.

## 6. Arrumar deixa de ser um botão de 11px

Entra por **pressão de 300 ms** sobre um cartão — o gesto que o iOS já ensinou
para "arrumar a tela de início". O cabeçalho vira sódio e diz `ARRUMANDO`, o
cartão sob o dedo sobe 2% e inclina 1,2°, os outros baixam para 92% de opacidade,
e a reordenação é arrasto de verdade com FLIP (**Motion One**, ~18 KB — a única
dependência nova, aprovada explicitamente).

Esconder continua existindo, agora como um alvo no próprio cartão. O SYNC não
sai da Home: é a resposta que traz alguém aqui.

## 7. Movimento

Tudo em CSS, exceto o FLIP do arrumar. Nada de WebGL — Aurora, Threads, Ribbons
e Silk do React Bits ficam de fora por peso, e o app abre no 4G.

| Onde | O quê |
|---|---|
| Números da Home | sobem de 0 ao valor na primeira pintura (CountUp) |
| Barras da semana | crescem de baixo, 40 ms de escada |
| Sync traz algo novo | o dígito rola na vertical e a hairline pisca em sódio 140 ms — nunca o fundo |
| Home vazia | a frase entra palavra a palavra (BlurText) |
| Itens de lista | escada de 30 ms por linha (AnimatedList) |
| Troca de dia no mês | cross-fade de 140 ms |
| Task marcada | o tique cresce de 60% a 100% |

O que já passou **não anima**: rastro não pede atenção.

Toda animação respeita `prefers-reduced-motion`. Não é acessibilidade
decorativa: o app abre em movimento e quem enjoa em movimento abre o app do mesmo
jeito.

## O que este spec NÃO decide

As telas sem artboard (Horas, Acadêmico, Fazer, Capturar, Lembretes, Mais) ganham
a linguagem — fonte, superfície, raio, movimento — mas o desenho delas é meu, e
deve ser revisto quando houver artboard.
