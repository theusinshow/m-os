import type { ReactNode } from "react";

/**
 * Iconografia do M/OS.
 *
 * Tres regras vindas do design system, e nenhuma delas e preferencia:
 *
 * 1. **Cada tamanho e um desenho proprio.** Nunca escalar o SVG. Um desenho de
 *    20px renderizado a 16px poe as linhas em meio pixel e o icone borra. Por
 *    isso `size` so aceita 16 nos nomes que tem desenho de 16.
 * 2. **Stroke por tamanho:** 1.25 em 20px, 1 em 16px. Terminais retos.
 * 3. **`filled` significa uma coisa so: destino ativo na navegacao.** Nao e
 *    enfase, nao e hover, nao e selecao.
 *
 * Sobre a construcao das silhuetas: elas sao compostas de retangulos,
 * triangulos e circulos separados por vaos, e nunca de recortes com
 * `fill-rule`. Um vao de meio pixel le como detalhe; um furo em caminho
 * composto e onde icone desenhado sem poder olhar para a tela costuma quebrar.
 */

export type IconName =
  | "home"
  | "hermes"
  | "inbox"
  | "projects"
  | "workspaces"
  | "apps"
  | "library"
  | "board"
  | "tempo"
  | "settings"
  | "search"
  | "capture"
  | "plus"
  | "more"
  | "close"
  | "archive"
  | "trash";

/** Desenho de 20px, contorno. Coordenadas na grade de .5 para o traco de 1.25
 *  cair sobre o pixel em vez de entre dois. */
const OUTLINE_20: Record<IconName, ReactNode> = {
  home: <><path d="M3.5 9.5 10 4.25 16.5 9.5" /><path d="M5.5 8.75V16.5h9V8.75" /></>,
  // Balao com o canto inferior esquerdo puxado: fala, nao chat generico.
  hermes: <path d="M3.5 4.5h13v8h-8l-5 4z" />,
  inbox: <><path d="M3.5 5.5h13v10h-13z" /><path d="M3.5 11h3l1.5 2h4l1.5-2h3" /></>,
  projects: <><path d="M3.5 5.5h5l1.5 2h6.5v8.5h-13z" /><path d="M3.5 7.5h13" /></>,
  workspaces: <><rect x="3.5" y="5" width="8.5" height="8.5" /><rect x="8" y="3.5" width="8.5" height="8.5" /><path d="M7 16.5h8.5v-8.5" /></>,
  apps: <><rect x="3.5" y="3.5" width="5" height="5" /><rect x="11.5" y="3.5" width="5" height="5" /><rect x="3.5" y="11.5" width="5" height="5" /><rect x="11.5" y="11.5" width="5" height="5" /></>,
  library: <><path d="M4 3.5h4.5v13H4zM8.5 3.5H13v13H8.5z" /><path d="m13 4.5 3-.7 1.5 11.7-3 .7z" /></>,
  board: <><rect x="3.5" y="4" width="4" height="12" /><rect x="9" y="4" width="3" height="8" /><rect x="13.5" y="4" width="3" height="10" /></>,
  // Cronometro, e nao relogio de parede: a coroa em cima e o que separa "medir
  // uma duracao" de "ver que horas sao". Duas maos apenas — a terceira, de
  // segundos, some no traco de 1.25.
  tempo: <><circle cx="10" cy="11.5" r="5.5" /><path d="M8 3.5h4M10 4v2M10 11.5V8M10 11.5h2.5" /></>,
  // Coroa de oito dentes em vez de engrenagem desenhada: a 20px com traco de
  // 1.25 os dentes de uma engrenagem colam uns nos outros.
  settings: <><circle cx="10" cy="10" r="2.5" /><path d="M10 3.5v2M10 14.5v2M3.5 10h2M14.5 10h2M5.4 5.4l1.4 1.4M13.2 13.2l1.4 1.4M14.6 5.4l-1.4 1.4M6.8 13.2l-1.4 1.4" /></>,
  search: <><circle cx="8.5" cy="8.5" r="4.5" /><path d="m12 12 4 4" /></>,
  capture: <path d="M10 3.5v13M3.5 10h13" />,
  plus: <path d="M10 5v10M5 10h10" />,
  // Pontos preenchidos, sem traco. Como circulos de r=.7 tracados eles viravam
  // aneis de 2.6px com um furo no meio — nem ponto, nem circulo.
  more: <g fill="currentColor" stroke="none"><circle cx="4.75" cy="10" r="1.15" /><circle cx="10" cy="10" r="1.15" /><circle cx="15.25" cy="10" r="1.15" /></g>,
  close: <path d="m5 5 10 10M15 5 5 15" />,
  archive: <><path d="M3.5 7.5h13v9h-13z" /><path d="M2.5 3.5h15v4h-15z" /><path d="M8 11h4" /></>,
  trash: <><path d="M5.5 6.5h9l-.7 10h-7.6z" /><path d="M3.5 6.5h13" /><path d="M8 3.5h4l.6 3" /></>,
};

/**
 * Silhuetas de 20px. `filled` so existe para destino de navegacao.
 *
 * Todas montadas com retangulo, triangulo e circulo separados por vao — nenhum
 * recorte, nenhum caminho composto.
 */
const SOLID_20: Partial<Record<IconName, ReactNode>> = {
  home: <><path d="M10 3.6 2.9 9.6h14.2z" /><path d="M5.2 10.9h9.6v5.7H5.2z" /></>,
  hermes: <path d="M3.5 4.5h13v8h-8l-5 4z" />,
  inbox: <><path d="M3.4 5.4h13.2v4.2H3.4z" /><path d="M3.4 10.9h13.2v4.7H3.4z" /></>,
  projects: <><path d="M3.4 5.4h5l1.5 2h6.7v1.4H3.4z" /><path d="M3.4 10.1h13.2v5.5H3.4z" /></>,
  workspaces: <><path d="M8.6 3.4h8v8h-8z" /><path d="M3.4 5.6h4v10.9h-4z" /><path d="M8.6 12.7h8v3.8h-8z" /></>,
  apps: <><rect x="3.4" y="3.4" width="5.2" height="5.2" /><rect x="11.4" y="3.4" width="5.2" height="5.2" /><rect x="3.4" y="11.4" width="5.2" height="5.2" /><rect x="11.4" y="11.4" width="5.2" height="5.2" /></>,
  library: <><rect x="3.4" y="3.5" width="3.4" height="13" /><rect x="7.6" y="3.5" width="3.4" height="13" /><rect x="12.1" y="4.6" width="3.4" height="11.9" transform="rotate(-11 13.8 10.5)" /></>,
  board: <><rect x="3.4" y="3.9" width="4.2" height="12.2" /><rect x="8.9" y="3.9" width="3.2" height="8.2" /><rect x="13.4" y="3.9" width="3.2" height="10.2" /></>,
  settings: <><circle cx="10" cy="10" r="3.1" /><path d="M9.1 2.9h1.8v2.4H9.1zM9.1 14.7h1.8v2.4H9.1zM2.9 9.1h2.4v1.8H2.9zM14.7 9.1h2.4v1.8h-2.4z" /><path d="m4.9 6.2 1.3-1.3 1.7 1.7-1.3 1.3zM12.4 13.7l1.3-1.3 1.7 1.7-1.3 1.3zM13.7 4.9l1.3 1.3-1.7 1.7-1.3-1.3zM6.2 12.4l1.3 1.3-1.7 1.7-1.3-1.3z" /></>,
  capture: <><rect x="9" y="3.4" width="2" height="13.2" /><rect x="3.4" y="9" width="13.2" height="2" /></>,
  // Sem os ponteiros: a silhueta nao recorta, e um ponteiro vazado exigiria
  // recorte. Corpo, coroa e haste bastam para reconhecer o cronometro.
  tempo: <><circle cx="10" cy="11.6" r="5.9" /><rect x="7.8" y="3.2" width="4.4" height="1.9" /><rect x="9.1" y="4.8" width="1.8" height="1.4" /></>,
};

/**
 * Desenho de 16px, so para os nomes que aparecem pequenos de verdade.
 *
 * Traco de 1 sobre a grade de .5: a linha cai inteira dentro do pixel. Nao ha
 * fallback para o desenho de 20 — o tipo impede pedir 16 onde nao existe, e um
 * fallback silencioso e exatamente o borrao que a regra existe para evitar.
 */
const OUTLINE_16: Record<string, ReactNode> = {
  plus: <path d="M8 4v8M4 8h8" />,
  trash: <><path d="M4.5 5.5h7l-.5 8h-6z" /><path d="M2.5 5.5h11" /><path d="M6.5 2.5h3l.4 3" /></>,
};

export type SmallIconName = keyof typeof OUTLINE_16 & IconName;

export function Icon({ name, filled = false }: { name: IconName; filled?: boolean }) {
  const solid = filled ? SOLID_20[name] : undefined;
  return (
    <svg
      className="mos-icon"
      data-filled={solid ? true : undefined}
      width="20"
      height="20"
      viewBox="0 0 20 20"
      aria-hidden="true"
      focusable="false"
    >
      {solid ?? OUTLINE_20[name]}
    </svg>
  );
}

/** Icone de 16px, com desenho proprio. Para alvos de 28px, onde 20 sufoca. */
export function SmallIcon({ name }: { name: SmallIconName }) {
  return (
    <svg
      className="mos-icon"
      data-size="16"
      width="16"
      height="16"
      viewBox="0 0 16 16"
      aria-hidden="true"
      focusable="false"
    >
      {OUTLINE_16[name]}
    </svg>
  );
}
