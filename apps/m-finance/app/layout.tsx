import type { Metadata, Viewport } from "next";
import { ServiceWorkerRegister } from "@/components/pwa/service-worker-register";
import "@/styles/globals.css";

export const metadata: Metadata = {
  title: "M Finance",
  description: "Cockpit financeiro pessoal de Matheus Mendes.",
  applicationName: "M Finance",
  manifest: "/manifest.webmanifest",
  icons: {
    icon: "/icon.svg",
    apple: "/apple-icon",
  },
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
    <html lang="pt-BR" data-scroll-behavior="smooth">
      <body>
        {children}
        <ServiceWorkerRegister />
      </body>
    </html>
  );
}
