# Editar e excluir sessao: alcance e aviso de sessao suspeita — Plano de implementacao

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tornar editar e excluir uma sessao alcancavel a partir do Painel e visivel no Historico, e sinalizar sessoes com duracao implausivel (cronometro esquecido ligado).

**Architecture:** Nenhuma mudanca de backend, banco ou migration. Uma regra pura nova em `src/lib/suspiciousEntry.ts` decide se uma sessao merece o selo "Conferir?"; um selo compartilhado em `src/components/ui/SuspicionBadge.tsx` a renderiza. Um modal de confirmacao novo (`DeleteEntryModal`) substitui o `window.confirm` e passa a ser usado tanto pelo Historico quanto pelo `EntryForm`, que ganha um botao "Excluir sessao". Painel e Historico consomem tudo isso.

**Tech Stack:** React 18 + TypeScript estrito, Vitest + @testing-library/react, Tailwind (tokens em `src/styles/tokens.css`), Zustand (`useEntriesStore`), lucide-react.

## Global Constraints

- **TypeScript estrito, `any` proibido** — o ESLint trata como erro.
- **Import alias `@/` aponta para `src/`.**
- **Sem SQL em componente React.** Toda escrita passa por `useEntriesStore`.
- **Nenhuma migration.** Nada neste plano altera schema nem grava colunas novas.
- **Nenhuma mudanca em Rust.** A regra de suspeita e visualizacao pura.
- **Texto de interface em pt-BR sem acentos**, seguindo o padrao ja usado no codigo ("Excluir sessao", "Historico", "Duracao").
- **Rotulos centralizados em `src/lib/labels.ts`** (secao 18 do projeto).
- **Nunca sobrescrever o tempo real gravado** (regra critica 5). O selo e informativo; nao altera `durationSeconds`.
- **Nunca descartar tempo silenciosamente** (regra critica 8). Toda exclusao passa por confirmacao explicita.
- Verificacao ao fim de cada task: `npm run typecheck`, `npm run lint`, `npm run test`.

---

### Task 1: Regra de sessao suspeita + selo compartilhado

**Files:**
- Create: `src/lib/suspiciousEntry.ts`
- Create: `src/lib/suspiciousEntry.test.ts`
- Create: `src/components/ui/SuspicionBadge.tsx`
- Modify: `src/lib/labels.ts` (acrescentar `SUSPICION_REASON_LABELS` ao fim do arquivo)

**Interfaces:**
- Consumes: `TimeEntry` de `@/types/domain`.
- Produces:
  - `type SuspicionReason = "muito-longa" | "madrugada"`
  - `interface Suspicion { suspicious: boolean; reasons: SuspicionReason[] }`
  - `const LONG_SESSION_HOURS = 8`
  - `function inspectEntry(entry: TimeEntry): Suspicion`
  - `const SUSPICION_REASON_LABELS: Record<SuspicionReason, string>` (em `@/lib/labels`)
  - `function SuspicionBadge({ reasons }: { reasons: SuspicionReason[] })` — retorna `null` quando `reasons` esta vazio

- [ ] **Step 1: Escrever o teste que falha**

Crie `src/lib/suspiciousEntry.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { inspectEntry, LONG_SESSION_HOURS } from "./suspiciousEntry";
import type { TimeEntry, TimeEntrySource } from "@/types/domain";

/**
 * Constroi um ISO UTC a partir de componentes de horario LOCAL. A regra e
 * definida em horario local, entao o teste precisa ser independente do fuso
 * da maquina que roda a suite.
 */
function localIso(
  year: number,
  month: number,
  day: number,
  hour: number,
  minute = 0,
): string {
  return new Date(year, month - 1, day, hour, minute, 0, 0).toISOString();
}

interface EntryOverrides {
  startedAt: string;
  endedAt: string | null;
  durationSeconds: number;
  source?: TimeEntrySource;
}

function entry({
  startedAt,
  endedAt,
  durationSeconds,
  source = "timer",
}: EntryOverrides): TimeEntry {
  return {
    id: "e1",
    projectId: "p1",
    startedAt,
    endedAt,
    durationSeconds,
    idleSeconds: 0,
    description: null,
    activityType: "drawing",
    billable: true,
    hourlyRateSnapshotCents: 9000,
    source,
    createdAt: startedAt,
    updatedAt: startedAt,
    deletedAt: null,
  };
}

describe("inspectEntry", () => {
  it("nao marca uma jornada exatamente no limite de horas", () => {
    const result = inspectEntry(
      entry({
        startedAt: localIso(2026, 8, 10, 9),
        endedAt: localIso(2026, 8, 10, 17),
        durationSeconds: LONG_SESSION_HOURS * 3600,
      }),
    );

    expect(result.suspicious).toBe(false);
    expect(result.reasons).toEqual([]);
  });

  it("marca um segundo acima do limite como muito longa", () => {
    const result = inspectEntry(
      entry({
        startedAt: localIso(2026, 8, 10, 9),
        endedAt: localIso(2026, 8, 10, 17),
        durationSeconds: LONG_SESSION_HOURS * 3600 + 1,
      }),
    );

    expect(result.suspicious).toBe(true);
    expect(result.reasons).toEqual(["muito-longa"]);
  });

  it("marca uma sessao curta que atravessa as 04:00 locais", () => {
    const result = inspectEntry(
      entry({
        startedAt: localIso(2026, 8, 10, 3, 30),
        endedAt: localIso(2026, 8, 10, 4, 30),
        durationSeconds: 3600,
      }),
    );

    expect(result.suspicious).toBe(true);
    expect(result.reasons).toEqual(["madrugada"]);
  });

  it("marca uma sessao que comeca exatamente as 04:00 locais", () => {
    const result = inspectEntry(
      entry({
        startedAt: localIso(2026, 8, 10, 4),
        endedAt: localIso(2026, 8, 10, 5),
        durationSeconds: 3600,
      }),
    );

    expect(result.reasons).toEqual(["madrugada"]);
  });

  it("nao marca uma sessao curta em horario comercial", () => {
    const result = inspectEntry(
      entry({
        startedAt: localIso(2026, 8, 10, 9),
        endedAt: localIso(2026, 8, 10, 10),
        durationSeconds: 3600,
      }),
    );

    expect(result.suspicious).toBe(false);
  });

  it("ignora sessao manual, por mais longa que seja", () => {
    const result = inspectEntry(
      entry({
        startedAt: localIso(2026, 8, 10, 22),
        endedAt: localIso(2026, 8, 11, 22),
        durationSeconds: 86400,
        source: "manual",
      }),
    );

    expect(result.suspicious).toBe(false);
  });

  it("ignora sessao reconstruida", () => {
    const result = inspectEntry(
      entry({
        startedAt: localIso(2026, 8, 10, 22),
        endedAt: localIso(2026, 8, 11, 22),
        durationSeconds: 86400,
        source: "reconstructed",
      }),
    );

    expect(result.suspicious).toBe(false);
  });

  it("ignora sessao ainda em aberto: cronometro rodando nao e erro", () => {
    const result = inspectEntry(
      entry({
        startedAt: localIso(2026, 8, 10, 22),
        endedAt: null,
        durationSeconds: 86400,
      }),
    );

    expect(result.suspicious).toBe(false);
  });

  it("acumula os dois motivos no cronometro esquecido a noite", () => {
    // Caso real que originou esta regra: 10/08 22:33 -> 11/08 22:46.
    const result = inspectEntry(
      entry({
        startedAt: localIso(2026, 8, 10, 22, 33),
        endedAt: localIso(2026, 8, 11, 22, 46),
        durationSeconds: 87160,
      }),
    );

    expect(result.suspicious).toBe(true);
    expect(result.reasons).toEqual(["muito-longa", "madrugada"]);
  });
});
```

- [ ] **Step 2: Rodar o teste e confirmar que falha**

Run: `npm run test -- src/lib/suspiciousEntry.test.ts`
Expected: FAIL — `Failed to resolve import "./suspiciousEntry"`.

- [ ] **Step 3: Implementar a regra**

Crie `src/lib/suspiciousEntry.ts`:

```ts
/**
 * Sinaliza sessoes com duracao implausivel — tipicamente o cronometro deixado
 * ligado durante a noite. E regra de VISUALIZACAO: nao altera nada no banco,
 * so decide se a tela mostra o selo "Conferir?" (regra critica 5).
 */

import type { TimeEntry } from "@/types/domain";

export type SuspicionReason = "muito-longa" | "madrugada";

export interface Suspicion {
  suspicious: boolean;
  reasons: SuspicionReason[];
}

/** Acima disto, uma unica sessao de cronometro vira candidata a esquecimento. */
export const LONG_SESSION_HOURS = 8;

/** Hora local em que praticamente ninguem esta desenhando. */
const DEAD_HOUR = 4;

const NOT_SUSPICIOUS: Suspicion = { suspicious: false, reasons: [] };

/**
 * True se o intervalo local [start, end] contem alguma ocorrencia de DEAD_HOUR.
 * Uma sessao de varios dias precisa de uma unica batida para valer.
 */
function crossesDeadHour(start: Date, end: Date): boolean {
  const probe = new Date(start);
  probe.setHours(DEAD_HOUR, 0, 0, 0);
  // A batida do dia do inicio pode ter ficado para tras; nesse caso a proxima
  // candidata e a do dia seguinte.
  if (probe.getTime() < start.getTime()) {
    probe.setDate(probe.getDate() + 1);
  }
  return probe.getTime() <= end.getTime();
}

export function inspectEntry(entry: TimeEntry): Suspicion {
  // Sessao manual foi digitada de proposito e a reconstruida nasce de uma
  // decisao explicita na linha do tempo: marcar as duas seria alarme falso.
  if (entry.source !== "timer") return NOT_SUSPICIOUS;
  // Cronometro em andamento nao e erro — ainda da tempo de encerrar.
  if (entry.endedAt === null) return NOT_SUSPICIOUS;

  const reasons: SuspicionReason[] = [];
  if (entry.durationSeconds > LONG_SESSION_HOURS * 3600) {
    reasons.push("muito-longa");
  }
  if (crossesDeadHour(new Date(entry.startedAt), new Date(entry.endedAt))) {
    reasons.push("madrugada");
  }
  return { suspicious: reasons.length > 0, reasons };
}
```

- [ ] **Step 4: Rodar o teste e confirmar que passa**

Run: `npm run test -- src/lib/suspiciousEntry.test.ts`
Expected: PASS — 9 testes.

- [ ] **Step 5: Acrescentar os rotulos**

Em `src/lib/labels.ts`, adicione o import no topo e o bloco ao fim do arquivo:

```ts
import {
  LONG_SESSION_HOURS,
  type SuspicionReason,
} from "@/lib/suspiciousEntry";
```

```ts
export const SUSPICION_REASON_LABELS: Record<SuspicionReason, string> = {
  "muito-longa": `Mais de ${LONG_SESSION_HOURS}h em uma unica sessao`,
  madrugada: "Atravessa a madrugada",
};
```

- [ ] **Step 6: Criar o selo compartilhado**

Crie `src/components/ui/SuspicionBadge.tsx`:

```tsx
import { AlertTriangle } from "lucide-react";
import type { SuspicionReason } from "@/lib/suspiciousEntry";
import { SUSPICION_REASON_LABELS } from "@/lib/labels";

interface SuspicionBadgeProps {
  reasons: SuspicionReason[];
}

/**
 * Selo discreto em sessoes com duracao implausivel. So chama atencao — nao
 * bloqueia nada e nao altera o tempo gravado.
 */
export function SuspicionBadge({ reasons }: SuspicionBadgeProps) {
  if (reasons.length === 0) return null;
  const motivos = reasons.map((r) => SUSPICION_REASON_LABELS[r]).join(" · ");
  return (
    <span
      className="inline-flex items-center gap-1 rounded border border-warning/40 px-1.5 py-0.5 text-2xs font-medium text-warning"
      title={motivos}
    >
      <AlertTriangle size={11} strokeWidth={2} aria-hidden />
      Conferir?
    </span>
  );
}
```

- [ ] **Step 7: Verificar tipos, lint e suite completa**

Run: `npm run typecheck && npm run lint && npm run test`
Expected: PASS nos tres.

- [ ] **Step 8: Commit**

```bash
git add src/lib/suspiciousEntry.ts src/lib/suspiciousEntry.test.ts src/lib/labels.ts src/components/ui/SuspicionBadge.tsx
git commit -m "feat(historico): regra e selo de sessao suspeita"
```

---

### Task 2: Modal de confirmacao de exclusao

Substitui o `window.confirm` cru do sistema por um dialogo do app que mostra exatamente o que sai da conta.

**Files:**
- Create: `src/features/history/DeleteEntryModal.tsx`
- Create: `src/features/history/DeleteEntryModal.test.tsx`
- Modify: `src/features/history/HistoryPage.tsx` (remover `handleDelete` com `window.confirm`, linhas 72-80; usar o modal)

**Interfaces:**
- Consumes: `Modal` e `Button` de `@/components/ui/*`, `amountForDuration` de `@/lib/money`, `formatCurrency`/`formatDate`/`formatDuration`/`formatTime` de `@/lib/format`.
- Produces: `function DeleteEntryModal(props: DeleteEntryModalProps)` com

```ts
interface DeleteEntryModalProps {
  open: boolean;
  entry: TimeEntry | null;
  projectName: string;
  busy?: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}
```

- [ ] **Step 1: Escrever o teste que falha**

Crie `src/features/history/DeleteEntryModal.test.tsx`:

```tsx
import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DeleteEntryModal } from "./DeleteEntryModal";
import type { TimeEntry } from "@/types/domain";

const entry: TimeEntry = {
  id: "e1",
  projectId: "p1",
  startedAt: "2026-08-11T01:33:44Z",
  endedAt: "2026-08-11T03:33:44Z",
  durationSeconds: 7200,
  idleSeconds: 0,
  description: null,
  activityType: "drawing",
  billable: true,
  hourlyRateSnapshotCents: 9000,
  source: "timer",
  createdAt: "2026-08-11T03:33:44Z",
  updatedAt: "2026-08-11T03:33:44Z",
  deletedAt: null,
};

function setup(overrides: Partial<TimeEntry> = {}) {
  const handlers = { onCancel: vi.fn(), onConfirm: vi.fn() };
  render(
    <DeleteEntryModal
      open
      entry={{ ...entry, ...overrides }}
      projectName="Residencial Aurora"
      {...handlers}
    />,
  );
  return handlers;
}

describe("DeleteEntryModal", () => {
  it("mostra o projeto, a duracao e o valor que sai da conta", () => {
    setup();

    expect(screen.getByText("Residencial Aurora")).toBeInTheDocument();
    expect(screen.getByText("2h 00min")).toBeInTheDocument();
    expect(screen.getByText("R$ 180,00")).toBeInTheDocument();
  });

  it("desconta a inatividade do valor exibido", () => {
    setup({ idleSeconds: 3600 });

    expect(screen.getByText("R$ 90,00")).toBeInTheDocument();
  });

  it("avisa que da para restaurar depois", () => {
    setup();

    expect(screen.getByText(/restaurar/i)).toBeInTheDocument();
  });

  it("confirma a exclusao ao clicar em Excluir", async () => {
    const { onConfirm, onCancel } = setup();

    await userEvent.click(screen.getByRole("button", { name: "Excluir" }));

    expect(onConfirm).toHaveBeenCalledTimes(1);
    expect(onCancel).not.toHaveBeenCalled();
  });

  it("nao exclui nada ao cancelar", async () => {
    const { onConfirm, onCancel } = setup();

    await userEvent.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("nao renderiza nada sem sessao", () => {
    const { container } = render(
      <DeleteEntryModal
        open
        entry={null}
        projectName="Residencial Aurora"
        onCancel={vi.fn()}
        onConfirm={vi.fn()}
      />,
    );

    expect(container).toBeEmptyDOMElement();
  });
});
```

- [ ] **Step 2: Rodar o teste e confirmar que falha**

Run: `npm run test -- src/features/history/DeleteEntryModal.test.tsx`
Expected: FAIL — `Failed to resolve import "./DeleteEntryModal"`.

- [ ] **Step 3: Implementar o modal**

Crie `src/features/history/DeleteEntryModal.tsx`:

```tsx
import type { TimeEntry } from "@/types/domain";
import {
  formatCurrency,
  formatDate,
  formatDuration,
  formatTime,
} from "@/lib/format";
import { amountForDuration } from "@/lib/money";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";

interface DeleteEntryModalProps {
  open: boolean;
  entry: TimeEntry | null;
  projectName: string;
  busy?: boolean;
  onCancel: () => void;
  onConfirm: () => void;
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-baseline justify-between gap-4 py-1.5">
      <span className="text-xs text-text-muted">{label}</span>
      <span className="tabular text-sm text-text">{value}</span>
    </div>
  );
}

/**
 * Confirmacao de exclusao de sessao. Mostra o que sai da conta antes de
 * remover: nunca descartar tempo silenciosamente (regra critica 8).
 */
export function DeleteEntryModal({
  open,
  entry,
  projectName,
  busy = false,
  onCancel,
  onConfirm,
}: DeleteEntryModalProps) {
  if (!entry) return null;

  const amount = amountForDuration(
    entry.durationSeconds - entry.idleSeconds,
    entry.hourlyRateSnapshotCents,
  );
  const periodo = entry.endedAt
    ? `${formatTime(entry.startedAt)}–${formatTime(entry.endedAt)}`
    : formatTime(entry.startedAt);

  return (
    <Modal
      open={open}
      title="Excluir sessao"
      onClose={onCancel}
      footer={
        <>
          <Button variant="ghost" onClick={onCancel} type="button">
            Cancelar
          </Button>
          <Button
            variant="danger"
            onClick={onConfirm}
            type="button"
            disabled={busy}
          >
            {busy ? "Excluindo…" : "Excluir"}
          </Button>
        </>
      }
    >
      <div className="divide-y divide-border">
        <Row label="Projeto" value={projectName} />
        <Row label="Data" value={formatDate(entry.startedAt)} />
        <Row label="Periodo" value={periodo} />
        <Row label="Duracao" value={formatDuration(entry.durationSeconds)} />
        <Row label="Valor" value={formatCurrency(amount)} />
      </div>
      <p className="mt-4 text-xs text-text-muted">
        A sessao sai das telas e dos relatorios, mas continua guardada: da para
        restaurar depois em Historico, marcando “Mostrar excluidas”.
      </p>
    </Modal>
  );
}
```

- [ ] **Step 4: Rodar o teste e confirmar que passa**

Run: `npm run test -- src/features/history/DeleteEntryModal.test.tsx`
Expected: PASS — 6 testes.

- [ ] **Step 5: Ligar o modal no Historico**

Em `src/features/history/HistoryPage.tsx`:

Acrescente o import junto dos outros de `./`:

```tsx
import { DeleteEntryModal } from "./DeleteEntryModal";
```

Acrescente o estado, logo abaixo de `const [quickAnchor, setQuickAnchor] = useState<TimeEntry | null>(null);`:

```tsx
const [deleting, setDeleting] = useState<TimeEntry | null>(null);
const [deleteBusy, setDeleteBusy] = useState(false);
```

Substitua a funcao `handleDelete` inteira (linhas 72-80, a que usa `window.confirm`) por:

```tsx
async function confirmDelete() {
  if (!deleting) return;
  setDeleteBusy(true);
  try {
    await remove(deleting.id);
    setDeleting(null);
  } finally {
    setDeleteBusy(false);
  }
}
```

Troque o `onClick` do botao de excluir da tabela (hoje `onClick={() => void handleDelete(entry)}`) por:

```tsx
onClick={() => setDeleting(entry)}
```

E acrescente o modal junto dos outros, logo antes do `<QuickTimeModal ... />` final:

```tsx
<DeleteEntryModal
  open={deleting !== null}
  entry={deleting}
  projectName={deleting ? projectName(deleting.projectId) : ""}
  busy={deleteBusy}
  onCancel={() => setDeleting(null)}
  onConfirm={() => void confirmDelete()}
/>
```

- [ ] **Step 6: Verificar tipos, lint e suite completa**

Run: `npm run typecheck && npm run lint && npm run test`
Expected: PASS nos tres. Se o lint reclamar de import nao usado, confirme que `formatDate` ainda e usado na tabela do Historico (e e, na coluna Data).

- [ ] **Step 7: Commit**

```bash
git add src/features/history/DeleteEntryModal.tsx src/features/history/DeleteEntryModal.test.tsx src/features/history/HistoryPage.tsx
git commit -m "feat(historico): confirmacao de exclusao mostrando o que sai da conta"
```

---

### Task 3: Botao "Excluir sessao" no formulario de edicao

Da ao Painel uma unica affordance por linha que resolve editar **e** excluir — a coluna de "Sessoes recentes" e estreita demais para dois botoes com texto.

**Files:**
- Modify: `src/features/history/EntryForm.tsx`
- Create: `src/features/history/EntryForm.test.tsx`

**Interfaces:**
- Consumes: `DeleteEntryModal` da Task 2, `useEntriesStore` (`remove`).
- Produces: nenhuma API nova — a assinatura de `EntryForm` nao muda.

- [ ] **Step 1: Escrever o teste que falha**

Crie `src/features/history/EntryForm.test.tsx`:

```tsx
import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { EntryForm } from "./EntryForm";
import { useEntriesStore } from "@/stores/entriesStore";
import { useCatalogStore } from "@/stores/catalogStore";
import type { Project, TimeEntry } from "@/types/domain";

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
  createdAt: "2026-08-01T08:00:00Z",
  updatedAt: "2026-08-01T08:00:00Z",
  archivedAt: null,
};

const entry: TimeEntry = {
  id: "e1",
  projectId: "p1",
  startedAt: "2026-08-11T01:33:44Z",
  endedAt: "2026-08-11T03:33:44Z",
  durationSeconds: 7200,
  idleSeconds: 0,
  description: null,
  activityType: "drawing",
  billable: true,
  hourlyRateSnapshotCents: 9000,
  source: "timer",
  createdAt: "2026-08-11T03:33:44Z",
  updatedAt: "2026-08-11T03:33:44Z",
  deletedAt: null,
};

const remove = vi.fn().mockResolvedValue(undefined);

beforeEach(() => {
  remove.mockClear();
  useCatalogStore.setState({ projects: [project] });
  useEntriesStore.setState({ remove });
});

describe("EntryForm", () => {
  it("nao oferece excluir ao criar uma sessao nova", () => {
    render(<EntryForm open entry={null} onClose={vi.fn()} />);

    expect(
      screen.queryByRole("button", { name: "Excluir sessao" }),
    ).not.toBeInTheDocument();
  });

  it("oferece excluir ao editar uma sessao existente", () => {
    render(<EntryForm open entry={entry} onClose={vi.fn()} />);

    expect(
      screen.getByRole("button", { name: "Excluir sessao" }),
    ).toBeInTheDocument();
  });

  it("so exclui depois da confirmacao, e fecha o formulario", async () => {
    const onClose = vi.fn();
    render(<EntryForm open entry={entry} onClose={onClose} />);

    await userEvent.click(
      screen.getByRole("button", { name: "Excluir sessao" }),
    );
    expect(remove).not.toHaveBeenCalled();

    await userEvent.click(screen.getByRole("button", { name: "Excluir" }));

    expect(remove).toHaveBeenCalledWith("e1");
    expect(onClose).toHaveBeenCalled();
  });

  it("nao exclui se a confirmacao for cancelada", async () => {
    render(<EntryForm open entry={entry} onClose={vi.fn()} />);

    await userEvent.click(
      screen.getByRole("button", { name: "Excluir sessao" }),
    );
    await userEvent.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(remove).not.toHaveBeenCalled();
  });
});
```

Nota: a confirmacao tem dois botoes "Cancelar" na tela (o do formulario e o do modal). O modal de confirmacao renderiza depois no DOM e o `getByRole` falharia com "multiple elements". Por isso, no Step 3, o rodape do `EntryForm` esconde Cancelar/Salvar enquanto a confirmacao esta aberta — o que tambem e o comportamento certo: nao se edita e exclui ao mesmo tempo.

- [ ] **Step 2: Rodar o teste e confirmar que falha**

Run: `npm run test -- src/features/history/EntryForm.test.tsx`
Expected: FAIL — `Unable to find an accessible element with the role "button" and name "Excluir sessao"`.

- [ ] **Step 3: Implementar**

Em `src/features/history/EntryForm.tsx`:

Acrescente o import do modal, depois dos imports de `@/components/ui/Field`:

```tsx
import { DeleteEntryModal } from "./DeleteEntryModal";
```

Pegue `remove` da store, junto de `create` e `update`:

```tsx
const remove = useEntriesStore((s) => s.remove);
```

Acrescente o estado, depois de `const [saving, setSaving] = useState(false);`:

```tsx
const [confirmingDelete, setConfirmingDelete] = useState(false);
```

Acrescente o handler, depois de `handleSubmit`:

```tsx
async function handleDelete() {
  if (!entry) return;
  setSaving(true);
  setError(null);
  try {
    await remove(entry.id);
    setConfirmingDelete(false);
    onClose();
  } catch (err) {
    setConfirmingDelete(false);
    setError(typeof err === "string" ? err : "Falha ao excluir a sessao.");
  } finally {
    setSaving(false);
  }
}
```

Troque o `footer` do `<Modal>` por:

```tsx
footer={
  <>
    {entry && (
      <Button
        variant="danger"
        onClick={() => setConfirmingDelete(true)}
        type="button"
        disabled={saving}
        className="mr-auto"
      >
        Excluir sessao
      </Button>
    )}
    {!confirmingDelete && (
      <>
        <Button variant="ghost" onClick={onClose} type="button">
          Cancelar
        </Button>
        <Button
          variant="primary"
          type="submit"
          form="entry-form"
          disabled={saving}
        >
          {saving ? "Salvando…" : "Salvar"}
        </Button>
      </>
    )}
  </>
}
```

`mr-auto` empurra o botao vermelho para a esquerda dentro do rodape, que e `flex justify-end gap-2`.

Por fim, envolva o `<Modal>` num fragmento e renderize a confirmacao como **irma** dele (nao dentro), para os dois overlays nao se aninharem. A estrutura do `return` fica:

```tsx
return (
  <>
    <Modal open={open} title={entry ? "Editar sessao" : "Nova sessao manual"} onClose={onClose} footer={/* como acima */}>
      {/* o formulario, inalterado */}
    </Modal>

    <DeleteEntryModal
      open={confirmingDelete}
      entry={entry}
      projectName={projectName}
      busy={saving}
      onCancel={() => setConfirmingDelete(false)}
      onConfirm={() => void handleDelete()}
    />
  </>
);
```

`projectName` ja existe no componente (linha 106-107) e serve exatamente para isso.

- [ ] **Step 4: Rodar o teste e confirmar que passa**

Run: `npm run test -- src/features/history/EntryForm.test.tsx`
Expected: PASS — 4 testes.

- [ ] **Step 5: Verificar tipos, lint e suite completa**

Run: `npm run typecheck && npm run lint && npm run test`
Expected: PASS nos tres.

- [ ] **Step 6: Commit**

```bash
git add src/features/history/EntryForm.tsx src/features/history/EntryForm.test.tsx
git commit -m "feat(historico): excluir sessao pelo formulario de edicao"
```

---

### Task 4: Historico — acoes visiveis e coluna que nao foge

Hoje as acoes sao botoes so-icone na **ultima** coluna de uma tabela com `min-w-[980px]` dentro de `overflow-x-auto`: em janela menor, elas ficam fora da area visivel ate rolar na horizontal.

**Files:**
- Modify: `src/features/history/HistoryPage.tsx` (cabecalho e corpo da tabela, linhas ~221-301)

**Interfaces:**
- Consumes: `inspectEntry` de `@/lib/suspiciousEntry` e `SuspicionBadge` de `@/components/ui/SuspicionBadge` (Task 1).
- Produces: nada novo.

- [ ] **Step 1: Acrescentar os imports**

Em `src/features/history/HistoryPage.tsx`:

```tsx
import { inspectEntry } from "@/lib/suspiciousEntry";
import { SuspicionBadge } from "@/components/ui/SuspicionBadge";
```

- [ ] **Step 2: Fixar a coluna de acoes no cabecalho**

Troque o `<th>` de Acoes por:

```tsx
<th className="sticky right-0 border-l border-border bg-surface px-4 py-2 text-right font-medium">
  Acoes
</th>
```

- [ ] **Step 3: Marcar a linha como grupo, para o hover atravessar a celula fixa**

A celula fixa tem fundo opaco proprio, entao ela tapa o `hover:bg-surface-hover` da linha. Troque a abertura de `<tr>` do corpo por:

```tsx
<tr key={entry.id} className="group hover:bg-surface-hover">
```

- [ ] **Step 4: Selo na coluna de duracao**

Troque a celula de Duracao por:

```tsx
<td className="tabular whitespace-nowrap px-4 py-3 text-right text-text">
  <span className="inline-flex items-center gap-2">
    <SuspicionBadge reasons={inspectEntry(entry).reasons} />
    {formatDuration(entry.durationSeconds)}
  </span>
</td>
```

- [ ] **Step 5: Trocar a celula de acoes por botoes com texto, fixada a direita**

Troque a `<td>` de acoes inteira (a que hoje contem os tres botoes so-icone) por:

```tsx
<td className="sticky right-0 border-l border-border bg-surface px-4 py-3 group-hover:bg-surface-hover">
  <div className="flex justify-end gap-1">
    <Button
      variant="ghost"
      size="sm"
      onClick={() => {
        setQuickAnchor(entry);
        setQuickOpen(true);
      }}
      aria-label="Adicionar tempo a esta sessao"
      title="Adicionar tempo a esta sessao"
      icon={<Clock size={15} strokeWidth={1.75} />}
    />
    <Button
      variant="ghost"
      size="sm"
      onClick={() => {
        setEditing(entry);
        setFormOpen(true);
      }}
      icon={<Pencil size={15} strokeWidth={1.75} />}
    >
      Editar
    </Button>
    <Button
      variant="danger"
      size="sm"
      onClick={() => setDeleting(entry)}
      icon={<Trash2 size={15} strokeWidth={1.75} />}
    >
      Excluir
    </Button>
  </div>
</td>
```

O botao "Adicionar tempo" continua so-icone de proposito: e acao secundaria e ja tem entrada propria no cabecalho da pagina ("Tempo esquecido"). Ganhou `title` para dar dica no hover.

- [ ] **Step 6: Conferir na tela**

Run: `npm run dev`
Abra `http://localhost:1420/historico`, estreite a janela ate a tabela rolar na horizontal e confirme: a coluna Acoes fica colada na direita, com borda a esquerda, fundo opaco e o hover da linha acompanhando. Os botoes leem "Editar" e "Excluir".

- [ ] **Step 7: Verificar tipos, lint e suite completa**

Run: `npm run typecheck && npm run lint && npm run test`
Expected: PASS nos tres.

- [ ] **Step 8: Commit**

```bash
git add src/features/history/HistoryPage.tsx
git commit -m "fix(historico): acoes com texto e coluna fixa que nao foge na rolagem"
```

---

### Task 5: Painel — editar a sessao de onde o cronometro mora

O usuario procurou a funcao no Painel, que e onde o cronometro fica. Hoje "Sessoes recentes" e somente leitura e nao aponta para o Historico.

**Files:**
- Modify: `src/features/dashboard/DashboardPage.tsx` (painel "Sessoes recentes", linhas ~87-119)
- Create: `src/features/dashboard/DashboardPage.test.tsx`

**Interfaces:**
- Consumes: `EntryForm` de `@/features/history/EntryForm`, `inspectEntry`, `SuspicionBadge`, `Button`, `ROUTES`.
- Produces: nada novo.

- [ ] **Step 1: Escrever o teste que falha**

Crie `src/features/dashboard/DashboardPage.test.tsx`:

```tsx
import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { DashboardPage } from "./DashboardPage";
import { useEntriesStore } from "@/stores/entriesStore";
import { useCatalogStore } from "@/stores/catalogStore";
import { useTimerStore } from "@/stores/timerStore";
import type { Project, TimeEntry } from "@/types/domain";

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
  createdAt: "2026-08-01T08:00:00Z",
  updatedAt: "2026-08-01T08:00:00Z",
  archivedAt: null,
};

/** Cronometro esquecido: 24h atravessando a madrugada. */
function forgotten(): TimeEntry {
  const startedAt = new Date(2026, 7, 10, 22, 33).toISOString();
  const endedAt = new Date(2026, 7, 11, 22, 46).toISOString();
  return {
    id: "e1",
    projectId: "p1",
    startedAt,
    endedAt,
    durationSeconds: 87160,
    idleSeconds: 0,
    description: null,
    activityType: "drawing",
    billable: true,
    hourlyRateSnapshotCents: 9000,
    source: "timer",
    createdAt: endedAt,
    updatedAt: endedAt,
    deletedAt: null,
  };
}

function renderPage() {
  render(
    <MemoryRouter>
      <DashboardPage />
    </MemoryRouter>,
  );
}

beforeEach(() => {
  useTimerStore.setState({ activeTimer: null });
  useCatalogStore.setState({ projects: [project], loaded: true });
  useEntriesStore.setState({ entries: [forgotten()] });
});

describe("DashboardPage — sessoes recentes", () => {
  it("marca a sessao suspeita com o selo Conferir?", () => {
    renderPage();

    expect(screen.getByText("Conferir?")).toBeInTheDocument();
  });

  it("leva ao historico completo", () => {
    renderPage();

    expect(screen.getByRole("link", { name: /historico/i })).toHaveAttribute(
      "href",
      "/historico",
    );
  });

  it("abre o formulario de edicao pela linha da sessao", async () => {
    renderPage();

    await userEvent.click(screen.getByRole("button", { name: "Editar" }));

    expect(screen.getByRole("dialog")).toHaveAccessibleName("Editar sessao");
  });
});
```

Se `RecoveryModal` ou `TodosPanel` chamarem o backend no `useEffect` e quebrarem o teste, adicione no topo do arquivo:

```tsx
vi.mock("@/features/timer/RecoveryModal", () => ({
  RecoveryModal: () => null,
}));
vi.mock("./TodosPanel", () => ({ TodosPanel: () => null }));
vi.mock("@/features/timer/TimerPanel", () => ({ TimerPanel: () => null }));
```

- [ ] **Step 2: Rodar o teste e confirmar que falha**

Run: `npm run test -- src/features/dashboard/DashboardPage.test.tsx`
Expected: FAIL — "Unable to find an element with the text: Conferir?".

- [ ] **Step 3: Acrescentar imports e estado**

Em `src/features/dashboard/DashboardPage.tsx`, junto dos imports existentes:

```tsx
import { useState } from "react";
import { Pencil } from "lucide-react";
import type { TimeEntry } from "@/types/domain";
import { Button } from "@/components/ui/Button";
import { SuspicionBadge } from "@/components/ui/SuspicionBadge";
import { inspectEntry } from "@/lib/suspiciousEntry";
import { EntryForm } from "@/features/history/EntryForm";
```

Dentro de `DashboardPage`, junto das outras chamadas de store:

```tsx
const [editing, setEditing] = useState<TimeEntry | null>(null);
const [formOpen, setFormOpen] = useState(false);
```

- [ ] **Step 4: Dar acao ao painel de sessoes recentes**

Troque o `<PanelHeader title="Sessoes recentes" />` por:

```tsx
<PanelHeader
  title="Sessoes recentes"
  action={
    <Link
      to={ROUTES.history}
      className="text-xs text-accent hover:underline"
    >
      Ver todo o historico →
    </Link>
  }
/>
```

Troque o `<li>` de cada sessao por:

```tsx
<li
  key={entry.id}
  className="flex items-center justify-between gap-3 px-4 py-3"
>
  <div className="min-w-0">
    <p className="truncate text-sm text-text">
      {projectName(entry.projectId)}
    </p>
    <p className="text-xs text-text-muted">
      {ACTIVITY_TYPE_LABELS[entry.activityType]}
      {entry.description ? ` · ${entry.description}` : ""}
    </p>
    <div className="mt-1 empty:mt-0">
      <SuspicionBadge reasons={inspectEntry(entry).reasons} />
    </div>
  </div>
  <div className="flex shrink-0 items-center gap-2">
    <span className="tabular text-sm text-text-muted">
      {formatDuration(entry.durationSeconds)}
    </span>
    <Button
      variant="ghost"
      size="sm"
      onClick={() => {
        setEditing(entry);
        setFormOpen(true);
      }}
      icon={<Pencil size={14} strokeWidth={1.75} />}
    >
      Editar
    </Button>
  </div>
</li>
```

O botao tem texto sempre visivel de proposito: affordance escondida em hover e exatamente a causa raiz deste trabalho.

- [ ] **Step 5: Montar o formulario na pagina**

Logo antes de `<RecoveryModal />`, no fim do `return`:

```tsx
<EntryForm
  open={formOpen}
  entry={editing}
  onClose={() => setFormOpen(false)}
/>
```

- [ ] **Step 6: Rodar o teste e confirmar que passa**

Run: `npm run test -- src/features/dashboard/DashboardPage.test.tsx`
Expected: PASS — 3 testes.

- [ ] **Step 7: Conferir na tela**

Run: `npm run dev`
No Painel: cada sessao recente mostra "Editar"; clicar abre o modal de edicao com o botao vermelho "Excluir sessao"; o cabecalho do painel leva ao Historico; sessoes longas ou de madrugada mostram "Conferir?".

- [ ] **Step 8: Verificar tipos, lint e suite completa**

Run: `npm run typecheck && npm run lint && npm run test`
Expected: PASS nos tres.

- [ ] **Step 9: Commit**

```bash
git add src/features/dashboard/DashboardPage.tsx src/features/dashboard/DashboardPage.test.tsx
git commit -m "feat(painel): editar e excluir sessao direto das sessoes recentes"
```

---

### Task 6: Atualizar a documentacao do usuario

**Files:**
- Modify: `docs/USAGE.md`
- Modify: `CLAUDE.md` (secao "Estado atual")

- [ ] **Step 1: Documentar em `docs/USAGE.md`**

Substitua a secao `## Historico` (linhas 50-55) por:

```markdown
## Corrigir uma sessao

Toda sessao pode ser corrigida ou removida em dois lugares:

- No **Painel**, em *Sessoes recentes*, clique em *Editar* na linha da sessao.
- No **Historico**, use *Editar* ou *Excluir* na coluna Acoes.

A confirmacao de exclusao mostra projeto, data, periodo, duracao e o valor que
sai da conta. Nada e apagado de verdade: sessoes excluidas voltam marcando
*Mostrar excluidas* no Historico e clicando em *Restaurar*.

## Sessoes marcadas com "Conferir?"

Um selo ambar **Conferir?** aparece em sessoes de cronometro com duracao
implausivel: mais de **8 horas** seguidas, ou atravessando as **04:00**. E o
sintoma classico de cronometro esquecido ligado durante a noite. O selo nao
altera nada — so chama atencao para voce corrigir o inicio/fim antes de cobrar.

Sessoes manuais e reconstruidas nunca sao marcadas: elas foram criadas por
decisao explicita sua.

## Historico

Em *Historico* voce filtra por periodo, cliente e projeto; edita inicio/fim,
descricao, atividade e faturavel; **adiciona sessoes manuais**; e **exclui**
(com confirmacao). Sessoes excluidas podem ser **restauradas** em *Mostrar
excluidas*.
```

- [ ] **Step 2: Atualizar o "Estado atual" do `CLAUDE.md`**

No paragrafo "Alem do MVP", acrescente: aviso de sessao suspeita ("Conferir?") e edicao/exclusao de sessao a partir do Painel.

- [ ] **Step 3: Commit**

```bash
git add docs/USAGE.md CLAUDE.md
git commit -m "docs: corrigir sessoes e aviso de sessao suspeita"
```

---

## Notas de verificacao final

Depois da Task 6, rode a suite inteira uma vez e confirme que nada quebrou:

```bash
npm run typecheck && npm run lint && npm run test && npm run build
```

O backend nao foi tocado, entao `cargo test` nao precisa rodar — mas rodar nao custa nada se houver duvida.
