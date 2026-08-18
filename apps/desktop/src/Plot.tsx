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
 * testado. Estes componentes só transformam número em `rect` e `path`.
 *
 * As três primeiras são retângulos com `rx`, e por isso não precisam de
 * compensação — o `rx` arredonda para dentro e a barra mantém a altura exata do
 * valor. Só o `Spark`, que é traço com cap redondo, recebe `inset` para o cap
 * não ser cortado pela borda do viewBox.
 */

const VIEW = { width: 240, height: 64 };

/** Barras de pílula, uma por período. `highlight` é o índice de hoje. */
export function Bars({ ratios, labels, highlight }: { ratios: number[]; labels: string[]; highlight?: number }) {
  const rects = barRects(ratios, { width: VIEW.width, height: VIEW.height, gap: 6 });

  return (
    <div className="mos-bars">
      <svg
        className="mos-bars-figure"
        viewBox={`0 0 ${VIEW.width} ${VIEW.height}`}
        preserveAspectRatio="none"
        aria-hidden="true"
        focusable="false"
      >
        {rects.map((rect, index) => (
          <rect
            key={index}
            className="mos-bars-track"
            x={rect.x}
            y={0}
            width={rect.width}
            height={VIEW.height}
            rx={rect.width / 2}
          />
        ))}
        {/* Altura zero não desenha: a mesma regra do anel, pelo mesmo motivo —
            um retângulo de altura zero com `rx` deixa resíduo de sub-pixel. */}
        {rects.map((rect, index) =>
          rect.height > 0 ? (
            <rect
              key={index}
              className="mos-bars-value"
              data-now={index === highlight || undefined}
              x={rect.x}
              y={rect.y}
              width={rect.width}
              height={rect.height}
              rx={rect.width / 2}
              style={{ ["--ring-delay" as string]: stagger(index) }}
            />
          ) : null,
        )}
      </svg>
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
  const segments = stackSegments(values, { width: VIEW.width, gap: 4 });

  return (
    <div className="mos-stack">
      <svg
        className="mos-stack-figure"
        viewBox={`0 0 ${VIEW.width} 16`}
        preserveAspectRatio="none"
        aria-hidden="true"
        focusable="false"
      >
        {segments.map((segment) => (
          <rect
            key={segment.index}
            className="mos-stack-value"
            /* O primeiro é o sódio cheio; os demais descem os mesmos degraus de
               profundidade que o anel usa, 55% e 30%. */
            data-depth={segment.index === 0 ? undefined : segment.index === 1 ? 2 : 3}
            x={segment.x}
            y={0}
            width={segment.width}
            height={16}
            rx={8}
            style={{ ["--ring-delay" as string]: stagger(segment.index) }}
          />
        ))}
      </svg>
      <ul className="mos-stack-legend">
        {labels.map((label, index) => (
          <li key={index}>
            <span className="mos-stack-chip" data-depth={index === 0 ? undefined : index === 1 ? 2 : 3} aria-hidden="true" />
            <span className="micro-label">{label}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

/** Valor contra meta, com a marca da meta desenhada — inclusive no estouro. */
export function Bullet({ value, target, over }: { value: number; target: number; over: boolean }) {
  const geometry = bulletGeometry(value, target, VIEW.width);

  return (
    <svg
      className="mos-bullet"
      viewBox={`0 0 ${VIEW.width} 16`}
      preserveAspectRatio="none"
      aria-hidden="true"
      focusable="false"
    >
      <rect className="mos-bullet-track" x={0} y={0} width={VIEW.width} height={16} rx={8} />
      {geometry.fill > 0 ? (
        <rect className="mos-bullet-value" data-over={over || undefined} x={0} y={0} width={geometry.fill} height={16} rx={8} />
      ) : null}
      {/* A marca da meta é branca de 2px, como agora/hoje no resto da família:
          o sódio está reservado para carga, e meta não é carga. */}
      <rect className="mos-bullet-mark" x={Math.max(0, geometry.mark - 1)} y={-2} width={2} height={20} />
    </svg>
  );
}

/** A série, como linha. Cap redondo compensado pelo `inset`. */
export function Spark({ ratios }: { ratios: number[] }) {
  const path = sparkPath(ratios, { width: VIEW.width, height: 32, inset: 2 });
  if (!path) return null;

  return (
    <svg
      className="mos-spark"
      viewBox={`0 0 ${VIEW.width} 32`}
      preserveAspectRatio="none"
      aria-hidden="true"
      focusable="false"
    >
      <path className="mos-spark-line" d={path} />
    </svg>
  );
}
