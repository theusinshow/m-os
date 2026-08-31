# Cobertura total do sync — os dois PCs mostrando o mesmo M/OS

Data: 2026-08-31
Estado: aprovado, aguardando implementação

## O problema

O sync funciona e está ligado nos dois PCs, mas cobre **12 de 26** tipos de
conteúdo. O que ficou de fora não ficou por decisão: ficou porque emissão e
projeção são duas listas, escritas à mão, em arquivos diferentes, sem nada que
verifique se concordam.

O CronoCAD é a prova. `tracking_repository.rs` e `cronocad_import.rs` são os
únicos arquivos que escrevem em `time_entries`, e nenhum dos dois tem uma única
chamada a `emitir`. Vinte e duas horas de trabalho registradas nunca geraram uma
operação. Não deu erro, não apareceu em log, não quebrou teste — simplesmente
não atravessou.

Corrigir os tipos que faltam sem fechar essa fresta seria consertar o sintoma.

## O objetivo

Sentar em qualquer um dos dois PCs e ver o mesmo M/OS: o mesmo conteúdo, o mesmo
histórico. Explicitamente **não** é ter dois bancos idênticos linha a linha — o
que descreve uma máquina fica na máquina.

## O que entra e o que fica fora

**Entra (14 tipos novos):**

| Tipo | Tabela | Id da entidade |
| --- | --- | --- |
| `time_entry` | `time_entries` | `id` |
| `project_tracking` | `project_tracking` | `project_id` (extensão 1:1) |
| `tracking_settings` | `tracking_settings` | UUID fixo (linha única) |
| `conversation` | `conversations` | `id` |
| `message` | `messages` | `id` |
| `message_part` | `message_parts` | `id` |
| `academic_external_ref` | `academic_external_refs` | derivado (chave composta) |
| `academic_material_url` | `academic_material_urls` | derivado (chave composta) |
| `academic_provider_subject_fact` | `academic_provider_subject_facts` | derivado (chave composta) |
| `client` | `clients` | `id` |
| `daily_session` | `daily_sessions` | `id` |
| `daily_objective` | `daily_objectives` | `id` |
| `daily_reflection` | `daily_reflections` | `session_id` (1:1) |
| `weekly_review` | `weekly_reviews` | `id` |

**Fica fora, e por quê:**

- **Maquinaria do próprio sync** (`sync_outbox`, `sync_state`, `sync_clock`,
  `sync_conflicts`, `devices`). Replicar a fila de um PC no outro faria cada
  máquina reenviar o trabalho da outra, em laço.
- **Telemetria** (`usage_requisicao` com 18.609 linhas, `usage_janela`,
  `usage_fonte`, `activity_events`). Elas descrevem o que aconteceu *naquela*
  máquina; misturadas, os relatórios de uso passariam a somar as duas como se
  fossem uma.
- **Descrição da máquina** (`apps`, `app_metadata`, `monitored_apps`,
  `active_timer`). Um cronômetro rodando é estado de máquina: replicá-lo faria
  dois PCs disputarem um timer só.
- **O que guarda arquivo em disco** (`meetings` e as seis tabelas de reunião,
  `voice_notes`, `ingestions`). O hub carrega campos, não blobs. Uma linha que
  aponta para um `audio_dir` inexistente do outro lado é pior que a ausência.
  Decisão deliberada de adiar, não esquecimento.
- **`academic_provider_state`**: é a sessão do Univirtus, com estado de conexão
  e credencial implícita. Pertence à máquina.
- **Layout** (`radial_pins`, `workspace_widget_layout`,
  `workspace_hidden_widgets`): as duas telas são diferentes, e um layout bom
  aqui é ruim lá.
- **Metade de `tracking_settings`**: `idle_*`, `process_*`, `remind_*` e
  `meeting_detection_enabled` são configuração de máquina. Só arredondamento
  (`rounding_*`) e dados do emissor (`issuer_*`) sincronizam.

## A costura

`mapa_de(kind)` em `sync_projecao.rs` já declara `campo → coluna` por tipo. Ele
sai para um módulo próprio (`sync_mapa.rs`) e passa a ser lido pelos dois lados:

- A **projeção** continua consumindo como consome hoje. Sem mudança de
  comportamento para os 12 tipos existentes.
- A **emissão** ganha `emitir_linha(tx, kind, id)`, que lê `mapa.colunas`, faz
  `SELECT` dessas colunas em `mapa.tabela` para aquele id, e emite um `Update`
  com os valores correntes.

Os 12 tipos que já funcionam **não mudam**. Código testado não se mexe sem
motivo; a costura nova vale para os tipos novos e para os próximos.

### O que impede o próximo CronoCAD

Uma constante `SINCRONIZAVEIS: &[&str]` com as tabelas que devem sincronizar, e
um teste que, para cada uma, cria uma linha pelo repositório e exige que uma op
apareça no `sync_outbox`.

Esse teste é o entregável de longo prazo. Sem ele, esta spec conserta 14 tipos e
deixa a armadilha armada para o décimo quinto.

## Três obstáculos achados na leitura

### 1. A projeção assume colunas que nem toda tabela tem

`sync_projecao.rs:365` monta todo `INSERT` com `["id", "created_at",
"updated_at"]` fixos. Mas `message_parts` não tem carimbo nenhum, `messages` não
tem `updated_at`, e `project_tracking` não tem `id` — a chave dela é
`project_id`.

**Decisão:** tirar o hardcode e declarar chave e carimbos no próprio `Mapa`
(`chave: &'static str`, e um enum de carimbos: nenhum / só criação / ambos).
A alternativa — alterar essas tabelas por migração — foi recusada: a migration
0027 não tocou em nenhuma tabela existente de propósito (SYNC.md §6), e não há
razão para quebrar isso agora.

### 2. ~~O CronoCAD tem um caminho que passa por fora do repositório~~ — ERRADO

Escrito na primeira leitura: `cronocad_import.rs:537` faria `INSERT INTO
time_entries VALUES` em lote, cru, por fora de `create_time_entry`.

**Está errado.** A linha 537 fica **dentro do `#[cfg(test)]`**, que começa na
linha 495 — é fixture de um banco CronoCAD falso para o teste da importação, não
caminho de produção. O import real (`cronocad_import.rs:442`) chama
`create_time_entry`, então herda a emissão de graça.

Fica registrado em vez de apagado porque a lição é sobre método: `grep` por
`INSERT INTO` acha a linha, e não diz se ela é produção. O que decide é ler onde
o `#[cfg(test)]` começa.

**O que sobra:** um teste que trava a herança
(`a_importacao_do_cronocad_emite_operacao_por_hora`). Ele é guarda de regressão,
não descoberta — passou de primeira, porque a emissão que ele protege já tinha
entrado por `create_time_entry`.

### 2b. O caminho que realmente estava mudo: parar o cronômetro

`stop_timer` grava a sessão com um `INSERT` próprio, sem passar por
`create_time_entry`. É o jeito **mais comum** de uma hora nascer, e emitia zero —
o teste no vermelho mediu `left: 0`.

Emite na mesma transação que grava a sessão e apaga o cronômetro. O
`active_timer` em si continua sem emitir: cronômetro em curso é estado de
máquina, e replicá-lo faria dois PCs disputarem um só.

### 3. Três tabelas acadêmicas têm chave composta, não UUID

`academic_external_refs` (`provider+kind+external_id`), `academic_material_urls`
(`provider+external_id`) e `academic_provider_subject_facts`
(`provider+subject_id`). O `Op` exige `entity.id: Uuid`.

**Decisão:** id derivado por UUID v5 da chave composta, com namespace próprio e
constante — o mesmo precedente que `mos_sync::Relacao` já usa
(`relacao.rs:44`), inclusive a advertência que vem com ele: o namespace nunca
muda, porque mudá-lo faria todas as entidades existentes ganharem ids novos e as
antigas ficarem órfãs.

## Onde a emissão entra

- `tracking_repository.rs` — `time_entries`, `project_tracking`,
  `tracking_settings`
- `cronocad_import.rs` — o `INSERT` em lote da linha 537
- `conversation_repository.rs` — `conversations`, `messages`, `message_parts`
- `academic_repository.rs` e `academic_provider_repository.rs` — os três extras
- os repositórios de `clients`, `daily_*` e `weekly_reviews`

Regra que já vale no repo e continua valendo: emitir **na mesma transação** da
escrita. `work_repository.rs:1300` diz por quê — *"esquecer de emitir tera
esquecido tambem de inserir"*.

## O backfill (migration 0037)

Percorre cada tabela sincronizável e enfileira um `create` por linha existente.
**Não altera nenhuma tabela**: só insere em `sync_outbox`.

**Ordem de dependência**, para não bater em chave estrangeira do outro lado
(`sync_projecao.rs:355-363` descreve o estrago quando isso falha):

1. `clients`
2. `project_tracking` (projects já sincroniza)
3. `time_entries`
4. `conversations` → `messages` → `message_parts`
5. `daily_sessions` → `daily_objectives`, `daily_reflections`
6. `weekly_reviews`
7. os três extras acadêmicos

**Idempotência:** o id de cada op é derivado deterministicamente de
`(kind, entity_id)`, não sorteado. `gravar_op` já usa `INSERT OR IGNORE` na
chave primária, então rodar duas vezes não duplica.

Roda nas duas máquinas, cada uma enfileirando a própria base. No PC de casa são
~165 entidades.

### A consequência que o dono precisa saber

Se a mesma coisa foi criada nos dois PCs de forma independente, são duas
entidades com ids diferentes. O backfill produz as duas, lado a lado. O sync não
funde duplicata — não tem como adivinhar que são a mesma — e resolver isso é
trabalho manual, depois.

## Testes

- Por tipo novo, ida e volta: cria pelo repositório → op emitida → aplica num
  segundo storage → a linha chega igual. É o padrão dos `cenario_*` em
  `mos-sync/src/tests.rs`.
- O teste de cobertura da seção "A costura", incluindo o caminho em lote do
  CronoCAD.
- Um teste do backfill: base povoada → migração → uma op por entidade, na ordem
  de dependência, e rodar de novo não acrescenta nada.

## O que NÃO muda

`crates/mos-sync` não muda. O contrato já aceita tipo novo porque `EntityKind` é
texto e não enum fechado — escolha feita em SYNC.md §9 exatamente para este dia.

Nenhuma tabela existente é alterada. A migration 0037 só insere no `sync_outbox`.
