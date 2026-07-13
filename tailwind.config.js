/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: ["class", '[data-theme="dark"]'],
  theme: {
    // Tokens mapeados para variaveis CSS (ver src/styles/tokens.css).
    // Isso mantem uma unica fonte de verdade e prepara o modo claro futuro.
    colors: {
      transparent: "transparent",
      current: "currentColor",
      bg: {
        DEFAULT: "var(--color-bg)",
        subtle: "var(--color-bg-subtle)",
      },
      surface: {
        DEFAULT: "var(--color-surface)",
        raised: "var(--color-surface-raised)",
        hover: "var(--color-surface-hover)",
      },
      border: {
        DEFAULT: "var(--color-border)",
        strong: "var(--color-border-strong)",
      },
      text: {
        DEFAULT: "var(--color-text)",
        muted: "var(--color-text-muted)",
        subtle: "var(--color-text-subtle)",
        inverted: "var(--color-text-inverted)",
      },
      accent: {
        DEFAULT: "var(--color-accent)",
        hover: "var(--color-accent-hover)",
        muted: "var(--color-accent-muted)",
        contrast: "var(--color-accent-contrast)",
      },
      running: "var(--color-state-running)",
      paused: "var(--color-state-paused)",
      stopped: "var(--color-state-stopped)",
      danger: {
        DEFAULT: "var(--color-danger)",
        muted: "var(--color-danger-muted)",
      },
      success: "var(--color-success)",
      warning: "var(--color-warning)",
    },
    borderRadius: {
      none: "0",
      sm: "var(--radius-sm)",
      DEFAULT: "var(--radius-md)",
      md: "var(--radius-md)",
      lg: "var(--radius-lg)",
      full: "9999px",
    },
    boxShadow: {
      none: "none",
      sm: "var(--shadow-sm)",
      DEFAULT: "var(--shadow-md)",
      md: "var(--shadow-md)",
      lg: "var(--shadow-lg)",
    },
    extend: {
      spacing: {
        nav: "var(--nav-width)",
      },
      fontFamily: {
        sans: "var(--font-sans)",
        mono: "var(--font-mono)",
        display: "var(--font-display)",
      },
      fontSize: {
        "2xs": ["0.6875rem", { lineHeight: "1rem" }],
      },
      transitionDuration: {
        fast: "var(--motion-fast)",
        base: "var(--motion-base)",
      },
    },
  },
  plugins: [],
};
