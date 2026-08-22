-- O rastro que uma reuniao apagada POR FORA do M/OS deixa.
--
-- Em 2026-08-22 o app recusou abrir na maquina do dono com "A migration deixou
-- 50 referencias orfas no banco local". A migration nao tinha deixado nada: as
-- 50 linhas — 48 em `meeting_transcript_index` e 2 em `meeting_search_index` —
-- ja estavam la desde 2026-08-21, e o snapshot `pre-migration-v26` prova.
--
-- O que aconteceu: as duas reunioes daquela noite foram apagadas por fora, e
-- **`PRAGMA foreign_keys` vem DESLIGADO por padrao em todo cliente SQLite** — o
-- `sqlite3`, o DB Browser, um script de tres linhas. So o M/OS liga, no
-- `configure_connection`. Sem ele, o `ON DELETE CASCADE` destas duas tabelas
-- nao dispara, e o indice fica apontando para o vazio.
--
-- Sumiram as tabelas que alguem nomearia — `meetings`, `meeting_segments`,
-- `meeting_search` — e ficaram as duas que sao detalhe interno do FTS. E a
-- verificacao de integridade so roda no caminho da MIGRACAO, entao a sujeira
-- dormiu ate a migration seguinte e trancou a porta num momento arbitrario.
--
-- Esta migration varre o rastro. Ela e idempotente e nao tem o que apagar num
-- banco saudavel — um `DELETE ... WHERE NOT EXISTS` sobre um indice consistente
-- e uma varredura sem baixas.
--
-- As DUAS metades saem juntas. Apagar so a linha do indice deixaria o texto no
-- FTS: a busca devolveria o titulo de uma reuniao que nao existe mais, e o
-- clique nao teria para onde ir. Por isso o `meeting_search` e o
-- `meeting_transcript_search` sao limpos ANTES, enquanto o indice ainda sabe
-- quais rowids eram deles — depois de apagar o indice, esse vinculo se perde e
-- o lixo do FTS vira permanente.

BEGIN IMMEDIATE;

-- 1. O texto do FTS das reunioes que ja nao existem.
DELETE FROM meeting_search
WHERE rowid IN (
    SELECT x.rowid FROM meeting_search_index x
    WHERE NOT EXISTS (SELECT 1 FROM meetings m WHERE m.id = x.meeting_id)
);

DELETE FROM meeting_transcript_search
WHERE rowid IN (
    SELECT x.rowid FROM meeting_transcript_index x
    WHERE NOT EXISTS (SELECT 1 FROM meetings m WHERE m.id = x.meeting_id)
);

-- 2. E so entao o indice.
DELETE FROM meeting_search_index
WHERE NOT EXISTS (
    SELECT 1 FROM meetings m WHERE m.id = meeting_search_index.meeting_id
);

DELETE FROM meeting_transcript_index
WHERE NOT EXISTS (
    SELECT 1 FROM meetings m WHERE m.id = meeting_transcript_index.meeting_id
);

PRAGMA user_version = 30;

COMMIT;
