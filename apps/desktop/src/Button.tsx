import type { ButtonHTMLAttributes, ReactNode } from "react";

/**
 * O botão do M/OS.
 *
 * Morava dentro do `App.tsx` e saiu quando o `Timer` precisou dele: importar de
 * lá criaria ciclo, já que o `App` importa o `Timer`. Um módulo próprio resolve
 * sem truque, e é onde ele deveria estar desde que passou a ter cinco variantes.
 */
export function Button({ variant = "secondary", size, className = "", children, ...props }: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: "primary" | "secondary" | "outline" | "ghost" | "danger"; size?: "sm"; children: ReactNode }) {
  // className e somado, nunca sobrescrito: espalhar props depois do className
  // fazia um className de fora apagar "button primary" inteiro.
  return <button className={`button ${variant} ${size ?? ""} ${className}`.replace(/\s+/g, " ").trim()} type="button" {...props}>{children}</button>;
}
