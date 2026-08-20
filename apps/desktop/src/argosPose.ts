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
    // O desvio é grande de propósito — a 24px um deslocamento sutil é o mesmo
    // que nenhum, e `trabalhando` viraria `desperto`.
    case "trabalhando": {
      const eyes = pair(3.2, 11.4, 1.9, 2.1);
      return { left: { ...eyes.left, x: eyes.left.x + 2.8 }, right: { ...eyes.right, x: eyes.right.x + 2.8 } };
    }
    // Os maiores de todos, redondos e fixos no centro: esperando você responder.
    case "encarando":
      return pair(3.6, 11, 2.4, 4.2);
    // Semicerrado: uma fresta, não uma linha — é o que o separa de `fechado`.
    case "concentrado":
      return pair(3.5, 11.6, 2, 1.8);
    // Duas linhas, e mais LARGAS que as do semicerrado: a diferença entre os
    // dois precisa sobreviver a 24px, e altura sozinha não sobrevive.
    case "fechado":
      return pair(3.5, 12, 2.7, 0.5);
    // Frestas verticais, afastadas e tortas. Não pode ser "arregalado" como o
    // `encarando`: os dois são sódio, e se também tivessem a mesma forma seriam
    // a mesma pose — "preciso de resposta" e "quebrou" leriam igual.
    case "assustado":
      return pair(4.2, 10.9, 1.3, 3.9, 16);
    case "desperto":
    default:
      return pair(3.5, 11, 2, 3.4);
  }
}

/**
 * O corpo de cada pose, em quatro numeros.
 *
 * `deformacao` e `velocidade` alimentam o ruido do vertex shader; `abertura`
 * escala o olho no eixo Y; `recuo` desloca o corpo no Z — negativo afasta.
 *
 * Sao quatro uniforms e nao seis malhas porque assim a troca de pose e uma
 * INTERPOLACAO, e nao um corte. O bicho vira o que ele passou a ser, em vez de
 * piscar para outro desenho.
 */
export type ArgosSceneParams = {
  deformacao: number;
  velocidade: number;
  abertura: number;
  recuo: number;
};

export function sceneParamsFor(pose: ArgosPose): ArgosSceneParams {
  switch (pose) {
    // Respiracao, e nada mais: e a pose que ocupa 90% do tempo, e e dela que
    // sai a conta de bateria da ADR-048.
    case "desperto":    return { deformacao: 0.06, velocidade: 0.25, abertura: 1,    recuo: 0 };
    case "concentrado": return { deformacao: 0.10, velocidade: 0.50, abertura: 0.45, recuo: 0 };
    case "trabalhando": return { deformacao: 0.18, velocidade: 1.20, abertura: 0.90, recuo: 0 };
    case "fechado":     return { deformacao: 0.04, velocidade: 0.20, abertura: 0,    recuo: -0.15 };
    // Encarar e ficar PARADO. Movimento aqui diluiria o unico caso em que o
    // sistema depende de voce agir.
    case "encarando":   return { deformacao: 0.02, velocidade: 0.10, abertura: 1.25, recuo: 0.20 };
    case "assustado":   return { deformacao: 0.30, velocidade: 2.40, abertura: 1.25, recuo: -0.30 };
  }
}

/**
 * O nome acessivel do botao.
 *
 * Saindo da topbar, Argos perdeu o texto ao lado que a ADR-041 usou para
 * justificar o `aria-hidden` — e virando controle, esconder deixou de ser
 * opcao. Entao ele fala por conta propria, e diz o FATO, nunca a expressao:
 * "aguardando sua aprovacao", e nao "arregalado".
 */
export function rotuloPara(pose: ArgosPose): string {
  switch (pose) {
    case "desperto":    return "Estado do sistema: em repouso";
    case "concentrado": return "Estado do sistema: Cronômetro correndo";
    case "trabalhando": return "Estado do sistema: Hermes trabalhando";
    case "fechado":     return "Estado do sistema: ocupado";
    case "encarando":   return "Estado do sistema: aguardando sua aprovação";
    case "assustado":   return "Estado do sistema: algo falhou";
  }
}
