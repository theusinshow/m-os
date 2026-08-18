import type { CookieOptions } from "@supabase/ssr";

/**
 * Opcoes extras para o cookie de sessao, por causa do embed.
 *
 * O M-Finance e exibido dentro de um iframe no M/OS (ADR-039), e ali quem
 * ocupa o topo e o app Tauri — `http://localhost:1420` em dev, `tauri://` no
 * empacotado. Do ponto de vista do navegador, todo request daquele iframe para
 * ca e cross-site. O padrao do `@supabase/ssr` e `SameSite=Lax`, que manda o
 * cookie ser gravado mas NAO ser enviado nesse cenario: a sessao existia, so
 * nunca chegava de volta, e cada retorno a aba Finance caia no login outra vez.
 *
 * - `sameSite: "none"` faz o cookie viajar no iframe.
 * - `secure` e obrigatorio junto de `none`, e o site ja e https.
 * - `partitioned` (CHIPS) mantem tudo funcionando quando o Chrome desliga
 *   cookies de terceiros. A particao e a do site do topo, que aqui e sempre o
 *   mesmo app — entao a sessao continua durando os 400 dias de sempre.
 *
 * O custo e ampliar a superficie de CSRF, ja que o cookie passa a ir junto em
 * request cross-site. As mutacoes do app sao Server Actions, que o Next protege
 * conferindo Origin contra Host, e o app tem um dono so.
 */
export const cookieDoEmbed = {
  sameSite: "none",
  secure: true,
  partitioned: true,
} satisfies Partial<CookieOptions> & { partitioned: boolean };
