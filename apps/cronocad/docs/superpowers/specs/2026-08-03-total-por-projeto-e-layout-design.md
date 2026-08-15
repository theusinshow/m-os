# Total acumulado por projeto + auditoria de layout

Data: 2026-08-03

## Problema

1. A tela **Projetos** mostra horas trabalhadas e valor/hora, mas nao mostra
   quanto o projeto ja acumulou em dinheiro. Para cobrar, o usuario precisa ir
   ao Relatorio e filtrar projeto por projeto.
2. O **Relatorio** nao quebra os totais por projeto — so por tipo de atividade.
3. Alguns paineis vazam/cortam conteudo em larguras diferentes de janela.

## Achado que motiva escopo extra

`entriesStore.load()` carregava apenas as **200 sessoes mais recentes**
(`listTimeEntries(200, false)`). Painel e Relatorio calculam tudo em cima desse
array, entao a partir de 200 sessoes o "Valor total" com filtro "Tudo" ficava
silenciosamente menor que a realidade. Calcular o acumulado do projeto no
frontend herdaria o mesmo teto. Corrigir isso entra no escopo: sem isso, o
numero pedido nao fecha com o Relatorio.

## Decisoes

- **Definicao do total**: faturavel, pela mesma regra do Relatorio — desconta
  inatividade, zera sessoes nao-faturaveis, aplica o arredondamento das
  Configuracoes **por sessao** e soma, usando o `hourly_rate_snapshot_cents` de
  cada sessao. Nunca reescreve o tempo real no banco (regra critica 5).
- **Onde aparece**: coluna na tela Projetos (vida inteira do projeto) **e**
  painel "Por projeto" no Relatorio (periodo/cliente filtrados).
- **Sem migration.** E tudo agregacao de leitura sobre tabelas existentes;
  nenhum dado do usuario e alterado ou apagado.

## Arquitetura

### Coluna de Projetos — agregada no backend

O total vitalicio nao pode depender do que esta carregado no frontend. Novo
comando le **todo** o historico direto do SQLite:

```
list_project_billing()
  -> settings::get()                 (le o arredondamento do proprio banco)
  -> time_entries::billing_rows()    (todas as sessoes nao excluidas)
  -> domain::billing::aggregate_by_project()
  -> Vec<ProjectBilling>
```

`domain::billing` e puro e compoe as funcoes ja testadas de `domain::timer`
(`net_duration`, `billable_duration`, `round_duration`, `amount_for_duration`).
Nada de logica nova de arredondamento — so a composicao e a agregacao.

`ProjectBilling { projectId, grossSeconds, idleSeconds, billableSeconds,
amountCents }`.

O comando le o arredondamento da tabela `settings`, nao de um parametro do
frontend: garante que backend e UI usem a mesma configuracao sem poder divergir.

`list_project_totals` (que devolvia so segundos por projeto) foi **removido**:
virou subconjunto exato de `list_project_billing`, e manter duas APIs
sobrepostas para o mesmo dado so criaria divergencia com o tempo.

### Painel do Relatorio — derivado no frontend

O bloco "Por projeto" agrupa o array `rows` que a pagina ja monta. Assim ele
respeita periodo, cliente, projeto e ajuste percentual, e a soma das linhas bate
com o "Valor total" **por construcao** — nao ha como os dois numeros divergirem.

Reimplementar esses filtros no Rust so criaria uma segunda fonte de verdade.

### Teto de 200 sessoes

`list_time_entries` passa a tratar `limit` ausente como "sem limite", e
`entriesStore` / `HistoryPage` deixam de passar 200. Para o volume real de um
projetista (ordem de milhares de sessoes em anos de uso) carregar tudo em
memoria e barato, e mantem toda a consistencia do calculo no frontend.

## Layout

Janela minima e 960x600; sidebar fixa em 232px. Auditoria com o app rodando em
960x600, 1120x740, 1440x900 e 1920x1080, nas 6 telas e nos modais.

A auditoria roda com Playwright sobre o Vite dev, com a ponte IPC do Tauri
substituida por dados realistas (nomes longos, valores de 5 digitos). Mede
overflow de todos os elementos, ignorando o que rola de proposito.

Encontrado e corrigido:

- **Historico**: tabela de 7 colunas sem rolagem — a coluna de acoes ficava fora
  da area visivel em 960px, com os botoes inalcancaveis.
- **Relatorios**: `min-width: auto` de item de grid impedia o painel de encolher
  e o conteudo vazava para fora da janela por volta de 1120px.
- **Projetos**: `truncate` em celula de tabela auto-layout nao funciona; passou a
  `table-fixed` com larguras declaradas.
- `max-w-5xl` no `AppLayout` sobe para `2xl:max-w-7xl`.

## Testes

- `domain::billing` (`cargo test`): sessao nao-faturavel nao soma valor;
  inatividade descontada; arredondamento por sessao e nao sobre a soma;
  snapshot de valor/hora por sessao; projeto sem sessoes.
- `src/lib/reportTotals.test.ts` (Vitest): agrupamento por projeto, ordenacao,
  e soma igual ao total do relatorio.
- Verificacao final: `npm run typecheck`, `npm run lint`, `npm run test`,
  `cargo test`, `cargo clippy`, `cargo fmt`.
