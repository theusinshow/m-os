import { barRects, bulletGeometry, sparkPath, stackSegments } from "./plotGeometry";
import { stagger } from "./Ring";

/**
 * Família de plot — `ADR-040`.
 *
 * A terceira classe da família de widgets, ao lado do anel (proporção de uma
 * coisa só) e da densidade (tempo como área). Aqui moram as formas que comparam
 * séries: barras, empilhada, bullet e linha.
 *
 * Nenhuma delas calcula: a aritmética inteira vem de `plotGeometry.ts`, que é
 * testado. Estes componentes só transformam número em posição.
 *
 * **Por que as três primeiras não são SVG.** Um SVG que preenche a largura do
 * card precisa de `preserveAspectRatio="none"`, e aí a escala horizontal deixa
 * de ser igual à vertical: todo `rx` vira elipse e a pílula sai como ovo. Em
 * HTML, `border-radius` é resolvido em pixels reais e o próprio CSS o limita a
 * metade da menor dimensão — a barra baixa sai arredondada e a alta sai pílula,
 * sem que ninguém precise medir nada. Por isso a geometria é pedida em
 * PORCENTAGEM (`width: 100`), e não em pixels.
 *
 * O `Spark` continua SVG porque linha não tem raio a distorcer — e leva
 * `vector-effect="non-scaling-stroke"` para a espessura não engordar na
 * vertical pelo mesmo motivo.
 */

/** O espaço de coordenadas é percentual: 100 de largura, 100 de altura. */
const SPAN = 100;

/** Barras de pílula, uma por período. `highlight` é o índice de hoje. */
export function Bars({ ratios, labels, highlight }: { ratios: number[]; labels: string[]; highlight?: number }) {
  const rects = barRects(ratios, { width: SPAN, height: SPAN, gap: 2 });

  return (
    <div className="mos-bars">
      <div className="mos-bars-figure">
        {rects.map((rect, index) => (
          <span
            className="mos-bars-track"
            key={index}
            style={{ left: `${rect.x}%`, width: `${rect.width}%` }}
          />
        ))}
        {/* Altura zero não desenha: a mesma regra do anel, pelo mesmo motivo —
            um elemento de altura zero com raio deixa resíduo de sub-pixel. */}
        {rects.map((rect, index) =>
          rect.height > 0 ? (
            <span
              className="mos-bars-value"
              key={index}
              data-now={index === highlight || undefined}
              style={{
                left: `${rect.x}%`,
                width: `${rect.width}%`,
                height: `${rect.height}%`,
                ["--ring-delay" as string]: stagger(index),
              }}
            />
          ) : null,
        )}
      </div>
      <div className="mos-bars-labels">
        {labels.map((label, index) => (
          <span className="micro-label" data-today={index === highlight || undefined} key={index}>
            {label}
          </span>
        ))}
      </div>
    </div>
  );
}

/** Uma barra repartida: composição, não comparação par a par. */
export function Stack({ values, labels }: { values: number[]; labels: string[] }) {
  const segments = stackSegments(values, { width: SPAN, gap: 1.5 });
  const depth = (index: number) => (index === 0 ? undefined : index === 1 ? 2 : 3);

  return (
    <div className="mos-stack">
      <div className="mos-stack-figure">
        {segments.map((segment) => (
          <span
            className="mos-stack-value"
            key={segment.index}
            /* O primeiro é o sódio cheio; os demais descem os mesmos degraus de
               profundidade que o anel usa, 55% e 30%. */
            data-depth={depth(segment.index)}
            style={{
              left: `${segment.x}%`,
              width: `${segment.width}%`,
              ["--ring-delay" as string]: stagger(segment.index),
            }}
          />
        ))}
      </div>
      <ul className="mos-stack-legend">
        {labels.map((label, index) => (
          <li key={index}>
            <span className="mos-stack-chip" data-depth={depth(index)} aria-hidden="true" />
            <span className="micro-label">{label}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

/** Valor contra meta, com a marca da meta desenhada — inclusive no estouro. */
export function Bullet({ value, target, over }: { value: number; target: number; over: boolean }) {
  const geometry = bulletGeometry(value, target, SPAN);

  return (
    <div className="mos-bullet">
      {geometry.fill > 0 ? (
        <span className="mos-bullet-value" data-over={over || undefined} style={{ width: `${geometry.fill}%` }} />
      ) : null}
      {/* A marca da meta é um traço branco, como agora/hoje no resto da família:
          o sódio está reservado para carga, e meta não é carga. */}
      <span className="mos-bullet-mark" style={{ left: `${geometry.mark}%` }} />
    </div>
  );
}

/** A série, como linha. Cap redondo compensado pelo `inset`. */
export function Spark({ ratios }: { ratios: number[] }) {
  const path = sparkPath(ratios, { width: SPAN, height: SPAN, inset: 4 });
  if (!path) return null;

  return (
    <svg
      className="mos-spark"
      viewBox={`0 0 ${SPAN} ${SPAN}`}
      preserveAspectRatio="none"
      aria-hidden="true"
      focusable="false"
    >
      {/* Sem isto a espessura seria esticada junto com o viewBox e a linha
          engordaria na vertical. */}
      <path className="mos-spark-line" d={path} vectorEffect="non-scaling-stroke" />
    </svg>
  );
}
