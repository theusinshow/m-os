import type { ReactNode } from "react";

/**
 * O anel de progresso.
 *
 * # Por que um anel, e não uma barra
 *
 * A barra responde *quanto falta* e precisa de largura para isso — num cartão de
 * meia coluna ela vira um traço de 140px que não distingue 60% de 70%. O anel
 * responde a mesma pergunta ocupando um quadrado, que é a forma que sobra no
 * canto de um cartão, e a diferença entre dois ângulos se lê sem régua.
 *
 * # Por que SVG e não canvas
 *
 * Porque ele precisa animar sozinho, herdar `currentColor` e ficar nítido em
 * qualquer densidade de tela. O `stroke-dasharray` faz o desenho inteiro com
 * dois números, e o navegador cuida do resto.
 */
export function Anel({
  fracao,
  tamanho = 44,
  espessura = 4,
  children,
}: {
  /** De 0 a 1. Fora disso é grampeado — um anel de 130% não quer dizer nada. */
  fracao: number;
  tamanho?: number;
  espessura?: number;
  /** O que vai no meio. Costuma ser o número. */
  children?: ReactNode;
}) {
  const parte = Math.max(0, Math.min(1, fracao));
  const raio = (tamanho - espessura) / 2;
  const volta = 2 * Math.PI * raio;

  return (
    <span className="anel" style={{ width: tamanho, height: tamanho }}>
      <svg width={tamanho} height={tamanho} aria-hidden="true">
        {/* O trilho fica sempre visível: sem ele, um anel em 10% parece um
            risco solto, e não 10% de alguma coisa. */}
        <circle
          cx={tamanho / 2}
          cy={tamanho / 2}
          r={raio}
          fill="none"
          strokeWidth={espessura}
          className="anel-trilho"
        />
        <circle
          cx={tamanho / 2}
          cy={tamanho / 2}
          r={raio}
          fill="none"
          strokeWidth={espessura}
          strokeLinecap="round"
          className="anel-arco"
          // Começa às 12h, e não às 3h: um progresso que nasce na direita lê
          // como se já tivesse andado um quarto.
          transform={`rotate(-90 ${tamanho / 2} ${tamanho / 2})`}
          strokeDasharray={`${volta * parte} ${volta}`}
        />
      </svg>
      {children ? <b className="anel-centro">{children}</b> : null}
    </span>
  );
}
