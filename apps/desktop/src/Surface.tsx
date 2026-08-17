import { forwardRef, useEffect, useRef, useState } from "react";
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
 * Cabeçalho de pane: localização à esquerda; contagem e ações silenciosas à
 * direita. É deliberadamente menor que `PageHeader`: listas master–detail não
 * precisam repetir um título grande acima do caminho que já as nomeia.
 */
export function PaneHeader({ segments, meta, actions }: {
  segments: string[];
  meta?: string;
  actions?: ReactNode;
}) {
  return (
    <header className="pane-heading">
      <ContextPath segments={segments} />
      {meta || actions ? <div className="pane-heading-meta">{meta ? <span className="micro-label">{meta}</span> : null}{actions}</div> : null}
    </header>
  );
}

/**
 * O contrato do Inspector do M/OS. Em desktop ele é a pane de detalhe; em
 * larguras estreitas vira a segunda etapa do fluxo e oferece volta explícita.
 * O componente só governa navegação/foco — o conteúdo e as regras continuam
 * pertencendo à superfície que o usa.
 */
export const Inspector = forwardRef<HTMLElement, {
  label: string;
  children: ReactNode;
  onBack?: () => void;
  onEscape?: () => void;
}>(function Inspector({ label, children, onBack, onEscape }, ref) {
  return (
    <article
      ref={ref}
      className="detail-pane inspector"
      tabIndex={-1}
      aria-label={label}
      onKeyDown={(event) => {
        if (event.key !== "Escape" || !onEscape) return;
        event.preventDefault();
        onEscape();
      }}
    >
      {onBack ? <div className="inspector-nav"><button type="button" onClick={onBack}>Voltar à lista</button><span className="micro-label">ESC</span></div> : null}
      {children}
    </article>
  );
});

export type ActionMenuItem = {
  label: string;
  danger?: boolean;
  disabled?: boolean;
  onSelect: () => void;
};

/** Menu compacto com o mesmo fechamento por mouse e teclado em toda página. */
export function ActionMenu({ trigger, items, label = "Mais ações" }: {
  trigger: ReactNode;
  items: ActionMenuItem[];
  label?: string;
}) {
  const root = useRef<HTMLDetailsElement>(null);
  const summary = useRef<HTMLElement>(null);
  const [open, setOpen] = useState(false);

  useEffect(() => {
    if (!open) return;
    const closeOutside = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) {
        root.current?.removeAttribute("open");
        setOpen(false);
      }
    };
    document.addEventListener("pointerdown", closeOutside);
    return () => document.removeEventListener("pointerdown", closeOutside);
  }, [open]);

  function closeMenu(restoreFocus = false) {
    root.current?.removeAttribute("open");
    setOpen(false);
    if (restoreFocus) requestAnimationFrame(() => summary.current?.focus());
  }

  return (
    <details
      ref={root}
      className="menu"
      onToggle={(event) => setOpen(event.currentTarget.open)}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          closeMenu(true);
          return;
        }
        if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
        event.preventDefault();
        const menuItems = Array.from(root.current?.querySelectorAll<HTMLButtonElement>("[role='menuitem']:not(:disabled)") ?? []);
        if (!menuItems.length) return;
        const currentIndex = menuItems.indexOf(document.activeElement as HTMLButtonElement);
        const nextIndex = event.key === "Home"
          ? 0
          : event.key === "End"
            ? menuItems.length - 1
            : currentIndex < 0
              ? event.key === "ArrowUp" ? menuItems.length - 1 : 0
              : (currentIndex + (event.key === "ArrowDown" ? 1 : -1) + menuItems.length) % menuItems.length;
        if (!root.current?.open) {
          root.current?.setAttribute("open", "");
          setOpen(true);
          requestAnimationFrame(() => menuItems[nextIndex]?.focus());
        } else {
          menuItems[nextIndex]?.focus();
        }
      }}
    >
      <summary ref={summary} aria-label={label} title={label} aria-haspopup="menu" aria-expanded={open}>{trigger}</summary>
      <div role="menu" aria-label={label}>
        {items.map((item) => <button key={item.label} type="button" role="menuitem" disabled={item.disabled} className={item.danger ? "danger-text" : undefined} onClick={() => { closeMenu(); item.onSelect(); }}>{item.label}</button>)}
      </div>
    </details>
  );
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

/**
 * Cabeçalho de tela: o que é isto, e o que dá para fazer aqui.
 *
 * O `ContextPath` diz onde você está; isto diz o que a tela FAZ. São perguntas
 * diferentes, e sem a segunda cada tela do Tempo começava direto no conteúdo —
 * era parte do que fazia a travessia parecer menos organizada que o CronoCAD.
 *
 * As ações vivem aqui, à direita, e não espalhadas no meio: numa tela que rola,
 * um botão no meio do conteúdo é um botão que precisa ser procurado.
 */
export function PageHeader({ title, subtitle, actions }: {
  title: string;
  subtitle?: string;
  actions?: ReactNode;
}) {
  return (
    <header className="page-head">
      <div>
        <h1>{title}</h1>
        {subtitle ? <p>{subtitle}</p> : null}
      </div>
      {actions ? <div className="page-head-actions">{actions}</div> : null}
    </header>
  );
}

/**
 * Painel com MOLDURA.
 *
 * O `Panel` do M/OS separa por rótulo e ar, o que funciona numa página de uma
 * coluna. O Tempo tem telas de duas e três colunas, e ali o ar não basta: sem
 * borda, dois cards lado a lado se leem como um bloco só de texto desalinhado.
 *
 * O design system diz que card é a resposta preguiçosa, e concordo em geral —
 * aqui ele é o que separa colunas que precisam ser lidas como coisas distintas,
 * que é exatamente quando ele deixa de ser preguiça.
 */
export function Card({ label, count, action, children, className = "" }: {
  label?: string;
  count?: string;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={`tempo-card ${className}`}>
      {label || action || count ? (
        <header className="tempo-card-head">
          {label ? <h2 className="micro-label">{label}</h2> : <span />}
          {count ? <span className="tempo-card-count">{count}</span> : null}
          {action}
        </header>
      ) : null}
      <div className="tempo-card-body">{children}</div>
    </section>
  );
}

/**
 * Um número grande com rótulo micro.
 *
 * A forma antes da palavra: o número é o que se lê em meio segundo, e o rótulo
 * só existe para dizer de que ele é.
 */
export function Stat({ label, value, hint }: { label: string; value: string; hint?: string }) {
  return (
    <div className="tempo-stat">
      <span className="micro-label">{label}</span>
      <strong>{value}</strong>
      {hint ? <small>{hint}</small> : null}
    </div>
  );
}
