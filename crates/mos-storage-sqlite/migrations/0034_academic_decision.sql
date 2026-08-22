-- A decisao da pessoa sobre um compromisso academico.
--
-- # O conflito que esta migration resolve
--
-- `academic_assignments.status` e `academic_exams.status` passaram a ser
-- escritos pelo sync do Univirtus (0032). Isso esta certo: eles descrevem o
-- FATO ACADEMICO — o portal diz se a atividade foi entregue, se a prova foi
-- feita, se saiu nota.
--
-- Mas a pessoa tambem precisa dizer coisas que o portal nao sabe:
--
--   "ja entreguei, o portal e que ainda nao atualizou";
--   "essa eu nao vou fazer";
--   "essa eu vou fazer sabado as 19h".
--
-- Enfiar isso em `status` faria o sync apagar a decisao a cada rodada. Duas
-- colunas para o mesmo fato tambem nao serve: nada diria qual manda. A saida e
-- separar os dois VOCABULARIOS — `status` responde "o que o portal registra",
-- `decision` responde "o que eu resolvi" — e deixar o sync tocar so no
-- primeiro. Os UPDATE de `academic_provider_repository.rs` listam colunas
-- explicitamente, e nenhum deles nomeia `decision`, `decided_at` ou `planned_at`.
--
-- # Por que so tres decisoes
--
-- `none`, `done`, `skipped`. Nao ha `planned` porque planejado nao e uma
-- decisao: e um FATO derivado de `planned_at` existir. Guardar um estado
-- "planned" ao lado da data criaria a linha que diz "planejado" sem data, e a
-- que diz "nao planejado" com data — e o sistema passaria a depender de alguem
-- manter as duas em acordo. E a mesma escolha do `status` de semestre na 0031.
--
-- Nao ha `ignored` separado de `skipped`. A diferenca ("nao vou fazer" versus
-- "isso nem me diz respeito") nao muda nada no comportamento: os dois saem da
-- atencao, ficam no historico e podem voltar. Um estado a mais que nao muda
-- nada e um estado a mais para explicar.
--
-- # Por que `planned_at` mora aqui, e nao na Task
--
-- A `Task` do M/OS nao tem data planejada nem prioridade — e a ADR-058 ja
-- registrou que promove-las e outra feature. O momento planejado e uma decisao
-- SOBRE O COMPROMISSO ACADEMICO, e nao sobre a Task: a Task e o braco executor,
-- e continua existindo com ou sem plano. Guardar aqui tambem mantem o plano de
-- pe quando a Task e apagada.

BEGIN IMMEDIATE;

ALTER TABLE academic_assignments
    ADD COLUMN decision TEXT NOT NULL DEFAULT 'none';
ALTER TABLE academic_assignments
    ADD COLUMN decided_at TEXT;
-- O instante em que a pessoa pretende FAZER. Diferente de `due_at`, que e
-- quando o prazo fecha. Confundir os dois e o erro que faz o calendario mostrar
-- "entregar APOL" as 23h59 de sexta quando a pessoa vai escrever na quarta.
ALTER TABLE academic_assignments
    ADD COLUMN planned_at TEXT;
-- Quanto tempo a pessoa reservou. Zero significa "sem duracao definida", que e
-- diferente de zero minuto.
ALTER TABLE academic_assignments
    ADD COLUMN planned_minutes INTEGER NOT NULL DEFAULT 0;

ALTER TABLE academic_exams
    ADD COLUMN decision TEXT NOT NULL DEFAULT 'none';
ALTER TABLE academic_exams
    ADD COLUMN decided_at TEXT;
ALTER TABLE academic_exams
    ADD COLUMN planned_at TEXT;
ALTER TABLE academic_exams
    ADD COLUMN planned_minutes INTEGER NOT NULL DEFAULT 0;

-- Os CHECK nao entram por ALTER TABLE no SQLite. A alternativa seria recriar as
-- duas tabelas inteiras — com todos os indices, todas as FK e o risco que
-- reescrever uma tabela com dados carrega. O dominio ja recusa valor
-- desconhecido em `Decision::parse`, e o repositorio so escreve `as_str()`; o
-- ganho de um CHECK aqui nao paga a reescrita.
--
-- Um indice, sim: a lista "o que precisa de mim" filtra por decisao em toda
-- abertura da tela.
CREATE INDEX academic_assignments_por_decisao
    ON academic_assignments(decision, lifecycle_state, due_at);
CREATE INDEX academic_exams_por_decisao
    ON academic_exams(decision, lifecycle_state, at);
CREATE INDEX academic_assignments_planejadas
    ON academic_assignments(planned_at) WHERE planned_at IS NOT NULL;
CREATE INDEX academic_exams_planejadas
    ON academic_exams(planned_at) WHERE planned_at IS NOT NULL;

PRAGMA user_version = 34;

COMMIT;
