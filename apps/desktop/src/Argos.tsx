import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, appError } from "./api";
import { BODY, type ArgosPose, type ArgosSignals, eyesFor, poseFor, rotuloPara, weightFor } from "./argosPose";
import type { ArgosCanto } from "./argosCorner";
import { criarCena, type ArgosScene } from "./argosScene";
import { hermes, type HermesConnectionState } from "./hermes";
import { type ArgosPresenca, corDaPresenca, presencaDe, rotuloDaPresenca } from "./argosPresenca";
import { deveEsperarAbertura, esperaDaTentativa } from "./abertura";

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
 * A presença do Hermes, para quem só precisa saber se ele está lá.
 *
 * Assina o `hermes-state` e pergunta uma vez na montagem — sem a pergunta, uma
 * janela aberta depois da conexão ficaria cinza até o próximo evento, que pode
 * não vir nunca.
 */
export function useArgosPresenca(): ArgosPresenca {
  const [state, setState] = useState<HermesConnectionState | null>(null);

  useEffect(() => {
    let vivo = true;

    /* A primeira pergunta cai na MESMA corrida de abertura que o boot: o portao
       recusa o comando enquanto o `setup` nao terminou. Engolir essa recusa
       deixava a presenca presa em `null` — que se le como "conectando" — para
       sempre, e o bicho ficava eternamente no meio-termo. Ver `abertura.ts`. */
    void (async () => {
      for (let tentativa = 0; vivo; tentativa += 1) {
        try {
          const status = await hermes.status();
          if (vivo) setState(status.state);
          return;
        } catch (error) {
          if (!deveEsperarAbertura(appError(error), tentativa)) return;
          await new Promise((resolve) => window.setTimeout(resolve, esperaDaTentativa(tentativa)));
        }
      }
    })();

    const subscription = hermes.onState((status) => setState(status.state));
    return () => { vivo = false; void subscription.then((dispose) => dispose()); };
  }, []);

  return presencaDe(state);
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
  presenca,
  canto,
  onAbrir,
  onAbrirHermes,
}: {
  pose: ArgosPose;
  presenca: ArgosPresenca;
  canto: ArgosCanto;
  onAbrir: () => void;
  onAbrirHermes: () => void;
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
     fonte, e a troca de tema tem de alcançar o bicho.

     E ela diz PRESENÇA, e não peso. O peso mudou de canal — foi para o
     movimento, no `data-peso` do botão. A razão está em `argosPresenca.ts`. */
  useEffect(() => {
    const aplicar = () => {
      const estilo = getComputedStyle(document.documentElement);
      const corpo = corDaPresenca(presenca);
      cenaRef.current?.setCores(estilo.getPropertyValue(corpo).trim(), estilo.getPropertyValue("--canvas").trim());
    };
    aplicar();
    const observador = new MutationObserver(aplicar);
    observador.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    return () => observador.disconnect();
  }, [presenca]);

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
    <div className="argos-canto" data-canto={canto}>
      {presenca === "desconectado" ? <BalaoDesconectado abrir={onAbrirHermes} /> : null}
      <button
        className="argos-botao"
        data-canto={canto}
        data-peso={weightFor(pose)}
        data-presenca={presenca}
        onClick={onAbrir}
        aria-label={`${rotuloPara(pose)}. ${rotuloDaPresenca(presenca)}.`}
      >
        {semWebGL
          ? <ArgosSvg pose={pose} />
          : <canvas ref={canvasRef} className="argos-canvas" width={CORPO} height={CORPO} aria-hidden="true" />}
      </button>
    </div>
  );
}

/**
 * O aviso de que o Hermes não está lá.
 *
 * **Fica enquanto durar a queda**, e não some sozinho: um balão que pisca e some
 * é indistinguível de um que nunca apareceu, e a queda continua depois dele. Mas
 * ele tem X, porque quem já sabe não precisa ser lembrado a cada olhada — e o X
 * vale só para esta queda: a próxima, ou a próxima abertura do app, traz o balão
 * de volta.
 *
 * Clicar leva ao Hermes, que é onde a queda se resolve. Um aviso que não oferece
 * o caminho é só uma reclamação.
 */
function BalaoDesconectado({ abrir }: { abrir: () => void }) {
  const [dispensado, setDispensado] = useState(false);
  if (dispensado) return null;

  return (
    <div className="argos-balao" role="status">
      <button type="button" className="argos-balao-corpo" onClick={abrir}>
        <strong>Hermes desconectado</strong>
        <span>A análise de reuniões e o chat não respondem enquanto isso. Clique para abrir.</span>
      </button>
      <button
        type="button"
        className="argos-balao-fechar"
        onClick={() => setDispensado(true)}
        aria-label="Dispensar o aviso até a próxima queda"
      >
        ×
      </button>
    </div>
  );
}
