import { BODY, type ArgosPose, eyesFor, weightFor } from "./argosPose";

/**
 * Argos — a face do estado do M/OS (ADR-041).
 *
 * Puramente apresentacional: recebe a pose e desenha. Quem decide a pose é o
 * `useArgosPose`, e quem decide a geometria é o `argosPose.ts`.
 *
 * `aria-hidden` de propósito: os mesmos fatos já são anunciados em texto pelo
 * estado de sistema ao lado e pela página do Hermes. Argos é redundante por
 * construção, e é isso que o torna seguro de esconder — um leitor de tela não
 * deve ouvir a mesma coisa duas vezes.
 */
export function Argos({ pose }: { pose: ArgosPose }) {
  const { left, right } = eyesFor(pose);

  return (
    <svg
      className="argos"
      data-pose={pose}
      data-weight={weightFor(pose)}
      width={BODY.size}
      height={BODY.size}
      viewBox={`0 0 ${BODY.size} ${BODY.size}`}
      aria-hidden="true"
      focusable="false"
    >
      <rect
        className="argos-body"
        x={BODY.inset}
        y={BODY.inset}
        width={BODY.size - BODY.inset * 2}
        height={BODY.size - BODY.inset * 2}
        rx={BODY.radius}
      />
      {[left, right].map((eye, index) => (
        <ellipse
          className="argos-eye"
          key={index}
          cx={eye.x}
          cy={eye.y}
          rx={eye.rx}
          ry={eye.ry}
          transform={eye.tilt ? `rotate(${eye.tilt} ${eye.x} ${eye.y})` : undefined}
        />
      ))}
    </svg>
  );
}
