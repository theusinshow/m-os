-- A fila do que chegou e ainda nao virou linha.
--
-- # Por que ela precisa existir no BANCO
--
-- Ela era um `Vec` em memoria dentro da projecao. O caminho do estrago: a
-- entidade chega, o estado e gravado em `sync_state`, a materializacao falha
-- (chave estrangeira cujo pai ainda nao chegou), a entidade entra na lista, o
-- cursor avanca no mesmo pull, e o app fecha. Na abertura seguinte a lista
-- nasce vazia e ninguem tenta de novo — a entidade fica viva no banco de
-- sincronizacao e invisivel na tela, para sempre.
--
-- Nao e hipotese: `sync_projecao.rs` guardava `pendentes: Vec<(String, Uuid)>`,
-- e `engine.rs` grava o cursor quando a rodada nao tem erro — e falha de
-- materializacao nao marca erro ali.
--
-- # Por que ela nao viaja
--
-- Ela descreve o que ESTA maquina ainda nao conseguiu materializar. Replicada,
-- faria um PC tentar consertar o que o outro nem tem.
CREATE TABLE sync_pendentes (
    entity_kind   TEXT NOT NULL,
    entity_id     TEXT NOT NULL,
    tentativas    INTEGER NOT NULL DEFAULT 0,
    ultimo_erro   TEXT NOT NULL DEFAULT '',
    atualizado_em TEXT NOT NULL,
    PRIMARY KEY (entity_kind, entity_id)
) STRICT;

PRAGMA user_version = 38;
