import type { Config } from "tailwindcss";

const config: Config = {
  content: [
    "./app/**/*.{ts,tsx}",
    "./components/**/*.{ts,tsx}",
    "./lib/**/*.{ts,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        background: {
          primary: "#020A06",
          secondary: "#091009",
          card: "#091009",
          hover: "#0D150F",
          elevated: "#0B120D",
        },
        text: {
          primary: "#F5F2ED",
          secondary: "#E8E4DE",
          muted: "#8A8780",
          inverse: "#020A06",
        },
        accent: {
          DEFAULT: "#FB3640",
          hover: "#ff4b54",
          soft: "rgba(251, 54, 64, 0.12)",
          border: "rgba(251, 54, 64, 0.35)",
        },
        status: {
          positive: "#54D18A",
          fair: "#D8B45A",
          tight: "#D98245",
          negative: "#FB3640",
        },
        border: {
          subtle: "rgba(245, 242, 237, 0.08)",
          DEFAULT: "rgba(245, 242, 237, 0.14)",
          strong: "rgba(245, 242, 237, 0.24)",
        },
      },
      borderRadius: {
        sm: "8px",
        md: "12px",
        lg: "16px",
        xl: "24px",
      },
      fontFamily: {
        sans: ["var(--font-satoshi)", "Satoshi", "Segoe UI", "system-ui", "sans-serif"],
        display: ["var(--font-panchang)", "Satoshi", "Segoe UI", "system-ui", "sans-serif"],
      },
    },
  },
  plugins: [],
};

export default config;
