"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import { CheckCircle2, Info, TriangleAlert, X } from "lucide-react";
import { cn } from "@/lib/utils";

type ToastVariant = "success" | "error" | "info";

type Toast = {
  id: number;
  message: string;
  variant: ToastVariant;
};

type ToastContextValue = {
  addToast: (message: string, variant?: ToastVariant) => void;
};

const ToastContext = createContext<ToastContextValue | null>(null);

const VARIANT_STYLES: Record<
  ToastVariant,
  { icon: typeof Info; ring: string; iconColor: string }
> = {
  success: {
    icon: CheckCircle2,
    ring: "border-status-positive/40",
    iconColor: "text-status-positive",
  },
  error: {
    icon: TriangleAlert,
    ring: "border-accent-border",
    iconColor: "text-accent",
  },
  info: {
    icon: Info,
    ring: "border-border-default",
    iconColor: "text-text-muted",
  },
};

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const counter = useRef(0);
  const timers = useRef<Map<number, ReturnType<typeof setTimeout>>>(new Map());

  const removeToast = useCallback((id: number) => {
    setToasts((current) => current.filter((toast) => toast.id !== id));
    const timer = timers.current.get(id);
    if (timer) {
      clearTimeout(timer);
      timers.current.delete(id);
    }
  }, []);

  const addToast = useCallback(
    (message: string, variant: ToastVariant = "info") => {
      counter.current += 1;
      const id = counter.current;
      setToasts((current) => [...current, { id, message, variant }]);
      const timer = setTimeout(() => removeToast(id), 4000);
      timers.current.set(id, timer);
    },
    [removeToast],
  );

  useEffect(() => {
    const map = timers.current;
    return () => {
      map.forEach((timer) => clearTimeout(timer));
      map.clear();
    };
  }, []);

  return (
    <ToastContext.Provider value={{ addToast }}>
      {children}
      <div
        aria-live="polite"
        className="pointer-events-none fixed inset-x-0 bottom-0 z-50 flex flex-col items-center gap-2 px-4 pb-[calc(5.5rem+env(safe-area-inset-bottom))] sm:items-end sm:pb-6 sm:pr-6 lg:pb-8 lg:pr-8"
      >
        {toasts.map((toast) => {
          const { icon: Icon, ring, iconColor } = VARIANT_STYLES[toast.variant];
          return (
            <div
              className={cn(
                "pointer-events-auto flex w-full max-w-sm items-start gap-3 rounded-lg border bg-background-card/95 p-3.5 shadow-xl shadow-black/30 backdrop-blur-sm",
                ring,
              )}
              key={toast.id}
              role="status"
            >
              <Icon className={cn("mt-0.5 shrink-0", iconColor)} size={18} aria-hidden="true" />
              <p className="flex-1 text-sm leading-5 text-text-secondary">{toast.message}</p>
              <button
                aria-label="Fechar"
                className="focus-ring shrink-0 rounded text-text-muted transition hover:text-text-primary"
                onClick={() => removeToast(toast.id)}
                type="button"
              >
                <X size={16} />
              </button>
            </div>
          );
        })}
      </div>
    </ToastContext.Provider>
  );
}

export function useToast() {
  const context = useContext(ToastContext);
  if (!context) {
    throw new Error("useToast deve ser usado dentro de ToastProvider.");
  }
  return context;
}
