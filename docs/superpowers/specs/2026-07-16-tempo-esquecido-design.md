# Adicionar tempo esquecido

Data: 2026-07-16
Status: aprovado, aguardando implementacao

## Problema

A pergunta que guia o produto e "isso reduz a possibilidade de o usuario
esquecer de registrar seu trabalho?". Hoje, quando o esquecimento **ja
aconteceu**, o unico caminho de reparo e o formulario de sessao manual
(`src/features/history/EntryForm.tsx`), que exige:

1. navegar ate o Historico;
2. escolher o projeto;
3. preencher **inicio** e **fim** em dois campos `datetime-local`.

Os passos 3 e 1 sao o atrito. O usuario nao lembra o horario exato em que
comecou — ele lembra a **duracao aproximada** ("umas duas horas e meia"). Ter
que inventar um horario para registrar uma duracao faz o registro ser adiado, e
adiado vira esquecido.

Dois momentos concretos relatados:

- **Nao ligou nada.** Trabalhou e so depois lembrou. Quer lancar um bloco de X
  horas num projeto.
- **Sessao ja encerrada ficou curta.** O cronometro rodou, mas so foi ligado no
  meio do trabalho. A sessao esta no historico com menos tempo do que a
  realidade.

## Objetivo

Registrar tempo esquecido em poucos cliques, informando **duracao** em vez de
horarios.

Nao-objetivos: mudar o schema, criar comando Tauri novo, alterar o Rust,
detectar sobreposicao entre sessoes (ver "Fora de escopo").

## Solucao

### 1. `QuickTimeModal` — a peca central

Novo componente `src/features/history/QuickTimeModal.tsx`, ao lado do
`EntryForm` e seguindo o padrao de `components/ui/Modal.tsx`.

Grava pelo caminho que **ja existe**, sem nada novo abaixo da UI:

```
QuickTimeModal -> entriesStore.create -> createTimeEntry -> create_time_entry -> repository
```

O payload e o `ManualEntryInput` atual (`src/services/timeEntries.ts:9-19`),
com `source: "manual"`.

Quatro campos, e essa contencao e o que o torna mais rapido que o `EntryForm`:

| Campo | Comportamento |
|---|---|
| Projeto | `Select`, pre-selecionado com o projeto mais recente |
| Total | Numero grande, alimentado pelos botoes de incremento |
| Dia | `input[type=date]`, padrao hoje (`isoToDateInput`) |
| Nota | Opcional, texto livre; vai para `description` |

`activityType` fica no padrao `"drawing"`, `billable` em `true` e `idleSeconds`
em `0`. Quem precisar de outro valor edita no Historico, que ja faz isso.

Controles de incremento: `+15min`, `+30min`, `+1h`, `+2h`, mais `-15min` e
`Limpar`.

### 2. Tres portas de entrada

| Onde | Gatilho | Estado inicial |
|---|---|---|
| `TimerPanel.tsx` (sem cronometro ativo) | link "Esqueceu de registrar? Adicionar tempo" | projeto = o do formulario de inicio |
| `HistoryPage.tsx` (cabecalho) | botao `Tempo esquecido`, ao lado do `Nova sessao` | projeto = mais recente, dia = hoje |
| `HistoryPage.tsx` (linha da tabela) | acao icone (lucide `Clock`), `aria-label="Adicionar tempo a esta sessao"` | ancorado: projeto e dia **travados** na sessao |

A terceira porta atende o caso "sessao ficou curta demais".

### 3. Ancoragem na linha do tempo

O usuario informa duracao, mas o banco exige `started_at` e `ended_at`. A regra
de conversao fica isolada numa funcao pura, `src/lib/quickTime.ts`:

```ts
resolveQuickEntryWindow(input: {
  durationSeconds: number;
  day: string;            // "YYYY-MM-DD" local
  anchorAtIso?: string;   // inicio da sessao ancora, se houver (o bloco termina ali)
  dayEntries: TimeEntry[];// sessoes ja existentes naquele dia
  now: Date;              // injetado — nunca Date.now() interno, para testar
}): { startedAt: string; endedAt: string }  // ISO UTC
```

O fim e escolhido pela **primeira** regra que casar, nesta ordem, e o inicio e
sempre `fim - duracao`:

| Situacao | O bloco termina em... |
|---|---|
| Ancorado numa sessao | o `startedAt` daquela sessao (o bloco vai **antes** dela) |
| Dia = hoje | agora (`now`) |
| Dia passado, com sessoes | o `endedAt` da ultima sessao do dia |
| Dia passado, vazio | 18:00 local daquele dia |

Isso mantem a linha do tempo plausivel sem exigir que o usuario digite relogio,
o que e o mesmo espirito aproximado da reconstrucao do dia
(`source = reconstructed`) ja existente.

### 4. Limites e erros

- **Salvar** so habilita com total > 0.
- **Teto de 24h** por lancamento; os botoes de incremento nao passam disso.
- **`-15min`** tem piso em zero (nunca gera total negativo).
- **Erro de gravacao**: mensagem no proprio modal, que **permanece aberto** com
  o total preservado. Nenhum tempo desaparece em silencio (regra critica 8).

### 5. Regras criticas atendidas

- **5 (banco preserva o tempo real):** o ajuste de uma sessao curta nasce como
  um registro `manual` **separado**; a sessao `timer` original nao e tocada. O
  historico mostra as duas linhas e continua distinguindo o cronometrado do
  estimado.
- **7 (duracoes em segundos):** o total vive em segundos no estado; os minutos
  sao so apresentacao.
- **9 (seguranca):** nenhuma superficie nova — reusa `create_time_entry`, que ja
  valida.

## Testes

### `src/lib/quickTime.test.ts` (Vitest)

Cobre a tabela de ancoragem com `now` injetado:

1. Ancorado: o bloco termina exatamente no `startedAt` da sessao ancora (o
   bloco entra antes dela).
2. Hoje: termina em `now`.
3. Dia passado com sessoes: termina no fim da ultima sessao daquele dia.
4. Dia passado vazio: termina as 18:00 locais.
5. Bloco que atravessa a meia-noite: ancorado numa sessao que comecou 00:30,
   3h produzem `startedAt` no dia anterior, sem erro.
6. `startedAt` e sempre anterior a `endedAt`, e a diferenca e a duracao pedida.

### `src/features/history/QuickTimeModal.test.tsx` (Vitest)

Com o `entriesStore` mockado:

1. Incrementos acumulam (`+30min` + `+1h` = 1h30).
2. `-15min` com total zero mantem zero.
3. `Limpar` zera o total.
4. Salvar com total zero esta desabilitado.
5. Salvar chama `create` uma vez, com `source: "manual"` e a duracao pedida.
6. Erro no `create` mantem o modal aberto e exibe a mensagem.

## Arquivos afetados

- Novo: `src/lib/quickTime.ts`
- Novo: `src/lib/quickTime.test.ts`
- Novo: `src/features/history/QuickTimeModal.tsx`
- Novo: `src/features/history/QuickTimeModal.test.tsx`
- Editado: `src/features/history/HistoryPage.tsx` (botao no cabecalho + acao na
  linha da tabela)
- Editado: `src/features/timer/TimerPanel.tsx` (link no estado sem cronometro)

Nenhuma migration. Nenhum comando Tauri novo. Nenhuma mudanca em Rust — `source`
ja aceita `'manual'` (`src-tauri/migrations/0001_initial_schema.sql:62-63`).

## Fora de escopo

- **Sobreposicao de sessoes.** Um bloco ancorado pode, em tese, cair por cima de
  outra sessao. O app ja permite isso no registro manual de hoje; trazer
  deteccao de conflito agora e um spec proprio, que deveria valer para todos os
  caminhos de criacao, nao so este.
- **Ajustar o inicio de um cronometro em curso** ("comecei 40 min atras"). E um
  cenario real, mas mexe no `active_timer` e no backend. Fica registrado como
  possivel evolucao.
