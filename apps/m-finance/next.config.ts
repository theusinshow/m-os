import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  typedRoutes: false,
  // A versão que este build carrega para o navegador. `/api/version` responde a
  // que está no ar; quando as duas divergem, a aba está velha e recarrega
  // sozinha (ADR-091).
  env: {
    NEXT_PUBLIC_BUILD_ID: process.env.VERCEL_GIT_COMMIT_SHA ?? "dev",
  },
  turbopack: {
    root: process.cwd(),
  },
};

export default nextConfig;
