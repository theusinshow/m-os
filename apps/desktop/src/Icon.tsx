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
  | "cronocad"
  | "meetings"
  | "calendar"
  | "finance"
  | "settings"
  | "search"
  | "capture"
  | "plus"
  | "more"
  | "close"
  | "archive"
  | "trash"
  | "attention";

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
  // Microfone: capsula, arco de captacao, haste e base. O arco e o que
  // separa "microfone" de "pilula" a 20px — sem ele a silhueta some.
  meetings: <><rect x="8.5" y="2.5" width="3" height="8" rx="1.5" /><path d="M5.5 9.5a4.5 4.5 0 0 0 9 0M10 14v3.5M7.5 17.5h5" /></>,
  tempo: <><circle cx="10" cy="11.5" r="5.5" /><path d="M8 3.5h4M10 4v2M10 11.5V8M10 11.5h2.5" /></>,
  // O CronoCAD: quadrado de cantos macios com o C dentro (ADR-050).
  //
  // O C e um ARCO ABERTO, e nao uma letra tipografica: a 20px com traco de
  // 1.25 as junturas de um "C" de fonte viram borrao, e o que sobrevive da
  // marca e a silhueta — quadrado fechado, anel aberto a direita.
  cronocad: <><rect x="3.5" y="3.5" width="13" height="13" rx="2" /><path d="M11.95 7.21A3.4 3.4 0 1 0 11.95 12.79" /></>,
  // Folha com a regua do cabecalho e as duas argolas. A regua e o que separa
  // "calendario" de "janela" a 20px.
  calendar: <><rect x="3.5" y="5.5" width="13" height="11" /><path d="M3.5 9.5h13M7 3.5v3M13 3.5v3" /></>,
  // Nota com o valor marcado por um circulo vazado: diferencia de "calendar" e
  // "board" sem precisar de simbolo de moeda (que nao cabe limpo no traco de
  // 1.25 a 20px).
  finance: <><rect x="3" y="6" width="14" height="8" /><circle cx="10" cy="10" r="2" /></>,
  // Coroa de oito dentes em vez de engrenagem desenhada: a 20px com traco de
  // 1.25 os dentes de uma engrenagem colam uns nos outros.
  settings: <><circle cx="10" cy="10" r="2.5" /><path d="M10 3.5v2M10 14.5v2M3.5 10h2M14.5 10h2M5.4 5.4l1.4 1.4M13.2 13.2l1.4 1.4M14.6 5.4l-1.4 1.4M6.8 13.2l-1.4 1.4" /></>,
  search: <><circle cx="8.5" cy="8.5" r="4.5" /><path d="m12 12 4 4" /></>,
  capture: <path d="M10 3.5v13M3.5 10h13" />,
  // Sino em linha reta, e nao em arco. O arco seria a unica curva deste
  // conjunto, e curva nao cai na grade de .5 — ela antialiaza, e o traco sai
  // mais grosso e mais claro que o dos vizinhos. No rail isso lia como uma
  // borda branca em volta de um borrao, ao lado de um `+` e de uma engrenagem
  // nitidos. O chanfro nos ombros mantem a silhueta sem cobrar a curva.
  attention: <><path d="M6.5 13.5V9.5L8 6.5h4l1.5 3v4" /><path d="M4.5 13.5h11" /></>,
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
  attention: <><path d="M6 13.2V9.2l1.8-3.4h4.4L14 9.2v4z" /><rect x="4" y="14.1" width="12" height="1.7" /></>,
  // Sem os ponteiros: a silhueta nao recorta, e um ponteiro vazado exigiria
  // recorte. Corpo, coroa e haste bastam para reconhecer o cronometro.
  meetings: <><rect x="8.2" y="2.2" width="3.6" height="8.8" rx="1.8" /><rect x="9.1" y="13.4" width="1.8" height="4.2" /><rect x="6.9" y="17.6" width="6.2" height="1.6" /><path d="M4.9 8.2h1.9v1.1a3.2 3.2 0 0 0 6.4 0V8.2h1.9v1.1a5.1 5.1 0 0 1-10.2 0z" /></>,
  tempo: <><circle cx="10" cy="11.6" r="5.9" /><rect x="7.8" y="3.2" width="4.4" height="1.9" /><rect x="9.1" y="4.8" width="1.8" height="1.4" /></>,
  // Ativo: o quadrado enche, e o C vira BURACO — leva a cor do fundo do rail,
  // como o olho do Argos. Nao e recorte com `fill-rule`: o C e um setor de
  // anel, caminho simples e fechado, que e o que a regra de construcao acima
  // manda usar no lugar de furo.
  cronocad: <><rect x="3" y="3" width="14" height="14" rx="2.5" /><path fill="var(--canvas)" d="M12.7 6.78A4.2 4.2 0 1 0 12.7 13.22L11.41 11.69A2.2 2.2 0 1 1 11.41 8.31Z" /></>,
  calendar: <><rect x="3.2" y="5.2" width="13.6" height="11.6" /><rect x="6.4" y="2.9" width="1.6" height="3.4" /><rect x="12" y="2.9" width="1.6" height="3.4" /></>,
  // Sem o circulo vazado: a silhueta nao recorta, e um furo no meio exigiria
  // caminho composto. O retangulo da nota basta para reconhecer o item ao
  // lado dos outros destinos ativos do rail.
  finance: <><rect x="2.9" y="5.9" width="14.2" height="8.2" /></>,
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
  close: <path d="m4 4 8 8M12 4l-8 8" />,
  trash: <><path d="M4.5 5.5h7l-.5 8h-6z" /><path d="M2.5 5.5h11" /><path d="M6.5 2.5h3l.4 3" /></>,
};

export type SmallIconName = keyof typeof OUTLINE_16 & IconName;

export function Icon({ name, filled = false }: { name: IconName; filled?: boolean }) {
  const solid = filled ? SOLID_20[name] : undefined;
  return (
    <svg
      className="mos-icon"
      data-icon={name}
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
