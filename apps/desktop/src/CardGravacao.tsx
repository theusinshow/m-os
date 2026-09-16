import type { CSSProperties } from "react";
import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { Button } from "./Button";
import { BARRAS, DEGRAUS, alturaDaBarra, degrausAcesos, empurrar } from "./ondaSonora";
import { relogio } from "./reuniao";
import type { Meeting, MeetingLevel, MeetingTick } from "./types";

/**
 * O card da reunião em curso.
 *
 * ```
 * ● Gravando · 32:14
 *
 * Você       ███████░
 * Reunião    ██████░░
 *
 * [★ Marcar momento]
 * [Pausar]             [Encerrar]
 * ```
 *
 * Só existe em `recording` e `paused` — é o posto de trabalho da reunião, e não
 * um indicador. O indicador continua no shell, porque a §17.2 promete que ele
 * apareça em QUALQUER tela, e esta é uma só.
 *
 * Dois canais, e não uma onda só: "o outro lado parou de falar, ou eu perdi o
 * áudio?" tem resposta diferente para cada um, e a separação VOCÊ/REMOTO é o que
 * a gravação protege acima de tudo.
 */
function Onda({ janela, semMovimento }: { janela: number[]; semMovimento: boolean }) {
  const agora = janela[janela.length - 1] ?? 0;
  return semMovimento ? (
    <span className="onda" data-degraus="" aria-hidden="true">
      {Array.from({ length: DEGRAUS }, (_, i) => <i key={i} data-on={i < degrausAcesos(agora) || undefined} />)}
    </span>
  ) : (
    <span className="onda" aria-hidden="true">
      {Array.from({ length: BARRAS }, (_, i) => (
        <i key={i} style={{ "--h": String(alturaDaBarra(janela[i] ?? 0)) } as CSSProperties} />
      ))}
    </span>
  );
}

export function CardGravacao({ meeting, onMudou, onEncerrou }: {
  meeting: Meeting;
  onMudou: (meeting: Meeting) => void;
  onEncerrou: (meeting: Meeting) => void;
}) {
  const [voce, setVoce] = useState<number[]>([]);
  const [remoto, setRemoto] = useState<number[]>([]);
  const [duracaoMs, setDuracaoMs] = useState(meeting.durationMs);
  const [notas, setNotas] = useState(meeting.notes);
  const [marcas, setMarcas] = useState(0);
  const [erro, setErro] = useState("");
  const [ocupado, setOcupado] = useState(false);
  const pausada = meeting.status === "paused";
  const gravado = useRef(meeting.notes);

  /* "Parar de rolar" é decisão de DADO e não de estilo: uma onda que se redesenha
     quinze vezes por segundo é movimento por mais que nenhuma transição exista. */
  const [semMovimento] = useState(
    () => window.matchMedia("(prefers-reduced-motion: reduce)").matches,
  );

  useEffect(() => {
    void api.meetingBookmarks(meeting.id).then((lista) => setMarcas(lista.length)).catch(() => undefined);
    const offs = [
      listen<MeetingTick>("meeting-tick", (evento) => {
        if (evento.payload.meetingId === meeting.id) setDuracaoMs(evento.payload.durationMs);
      }),
      listen("meeting-bookmarked", () => setMarcas((n) => n + 1)),
    ];
    return () => { offs.forEach((off) => void off.then((fn) => fn())); };
  }, [meeting.id]);

  // Pausada, a onda para de ouvir: pausado é "não estou ouvindo", e a onda não
  // pode dizer a mesma coisa que silêncio.
  useEffect(() => {
    if (pausada) return;
    const off = listen<MeetingLevel>("meeting-level", (evento) => {
      setVoce((atual) => empurrar(atual, evento.payload.mic));
      setRemoto((atual) => empurrar(atual, evento.payload.system));
    });
    return () => { void off.then((fn) => fn()); };
  }, [pausada]);

  // Autosave com debounce. Sem botão de salvar: numa nota de reunião, um botão
  // de salvar é uma chance de perder o que se escreveu.
  useEffect(() => {
    if (notas === gravado.current) return;
    const timer = setTimeout(() => {
      api.meetingSetNotes(meeting.id, notas)
        .then((atualizada) => { gravado.current = notas; onMudou(atualizada); })
        .catch((causa) => setErro(causa instanceof Error ? causa.message : String(causa)));
    }, 800);
    return () => clearTimeout(timer);
  }, [notas, meeting.id, onMudou]);

  const agir = useCallback(async (acao: () => Promise<void>) => {
    setErro("");
    setOcupado(true);
    try { await acao(); } catch (causa) {
      setErro(causa instanceof Error ? causa.message : String(causa));
    } finally { setOcupado(false); }
  }, []);

  return (
    <section className="card-gravacao" data-pausada={pausada || undefined}>
      <header className="card-gravacao-topo">
        <span className="recording-dot" data-pausada={pausada || undefined} aria-hidden="true" />
        <strong>{pausada ? "Pausada" : "Gravando"} · <span className="recording-clock">{relogio(duracaoMs)}</span></strong>
        <span className="visually-hidden" aria-live="polite">{pausada ? "Gravação pausada" : "Gravando"}</span>
      </header>

      <div className="card-gravacao-canais">
        <span className="micro-label">VOCÊ</span>
        <Onda janela={voce} semMovimento={semMovimento} />
        <span className="micro-label">REUNIÃO</span>
        <Onda janela={remoto} semMovimento={semMovimento} />
      </div>

      <div className="card-gravacao-acoes">
        <Button
          variant="outline"
          size="sm"
          disabled={ocupado || pausada}
          onClick={() => void agir(async () => { await api.meetingMarkMoment(); })}
          title="Ctrl+Alt+M"
        >
          ★ Marcar momento{marcas ? ` · ${marcas}` : ""}
        </Button>
        <span className="card-gravacao-espaco" />
        <Button
          variant="ghost"
          size="sm"
          disabled={ocupado}
          onClick={() => void agir(async () => onMudou(pausada ? await api.meetingResume() : await api.meetingPause()))}
        >
          {pausada ? "Retomar" : "Pausar"}
        </Button>
        <Button
          variant="primary"
          size="sm"
          disabled={ocupado}
          onClick={() => void agir(async () => onEncerrou(await api.meetingStop()))}
        >
          Encerrar
        </Button>
      </div>

      <label className="visually-hidden" htmlFor="notas-da-reuniao">Anotações</label>
      <textarea
        id="notas-da-reuniao"
        className="card-gravacao-notas"
        value={notas}
        placeholder={"Anote se quiser. !task, !decision e !question viram itens; @pessoa e #projeto dão contexto."}
        onChange={(evento) => setNotas(evento.currentTarget.value)}
      />

      {erro ? <p className="support-copy" role="alert">{erro}</p> : null}
    </section>
  );
}
