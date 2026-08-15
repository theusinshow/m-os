# Confirmacao ao encerrar o cronometro

Data: 2026-07-13
Status: aprovado, aguardando implementacao

## Problema

Encerrar o cronometro e uma acao irreversivel: `stop_timer` grava a sessao em
`time_entries`, congela a tarifa/hora e **apaga** o `active_timer`
(`src-tauri/src/repository/timer.rs:114-161`). Nao existe nenhum caminho de
volta — nao ha comando que reabra uma sessao ja gravada.

Apesar disso, hoje o Encerrar dispara com **um unico clique, sem confirmacao**
(`src/features/timer/TimerPanel.tsx:100-107`).

Pior: a interface vem empurrando o usuario para o Encerrar em tres lugares.

| Local | Pausar | Encerrar |
|---|---|---|
| `TimerPanel.tsx:80-108` | `secondary` | `danger` |
| `MonitorPrompts.tsx:186-204` (CAD fechado) | `secondary` | **`primary`** |
| `MonitorPrompts.tsx:60-74` (sair do app) | `secondary` | **`primary`** |

O usuario relatou o erro na pratica: queria pausar, encerrou. O resultado sao
sessoes picadas no historico, sem vinculo entre si.

Isso viola a regra critica 8 do projeto ("nunca encerrar/descartar tempo
silenciosamente") no espirito, ainda que nao na letra: o tempo nao se perde,
mas a decisao irreversivel acontece sem decisao consciente.

## Objetivo

Tornar o Encerrar uma decisao deliberada, e transformar o erro comum (queria
pausar) em acerto — sem adicionar atrito ao uso correto do app.

Nao-objetivos: mudar o backend, permitir "reabrir" sessao encerrada, mexer no
tray.

## Solucao

### 1. Modal de confirmacao (`StopConfirmModal`)

Novo componente `src/features/timer/StopConfirmModal.tsx`, seguindo o padrao ja
estabelecido pelo `RecoveryModal.tsx` e usando `components/ui/Modal.tsx`.

O `TimerPanel` deixa de chamar `stop()` direto no clique do Encerrar; passa a
abrir este modal.

Conteudo:

- Titulo: **"Encerrar sessao?"**
- Projeto + tipo de atividade (ex.: "Edificio Aurora · Desenho")
- Tempo decorrido **ao vivo**, recalculado a cada segundo com o mesmo
  `useNow(1000)` + `elapsedSeconds(timer, now)` do `TimerCard.tsx:21-23`.
  O cronometro continua rodando enquanto o modal esta aberto — nenhum tempo se
  perde se o usuario hesitar.
- Valor estimado que sera gravado (`amountForDuration`).
- Texto: "Depois de encerrar, esta sessao vira um registro definitivo no
  historico. Se voce so vai dar uma pausa, use Pausar."

Botoes (na ordem do footer):

| Botao | Variante | Acao |
|---|---|---|
| Cancelar | `ghost` | fecha o modal, nada acontece |
| Encerrar mesmo assim | `danger` | chama `stop()`, fecha |
| Pausar em vez disso | `primary` | chama `pause()`, fecha |

O botao primario e o **Pausar** — e a saida correta do erro mais provavel.

### 2. Hierarquia visual

Inverter a proeminencia nos tres locais da tabela acima: **Pausar vira a acao
primaria** e **Encerrar fica discreto** (secundario, com o vermelho apenas no
texto/hover, nao no preenchimento).

Racional: o botao mais chamativo deve ser o da acao reversivel e cotidiana.

### 3. Casos de borda

- **Cronometro pausado + clique em Encerrar:** o modal abre sem o botao "Pausar
  em vez disso" (nao faz sentido pausar o que ja esta pausado). Restam Cancelar
  e Encerrar.
- **Tray:** permanece **inalterado**. Encerrar pelo tray continua direto, sem
  confirmacao. O usuario nao usa esse caminho e um menu nativo do Windows nao
  abre modal React. Decisao explicita de YAGNI.
- **Backend:** `stop_timer` nao muda. A confirmacao e puramente de UI.
- **Erro no `stop()`:** o modal exibe a mensagem e permanece aberto (mesmo
  tratamento do `run()` existente em `TimerPanel.tsx:55-65`).

## Testes (Vitest)

Sobre o `TimerPanel`, com o `timerStore` mockado:

1. Clicar em "Encerrar" **nao** chama `stop` — apenas abre o modal.
2. Confirmar no modal chama `stop` exatamente uma vez.
3. "Pausar em vez disso" chama `pause` e **nunca** `stop`.
4. "Cancelar" nao chama nem `pause` nem `stop`.
5. Com o timer `paused`, o modal nao renderiza "Pausar em vez disso".

## Arquivos afetados

- Novo: `src/features/timer/StopConfirmModal.tsx`
- Novo: `src/features/timer/TimerPanel.test.tsx`
- Editado: `src/features/timer/TimerPanel.tsx` (abre o modal; hierarquia)
- Editado: `src/features/monitoring/MonitorPrompts.tsx` (hierarquia no
  `ClosePrompt` e no `QuitPrompt`)

Nenhuma migration. Nenhum comando Tauri novo. Nenhuma mudanca em Rust.

## Limpeza dos registros ja picados

Fora do escopo deste spec, mas registrado: as sessoes encerradas por engano
podem ser corrigidas manualmente no Historico (editar inicio/fim recalcula a
duracao; `repository/time_entries.rs:86-111`) e as sobrantes removidas por soft
delete (restauraveis).
