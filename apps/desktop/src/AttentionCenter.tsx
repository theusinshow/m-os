import { useCallback, useEffect, useRef, useState } from "react";
import { LazyMotion, m } from "framer-motion";
import { api } from "./api";
import { Button } from "./Button";
import { Icon } from "./Icon";
import { AnimatedList, AnimatedListItem } from "./motion/AnimatedList";
import { MOTION_DURATIONS, MOTION_EASINGS } from "./motion";
import type { Reminder } from "./types";

const loadMotionFeatures = () => import("./motionFeatures").then((module) => module.default);

/**
 * O Attention Center: a memória de atenção do M/OS.
 *
 * **É projeção, não entidade.** `CORE-FOUNDATION.md` §2 princípio 7 diz que
 * Kanban, Inbox, Library, Home e Search são visualizações, e o princípio 6
 * proíbe duplicar dado para exibir em outra superfície. Este painel lê os
 * Reminders e agrupa — do mesmo jeito que `calendar.rs::compose` faz com as
 * quatro fontes dele. Não há terceira tabela de itens.
 *
 * Mora no rodapé do rail e não como destino, por decisão registrada: a ADR-031
 * fixa que "Quick Capture e Settings continuam fora da contagem: eles não são
 * destinos de conteúdo, e o rodapé do rail é uma zona própria". O Attention
 * Center é superfície de sistema, não um substantivo do produto ao lado de
 * Tasks e Projects.
 */

/** Os grupos, na ordem em que se lê. */
type Bucket = "now" | "action" | "later";

const BUCKET_LABEL: Record<Bucket, string> = {
  now: "Agora",
  action: "Precisa de ação",
  later: "Depois",
};

/**
 * Em que grupo cada Reminder cai.
 *
 * `missed` fica em "Precisa de ação" e não num grupo próprio de perdidos: um
 * lembrete perdido não é uma categoria diferente de trabalho, é o mesmo
 * trabalho com atraso — e separá-lo criaria uma quarta lista que a pessoa
 * precisaria lembrar de olhar.
 */
function bucketOf(reminder: Reminder, now: number): Bucket {
  if (reminder.status === "missed") return "action";
  if (reminder.status === "due" || reminder.status === "delivered") return "action";

  const due = reminder.nextDueAt ? Date.parse(reminder.nextDueAt) : null;
  if (due !== null && due - now <= 60 * 60 * 1000) return "now";
  return "later";
}

/**
 * Tempo em palavras, nunca só um número solto.
 *
 * `DESIGN-FOUNDATIONS.md` §14 exige que nenhum estado dependa apenas de cor, e
 * a mesma lógica vale para o tempo: "1h" sozinho não diz se falta ou passou.
 */
function whenLabel(reminder: Reminder, now: number): string {
  if (!reminder.nextDueAt) return "";

  const due = Date.parse(reminder.nextDueAt);
  const minutes = Math.round(Math.abs(due - now) / 60000);
  const late = due < now;

  const span =
    minutes < 1
      ? "menos de um minuto"
      : minutes < 60
        ? `${minutes} min`
        : minutes < 60 * 24
          ? `${Math.round(minutes / 60)} h`
          : `${Math.round(minutes / (60 * 24))} d`;

  if (reminder.status === "snoozed") return `adiado · volta em ${span}`;
  return late ? `atrasado ${span}` : `em ${span}`;
}

/** Os adiamentos rápidos. Contextuais dependem de agenda, que não existe. */
const SNOOZES: ReadonlyArray<{ label: string; minutes: number }> = [
  { label: "15 min", minutes: 15 },
  { label: "1 hora", minutes: 60 },
  { label: "Amanhã", minutes: 0 },
];

/** O instante local de amanhã às 9h, resolvido aqui porque só a interface
 *  conhece o fuso de quem clicou. O backend recebe pronto e guarda UTC. */
function tomorrowMorning(): Date {
  const when = new Date();
  when.setDate(when.getDate() + 1);
  when.setHours(9, 0, 0, 0);
  return when;
}

export function AttentionCenter({ close, compose }: { close: () => void; compose: () => void }) {
  const [reminders, setReminders] = useState<Reminder[]>([]);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const panel = useRef<HTMLDivElement>(null);

  const refresh = useCallback(async () => {
    try {
      setReminders(await api.reminders());
      setError("");
    } catch (err) {
      setError((err as Error).message ?? "Falha ao carregar lembretes");
    }
  }, []);

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => setNow(Date.now()), 15000);
    return () => window.clearInterval(timer);
  }, [refresh]);

  useEffect(() => {
    panel.current?.focus();
    function onKey(event: globalThis.KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        close();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [close]);

  async function act(id: string, fn: () => Promise<unknown>) {
    setBusy(id);
    try {
      await fn();
      await refresh();
    } catch (err) {
      setError((err as Error).message ?? "Ação falhou");
    } finally {
      setBusy(null);
    }
  }

  const grouped: Record<Bucket, Reminder[]> = { now: [], action: [], later: [] };
  for (const reminder of reminders) {
    grouped[bucketOf(reminder, now)].push(reminder);
  }

  const empty = reminders.length === 0;

  return (
    <LazyMotion features={loadMotionFeatures} strict>
      <m.button
        aria-hidden="true"
        className="attention-scrim"
        onClick={close}
        tabIndex={-1}
        type="button"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        transition={{ duration: MOTION_DURATIONS.enter }}
      />
      <m.div
        aria-label="Atenção"
        className="attention-center"
        ref={panel}
        role="dialog"
        tabIndex={-1}
        initial={{ opacity: 0, x: -20, scale: 0.98 }}
        animate={{ opacity: 1, x: 0, scale: 1 }}
        exit={{ opacity: 0, x: -16, scale: 0.98 }}
        transition={{ duration: MOTION_DURATIONS.enter, ease: MOTION_EASINGS.enter }}
      >
        <header className="attention-header">
          <span className="micro-label">ATENÇÃO</span>
          <div className="button-line">
            <Button onClick={compose} size="sm" variant="secondary">
              Novo lembrete
            </Button>
            <button aria-label="Fechar" className="icon-button" onClick={close} type="button">
              <Icon name="close" />
            </button>
          </div>
        </header>

        {error ? (
          <p className="inline-error" role="alert">
            ! {error}
          </p>
        ) : null}

        {empty ? (
          <p className="attention-empty">
            Nada esperando por você.
          </p>
        ) : (
          (["action", "now", "later"] as const).map((bucket) =>
            grouped[bucket].length ? (
              <section className="attention-group" key={bucket}>
                <span className="micro-label">{BUCKET_LABEL[bucket]}</span>
                <AnimatedList className="attention-list">
                  {grouped[bucket].map((reminder) => (
                    <AnimatedListItem className="attention-item" key={reminder.id} itemKey={reminder.id}>
                      <div className="attention-body">
                        <strong>{reminder.title}</strong>
                        <span className="attention-when">{whenLabel(reminder, now)}</span>
                        {reminder.body ? <p>{reminder.body}</p> : null}
                      </div>
                      <div className="attention-actions">
                        <Button
                          disabled={busy === reminder.id}
                          onClick={() => void act(reminder.id, () => api.completeReminder(reminder.id))}
                          variant="secondary"
                        >
                          Concluir
                        </Button>
                        {reminder.policy.snoozeAllowed
                          ? SNOOZES.map((snooze) => (
                              <Button
                                disabled={busy === reminder.id}
                                key={snooze.label}
                                onClick={() =>
                                  void act(reminder.id, () =>
                                    api.snoozeReminder(
                                      reminder.id,
                                      snooze.minutes === 0
                                        ? tomorrowMorning()
                                        : new Date(Date.now() + snooze.minutes * 60000),
                                    ),
                                  )
                                }
                                variant="ghost"
                              >
                                {snooze.label}
                              </Button>
                            ))
                          : null}
                      </div>
                      {reminder.snoozeCount >= 5 ? (
                        <p className="attention-fatigue">
                          Adiado {reminder.snoozeCount} vezes.{" "}
                          <Button
                            onClick={() => void act(reminder.id, () => api.cancelReminder(reminder.id))}
                            size="sm"
                            variant="ghost"
                          >
                            Cancelar este lembrete
                          </Button>
                        </p>
                      ) : null}
                    </AnimatedListItem>
                  ))}
                </AnimatedList>
              </section>
            ) : null,
          )
        )}
      </m.div>
    </LazyMotion>
  );
}
