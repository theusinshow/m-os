import type { Capture } from "../api";
import { Vazio } from "../componentes/Vazio";

/** "há 3 min", "ontem". O relógio exato não ajuda a decidir nada aqui. */
export function quando(iso: string): string {
  const momento = new Date(iso).getTime();
  if (Number.isNaN(momento)) return "";
  const minutos = Math.round((Date.now() - momento) / 60_000);
  if (minutos < 1) return "agora";
  if (minutos < 60) return `há ${minutos} min`;
  const horas = Math.round(minutos / 60);
  if (horas < 24) return `há ${horas} h`;
  const dias = Math.round(horas / 24);
  return dias === 1 ? "ontem" : `há ${dias} dias`;
}

export function Inbox({
  capturas,
  aoCapturar,
}: {
  capturas: Capture[];
  aoCapturar: () => void;
}) {
  if (capturas.length === 0) {
    return (
      <Vazio
        frase="Nada esperando. O que você capturar aparece aqui."
        acao={{ rotulo: "Capturar agora", aoTocar: aoCapturar }}
      />
    );
  }
  return (
    <ul className="lista">
      {capturas.map((capture) => (
        <li className="item" key={capture.id}>
          <div className="item-corpo">
            <p>{capture.content}</p>
            <small>{quando(capture.capturedAt)}</small>
          </div>
        </li>
      ))}
    </ul>
  );
}
