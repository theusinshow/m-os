/**
 * Argos — a precedência de poses e a geometria dos olhos (ADR-041).
 *
 * Vive separado do desenho pelo mesmo motivo de `plotGeometry.ts`: o
 * `vitest.config.ts` roda só funções puras em ambiente de nó, e é aqui que mora
 * o que pode mentir. O `Argos.tsx` desenha o que este arquivo decide.
 *
 * A regra que organiza o repertório: **cada pose é um fato**. Onde não há sinal,
 * não há pose — é por isso que não existe "dormindo" (o sinal de inatividade
 * real mora no Rust, no monitor da ADR-037) nem ponto de notificação (o M/OS não
 * tem domínio de notificação).
 */

export type ArgosPose = "desperto" | "trabalhando" | "encarando" | "concentrado" | "fechado" | "assustado";

/** Três pesos de cor, e cada um significa uma coisa. Ver ADR-041. */
export type ArgosWeight = "repouso" | "atento" | "chamando";

/**
 * O que Argos consegue ver.
 *
 * **A conexão não entra.** Hermes nunca configurado é `offline`, que é um estado
 * perfeitamente normal — tratá-la como falha deixaria Argos permanentemente
 * aterrorizado em qualquer instalação sem gateway. Ela já é dita em texto pelo
 * SystemHealth, que é onde ela cabe.
 */
export type ArgosSignals = {
  hermes: "idle" | "working" | "waiting" | "failed";
  busy: boolean;
  boot: "loading" | "ready" | "error";
  timerRunning: boolean;
};

/** O corpo: quadrado de cantos macios, herdando a família do símbolo do rail. */
export const BODY = { size: 24, inset: 1, radius: 8 } as const;

export type Eye = { x: number; y: number; rx: number; ry: number; tilt: number };

/**
 * Quem precisa mais de você ganha.
 *
 * `encarando` vem antes de `assustado` de propósito: uma falha é ruim, mas uma
 * aprovação pendente está travada esperando VOCÊ. A ordem é a resposta à
 * pergunta "o que acontece se eu não olhar agora".
 */
export function poseFor(signals: ArgosSignals): ArgosPose {
  if (signals.hermes === "waiting") return "encarando";
  if (signals.boot === "error" || signals.hermes === "failed") return "assustado";
  if (signals.hermes === "working") return "trabalhando";
  if (signals.boot === "loading" || signals.busy) return "fechado";
  if (signals.timerRunning) return "concentrado";
  return "desperto";
}

/** As duas poses em que o sistema não continua sozinho puxam o olho. */
export function weightFor(pose: ArgosPose): ArgosWeight {
  if (pose === "encarando" || pose === "assustado") return "chamando";
  if (pose === "desperto") return "repouso";
  return "atento";
}

/**
 * A expressão inteira, em duas cápsulas.
 *
 * É a lição da referência: as vinte expressões dela são a mesma silhueta com
 * olhos diferentes. A 22px isso não é simplificação — é a única coisa que se lê.
 *
 * Coordenadas no viewBox de 24. O centro horizontal é 12.
 */
export function eyesFor(pose: ArgosPose): { left: Eye; right: Eye } {
  // `tilt ? -tilt : 0` e nao `-tilt`: negar zero devolve `-0`, que viraria um
  // literal `rotate(-0 ...)` no atributo do SVG.
  const pair = (dx: number, y: number, rx: number, ry: number, tilt = 0) => ({
    left: { x: 12 - dx, y, rx, ry, tilt: tilt ? -tilt : 0 },
    right: { x: 12 + dx, y, rx, ry, tilt },
  });

  switch (pose) {
    // Olhando pro lado, pálpebra baixa: ocupado com outra coisa, não com você.
    case "trabalhando": {
      const eyes = pair(3.5, 11.5, 1.9, 2.2);
      return { left: { ...eyes.left, x: eyes.left.x + 1.4 }, right: { ...eyes.right, x: eyes.right.x + 1.4 } };
    }
    // Arregalado e fixo no centro: está esperando você responder.
    case "encarando":
      return pair(3.6, 11, 2.3, 4.2);
    // Semicerrado: concentrado no próprio trabalho.
    case "concentrado":
      return pair(3.5, 11.6, 2, 1.4);
    // Duas linhas.
    case "fechado":
      return pair(3.5, 12, 2.1, 0.7);
    // Arregalado, afastado e torto — a única pose que inclina.
    case "assustado":
      return pair(4.1, 10.8, 2.2, 3.9, 14);
    case "desperto":
    default:
      return pair(3.5, 11, 2, 3.4);
  }
}
