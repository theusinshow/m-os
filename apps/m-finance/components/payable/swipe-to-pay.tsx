"use client";

import { useCallback, useRef, useState, type ReactNode } from "react";
import { Check, Undo2 } from "lucide-react";
import { cn } from "@/lib/utils";

/** Abaixo disto o gesto volta sozinho: foi tremida de dedo, não intenção. */
const MIN_THRESHOLD_PX = 96;
const THRESHOLD_RATIO = 0.4;
const MAX_DRAG_RATIO = 0.55;
/** Enquanto o dedo não anda isto, ainda não se sabe se o gesto é rolagem. */
const AXIS_LOCK_PX = 6;

/** Um toque em cima de um controle é um toque, não o começo de um arrasto. */
const INTERACTIVE = "button, a, input, select, textarea, summary, label, [role='button']";

type Axis = "x" | "y" | null;

/**
 * Arrastar a linha para marcar paga — o gesto do app antigo, que o dono repete
 * dezenas de vezes no dia em que o salário cai.
 *
 * É atalho, nunca a única porta: o botão "Marcar pago" continua na linha, para
 * teclado, leitor de tela e mouse. Quem não descobrir o gesto não perde nada.
 */
export function SwipeToPay({
  paid,
  disabled = false,
  onPay,
  onReopen,
  children,
}: {
  paid: boolean;
  disabled?: boolean;
  onPay: () => void;
  onReopen: () => void;
  children: ReactNode;
}) {
  const rootRef = useRef<HTMLDivElement>(null);
  const startRef = useRef<{
    x: number;
    y: number;
    pointerId: number;
    threshold: number;
    maxDrag: number;
  } | null>(null);
  const axisRef = useRef<Axis>(null);
  const [offset, setOffset] = useState(0);
  const [dragging, setDragging] = useState(false);
  // A medida entra em estado porque a renderização precisa dela para desenhar
  // o quanto o gesto já andou — e ler a ref durante o render é proibido.
  const [threshold, setThreshold] = useState(MIN_THRESHOLD_PX);

  // Pendente arrasta para a esquerda e vira paga; paga arrasta para a direita
  // e reabre. Um sentido só por estado, para o gesto nunca ser ambíguo.
  const direction = paid ? 1 : -1;

  const reset = useCallback(() => {
    startRef.current = null;
    axisRef.current = null;
    setDragging(false);
    setOffset(0);
  }, []);

  function handlePointerDown(event: React.PointerEvent<HTMLDivElement>) {
    if (disabled || event.button !== 0) return;
    if ((event.target as HTMLElement).closest(INTERACTIVE)) return;

    // Medido uma vez por gesto: a largura não muda no meio do arrasto.
    const width = rootRef.current?.getBoundingClientRect().width ?? 0;
    const measured = Math.max(MIN_THRESHOLD_PX, width * THRESHOLD_RATIO);
    startRef.current = {
      x: event.clientX,
      y: event.clientY,
      pointerId: event.pointerId,
      threshold: measured,
      maxDrag: width * MAX_DRAG_RATIO,
    };
    axisRef.current = null;
    setThreshold(measured);
  }

  function handlePointerMove(event: React.PointerEvent<HTMLDivElement>) {
    const start = startRef.current;
    if (!start || start.pointerId !== event.pointerId) return;

    const dx = event.clientX - start.x;
    const dy = event.clientY - start.y;

    if (axisRef.current === null) {
      if (Math.max(Math.abs(dx), Math.abs(dy)) < AXIS_LOCK_PX) return;
      // Rolagem vertical ganha do arrasto: a lista precisa continuar rolando.
      axisRef.current = Math.abs(dx) > Math.abs(dy) ? "x" : "y";
      if (axisRef.current === "y") {
        reset();
        return;
      }
      event.currentTarget.setPointerCapture(event.pointerId);
      setDragging(true);
    }

    const travelled = direction > 0 ? Math.max(dx, 0) : Math.min(dx, 0);
    setOffset(Math.max(-start.maxDrag, Math.min(start.maxDrag, travelled)));
  }

  function handlePointerUp(event: React.PointerEvent<HTMLDivElement>) {
    const start = startRef.current;
    if (!start || start.pointerId !== event.pointerId) return;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }

    const committed = axisRef.current === "x" && Math.abs(offset) >= start.threshold;
    reset();
    if (committed) {
      if (paid) onReopen();
      else onPay();
    }
  }

  const progress = threshold > 0 ? Math.min(1, Math.abs(offset) / threshold) : 0;
  const armed = progress >= 1;

  return (
    <div
      // `select-none` só enquanto arrasta: sem isso o gesto de mouse pinta a
      // linha de texto selecionado, que parece defeito.
      className={cn("relative overflow-hidden rounded-lg", dragging && "select-none")}
      onPointerCancel={reset}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      ref={rootRef}
      // Sem isto o navegador engole o arrasto horizontal como rolagem.
      style={{ touchAction: "pan-y" }}
    >
      <div
        aria-hidden="true"
        className={cn(
          "absolute inset-0 flex items-center gap-2 px-5 text-sm font-semibold",
          paid
            ? "justify-start bg-background-hover text-text-secondary"
            : "justify-end bg-status-positive text-text-inverse",
        )}
        style={{ opacity: progress }}
      >
        {paid ? (
          <>
            <Undo2 className={cn("transition-transform", armed && "scale-110")} size={16} />
            Reabrir
          </>
        ) : (
          <>
            Pago
            <Check className={cn("transition-transform", armed && "scale-125")} size={18} />
          </>
        )}
      </div>

      {/* Piso opaco: os realces da linha (vencida, hoje, paga) são tintas
          translúcidas, e sem isto a faixa verde apareceria através do próprio
          conteúdo enquanto ele desliza. */}
      <div
        className="rounded-lg bg-background-card"
        style={{
          transform: `translate3d(${offset}px, 0, 0)`,
          transition: dragging ? "none" : "transform 220ms cubic-bezier(0.22, 1, 0.36, 1)",
        }}
      >
        {children}
      </div>
    </div>
  );
}
