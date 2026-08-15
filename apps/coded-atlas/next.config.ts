import type { NextConfig } from "next";
import path from "node:path";

const nextConfig: NextConfig = {
  // Há um package-lock.json solto em C:\Dev que o Next confunde como raiz do
  // workspace. Fixar a raiz no diretório do projeto silencia o aviso e garante
  // que o build trace os arquivos certos.
  outputFileTracingRoot: path.resolve(process.cwd()),

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  webpack(config: any, { dev }: { dev: boolean }) {
    if (dev) {
      // No Windows o cache em disco pode corromper (rename atômico falha).
      // Memória é mais lenta no cold start mas nunca quebra entre requests.
      config.cache = { type: "memory" };
    }
    return config;
  },
};

export default nextConfig;
