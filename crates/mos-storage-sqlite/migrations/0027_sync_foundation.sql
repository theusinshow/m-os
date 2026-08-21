-- Migration 0027: a fundacao de sincronizacao multi-dispositivo.
--
-- Esta migration NAO liga a sincronizacao. Ela cria o lugar onde a
-- sincronizacao vai morar, e faz isso agora por um motivo especifico: o dia em
-- que o iPhone existir, o banco do desktop ja precisa saber de onde cada coisa
-- veio. Adicionar a origem depois significaria olhar para meses de dados sem
-- resposta para "quem criou isto?" — e essa resposta nao se recupera.
--
-- Nada aqui altera tabela existente. Nenhuma coluna nova em `tasks`, `captures`
-- ou `projects`, nenhuma foreign key nova sobre elas. A razao e o §62 e o §75:
-- o desktop tem dados de verdade e nao pode regredir por causa de uma feature
-- que ainda nao tem cliente do outro lado. Se a sincronizacao mudar de desenho,
-- estas tres tabelas se apagam sem tocar em uma linha do que existe.
--
-- Por que UUID v7 nao precisou mudar: todo id do M/OS ja e v7, que e ordenavel
-- por tempo e nao colide entre maquinas. Foi a decisao mais barata desta missao
-- inteira, e ela ja estava tomada.

BEGIN IMMEDIATE;

-- Os dispositivos conhecidos.
--
-- O proprio aparelho tambem tem linha aqui: `is_this_device` responde "qual
-- sou eu?" sem depender de um arquivo de configuracao paralelo, que seria uma
-- segunda fonte de verdade sobre a mesma pergunta.
CREATE TABLE devices (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    platform TEXT NOT NULL,
    app_version TEXT NOT NULL DEFAULT '',
    -- ISO-8601. Vazio significa "nunca sincronizou", que e diferente de
    -- "sincronizou na epoca zero".
    last_sync_at TEXT NOT NULL DEFAULT '',
    is_this_device INTEGER NOT NULL DEFAULT 0 CHECK (is_this_device IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- So um dispositivo pode ser este. Indice parcial em vez de CHECK porque a
-- restricao e sobre a TABELA, e nao sobre a linha.
CREATE UNIQUE INDEX devices_this_one ON devices (is_this_device)
WHERE is_this_device = 1;

-- A fila de saida: o que este dispositivo mudou e ainda nao confirmou.
--
-- `id` e a chave de idempotencia, e nasce na origem antes de qualquer envio. E
-- ele que faz um retry aplicar uma vez so (§53), e e o mesmo id que as acoes do
-- Hermes usam (§78).
--
-- `payload` guarda a operacao inteira em JSON, no formato de `mos-sync`. Nao
-- normalizamos os campos em colunas de proposito: o formato pertence ao
-- contrato, que versiona por conta propria, e espalhar o contrato pelo schema
-- faria toda mudanca de contrato virar migration.
CREATE TABLE sync_outbox (
    id TEXT PRIMARY KEY,
    entity_kind TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    -- Os tres campos do HLC, em colunas, porque a ORDEM se consulta: o envio
    -- sai em ordem de instante, e ordenar por JSON custaria uma varredura.
    hlc_wall_ms INTEGER NOT NULL,
    hlc_counter INTEGER NOT NULL,
    hlc_device TEXT NOT NULL,
    payload TEXT NOT NULL,
    -- pending, sent, acked, failed. Texto e nao enum porque um cliente antigo
    -- precisa conseguir ler uma linha escrita por um mais novo.
    status TEXT NOT NULL DEFAULT 'pending',
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX sync_outbox_pendentes ON sync_outbox (status, hlc_wall_ms, hlc_counter);
CREATE INDEX sync_outbox_entidade ON sync_outbox (entity_kind, entity_id);

-- Escritas concorrentes no mesmo campo, com o lado perdedor guardado.
--
-- Esta tabela e o §8 virando estrutura: o valor que perdeu NAO e descartado. Se
-- um dia a interface precisar perguntar "voce quis dizer isto ou aquilo?", o
-- dado necessario existe. Sem ela, resolver conflito seria escolher um e apagar
-- o outro em silencio, que e o que a missao proibe.
CREATE TABLE sync_conflicts (
    id TEXT PRIMARY KEY,
    entity_kind TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    field TEXT NOT NULL,
    winner_value TEXT NOT NULL,
    winner_device TEXT NOT NULL,
    winner_wall_ms INTEGER NOT NULL,
    loser_value TEXT NOT NULL,
    loser_device TEXT NOT NULL,
    loser_wall_ms INTEGER NOT NULL,
    -- Vazio enquanto ninguem olhou. Um conflito reconhecido nao some: ele deixa
    -- de pedir atencao, que e outra coisa.
    acknowledged_at TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL
);

CREATE INDEX sync_conflicts_abertos ON sync_conflicts (acknowledged_at, created_at);

-- O estado do relogio logico deste dispositivo, entre execucoes.
--
-- Sem persistir, reabrir o app com o relogio de parede atrasado geraria eventos
-- que se ordenam antes de coisas ja sincronizadas. Uma linha so, e por isso o
-- CHECK: nao existe "o relogio de outro dispositivo" aqui dentro.
CREATE TABLE sync_clock (
    only_row INTEGER PRIMARY KEY CHECK (only_row = 1),
    hlc_wall_ms INTEGER NOT NULL,
    hlc_counter INTEGER NOT NULL,
    hlc_device TEXT NOT NULL,
    -- Ate onde este dispositivo ja recebeu do outro lado. E o cursor do §43:
    -- o custo do sync cresce com o que mudou, e nao com o tamanho da base.
    pull_cursor TEXT NOT NULL DEFAULT '',
    updated_at TEXT NOT NULL
);

PRAGMA user_version = 27;

COMMIT;
