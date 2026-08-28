import { forwardRef, useEffect, useId, useRef, useState } from "react";
import type { ReactNode } from "react";
import { AnimatePresence, LazyMotion, m, useReducedMotion } from "framer-motion";
import { MOTION_DURATIONS, MOTION_EASINGS } from "./motion";
import { AnimatedNumber } from "./motion/AnimatedNumber";

const loadMotionFeatures = () => import("./motionFeatures").then((module) => module.default);

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
  open?: boolean;
  onBack?: () => void;
  onEscape?: () => void;
}>(function Inspector({ label, children, open = true, onBack, onEscape }, ref) {
  const [compact, setCompact] = useState(() => window.matchMedia("(max-width: 960px)").matches);
  const reducedMotion = useReducedMotion();

  useEffect(() => {
    const media = window.matchMedia("(max-width: 960px)");
    const update = () => setCompact(media.matches);
    media.addEventListener("change", update);
    return () => media.removeEventListener("change", update);
  }, []);

  const visible = !compact || open;
  return (
    <LazyMotion features={loadMotionFeatures} strict>
      <AnimatePresence initial={false}>
        {visible ? <m.article
          ref={ref}
          className="detail-pane inspector"
          tabIndex={-1}
          aria-label={label}
          initial={reducedMotion ? { opacity: 0 } : { opacity: 0, x: 16 }}
          animate={{ opacity: 1, x: 0 }}
          exit={reducedMotion ? { opacity: 0, pointerEvents: "none" } : { opacity: 0, x: 12, pointerEvents: "none" }}
          transition={{ duration: reducedMotion ? 0 : MOTION_DURATIONS.enter, ease: MOTION_EASINGS.enter }}
          onKeyDown={(event) => {
            if (event.key !== "Escape" || !onEscape) return;
            event.preventDefault();
            onEscape();
          }}
        >
          {onBack ? <div className="inspector-nav"><button type="button" onClick={onBack}>Voltar à lista</button><span className="micro-label">ESC</span></div> : null}
          {children}
        </m.article> : null}
      </AnimatePresence>
    </LazyMotion>
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
  const root = useRef<HTMLDivElement>(null);
  const triggerButton = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);
  const menuId = useId();
  const reducedMotion = useReducedMotion();

  useEffect(() => {
    if (!open) return;
    const closeOutside = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("pointerdown", closeOutside);
    return () => document.removeEventListener("pointerdown", closeOutside);
  }, [open]);

  function closeMenu(restoreFocus = false) {
    if (restoreFocus) triggerButton.current?.focus();
    setOpen(false);
  }

  function focusMenuItem(position: "first" | "last") {
    requestAnimationFrame(() => {
      const menuItems = Array.from(root.current?.querySelectorAll<HTMLButtonElement>("[role='menuitem']:not(:disabled)") ?? []);
      menuItems[position === "first" ? 0 : menuItems.length - 1]?.focus();
    });
  }

  return (
    <div
      ref={root}
      className="menu"
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setOpen(false);
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape" && open) {
          event.preventDefault();
          event.stopPropagation();
          closeMenu(true);
          return;
        }
        if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
        event.preventDefault();
        const menuItems = Array.from(root.current?.querySelectorAll<HTMLButtonElement>("[role='menuitem']:not(:disabled)") ?? []);
        if (!open) {
          setOpen(true);
          focusMenuItem(event.key === "ArrowUp" || event.key === "End" ? "last" : "first");
          return;
        }
        if (!menuItems.length) return;
        const currentIndex = menuItems.indexOf(document.activeElement as HTMLButtonElement);
        const nextIndex = event.key === "Home"
          ? 0
          : event.key === "End"
            ? menuItems.length - 1
            : currentIndex < 0
              ? event.key === "ArrowUp" ? menuItems.length - 1 : 0
              : (currentIndex + (event.key === "ArrowDown" ? 1 : -1) + menuItems.length) % menuItems.length;
        menuItems[nextIndex]?.focus();
      }}
    >
      <button
        ref={triggerButton}
        className="menu-trigger"
        type="button"
        aria-label={label}
        title={label}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={open ? menuId : undefined}
        onClick={() => setOpen((current) => !current)}
      >{trigger}</button>
      <LazyMotion features={loadMotionFeatures} strict>
        <AnimatePresence>
          {open ? <m.div
            id={menuId}
            role="menu"
            aria-label={label}
            initial={reducedMotion ? { opacity: 0 } : { opacity: 0, scale: 0.97, y: -4 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={reducedMotion ? { opacity: 0, pointerEvents: "none" } : { opacity: 0, scale: 0.97, y: -2, pointerEvents: "none" }}
            transition={{ duration: reducedMotion ? 0 : MOTION_DURATIONS.micro, ease: MOTION_EASINGS.enter }}
          >
            {items.map((item) => <button key={item.label} type="button" role="menuitem" disabled={item.disabled} className={item.danger ? "danger-text" : undefined} onClick={() => { closeMenu(true); item.onSelect(); }}>{item.label}</button>)}
          </m.div> : null}
        </AnimatePresence>
      </LazyMotion>
    </div>
  );
}

export type StateMessageState = "empty" | "loading" | "error" | "saving" | "saved";

/** Estado operacional curto e consistente para formulários e superfícies. */
export function StateMessage({ state, label, detail, className = "" }: {
  state: StateMessageState;
  label: string;
  detail?: string;
  className?: string;
}) {
  return <div className={`state-message ${className}`.trim()} data-state={state} role="status" aria-live="polite" aria-atomic="true">
    <span className="state-message-marker" aria-hidden="true" />
    <div><strong>{label}</strong>{detail ? <details><summary>Detalhes técnicos</summary><p>{detail}</p></details> : null}</div>
  </div>;
}

/**
 * `rule` troca a régua: em vez de sublinhar o cabeçalho inteiro, ela sai do
 * rótulo e atravessa a linha. É como o desenho separa uma seção que abre a
 * página de um painel que mostra conteúdo.
 */
export function Panel({ label, count, action, rule = false, value, unit, children, className = "" }: { label: string; count?: string; action?: ReactNode; rule?: boolean; value?: string; unit?: string; children: ReactNode; className?: string }) {
  return <section className={`panel ${className}`} data-panel={label} data-rule={rule || undefined}><header className="panel-header"><h2>{label}</h2>{rule ? <span className="panel-rule" aria-hidden="true" /> : null}{count ? <span className="panel-count">{count}</span> : null}{action}</header>{value ? <p className="widget-head"><span className="widget-value">{value}</span>{unit ? <span className="widget-unit">{unit}</span> : null}</p> : null}{children}</section>;
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
 * Uma região: o mesmo cabeçalho do `Card`, SEM a moldura.
 *
 * O `Card` acima defende que borda é o que separa duas colunas que precisam ser
 * lidas como coisas distintas. O argumento vale — mas foi aplicado a tudo, e
 * quando toda peça da tela tem moldura, moldura para de separar coisa alguma e
 * vira só o barulho que a auditoria chamou de cardização.
 *
 * A regra que substitui: UMA superfície elevada por tela, reservada à intenção
 * dominante. No Painel é o cronômetro; em Projetos é o inspector de cobrança.
 * Todo o resto é região — rótulo, régua de 1px e o conteúdo encostado nela.
 *
 * Separação continua existindo: ela passa a vir da régua e do vão, que é o que
 * o design system pede quando diz que dado temporal não precisa virar card.
 */
export function Region({ label, count, action, children, className = "" }: {
  label: string;
  count?: string;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={`tempo-region ${className}`}>
      <header className="tempo-region-head">
        <h2 className="micro-label">{label}</h2>
        {count ? <span className="tempo-region-count">{count}</span> : null}
        {action}
      </header>
      {children}
    </section>
  );
}

/**
 * Uma linha de proporção: nome, valor, barra e a fração que ela representa.
 *
 * A barra é neutra por decisão, não por falta de cor: proporção é quantidade, e
 * quantidade não é sinal. Em sódio ela seria a segunda cor de sinal do sistema,
 * que é o que a ADR-034 não autoriza — e disputaria com o botão que de fato faz
 * alguma coisa na mesma tela.
 */
export function Share({ name, value, hours, share }: {
  name: string;
  value: string;
  hours: string;
  /** 0 a 1. Fora da faixa, a barra satura em vez de vazar da régua. */
  share: number;
}) {
  const pct = Math.max(0, Math.min(1, share));
  return (
    <div className="tempo-share">
      <div className="tempo-share-head">
        <span>{name}</span>
        <strong>{value}</strong>
      </div>
      <div className="tempo-meter" aria-hidden="true">
        <span style={{ width: `${(pct * 100).toFixed(1)}%` }} />
      </div>
      <span className="micro-label">{hours} · {Math.round(pct * 100)}%</span>
    </div>
  );
}

/**
 * A faixa de filtros: rótulo, campos e atalhos de período.
 *
 * Não é região nem card. Ela é delimitada em cima E embaixo porque é a única
 * peça da tela que não se lê — se opera, e depois se esquece. A régua dupla diz
 * "isto é o controle, o que vem abaixo é a resposta", que é a leitura que o
 * card não dava: dentro de uma moldura, filtro e resultado tinham o mesmo peso.
 */
export function FilterBand({ children, className = "" }: { children: ReactNode; className?: string }) {
  return (
    <section className={`tempo-filter-band ${className}`}>
      <span className="micro-label">FILTRO</span>
      {children}
    </section>
  );
}

/**
 * Uma faixa de leitura: números lado a lado, separados por régua vertical.
 *
 * Não é card e não é região — não tem rótulo próprio porque cada número já
 * carrega o dele. Existe para o olho comparar três ou quatro valores sem
 * descer, que é a única razão de pô-los lado a lado.
 */
export function StatBand({ children, className = "" }: { children: ReactNode; className?: string }) {
  return <div className={`tempo-band ${className}`}>{children}</div>;
}

/**
 * Um número grande com rótulo micro.
 *
 * A forma antes da palavra: o número é o que se lê em meio segundo, e o rótulo
 * só existe para dizer de que ele é.
 */
export function Stat({ label, value, hint, settled = false }: {
  label: string;
  value: string;
  hint?: string;
  /** Dinheiro que já entrou. Recua para secundário: não muda mais decisão nenhuma. */
  settled?: boolean;
}) {
  const numericVal = Number(value);
  const isPureNumber = !Number.isNaN(numericVal) && /^-?\d+(\.\d+)?$/.test(value.trim());

  return (
    <div className="tempo-stat" data-settled={settled || undefined}>
      <span className="micro-label">{label}</span>
      <strong>
        {isPureNumber ? <AnimatedNumber value={numericVal} /> : value}
      </strong>
      {hint ? <small>{hint}</small> : null}
    </div>
  );
}
