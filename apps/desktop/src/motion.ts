/**
 * M/OS — Motion System Foundation
 *
 * Princípios:
 * 1. Movimento como informação e continuidade espacial, nunca decoração solta.
 * 2. Tempos controlados: 75ms–260ms (máximo 400ms para abertura inicial).
 * 3. Física de springs controlada para microinterações táteis e confiáveis.
 * 4. Paridade e segurança total com `prefers-reduced-motion`.
 */

export const MOTION_DURATIONS = {
  instant: 0.075, // 75ms - hover, press
  micro: 0.12,    // 120ms - microinterações
  state: 0.14,    // 140ms - check, select, toggle
  enter: 0.18,    // 180ms - overlay/drawer entra
  exit: 0.09,     // 90ms - overlay/drawer sai
  move: 0.20,     // 200ms - layout/reorder
  context: 0.26,  // 260ms - troca de workspace/página
  slow: 0.40,     // 400ms - boot inicial / first paint
} as const;

export type MotionDurationKey = keyof typeof MOTION_DURATIONS;

export const MOTION_EASINGS = {
  // Curvas de Bezier puras
  standard: [0.2, 0, 0, 1] as const,
  enter: [0.16, 1, 0.3, 1] as const,
  exit: [0.4, 0, 1, 1] as const,

  // Curvas de mola (Spring Physics) para microinterações táteis e interfaces nativas
  tactileSpring: { type: "spring", stiffness: 600, damping: 35 } as const,
  defaultSpring: { type: "spring", stiffness: 450, damping: 32 } as const,
  gentleSpring: { type: "spring", stiffness: 300, damping: 28 } as const,
  snappySpring: { type: "spring", stiffness: 520, damping: 36 } as const,
} as const;

/**
 * Transições pré-configuradas para o Framer Motion e componentes M/OS
 */
export const MOTION_TRANSITIONS = {
  instant: {
    duration: MOTION_DURATIONS.instant,
    ease: MOTION_EASINGS.standard,
  },
  state: {
    duration: MOTION_DURATIONS.state,
    ease: MOTION_EASINGS.standard,
  },
  enter: {
    duration: MOTION_DURATIONS.enter,
    ease: MOTION_EASINGS.enter,
  },
  exit: {
    duration: MOTION_DURATIONS.exit,
    ease: MOTION_EASINGS.exit,
  },
  context: {
    duration: MOTION_DURATIONS.context,
    ease: MOTION_EASINGS.enter,
  },
  tactile: MOTION_EASINGS.tactileSpring,
  spring: MOTION_EASINGS.defaultSpring,
  gentle: MOTION_EASINGS.gentleSpring,
} as const;

/**
 * Variantes de animação estruturais (Framer Motion)
 */
export const MOTION_VARIANTS = {
  // Fade sutil
  fadeIn: {
    initial: { opacity: 0 },
    animate: { opacity: 1 },
    exit: { opacity: 0 },
  },

  // Entrada de conteúdo / painéis com leve subida
  slideUp: {
    initial: { opacity: 0, y: 6 },
    animate: { opacity: 1, y: 0 },
    exit: { opacity: 0, y: -4 },
  },

  // Entrada com escala sutil (Command, Dialogs, Popovers)
  scaleFade: {
    initial: { opacity: 0, scale: 0.98, y: -4 },
    animate: { opacity: 1, scale: 1, y: 0 },
    exit: { opacity: 0, scale: 0.98, y: -2 },
  },

  // Gaveta lateral / Inspector / TaskDrawer
  drawerRight: {
    initial: { opacity: 0, x: 20 },
    animate: { opacity: 1, x: 0 },
    exit: { opacity: 0, x: 16 },
  },

  // Toast / Notificações de atenção
  toastSlide: {
    initial: { opacity: 0, y: 16, scale: 0.96 },
    animate: { opacity: 1, y: 0, scale: 1 },
    exit: { opacity: 0, y: 8, scale: 0.96 },
  },

  // Linhas de lista e itens de dados
  listItem: {
    initial: { opacity: 0, y: -3 },
    animate: { opacity: 1, y: 0 },
    exit: { opacity: 0, scale: 0.98, transition: { duration: MOTION_DURATIONS.exit } },
  },
} as const;

/**
 * Calcula passos de decriptação para o efeito DecryptedText (React Bits adaptado).
 * Mantido como função pura para testabilidade e precisão sem layout shifts.
 */
export function generateDecryptedStep(
  targetText: string,
  progressRatio: number,
  glyphSet: string = "01_/*#[]<>:;="
): string {
  if (progressRatio <= 0) {
    return targetText
      .split("")
      .map((char) => (char === " " ? " " : glyphSet[Math.floor(Math.random() * glyphSet.length)]))
      .join("");
  }
  if (progressRatio >= 1) return targetText;

  const totalLength = targetText.length;
  const revealedCount = Math.floor(totalLength * progressRatio);

  return targetText
    .split("")
    .map((char, index) => {
      if (char === " " || char === "\n") return char;
      if (index < revealedCount) return char;
      return glyphSet[(index + Math.floor(progressRatio * 10)) % glyphSet.length];
    })
    .join("");
}

/**
 * Easing para interpolação numérica suave (AnimatedNumber / CountUp).
 */
export function easeOutQuart(t: number): number {
  return 1 - Math.pow(1 - t, 4);
}

/**
 * Interpola um número de `from` a `to` com base no tempo atual e duração.
 */
export function interpolateNumber(from: number, to: number, progress: number): number {
  const clamped = Math.max(0, Math.min(1, progress));
  const eased = easeOutQuart(clamped);
  return from + (to - from) * eased;
}

/**
 * Retorna variantes modificadas para respeitar Reduced Motion quando ativo.
 */
export function safeVariants<T extends Record<string, { initial?: Record<string, unknown>; animate?: Record<string, unknown>; exit?: Record<string, unknown> }>>(
  variants: T,
  reduced: boolean
): T {
  if (!reduced) return variants;
  const result: Record<string, unknown> = {};
  for (const [key, val] of Object.entries(variants)) {
    result[key] = {
      initial: { opacity: val.initial?.opacity ?? 0 },
      animate: { opacity: val.animate?.opacity ?? 1 },
      exit: { opacity: val.exit?.opacity ?? 0 },
    };
  }
  return result as T;
}
