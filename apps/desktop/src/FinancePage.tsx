const FINANCE_URL = "https://m-finance-silk.vercel.app";

/**
 * M-Finance continua Next.js/Postgres/Vercel (ADR-032) — isto so exibe a
 * mesma URL dentro da janela do M/OS em vez de abrir no navegador padrao.
 * Sessao de login fica na propria webview; nao ha passagem de token.
 */
export function FinancePage() {
  return (
    <div className="page finance-page">
      <iframe
        className="finance-frame"
        src={FINANCE_URL}
        title="M Finance"
        allow="clipboard-write"
      />
    </div>
  );
}
