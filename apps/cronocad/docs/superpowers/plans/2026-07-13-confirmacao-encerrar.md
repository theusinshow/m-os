# Confirmacao ao Encerrar — Plano de Implementacao

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Impedir que o usuario encerre o cronometro por engano quando queria apenas pausar, exigindo confirmacao e oferecendo o Pausar como saida.

**Architecture:** Mudanca 100% frontend. O `TimerPanel` deixa de chamar `stop()` no clique do botao Encerrar e passa a abrir um `StopConfirmModal` (novo, construido sobre o `Modal` existente). O modal mostra o tempo ao vivo e oferece tres saidas: Pausar (primaria), Encerrar (danger), Cancelar (ghost). Em paralelo, a hierarquia visual dos botoes e invertida nos tres lugares onde o app hoje empurra o usuario para o Encerrar.

**Tech Stack:** React 18, TypeScript estrito, Tailwind, Vitest + @testing-library/react, Zustand.

Spec: `docs/superpowers/specs/2026-07-13-confirmacao-encerrar-design.md`

## Global Constraints

- TypeScript estrito. **`any` e proibido** (o ESLint erra).
- `npm run lint` deve passar com **0 warnings**.
- Import alias `@/` -> `src/`.
- Textos da UI em portugues, **sem acentos** (o codebase inteiro segue isso: "Cronometro", "Sessao", "Anotacoes"). Manter o padrao.
- Nao mexer no backend: `stop_timer`, `pause_timer` e o tray ficam **inalterados**.
- O `Button` tem exatamente 4 variantes: `primary` | `secondary` | `ghost` | `danger` (`src/components/ui/Button.tsx:4`). Nao criar novas.
- Nos testes, mockar a **camada de servico** (`@/services/*`), nunca os stores — e o padrao ja estabelecido em `src/stores/timerStore.test.ts:5-15`.

---

### Task 1: StopConfirmModal

**Files:**
- Create: `src/features/timer/StopConfirmModal.tsx`
- Test: `src/features/timer/StopConfirmModal.test.tsx`

**Interfaces:**
- Consumes: `Modal` (`@/components/ui/Modal`), `Button` (`@/components/ui/Button`), `useNow` (`@/hooks/useNow`), `elapsedSeconds` (`@/lib/duration`), `amountForDuration` (`@/lib/money`), `formatClock`/`formatCurrency` (`@/lib/format`), `ACTIVITY_TYPE_LABELS` (`@/lib/labels`), tipos `ActiveTimer` e `Project` (`@/types/domain`).
- Produces:
  ```ts
  interface StopConfirmModalProps {
    open: boolean;
    timer: ActiveTimer;
    project: Project | null;
    busy: boolean;
    onCancel: () => void;
    onPause: () => void;
    onStop: () => void;
  }
  export function StopConfirmModal(props: StopConfirmModalProps): JSX.Element | null;
  ```
  O componente e **puramente apresentacional**: nao chama o store. Quem decide o que fazer e o `TimerPanel` (Task 2).

- [ ] **Step 1: Escrever o teste que falha**

Criar `src/features/timer/StopConfirmModal.test.tsx`:

```tsx
import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { StopConfirmModal } from "./StopConfirmModal";
import type { ActiveTimer, Project } from "@/types/domain";

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
  createdAt: "2026-07-11T08:00:00Z",
  updatedAt: "2026-07-11T08:00:00Z",
  archivedAt: null,
};

const running: ActiveTimer = {
  id: "t1",
  projectId: "p1",
  startedAt: "2026-07-11T08:00:00Z",
  lastResumedAt: "2026-07-11T08:00:00Z",
  accumulatedSeconds: 0,
  status: "running",
  description: null,
  activityType: "drawing",
  createdAt: "2026-07-11T08:00:00Z",
  updatedAt: "2026-07-11T08:00:00Z",
};

const paused: ActiveTimer = { ...running, status: "paused" };

function setup(timer: ActiveTimer) {
  const handlers = {
    onCancel: vi.fn(),
    onPause: vi.fn(),
    onStop: vi.fn(),
  };
  render(
    <StopConfirmModal
      open
      timer={timer}
      project={project}
      busy={false}
      {...handlers}
    />,
  );
  return handlers;
}

describe("StopConfirmModal", () => {
  it("mostra o projeto e o tempo que sera gravado", () => {
    setup(running);
    expect(screen.getByText(/Residencial Aurora/)).toBeInTheDocument();
    expect(screen.getByText(/^\d{2}:\d{2}:\d{2}$/)).toBeInTheDocument();
  });

  it("oferece pausar quando o cronometro esta rodando", async () => {
    const h = setup(running);
    await userEvent.click(
      screen.getByRole("button", { name: /Pausar em vez disso/i }),
    );
    expect(h.onPause).toHaveBeenCalledOnce();
    expect(h.onStop).not.toHaveBeenCalled();
  });

  it("nao oferece pausar quando o cronometro ja esta pausado", () => {
    setup(paused);
    expect(
      screen.queryByRole("button", { name: /Pausar em vez disso/i }),
    ).not.toBeInTheDocument();
  });

  it("encerra somente quando o usuario confirma", async () => {
    const h = setup(running);
    await userEvent.click(
      screen.getByRole("button", { name: /Encerrar mesmo assim/i }),
    );
    expect(h.onStop).toHaveBeenCalledOnce();
  });

  it("cancelar nao encerra nem pausa", async () => {
    const h = setup(running);
    await userEvent.click(screen.getByRole("button", { name: /^Cancelar$/i }));
    expect(h.onCancel).toHaveBeenCalledOnce();
    expect(h.onStop).not.toHaveBeenCalled();
    expect(h.onPause).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Rodar o teste e confirmar que falha**

Run: `npm run test -- StopConfirmModal`
Expected: FAIL — "Failed to resolve import ./StopConfirmModal" (o arquivo ainda nao existe).

- [ ] **Step 3: Verificar se `@testing-library/user-event` esta instalado**

Run: `node -e "console.log(require('./package.json').devDependencies['@testing-library/user-event'] ?? 'AUSENTE')"`

Se imprimir `AUSENTE`, instalar: `npm install -D @testing-library/user-event`
Se imprimir uma versao, seguir sem instalar nada.

- [ ] **Step 4: Implementar o componente**

Criar `src/features/timer/StopConfirmModal.tsx`:

```tsx
import type { ActiveTimer, Project } from "@/types/domain";
import { elapsedSeconds } from "@/lib/duration";
import { amountForDuration } from "@/lib/money";
import { formatClock, formatCurrency } from "@/lib/format";
import { ACTIVITY_TYPE_LABELS } from "@/lib/labels";
import { useNow } from "@/hooks/useNow";
import { Modal } from "@/components/ui/Modal";
import { Button } from "@/components/ui/Button";

interface StopConfirmModalProps {
  open: boolean;
  timer: ActiveTimer;
  project: Project | null;
  busy: boolean;
  onCancel: () => void;
  onPause: () => void;
  onStop: () => void;
}

/**
 * Confirmacao antes de encerrar o cronometro (regra critica 8: nunca encerrar
 * tempo sem decisao consciente). Encerrar grava a sessao em `time_entries` e e
 * irreversivel — nao existe comando que reabra uma sessao encerrada.
 *
 * O erro mais provavel e querer pausar e encerrar por engano, entao o botao
 * primario e o Pausar. O tempo continua correndo enquanto o modal esta aberto:
 * o cronometro nao para, so a decisao e adiada.
 *
 * Apresentacional: nao chama o store. As acoes vem do TimerPanel.
 */
export function StopConfirmModal({
  open,
  timer,
  project,
  busy,
  onCancel,
  onPause,
  onStop,
}: StopConfirmModalProps) {
  const now = useNow(1000);
  const seconds = elapsedSeconds(timer, now);
  const amount = amountForDuration(seconds, project?.hourlyRateCents ?? 0);
  const running = timer.status === "running";

  return (
    <Modal
      open={open}
      title="Encerrar sessao?"
      onClose={onCancel}
      footer={
        <>
          <Button variant="ghost" onClick={onCancel} disabled={busy}>
            Cancelar
          </Button>
          <Button variant="danger" onClick={onStop} disabled={busy}>
            Encerrar mesmo assim
          </Button>
          {running && (
            <Button variant="primary" onClick={onPause} disabled={busy}>
              Pausar em vez disso
            </Button>
          )}
        </>
      }
    >
      <p className="text-sm text-text">
        {project ? project.name : "Projeto"}
        {project?.code ? ` · ${project.code}` : ""} ·{" "}
        {ACTIVITY_TYPE_LABELS[timer.activityType]}
      </p>

      <div className="my-5 flex items-baseline justify-center gap-3">
        <span className="tabular text-4xl font-semibold tracking-tight text-text">
          {formatClock(seconds)}
        </span>
        <span className="tabular text-sm text-text-muted">
          {formatCurrency(amount)}
        </span>
      </div>

      <p className="text-sm text-text-muted">
        Isso vira um registro definitivo no historico. Se voce so vai dar uma
        pausa, use Pausar — o cronometro continua de onde parou.
      </p>
    </Modal>
  );
}
```

- [ ] **Step 5: Rodar os testes e confirmar que passam**

Run: `npm run test -- StopConfirmModal`
Expected: PASS — 5 testes.

- [ ] **Step 6: Commit**

```bash
git add src/features/timer/StopConfirmModal.tsx src/features/timer/StopConfirmModal.test.tsx
git commit -m "feat(timer): modal de confirmacao ao encerrar o cronometro"
```

---

### Task 2: Ligar o modal ao TimerPanel

**Files:**
- Modify: `src/features/timer/TimerPanel.tsx:75-112` (bloco do cronometro ativo)
- Test: `src/features/timer/TimerPanel.test.tsx` (criar)

**Interfaces:**
- Consumes: `StopConfirmModal` da Task 1 (assinatura exata no bloco "Produces" daquela task).
- Produces: nada para tasks posteriores.

O `TimerPanel` ganha um estado local `confirmingStop: boolean`. O botao Encerrar passa a apenas abri-lo. As acoes do modal reusam o `run()` que ja existe (`TimerPanel.tsx:55-65`), que cuida de `busy` e `error`.

- [ ] **Step 1: Escrever o teste que falha**

Criar `src/features/timer/TimerPanel.test.tsx`:

```tsx
import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import type { ActiveTimer, Project } from "@/types/domain";

vi.mock("@/services/timer", () => ({
  getActiveTimer: vi.fn(),
  startTimer: vi.fn(),
  pauseTimer: vi.fn(),
  resumeTimer: vi.fn(),
  stopTimer: vi.fn(),
  discardTimer: vi.fn(),
  discountIdle: vi.fn(),
}));
vi.mock("@/services/timeEntries", () => ({
  listTimeEntries: vi.fn().mockResolvedValue([]),
}));

import * as timerService from "@/services/timer";
import { useTimerStore } from "@/stores/timerStore";
import { useCatalogStore } from "@/stores/catalogStore";
import { TimerPanel } from "./TimerPanel";

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
  createdAt: "2026-07-11T08:00:00Z",
  updatedAt: "2026-07-11T08:00:00Z",
  archivedAt: null,
};

const timer: ActiveTimer = {
  id: "t1",
  projectId: "p1",
  startedAt: "2026-07-11T08:00:00Z",
  lastResumedAt: "2026-07-11T08:00:00Z",
  accumulatedSeconds: 0,
  status: "running",
  description: null,
  activityType: "drawing",
  createdAt: "2026-07-11T08:00:00Z",
  updatedAt: "2026-07-11T08:00:00Z",
};

function renderPanel() {
  render(
    <MemoryRouter>
      <TimerPanel />
    </MemoryRouter>,
  );
}

describe("TimerPanel — encerrar com confirmacao", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useCatalogStore.setState({ projects: [project] });
    useTimerStore.setState({
      activeTimer: timer,
      loaded: true,
      error: null,
      recoveryPending: false,
    });
  });

  it("clicar em Encerrar nao encerra: apenas abre a confirmacao", async () => {
    renderPanel();
    await userEvent.click(screen.getByRole("button", { name: /^Encerrar$/i }));

    expect(timerService.stopTimer).not.toHaveBeenCalled();
    expect(
      screen.getByRole("dialog", { name: /Encerrar sessao/i }),
    ).toBeInTheDocument();
  });

  it("encerra apenas apos confirmar", async () => {
    vi.mocked(timerService.stopTimer).mockResolvedValue({ id: "e1" } as never);
    renderPanel();

    await userEvent.click(screen.getByRole("button", { name: /^Encerrar$/i }));
    await userEvent.click(
      screen.getByRole("button", { name: /Encerrar mesmo assim/i }),
    );

    expect(timerService.stopTimer).toHaveBeenCalledOnce();
  });

  it("Pausar em vez disso pausa e nunca encerra", async () => {
    vi.mocked(timerService.pauseTimer).mockResolvedValue({
      ...timer,
      status: "paused",
    });
    renderPanel();

    await userEvent.click(screen.getByRole("button", { name: /^Encerrar$/i }));
    await userEvent.click(
      screen.getByRole("button", { name: /Pausar em vez disso/i }),
    );

    expect(timerService.pauseTimer).toHaveBeenCalledOnce();
    expect(timerService.stopTimer).not.toHaveBeenCalled();
  });

  it("cancelar fecha a confirmacao sem tocar no cronometro", async () => {
    renderPanel();

    await userEvent.click(screen.getByRole("button", { name: /^Encerrar$/i }));
    await userEvent.click(screen.getByRole("button", { name: /^Cancelar$/i }));

    expect(timerService.stopTimer).not.toHaveBeenCalled();
    expect(timerService.pauseTimer).not.toHaveBeenCalled();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Rodar o teste e confirmar que falha**

Run: `npm run test -- TimerPanel`
Expected: FAIL — o primeiro teste falha porque `stopTimer` **foi** chamado (hoje o clique encerra direto) e nao existe nenhum `dialog`.

- [ ] **Step 3: Implementar**

Em `src/features/timer/TimerPanel.tsx`:

3a. Adicionar o import (junto aos outros imports do topo):

```tsx
import { StopConfirmModal } from "./StopConfirmModal";
```

3b. Adicionar o estado local, logo abaixo de `const [busy, setBusy] = useState(false);`:

```tsx
const [confirmingStop, setConfirmingStop] = useState(false);
```

3c. Substituir todo o bloco `if (activeTimer) { ... }` (hoje em `TimerPanel.tsx:75-112`) por:

```tsx
  if (activeTimer) {
    const running = activeTimer.status === "running";
    return (
      <Panel className="border-l-2 border-l-accent p-6">
        <TimerCard timer={activeTimer} project={activeProject} />
        <div className="mt-6 flex gap-2">
          {running ? (
            <Button
              variant="primary"
              onClick={() => void run(pause)}
              disabled={busy}
              icon={<Pause size={16} strokeWidth={2} />}
            >
              Pausar
            </Button>
          ) : (
            <Button
              variant="primary"
              onClick={() => void run(resume)}
              disabled={busy}
              icon={<Play size={16} strokeWidth={2} />}
            >
              Continuar
            </Button>
          )}
          <Button
            variant="danger"
            onClick={() => setConfirmingStop(true)}
            disabled={busy}
            icon={<Square size={16} strokeWidth={2} />}
          >
            Encerrar
          </Button>
        </div>
        {error && <p className="mt-3 text-sm text-danger">{error}</p>}

        <StopConfirmModal
          open={confirmingStop}
          timer={activeTimer}
          project={activeProject}
          busy={busy}
          onCancel={() => setConfirmingStop(false)}
          onPause={() => {
            setConfirmingStop(false);
            void run(pause);
          }}
          onStop={() => {
            setConfirmingStop(false);
            void run(stop);
          }}
        />
      </Panel>
    );
  }
```

Notas para quem implementa:
- O `Pausar` virou `variant="primary"` (era `secondary`). Essa e a correcao de hierarquia: a acao reversivel e cotidiana e a que deve chamar mais atencao.
- O `Encerrar` continua `danger` — no `Button.tsx:21-22` essa variante ja e discreta (fundo transparente, so o texto em vermelho). Nao precisa mudar.
- `setConfirmingStop(false)` vem **antes** do `run(...)` para o modal fechar imediatamente; o erro, se houver, aparece no `<p>` do painel, que ja existe.

- [ ] **Step 4: Rodar os testes e confirmar que passam**

Run: `npm run test -- TimerPanel`
Expected: PASS — 4 testes.

- [ ] **Step 5: Commit**

```bash
git add src/features/timer/TimerPanel.tsx src/features/timer/TimerPanel.test.tsx
git commit -m "feat(timer): exigir confirmacao para encerrar e promover Pausar a acao primaria"
```

---

### Task 3: Hierarquia nos modais de monitoramento

**Files:**
- Modify: `src/features/monitoring/MonitorPrompts.tsx:44-86` (`QuitPrompt`) e `:181-212` (`ClosePrompt`)

**Interfaces:**
- Consumes: nada das tasks anteriores.
- Produces: nada.

Hoje esses dois modais colocam **Encerrar como `primary`** e Pausar como `secondary` — ou seja, o app empurra o usuario exatamente para o erro que a Task 2 acabou de proteger. Sem esta task, a correcao fica pela metade.

Nao ha teste automatizado nesta task: a mudanca e so de variante visual, sem logica nova. A verificacao e o `lint` + `typecheck` e a checagem manual da Task 4.

- [ ] **Step 1: Corrigir o `ClosePrompt` (CAD fechado)**

Em `src/features/monitoring/MonitorPrompts.tsx`, no `footer` do `ClosePrompt`, trocar as variantes:

```tsx
      footer={
        <>
          <Button variant="ghost" onClick={clearClose} disabled={busy}>
            Manter ativo
          </Button>
          <Button
            variant="danger"
            onClick={() => void act(stop)}
            disabled={busy}
          >
            Encerrar
          </Button>
          <Button
            variant="primary"
            onClick={() => void act(pause)}
            disabled={busy}
          >
            Pausar
          </Button>
        </>
      }
```

(Pausar era `secondary` -> vira `primary`; Encerrar era `primary` -> vira `danger`. A ordem tambem muda: a acao primaria fica por ultimo, encostada na borda, como nos outros modais do app.)

- [ ] **Step 2: Corrigir o `QuitPrompt` (sair do app)**

No `footer` do `QuitPrompt`:

```tsx
      footer={
        <>
          <Button variant="ghost" onClick={clearQuit} disabled={busy}>
            Cancelar
          </Button>
          <Button
            variant="ghost"
            onClick={() => void act()}
            disabled={busy}
          >
            Sair assim mesmo
          </Button>
          <Button
            variant="danger"
            onClick={() => void act(stop)}
            disabled={busy}
          >
            Encerrar e sair
          </Button>
          <Button
            variant="primary"
            onClick={() => void act(pause)}
            disabled={busy}
          >
            Pausar e sair
          </Button>
        </>
      }
```

("Encerrar e sair" era `primary` -> vira `danger`; "Pausar e sair" era `secondary` -> vira `primary`; "Sair assim mesmo" era `secondary` -> vira `ghost`, porque nao e a acao recomendada.)

- [ ] **Step 3: Verificar que nada quebrou**

Run: `npm run typecheck && npm run lint && npm run test`
Expected: typecheck sem erros; lint com 0 warnings; **todos** os testes passando (os novos e os que ja existiam).

- [ ] **Step 4: Commit**

```bash
git add src/features/monitoring/MonitorPrompts.tsx
git commit -m "fix(monitoring): promover Pausar sobre Encerrar nos avisos de CAD fechado e saida"
```

---

### Task 4: Verificacao no app real

**Files:** nenhum (verificacao manual).

**Interfaces:**
- Consumes: o app inteiro, ja com as Tasks 1-3.
- Produces: nada.

**PRE-REQUISITO — LEIA ANTES DE RODAR:** o app instalado (`%LOCALAPPDATA%\CronoCAD\cronocad.exe`) usa **o mesmo banco SQLite** que o `tauri:dev`. Duas instancias escrevendo no mesmo banco corrompem dados. **O app instalado precisa estar fechado**, e o cronometro do usuario **pausado ou encerrado** (nunca descartado) antes de comecar.

- [ ] **Step 1: Confirmar que o app instalado esta fechado**

Run: `powershell -NoProfile -Command "Get-Process cronocad -ErrorAction SilentlyContinue | Select-Object Id, Path"`
Expected: **sem saida**. Se algo aparecer, **pare** e peca ao usuario para fechar o app.

- [ ] **Step 2: Subir o app em modo dev**

Run: `npm run tauri:dev`

- [ ] **Step 3: Percorrer o checklist manual**

Com um projeto qualquer:

1. Iniciar o cronometro. Conferir que **Pausar** e agora o botao em destaque (fundo de acento) e **Encerrar** e o discreto (texto vermelho, sem preenchimento).
2. Clicar em **Encerrar**. O modal abre. O cronometro **continua correndo** (o tempo no modal avanca a cada segundo).
3. Clicar em **Pausar em vez disso**. O modal fecha, o cronometro pausa e **nada** e gravado no Historico.
4. Clicar em **Continuar**, depois em **Encerrar** -> **Cancelar**. Nada acontece; o cronometro segue rodando.
5. **Encerrar** -> **Encerrar mesmo assim**. A sessao aparece no Historico com a duracao correta.
6. Iniciar, **Pausar**, e entao **Encerrar**: o modal abre **sem** o botao "Pausar em vez disso".
7. Fechar o app com o cronometro ativo: no aviso de saida, **Pausar e sair** e o botao em destaque.

- [ ] **Step 4: Atualizar o CHANGELOG e commitar**

Acrescentar a mudanca ao `CHANGELOG.md` seguindo o formato ja usado no arquivo, e:

```bash
git add CHANGELOG.md
git commit -m "docs: registrar confirmacao ao encerrar no changelog"
git push
```
