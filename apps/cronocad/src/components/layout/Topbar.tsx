import { Moon, Sun } from "lucide-react";
import { useUiStore } from "@/stores/uiStore";
import { Button } from "@/components/ui/Button";
import { formatDate } from "@/lib/format";

/** Barra superior com data atual e alternador de tema (prepara modo claro). */
export function Topbar() {
  const theme = useUiStore((s) => s.theme);
  const toggleTheme = useUiStore((s) => s.toggleTheme);

  // Data exibida apenas para contexto; calculos de dominio ficam no backend.
  const today = formatDate(new Date().toISOString());

  return (
    <header className="no-print flex h-14 shrink-0 items-center justify-between border-b border-border bg-bg px-6">
      <span className="text-sm text-text-muted tabular">{today}</span>
      <Button
        variant="ghost"
        size="sm"
        onClick={toggleTheme}
        aria-label={theme === "dark" ? "Ativar modo claro" : "Ativar modo escuro"}
        icon={
          theme === "dark" ? (
            <Sun size={16} strokeWidth={1.75} />
          ) : (
            <Moon size={16} strokeWidth={1.75} />
          )
        }
      />
    </header>
  );
}
