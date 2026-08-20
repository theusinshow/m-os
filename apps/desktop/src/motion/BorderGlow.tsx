import { type ReactNode, forwardRef, type HTMLAttributes } from "react";

export interface BorderGlowProps extends HTMLAttributes<HTMLDivElement> {
  children: ReactNode;
  active?: boolean;
  color?: "signal" | "success" | "danger";
  className?: string;
}

/**
 * BorderGlow (React Bits Border Glow adaptado para M/OS)
 *
 * Utilizado EXCLUSIVAMENTE como estado operacional (gravação, processamento ativo,
 * Hermes gerando resposta, etc.). Nunca como adorno estático.
 */
export const BorderGlow = forwardRef<HTMLDivElement, BorderGlowProps>(function BorderGlow(
  { children, active = false, color = "signal", className = "", ...props },
  ref
) {
  return (
    <div
      ref={ref}
      className={`border-glow-container ${className}`.trim()}
      data-glow-active={active || undefined}
      data-glow-color={color}
      {...props}
    >
      {active ? <div className="border-glow-aura" aria-hidden="true" /> : null}
      <div className="border-glow-inner">{children}</div>
    </div>
  );
});
