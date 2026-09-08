import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { LazyMotion, m } from "framer-motion";
import { api } from "./api";
import { Button } from "./Button";
import { Icon } from "./Icon";
import { AnimatedList, AnimatedListItem } from "./motion/AnimatedList";
import { MOTION_DURATIONS, MOTION_EASINGS } from "./motion";
import { ReminderSheet } from "./ReminderSheet";
import {
  acaoPrincipal,
  alertasPendentes,
  BUCKET_LABEL,
  bucketOf,
  perguntaDoCartao,
  recurrenceLabel,
  REASON_LABEL,
  snoozeChoices,
  whenLabel,
  type Bucket,
} from "./lembretes";
import type { AttentionReason, AttentionRow, Reminder, ReminderTrigger } from "./types";

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
 * **A regra de "quem está sendo esquecido" NÃO mora aqui.** Ela vem pronta do
 * domínio, por `needsAttention`, com o motivo de cada item ao lado. A tela
 * agrupa e desenha; ela não decide. Uma segunda implementação da mesma regra
 * divergiria da primeira, e aí a lista e o badge diriam números diferentes
 * sobre a mesma coisa.
 *
 * Mora no rodapé do rail e não como destino, por decisão registrada: a ADR-031
 * fixa que "Quick Capture e Settings continuam fora da contagem: eles não são
 * destinos de conteúdo, e o rodapé do rail é uma zona própria".
 */

/** A ordem em que os grupos se leem. Atenção primeiro, sempre. */
const ORDEM: readonly Bucket[] = ["attention", "today", "upcoming", "snoozed", "someday"];

export function AttentionCenter({ close, compose, openTarget }: {
  close: () => void;
  compose: () => void;
  openTarget?: (target: { type: string; id: string }) => void;
}) {
  const [reminders, setReminders] = useState<Reminder[]>([]);
  const [atencao, setAtencao] = useState<AttentionRow[]>([]);
  const [pilhas, setPilhas] = useState<Record<string, ReminderTrigger[]>>({});
  const [resolvidos, setResolvidos] = useState<Reminder[]>([]);
  const [verResolvidos, setVerResolvidos] = useState(false);
  const [aberto, setAberto] = useState<Reminder | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const panel = useRef<HTMLDivElement>(null);

  const refresh = useCallback(async () => {
    try {
      const [lista, motivos] = await Promise.all([api.reminders(), api.needsAttention()]);
      setReminders(lista);
      setAtencao(motivos);
      setError("");

      // As pilhas, só de quem tem uma. Uma chamada por lembrete seria N
      // chamadas para desenhar um número que quase sempre é um — e quase sempre
      // é um porque a pilha é o caso raro, não o normal.
      const comAlvo = lista.filter((item) => item.trigger.kind === "at");
      const pilhasLidas = await Promise.all(
        comAlvo.map(async (item) => [item.id, await api.reminderTriggers(item.id)] as const),
      );
      setPilhas(Object.fromEntries(pilhasLidas.filter(([, alertas]) => alertas.length > 1)));
    } catch (err) {
      setError((err as Error).message ?? "Falha ao carregar lembretes");
    }
  }, []);

  useEffect(() => {
    void refresh();
    // O relógio da tela anda sozinho: "atrasado 5 min" que continua dizendo 5
    // vinte minutos depois é uma tela que mente devagar.
    const timer = window.setInterval(() => setNow(Date.now()), 15000);
    // E a LISTA também: o agendador vive no backend, e um lembrete que vence
    // com o painel aberto tem de aparecer nele. Sem isto, a única tela que
    // existe para não deixar nada passar era a última a saber que algo passou.
    const aviso = listen("attention-delivered", () => void refresh());
    const contagem = listen("attention-count", () => void refresh());
    return () => {
      window.clearInterval(timer);
      void aviso.then((parar) => parar());
      void contagem.then((parar) => parar());
    };
  }, [refresh]);

  useEffect(() => {
    panel.current?.focus();
    function onKey(event: globalThis.KeyboardEvent) {
      if (event.key === "Escape" && !aberto) {
        event.preventDefault();
        close();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [close, aberto]);

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

  const motivosPorId = useMemo(() => {
    const mapa = new Map<string, AttentionReason[]>();
    for (const linha of atencao) mapa.set(linha.reminder.id, linha.reasons);
    return mapa;
  }, [atencao]);

  const precisam = useMemo(() => new Set(motivosPorId.keys()), [motivosPorId]);

  const grupos = useMemo(() => {
    const vazio: Record<Bucket, Reminder[]> = {
      attention: [],
      today: [],
      upcoming: [],
      snoozed: [],
      someday: [],
    };
    for (const reminder of reminders) {
      vazio[bucketOf(reminder, precisam, now)].push(reminder);
    }
    // Dentro de "precisa de atenção", a ordem é a do domínio: o mais pesado
    // primeiro. Nos outros grupos, o relógio manda.
    const peso = new Map(atencao.map((linha, indice) => [linha.reminder.id, indice]));
    vazio.attention.sort((a, b) => (peso.get(a.id) ?? 0) - (peso.get(b.id) ?? 0));
    return vazio;
  }, [reminders, precisam, atencao, now]);

  const adiamentos = useMemo(() => snoozeChoices(new Date(now)), [now]);
  const vazio = reminders.length === 0;

  async function mostrarResolvidos() {
    setVerResolvidos(true);
    setResolvidos(await api.resolvedReminders(20).catch(() => []));
  }

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

        {vazio ? (
          <div className="attention-empty">
            <p>Nada esperando por você.</p>
            <Button onClick={compose} size="sm" variant="secondary">
              Criar um lembrete
            </Button>
          </div>
        ) : (
          ORDEM.map((bucket) =>
            grupos[bucket].length ? (
              <section className="attention-group" key={bucket}>
                <span className="micro-label">
                  {BUCKET_LABEL[bucket]} · {grupos[bucket].length}
                </span>
                <AnimatedList className="attention-list">
                  {grupos[bucket].map((reminder) => (
                    <AnimatedListItem
                      className="attention-item"
                      key={reminder.id}
                      itemKey={reminder.id}
                    >
                      <Cartao
                        reminder={reminder}
                        motivos={motivosPorId.get(reminder.id) ?? []}
                        alertas={pilhas[reminder.id] ?? []}
                        now={now}
                        ocupado={busy === reminder.id}
                        adiamentos={adiamentos}
                        abrir={() => setAberto(reminder)}
                        agir={(fn) => void act(reminder.id, fn)}
                      />
                    </AnimatedListItem>
                  ))}
                </AnimatedList>
              </section>
            ) : null,
          )
        )}

        {/* O histórico fica atrás de um clique. Uma lista de lembretes que
            mostra o que já foi resolvido junto com o que falta responde as duas
            perguntas ao mesmo tempo e nenhuma bem. */}
        <section className="attention-group">
          {verResolvidos ? (
            <>
              <span className="micro-label">Resolvidos · {resolvidos.length}</span>
              <ul className="attention-list attention-list--quiet">
                {resolvidos.map((reminder) => (
                  <li key={reminder.id}>
                    <span>{reminder.title}</span>
                    <span className="attention-when">{ESTADO_FINAL[reminder.status] ?? ""}</span>
                  </li>
                ))}
              </ul>
            </>
          ) : (
            <Button onClick={() => void mostrarResolvidos()} size="sm" variant="ghost">
              Ver resolvidos
            </Button>
          )}
        </section>
      </m.div>

      {aberto ? (
        <ReminderSheet
          changed={() => void refresh()}
          close={() => setAberto(null)}
          key={aberto.id}
          openTarget={openTarget}
          reminder={aberto}
        />
      ) : null}
    </LazyMotion>
  );
}

const ESTADO_FINAL: Partial<Record<Reminder["status"], string>> = {
  completed: "concluído",
  cancelled: "cancelado",
  expired: "expirado",
};

/**
 * Um lembrete na lista.
 *
 * As duas ações que se fazem dez vezes por dia — concluir e adiar — são um
 * clique cada. Todo o resto mora na folha, atrás de "Abrir". É o §43: primárias
 * em um clique, secundárias a um clique de distância.
 */
function Cartao({
  reminder,
  motivos,
  alertas,
  now,
  ocupado,
  adiamentos,
  abrir,
  agir,
}: {
  reminder: Reminder;
  motivos: AttentionReason[];
  alertas: ReminderTrigger[];
  now: number;
  ocupado: boolean;
  adiamentos: ReturnType<typeof snoozeChoices>;
  abrir: () => void;
  agir: (fn: () => Promise<unknown>) => void;
}) {
  const pergunta = perguntaDoCartao(reminder);
  const repeticao = recurrenceLabel(reminder);
  const pendentes = alertasPendentes(alertas);
  const fadiga = reminder.snoozeCount >= 5;

  return (
    <>
      <div className="attention-body">
        <strong>{reminder.title}</strong>
        <span className="attention-when">{whenLabel(reminder, now)}</span>
        {pergunta ? <p className="attention-question">{pergunta}</p> : null}
        {reminder.body ? <p>{reminder.body}</p> : null}

        {/* Os selos. Nunca só cor: `DESIGN-FOUNDATIONS.md` §14 pede que nenhum
            estado dependa apenas dela, então cada um carrega a palavra. */}
        <div className="attention-marks">
          {motivos.map((motivo) => (
            <span className="attention-mark" data-reason={motivo} key={motivo}>
              {REASON_LABEL[motivo]}
            </span>
          ))}
          {reminder.persistent && !motivos.includes("persistent") ? (
            <span className="attention-mark" data-reason="persistent">
              não deixar esquecer
            </span>
          ) : null}
          {pendentes > 1 ? (
            <span className="attention-mark" data-reason="stack">
              {pendentes} alertas
            </span>
          ) : null}
          {repeticao ? (
            <span className="attention-mark" data-reason="recurring">
              {repeticao}
            </span>
          ) : null}
        </div>
      </div>

      <div className="attention-actions">
        <Button disabled={ocupado} onClick={() => agir(() => api.completeReminder(reminder.id))} variant="secondary">
          {acaoPrincipal(reminder)}
        </Button>
        {/* DOIS adiamentos no cartao, e nao os seis.
            Com tres, a linha de acoes quebrava e "Abrir" caia sozinho embaixo —
            cada lembrete virava um bloco de noventa pixels, e uma lista de dez
            deixava de caber na tela. Os outros quatro continuam na folha, a um
            clique, que e onde se escolhe uma data com calma. */}
        {reminder.policy.snoozeAllowed && !fadiga
          ? adiamentos.slice(0, 2).map((escolha) => (
              <Button
                disabled={ocupado}
                key={escolha.label}
                onClick={() => agir(() => api.snoozeReminder(reminder.id, escolha.resolve()))}
                variant="ghost"
              >
                {escolha.label}
              </Button>
            ))
          : null}
        <Button disabled={ocupado} onClick={abrir} variant="ghost">
          Abrir
        </Button>
      </div>

      {/* Depois do quinto adiamento, o sistema para de oferecer só "adiar".
          Continuar oferecendo é cumplicidade com uma lista que não anda. */}
      {fadiga ? (
        <div className="attention-fatigue">
          <span>Você já adiou isto {reminder.snoozeCount} vezes.</span>
          <Button
            disabled={ocupado}
            onClick={() => agir(() => api.snoozeReminder(reminder.id, umaHora()))}
            size="sm"
            variant="ghost"
          >
            Fazer hoje
          </Button>
          <Button disabled={ocupado} onClick={abrir} size="sm" variant="ghost">
            Escolher data
          </Button>
          <Button
            disabled={ocupado}
            onClick={() => agir(() => api.cancelReminder(reminder.id))}
            size="sm"
            variant="ghost"
          >
            Cancelar
          </Button>
        </div>
      ) : null}
    </>
  );
}

function umaHora(): Date {
  return new Date(Date.now() + 60 * 60000);
}
