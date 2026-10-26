import { useCallback, useEffect, useRef, useState } from "react";
import { LazyMotion, m } from "framer-motion";
import { api, appError } from "./api";
import { Button } from "./Button";
import { Icon } from "./Icon";
import { MOTION_DURATIONS, MOTION_EASINGS } from "./motion";
import {
  acaoPrincipal,
  perguntaDoCartao,
  recurrenceLabel,
  snoozeChoices,
  triggerLabel,
  whenLabel,
} from "./lembretes";
import type { Reminder, ReminderEvent, ReminderTrigger } from "./types";

const loadMotionFeatures = () => import("./motionFeatures").then((module) => module.default);

/**
 * A folha de um lembrete: tudo o que ele é, e o que dá para fazer com ele.
 *
 * **Não é formulário** (§42 do pedido). O que se lê aqui é uma ficha — o quê,
 * quando, ligado a quê, com que insistência —, e o que se faz são quatro botões.
 * Editar acontece em cima do que já está na tela, campo a campo, e só quando
 * alguém pede.
 *
 * O histórico fica no rodapé, discreto. Ele responde a perguntas que só
 * aparecem quando algo deu errado — "por que isso apareceu?", "eu adiei isso
 * quantas vezes?" —, e uma resposta que ocupa o topo da tela para uma pergunta
 * que se faz uma vez por mês é uma resposta no lugar errado.
 */
export function ReminderSheet({
  reminder,
  close,
  changed,
  openTarget,
}: {
  reminder: Reminder;
  close: () => void;
  /** Chamado depois de qualquer mudança, para a lista atrás se atualizar. */
  changed: () => void;
  /** Abrir a entidade a que o lembrete se prende. */
  openTarget?: (target: { type: string; id: string }) => void;
}) {
  const [atual, setAtual] = useState(reminder);
  const [pilha, setPilha] = useState<ReminderTrigger[]>([]);
  const [historico, setHistorico] = useState<ReminderEvent[]>([]);
  const [erro, setErro] = useState("");
  const [ocupado, setOcupado] = useState(false);
  const [editando, setEditando] = useState(false);
  const [titulo, setTitulo] = useState(reminder.title);
  const [nota, setNota] = useState(reminder.body);
  const [quando, setQuando] = useState<string>(paraCampoLocal(reminder.nextDueAt));
  const [salvo, setSalvo] = useState(false);
  const painel = useRef<HTMLDivElement>(null);
  const campoTitulo = useRef<HTMLInputElement>(null);

  /* Entrar na edicao sempre parte do que esta salvo. Sem isto, cancelar uma
     edicao e reabrir trazia de volta o rascunho abandonado. */
  function comecarEdicao() {
    setTitulo(atual.title);
    setNota(atual.body);
    setQuando(paraCampoLocal(atual.nextDueAt));
    setErro("");
    setSalvo(false);
    setEditando(true);
  }

  function cancelarEdicao() {
    setEditando(false);
    setErro("");
  }

  useEffect(() => {
    if (editando) campoTitulo.current?.focus();
  }, [editando]);

  const recarregar = useCallback(async () => {
    const [alertas, eventos] = await Promise.all([
      api.reminderTriggers(atual.id).catch(() => [] as ReminderTrigger[]),
      api.reminderHistory(atual.id, 20).catch(() => [] as ReminderEvent[]),
    ]);
    setPilha(alertas);
    setHistorico(eventos);
  }, [atual.id]);

  useEffect(() => {
    void recarregar();
  }, [recarregar]);

  useEffect(() => {
    painel.current?.focus();
  }, []);

  useEffect(() => {
    function aoTeclar(evento: globalThis.KeyboardEvent) {
      if (evento.key === "Escape") {
        evento.preventDefault();
        // Esc desfaz o passo mais recente: editando, sai da edicao; senao, fecha.
        if (editando) cancelarEdicao();
        else close();
      }
    }
    window.addEventListener("keydown", aoTeclar);
    return () => window.removeEventListener("keydown", aoTeclar);
  }, [close, editando]);

  async function agir(fn: () => Promise<Reminder | void>): Promise<boolean> {
    setOcupado(true);
    setErro("");
    setSalvo(false);
    try {
      const resultado = await fn();
      if (resultado) setAtual(resultado);
      await recarregar();
      changed();
      return true;
    } catch (falha) {
      setErro(appError(falha).message);
      return false;
    } finally {
      setOcupado(false);
    }
  }

  /* O que mudou de fato. Remarcar sem a hora ter mudado devolvia o lembrete a
     `scheduled`, zerava o adiamento e escrevia "Remarcado" no historico por um
     clique em Salvar que so corrigiu uma virgula na nota. */
  const quandoOriginal = paraCampoLocal(atual.nextDueAt);
  const quandoMudou = quando !== quandoOriginal && quando !== "";
  const instanteNovo = quandoMudou ? new Date(quando) : null;
  const quandoNoPassado =
    instanteNovo !== null && !Number.isNaN(instanteNovo.getTime()) && instanteNovo.getTime() < Date.now();
  const tituloVazio = !titulo.trim();

  async function salvar(evento: React.FormEvent) {
    evento.preventDefault();
    if (ocupado || tituloVazio || quandoNoPassado) return;
    const ok = await agir(async () => {
      // Remarcar antes de renomear: mudar a hora devolve o lembrete a
      // `scheduled`, e o título viaja no mesmo objeto.
      let remarcado: Reminder | undefined;
      if (instanteNovo && !Number.isNaN(instanteNovo.getTime())) {
        remarcado = await api.rescheduleReminder(atual.id, instanteNovo);
      }
      if (titulo.trim() !== atual.title || nota !== atual.body) {
        return api.editReminder(atual.id, { title: titulo.trim(), body: nota });
      }
      return remarcado;
    });
    if (ok) {
      setEditando(false);
      setSalvo(true);
    }
  }

  const pergunta = perguntaDoCartao(atual);
  const repeticao = recurrenceLabel(atual);
  const agora = Date.now();
  const adiamentos = snoozeChoices(new Date());
  const pendentes = pilha.filter(
    (alerta) => alerta.status === "pending" && alerta.lifecycleState === "active",
  );

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
        aria-label={atual.title}
        className="reminder-sheet"
        ref={painel}
        role="dialog"
        tabIndex={-1}
        initial={{ opacity: 0, scale: 0.98, y: -6 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.98, y: -4 }}
        transition={{ duration: MOTION_DURATIONS.enter, ease: MOTION_EASINGS.enter }}
      >
        <header className="reminder-sheet-head">
          <div>
            <span className="micro-label">{editando ? "EDITAR LEMBRETE" : "LEMBRETE"}</span>
            <h2>{atual.title}</h2>
            <p className="attention-when">{whenLabel(atual, agora)}</p>
          </div>
          <button aria-label="Fechar" className="icon-button" onClick={close} type="button">
            <Icon name="close" />
          </button>
        </header>

        {pergunta ? <p className="reminder-sheet-question">{pergunta}</p> : null}

        {editando ? (
          <form className="stack-form reminder-sheet-edit" onSubmit={salvar}>
            <label>
              <span>LEMBRAR DE</span>
              <input
                aria-describedby={tituloVazio ? "sheet-titulo-erro" : undefined}
                aria-invalid={tituloVazio ? true : undefined}
                className="reminder-sheet-title-input"
                onChange={(evento) => setTitulo(evento.currentTarget.value)}
                ref={campoTitulo}
                value={titulo}
              />
              {tituloVazio ? (
                <small className="field-error" id="sheet-titulo-erro">
                  O lembrete precisa de um título.
                </small>
              ) : null}
            </label>
            <label>
              <span>NOTA (OPCIONAL)</span>
              <textarea
                onChange={(evento) => setNota(evento.currentTarget.value)}
                rows={3}
                value={nota}
              />
            </label>
            <label>
              <span>QUANDO</span>
              <input
                aria-describedby="sheet-quando-dica"
                aria-invalid={quandoNoPassado ? true : undefined}
                min={paraCampoLocal(new Date().toISOString())}
                onChange={(evento) => setQuando(evento.currentTarget.value)}
                type="datetime-local"
                value={quando}
              />
              {quandoNoPassado ? (
                <small className="field-error" id="sheet-quando-dica">
                  Esse horário já passou. Escolha um horário futuro.
                </small>
              ) : (
                <small className="composer-resolved" id="sheet-quando-dica">
                  {quandoMudou
                    ? "Vai ser remarcado ao salvar."
                    : atual.nextDueAt
                      ? "Sem mudança, o horário atual continua valendo."
                      : "Sem data. Escolha um horário para ser avisado."}
                </small>
              )}
            </label>
            {erro ? (
              <p className="inline-error" role="alert">
                ! {erro} Suas alterações continuam nos campos.
              </p>
            ) : null}
            <div className="form-actions">
              <Button disabled={ocupado} onClick={cancelarEdicao} variant="ghost">
                Descartar alterações
              </Button>
              <Button
                aria-busy={ocupado || undefined}
                disabled={ocupado || tituloVazio || quandoNoPassado}
                type="submit"
                variant="primary"
              >
                {ocupado ? "Salvando…" : "Salvar alterações"}
              </Button>
            </div>
          </form>
        ) : (
          <dl className="reminder-sheet-facts">
            {atual.body ? (
              <div>
                <dt>Nota</dt>
                <dd>{atual.body}</dd>
              </div>
            ) : null}
            {atual.target ? (
              <div>
                <dt>Ligado a</dt>
                <dd>
                  <Button
                    onClick={() => openTarget?.({ type: atual.target!.type, id: atual.target!.id })}
                    size="sm"
                    variant="ghost"
                  >
                    Abrir {rotuloDeAlvo(atual.target.type)}
                  </Button>
                </dd>
              </div>
            ) : null}
            <div>
              <dt>Prioridade</dt>
              <dd>{PRIORIDADE[atual.priority]}</dd>
            </div>
            <div>
              <dt>Insistência</dt>
              <dd>
                {atual.persistent
                  ? "Continua cobrando até você resolver"
                  : "Avisa uma vez"}
              </dd>
            </div>
            {repeticao ? (
              <div>
                <dt>Repete</dt>
                <dd>{repeticao}</dd>
              </div>
            ) : null}
            {atual.snoozeCount > 0 ? (
              <div>
                <dt>Adiado</dt>
                <dd>{atual.snoozeCount === 1 ? "1 vez" : `${atual.snoozeCount} vezes`}</dd>
              </div>
            ) : null}
          </dl>
        )}

        {/* A pilha. Um lembrete com quatro alertas é UM lembrete — o que se
            mostra aqui é a lista dos avisos dele, e não quatro cartões. */}
        {pilha.length > 0 ? (
          <section className="reminder-sheet-stack">
            <span className="micro-label">
              {pendentes.length === 1 ? "1 ALERTA" : `${pendentes.length} ALERTAS`}
            </span>
            <ul>
              {pilha.map((alerta) => (
                <li key={alerta.id} data-status={alerta.status}>
                  <span>{triggerLabel(alerta)}</span>
                  <span className="attention-when">{horaCurta(alerta.scheduledAt)}</span>
                  {alerta.status === "pending" ? (
                    <Button
                      aria-label={`Remover alerta ${triggerLabel(alerta)}`}
                      disabled={ocupado}
                      onClick={() =>
                        void agir(() => api.cancelReminderTrigger(atual.id, alerta.id))
                      }
                      size="sm"
                      variant="ghost"
                    >
                      Remover
                    </Button>
                  ) : (
                    <span className="attention-when">{ESTADO_DO_ALERTA[alerta.status]}</span>
                  )}
                </li>
              ))}
            </ul>
          </section>
        ) : null}

        {erro && !editando ? (
          <p className="inline-error" role="alert">
            ! {erro}
          </p>
        ) : null}

        <p aria-live="polite" className="reminder-sheet-saved">
          {salvo && !editando ? "Alterações salvas." : ""}
        </p>

        {/* Editando, a barra de acoes some: "Descartar" e "Cancelar lembrete"
            lado a lado eram dois cancelar com efeitos opostos, um deles
            destrutivo. */}
        {editando ? null : (
        <div className="reminder-sheet-actions">
          <Button
            disabled={ocupado}
            onClick={() =>
              // So fecha se deu certo: fechar no erro escondia a mensagem.
              void agir(() => api.completeReminder(atual.id)).then((ok) => {
                if (ok) close();
              })
            }
            variant="primary"
          >
            {acaoPrincipal(atual)}
          </Button>
          {atual.policy.snoozeAllowed
            ? adiamentos.slice(0, 3).map((escolha) => (
                <Button
                  disabled={ocupado}
                  key={escolha.label}
                  onClick={() => void agir(() => api.snoozeReminder(atual.id, escolha.resolve()))}
                  variant="ghost"
                >
                  {escolha.label}
                </Button>
              ))
            : null}
          <Button disabled={ocupado} onClick={comecarEdicao} variant="ghost">
            Editar
          </Button>
          <Button
            disabled={ocupado}
            onClick={() =>
              void agir(() =>
                api.novoLembrete({
                  title: atual.title,
                  at: null,
                }),
              )
            }
            variant="ghost"
          >
            {/* Duplicar para "algum dia" é o caminho de tirar da agenda sem
                perder a intenção. Cancelar apagaria a intenção junto. */}
            Mandar para Algum dia
          </Button>
          <Button
            disabled={ocupado}
            onClick={() =>
              void agir(() => api.cancelReminder(atual.id)).then((ok) => {
                if (ok) close();
              })
            }
            variant="danger"
          >
            Cancelar lembrete
          </Button>
        </div>
        )}

        {historico.length > 0 ? (
          <section className="reminder-sheet-history">
            <span className="micro-label">HISTÓRICO</span>
            <ul>
              {historico.map((evento) => (
                <li key={evento.id}>
                  <span className="attention-when">{horaCurta(evento.at)}</span>
                  <span>{EVENTO[evento.kind]}</span>
                </li>
              ))}
            </ul>
          </section>
        ) : null}
      </m.div>
    </LazyMotion>
  );
}

const PRIORIDADE: Record<Reminder["priority"], string> = {
  low: "Baixa",
  normal: "Normal",
  high: "Alta",
  urgent: "Urgente",
};

const ESTADO_DO_ALERTA: Record<ReminderTrigger["status"], string> = {
  pending: "pendente",
  fired: "tocou",
  skipped: "pulado",
  cancelled: "removido",
};

/** As frases do histórico. Sem jargão de máquina de estados. */
const EVENTO: Record<ReminderEvent["kind"], string> = {
  created: "Criado",
  triggered: "Venceu",
  delivered: "Notificou",
  acknowledged: "Você viu",
  snoozed: "Adiado",
  rescheduled: "Remarcado",
  completed: "Concluído",
  cancelled: "Cancelado",
  missed: "Perdido",
  escalated: "Insistiu",
  recurrence_generated: "Próxima ocorrência",
  edited: "Editado",
};

function rotuloDeAlvo(tipo: string): string {
  const nomes: Record<string, string> = {
    task: "a Task",
    project: "o Project",
    capture: "a Capture",
    resource: "a referência",
    conversation: "a conversa",
    app: "o app",
    meeting: "a reunião",
  };
  return nomes[tipo] ?? "o item";
}

function horaCurta(iso: string): string {
  const quando = new Date(iso);
  if (Number.isNaN(quando.getTime())) return "";
  return quando.toLocaleString("pt-BR", {
    day: "2-digit",
    month: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/** `datetime-local` fala no fuso do usuário e não aceita sufixo de zona. */
function paraCampoLocal(iso: string | null): string {
  if (!iso) return "";
  const quando = new Date(iso);
  if (Number.isNaN(quando.getTime())) return "";
  const deslocado = new Date(quando.getTime() - quando.getTimezoneOffset() * 60000);
  return deslocado.toISOString().slice(0, 16);
}
