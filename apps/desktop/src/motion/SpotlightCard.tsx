import { forwardRef, useEffect, useRef, useState, type HTMLAttributes, type ReactNode } from "react";

export interface SpotlightCardProps extends HTMLAttributes<HTMLDivElement> {
  children: ReactNode;
  className?: string;
  spotlightColor?: string;
  size?: number;
  disabled?: boolean;
}

/**
 * SpotlightCard (React Bits adaptado para M/OS)
 *
 * Adiciona um sutil realce radial seguindo o cursor na superfície do card.
 * Performance máxima: atualiza variáveis CSS diretamente no nó do DOM sem re-render.
 * Respeita preferências de redução de movimento.
 */
export const SpotlightCard = forwardRef<HTMLDivElement, SpotlightCardProps>(function SpotlightCard(
  {
    children,
    className = "",
    spotlightColor,
    // Raio menor que o do React Bits original: num card de widget do M/OS, 360px
    // acendia a superficie inteira e o facho deixava de ler como cursor.
    size = 240,
    disabled = false,
    onPointerMove,
    onPointerEnter,
    onPointerLeave,
    style,
    ...props
  },
  forwardedRef
) {
  const innerRef = useRef<HTMLDivElement>(null);
  const rafId = useRef<number | null>(null);
  const [reducedMotion, setReducedMotion] = useState(false);

  useEffect(() => {
    const media = window.matchMedia("(prefers-reduced-motion: reduce)");
    setReducedMotion(media.matches);
    const listener = () => setReducedMotion(media.matches);
    media.addEventListener("change", listener);
    return () => media.removeEventListener("change", listener);
  }, []);

  const handlePointerEnter = (e: React.PointerEvent<HTMLDivElement>) => {
    if (disabled || reducedMotion) return;
    const el = innerRef.current;
    if (el) {
      el.style.setProperty("--spotlight-opacity", "1");
    }
    onPointerEnter?.(e);
  };

  const handlePointerLeave = (e: React.PointerEvent<HTMLDivElement>) => {
    if (disabled || reducedMotion) return;
    const el = innerRef.current;
    if (el) {
      el.style.setProperty("--spotlight-opacity", "0");
    }
    onPointerLeave?.(e);
  };

  const handlePointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (disabled || reducedMotion) return;
    const el = innerRef.current;
    if (!el) return;

    if (rafId.current) cancelAnimationFrame(rafId.current);
    const clientX = e.clientX;
    const clientY = e.clientY;

    rafId.current = requestAnimationFrame(() => {
      const rect = el.getBoundingClientRect();
      const x = clientX - rect.left;
      const y = clientY - rect.top;
      el.style.setProperty("--spotlight-x", `${x}px`);
      el.style.setProperty("--spotlight-y", `${y}px`);
    });

    onPointerMove?.(e);
  };

  const defaultSpotlight =
    spotlightColor ||
    "radial-gradient(var(--spotlight-size, 240px) circle at var(--spotlight-x, 0px) var(--spotlight-y, 0px), var(--signal-wash) 0%, transparent 70%)";

  return (
    <div
      ref={(node) => {
        (innerRef as React.MutableRefObject<HTMLDivElement | null>).current = node;
        if (typeof forwardedRef === "function") forwardedRef(node);
        else if (forwardedRef) (forwardedRef as React.MutableRefObject<HTMLDivElement | null>).current = node;
      }}
      className={`spotlight-card ${className}`.trim()}
      onPointerEnter={handlePointerEnter}
      onPointerLeave={handlePointerLeave}
      onPointerMove={handlePointerMove}
      style={{
        ...style,
        ["--spotlight-size" as string]: `${size}px`,
        ["--spotlight-gradient" as string]: defaultSpotlight,
      }}
      {...props}
    >
      <div className="spotlight-card-overlay" aria-hidden="true" />
      <div className="spotlight-card-content">{children}</div>
    </div>
  );
});
