# Calendário — fase 1 (retrospectivo)

**Data:** 2026-08-17
**Estado:** proposto, aguardando revisão

## O problema

O M/OS registra *quando* tudo acontece — sessão de trabalho, Task concluída,
Capture escrita, programa aberto — e não tem nenhuma superfície que responda
"o que aconteceu no dia 12?". A Home mostra o mês como densidade, mas a célula
não conta horas, não clica e não navega.

O `VISION.md` já posicionava isto: o M/OS "não é apenas um calendário", mas
"esses elementos podem existir dentro do produto — o produto é a camada que os
conecta".

## Escopo

Três fases, decididas com o proprietário. **Este spec cobre só a primeira.**

| Fase | Entrega | Domínio novo |
|---|---|---|
| **1 · Retrospectivo** | grade do mês, célula do dia, navegação, detalhe do dia | nenhum |
| 2 · Prospectivo | prazo na Task ou tipo Evento, "vence hoje" | sim |
| 3 · Externo | Google Calendar | sincronia, credencial, rede |

A ordem é 1 → 2 → 3 porque a fase 1 constrói a máquina que a 2 precisa — grade,
célula, navegação, modelo de interação — e é a única verificável contra dado que
já existe. A fase 3 vai por último porque é a única que fura o local-first.

**A fase 1 não agenda nada.** Isso é sabido e aceito: ela é a casca, e a casca
tem que existir antes do conteúdo.

## A armadilha que decide a arquitetura

O usuário trabalha de madrugada. Nas sessões reais dele: `30/07 23:31—00:21`,
`12/07 02:27—03:18`, `26/07 19:31—00:31`. O banco guarda tudo em **UTC**, e o
fuso local está 3 horas atrás.

Agrupar por dia UTC joga as noites dele para o dia seguinte. A grade mostraria
horas em dias que ele não trabalhou, **sem nenhum erro aparente** — nada
quebra, nada falha, o número só está no lugar errado.

Isso elimina a solução mais óbvia (`GROUP BY substr(at, 1, 10)` no SQL) e
define a fronteira: **o backend responde sobre INSTANTES, o renderer decide o
que é um DIA.** Ele é o único dos dois que conhece o fuso.

## Arquitetura

### Domínio — `crates/mos-core/src/calendar.rs`

Um tipo só, para as quatro fontes:

```rust
pub enum CalendarKind {
    Session,      // sessão de trabalho encerrada
    TaskDone,     // Task concluída
    TaskCreated,  // Task criada
    Capture,      // Capture registrada
    AppOpened,    // programa monitorado aberto
}
```

**Só `app_opened`, e não `app_closed`.** Abertura sugere que o trabalho começou,
que é a informação; o fechamento dobraria a contagem de marcas no dia sem
responder nada que a abertura já não tenha respondido. Os 321 eventos
importados viram ~metade disso na grade.

```rust

pub struct CalendarItem {
    pub kind: CalendarKind,
    pub at: OffsetDateTime,
    pub ends_at: Option<OffsetDateTime>,
    pub title: String,
    pub project_id: Option<ProjectId>,
    /// Zero quando o item não tem duração.
    pub seconds: i64,
    /// Zero quando o item não é hora cobrável. Calculado com `settle`.
    pub amount_cents: i64,
}
```

`amount_cents` sai de `settle`, a mesma função que produz o total do Painel e a
linha do Relatório. Não há segundo caminho de cálculo.

### Composição — `apps/desktop/src-tauri/src/calendar.rs`

A composição das quatro fontes vive na camada de comando, e **não** num serviço
novo do core. O motivo é preciso: nenhum serviço existente tem os quatro
repositórios, e a camada de comando é onde eles se encontram. Há precedente
exato — `monitoring_timeline` compõe `monitoring` e `tracking` ali mesmo, pelo
mesmo motivo.

```rust
#[tauri::command]
pub fn calendar_window(since: String, until: String) -> Result<Vec<CalendarItem>, CoreError>
```

Devolve ordenado por `at` crescente. A janela é fechada nos dois lados e vem
como instante ISO — calculada pelo renderer a partir do mês local visível.

### Tela — `apps/desktop/src/CalendarPage.tsx`

- grade do mês, **segunda como primeiro dia**, igual ao `WeekRings` e ao
  `MonthDensity`;
- navegação mês anterior / próximo / "Hoje";
- célula: número do dia, horas do dia em destaque, e **um ponto por tipo
  presente** — não um ponto por item. Três Tasks concluídas fazem um ponto, não
  três: a célula responde "houve Task aqui", e a contagem exata é o que o
  detalhe do dia existe para dar. Sem isso, um dia movimentado vira uma nuvem de
  pontos que não se conta de relance nem se lê como número;
- clicar num dia abre o detalhe: os itens daquele dia em ordem de hora;
- agrupamento por dia **local**, com `new Date(item.at)` e comparação de data
  local — nunca `toISOString`.

### O que a fase 1 NÃO faz

Criar evento, editar, arrastar, prazo, visão de semana, visão de dia,
recorrência, lembrete. Tudo isso é fase 2 ou depois.

## Custo no rail

O rail está no teto de nove (ADR-036), e o décimo "exige retirar um".

**`Apps` sai.** Dois motivos que se somam:

1. o banco do usuário tem **zero apps cadastrados** — sair não tira nada dele
   hoje;
2. o critério da própria ADR-036 é "algo de que depende a renda ou a memória do
   usuário, não algo que ele usa com frequência". Um lançador é conveniência.
   Library **é** memória, Inbox é a entrada dela, Workspaces é a lente.

`Workspaces` está fora de cogitação por evidência: a ADR-031 registra que ele já
foi rebaixado uma vez e o resultado foi ficar "invisível para quem não conhece o
Command, até ser promovido de volta".

Apps continua alcançável pelo Command e pelos atalhos `Ctrl+1..9`. A diferença
para o caso do Workspaces é que Workspaces era uma lente que mudava a Home
inteira — quem não sabia que existia não sabia o que estava perdendo. Um
lançador é procurado pelo nome do programa, e busca serve bem para isso.

Isso vira **ADR-038**, revisando a ADR-036.

**Ressalva registrada:** "zero apps cadastrados" mede conteúdo, não frequência.
O M/OS não registra clique de navegação, então este é o melhor sinal disponível
e não o sinal ideal. A decisão foi tomada pelo proprietário com essa ressalva
dita.

## Testes

**Rust.** A composição e a janela: item fora da janela fica fora; sessão carrega
duração e valor; Task sem `completed_at` não vira `TaskDone`; a ordem é
crescente por instante.

**Renderer — e isto é uma adição deliberada ao escopo.** O agrupamento por dia
local é a peça mais perigosa deste trabalho, e hoje o renderer não tem como
testar nada. Entra o **vitest**, e com ele o teste que importa: uma sessão às
23:31 no horário local cai no dia 30, e não no 31.

É a terceira vez que uma função pura do renderer mereceria teste
(`suspiciousEntry`, os widgets de tempo, agora o agrupamento). A adição resolve
as três, e o custo é um arquivo de configuração e um script no `package.json`.

## Riscos

| Risco | Tratamento |
|---|---|
| Dia local vs UTC | fronteira arquitetural + teste do caso das 23:31 |
| A grade fica vazia | com o dado real são ~13 dias com horas; a célula vazia é informação, não falha |
| A fase 1 frustra por não agendar | dito antes de construir, e aceito |
| `Apps` fazer falta depois | reversível: a ADR-038 pode ser revisada como a 036 revisou a 031 |
