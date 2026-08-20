import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { BODY, type ArgosPose, type ArgosSignals, eyesFor, poseFor, rotuloPara, weightFor } from "./argosPose";
import type { ArgosCanto } from "./argosCorner";
import { criarCena, type ArgosScene } from "./argosScene";
import { hermes } from "./hermes";

/**
 * Os sinais, vindos de onde eles já vivem.
 *
 * O estado de streaming e de aprovação **não é levantado do `HermesPage`**:
 * `hermes.onEvent()` já entrega `TurnEvent` no barramento global para quem
 * assinar. E o cronômetro ganha uma assinatura própria e leve — `useTrackedTime`
 * carrega TODAS as entradas de tempo, e Argos só precisa saber se ele corre.
 *
 * **Argos só escuta.** Nunca chama `hermes.approve` nem qualquer método que
 * escreva: a ADR-024 fixou que Hermes é superfície, não segundo agente, e o
 * próprio `hermes.ts` registra o que acontece quando duas superfícies disputam o
 * mesmo barramento.
 */
export function useArgosPose({ busy, boot }: { busy: boolean; boot: "loading" | "ready" | "error" }): ArgosPose {
  const [hermesState, setHermesState] = useState<ArgosSignals["hermes"]>("idle");
  const [timerRunning, setTimerRunning] = useState(false);

  useEffect(() => {
    const subscription = hermes.onEvent((event) => {
      switch (event.outcome) {
        case "delta":
        case "reasoning":
          return setHermesState("working");
        case "tool":
          return setHermesState(event.running ? "working" : "idle");
        case "approval":
        case "clarify":
          return setHermesState("waiting");
        case "failed":
          return setHermesState("failed");
        case "complete":
        case "sudo_refused":
          return setHermesState("idle");
        default:
          return undefined;
      }
    });
    return () => { void subscription.then((dispose) => dispose()); };
  }, []);

  useEffect(() => {
    const read = () => { void api.timerCurrent().then((timer) => setTimerRunning(timer?.status === "running")).catch(() => setTimerRunning(false)); };
    read();
    const subscription = listen("timer-changed", read);
    return () => { void subscription.then((dispose) => dispose()); };
  }, []);

  return poseFor({ hermes: hermesState, busy, boot, timerRunning });
}

/**
 * O piso.
 *
 * Este era o Argos inteiro até a ADR-048. Ele não foi apagado porque o M/OS roda
 * em WebView2 sobre máquinas que nem sempre têm WebGL — driver velho, VM, sessão
 * remota. Um retângulo preto no canto seria pior que não ter bicho.
 *
 * `aria-hidden` continua: quem fala agora é o botão que o envolve.
 */
function ArgosSvg({ pose }: { pose: ArgosPose }) {
  const { left, right } = eyesFor(pose);

  return (
    <svg
      className="argos"
      data-pose={pose}
      data-weight={weightFor(pose)}
      width={BODY.size}
      height={BODY.size}
      viewBox={`0 0 ${BODY.size} ${BODY.size}`}
      aria-hidden="true"
      focusable="false"
    >
      <rect
        className="argos-body"
        x={BODY.inset}
        y={BODY.inset}
        width={BODY.size - BODY.inset * 2}
        height={BODY.size - BODY.inset * 2}
        rx={BODY.radius}
      />
      {[left, right].map((eye, index) => (
        <ellipse
          className="argos-eye"
          key={index}
          cx={eye.x}
          cy={eye.y}
          rx={eye.rx}
          ry={eye.ry}
          transform={eye.tilt ? `rotate(${eye.tilt} ${eye.x} ${eye.y})` : undefined}
        />
      ))}
    </svg>
  );
}

/** Onde o corpo desenha, em px. A ADR-048 fixou 72. */
const CORPO = 72;

/**
 * Argos, a face do estado (ADR-041, revisada pela ADR-048).
 *
 * A casca não desenha: ela monta o canvas, empurra pose e ponteiro para a cena,
 * e a pausa quando ninguém está olhando. Quem decide a pose é `useArgosPose`;
 * quem decide o canto é `argosCorner.ts`; quem desenha é `argosScene.ts`.
 */
export function Argos({
  pose,
  canto,
  onAbrir,
}: {
  pose: ArgosPose;
  canto: ArgosCanto;
  onAbrir: () => void;
}) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const cenaRef = useRef<ArgosScene | null>(null);
  const [semWebGL, setSemWebGL] = useState(false);
  const oculto = canto === "oculto";

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || oculto) return;
    let vivo = true;
    const reduzido = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

    void criarCena(canvas, reduzido)
      .then((cena) => {
        // `vivo` cobre o desmonte durante o `await` do import dinâmico: sem ele,
        // uma cena órfã continuaria segurando o contexto WebGL.
        if (!vivo || !cena) { cena?.dispose(); if (!cena) setSemWebGL(true); return; }
        cenaRef.current = cena;
        cena.setPose(pose);
        cena.resume();
      })
      .catch(() => setSemWebGL(true));

    return () => { vivo = false; cenaRef.current?.dispose(); cenaRef.current = null; };
  }, [oculto]);

  useEffect(() => { cenaRef.current?.setPose(pose); }, [pose]);

  /* A cor vem do token, e não de um literal: o design system continua sendo a
     fonte, e a troca de tema tem de alcançar o bicho. */
  useEffect(() => {
    const aplicar = () => {
      const estilo = getComputedStyle(document.documentElement);
      const peso = weightFor(pose);
      const corpo = peso === "chamando" ? "--signal-ink" : peso === "atento" ? "--text" : "--text-system";
      cenaRef.current?.setCores(estilo.getPropertyValue(corpo).trim(), estilo.getPropertyValue("--canvas").trim());
    };
    aplicar();
    const observador = new MutationObserver(aplicar);
    observador.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    return () => observador.disconnect();
  }, [pose]);

  /* O olhar. Coalescido por rAF porque `mousemove` dispara muito mais que o
     quadro, e empurrar uniform a cada evento seria trabalho jogado fora. */
  useEffect(() => {
    if (oculto) return;
    let pendente = 0;
    const mover = (evento: MouseEvent) => {
      if (pendente) return;
      pendente = requestAnimationFrame(() => {
        pendente = 0;
        const x = (evento.clientX / window.innerWidth) * 2 - 1;
        const y = (evento.clientY / window.innerHeight) * 2 - 1;
        cenaRef.current?.setPointer(x, y);
      });
    };
    window.addEventListener("mousemove", mover);
    return () => { window.removeEventListener("mousemove", mover); cancelAnimationFrame(pendente); };
  }, [oculto]);

  /* A conta de bateria da ADR-048: janela escondida não desenha. */
  useEffect(() => {
    const acompanhar = () => {
      if (document.hidden || !document.hasFocus()) cenaRef.current?.pause();
      else cenaRef.current?.resume();
    };
    document.addEventListener("visibilitychange", acompanhar);
    window.addEventListener("focus", acompanhar);
    window.addEventListener("blur", acompanhar);
    return () => {
      document.removeEventListener("visibilitychange", acompanhar);
      window.removeEventListener("focus", acompanhar);
      window.removeEventListener("blur", acompanhar);
    };
  }, []);

  if (oculto) return null;

  return (
    <button className="argos-botao" data-canto={canto} onClick={onAbrir} aria-label={rotuloPara(pose)}>
      {semWebGL
        ? <ArgosSvg pose={pose} />
        : <canvas ref={canvasRef} className="argos-canvas" width={CORPO} height={CORPO} aria-hidden="true" />}
    </button>
  );
}
