import { useEffect, useRef, useState } from "react";
import { LazyMotion, m } from "framer-motion";
import { api } from "./api";
import { Button } from "./Button";
import { MOTION_DURATIONS, MOTION_EASINGS } from "./motion";
import { whenChoices } from "./lembretes";
import type { Recurrence, RecurrenceRule, Reminder, ReminderTarget } from "./types";

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
 *
 * # O alvo, quando existe, não é um campo
 *
 * Ele chega pronto de quem abriu — o botão "Lembrar" da Task manda a Task —, e
 * a folha mostra de que entidade se trata em vez de perguntar. Um seletor de
 * "prender a quê" na tela de criar lembrete faria pagar em decisão o que o
 * contexto já respondeu: quem clicou no botão dentro da Task já disse a qual.
 *
 * Sem alvo, o lembrete nasce solto — e isso continua sendo legítimo. Nem tudo
 * que se precisa lembrar é uma entidade do M/OS.
 */

/* As opcoes de "quando" e a limpeza do que ja passou moram em `lembretes.ts`.
   Elas eram daqui, e sairam quando o Attention Center passou a precisar das
   mesmas: duas listas de atalhos de tempo no mesmo app divergiriam, e "amanha
   9h" acabaria significando duas horas diferentes em duas telas. */

/** As repeticoes que a tela oferece. Seis, e nenhuma marcada por default.

   Nao ha editor de regra livre aqui de proposito: um construtor de recorrencia
   com ordinal, dia da semana e ancora e uma tela inteira, e quem cria um
   lembrete quer ser lembrado — nao configurar. O que falta se escreve pelo
   Hermes, que fala a mesma linguagem do dominio. */
const REPETICOES: ReadonlyArray<{ label: string; rule: RecurrenceRule | null }> = [
  { label: "Nao repete", rule: null },
  { label: "Todo dia", rule: { kind: "daily" } },
  { label: "Dias uteis", rule: { kind: "weekdays" } },
  { label: "Toda semana", rule: { kind: "everyWeeks", weeks: 1 } },
  { label: "Todo mes", rule: { kind: "everyDays", days: 30 } },
];

/** Os adiantamentos oferecidos, em minutos. Espelham `LEAD_PRESETS` do dominio. */
const ADIANTAMENTOS: ReadonlyArray<{ label: string; minutes: number }> = [
  { label: "15 min antes", minutes: 15 },
  { label: "1 hora antes", minutes: 60 },
  { label: "2 horas antes", minutes: 120 },
  { label: "1 dia antes", minutes: 24 * 60 },
];

/** `datetime-local` fala no fuso do usuário e não aceita sufixo de zona. */
function toLocalInput(when: Date): string {
  const shifted = new Date(when.getTime() - when.getTimezoneOffset() * 60000);
  return shifted.toISOString().slice(0, 16);
}

/** A regra de repeticao, montada a partir do instante escolhido.

   Hora e minuto LOCAIS mais o deslocamento em que a regra nasceu: "todo dia as
   08:00" quer dizer oito da manha onde a pessoa esta. Ver
   `crates/mos-core/src/recurrence.rs`. */
function regraDe(rule: RecurrenceRule, quando: Date): Recurrence {
  return {
    rule,
    anchor: "fixed",
    hour: quando.getHours(),
    minute: quando.getMinutes(),
    offsetMinutes: -quando.getTimezoneOffset(),
  };
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
  initialTitle = "",
  target,
  targetLabel,
}: {
  close: () => void;
  created: (reminder: Reminder) => void;
  /** O que já se sabe que se quer lembrar. Vem preenchido, e continua editável. */
  initialTitle?: string;
  /** A entidade a que ele se prende, quando alguém abriu a folha de dentro dela. */
  target?: ReminderTarget;
  /** Como chamar essa entidade na tela. "TASK", "PROJECT". */
  targetLabel?: string;
}) {
  const [title, setTitle] = useState(initialTitle);
  const [body, setBody] = useState("");
  const [when, setWhen] = useState<Date>(() => whenChoices(new Date())[0]?.resolve() ?? new Date(Date.now() + 900000));
  const [custom, setCustom] = useState(false);
  const [details, setDetails] = useState(false);
  /* "Nao me deixa esquecer". Desligado por default, e a decisao e da pessoa:
     um sistema que decide sozinho insistir e um sistema que se aprende a
     silenciar. */
  const [persistente, setPersistente] = useState(false);
  /* Sem data. Nao e lembrete pela metade — e a resposta honesta para o que se
     quer nao esquecer sem se querer ser interrompido. */
  const [semData, setSemData] = useState(false);
  const [repeticao, setRepeticao] = useState<RecurrenceRule | null>(null);
  const [adiantamentos, setAdiantamentos] = useState<number[]>([]);
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
      created(
        await api.novoLembrete({
          title: title.trim(),
          body: body.trim(),
          at: semData ? null : when,
          target,
          persistent: persistente,
          // A regra nasce do instante que a pessoa escolheu: ela ja disse a
          // hora ao escolher "amanha 9h", e perguntar de novo seria perguntar
          // duas vezes a mesma coisa.
          recurrence: repeticao && !semData ? regraDe(repeticao, when) : null,
          leads: semData ? [] : adiantamentos,
        }),
      );
      close();
    } catch (nextError) {
      setError(String(nextError));
      setSaving(false);
    }
  }

  const choices = whenChoices(new Date());

  return (
    <LazyMotion features={loadMotionFeatures} strict>
      <m.button
        aria-hidden="true"
        className="attention-scrim attention-scrim--composer"
        onClick={close}
        tabIndex={-1}
        type="button"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        transition={{ duration: MOTION_DURATIONS.enter }}
      />
      <m.div
        aria-label={targetLabel ? `Lembrete para ${targetLabel}` : "Novo lembrete"}
        className="reminder-composer"
        ref={panel}
        role="dialog"
        initial={{ opacity: 0, scale: 0.98, y: -6 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.98, y: -4 }}
        transition={{ duration: MOTION_DURATIONS.enter, ease: MOTION_EASINGS.enter }}
      >
        <form className="stack-form" onSubmit={submit}>
          {/* De onde a folha saiu. Sem isto, um lembrete criado de dentro de uma
              Task seria indistinguível de um lembrete solto no instante em que
              mais importa distinguir: antes de confirmar. */}
          {targetLabel ? <p className="composer-target">PRESO À {targetLabel}</p> : null}
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
              <Button
                aria-pressed={semData}
                onClick={() => setSemData((atual) => !atual)}
                size="sm"
                variant={semData ? "primary" : "ghost"}
              >
                Algum dia
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
              {semData ? "Sem data — nao vai interromper voce" : whenLabel(when)}
            </p>
          </fieldset>

          {/* "Nao me deixa esquecer" fica no primeiro nivel, e nao atras do
              "mais opcoes": e a escolha que muda o COMPORTAMENTO do lembrete, e
              esconde-la faria a capacidade central do sistema depender de a
              pessoa descobrir um link. */}
          <label className="composer-switch">
            <input
              checked={persistente}
              onChange={(event) => setPersistente(event.currentTarget.checked)}
              type="checkbox"
            />
            <span>
              Nao me deixe esquecer
              <em>continua cobrando ate voce resolver</em>
            </span>
          </label>

          {details ? (
            <>
              <label>
                <span>NOTA</span>
                <textarea
                  onChange={(event) => setBody(event.currentTarget.value)}
                  rows={3}
                  value={body}
                />
              </label>

              <fieldset className="composer-when">
                <legend className="micro-label">REPETE</legend>
                <div className="composer-choices">
                  {REPETICOES.map((opcao) => {
                    const ativa = JSON.stringify(repeticao) === JSON.stringify(opcao.rule);
                    return (
                      <Button
                        aria-pressed={ativa}
                        disabled={semData}
                        key={opcao.label}
                        onClick={() => setRepeticao(opcao.rule)}
                        size="sm"
                        variant={ativa ? "primary" : "ghost"}
                      >
                        {opcao.label}
                      </Button>
                    );
                  })}
                </div>
              </fieldset>

              {/* Os alertas extras. Nenhum marcado por default: o §17 do pedido
                  e explicito em que o M/OS pode SUGERIR um alerta a mais, e nao
                  criar varios sozinho. */}
              <fieldset className="composer-when">
                <legend className="micro-label">AVISAR TAMBEM</legend>
                <div className="composer-choices">
                  {ADIANTAMENTOS.map((opcao) => {
                    const ativa = adiantamentos.includes(opcao.minutes);
                    return (
                      <Button
                        aria-pressed={ativa}
                        disabled={semData}
                        key={opcao.label}
                        onClick={() =>
                          setAdiantamentos((atuais) =>
                            atuais.includes(opcao.minutes)
                              ? atuais.filter((valor) => valor !== opcao.minutes)
                              : [...atuais, opcao.minutes],
                          )
                        }
                        size="sm"
                        variant={ativa ? "primary" : "ghost"}
                      >
                        {opcao.label}
                      </Button>
                    );
                  })}
                </div>
                {adiantamentos.length > 0 && !semData ? (
                  <p className="composer-resolved">
                    {adiantamentos.length + 1} alertas, um lembrete so.
                  </p>
                ) : null}
              </fieldset>
            </>
          ) : (
            <Button onClick={() => setDetails(true)} size="sm" variant="ghost">
              Mais opcoes
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
