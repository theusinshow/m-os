-- Anotacoes e pendencias por projeto.
--
-- `projects.notes`: bloco de texto livre, 1-para-1 com o projeto (contexto solto
-- que o usuario re-le ao voltar ao trabalho).
--
-- `project_todos`: pendencias curtas. Hard delete de proposito (sem `deleted_at`):
-- uma pendencia nao e registro de tempo nem gera cobranca — a regra de preservar
-- historico protege `time_entries`, nao um item de checklist.

ALTER TABLE projects ADD COLUMN notes TEXT;

CREATE TABLE project_todos (
  id          TEXT PRIMARY KEY,
  project_id  TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  text        TEXT NOT NULL,
  done        INTEGER NOT NULL DEFAULT 0 CHECK (done IN (0, 1)),
  done_at     TEXT,
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
);

CREATE INDEX idx_project_todos_project ON project_todos(project_id);
CREATE INDEX idx_project_todos_done ON project_todos(done);
