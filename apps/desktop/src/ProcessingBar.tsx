import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { rotuloDoProcessamento, type Processamento } from "./processamento";

/**
 * A barra do que o M/OS está fazendo com uma reunião.
 *
 * **Irmã da `RecordingBar`, e no mesmo lugar dela: o shell.** Uma para gravar,
 * outra para processar. A razão de morar no shell é a mesma registrada lá:
 * navegar para outra página não pode apagar da vista que existe trabalho em
 * curso — e transcrever uma reunião de uma hora leva minutos em que a pessoa vai
 * fazer outra coisa.
 *
 * **Ela não some quando falha.** Barra que desaparece é indistinguível de barra
 * que terminou, e "terminou" seria mentira. A falha vira a mensagem, e a mensagem
 * fica até a pessoa fechá-la.
 */
export function ProcessingBar({ abrirReuniao }: { abrirReuniao: (id: string) => void }) {
  const [estado, setEstado] = useState<Processamento | null>(null);

  const fechar = useCallback(() => setEstado(null), []);

  useEffect(() => {
    const assinaturas = [
      listen<{ meetingId: string; progress: number; channel: string }>("meeting-transcribing", (evento) =>
        setEstado({
          tipo: "transcrevendo",
          meetingId: evento.payload.meetingId,
          canal: evento.payload.channel === "mic" ? "mic" : "system",
          progress: evento.payload.progress,
        }),
      ),
      listen<{ meetingId: string; window: number; windows: number }>("meeting-analyzing", (evento) =>
        setEstado({
          tipo: "analisando",
          meetingId: evento.payload.meetingId,
          window: evento.payload.window,
          windows: evento.payload.windows,
        }),
      ),
      /* Transcrever termina e a barra sai — mas só se o que terminou for o que
         ela mostra. Sem essa conferência, uma reunião antiga terminando de
         analisar apagaria a barra de outra que acabou de começar. */
      listen<{ id?: string }>("meeting-transcribed", () => setEstado((atual) =>
        atual?.tipo === "transcrevendo" ? null : atual,
      )),
      listen("meeting-analyzed", () => setEstado((atual) =>
        atual?.tipo === "analisando" ? null : atual,
      )),
      listen<string>("meeting-failed", (evento) =>
        setEstado({
          tipo: "falhou",
          meetingId: evento.payload,
          detalhe: "A reunião está segura. Abra para ver o que houve e tentar de novo.",
        }),
      ),
    ];
    return () => { assinaturas.forEach((a) => void a.then((dispose) => dispose())); };
  }, []);

  if (!estado) return null;

  const rotulo = rotuloDoProcessamento(estado);
  const porcento = rotulo.fracao === null ? null : Math.round(rotulo.fracao * 100);

  return (
    <div className="processing-bar" role="status" aria-live="polite" data-erro={rotulo.erro || undefined}>
      <button
        type="button"
        className="processing-corpo"
        onClick={() => abrirReuniao(estado.meetingId)}
        aria-label={`${rotulo.titulo}: ${rotulo.detalhe}. Abrir a reunião.`}
      >
        <span className="processing-titulo">{rotulo.titulo}</span>
        <span className="micro-label">{rotulo.detalhe}</span>
        {/* A trilha só existe quando há fração. Indeterminado ganha pulso, e não
            uma barra parada em algum lugar que sugira progresso que ninguém mediu. */}
        <span className="processing-trilha" data-indeterminado={porcento === null || undefined}>
          <span className="processing-preenchimento" style={porcento === null ? undefined : { width: `${porcento}%` }} />
        </span>
      </button>
      {porcento === null ? null : <span className="processing-porcento">{porcento}%</span>}
      {rotulo.erro ? (
        <button type="button" className="processing-fechar" onClick={fechar} aria-label="Fechar o aviso">
          ×
        </button>
      ) : null}
    </div>
  );
}
