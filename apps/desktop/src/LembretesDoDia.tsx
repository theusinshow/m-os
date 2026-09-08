import { useCallback, useEffect, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { REASON_LABEL, whenLabel } from "./lembretes";
import type { AttentionRow, Reminder } from "./types";

/**
 * Os lembretes pendentes, dentro do Começar e do Encerrar o Dia.
 *
 * # Por que ele existe nos dois lugares
 *
 * Porque o ciclo que o Attention System existe para quebrar é *dispara →
 * ignoro → some → esqueço*, e o dia é onde ele se fecha. Começar um dia sem ver
 * o que ficou do anterior é começar já devendo; encerrar sem decidir o que
 * sobrou é empurrar a dívida para amanhã sem olhar para ela.
 *
 * # Por que ele NÃO move nada sozinho
 *
 * O §23 do pedido é explícito: *não mover automaticamente tudo para amanhã. Eu
 * devo conscientemente decidir*. Um botão "adiar todos" existiria para poupar
 * cliques, e pouparia exatamente o clique que faz a pessoa olhar para o item. O
 * que este bloco oferece é uma decisão por linha — e "deixar como está" continua
 * sendo uma delas, tomada por não tocar em nada.
 *
 * # As duas modalidades
 *
 * No **começo** a pergunta é *o que preciso resolver hoje*, e a ação principal é
 * trazer para hoje. No **fim** a pergunta é *o que faço com o que sobrou*, e as
 * ações são para onde mandar. Mesmo dado, dois momentos, duas perguntas.
 */
export function LembretesDoDia({ momento }: { momento: "comeco" | "fim" }) {
  const [linhas, setLinhas] = useState<AttentionRow[]>([]);
  const [carregando, setCarregando] = useState(true);
  const [ocupado, setOcupado] = useState<string | null>(null);
  const [erro, setErro] = useState("");

  const recarregar = useCallback(async () => {
    try {
      setLinhas(await api.needsAttention());
      setErro("");
    } catch (falha) {
      // Falha aqui NÃO derruba o fluxo do dia: montar o dia sem a lista é
      // melhor que não poder começá-lo porque uma consulta não respondeu.
      setErro((falha as Error).message ?? "Não consegui ler os lembretes");
    } finally {
      setCarregando(false);
    }
  }, []);

  useEffect(() => {
    void recarregar();
  }, [recarregar]);

  async function agir(id: string, fn: () => Promise<unknown>) {
    setOcupado(id);
    try {
      await fn();
      await recarregar();
    } catch (falha) {
      setErro((falha as Error).message ?? "Ação falhou");
    } finally {
      setOcupado(null);
    }
  }

  if (carregando) return null;
  if (linhas.length === 0 && !erro) {
    // Silêncio quando não há nada. Um "0 pendentes" no meio do fluxo do dia é
    // uma linha que se lê todo dia para não descobrir nada.
    return null;
  }

  const agora = Date.now();

  return (
    <section aria-labelledby="lembretes-do-dia" className="daily-carry">
      <span className="micro-label" id="lembretes-do-dia">
        {momento === "comeco"
          ? `PENDENTE DE ANTES · ${linhas.length}`
          : `AINDA PENDENTE · ${linhas.length}`}
      </span>

      {erro ? (
        <p className="inline-error" role="alert">
          ! {erro}
        </p>
      ) : null}

      {linhas.map(({ reminder, reasons }) => (
        <div className="daily-lembrete" key={reminder.id}>
          <div className="attention-body">
            <strong>{reminder.title}</strong>
            <span className="attention-when">{whenLabel(reminder, agora)}</span>
            <div className="attention-marks">
              {reasons.map((motivo) => (
                <span className="attention-mark" data-reason={motivo} key={motivo}>
                  {REASON_LABEL[motivo]}
                </span>
              ))}
            </div>
          </div>
          <div className="attention-actions">
            <Button
              disabled={ocupado === reminder.id}
              onClick={() => void agir(reminder.id, () => api.completeReminder(reminder.id))}
              variant="secondary"
            >
              {reminder.kind === "follow_up" ? "Respondeu" : "Concluir"}
            </Button>
            {momento === "comeco" ? (
              <Button
                disabled={ocupado === reminder.id}
                onClick={() =>
                  void agir(reminder.id, () => api.snoozeReminder(reminder.id, maisTarde()))
                }
                variant="ghost"
              >
                Mais tarde hoje
              </Button>
            ) : (
              <Button
                disabled={ocupado === reminder.id}
                onClick={() =>
                  void agir(reminder.id, () => api.snoozeReminder(reminder.id, amanhaDeManha()))
                }
                variant="ghost"
              >
                Amanhã
              </Button>
            )}
            {/* "Algum dia" tira da agenda sem apagar a intenção. É a saída para
                o que importa e não tem hora — e sem ela, a única forma de parar
                de ser cobrado seria cancelar, que joga a intenção fora. */}
            <Button
              disabled={ocupado === reminder.id}
              onClick={() => void agir(reminder.id, () => mandarParaAlgumDia(reminder))}
              variant="ghost"
            >
              Algum dia
            </Button>
            <Button
              disabled={ocupado === reminder.id}
              onClick={() => void agir(reminder.id, () => api.cancelReminder(reminder.id))}
              variant="ghost"
            >
              Cancelar
            </Button>
          </div>
        </div>
      ))}
    </section>
  );
}

/**
 * Mandar para "algum dia": cria o lembrete sem data e cancela o que tinha hora.
 *
 * Duas escritas e não uma porque o domínio não tem transição para "tirar a
 * hora": mudar `Trigger::At` para `Trigger::Someday` num lembrete que já venceu
 * apagaria o instante original, que é o que sustenta o "atrasado há três dias"
 * — e é essa informação que faz alguém decidir se ainda quer aquilo.
 *
 * O que fica registrado é honesto: um cancelado com data, e um novo sem. O
 * histórico de cada um conta a sua parte.
 */
async function mandarParaAlgumDia(reminder: Reminder) {
  await api.novoLembrete({
    title: reminder.title,
    body: reminder.body,
    at: null,
    target: reminder.target ?? undefined,
    persistent: reminder.persistent,
  });
  await api.cancelReminder(reminder.id);
}

function maisTarde(): Date {
  return new Date(Date.now() + 3 * 60 * 60000);
}

function amanhaDeManha(): Date {
  const quando = new Date();
  quando.setDate(quando.getDate() + 1);
  quando.setHours(9, 0, 0, 0);
  return quando;
}
