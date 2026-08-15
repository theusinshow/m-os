import { useEffect, type ReactNode } from "react";
import { useUiStore } from "@/stores/uiStore";
import { useAppSync } from "@/hooks/useAppSync";

/**
 * Provedores globais da aplicacao: aplica o tema atual e dispara a
 * sincronizacao de estado com o backend (catalogo, sessoes e cronometro).
 */
export function AppProviders({ children }: { children: ReactNode }) {
  const theme = useUiStore((s) => s.theme);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  useAppSync();

  return <>{children}</>;
}
