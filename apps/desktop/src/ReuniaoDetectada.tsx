import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { Button } from "./Button";
import { copyDoGuardian } from "./reuniao";
import type { GuardianView } from "./types";

type Detectada = { processo: string; nome: string };
type Terminou = { view: GuardianView; durationMs?: number };

/**
 * A janelinha de sobreposição das reuniões. Dois papéis, nunca ao mesmo tempo:
 *
 * 1. **a oferta** — um microfone abriu; gravar? (ADR-047). Ela só existe quando
 *    NADA está gravando;
 * 2. **a pergunta do Recording Guardian** — a reunião parece ter terminado;
 *    encerrar? Ela só existe quando ALGO está gravando.
 *
 * Por isso compartilham a janela sem disputar espaço. As duas **não roubam o
 * foco** e **não dizem "IA"**: o que se inicia é uma gravação, e o que se
 * encerra também.
 */
export function ReuniaoDetectada() {
  const [alvo, setAlvo] = useState<Detectada | null>(null);
  const [terminou, setTerminou] = useState<Terminou | null>(null);
  const [erro, setErro] = useState("");
  const [ocupado, setOcupado] = useState(false);

  useEffect(() => {
    const offs = [
      listen<Detectada>("reuniao-detectada", (evento) => {
        setTerminou(null);
        setAlvo(evento.payload);
        setErro("");
      }),
      listen<Terminou>("reuniao-terminou", (evento) => {
        if (evento.payload.view.kind === "idle") {
          setTerminou(null);
          return;
        }
        setAlvo(null);
        setTerminou(evento.payload);
        setErro("");
      }),
    ];
    return () => { offs.forEach((off) => void off.then((fn) => fn())); };
  }, []);

  async function agir(run: () => Promise<unknown>) {
    setOcupado(true);
    try {
      await run();
      await api.fecharReuniaoDetectada();
      setAlvo(null);
      setTerminou(null);
    } catch (causa) {
      // O erro fica AQUI. Mandar procurar o motivo no M/OS desfaz o motivo de a
      // janelinha existir.
      setErro(causa instanceof Error ? causa.message : String(causa));
    } finally {
      setOcupado(false);
    }
  }

  if (terminou) {
    const copy = copyDoGuardian(terminou.view, terminou.durationMs ?? 0);
    if (!copy) return null;
    return (
      <main className="oferta-shell" data-modo="terminou">
        <header className="oferta-head">
          <span className="micro-label">M/OS · REUNIÃO</span>
          <strong>{copy.titulo}</strong>
          <span className="support-copy">{copy.corpo}</span>
        </header>
        {erro ? <p className="support-copy" role="alert">{erro}</p> : null}
        <div className="oferta-acoes">
          <Button
            variant="primary"
            size="sm"
            disabled={ocupado}
            onClick={() => void agir(() => (copy.cortarEm != null ? api.meetingStopAndTrim(copy.cortarEm) : api.meetingStop()))}
          >
            {copy.encerrar}
          </Button>
          <Button variant="ghost" size="sm" disabled={ocupado} onClick={() => void agir(() => api.meetingGuardianContinue())}>
            {copy.continuar}
          </Button>
        </div>
      </main>
    );
  }

  if (!alvo) return null;

  return (
    <main className="oferta-shell">
      <header className="oferta-head">
        <span className="micro-label">M/OS · REUNIÕES</span>
        <strong>Parece que você entrou em uma reunião</strong>
        <span className="support-copy">{alvo.nome} abriu o microfone.</span>
      </header>

      {erro ? <p className="support-copy" role="alert">{erro}</p> : null}

      <div className="oferta-acoes">
        <Button
          variant="primary"
          size="sm"
          disabled={ocupado}
          onClick={() => void agir(async () => {
            // A primeira gravação da vida passa pela tela de consentimento, que
            // mora na janela principal.
            const consentimento = await api.meetingAnalysisConsent();
            if (!consentimento.granted) {
              throw new Error("Abra Reuniões no M/OS uma vez para autorizar a primeira gravação.");
            }
            await api.meetingStart("", null, "detected", alvo.processo);
          })}
        >
          Gravar
        </Button>
        <Button variant="ghost" size="sm" disabled={ocupado} onClick={() => void agir(async () => undefined)}>
          Agora não
        </Button>
        <Button
          variant="ghost"
          size="sm"
          disabled={ocupado}
          onClick={() => void agir(() => api.silenciarDeteccao(alvo.processo))}
        >
          Não neste app
        </Button>
      </div>
    </main>
  );
}
