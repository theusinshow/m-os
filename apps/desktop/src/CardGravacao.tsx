import type { CSSProperties } from "react";
import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { Button } from "./Button";
import { BARRAS, DEGRAUS, alturaDaBarra, degrausAcesos, empurrar } from "./ondaSonora";
import type { Meeting, MeetingLevel } from "./types";

/**
 * O card da reunião em curso.
 *
 * Só existe em `recording` e `paused` — é o posto de trabalho da reunião, e não
 * um indicador. O indicador continua na topbar, porque a §17.2 promete que ele
 * apareça em QUALQUER tela, e esta é uma só.
 */
export function CardGravacao({ meeting, onMudou }: {
  meeting: Meeting;
  onMudou: (meeting: Meeting) => void;
}) {
  const [janela, setJanela] = useState<number[]>([]);
  const [notas, setNotas] = useState(meeting.notes);
  const [erro, setErro] = useState("");
  const pausada = meeting.status === "paused";
  const gravado = useRef(meeting.notes);

  /* Este é o PRIMEIRO leitor de `prefers-reduced-motion` em JS neste repo — os
     outros sete vivem no CSS. Aqui não dá: "parar de rolar" é decisão de DADO e
     não de estilo, porque uma onda que se redesenha quinze vezes por segundo é
     movimento por mais que nenhuma transição exista. */
  const [semMovimento] = useState(
    () => window.matchMedia("(prefers-reduced-motion: reduce)").matches,
  );

  // A onda ouve o evento de 15 Hz. Pausada, ela para de ouvir: o nível já zera
  // no backend durante a pausa, e a janela congela no que havia — silêncio é
  // "ninguém falou", pausado é "não estou ouvindo", e a onda não pode dizer a
  // mesma coisa nos dois casos.
  useEffect(() => {
    if (pausada) return;
    const off = listen<MeetingLevel>("meeting-level", (evento) => {
      setJanela((atual) => empurrar(atual, Math.max(evento.payload.mic, evento.payload.system)));
    });
    return () => { void off.then((fn) => fn()); };
  }, [pausada]);

  // Autosave com debounce. `gravado` guarda o que já foi ao banco para não
  // reenviar o mesmo texto quando a tela recarrega por outro motivo.
  useEffect(() => {
    if (notas === gravado.current) return;
    const timer = setTimeout(() => {
      api.meetingSetNotes(meeting.id, notas)
        .then((atualizada) => { gravado.current = notas; onMudou(atualizada); })
        .catch((causa) => setErro(causa instanceof Error ? causa.message : String(causa)));
    }, 800);
    return () => clearTimeout(timer);
  }, [notas, meeting.id, onMudou]);

  const alternarPausa = useCallback(async () => {
    setErro("");
    try {
      onMudou(pausada ? await api.meetingResume() : await api.meetingPause());
    } catch (causa) {
      setErro(causa instanceof Error ? causa.message : String(causa));
    }
  }, [pausada, onMudou]);

  const agora = janela[janela.length - 1] ?? 0;

  return (
    <section className="card-gravacao" data-pausada={pausada || undefined}>
      <div className="card-gravacao-barra">
        {semMovimento ? (
          <span className="onda" data-degraus="" aria-hidden="true">
            {Array.from({ length: DEGRAUS }, (_, i) => (
              <i key={i} data-on={i < degrausAcesos(agora) || undefined} />
            ))}
          </span>
        ) : (
          <span className="onda" aria-hidden="true">
            {Array.from({ length: BARRAS }, (_, i) => (
              <i key={i} style={{ "--h": String(alturaDaBarra(janela[i] ?? 0)) } as CSSProperties} />
            ))}
          </span>
        )}
        {/* O nível é decorativo para quem lê por leitor de tela: o estado que
            importa é gravando ou pausado, e ele é dito em palavras. */}
        <span className="visually-hidden" aria-live="polite">
          {pausada ? "Gravação pausada" : "Gravando"}
        </span>
        <Button variant="outline" size="sm" onClick={() => void alternarPausa()}>
          {pausada ? "Retomar" : "Pausar"}
        </Button>
      </div>

      <label className="visually-hidden" htmlFor="notas-da-reuniao">Anotações</label>
      <textarea
        id="notas-da-reuniao"
        className="card-gravacao-notas"
        value={notas}
        placeholder="O que você escrever aqui sobe junto com a transcrição."
        onChange={(evento) => setNotas(evento.currentTarget.value)}
      />

      {erro ? <p className="support-copy" role="alert">{erro}</p> : null}
    </section>
  );
}
