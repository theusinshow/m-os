import { pontosDoPoligono } from "./marca";

/**
 * A barra do M/OS.
 *
 * `girando` e o unico spinner deste app: a barra da meia-volta. O brief e
 * taxativo — nao existe circulo girando, nao existem tres pontos. A animacao
 * mora no CSS (`.marca-barra[data-girando]`), e nao aqui, para o
 * `prefers-reduced-motion` poder desliga-la sem a tela saber.
 */
export function Marca({
  tamanho = 24,
  girando = false,
  className,
}: {
  tamanho?: number;
  girando?: boolean;
  className?: string;
}) {
  return (
    <svg
      className={className ? `marca-barra ${className}` : "marca-barra"}
      data-girando={girando || undefined}
      viewBox="0 0 64 64"
      width={tamanho}
      height={tamanho}
      role="img"
      aria-label="M/OS"
    >
      <polygon points={pontosDoPoligono(tamanho)} fill="currentColor" />
    </svg>
  );
}
