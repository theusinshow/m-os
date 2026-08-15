import type { MetadataRoute } from "next";

export default function manifest(): MetadataRoute.Manifest {
  return {
    name: "M Finance",
    short_name: "M Finance",
    description: "Cockpit financeiro pessoal — contas, vencimentos e faturas do mês.",
    start_url: "/app/dashboard",
    scope: "/",
    display: "standalone",
    orientation: "portrait",
    lang: "pt-BR",
    background_color: "#020A06",
    theme_color: "#020A06",
    categories: ["finance", "productivity"],
    icons: [
      { src: "/icon.svg", sizes: "any", type: "image/svg+xml", purpose: "any" },
      { src: "/maskable.svg", sizes: "any", type: "image/svg+xml", purpose: "maskable" },
    ],
  };
}
