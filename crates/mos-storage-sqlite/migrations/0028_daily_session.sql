-- Migration 0028: a Daily Session — a camada de intencao sobre o dia.
--
-- Tres tabelas NOVAS, e nenhuma alteracao em tabela existente. Nem uma coluna
-- em `tasks`, nem uma em `projects`. E a regra da §4 do `FEATURE-DEVELOPMENT.md`
-- ("prefira acrescentar tabela a alterar tabela"), e ela vale aqui por um motivo
-- concreto: um objetivo do dia aponta para uma Task, mas a Task nao pertence ao
-- dia — ela existia antes e continua depois. Uma coluna `daily_objective_id` em
-- `tasks` daria a leitura errada e ainda amarraria as duas na mesma migration
-- para sempre.
--
-- Se o desenho da Daily Session mudar, estas tres tabelas se apagam sem tocar em
-- uma linha do que existe.

BEGIN IMMEDIATE;

-- ---------------------------------------------------------------- a sessao
--
-- Um dia de trabalho dentro do M/OS.
CREATE TABLE daily_sessions (
    id              TEXT PRIMARY KEY NOT NULL,

    -- A data CIVIL, `AAAA-MM-DD`, no fuso de quem estava na frente da tela.
    --
    -- O resto do M/OS guarda UTC e deixa o renderer decidir a que dia um
    -- instante pertence — ali isso esta certo, porque um item de calendario nao
    -- tem identidade de dia. Aqui tem: "uma sessao por data" e impossivel de
    -- garantir se cada leitor decidir sozinho que dia e hoje. Quem trabalha ate
    -- 23h30 em UTC-3 esta no dia 21; em UTC ja e dia 22, e o mesmo dia de
    -- trabalho viraria duas sessoes.
    day             TEXT NOT NULL,

    -- So `active` e `completed`. `not_started` existe no dominio como o nome da
    -- AUSENCIA de linha, e gravar uma linha dizendo que ela nao existe seria um
    -- estado que se contradiz.
    status          TEXT NOT NULL,

    -- A justificativa curta de quem montou o dia — hoje, so o Hermes escreve.
    -- Vazio significa nenhuma. NAO e raciocinio: e uma frase de contexto,
    -- limitada no dominio, para a sessao poder explicar de onde veio sem
    -- guardar o caminho que o modelo percorreu.
    note            TEXT NOT NULL DEFAULT '',

    started_at      TEXT NOT NULL,
    ended_at        TEXT,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,

    CONSTRAINT daily_sessions_status_known CHECK (status IN ('active', 'completed')),
    -- Formato exato, e nao "parece uma data": `2026-8-21` e `2026-08-21` sao a
    -- mesma data e duas chaves diferentes, e o indice unico abaixo nao veria a
    -- duplicata. O CHECK e o que impede a segunda sessao do mesmo dia de nascer
    -- por uma diferenca de zero a esquerda.
    CONSTRAINT daily_sessions_day_shape CHECK (
        day GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
    ),
    -- `ended_at` e exclusivo de `completed`: entrar carimba, sair limpa. Mesma
    -- regra que `reminders.completed_at` segue desde a 0015.
    CONSTRAINT daily_sessions_end_stamp CHECK (
        (status = 'completed' AND ended_at IS NOT NULL)
        OR (status <> 'completed' AND ended_at IS NULL)
    )
);

-- UMA sessao por data. E a garantia estrutural de que o dia tem um placar so.
CREATE UNIQUE INDEX daily_sessions_one_per_day ON daily_sessions (day);

-- Achar "o dia aberto" sem varrer o historico inteiro. Parcial porque num banco
-- com anos de dias encerrados so um punhado interessa ao agendamento da tela.
CREATE INDEX daily_sessions_open ON daily_sessions (day)
    WHERE status = 'active';

-- -------------------------------------------------------------- o objetivo
--
-- Algo que a pessoa decidiu que importa hoje. NAO e uma Task: a Task e o
-- trabalho, o objetivo e a decisao sobre ele.
CREATE TABLE daily_objectives (
    id                TEXT PRIMARY KEY NOT NULL,
    session_id        TEXT NOT NULL REFERENCES daily_sessions(id) ON DELETE CASCADE,

    title             TEXT NOT NULL,
    description       TEXT NOT NULL DEFAULT '',

    -- Vinculo polimorfico como par (tipo, id), e nao tabela generica de arestas:
    -- mesma escolha da 0015, pelo mesmo motivo (ADR-012). Sem foreign key
    -- porque uma FK por braco multiplicaria colunas nulas; a integridade fica na
    -- aplicacao, que tambem precisa tratar vinculo ORFAO sem apagar o objetivo
    -- junto — um objetivo continua sendo o registro do que importou naquele dia
    -- mesmo depois de a Task ser apagada.
    --
    -- NAO existe coluna `type`. O tipo do objetivo E a presenca e o tipo deste
    -- vinculo: sem vinculo, intencao livre. Duas colunas para a mesma pergunta
    -- e como as duas versoes divergem.
    link_kind         TEXT,
    link_id           TEXT,

    priority          TEXT NOT NULL,
    status            TEXT NOT NULL,

    -- `position` e nao `order`: `order` e palavra reservada em SQL.
    position          INTEGER NOT NULL DEFAULT 0,

    -- O objetivo de que este veio, quando veio de um carry-over. E o que
    -- permite responder "isto ja foi adiado quatro vezes" sem comparar titulos —
    -- e titulo nao serve de chave, porque a pessoa pode reescrever o objetivo ao
    -- carrega-lo.
    --
    -- SEM cascade: a corrente e historia. `ON DELETE SET NULL` porque remover um
    -- objetivo velho nao pode apagar o novo que veio dele.
    carried_from      TEXT REFERENCES daily_objectives(id) ON DELETE SET NULL,

    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL,
    completed_at      TEXT,

    CONSTRAINT daily_objectives_title_present CHECK (length(trim(title)) > 0),
    -- Vinculo e tudo ou nada: metade de um vinculo e um vinculo que nao resolve.
    CONSTRAINT daily_objectives_link_whole CHECK (
        (link_kind IS NULL AND link_id IS NULL)
        OR (link_kind IS NOT NULL AND link_id IS NOT NULL)
    ),
    CONSTRAINT daily_objectives_link_known CHECK (
        link_kind IS NULL
        OR link_kind IN ('task', 'project', 'capture', 'resource', 'meeting')
    ),
    CONSTRAINT daily_objectives_priority_known CHECK (priority IN ('main', 'secondary')),
    CONSTRAINT daily_objectives_status_known CHECK (
        status IN ('pending', 'completed', 'carried_over', 'dropped')
    ),
    CONSTRAINT daily_objectives_completed_stamp CHECK (
        (status = 'completed' AND completed_at IS NOT NULL)
        OR (status <> 'completed' AND completed_at IS NULL)
    )
);

-- No maximo UM principal por sessao.
--
-- Indice parcial e nao CHECK porque a restricao e sobre a TABELA e nao sobre a
-- linha — mesma forma de `devices_this_one` na 0027. Zero principais e legitimo:
-- um dia so de secundarios e uma escolha, e o principal largado tambem.
CREATE UNIQUE INDEX daily_objectives_one_main ON daily_objectives (session_id)
    WHERE priority = 'main';

CREATE INDEX daily_objectives_by_session ON daily_objectives (session_id, position);

-- Para a conclusao automatica: ao concluir uma Task, achar o objetivo que E
-- aquela Task sem varrer todos os dias ja vividos. Parcial porque so objetivo
-- vinculado participa.
CREATE INDEX daily_objectives_by_link ON daily_objectives (link_kind, link_id)
    WHERE link_kind IS NOT NULL;

-- Para seguir a corrente de carry-over de tras para frente.
CREATE INDEX daily_objectives_chain ON daily_objectives (carried_from)
    WHERE carried_from IS NOT NULL;

-- ------------------------------------------------------------- a reflexao
--
-- O fecho opcional do dia. Uma linha por sessao, e por isso a chave primaria e a
-- sessao: duas reflexoes sobre o mesmo dia seriam duas respostas para uma
-- pergunta que so tem uma.
--
-- NAO vira Capture: a Inbox e uma fila de coisas por PROCESSAR, e uma reflexao
-- arquivada la pediria uma decisao que ela nao tem.
CREATE TABLE daily_reflections (
    session_id      TEXT PRIMARY KEY NOT NULL
                    REFERENCES daily_sessions(id) ON DELETE CASCADE,
    -- NULL significa "nao respondeu", que e diferente de "dia normal".
    mood            TEXT,
    summary         TEXT NOT NULL DEFAULT '',
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,

    CONSTRAINT daily_reflections_mood_known CHECK (
        mood IS NULL OR mood IN ('productive', 'normal', 'blocked')
    )
);

PRAGMA user_version = 28;

COMMIT;
