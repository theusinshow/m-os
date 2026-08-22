-- O que a INSTITUICAO diz sobre a disciplina.
--
-- # Por que isto nao esta na 0032, onde nasceu
--
-- Esta tabela foi escrita junto com a 0032, e a 0032 rodou ANTES dela existir:
-- o `tauri dev` recompila e reinicia a cada mudanca em Rust, e um desses
-- reinicios aplicou a 0032 com tres tabelas em vez de quatro. O banco ficou com
-- `user_version = 32` e sem esta tabela — e um banco em 32 nunca mais roda a 32.
--
-- Renumerar a 0032 nao resolveria: e a saida quando a migration nunca rodou na
-- maquina de ninguem (e o que os comentarios da 0020 e da 0025 registram), e
-- aqui ela rodou. Um banco que ja viu a 0032 antiga nao pode ser distinguido de
-- um que viu a nova, entao a saida e a outra: uma migration de conserto, com o
-- motivo escrito.
--
-- # Por que a media oficial nao mora em `academic_subjects`
--
-- O ADR-058: a media do M/OS e DERIVADA das avaliacoes, em
-- `mos_core::academic::desempenho`. A media da instituicao e outra conta — ela
-- conhece regra de exame e de recuperacao que o M/OS nao modela — e as duas
-- discordam de proposito. Gravar a oficial numa coluna de `academic_subjects`
-- criaria a terceira fonte que o ADR eliminou: alguem leria o campo, alguem
-- leria a funcao, e nada no banco diria qual das duas responde "como estou nesta
-- materia".
--
-- Aqui ela e o que e: um FATO DO PROVEDOR sobre a disciplina, exibivel ao lado
-- da media propria e jamais no lugar dela.

BEGIN IMMEDIATE;

CREATE TABLE academic_provider_subject_facts (
    provider        TEXT NOT NULL,
    subject_id      TEXT NOT NULL REFERENCES academic_subjects(id) ON DELETE CASCADE,
    situation       TEXT NOT NULL DEFAULT '',
    official_grade  REAL,
    updated_at      TEXT NOT NULL,

    PRIMARY KEY (provider, subject_id),

    CONSTRAINT academic_provider_grade_range
        CHECK (official_grade IS NULL OR (official_grade >= 0 AND official_grade <= 100))
) STRICT;

CREATE INDEX academic_provider_subject_facts_por_disciplina
    ON academic_provider_subject_facts(subject_id);

PRAGMA user_version = 33;

COMMIT;
