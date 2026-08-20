import { useEffect, useRef, useState } from "react";
import { LazyMotion, m } from "framer-motion";
import { api } from "./api";
import { Button } from "./Button";
import { MOTION_DURATIONS, MOTION_EASINGS } from "./motion";
import type { Reminder } from "./types";

const loadMotionFeatures = () => import("./motionFeatures").then((module) => module.default);

/**
 * Criar lembrete. A operação precisa ser rápida acima de tudo.
 *
 * `UX-PRINCIPLES.md` §7 pede entrada natural antes de formulário e §8 pede
 * progressive disclosure. Aqui isso vira: **título e quando, e nada mais**. Nota
 * e prioridade existem atrás de um clique, e só aparecem para quem os quer.
 *
 * **Todos os instantes são calculados AQUI**, e não no backend. "Amanhã de
 * manhã" é um conceito local: o backend guarda UTC e não conhece o fuso de quem
 * clicou. Meia-noite em UTC é nove da noite no Brasil — mandar o cálculo para lá
 * faria "amanhã" começar hoje à noite. É o mesmo padrão que o lembrete do
 * monitor já usa, e a regra normativa da `CORE-FOUNDATION.md` §5.
 *
 * O que NÃO está aqui, e a ausência é deliberada: repetição, e qualquer opção
 * relativa a prazo ou evento. As decisões D-1 e D-4 deixaram o M/OS sem prazo em
 * Task e sem entidade Event, então não há âncora de tempo futuro para
 * referenciar. Um campo desabilitado ensinaria que a capacidade existe e está
 * quebrada; a ausência é honesta (`ATTENTION-SYSTEM.md` §35.1).
 */

type Choice = { label: string; resolve: () => Date };

/** As opções rápidas. Relógio apenas — sugestão por agenda exigiria agenda. */
const CHOICES: ReadonlyArray<Choice> = [
  { label: "15 min", resolve: () => new Date(Date.now() + 15 * 60000) },
  { label: "1 hora", resolve: () => new Date(Date.now() + 60 * 60000) },
  { label: "3 horas", resolve: () => new Date(Date.now() + 3 * 60 * 60000) },
  {
    label: "Hoje 18h",
    resolve: () => {
      const when = new Date();
      when.setHours(18, 0, 0, 0);
      return when;
    },
  },
  {
    label: "Amanhã 9h",
    resolve: () => {
      const when = new Date();
      when.setDate(when.getDate() + 1);
      when.setHours(9, 0, 0, 0);
      return when;
    },
  },
  {
    label: "Segunda 9h",
    resolve: () => {
      const when = new Date();
      // 8 - dia da semana, com resto 7 quando hoje já é segunda: pedir "segunda"
      // numa segunda quer dizer a próxima, e não daqui a instante nenhum.
      const ahead = ((8 - when.getDay()) % 7) || 7;
      when.setDate(when.getDate() + ahead);
      when.setHours(9, 0, 0, 0);
      return when;
    },
  },
];

/** Uma opção que já passou não é oferecida: "Hoje 18h" às 19h não quer dizer
 *  nada, e o backend a recusaria de todo jeito. */
function available(): Choice[] {
  const now = Date.now();
  return CHOICES.filter((choice) => choice.resolve().getTime() > now);
}

/** `datetime-local` fala no fuso do usuário e não aceita sufixo de zona. */
function toLocalInput(when: Date): string {
  const shifted = new Date(when.getTime() - when.getTimezoneOffset() * 60000);
  return shifted.toISOString().slice(0, 16);
}

function whenLabel(when: Date): string {
  return when.toLocaleString("pt-BR", {
    weekday: "short",
    day: "2-digit",
    month: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function ReminderComposer({
  close,
  created,
}: {
  close: () => void;
  created: (reminder: Reminder) => void;
}) {
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [when, setWhen] = useState<Date>(() => available()[0]?.resolve() ?? new Date(Date.now() + 900000));
  const [custom, setCustom] = useState(false);
  const [details, setDetails] = useState(false);
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const panel = useRef<HTMLDivElement>(null);

  useEffect(() => {
    input.current?.focus();
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        close();
      }
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [close]);

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    if (!title.trim() || saving) return;

    setSaving(true);
    try {
      created(await api.createReminder(title.trim(), body.trim(), when));
      close();
    } catch (nextError) {
      setError(String(nextError));
      setSaving(false);
    }
  }

  const choices = available();

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
        aria-label="Novo lembrete"
        className="reminder-composer"
        ref={panel}
        role="dialog"
        initial={{ opacity: 0, scale: 0.98, y: -6 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.98, y: -4 }}
        transition={{ duration: MOTION_DURATIONS.enter, ease: MOTION_EASINGS.enter }}
      >
        <form className="stack-form" onSubmit={submit}>
          <label>
            <span>LEMBRAR DE</span>
            <input
              onChange={(event) => setTitle(event.currentTarget.value)}
              placeholder="Enviar a proposta"
              ref={input}
              value={title}
            />
          </label>

          <fieldset className="composer-when">
            <legend className="micro-label">QUANDO</legend>
            <div className="composer-choices">
              {choices.map((choice) => {
                const instant = choice.resolve();
                const active = !custom && Math.abs(instant.getTime() - when.getTime()) < 60000;
                return (
                  <Button
                    aria-pressed={active}
                    key={choice.label}
                    onClick={() => {
                      setCustom(false);
                      setWhen(choice.resolve());
                    }}
                    size="sm"
                    variant={active ? "primary" : "ghost"}
                  >
                    {choice.label}
                  </Button>
                );
              })}
              <Button
                aria-pressed={custom}
                onClick={() => setCustom(true)}
                size="sm"
                variant={custom ? "primary" : "ghost"}
              >
                Escolher
              </Button>
            </div>

            {custom ? (
              <input
                aria-label="Data e hora"
                min={toLocalInput(new Date())}
                onChange={(event) => {
                  const parsed = new Date(event.currentTarget.value);
                  if (!Number.isNaN(parsed.getTime())) setWhen(parsed);
                }}
                type="datetime-local"
                value={toLocalInput(when)}
              />
            ) : null}

            {/* O instante resolvido, sempre visível. Um lembrete que dispara em
                hora diferente da que a pessoa achou que escolheu é pior que um
                lembrete que não dispara. */}
            <p className="composer-resolved" aria-live="polite">
              {whenLabel(when)}
            </p>
          </fieldset>

          {details ? (
            <label>
              <span>NOTA</span>
              <textarea
                onChange={(event) => setBody(event.currentTarget.value)}
                rows={3}
                value={body}
              />
            </label>
          ) : (
            <Button onClick={() => setDetails(true)} size="sm" variant="ghost">
              Adicionar nota
            </Button>
          )}

          {error ? (
            <p className="inline-error" role="alert">
              ! {error}
            </p>
          ) : null}

          <div className="form-actions">
            <Button disabled={saving} onClick={close} variant="ghost">
              Cancelar
            </Button>
            <Button disabled={!title.trim() || saving} type="submit" variant="primary">
              {saving ? "Criando" : "Criar"}
            </Button>
          </div>
        </form>
      </m.div>
    </LazyMotion>
  );
}
