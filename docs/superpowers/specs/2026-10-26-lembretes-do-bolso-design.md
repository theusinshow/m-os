# Lembretes e avisos do bolso — desenho

**Data:** 2026-10-26 · **Estado:** aguardando revisão do dono
**Antes deste:** ADR-089 (cortes de ruído no desktop), ADR-090 (porta, ícone, sódio P3)

## O problema, nas palavras do dono

Adiar pela notificação, criar lembrete mais rápido, avisar prova e as coisas do
dia, e menos ruído. Tudo no iPhone.

## O limite que molda o desenho

**Web Push no iOS não tem botões de ação.** Não existe "Adiar 1h" dentro da
notificação, e nenhum código resolve isso. O que existe é o toque — e o toque
hoje abre a Home, ou nem navega se o app já estiver aberto (`sw.js:83-98`).
"Adiar pela notificação" vira: **o toque cai na folha daquele lembrete, com
Concluir e Adiar à mão**. Um toque a mais que um botão no aviso, e nenhum a mais
que isso.

## 1. O toque leva ao lembrete

- `Aviso.url` passa de `"/"` para `/?lembrete={id}` (ou `?aviso={tipo}:{id}`
  para os avisos do piloto, §3). A navegação do PWA (`navegacao.ts`) abre a folha
  do lembrete a partir do parâmetro.
- `sw.js` no `notificationclick`: com janela aberta, **navega** a janela
  (`client.navigate(url)`) antes de focar; sem janela, abre na URL.
- A folha (`Lembrete.tsx`) abre com as ações em cima: Concluir, e as opções de
  adiar.

## 2. Adiar e criar falam a mesma língua nos dois aparelhos

- **Opções de adiar**: as do desktop (`lembretes.ts::snoozeChoices`) — 10 min,
  1 hora, Mais tarde (antes das 19h), À noite 20h (antes das 17h), Amanhã 9h,
  Semana que vem. O "amanhã" do bolso hoje é +24h, e passa a ser amanhã às 9h.
- **Presets de quando** e **adiar** saem do código de cada app para um módulo TS
  só, importado pelo desktop e pelo bolso (`packages/`, ao lado do
  `design-system`). Lógica de instante é regra de domínio; duas cópias já
  divergiram uma vez.
- **Adiar registra história**: `api.rs::adiar_lembrete` passa a chamar
  `AttentionService::snooze`, como `concluir` já chama o serviço — hoje adiar no
  bolso não escreve o evento `Snoozed`.
- **Criar** (`Quando.tsx`) recebe o que o `ReminderComposer` ganhou na ADR-089:
  "Algum dia" como resposta de QUANDO, erro de horário no passado antes do
  clique, erro de título vazio depois da tentativa, e a folha só fecha quando
  salvou.
- README do `mos-web` e as docs de `transitar`/`de_bolso.rs` deixam de dizer que
  o bolso não adia.

## 3. O bolso roda o motor de avisos

O `mos-core::piloto::avisos` (`candidatos` + `decidir`) é puro e o `mos-web` já
monta a `Leitura` dele para `/api/piloto`. Um laço de 60s no `mos-web` passa a
chamá-lo e entregar por Web Push, ao lado do laço de lembretes.

**O que o bolso avisa:** Academic (prova: janela fechando hoje/amanhã; prova e
bloco de estudo em 15 min), Overdue, ForgottenTask, Upcoming.
**O que fica só no desktop:** DayNotStarted, UnfinishedDay, Sync, WaitingFor —
são coisas de quem está na mesa.

A memória do que já foi avisado é a `autopilot_avisos` do banco do `mos-web`,
que é local e não sincroniza — o mesmo arranjo do desktop.

## 4. O iPhone cala quando o PC está em uso

Decisão do dono: com o PC **em uso**, o iPhone segura os avisos do §3. Lembretes
continuam tocando nos dois — são compromisso explícito da pessoa, e o que for
resolvido num some do outro pelo sync.

"Em uso" e não "ligado": um PC esquecido ligado em casa não pode calar o aviso da
prova na rua.

- O desktop já bate em `/sync/aparelho` (`mos-sync-server/src/http.rs:64`). A
  batida ganha `em_uso_em`: o instante da última entrada de teclado/mouse
  (`GetLastInputInfo` no Windows, atrás do `platform` do desktop — nada de
  plataforma no `mos-sync`). Campo opcional: aparelho antigo que não manda
  continua valendo.
- O hub guarda numa coluna aditiva de `aparelhos`. É metadado operacional, como
  `visto_em` — não é entidade, não vai para o log de operações.
- O `mos-web` lê `/sync/aparelhos` (mesmo token que já usa) e considera o PC em
  uso se algum aparelho desktop teve `em_uso_em` nos últimos **5 minutos**.
- Aviso segurado não é perdido: se o PC sair de uso e o aviso ainda valer (prova
  ainda não fechou, task ainda vencida), a rodada seguinte o entrega.

## 5. Menos ruído

- **Sai** o push "N itens novos vieram do computador" (`avisos.rs:198`). Ninguém
  age sobre ele; o badge e a inbox já mostram.
- Todo push do bolso — lembrete e piloto — passa pelo teto por hora e pelo
  cooldown por tipo do `decidir`. Lembrete marcado "Não me deixe esquecer" fura o
  teto, como fura no desktop.
- Horário silencioso continua o de `attention_settings` (00–08h).

## Os oito eixos

```
core:          nenhuma regra nova — o bolso passa a chamar piloto::avisos e
               AttentionService::snooze, que já existem. Presets de instante
               saem para um módulo TS compartilhado
database:      mos-web: autopilot_avisos já existe no schema do storage. Hub:
               coluna aditiva em_uso_em em `aparelhos`
sync:          nada novo no log de operações. A batida /sync/aparelho ganha um
               campo opcional
desktop:       manda em_uso_em na batida; importa os presets do módulo comum
ios:           toque leva ao lembrete; adiar/criar revistos; avisos de prova e
               do dia; silêncio com PC em uso; sai o aviso de sync
notifications: remotas: +Academic/Overdue/ForgottenTask/Upcoming, −Sync, teto
               e cooldown. Locais (desktop): nenhuma mudança
hermes:        nenhuma
tests:         avisos.rs: candidatos do bolso filtrados, silêncio com PC em uso
               (5 min, aparelho sem campo, aviso segurado que volta), teto por
               hora; hub: batida com e sem em_uso_em; vitest: presets
               compartilhados (amanhã 9h, Mais tarde só antes das 19h),
               navegação por ?lembrete=; de_bolso.rs: adiar escreve Snoozed
```

## Fora deste desenho

- **Revisão visual do bolso** com referência externa — próxima frente, depois
  desta, para não redesenhar telas que esta ainda muda.
- **Aviso de abertura de janela da prova** — exige migration e o campo
  atravessando o sync (ADR-089, "O que ficou de fora").
- **Botões dentro da notificação** — o iOS não oferece.
