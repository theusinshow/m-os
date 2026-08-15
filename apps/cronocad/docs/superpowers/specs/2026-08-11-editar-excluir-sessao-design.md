# Editar e excluir sessao: alcance e aviso de sessao suspeita

Data: 2026-08-11

## Problema

O usuario deixou o cronometro ligado ao dormir e gerou uma sessao de **24h12min**
no projeto JABOTICATUBA (10/08 22:33 -> 11/08 22:46). Ao tentar corrigir, pediu
"uma funcao de excluir e editar um cronometro" — mas **essas funcoes ja existem**
no Historico (`EntryForm` + `entriesStore.update/remove`, comandos
`update_time_entry`/`delete_time_entry`). O problema nao e falta de recurso, e
falta de alcance:

1. **Procurou no Painel.** "Sessoes recentes" (`DashboardPage.tsx`) e somente
   leitura: nenhuma acao por linha e nenhum link para o Historico. O cronometro
   mora no Painel, entao e ali que o usuario espera consertar a sessao.
2. **Nao viu os botoes no Historico.** As acoes sao botoes so-icone
   (`aria-label` sem texto visivel) na **ultima** coluna de uma tabela com
   `min-w-[980px]` dentro de `overflow-x-auto`. Em janela menor que isso a coluna
   de acoes fica fora da area visivel ate rolar na horizontal. O proprio codigo
   ja registra esse risco em comentario (`HistoryPage.tsx:216-220`).
3. **Nao sabia que o Historico existia.** O item esta na barra lateral, mas o
   rotulo "Historico" nao comunica "e aqui que voce edita suas sessoes", e nada
   no Painel aponta para la.

Alem disso, nada no app avisa que uma sessao tem duracao implausivel. O
esquecimento so aparece quando o usuario vai olhar o relatorio — ou nunca.

## Decisoes

- **Nao criar exclusao permanente.** A exclusao reversivel (`deleted_at`) ja
  atende: o Historico tem o filtro "Mostrar excluidas" e o botao Restaurar.
  Preservar historico e regra do projeto; o usuario confirmou que nao precisa
  apagar do banco.
- **Sem migration.** Nenhuma coluna nova. O aviso de sessao suspeita e regra de
  visualizacao, derivada dos campos que ja existem.
- **Limite de sessao longa: 8 horas.** Escolhido pelo usuario. Marca dias longos
  legitimos, mas o selo e informativo e nao bloqueia nada.
- **A regra de suspeita vive so no frontend** (`src/lib/`). Nao toca em banco,
  duracao nem dinheiro, entao nao precisa de contraparte em Rust.

## Arquitetura

### 1. Regra de sessao suspeita — `src/lib/suspiciousEntry.ts`

Funcao pura, sem dependencia de store nem de React:

```ts
export type SuspicionReason = "muito-longa" | "madrugada";

export interface Suspicion {
  suspicious: boolean;
  reasons: SuspicionReason[];
}

export const LONG_SESSION_HOURS = 8;

export function inspectEntry(entry: TimeEntry, now?: Date): Suspicion;
```

Regras:

- Avalia **somente** `source === "timer"`. Sessao `manual` foi digitada de
  proposito e `reconstructed` nasce de uma decisao explicita do usuario na linha
  do tempo; marcar essas duas seria alarme falso garantido.
- `muito-longa`: `durationSeconds > LONG_SESSION_HOURS * 3600`.
- `madrugada`: o intervalo `[startedAt, endedAt]` contem **alguma ocorrencia das
  04:00 do horario local** (para sessoes de varios dias, basta uma). Comparacao
  em horario local, nao UTC, reaproveitando os helpers de `src/lib/datetime.ts`.
  Se `endedAt` for nulo (sessao em aberto), retorna `suspicious: false` —
  cronometro rodando nao e erro.
- Os motivos sao acumulativos: a sessao de 24h retorna os dois.

Rotulos legiveis ficam em `src/lib/labels.ts`, junto dos demais.

### 2. Painel — "Sessoes recentes" acionavel

Em `DashboardPage.tsx`:

- Cada linha ganha um botao **"Editar"** com texto sempre visivel (nao em hover:
  affordance escondida e a causa raiz deste problema) que abre o mesmo
  `EntryForm` usado pelo Historico.
- O `PanelHeader` ganha a acao **"Ver todo o historico ->"** apontando para
  `ROUTES.history`.
- Linhas suspeitas exibem um selo ambar **"Conferir?"** com o motivo em `title`.

O `EntryForm` passa a ser montado tambem no Painel, com estado local
`editing`/`formOpen` — mesmo padrao ja usado no Historico.

### 3. `EntryForm` ganha "Excluir sessao"

O rodape do modal, quando `entry !== null`, ganha um botao **Excluir sessao** em
vermelho, alinhado a esquerda (separado de Cancelar/Salvar). Abre o modal de
confirmacao da secao 5; ao confirmar, chama `remove(entry.id)` e fecha os dois.

Isso resolve o Painel com uma unica affordance por linha — a coluna de "Sessoes
recentes" e estreita demais para dois botoes — e torna excluir um ato
deliberado, coerente com a regra critica 8 ("nunca encerrar/descartar tempo
silenciosamente").

### 4. Historico — acoes visiveis

Em `HistoryPage.tsx`:

- "Editar" e "Excluir" passam a ter **texto visivel**, nao so icone. O botao de
  "Adicionar tempo a esta sessao" continua so-icone: e uma acao secundaria e ja
  tem ponto de entrada proprio no cabecalho ("Tempo esquecido").
- A coluna Acoes fica **fixa a direita** (`position: sticky; right: 0`) com fundo
  opaco e borda esquerda, para nao sumir na rolagem horizontal. Precisa de fundo
  solido nos dois temas e do estado `hover` da linha respeitado.
- Linhas suspeitas exibem o mesmo selo "Conferir?" da secao 2, na coluna Duracao.

### 5. Modal de confirmacao de exclusao — `src/features/history/DeleteEntryModal.tsx`

Substitui o `window.confirm` de `HistoryPage.handleDelete`. Mostra projeto, data,
periodo, duracao e o valor que sai da conta (mesma formula da coluna Valor:
`amountForDuration(duration - idle, snapshot)`), e diz explicitamente que a
sessao pode ser restaurada depois pelo filtro "Mostrar excluidas".

Usado pelo Historico e pelo `EntryForm`. Recebe `entry` e `onConfirm`.

## Testes

`src/lib/suspiciousEntry.test.ts`:

- duracao exatamente no limite (8h) nao marca; um segundo acima marca
- sessao que atravessa as 04:00 locais marca, mesmo sendo curta
- sessao curta fora da madrugada nao marca
- sessao longa com `source: "manual"` nao marca
- sessao com `endedAt: null` nao marca
- a sessao real de 24h retorna os dois motivos

Componentes:

- `EntryForm`: botao Excluir aparece so em edicao, nunca em criacao; confirmar
  chama `remove` com o id certo; cancelar nao chama nada
- `DashboardPage`: linha suspeita renderiza o selo; clicar em Editar abre o modal
- `HistoryPage`: exclusao passa pelo modal novo, nao por `window.confirm`

## Fora de escopo

- Exclusao permanente (hard delete)
- Limite de horas configuravel nas Configuracoes
- Encerrar o cronometro automaticamente por inatividade prolongada — mexe no
  motor do cronometro e merece desenho proprio

## Correcao pontual do dado existente

Independente da implementacao, a sessao `3937616c-4e3f-4b83-afcd-20f7d2bb0dd8`
sera corrigida direto no banco (com copia de seguranca antes e o app fechado):
fim de `2026-08-12T01:46:24Z` para `2026-08-11T01:53:44Z`, `duration_seconds` de
`87160` para `1200` (20 minutos, conforme o usuario lembra), `updated_at`
atualizado. Mesmos campos e formato que `repository::time_entries::update`
escreveria.
