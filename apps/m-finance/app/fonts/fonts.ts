import localFont from "next/font/local";

/**
 * Fontes auto-hospedadas da identidade Coded by M.
 * Antes eram carregadas via @import CSS externo (Fontshare/Google), o que
 * bloqueava renderização e quebrava offline no PWA. Agora são locais,
 * com preload automático do next/font.
 */

export const satoshi = localFont({
  src: [
    { path: "./satoshi-400.woff2", weight: "400", style: "normal" },
    { path: "./satoshi-500.woff2", weight: "500", style: "normal" },
    { path: "./satoshi-700.woff2", weight: "700", style: "normal" },
    { path: "./satoshi-900.woff2", weight: "900", style: "normal" },
  ],
  variable: "--font-satoshi",
  display: "swap",
});

export const panchang = localFont({
  src: [
    { path: "./panchang-500.woff2", weight: "500", style: "normal" },
    { path: "./panchang-600.woff2", weight: "600", style: "normal" },
    { path: "./panchang-700.woff2", weight: "700", style: "normal" },
  ],
  variable: "--font-panchang",
  display: "swap",
});

/** Variável (400–700). Subset latin cobre todo o português (U+0000–00FF). */
export const jetbrainsMono = localFont({
  src: "./jetbrains-var-latin.woff2",
  variable: "--font-jetbrains",
  display: "swap",
});
