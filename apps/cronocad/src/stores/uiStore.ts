/**
 * Estado de interface (nao persistente / derivado).
 *
 * O banco e a fonte da verdade do dominio (secao 21). Este store guarda apenas
 * preferencias de apresentacao, como o tema, que e persistido em localStorage
 * para respeitar a escolha do usuario entre sessoes. Inicia em modo escuro.
 */

import { create } from "zustand";

export type Theme = "dark" | "light";

const STORAGE_KEY = "cronocad.theme";

function loadTheme(): Theme {
  if (typeof localStorage === "undefined") return "dark";
  const saved = localStorage.getItem(STORAGE_KEY);
  return saved === "light" || saved === "dark" ? saved : "dark";
}

function applyTheme(theme: Theme): void {
  if (typeof document !== "undefined") {
    document.documentElement.setAttribute("data-theme", theme);
  }
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(STORAGE_KEY, theme);
  }
}

interface UiState {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
}

export const useUiStore = create<UiState>((set, get) => ({
  theme: loadTheme(),
  setTheme: (theme) => {
    applyTheme(theme);
    set({ theme });
  },
  toggleTheme: () => {
    const next: Theme = get().theme === "dark" ? "light" : "dark";
    applyTheme(next);
    set({ theme: next });
  },
}));
