# Sync V2 — convergência verificável

Data: 2026-09-04
Estado: aprovado, aguardando implementação
Substitui: a camada de transporte e reconciliação da V1. O domínio, o banco
local e o modelo local-first **não** mudam.

## O problema

A V1 sincroniza e ninguém consegue provar que sincronizou. Em três dias de uso
real, com três aparelhos, o dono viu: horas do CronoCAD que não apareciam no
segundo PC, dados que existem numa máquina e não na outra, e uma tela de
sincronização dizendo "0 na fila" enquanto as duas discordavam.

Quatro defeitos sustentam isso, e três deles foram lidos no código, não
supostos:

1. **`ack` significa "o hub recebeu", nunca "o aparelho aplicou".** A fila
   esvazia quando o servidor confirma o recebimento. O que acontece depois — a
   operação virar linha na tabela que a tela lê — não tem confirmação nenhuma.
2. **A fila de pendentes é volátil.** `ProjecaoSqlite.pendentes` é um `Vec` em
   memória (`sync_projecao.rs:968`). Uma entidade que chega e falha ao
   materializar entra nessa lista; o cursor avança no mesmo pull
   (`engine.rs:192`, gravado quando a rodada não tem erro); o app fecha. **Nada
   nunca mais tenta de novo.** A entidade fica viva em `sync_state` e invisível
   na tela, para sempre. É a explicação mais provável do "chegou e não aparece".
3. **O HLC ordena, mas não prova concorrência.** O merge marca conflito quando
   dois valores diferentes vêm de dispositivos diferentes (`merge.rs:111`),
   mesmo quando a segunda escrita já conhecia a primeira. Há 13 conflitos
   guardados nesta máquina, nenhum deles visto por ninguém, e parte é falso
   positivo.
4. **Não existe pergunta "estamos iguais?".** O hub guarda um log append-only e
   se declara passivo (`hub.rs:6`). Ele não sabe dizer qual é o estado correto
   de uma entidade, nem qual aparelho está incompleto.

## O objetivo

Abrir qualquer um dos três aparelhos e **provar**, na tela, que os três têm o
mesmo conteúdo — ou ver nomeado exatamente quem está atrás e em quê.

O que **não** muda, e é premissa: o M/OS continua funcionando inteiro com a VPS
fora do ar. Escrita local é imediata e não espera rede. O servidor coordena e
verifica; ele não é o dono do dado.

## Escopo

**Entra:** as 26 famílias que já atravessam hoje — tasks, projects, capturas,
recursos, lembretes, acadêmico, CronoCAD (horas, cobrança, clientes,
configuração), conversas do Hermes, diário e revisão semanal.

**Fica de fora, por decisão:**

- **Arquivos e anexos** (reuniões, notas de voz, ingestões). Eles apontam para
  caminhos em disco; trazê-los exige armazenamento de objetos, manifesto de
  binários e reparo de arquivo faltando — metade de um projeto, sozinho.
- **Credencial por dispositivo.** O token único não é o que está quebrado.
- **Postgres.** Três aparelhos não pagam a operação de um banco novo; o
  `mos-sync-server` em Rust com SQLite continua.
- **Shadow mode.** Com um dono e três máquinas, rodar duas sincronizações em
  paralelo custa mais do que o risco que evita.

## A arquitetura

```
PC casa ─── SQLite + outbox ──┐
PC trabalho ─ SQLite + outbox ─┼── HTTPS /sync/* ──> mos-sync-server
iPhone ───── SQLite + outbox ──┘                     ├── entidades (estado canônico)
                                                     ├── campos (revisão por campo)
                                                     ├── mudancas (log, server_seq)
                                                     ├── checkpoints (aplicado por aparelho)
                                                     ├── conflitos
                                                     └── manifestos
```

### 1. O servidor passa a ter estado, e continua sem ter regra

O hub deixa de ser só um log. Ele passa a manter o estado canônico de cada
entidade e a revisão de cada campo.

O que ele **não** ganha: conhecimento de domínio. Ele não sabe o que é uma Task,
não valida conteúdo, não deriva nada. Ele guarda `(entidade, campo) → valor,
revisão, instante, dispositivo` e aplica uma regra de merge que é a mesma do
cliente. É a diferença entre "o servidor coordena" e "o cliente virou terminal
burro" — a primeira é o que o `SYNC.md` sempre pediu, e é onde isto para.

Tabelas novas no hub:

| Tabela | O que guarda |
| --- | --- |
| `entidades` | uma linha por entidade: kind, id, revisão atual, apagada |
| `campos` | `(kind, id, campo)` → valor, revisão, hlc, dispositivo |
| `mudancas` | log incremental: `server_seq`, entidade, campos alterados, revisão |
| `checkpoints` | `(dispositivo)` → último `server_seq` **aplicado**, e quando |
| `conflitos` | concorrência real: os dois valores, quem venceu, se foi resolvido |
| `manifestos` | `(dispositivo, familia)` → contagem, hash, revisão em que foi tirado |

`sync_log` da V1 continua existindo até o corte, e some depois.

### 2. A operação passa a carregar a revisão-base

Hoje uma operação diz "este campo virou X, no instante T". Na V2 ela diz "este
campo virou X, e quando eu escrevi, ele estava na revisão R".

Com isso o servidor distingue três casos que hoje são um só:

- **R é a revisão atual** → escrita em cima do que ela conhecia. Aplica, sem
  conflito.
- **R é anterior, e o campo mudou desde então** → concorrência **real**.
  Registra conflito, e o LWW por HLC decide o vencedor (mesma regra de hoje —
  o que muda é que agora o conflito significa alguma coisa).
- **R é anterior, mas o valor atual é igual ao que ela viu** → não houve
  disputa. Aplica em silêncio.

O terceiro caso é o que hoje enche a tabela de conflito com falso positivo.

### 3. Confirmar é aplicar, e não receber

Dois reconhecimentos, e não um:

- **`aceita`** — o servidor gravou a operação. É o que a V1 já faz, e é o que
  esvazia a outbox.
- **`aplicado_ate`** — o cliente materializou até o `server_seq` N nas tabelas
  que a tela lê. Só isso conta como convergência, e é o que vai para
  `checkpoints`.

A consequência prática: uma entidade que chega e não vira linha **impede** o
checkpoint de avançar. O aparelho passa a se declarar atrasado em vez de dizer
"em dia" com a tela vazia.

### 4. Os pendentes viram tabela, e a abertura repara

`pendentes` sai da memória e vira `sync_pendentes` no banco local: kind, id,
tentativas, último erro. Duas coisas passam a mexer nela:

- cada rodada tenta de novo o que está lá;
- **a abertura do app faz uma varredura**: toda entidade em `sync_state` que não
  tem linha correspondente na tabela de domínio entra na fila e é
  re-materializada.

É esse segundo ponto que conserta os bancos que já estão nesse estado hoje — e
ele funciona sem servidor nenhum, porque o dado já está no aparelho.

### 5. O manifesto, que é a prova

Na batida, cada aparelho manda, por família: **contagem** e **hash** das linhas
vivas (id + revisão, ordenados, SHA-256). O servidor guarda e compara.

A tela de sincronização passa a mostrar:

```
A MALHA
DESKTOP-634TJR1   0.4.0   este aparelho   ✓ 3/3 alinhados
6ZXJM74           0.4.0   há 2 min        ✗ atrás: time_entry (1 de 26), task (17 de 19)
M/OS de bolso     0.4.0   há 1 min        ✓ alinhado
```

"Atrás" é diferente de "divergente": o primeiro é o aparelho que ainda não
aplicou tudo; o segundo é hash diferente com a mesma contagem, e aí a linha
oferece **reparo** — pedir ao servidor o estado canônico daquela família e
reescrever o local.

### 6. Bootstrap por snapshot

Aparelho novo (ou reparado) não reproduz o log inteiro. Ele pede um **snapshot**
— o estado canônico de todas as entidades na revisão R — e depois os deltas a
partir de R. O servidor gera o snapshot sob demanda a partir de `entidades` +
`campos`; com o volume atual (menos de mil entidades) isso é uma consulta, não
um arquivo.

## A migração

**Este PC é a fonte da verdade** — é o mais completo: 998 operações aplicadas,
`sync_state` e tabelas de domínio batendo em todas as famílias conferidas.

Antes de qualquer coisa, e sem exceção: **cópia do `m-os.db` dos outros dois
aparelhos**, guardada fora deles. A escolha da fonte única apaga o que existir
só no PC do trabalho, e essa cópia é o que torna a decisão reversível — se
faltar algo depois, dá para extrair de lá.

A ordem:

1. Cópia dos três bancos, com data no nome.
2. Este PC manda um retrato completo; o servidor grava como revisão 1 do estado
   canônico. Nenhum outro aparelho escreve até aqui.
3. Os outros dois recebem o snapshot e **reconstroem** as tabelas
   sincronizáveis. O que é local por natureza não é tocado: layout de tela, apps
   instalados, cronômetro em curso, credenciais, monitoramento, notificações já
   entregues.
4. Cada um manda seu manifesto. O corte só é declarado quando os três
   responderem o mesmo hash em todas as 26 famílias.
5. A V1 sai do código depois de uma semana de dogfood, não antes.

## Como isto é conferido

**Testes, com três bancos SQLite de verdade:**

- Três aparelhos, uma escrita em cada, e os três convergem para o mesmo
  manifesto.
- Escrita concorrente real (mesma revisão-base) → conflito registrado.
- Escrita sequencial (revisão-base atual) → **nenhum** conflito. É o teste que
  o falso positivo de hoje falharia.
- Falha de materialização: a entidade fica pendente, o checkpoint **não** avança,
  e a abertura seguinte repara.
- Crash entre `aceita` e `aplicado_ate`: nada se perde, e a rodada seguinte
  retoma do checkpoint.
- Aparelho novo entra por snapshot e chega ao mesmo hash sem ler o log inteiro.
- Um aparelho com uma linha adulterada à mão → hash diferente → reparo restaura.

**Na máquina, e não só em teste:** os três aparelhos mostrando `3/3 alinhados`
na tela, e o `time_entry` do CronoCAD aparecendo no PC do trabalho — que é o
sintoma que originou tudo isto.

## O que muda no repositório

| Onde | O quê |
| --- | --- |
| `crates/mos-sync` | `Op` ganha revisão-base; `Rodada` separa aceito de aplicado |
| `crates/mos-sync-server` | as seis tabelas, o merge canônico, snapshot e manifesto |
| `crates/mos-storage-sqlite` | `sync_pendentes`, varredura de reparo, cálculo de manifesto |
| `apps/desktop` | tela da malha com alinhamento e reparo; comando de reparo |
| `apps/mos-web` | mesma batida e manifesto |
| `docs/SYNC.md` | reescrito para a V2 |
| `docs/PLATFORMS.md` | §8 mente hoje: diz que não há servidor, e há |

## Decisões que este spec fecha

- **Servidor com estado, sem domínio.** Ele guarda revisão por campo e aplica o
  merge; não sabe o que é uma Task.
- **Local-first mantido.** Nenhuma escrita espera rede. O M/OS abre e funciona
  com a VPS fora.
- **Um dono, um token.** Credencial por dispositivo fica para o dia em que
  houver aparelho para revogar.
- **Sem arquivos.** Reuniões, voz e ingestões continuam locais, e o
  `sync_cobertura.rs` continua sendo onde essa decisão está escrita.
