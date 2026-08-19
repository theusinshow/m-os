# O leque — o rail volta a oito e os recém-chegados ganham um gesto — Design

**Status:** aprovado para plano de implementação

**Data:** 2026-08-19

**Baseline:** M/OS `v0.3.0` no commit `0d73903`. Shell e rail em `apps/desktop/src/App.tsx`, tokens em `packages/design-system/tokens.css`, arranjo da Home em `homeLayout.ts` e nas migrations `0017`–`0019`.

**Origem:** esboço do proprietário — um ponto grande em sódio no rodapé ao centro, com cinco pontos menores abertos em leque acima dele, descrito como *"menu radial com ramificações"*. Em conversa ficou definido que "ramificações" é o **formato**, e não um segundo nível.

**Revisa:** ADR-038, ADR-039 e ADR-044, por meio da ADR-045 que este desenho exige.

## 1. Objetivo

Devolver ao rail um tamanho que se leia de relance, e dar aos destinos que saem dele um gesto próprio — de mão, não de teclado.

## 2. Por que existir, se o Ctrl+K já é um lançador

Esta é a pergunta que o desenho precisa responder antes de qualquer pixel, porque o `CommandSurface` já busca captures, tasks, projects, workspaces, apps, resources e functions. Um segundo caminho para o mesmo destino é peso, não recurso.

A diferença é **evocar contra reconhecer**. O Ctrl+K exige saber o nome e digitá-lo: é recall, e é ótimo para o acervo inteiro, que é grande e muda o tempo todo. O leque é memória muscular: sempre os mesmos alvos, nos mesmos ângulos, num gesto só, sem ler nada. Ele não compete com o Ctrl+K — ele cobre as cinco coisas que não valem uma frase digitada.

**A consequência dessa razão é uma restrição, e ela manda no resto do desenho:** o leque só é mais rápido enquanto for estável. Se as pétalas se reordenarem sozinhas — por uso recente, por relevância, por qualquer heurística — o alvo se move debaixo da mão, a memória muscular morre, e o que sobra é um Ctrl+K pior, com menos alcance e sem busca. Por isso as pétalas são **fixas e escolhidas pela pessoa**, e por isso o número de slots é **constante**: fixar uma sexta pétala mudaria o ângulo das outras cinco, que é a mesma falha por outro caminho.

## 3. Escopo

**Dentro:**

- o rail volta a oito destinos, com três grupos que significam algo;
- `Leque.tsx` — a âncora e as cinco pétalas, no rodapé ao centro da coluna principal;
- `leque.ts` — módulo puro com a geometria dos ângulos e a resolução do padrão, coberto por testes de nó;
- migration `0021_radial_pins.sql` e repositório de leitura/escrita;
- as três portas que faltam na Home: `openFinancePage`, `openCalendarPage`, `openMeetingsPage`;
- ADR-045, revisando 038, 039 e 044;
- correção da linha duplicada da ADR-044 no índice do `DECISIONS.md`.

**Fora:**

- segundo nível de pétalas — decidido pelo proprietário, e §2 explica por quê;
- leque por Workspace — a coluna nasce no banco, o comportamento não vem agora;
- atalho de teclado próprio para abrir o leque; a âncora é o único gatilho nesta versão;
- reordenar pétala por arrasto; nesta versão troca-se o conteúdo de um slot, não a posição dele;
- qualquer mudança no `CommandSurface`.

## 4. O rail volta a oito

Saem **Calendário**, **Finance** e **Reuniões** — os três últimos a entrar (ADR-038, 039 e 044). Os oito restantes se reagrupam pelo vocabulário que a própria ADR-038 fixou ao definir o que é item de rail: *"Library é memória, Inbox é a entrada dela, Workspaces é a lente sobre tudo, e Tempo é de onde sai a renda."*

| grupo | destinos |
|---|---|
| GERAL | Home, Hermes |
| TRABALHO | Tasks, Projects, Tempo, Workspaces |
| MEMÓRIA | Inbox, Library |

O agrupamento de hoje tem GERAL com três, TRABALHO com **sete** e MEMÓRIA com um. Sete itens sob um rótulo é uma lista, não um grupo — o rótulo para de informar. E Inbox está em GERAL, longe da Library que ele alimenta, enquanto Workspaces — a lente sobre tudo — fica enterrado no meio da lista de sete. A mudança é de significado antes de ser de tamanho.

## 5. O leque

**Onde.** Rodapé ao centro da **coluna principal**, e não da janela: sobre o rail ele competiria com a navegação que acabou de encolher, e o rail é justamente o que ele não é.

A âncora é `position: absolute` sobre o conteúdo, e **não** ocupa espaço no fluxo — uma faixa permanente roubaria altura de todas as páginas para servir a um gesto. Em troca, a `.page-surface` ganha um `padding-bottom` da altura da âncora, para que o fim de uma lista longa nunca fique escondido embaixo dela. Sem esse par, "sobrepor" vira "esconder".

**O rodapé já é disputado, e o centro é o que sobrou.** O recibo de desfazer (`.receipt`) é `fixed` no canto inferior esquerdo, logo após o rail; o toast de atenção (`.attention-toast`) é `fixed` no canto inferior direito. Nenhum dos dois passa pelo centro, então a âncora fechada convive com os dois sem negociação.

Aberto, o leque é outra história: as pétalas sobem num arco e, em 840px, o arco chega perto do recibo. A regra é **o leque por cima**, com `z-index` acima de `--z-receipt`. O critério não é hierarquia visual, é intenção: o leque só está aberto porque alguém acabou de clicar nele, enquanto recibo e toast aparecem sozinhos. O que a pessoa pediu agora cobre o que o sistema ofereceu sozinho — e os dois são transitórios de qualquer forma.

**Gesto.** A âncora fica sempre visível. Clique abre; clique de novo, `Esc`, ou clique fora fecha. Foi escolhida contra o hover porque o rodapé é caminho de passagem, e um menu que se abre quando o cursor só atravessa é um menu que interrompe.

**Um nível.** Cada pétala dispara direto. Três tipos:

| tipo | o que faz | exemplo |
|---|---|---|
| `app` | abre o app registrado | M-Finance |
| `acao` | dispara sem sair da tela | Quick Capture |
| `pagina` | navega | Reuniões |

**Slot vazio** aparece como contorno, e clicar nele abre o seletor do que fixar. É assim que a feature se ensina: sem tutorial, sem estado escondido, e sem que o primeiro uso seja um leque vazio que não explica o que quer.

**Teclado.** A âncora é botão focável com `aria-expanded`. Aberto, as pétalas vivem num `role="menu"`; setas percorrem, `Enter` dispara, `Esc` fecha e devolve o foco à âncora. Ordem de foco igual à ordem angular, da esquerda para a direita — a mesma que os olhos leem.

**Movimento.** As pétalas abrem escalonadas a partir da âncora, dentro do orçamento da ADR-034. Com `prefers-reduced-motion: reduce` elas aparecem no lugar, sem percurso.

## 6. O padrão de fábrica, que é o que resolve o órfão

Os cinco slots já nascem preenchidos. A numeração é a do banco, **base zero**, da esquerda para a direita no arco:

| slot | pétala | `kind` |
|---|---|---|
| 0 | Calendário | `pagina` |
| 1 | Finance | `pagina` |
| 2 | Reuniões | `pagina` |
| 3 | M-Finance | `app` |
| 4 | Quick Capture | `acao` |

Os três que saem do rail chegam ao leque **no mesmo movimento**. Isso não é conveniência: é o que a ADR-038 fez quando tirou Apps do rail — ela acrescentou o botão no widget APPS da Home *junto com* a saída, registrando que sem ele *"a pagina ficaria inalcancavel"*.

Por isso as três portas da Home entram nesta mesma mudança. A `HomePage` recebe hoje `openInbox`, `openTasksPage`, `openTempoPage`, `openProjectsPage`, `openLibraryPage` e `openAppsPage` — e nenhuma equivalente para Finance, Calendário ou Reuniões. Sair do rail sem elas deixaria os três dependendo do Ctrl+K, que é recall, que é exatamente o que a ADR-031 registra ter falhado com Workspaces.

**M-Finance ocupa o quarto slot porque é o único app abrível que existe.** Dos cinco cadastrados, só ele tem `launch_kind` e `can_open`; NexoDoc, ChronoCAD, Screenshot Tool e KNOW/OS estão registrados sem alvo de abertura. Fixá-los hoje daria pétalas que não fazem nada.

## 7. Dados

Migration `0021_radial_pins.sql`:

```sql
CREATE TABLE radial_pins (
    workspace_id TEXT REFERENCES workspaces(id) ON DELETE CASCADE,
    slot INTEGER NOT NULL CHECK (slot >= 0 AND slot <= 11),
    kind TEXT NOT NULL CHECK (kind GLOB '[a-z][a-z0-9_]*'),
    target TEXT NOT NULL,
    created_at TEXT NOT NULL
) STRICT;

CREATE UNIQUE INDEX radial_pins_escopo
    ON radial_pins (COALESCE(workspace_id, ''), slot);
```

Quatro escolhas, todas herdadas das migrations `0017` e `0018` de propósito:

**Tabela vazia significa "o que o desenho escolheu", e não "nada fixado".** É a mesma inversão que a 0017 documenta para `section` e `span`: ausência de valor é o padrão do desenho. A consequência desejada é que mudar o padrão de fábrica alcança todo mundo que ainda não personalizou, e que personalizar um slot não congela os outros quatro.

**`kind` é string opaca.** Um enum aqui faria de cada tipo novo de pétala uma migration; o `CHECK` garante forma, não vocabulário — a mesma razão que a 0017 dá para `widget_id` e `section`.

**`slot` aceita até doze, embora o desenho use cinco.** O banco guarda "qual das posições", que é forma; **quantas** posições a interface oferece é vocabulário, e a 0017 registra que vocabulário muda mais rápido que migration. Ir a seis pétalas um dia não custa migration.

**`workspace_id` nasce nullable, com `NULL` significando "Todos".** Cópia direta da 0018, e pelo mesmo motivo: o índice único sobre `COALESCE(workspace_id, '')` fecha o buraco de o SQLite aceitar `NULL` repetido em PRIMARY KEY. Com isso, "um leque por Workspace" depois é comportamento novo e não estrutura nova.

## 8. Riscos

**Reuniões sai do rail com dois dias de vida.** Ela nasceu no commit `bafbfb5` (19/08) e nunca chegou a formar hábito — tirá-la agora é decidir sem evidência de uso, que é o oposto do que a ADR-038 fez com Apps. Mitigação em duas camadas: ela entra fixada no leque por padrão, e a **barra de gravação continua na topbar**, que é onde mora a promessa da §17.2 do `MEETING-AGENT.md` — a indicação de que o microfone está aberto nunca dependeu do rail.

**O leque pode virar um segundo rail.** Se pétalas forem sendo acrescentadas, ele deixa de ser gesto e vira lista. O teto de cinco é a defesa, e a ADR-045 precisa registrá-lo como teto e não como ponto de partida.

## 9. ADR-045

**"O rail volta a oito, e o recém-chegado nasce no leque."** Revisa 038, 039 e 044.

O teto do rail foi de seis a oito (031), nove (036), dez (038), onze (039) e doze (044) — cinco revisões em pouco mais de duas semanas, cada uma argumentando bem o seu caso e nenhuma segurando o conjunto. A regra nova troca o teto móvel por um caminho: **destino novo nasce no leque; ele só sobe ao rail quando provar ser renda ou memória**, pelo critério que a ADR-036 já tinha escrito. O leque deixa de ser só um gesto e passa a ser o degrau que faltava.

## 10. Verificação

**Nó, em `leque.test.ts`:** a resolução do padrão (tabela vazia devolve os cinco de fábrica; um slot gravado substitui só aquele); a geometria dos ângulos (cinco posições estáveis, simétricas em torno da vertical, independentes de quantos slots estão preenchidos); e o comportamento do slot vazio.

**Rust:** repositório com testes de escrita, leitura e escopo, no molde do `workspace_widget_layout` — incluindo o caso que a 0018 existe para cobrir, que é `workspace_id` nulo não aceitar duas linhas no mesmo slot.

**Gate visual, pela skill `ver-o-app`:** foto da janela real nos dois temas, em 1280 e 840px, com o leque aberto e fechado; navegação completa por teclado; e `prefers-reduced-motion` conferido de fato, não presumido.

**O que este desenho não consegue verificar:** se o leque de fato substitui o rail no uso diário. Isso só aparece depois de uma semana de uso real, e a ADR-045 deve dizer o que fazer se não substituir.
