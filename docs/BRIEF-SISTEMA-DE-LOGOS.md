# Brief — Sistema de logotipos da família M/OS

Prompt para o Claude Design. Copiar do bloco abaixo.

---

Preciso de um **sistema de logotipos** para a família de softwares do M/OS. Não é um logo:
é a regra que faz quatro marcas diferentes se reconhecerem como parentes.

## O que o M/OS é

Um sistema operacional pessoal — o cérebro digital de uma pessoa só. Ele centraliza
captura, projetos, tarefas, biblioteca e as ferramentas que essa pessoa mesma constrói.
Os softwares abaixo não são produtos de mercado: são ferramentas próprias que agora vivem
dentro do M/OS.

| App | O que é | Natureza |
|---|---|---|
| **M/OS** | o sistema; captura, organiza, conecta e age | a casa |
| **CronoCAD** | rastreador de horas para projetos de desenho técnico | tempo |
| **M-Finance** | cockpit de contas, vencimentos e faturas | dinheiro |
| **Coded Atlas** | catálogo visual de projetos e gerador de assets | acervo |

A família vai crescer. O sistema precisa aceitar um quinto membro sem ser redesenhado.

## O que já existe e NÃO muda

O design system está fechado. Estes pontos são entrada, não assunto:

**A barra.** O símbolo do M/OS é uma barra sólida inclinada — um paralelogramo — em campo
sódio. Ela não é só o ícone: ela reaparece como o limiar de entrada do campo de captura
(vertical), como o filete de 2px que marca autoria do sistema nas respostas do Hermes, e
como o marcador do item ativo nas listas. **A barra é a marca.**

**Ângulo corrigido por escala, nunca um SVG escalado.** A mesma inclinação geométrica lê
mais fina conforme o desenho encolhe, então o ângulo abre para compensar:

```
1024 · 512 · 256 · 128   →  22°   polygon 38,8 53,8 26,56 11,56
64 · 48                  →  18°   polygon 40,10 54,10 24,54 10,54
32 · 24 · 16             →  14°   polygon 42,12 56,12 22,52 8,52
```

viewBox `0 0 64 64` nos três, centroide em (32,32).

**Raio do ícone ≈ 18% do lado:** 112→20px, 64→11px, 32→6px, 16→3px.

**Um acento só: o sódio `#E7C24E`.** Tinta sobre ele: `#0A0C0E`. Nenhuma cor de marca nova
entra — nem uma por app.

**Tipografia:** Schibsted Grotesk (400/500/700) e JetBrains Mono (400/500). Nada mais.

**Paleta:** canvas `#0A0C0E` · surface `#101316` · border `#1E2429` · border forte
`#2A3136` · texto `#E7EAEC` · secundário `#8C949A` · sistema `#626A70`.

**Movimento:** a barra dá meia-volta (180°). É o único movimento da marca, e é também o
único spinner do sistema — não existe círculo girando, não existem três pontos.

## O problema

O sistema precisa resolver duas coisas que puxam em direções opostas:

1. **Parentesco.** Ver os quatro juntos e saber, sem ler, que são da mesma casa.
2. **Individuação a 16px.** Distinguir CronoCAD de M-Finance na barra de tarefas do
   Windows, em 16 pixels, sem cor de marca própria e sem letra dentro de caixinha.

## A pergunta de desenho

Se a barra é constante e o sódio é constante, **o que varia?**

Explore e compare pelo menos três sistemas de variação. Candidatos, não obrigações:

- **Contagem** — uma barra, duas, três. O número diz qual app.
- **Corte** — a mesma barra cortada pela moldura em pontos diferentes.
- **Ângulo** — cada app tem sua inclinação, dentro da correção óptica por escala.
- **Campo** — sódio cheio, sódio invertido, campo escuro com barra sódio.
- **Segundo elemento** — a barra mais uma marca mínima que codifica a natureza do app
  (tempo, dinheiro, acervo).

Para cada sistema, mostre os **quatro apps juntos**, não um exemplo isolado — o sistema só
se prova no conjunto. Termine com uma **DECISÃO** explícita e o motivo.

## O que precisa vir junto

- espécimes em **escala real**: 1024, 256, 64, 32, 16;
- os quatro lado a lado, no tamanho da barra de tarefas;
- **1 bit**: tudo preto sobre branco, sem sódio. Se o sistema depende da cor para
  distinguir, ele falhou;
- lockup com o nome, em Schibsted, e a regra de espaço livre;
- comportamento do movimento de meia-volta em cada marca;
- **o teste do quinto membro**: invente um app novo plausível e mostre que a regra o
  acomoda sem exceção.

## Testes que o sistema precisa passar

- **Taskbar.** A 16px, com os quatro abertos, dá para acertar qual é qual?
- **Sem cor.** Em preto e branco o parentesco sobrevive?
- **Sem o nome.** Tirando o texto, cada marca ainda diz alguma coisa?
- **Ruído.** Sobre um fundo claro do Windows, o ícone continua legível?

## O que não fazer

Estas coisas não existem no M/OS, e não passam a existir num logo:

gradiente · glow · neon · sombra decorativa · glassmorphism · mascote · orbe · cérebro ou
rede neural como ícone · sparkle de IA · emoji · letra dentro de caixinha como sistema
inteiro · cor de acento que não seja o sódio · fonte fora das duas · círculo girando ·
ícone que só funciona grande.

## Formato

Siga o formato dos outros documentos do projeto: seções numeradas, comparações A/B com
**DECISÃO** declarada, tabela "é / não é", espécimes em escala real e uma lista final do
que ficou proibido. Português. Fundo `#0A0C0E`.
