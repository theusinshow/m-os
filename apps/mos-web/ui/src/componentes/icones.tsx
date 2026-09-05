import type { ReactNode } from "react";
import type { Pagina } from "../navegacao";

/**
 * Os icones da barra.
 *
 * # Por que desenhados aqui, e nao importados
 *
 * Sao cinco formas de traco unico. Uma biblioteca de icones custaria dezenas de
 * KB e uma dependencia para entregar cinco caminhos SVG que cabem nesta tela —
 * e o app abre no 4G.
 *
 * Todos herdam `currentColor`: quem decide a cor e o estado do botao, e nao o
 * icone. Sem isso, o icone ativo precisaria ser um segundo arquivo.
 */
export function Icone({ pagina }: { pagina: Pagina }) {
  return (
    <svg
      width="22"
      height="22"
      viewBox="0 0 22 22"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.4"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {CAMINHOS[pagina] ?? CAMINHOS.mais}
    </svg>
  );
}

const CAMINHOS: Partial<Record<Pagina, ReactNode>> = {
  // Quatro quadrados: a Home é uma grade de cartões, e o ícone é a planta dela.
  home: (
    <>
      <rect x="3" y="3" width="7" height="7" />
      <rect x="12" y="3" width="7" height="7" />
      <rect x="3" y="12" width="7" height="7" />
      <rect x="12" y="12" width="7" height="7" />
    </>
  ),
  agenda: (
    <>
      <rect x="3" y="4.5" width="16" height="14.5" rx="1" />
      <path d="M3 9.5h16M7.5 2.5v4M14.5 2.5v4" />
    </>
  ),
  // Dois tiques e duas linhas: a tela tem duas metades — o que triar e o que
  // fazer — e o ícone diz isso antes do rótulo.
  fazer: <path d="M3 6.5l2.4 2.4L9.8 4.5M13 7h6M3 15.5l2.4 2.4 4.4-4.4M13 16h6" />,
  mais: <path d="M3 6h16M3 11h16M3 16h10" />,
};
