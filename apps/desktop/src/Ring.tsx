import type { ReactNode } from "react";

/**
 * Família anel — `M-OS Widgets - Visuais e Animados v0.1`.
 *
 * A tese do desenho é que o widget mostra FORMA antes de mostrar número: a
 * leitura acontece em meio segundo, antes de qualquer leitura de palavra.
 *
 * Quatro regras que o desenho fixa e que não são negociáveis aqui:
 *
 * - ponta reta, sempre. Cap arredondado mente sobre o valor em anéis pequenos;
 * - começa às 12h, sentido horário;
 * - zero mostra só o trilho e o número, **nunca** um ponto solto de sódio;
 * - agora e hoje são traço branco, porque o sódio está reservado para carga.
 */

/** Tamanhos do desenho, com o traço de cada um. O raio não é derivado: em 44 o
 *  desenho usa 19 em vez dos 20 que a fórmula daria, e seguir a fórmula
 *  engordaria o anel da semana em 1px contra o resto da família. */
const SIZES = {
  88: { radius: 41, stroke: 6 },
  56: { radius: 25.5, stroke: 5 },
  44: { radius: 19, stroke: 4 },
  32: { radius: 14, stroke: 4 },
  14: { radius: 5.75, stroke: 2.5 },
} as const;

export type RingSize = keyof typeof SIZES;

/** Um traço do anel. `value` de 0 a 1. */
export type RingSegment = {
  value: number;
  /** 1 é o sódio cheio; 2 e 3 são os degraus de 55% e 30%. */
  depth?: 1 | 2 | 3;
  /** Anel interno, para a variante concêntrica. Em pixels de raio. */
  radius?: number;
};

const circumference = (radius: number) => 2 * Math.PI * radius;

/**
 * Cascata de 40ms com teto de 8.
 *
 * Sem o teto, o oitavo elemento de uma grade de trinta entraria meio segundo
 * depois do primeiro, e a cascata viraria espera.
 */
export const stagger = (index: number) => `${Math.min(index, 8) * 40}ms`;

export function Ring({
  size,
  segments,
  arc,
  future = false,
  now,
  delay = "0ms",
  children,
}: {
  size: RingSize;
  segments: RingSegment[];
  /** 270° com abertura no pé: a variante de tempo. */
  arc?: 270;
  /** Dia que ainda não aconteceu: só o trilho, tracejado. */
  future?: boolean;
  /** Instante atual, de 0 a 1. Desenhado em branco, nunca em sódio. */
  now?: number;
  delay?: string;
  children?: ReactNode;
}) {
  const { radius, stroke } = SIZES[size];
  const center = size / 2;
  const full = circumference(radius);
  // No arco de 270° a volta inteira representa três quartos do círculo, então
  // o valor precisa ser reescalado — senão 100% desenharia a volta toda e
  // fecharia a abertura que dá o sentido de "início e fim do dia".
  const span = arc === 270 ? 0.75 : 1;

  return (
    <span className="mos-ring-figure" data-size={size} style={{ width: size, height: size }}>
      <svg
        className="mos-ring"
        data-arc={arc}
        width={size}
        height={size}
        viewBox={`0 0 ${size} ${size}`}
        aria-hidden="true"
        focusable="false"
      >
        <circle
          className="mos-ring-track"
          data-future={future || undefined}
          cx={center}
          cy={center}
          r={radius}
          strokeWidth={stroke}
          strokeDasharray={arc === 270 ? `${full * span} ${full}` : undefined}
        />
        {!future &&
          segments.map((segment, index) => {
            const ringRadius = segment.radius ?? radius;
            const length = circumference(ringRadius);
            const drawn = Math.max(0, Math.min(1, segment.value)) * span;
            // Zero não desenha nada: um traço de comprimento zero com ponta
            // reta some, mas deixar o elemento no DOM manteria o `dasharray`
            // ativo e, com sub-pixel, o navegador pinta um ponto de sódio.
            if (drawn <= 0) return null;
            return (
              <circle
                key={index}
                className="mos-ring-value"
                data-depth={segment.depth && segment.depth > 1 ? segment.depth : undefined}
                cx={center}
                cy={center}
                r={ringRadius}
                strokeWidth={stroke}
                strokeDasharray={length}
                style={{
                  strokeDashoffset: length * (1 - drawn),
                  ["--ring-circumference" as string]: `${length}`,
                  ["--ring-delay" as string]: delay,
                }}
              />
            );
          })}
        {now !== undefined ? (
          <circle
            className="mos-ring-now"
            cx={center}
            cy={center}
            r={radius}
            strokeWidth={stroke}
            // Dois pixels de traço branco, na posição do instante.
            strokeDasharray={`0 ${full * now * span} 2 ${full}`}
          />
        ) : null}
      </svg>
      {children ? <span className="mos-ring-center">{children}</span> : null}
    </span>
  );
}

/** Número no centro em Schibsted; unidade em mono abaixo. */
export function RingLabel({ value, unit }: { value: string; unit?: string }) {
  return (
    <>
      {/* O comprimento vai para o CSS porque e ele que decide o tamanho:
          "0,0h" e "100%" tem quatro caracteres, e a 28px eles passam da
          largura interna do anel de 88px. Encolher e melhor que cortar —
          um valor cortado mente sobre o dado. Teto em 6 para o CSS nao
          precisar de uma regra por comprimento possivel. */}
      <span className="mos-ring-number" data-len={Math.min(value.length, 6)}>{value}</span>
      {unit ? <span className="mos-ring-unit">{unit}</span> : null}
    </>
  );
}
