# Adicionar Tempo Esquecido — Plano de Implementacao

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Permitir registrar tempo esquecido informando a **duracao** (botoes `+15min`, `+30min`, `+1h`, `+2h`) em vez de digitar horarios de inicio e fim.

**Architecture:** Mudanca 100% frontend. Uma funcao pura (`src/lib/quickTime.ts`) converte "duracao + dia" em `startedAt`/`endedAt` ISO; um modal (`src/features/history/QuickTimeModal.tsx`) coleta projeto/total/dia/nota e grava pelo caminho que ja existe (`entriesStore.create` -> `createTimeEntry` -> `create_time_entry`), com `source: "manual"`. O modal e aberto de tres lugares: o painel do cronometro parado, o cabecalho do Historico e uma acao por linha da tabela do Historico.

**Tech Stack:** React 18, TypeScript estrito, Tailwind, Vitest + @testing-library/react, Zustand.

Spec: `docs/superpowers/specs/2026-07-16-tempo-esquecido-design.md`

## Global Constraints

- TypeScript estrito. **`any` e proibido** (o ESLint erra).
- `npm run lint` deve passar com **0 warnings**.
- Import alias `@/` -> `src/`.
- Textos da UI em portugues, **sem acentos** (o codebase inteiro segue isso: "Cronometro", "Sessao", "Duracao"). Manter o padrao.
- **Nenhuma mudanca no backend.** Sem migration, sem comando Tauri novo, sem Rust. O `source` ja aceita `'manual'` (`src-tauri/migrations/0001_initial_schema.sql:62-63`) e o `create_time_entry` ja existe.
- **Duracoes em segundos** (regra critica 7). Minutos sao apenas apresentacao.
- O `Button` tem exatamente 4 variantes: `primary` | `secondary` | `ghost` | `danger` (`src/components/ui/Button.tsx:4`). Nao criar novas.
- Nos testes, mockar a **camada de servico** (`@/services/*`) e semear estado com `useStore.setState(...)`. Nunca mockar os stores. Padrao ja estabelecido em `src/features/timer/TimerPanel.test.tsx:7-23`.
- **Nao tocar na sessao original** do cronometro em nenhum caminho (regra critica 5). Todo tempo adicionado nasce como um registro `manual` novo.

---

### Task 1: `resolveQuickEntryWindow` — duracao vira janela de tempo

**Files:**
- Create: `src/lib/quickTime.ts`
- Test: `src/lib/quickTime.test.ts`

**Interfaces:**
- Consumes: tipo `TimeEntry` (`@/types/domain`), `isoToDateInput` (`@/lib/datetime`).
- Produces:
  ```ts
  export const MAX_QUICK_SECONDS: number;      // 24 * 3600
  export const QUICK_INCREMENTS: ReadonlyArray<{ label: string; seconds: number }>;

  export interface QuickEntryWindowInput {
    durationSeconds: number;
    day: string;                 // "YYYY-MM-DD" local
    anchorEndIso?: string | null;
    dayEntries: TimeEntry[];
    now: Date;
  }
  export interface QuickEntryWindow {
    startedAt: string;           // ISO UTC
    endedAt: string;             // ISO UTC
  }
  export function resolveQuickEntryWindow(input: QuickEntryWindowInput): QuickEntryWindow;
  export function clampQuickSeconds(seconds: number): number;
  ```

`now` e **injetado**, nunca lido de dentro da funcao — e isso que torna as regras testaveis.

- [ ] **Step 1: Escrever o teste que falha**

Criar `src/lib/quickTime.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import type { TimeEntry } from "@/types/domain";
import { clampQuickSeconds, resolveQuickEntryWindow } from "./quickTime";

/** Cria uma sessao minima para os testes; so importam projectId/startedAt/endedAt. */
function entry(startedAt: string, endedAt: string): TimeEntry {
  return {
    id: `e-${startedAt}`,
    projectId: "p1",
    startedAt,
    endedAt,
    durationSeconds: 0,
    idleSeconds: 0,
    description: null,
    activityType: "drawing",
    billable: true,
    hourlyRateSnapshotCents: 0,
    source: "timer",
    createdAt: startedAt,
    updatedAt: startedAt,
    deletedAt: null,
  };
}

/** Local -> Date, sem depender do fuso da maquina que roda o teste. */
function local(day: string, time: string): Date {
  return new Date(`${day}T${time}`);
}

const HOUR = 3600;

describe("resolveQuickEntryWindow", () => {
  it("ancorado: o bloco termina no fim da sessao ancora", () => {
    const anchorEnd = local("2026-07-16", "11:30:00").toISOString();
    const w = resolveQuickEntryWindow({
      durationSeconds: HOUR,
      day: "2026-07-16",
      anchorEndIso: anchorEnd,
      dayEntries: [],
      now: local("2026-07-16", "20:00:00"),
    });

    expect(w.endedAt).toBe(anchorEnd);
    expect(w.startedAt).toBe(local("2026-07-16", "10:30:00").toISOString());
  });

  it("hoje: o bloco termina agora", () => {
    const now = local("2026-07-16", "15:00:00");
    const w = resolveQuickEntryWindow({
      durationSeconds: 2 * HOUR,
      day: "2026-07-16",
      dayEntries: [],
      now,
    });

    expect(w.endedAt).toBe(now.toISOString());
    expect(w.startedAt).toBe(local("2026-07-16", "13:00:00").toISOString());
  });

  it("dia passado com sessoes: termina no fim da ultima sessao daquele dia", () => {
    const w = resolveQuickEntryWindow({
      durationSeconds: HOUR,
      day: "2026-07-14",
      dayEntries: [
        entry(
          local("2026-07-14", "08:00:00").toISOString(),
          local("2026-07-14", "10:00:00").toISOString(),
        ),
        entry(
          local("2026-07-14", "13:00:00").toISOString(),
          local("2026-07-14", "16:45:00").toISOString(),
        ),
        // Sessao de outro dia: deve ser ignorada.
        entry(
          local("2026-07-15", "09:00:00").toISOString(),
          local("2026-07-15", "23:00:00").toISOString(),
        ),
      ],
      now: local("2026-07-16", "15:00:00"),
    });

    expect(w.endedAt).toBe(local("2026-07-14", "16:45:00").toISOString());
    expect(w.startedAt).toBe(local("2026-07-14", "15:45:00").toISOString());
  });

  it("dia passado vazio: termina as 18:00 locais", () => {
    const w = resolveQuickEntryWindow({
      durationSeconds: 3 * HOUR,
      day: "2026-07-14",
      dayEntries: [],
      now: local("2026-07-16", "15:00:00"),
    });

    expect(w.endedAt).toBe(local("2026-07-14", "18:00:00").toISOString());
    expect(w.startedAt).toBe(local("2026-07-14", "15:00:00").toISOString());
  });

  it("atravessa a meia-noite: ancorado as 00:30, 3h comecam no dia anterior", () => {
    const w = resolveQuickEntryWindow({
      durationSeconds: 3 * HOUR,
      day: "2026-07-16",
      anchorEndIso: local("2026-07-16", "00:30:00").toISOString(),
      dayEntries: [],
      now: local("2026-07-16", "09:00:00"),
    });

    expect(w.startedAt).toBe(local("2026-07-15", "21:30:00").toISOString());
    expect(new Date(w.startedAt).getDate()).toBe(15);
  });

  it("o inicio e sempre anterior ao fim, pela duracao exata", () => {
    const w = resolveQuickEntryWindow({
      durationSeconds: 90 * 60,
      day: "2026-07-16",
      dayEntries: [],
      now: local("2026-07-16", "15:00:00"),
    });

    const delta =
      (new Date(w.endedAt).getTime() - new Date(w.startedAt).getTime()) / 1000;
    expect(delta).toBe(90 * 60);
  });

  it("ignora sessoes sem fim ao procurar a ultima do dia", () => {
    const openEnded = { ...entry("x", "y"), endedAt: null };
    const w = resolveQuickEntryWindow({
      durationSeconds: HOUR,
      day: "2026-07-14",
      dayEntries: [
        { ...openEnded, startedAt: local("2026-07-14", "20:00:00").toISOString() },
      ],
      now: local("2026-07-16", "15:00:00"),
    });

    // Sem sessao com fim, cai na regra do dia vazio.
    expect(w.endedAt).toBe(local("2026-07-14", "18:00:00").toISOString());
  });
});

describe("clampQuickSeconds", () => {
  it("tem piso em zero", () => {
    expect(clampQuickSeconds(-900)).toBe(0);
  });

  it("tem teto de 24h", () => {
    expect(clampQuickSeconds(30 * HOUR)).toBe(24 * HOUR);
  });

  it("mantem valores validos", () => {
    expect(clampQuickSeconds(2 * HOUR)).toBe(2 * HOUR);
  });
});
```

- [ ] **Step 2: Rodar o teste e confirmar que falha**

Run: `npm run test -- quickTime`
Expected: FAIL — "Failed to resolve import ./quickTime" (o arquivo ainda nao existe).

- [ ] **Step 3: Implementar a funcao**

Criar `src/lib/quickTime.ts`:

```ts
/**
 * Converte "duracao + dia" na janela (inicio/fim) que o banco exige.
 *
 * O usuario que esqueceu de ligar o cronometro lembra da duracao ("umas duas
 * horas"), nao do horario exato. Exigir horario e o que faz o registro ser
 * adiado — e adiado vira esquecido. Entao ancoramos o bloco num ponto plausivel
 * e derivamos o inicio de tras para frente.
 *
 * `now` e sempre injetado para as regras serem testaveis.
 */

import type { TimeEntry } from "@/types/domain";
import { isoToDateInput } from "./datetime";

/** Teto por lancamento. Acima disso e quase certamente engano. */
export const MAX_QUICK_SECONDS = 24 * 3600;

/** Hora do fim para um dia passado sem nenhuma sessao registrada. */
const FALLBACK_END_HOUR = 18;

/** Incrementos oferecidos pelo modal, em segundos. */
export const QUICK_INCREMENTS: ReadonlyArray<{
  label: string;
  seconds: number;
}> = [
  { label: "+15min", seconds: 15 * 60 },
  { label: "+30min", seconds: 30 * 60 },
  { label: "+1h", seconds: 3600 },
  { label: "+2h", seconds: 2 * 3600 },
];

export interface QuickEntryWindowInput {
  durationSeconds: number;
  /** "YYYY-MM-DD" no horario local. */
  day: string;
  /** Fim da sessao ancora, quando o tempo e um ajuste de sessao existente. */
  anchorEndIso?: string | null;
  /** Sessoes ja existentes (a funcao filtra as do dia pedido). */
  dayEntries: TimeEntry[];
  now: Date;
}

export interface QuickEntryWindow {
  /** ISO UTC. */
  startedAt: string;
  /** ISO UTC. */
  endedAt: string;
}

/** Mantem a duracao entre 0 e o teto de 24h. */
export function clampQuickSeconds(seconds: number): number {
  if (!Number.isFinite(seconds)) return 0;
  return Math.min(Math.max(0, Math.floor(seconds)), MAX_QUICK_SECONDS);
}

/** Fim da ultima sessao encerrada naquele dia local, ou null. */
function lastEndOfDay(entries: TimeEntry[], day: string): Date | null {
  let latest: Date | null = null;
  for (const e of entries) {
    if (e.endedAt === null) continue;
    if (isoToDateInput(e.startedAt) !== day) continue;
    const end = new Date(e.endedAt);
    if (latest === null || end.getTime() > latest.getTime()) latest = end;
  }
  return latest;
}

/** Aplica a primeira regra de ancoragem que casar (a ordem importa). */
function resolveEnd(input: QuickEntryWindowInput): Date {
  const { day, anchorEndIso, dayEntries, now } = input;

  if (anchorEndIso) return new Date(anchorEndIso);
  if (isoToDateInput(now.toISOString()) === day) return now;

  const last = lastEndOfDay(dayEntries, day);
  if (last) return last;

  return new Date(`${day}T${String(FALLBACK_END_HOUR).padStart(2, "0")}:00:00`);
}

export function resolveQuickEntryWindow(
  input: QuickEntryWindowInput,
): QuickEntryWindow {
  const end = resolveEnd(input);
  const seconds = clampQuickSeconds(input.durationSeconds);
  const start = new Date(end.getTime() - seconds * 1000);
  return { startedAt: start.toISOString(), endedAt: end.toISOString() };
}
```

- [ ] **Step 4: Rodar os testes e confirmar que passam**

Run: `npm run test -- quickTime`
Expected: PASS — 10 testes (7 de `resolveQuickEntryWindow`, 3 de `clampQuickSeconds`).

- [ ] **Step 5: Commit**

```bash
git add src/lib/quickTime.ts src/lib/quickTime.test.ts
git commit -m "feat(lib): converter duracao esquecida em janela de tempo"
```

---

### Task 2: `QuickTimeModal`

**Files:**
- Create: `src/features/history/QuickTimeModal.tsx`
- Test: `src/features/history/QuickTimeModal.test.tsx`

**Interfaces:**
- Consumes: `resolveQuickEntryWindow`, `clampQuickSeconds`, `QUICK_INCREMENTS` (Task 1 — assinaturas no bloco "Produces" daquela task; o teto de 24h ja vem aplicado dentro do `clampQuickSeconds`, o modal nao precisa do `MAX_QUICK_SECONDS`); `Modal`, `Button`, `Field`/`Input`/`Select` (`@/components/ui/*`); `useEntriesStore` (`@/stores/entriesStore`), `useCatalogStore` (`@/stores/catalogStore`); `formatDuration` (`@/lib/format`), `isoToDateInput` (`@/lib/datetime`); tipo `TimeEntry` (`@/types/domain`).
- Produces:
  ```ts
  export interface QuickTimeModalProps {
    open: boolean;
    onClose: () => void;
    /** Sessao ancora: trava projeto e dia, e cola o ajuste no fim dela. */
    anchor?: TimeEntry | null;
    /** Projeto pre-selecionado quando nao ha ancora. */
    defaultProjectId?: string;
  }
  export function QuickTimeModal(props: QuickTimeModalProps): JSX.Element;
  ```

O modal **grava direto** pelo `entriesStore` (mesmo padrao do `EntryForm.tsx:36-104`, que tambem chama `create`/`update` de dentro do componente). Nao e apresentacional.

- [ ] **Step 1: Escrever o teste que falha**

Criar `src/features/history/QuickTimeModal.test.tsx`:

```tsx
import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Project } from "@/types/domain";

vi.mock("@/services/timeEntries", () => ({
  listTimeEntries: vi.fn().mockResolvedValue([]),
  createTimeEntry: vi.fn(),
  updateTimeEntry: vi.fn(),
  deleteTimeEntry: vi.fn(),
  restoreTimeEntry: vi.fn(),
}));

import * as entriesService from "@/services/timeEntries";
import { useEntriesStore } from "@/stores/entriesStore";
import { useCatalogStore } from "@/stores/catalogStore";
import { QuickTimeModal } from "./QuickTimeModal";

const project: Project = {
  id: "p1",
  clientId: null,
  name: "Residencial Aurora",
  code: "083-22",
  description: null,
  hourlyRateCents: 9000,
  budgetMinutes: 0,
  status: "active",
  color: null,
  notes: null,
  createdAt: "2026-07-11T08:00:00Z",
  updatedAt: "2026-07-11T08:00:00Z",
  archivedAt: null,
};

function renderModal() {
  const onClose = vi.fn();
  render(<QuickTimeModal open onClose={onClose} defaultProjectId="p1" />);
  return { onClose };
}

const click = (name: RegExp) =>
  userEvent.click(screen.getByRole("button", { name }));

describe("QuickTimeModal", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useCatalogStore.setState({ projects: [project] });
    useEntriesStore.setState({ entries: [], loaded: true, error: null });
  });

  it("os incrementos acumulam num total", async () => {
    renderModal();
    await click(/^\+30min$/);
    await click(/^\+1h$/);

    expect(screen.getByTestId("quick-total")).toHaveTextContent("1h 30min");
  });

  it("-15min nao deixa o total ficar negativo", async () => {
    renderModal();
    await click(/^-15min$/);

    expect(screen.getByTestId("quick-total")).toHaveTextContent("0s");
  });

  it("Limpar zera o total", async () => {
    renderModal();
    await click(/^\+2h$/);
    await click(/^Limpar$/);

    expect(screen.getByTestId("quick-total")).toHaveTextContent("0s");
  });

  it("nao deixa salvar com total zerado", () => {
    renderModal();
    expect(screen.getByRole("button", { name: /^Salvar$/ })).toBeDisabled();
  });

  it("salva um registro manual com a duracao pedida", async () => {
    vi.mocked(entriesService.createTimeEntry).mockResolvedValue({
      id: "e1",
    } as never);
    const { onClose } = renderModal();

    await click(/^\+2h$/);
    await click(/^\+30min$/);
    await click(/^Salvar$/);

    expect(entriesService.createTimeEntry).toHaveBeenCalledOnce();
    const input = vi.mocked(entriesService.createTimeEntry).mock.calls[0][0];

    expect(input.projectId).toBe("p1");
    expect(input.source).toBe("manual");
    expect(input.idleSeconds).toBe(0);
    const delta =
      (new Date(input.endedAt).getTime() - new Date(input.startedAt).getTime()) /
      1000;
    expect(delta).toBe(2.5 * 3600);
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("erro ao salvar mantem o modal aberto, com o total preservado", async () => {
    vi.mocked(entriesService.createTimeEntry).mockRejectedValue(
      new Error("banco indisponivel"),
    );
    const { onClose } = renderModal();

    await click(/^\+1h$/);
    await click(/^Salvar$/);

    expect(screen.getByText(/Falha ao adicionar o tempo/i)).toBeInTheDocument();
    expect(screen.getByTestId("quick-total")).toHaveTextContent("1h 00min");
    expect(onClose).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Rodar o teste e confirmar que falha**

Run: `npm run test -- QuickTimeModal`
Expected: FAIL — "Failed to resolve import ./QuickTimeModal".

- [ ] **Step 3: Implementar o componente**

Criar `src/features/history/QuickTimeModal.tsx`:

```tsx
import { useState } from "react";
import type { TimeEntry } from "@/types/domain";
import { useEntriesStore } from "@/stores/entriesStore";
import { useCatalogStore } from "@/stores/catalogStore";
import {
  clampQuickSeconds,
  QUICK_INCREMENTS,
  resolveQuickEntryWindow,
} from "@/lib/quickTime";
import { isoToDateInput } from "@/lib/datetime";
import { formatDuration } from "@/lib/format";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";
import { Field, Input, Select } from "@/components/ui/Field";

export interface QuickTimeModalProps {
  open: boolean;
  onClose: () => void;
  /** Sessao ancora: trava projeto e dia, e cola o ajuste no fim dela. */
  anchor?: TimeEntry | null;
  /** Projeto pre-selecionado quando nao ha ancora. */
  defaultProjectId?: string;
}

/**
 * Registra tempo esquecido por duracao, nao por horario.
 *
 * O `EntryForm` ja cria sessoes manuais, mas exige inicio e fim em
 * `datetime-local`. Quem esqueceu de ligar o cronometro nao lembra do horario —
 * lembra da duracao. Este modal e o caminho curto: projeto, total, dia, nota.
 *
 * Com `anchor`, o tempo vira um registro **separado** colado no fim da sessao
 * indicada; a sessao original nunca e alterada (regra critica 5), para o
 * historico continuar distinguindo o cronometrado do estimado.
 */
export function QuickTimeModal({
  open,
  onClose,
  anchor,
  defaultProjectId,
}: QuickTimeModalProps) {
  const projects = useCatalogStore((s) => s.projects);
  const entries = useEntriesStore((s) => s.entries);
  const create = useEntriesStore((s) => s.create);

  const [projectId, setProjectId] = useState("");
  const [seconds, setSeconds] = useState(0);
  const [day, setDay] = useState("");
  const [note, setNote] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  // Mesmo padrao de reinicializacao do EntryForm.tsx:51-76.
  const [initializedFor, setInitializedFor] = useState<string | null>(null);
  const key = anchor?.id ?? `new-${defaultProjectId ?? ""}`;
  if (open && initializedFor !== key) {
    setProjectId(anchor?.projectId ?? defaultProjectId ?? "");
    setDay(
      isoToDateInput(anchor?.startedAt ?? new Date().toISOString()),
    );
    setSeconds(0);
    setNote("");
    setError(null);
    setInitializedFor(key);
  }
  if (!open && initializedFor !== null) setInitializedFor(null);

  const locked = Boolean(anchor);
  const anchorProjectName =
    projects.find((p) => p.id === anchor?.projectId)?.name ?? "Projeto";

  function add(delta: number) {
    setSeconds((s) => clampQuickSeconds(s + delta));
  }

  async function handleSave() {
    setSaving(true);
    setError(null);
    try {
      if (!projectId) throw "Selecione um projeto.";
      const { startedAt, endedAt } = resolveQuickEntryWindow({
        durationSeconds: seconds,
        day,
        anchorEndIso: anchor?.endedAt ?? null,
        dayEntries: entries,
        now: new Date(),
      });
      await create({
        projectId,
        startedAt,
        endedAt,
        description: note || null,
        activityType: "drawing",
        billable: true,
        idleSeconds: 0,
        source: "manual",
      });
      onClose();
    } catch (err) {
      setError(
        typeof err === "string" ? err : "Falha ao adicionar o tempo.",
      );
    } finally {
      setSaving(false);
    }
  }

  return (
    <Modal
      open={open}
      title="Adicionar tempo esquecido"
      onClose={onClose}
      footer={
        <>
          <Button variant="ghost" onClick={onClose} type="button">
            Cancelar
          </Button>
          <Button
            variant="primary"
            type="button"
            onClick={() => void handleSave()}
            disabled={saving || seconds === 0}
          >
            {saving ? "Salvando…" : "Salvar"}
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        {locked ? (
          <Field label="Projeto" htmlFor="q-project">
            <Input id="q-project" value={anchorProjectName} disabled />
          </Field>
        ) : (
          <Field label="Projeto" htmlFor="q-project" required>
            <Select
              id="q-project"
              value={projectId}
              onChange={(e) => setProjectId(e.target.value)}
              required
            >
              <option value="">Selecione…</option>
              {projects.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.code ? `${p.code} · ${p.name}` : p.name}
                </option>
              ))}
            </Select>
          </Field>
        )}

        <div className="rounded border border-border bg-surface-raised py-5 text-center">
          <p
            data-testid="quick-total"
            className="tabular text-4xl font-semibold tracking-tight text-text"
          >
            {formatDuration(seconds)}
          </p>
          <p className="mt-1 text-2xs uppercase tracking-wide text-text-subtle">
            total a adicionar
          </p>
        </div>

        <div className="flex flex-wrap gap-2">
          {QUICK_INCREMENTS.map((inc) => (
            <Button
              key={inc.label}
              type="button"
              variant="secondary"
              size="sm"
              onClick={() => add(inc.seconds)}
            >
              {inc.label}
            </Button>
          ))}
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => add(-15 * 60)}
          >
            -15min
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => setSeconds(0)}
          >
            Limpar
          </Button>
        </div>

        {locked ? (
          <p className="text-xs text-text-muted">
            O tempo entra como um registro separado, logo apos esta sessao. A
            sessao original nao e alterada.
          </p>
        ) : (
          <Field
            label="Dia"
            htmlFor="q-day"
            hint="O horario e aproximado — o que importa e a duracao."
          >
            <Input
              id="q-day"
              type="date"
              value={day}
              onChange={(e) => setDay(e.target.value)}
            />
          </Field>
        )}

        <Field label="Nota (opcional)" htmlFor="q-note">
          <Input
            id="q-note"
            value={note}
            onChange={(e) => setNote(e.target.value)}
            placeholder="No que voce estava trabalhando?"
          />
        </Field>

        {error && <p className="text-sm text-danger">{error}</p>}
      </div>
    </Modal>
  );
}
```

Notas para quem implementa:
- O `data-testid="quick-total"` existe porque `formatDuration(0)` devolve `"0s"` e `1h 30min` tem espaco — buscar por texto solto seria fragil.
- `activityType: "drawing"`, `billable: true` e `idleSeconds: 0` sao deliberados: manter o modal em 4 campos e o que o torna mais rapido que o `EntryForm`. Quem precisar de outro valor edita no Historico.
- Os botoes tem `type="button"` explicito: dentro de um `<form>` o padrao seria `submit`, e cada incremento enviaria o formulario.

- [ ] **Step 4: Rodar os testes e confirmar que passam**

Run: `npm run test -- QuickTimeModal`
Expected: PASS — 6 testes.

- [ ] **Step 5: Commit**

```bash
git add src/features/history/QuickTimeModal.tsx src/features/history/QuickTimeModal.test.tsx
git commit -m "feat(history): modal para adicionar tempo esquecido por duracao"
```

---

### Task 3: Ligar o modal ao Historico

**Files:**
- Modify: `src/features/history/HistoryPage.tsx` (imports; cabecalho em `:80-95`; acoes da linha em `:244-264`; render do modal em `:272-276`)

**Interfaces:**
- Consumes: `QuickTimeModal` da Task 2 (assinatura exata no "Produces" daquela task).
- Produces: nada para tasks posteriores.

Duas portas: o botao do cabecalho (sem ancora) e a acao por linha (ancorada na sessao). Ambas abrem o mesmo modal; a diferenca esta no `anchor`.

Sem teste automatizado proprio: a logica ja esta coberta pelas Tasks 1 e 2, e o que sobra aqui e fiacao de estado. A verificacao e o `typecheck`/`lint` e o checklist manual da Task 5.

- [ ] **Step 1: Trocar o import de icones**

Em `src/features/history/HistoryPage.tsx:2`, trocar:

```tsx
import { Pencil, Plus, RotateCcw, Trash2 } from "lucide-react";
```

por:

```tsx
import { Clock, Pencil, Plus, RotateCcw, Trash2 } from "lucide-react";
```

- [ ] **Step 2: Importar o modal**

Logo abaixo de `import { EntryForm } from "./EntryForm";` (linha 21):

```tsx
import { QuickTimeModal } from "./QuickTimeModal";
```

- [ ] **Step 3: Adicionar o estado local**

Logo abaixo de `const [editing, setEditing] = useState<TimeEntry | null>(null);` (linha 37):

```tsx
  const [quickOpen, setQuickOpen] = useState(false);
  const [quickAnchor, setQuickAnchor] = useState<TimeEntry | null>(null);
```

- [ ] **Step 4: Adicionar o botao no cabecalho**

Substituir o `<PageHeader ... />` inteiro (linhas 80-95) por:

```tsx
      <PageHeader
        title="Historico"
        description="Sessoes registradas: filtre, edite, adicione ou remova."
        action={
          <div className="flex gap-2">
            <Button
              variant="secondary"
              onClick={() => {
                setQuickAnchor(null);
                setQuickOpen(true);
              }}
              icon={<Clock size={16} strokeWidth={2} />}
            >
              Tempo esquecido
            </Button>
            <Button
              variant="primary"
              onClick={() => {
                setEditing(null);
                setFormOpen(true);
              }}
              icon={<Plus size={16} strokeWidth={2} />}
            >
              Nova sessao
            </Button>
          </div>
        }
      />
```

- [ ] **Step 5: Adicionar a acao na linha da tabela**

Substituir o `<div className="flex justify-end gap-1">` inteiro (linhas 245-263) por:

```tsx
                    <div className="flex justify-end gap-1">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => {
                          setQuickAnchor(entry);
                          setQuickOpen(true);
                        }}
                        aria-label="Adicionar tempo a esta sessao"
                        icon={<Clock size={15} strokeWidth={1.75} />}
                      />
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => {
                          setEditing(entry);
                          setFormOpen(true);
                        }}
                        aria-label="Editar sessao"
                        icon={<Pencil size={15} strokeWidth={1.75} />}
                      />
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => void handleDelete(entry)}
                        aria-label="Excluir sessao"
                        icon={<Trash2 size={15} strokeWidth={1.75} />}
                      />
                    </div>
```

- [ ] **Step 6: Renderizar o modal**

Logo abaixo do `<EntryForm ... />` (linhas 272-276), antes do `</div>` final:

```tsx
      <QuickTimeModal
        open={quickOpen}
        anchor={quickAnchor}
        onClose={() => setQuickOpen(false)}
      />
```

- [ ] **Step 7: Verificar que nada quebrou**

Run: `npm run typecheck && npm run lint && npm run test`
Expected: typecheck sem erros; lint com 0 warnings; todos os testes passando.

- [ ] **Step 8: Commit**

```bash
git add src/features/history/HistoryPage.tsx
git commit -m "feat(history): abrir tempo esquecido pelo cabecalho e por sessao"
```

---

### Task 4: Ligar o modal ao painel do cronometro

**Files:**
- Modify: `src/features/timer/TimerPanel.tsx` (imports; estado; bloco sem cronometro ativo em `:132-233`)
- Test: `src/features/timer/TimerPanel.test.tsx` (acrescentar um `describe`)

**Interfaces:**
- Consumes: `QuickTimeModal` da Task 2.
- Produces: nada.

Esta e a porta que mais importa: e onde o usuario esta quando percebe que esqueceu. O link so aparece no estado **sem cronometro ativo** e quando ha projetos.

- [ ] **Step 1: Escrever o teste que falha**

Acrescentar ao fim de `src/features/timer/TimerPanel.test.tsx`, **depois** do `describe` existente:

```tsx
describe("TimerPanel — tempo esquecido", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useCatalogStore.setState({ projects: [project] });
    useTimerStore.setState({
      activeTimer: null,
      loaded: true,
      error: null,
      recoveryPending: false,
    });
  });

  it("oferece adicionar tempo esquecido quando nao ha cronometro ativo", async () => {
    renderPanel();
    await userEvent.click(
      screen.getByRole("button", { name: /Adicionar tempo/i }),
    );

    expect(
      screen.getByRole("dialog", { name: /Adicionar tempo esquecido/i }),
    ).toBeInTheDocument();
  });

  it("nao oferece o atalho com um cronometro rodando", () => {
    useTimerStore.setState({ activeTimer: timer });
    renderPanel();

    expect(
      screen.queryByRole("button", { name: /Adicionar tempo/i }),
    ).not.toBeInTheDocument();
  });
});
```

O `vi.mock("@/services/timeEntries", ...)` no topo do arquivo (linhas 16-18) hoje so expoe `listTimeEntries`. O `QuickTimeModal` importa o `entriesStore`, que importa `createTimeEntry` — sem isso o mock quebra o modulo. Substituir aquele bloco por:

```tsx
vi.mock("@/services/timeEntries", () => ({
  listTimeEntries: vi.fn().mockResolvedValue([]),
  createTimeEntry: vi.fn(),
  updateTimeEntry: vi.fn(),
  deleteTimeEntry: vi.fn(),
  restoreTimeEntry: vi.fn(),
}));
```

- [ ] **Step 2: Rodar o teste e confirmar que falha**

Run: `npm run test -- TimerPanel`
Expected: FAIL — "Unable to find an accessible element with the role button and name /Adicionar tempo/i".

- [ ] **Step 3: Implementar**

3a. Em `src/features/timer/TimerPanel.tsx`, trocar o import de icones (linha 3):

```tsx
import { Clock, Pause, Play, Square, Zap } from "lucide-react";
```

3b. Acrescentar o import do modal, junto aos imports de feature do topo:

```tsx
import { QuickTimeModal } from "@/features/history/QuickTimeModal";
```

3c. Acrescentar o estado, logo abaixo de `const [confirmingStop, setConfirmingStop] = useState(false);`:

```tsx
  const [quickOpen, setQuickOpen] = useState(false);
```

3d. No bloco final (estado sem cronometro ativo), substituir o fechamento do `<form>` ate o fim do `<Panel>` — ou seja, o trecho que hoje vai de `{error && <p className="text-sm text-danger">{error}</p>}` (linha 229) ate `</Panel>` (linha 232) — por:

```tsx
          {error && <p className="text-sm text-danger">{error}</p>}
        </form>
      )}

      {projects.length > 0 && (
        <div className="mt-5 border-t border-border pt-4">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setQuickOpen(true)}
            icon={<Clock size={14} strokeWidth={1.75} />}
          >
            Esqueceu de registrar? Adicionar tempo
          </Button>
        </div>
      )}

      <QuickTimeModal
        open={quickOpen}
        defaultProjectId={projectId}
        onClose={() => setQuickOpen(false)}
      />
    </Panel>
  );
}
```

Nota: o `</form>` e o `)}` acima fecham o `{projects.length === 0 ? (...) : (<form>...)}` que ja existe. O atalho e o modal ficam **fora** desse ternario, mas dentro do `<Panel>`.

- [ ] **Step 4: Rodar os testes e confirmar que passam**

Run: `npm run test -- TimerPanel`
Expected: PASS — 6 testes (os 4 que ja existiam + os 2 novos).

- [ ] **Step 5: Rodar a suite inteira**

Run: `npm run typecheck && npm run lint && npm run test`
Expected: typecheck sem erros; lint com 0 warnings; todos os testes passando.

- [ ] **Step 6: Commit**

```bash
git add src/features/timer/TimerPanel.tsx src/features/timer/TimerPanel.test.tsx
git commit -m "feat(timer): atalho para adicionar tempo esquecido no painel"
```

---

### Task 5: Verificacao no app real

**Files:** nenhum (verificacao manual) + `CHANGELOG.md`

**Interfaces:**
- Consumes: o app inteiro, ja com as Tasks 1-4.
- Produces: nada.

**PRE-REQUISITO — LEIA ANTES DE RODAR:** o app instalado (`%LOCALAPPDATA%\CronoCAD\cronocad.exe`) usa **o mesmo banco SQLite** que o `tauri:dev`. Duas instancias escrevendo no mesmo banco corrompem dados. **O app instalado precisa estar fechado**, e o cronometro do usuario **pausado ou encerrado** (nunca descartado) antes de comecar.

- [ ] **Step 1: Confirmar que o app instalado esta fechado**

Run: `powershell -NoProfile -Command "Get-Process cronocad -ErrorAction SilentlyContinue | Select-Object Id, Path"`
Expected: **sem saida**. Se algo aparecer, **pare** e peca ao usuario para fechar o app.

- [ ] **Step 2: Subir o app em modo dev**

Run: `npm run tauri:dev`

- [ ] **Step 3: Percorrer o checklist manual**

Com um projeto qualquer, **sem** cronometro rodando:

1. No painel, clicar em **"Esqueceu de registrar? Adicionar tempo"**. O modal abre com o projeto ja selecionado e o dia em hoje.
2. Clicar `+1h`, `+30min`. O total mostra **1h 30min**. Clicar `-15min` -> **1h 15min**. Clicar `Limpar` -> **0s** e o **Salvar fica desabilitado**.
3. Clicar `+2h` e **Salvar**. A sessao aparece no Historico com 2h, terminando **agora**, marcada como manual.
4. No Historico, abrir **"Tempo esquecido"** pelo cabecalho, escolher um **dia passado sem nenhuma sessao**, `+1h`, Salvar. O registro cai naquele dia, terminando as **18:00**.
5. Numa linha do Historico, clicar no icone de relogio (**"Adicionar tempo a esta sessao"**). O projeto aparece travado e o campo de dia some. Adicionar `+30min` e Salvar.
6. Conferir na tabela que a sessao original **continua com a duracao antiga** e que o ajuste de 30min e uma **linha separada**, comecando onde a original terminou.
7. Conferir que o total do periodo (rodape dos filtros) somou os dois.

- [ ] **Step 4: Atualizar o CHANGELOG e commitar**

Acrescentar a mudanca ao `CHANGELOG.md` seguindo o formato ja usado no arquivo, e:

```bash
git add CHANGELOG.md
git commit -m "docs: registrar adicionar tempo esquecido no changelog"
git push
```
