import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { Ring, RingLabel, stagger } from "./Ring";
import type { ActiveTimer, Project, ProjectTracking, TimeEntry } from "./types";

/**
 * Os widgets de tempo da Home.
 *
 * Existiam no catálogo do desenho e estavam bloqueados: o `Widgets.tsx` dizia
 * que dependiam de "tempo rastreado (CronoCAD, ainda não absorvido)". A absorção
 * aconteceu, e o bloqueio caiu.
 *
 * **Eles mostram HORAS, nunca dinheiro.** A conversão de hora em valor passa por
 * desconto de inatividade e arredondamento, que vivem no Rust (`settle`) — e
 * repetir essa conta aqui criaria um segundo caminho de cálculo capaz de
 * divergir do que vai na fatura. Quem quer o valor tem a página de Tempo, onde
 * o número vem pronto do backend.
 */

/** Meia-noite local do dia de uma data. */
function dayKey(iso: string) {
  const date = new Date(iso);
  return new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
}

/** `3,2 h` — a unidade do trabalho cobrado é a hora, e uma casa basta. */
function hoursOf(seconds: number) {
  return `${(seconds / 3600).toFixed(1).replace(".", ",")} h`;
}

/** `2h07`, para quando os minutos importam. */
function clockOf(seconds: number) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return hours ? `${hours}h${String(minutes).padStart(2, "0")}` : `${minutes}min`;
}

export type TrackedTime = {
  entries: TimeEntry[];
  tracking: ProjectTracking[];
  timer: ActiveTimer | null;
};

const EMPTY: TrackedTime = { entries: [], tracking: [], timer: null };

/**
 * Carrega o tempo rastreado, fora do `refresh()` da Home.
 *
 * Separado de propósito: o `refresh()` é o caminho de boot do aplicativo
 * inteiro, e um erro no rastreio de tempo não pode ser motivo para a Home não
 * abrir. Aqui a falha degrada para "sem dados" e os widgets somem — que é o
 * comportamento certo para um widget.
 */
export function useTrackedTime(): TrackedTime {
  const [data, setData] = useState<TrackedTime>(EMPTY);

  const load = useCallback(async () => {
    const [entries, tracking, timer] = await Promise.all([
      api.trackingEntries().catch(() => [] as TimeEntry[]),
      api.projectTracking().catch(() => [] as ProjectTracking[]),
      api.timerCurrent().catch(() => null),
    ]);
    setData({ entries, tracking, timer });
  }, []);

  useEffect(() => { void load(); }, [load]);

  // Encerrar o cronômetro numa página muda o número da Home na outra. Sem ouvir
  // os dois eventos, a Home mostraria as horas de antes até alguém reabrir o app.
  useEffect(() => {
    const handles = [listen("data-changed", () => { void load(); }), listen("timer-changed", () => { void load(); })];
    return () => { handles.forEach((handle) => void handle.then((dispose) => dispose())); };
  }, [load]);

  return data;
}

/** Segundos por dia, dos últimos `days` dias, do mais antigo para o mais novo. */
function dailySeconds(entries: TimeEntry[], days: number) {
  const today = new Date();
  const start = new Date(today.getFullYear(), today.getMonth(), today.getDate());
  start.setDate(start.getDate() - (days - 1));

  const buckets = new Map<number, number>();
  for (let index = 0; index < days; index += 1) {
    const date = new Date(start);
    date.setDate(start.getDate() + index);
    buckets.set(date.getTime(), 0);
  }
  for (const entry of entries) {
    const key = dayKey(entry.startedAt);
    if (buckets.has(key)) buckets.set(key, (buckets.get(key) ?? 0) + Math.max(0, entry.durationSeconds));
  }
  return [...buckets.entries()].map(([at, seconds]) => ({ at, seconds }));
}

/**
 * W-T1 · HOJE — quanto do dia já está registrado.
 *
 * O arco de 270° é a variante de tempo do desenho, e estava desenhada sem uso.
 * A abertura no pé dá o sentido de começo e fim do dia.
 *
 * A proporção é contra o MELHOR DIA dos últimos sete, e não contra uma meta
 * diária. O M/OS não tem meta diária de horas, e inventar oito horas faria o
 * anel medir uma régua que ninguém definiu — o mesmo motivo pelo qual o anel da
 * semana mede contra o pico, e não contra um alvo.
 *
 * **Sem o traço do instante**, embora o `Ring` ofereça: ele marca uma posição no
 * DIA, e o eixo deste anel é "horas contra o pico da semana". Seriam duas
 * escalas diferentes desenhadas na mesma volta, e o traço apontaria para nada.
 * O arco de 270° fica porque é a marca visual da família de tempo — a abertura
 * no pé —, e não uma afirmação sobre o eixo.
 */
export function TodayHours({ time }: { time: TrackedTime }) {
  const week = useMemo(() => dailySeconds(time.entries, 7), [time.entries]);
  const running = time.timer?.status === "running";

  const today = week[week.length - 1]?.seconds ?? 0;
  const peak = Math.max(1, ...week.map((day) => day.seconds));
  const best = week.slice(0, -1).reduce((top, day) => Math.max(top, day.seconds), 0);

  return (
    <div className="widget-progress">
      <Ring size={88} arc={270} segments={[{ value: today / peak }]}>
        <RingLabel value={hoursOf(today)} unit={running ? "CONTANDO" : "HOJE"} />
      </Ring>
      <div className="widget-progress-copy">
        <span className="micro-label">HOJE</span>
        <p className="hermes-quiet">
          {today === 0
            ? "Nenhuma hora registrada hoje."
            : best === 0
              ? "Primeiro dia com horas nesta semana."
              : today >= best
                ? "Seu melhor dia da semana."
                : `Melhor dia da semana: ${clockOf(best)}.`}
        </p>
      </div>
    </div>
  );
}

/**
 * W-T2 · SEMANA POR PROJECT — para onde o tempo foi.
 *
 * Anéis e não barras, pela mesma tese do resto da família: forma antes de
 * número, leitura em meio segundo. A proporção é contra o Project que mais
 * pesou na semana, então o maior anel fecha e os outros se leem por comparação
 * — que é a pergunta real ("onde foi parar a semana?"), e não "quanto por
 * cento de um alvo".
 *
 * Quatro, e não todos: com sete Projects os anéis ficariam pequenos demais para
 * a forma dizer alguma coisa, e a página de Tempo mostra a lista inteira.
 */
export function WeekByProject({ time, projects, onOpen }: {
  time: TrackedTime;
  projects: Project[];
  onOpen: () => void;
}) {
  const ranked = useMemo(() => {
    const since = new Date();
    since.setDate(since.getDate() - 6);
    since.setHours(0, 0, 0, 0);

    const perProject = new Map<string, number>();
    for (const entry of time.entries) {
      if (new Date(entry.startedAt) < since) continue;
      perProject.set(entry.projectId, (perProject.get(entry.projectId) ?? 0) + Math.max(0, entry.durationSeconds));
    }
    return [...perProject.entries()]
      .map(([id, seconds]) => ({
        id,
        seconds,
        name: projects.find((project) => project.id === id)?.name ?? "Project removido",
      }))
      // Um minuto de corte: um cronômetro parado por engano deixa segundos, e um
      // anel para trinta segundos ocuparia o lugar de um Project real.
      .filter((row) => row.seconds >= 60)
      .sort((left, right) => right.seconds - left.seconds)
      .slice(0, 4);
  }, [time.entries, projects]);

  const total = ranked.reduce((sum, row) => sum + row.seconds, 0);
  const peak = Math.max(1, ...ranked.map((row) => row.seconds));

  return (
    <div className="widget-week">
      <div className="widget-week-head">
        <span className="micro-label">SEMANA</span>
        <button type="button" className="filter-label" onClick={onOpen}>
          {total ? clockOf(total) : "SEM HORAS"}
        </button>
      </div>
      {ranked.length ? (
        // Grade própria e não a da semana: aquela é fixa em SETE colunas, e
        // quatro Projects ficariam espremidos à esquerda com três vãos mortos.
        <div className="widget-time-grid">
          {ranked.map((row, index) => (
            <div className="widget-week-day" key={row.id}>
              <Ring
                size={44}
                delay={stagger(index)}
                segments={[{ value: row.seconds / peak, depth: index === 0 ? 1 : 2 }]}
              >
                <span className="mos-ring-number">{(row.seconds / 3600).toFixed(0)}</span>
              </Ring>
              {/* `title` porque o rótulo trunca: o nome inteiro continua
                  alcançável sem o widget precisar crescer. */}
              <span className="micro-label widget-time-name" title={`${row.name} · ${clockOf(row.seconds)}`}>
                {row.name}
              </span>
            </div>
          ))}
        </div>
      ) : (
        <p className="hermes-quiet">Nenhuma hora nos últimos sete dias.</p>
      )}
    </div>
  );
}

/**
 * W-T3 · META — quanto falta no Project com meta.
 *
 * **Some quando não há meta**, e isso não é um caso de borda: é a regra. Um anel
 * preenchido contra um alvo que ninguém definiu ensina a confiar numa medida que
 * não existe — a mesma razão pela qual os widgets do catálogo ficaram de fora
 * até o tempo rastreado existir.
 *
 * O Project escolhido é o do cronômetro que está correndo; sem cronômetro, o que
 * tem meta e mais horas acumuladas. Nunca uma soma de metas: metas de projetos
 * diferentes não somam em nada que signifique alguma coisa.
 */
export function BudgetRing({ time, projects, onOpen }: {
  time: TrackedTime;
  projects: Project[];
  onOpen: (project: Project) => void;
}) {
  const target = useMemo(() => {
    const withBudget = time.tracking.filter((row) => row.budgetMinutes > 0);
    if (!withBudget.length) return null;

    const worked = (projectId: string) => time.entries
      .filter((entry) => entry.projectId === projectId)
      .reduce((sum, entry) => sum + Math.max(0, entry.durationSeconds), 0);

    const running = time.timer
      ? withBudget.find((row) => row.projectId === time.timer?.projectId)
      : undefined;
    const chosen = running
      ?? withBudget
        .map((row) => ({ row, seconds: worked(row.projectId) }))
        .sort((left, right) => right.seconds - left.seconds)[0]?.row;
    if (!chosen) return null;

    const project = projects.find((candidate) => candidate.id === chosen.projectId);
    if (!project) return null;

    return {
      project,
      seconds: worked(chosen.projectId),
      budgetSeconds: chosen.budgetMinutes * 60,
      live: chosen.projectId === time.timer?.projectId,
    };
  }, [time, projects]);

  if (!target) return null;

  const ratio = target.seconds / target.budgetSeconds;
  const over = target.seconds > target.budgetSeconds;
  const left = target.budgetSeconds - target.seconds;

  return (
    <div className="widget-progress">
      {/* Passa de 100% e o anel PARA em cheio, em vez de dar a segunda volta:
          uma volta a mais se lê como "começou de novo". O estouro é dito no
          texto, que é onde ele cabe sem mentir sobre a forma. */}
      <Ring size={88} segments={[{ value: Math.min(1, ratio) }]}>
        <RingLabel value={`${Math.round(ratio * 100)}%`} unit={target.live ? "CONTANDO" : "DA META"} />
      </Ring>
      <div className="widget-progress-copy">
        <span className="micro-label">META</span>
        <button type="button" className="filter-label" onClick={() => onOpen(target.project)}>
          {target.project.name}
        </button>
        <p className="hermes-quiet">
          {over
            ? `${clockOf(-left)} acima da meta de ${clockOf(target.budgetSeconds)}.`
            : `Faltam ${clockOf(left)} de ${clockOf(target.budgetSeconds)}.`}
        </p>
      </div>
    </div>
  );
}
