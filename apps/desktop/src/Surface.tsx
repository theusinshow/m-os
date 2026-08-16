import type { ReactNode } from "react";

/**
 * As três peças que montam qualquer página do M/OS.
 *
 * Moravam dentro do `App.tsx` e saíram quando a página de Tempo precisou delas:
 * importar de lá criaria ciclo, já que o `App` importa as páginas. É o mesmo
 * caminho que o `Button` fez, e pelo mesmo motivo.
 */

export function ContextPath({ segments }: { segments: string[] }) {
  return <div className="context-path" aria-label={segments.join(" / ")}>{segments.map((segment, index) => <span key={`${segment}-${index}`} className={index === segments.length - 1 ? "current" : undefined}>{index ? <b>/</b> : null}{segment}</span>)}</div>;
}

/**
 * `rule` troca a régua: em vez de sublinhar o cabeçalho inteiro, ela sai do
 * rótulo e atravessa a linha. É como o desenho separa uma seção que abre a
 * página de um painel que mostra conteúdo.
 */
export function Panel({ label, count, action, rule = false, children, className = "" }: { label: string; count?: string; action?: ReactNode; rule?: boolean; children: ReactNode; className?: string }) {
  return <section className={`panel ${className}`} data-panel={label} data-rule={rule || undefined}><header className="panel-header"><h2>{label}</h2>{rule ? <span className="panel-rule" aria-hidden="true" /> : null}{count ? <span className="panel-count">{count}</span> : null}{action}</header>{children}</section>;
}

export function EmptyState({ children }: { children: ReactNode }) {
  return <p className="empty-state">{children}</p>;
}
