import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { prontaDe, rotuloDoProcessamento, type Processamento } from "./processamento";
import type { MeetingOverview, MeetingProgressEvent } from "./types";

/**
 * A barra do que o M/OS está fazendo com uma reunião — sem ninguém ter pedido.
 *
 * **Irmã da `RecordingBar`, e no mesmo lugar dela: o shell.** Uma para gravar,
 * outra para organizar. Navegar para outra página não pode apagar da vista que
 * existe trabalho em curso — e transcrever uma hora leva minutos em que a
 * pessoa vai fazer outra coisa.
 *
 * Três destinos, e nenhum some sozinho de forma enganosa:
 *
 * - **organizando** some quando termina, e dá lugar a **pronta**;
 * - **pronta** fica até a pessoa abrir ou fechar — é o "Reunião pronta · 3
 *   ações · Revisar" que substitui o clique em Transcrever e Analisar;
 * - **falhou** fica até a pessoa fechar. Barra que desaparece é indistinguível
 *   de barra que terminou, e "terminou" seria mentira.
 */
export function ProcessingBar({ abrirReuniao }: { abrirReuniao: (id: string) => void }) {
  const [estado, setEstado] = useState<Processamento | null>(null);

  const fechar = useCallback(() => setEstado(null), []);

  useEffect(() => {
    let esperando: number | undefined;
    const assinaturas = [
      listen<MeetingProgressEvent>("meeting-progress", (evento) => {
        window.clearTimeout(esperando);
        setEstado({ tipo: "progresso", meetingId: evento.payload.meetingId, evento: evento.payload });
      }),
      listen<MeetingOverview>("meeting-ready", (evento) => {
        window.clearTimeout(esperando);
        setEstado(prontaDe(evento.payload));
      }),
      /* Espera não é falha: a barra avisa por alguns segundos e sai. O M/OS
         tenta de novo sozinho, e deixar um aviso parado ensinaria a pessoa a
         achar que precisa fazer algo. */
      listen<string>("meeting-waiting", (evento) => {
        setEstado({ tipo: "aguardando", meetingId: evento.payload });
        window.clearTimeout(esperando);
        esperando = window.setTimeout(
          () => setEstado((atual) => (atual?.tipo === "aguardando" ? null : atual)),
          8000,
        );
      }),
      listen<string>("meeting-failed", (evento) => {
        window.clearTimeout(esperando);
        setEstado({
          tipo: "falhou",
          meetingId: evento.payload,
          detalhe: "A gravação está segura. Abra para ver o que houve.",
        });
      }),
      listen<string>("meeting-trashed", (evento) =>
        setEstado((atual) => (atual?.meetingId === evento.payload ? null : atual)),
      ),
    ];
    return () => {
      window.clearTimeout(esperando);
      assinaturas.forEach((a) => void a.then((dispose) => dispose()));
    };
  }, []);

  if (!estado) return null;

  const rotulo = rotuloDoProcessamento(estado);
  const porcento = rotulo.fracao === null ? null : Math.round(rotulo.fracao * 100);
  const abrir = () => {
    abrirReuniao(estado.meetingId);
    if (estado.tipo !== "progresso") setEstado(null);
  };

  return (
    <div
      className="processing-bar"
      role="status"
      aria-live="polite"
      data-erro={rotulo.erro || undefined}
      data-pronta={estado.tipo === "pronta" || undefined}
    >
      <button
        type="button"
        className="processing-corpo"
        onClick={abrir}
        aria-label={`${rotulo.titulo}: ${rotulo.detalhe}. Abrir a reunião.`}
      >
        <span className="processing-titulo">{rotulo.titulo}</span>
        <span className="micro-label">{rotulo.detalhe}</span>
        {porcento === null ? null : (
          <span className="processing-trilha">
            <span className="processing-preenchimento" style={{ width: `${porcento}%` }} />
          </span>
        )}
      </button>
      {porcento === null ? null : <span className="processing-porcento">{porcento}%</span>}
      {rotulo.acao ? (
        <button type="button" className="processing-acao" onClick={abrir}>{rotulo.acao}</button>
      ) : null}
      {estado.tipo !== "progresso" ? (
        <button type="button" className="processing-fechar" onClick={fechar} aria-label="Fechar o aviso">×</button>
      ) : null}
    </div>
  );
}
