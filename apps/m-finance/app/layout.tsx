import type { Metadata, Viewport } from "next";
// As fontes locais (Satoshi, Panchang) deixaram de ser carregadas na unificação
// do design system: `globals.css` mapeia `--font-sans/display/mono` para os
// tokens do M/OS, e nada mais referencia `--font-satoshi`. O merge do agente de
// WhatsApp trouxe o carregamento de volta, e mantê-lo baixaria duas famílias
// para desenhar com uma. `app/fonts/fonts.ts` e os arquivos continuam no
// repositório — são asset do M-Finance, e removê-los é decisão de outra hora.
import { ServiceWorkerRegister } from "@/components/pwa/service-worker-register";
import "@/styles/globals.css";

export const metadata: Metadata = {
  title: "M Finance",
  description: "Cockpit financeiro pessoal de Matheus Mendes.",
  applicationName: "M Finance",
  manifest: "/manifest.webmanifest",
  // Sem bloco `icons`: `app/icon.svg` e `app/apple-icon.png` sao convencao de
  // arquivo, e o Next emite as tags sozinho. O bloco explicito apontava para
  // `/apple-icon`, a rota que o antigo `apple-icon.tsx` servia — mante-lo
  // deixaria o link quebrado agora que a marca virou PNG estatico.
  appleWebApp: {
    capable: true,
    statusBarStyle: "black-translucent",
    title: "M Finance",
  },
};

export const viewport: Viewport = {
  themeColor: "#020A06",
  colorScheme: "dark",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="pt-BR"
      data-scroll-behavior="smooth"
    >
      <body>
        {children}
        <ServiceWorkerRegister />
      </body>
    </html>
  );
}
