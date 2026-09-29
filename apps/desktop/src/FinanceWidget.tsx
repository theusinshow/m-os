import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { finance } from "./finance";
import { diaCurto, perguntaDoInsight, resumoDaHome, type FinanceHomeSummary } from "./financeHome";
import { moneyOf } from "./TempoShared";

/** De quanto em quanto tempo a Home relê o M-Finance, com a tela aberta. */
const RELEITURA_MS = 10 * 60 * 1000;

/**
 * O resumo financeiro da Home, lido pelo Rust — o renderer recebe os números,
 * nunca o secret (ADR-073).
 *
 * `null` enquanto não há leitura: sem M-Finance configurado o widget fica
 * indisponível, e a Home não gasta espaço dizendo que falta configuração.
 */
export function useFinanceHome() {
  const [resumo, setResumo] = useState<FinanceHomeSummary | null>(null);
  const [erro, setErro] = useState("");

  const ler = useCallback(() => {
    finance
      .homeSummary()
      .then((leitura) => {
        setResumo(resumoDaHome(leitura.data, leitura.asOf));
        setErro("");
      })
      .catch((error: unknown) => {
        const texto = typeof error === "string" ? error : "";
        // Não configurado não é erro: é o widget não ter o que mostrar.
        if (texto === "not_configured") setResumo(null);
        else setErro(texto || "O M-Finance não respondeu.");
      });
  }, []);

  useEffect(() => {
    ler();
    const intervalo = window.setInterval(ler, RELEITURA_MS);
    // Uma ação do Hermes no M-Finance muda os números: relê na hora.
    const parar = listen<string>("data-changed", (event) => {
      if (event.payload === "finance") ler();
    });
    return () => {
      window.clearInterval(intervalo);
      void parar.then((desligar) => desligar());
    };
  }, [ler]);

  return { resumo, erro };
}

/**
 * O corpo do widget FINANÇAS: o Safe-to-Spend com a sobra ao lado, o próximo
 * vencimento e o alerta mais grave. O Safe-to-Spend nunca aparece sozinho —
 * um número que não se reconstrói não serve para decidir.
 */
export function FinanceWidget({
  resumo,
  erro,
  abrirFinance,
  perguntarAoHermes,
}: {
  resumo: FinanceHomeSummary;
  erro: string;
  abrirFinance: () => void;
  perguntarAoHermes: (rascunho: string) => void;
}) {
  return (
    <div className="finance-widget">
      <div className="finance-widget-col">
        <span className="micro-label">PODE GASTAR COM SEGURANÇA</span>
        <strong className="finance-widget-value" data-negative={(resumo.safeToSpendCents ?? 0) < 0 || undefined}>
          {resumo.status === "unknown" || resumo.safeToSpendCents === null ? "—" : moneyOf(resumo.safeToSpendCents)}
        </strong>
        <small>
          {resumo.status === "unknown"
            ? "Sem receita lançada para calcular."
            : `Sobra ${moneyOf(resumo.accountingRemainingCents ?? 0)} − ${moneyOf(resumo.deductionsCents)} protegidos`}
        </small>
      </div>
      <div className="finance-widget-col">
        <span className="micro-label">{resumo.proximo?.overdue ? "VENCIDO" : "PRÓXIMO"}</span>
        {resumo.proximo ? (
          <button type="button" className="data-row" data-stale={resumo.proximo.overdue || undefined} onClick={abrirFinance}>
            <span className="row-copy">
              <strong>{resumo.proximo.label}</strong>
              <small>
                {moneyOf(resumo.proximo.amountCents)}
                {resumo.proximo.estimated ? " · estimado" : ""}
              </small>
            </span>
            <span className="row-meta">{diaCurto(resumo.proximo.dueDate)}</span>
          </button>
        ) : (
          <p className="empty-state">Nada vencendo em 30 dias.</p>
        )}
      </div>
      <div className="finance-widget-col">
        <span className="micro-label">ATENÇÃO</span>
        {resumo.atencao ? (
          <button
            type="button"
            className="data-row"
            data-severity={resumo.atencao.severity}
            title="Abre o Hermes com a pergunta pronta. Nada é enviado sozinho."
            onClick={() => perguntarAoHermes(perguntaDoInsight(resumo.atencao!))}
          >
            <span className="row-copy">
              <strong>{resumo.atencao.title}</strong>
              <small>Ver análise</small>
            </span>
          </button>
        ) : (
          <p className="empty-state">Nada material agora.</p>
        )}
        {erro ? <small className="finance-widget-error">{erro}</small> : null}
      </div>
    </div>
  );
}
