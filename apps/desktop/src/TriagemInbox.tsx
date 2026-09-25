/**
 * A triagem sugerida da Inbox (ADR-084).
 *
 * A sugestão vem pronta do core (`mos_core::triagem`) — a mesma leitura
 * determinística da voz, sem LLM — e NADA é aplicado sozinho: a pessoa aplica
 * uma, ou revisa um lote pequeno e confirma. Cada item volta com resultado; o
 * que falhou diz por quê, e o que deu certo tem caminho de volta.
 */
import { useState } from "react";
import { api, appError } from "./api";
import { Button } from "./Button";
import { conversations, type UndoStep } from "./hermes";
import type { ResultadoDaTriagem, SugestaoDeTriagem } from "./types";

export function rotuloDaSugestao(s: SugestaoDeTriagem): string {
  switch (s.acao.acao) {
    case "referencia": return "Guardar como referência";
    case "task": return `Virar Task: ${s.acao.titulo}`;
    case "task_com_lembrete": {
      const quando = new Date(s.acao.quando).toLocaleString("pt-BR", { weekday: "short", day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" });
      return `Virar Task com lembrete (${quando}): ${s.acao.titulo}`;
    }
  }
}

/** Desfaz, na ordem, o que um lote aplicou. */
export async function desfazerLote(resultados: ResultadoDaTriagem[]) {
  for (const r of resultados) if (r.undo) await conversations.undoAction(r.undo as UndoStep);
}

/** O bloco do detalhe: a sugestão desta Capture, com o motivo e um botão. */
export function SugestaoDaCapture({ sugestao, aplicado }: { sugestao: SugestaoDeTriagem | undefined; aplicado: (resultados: ResultadoDaTriagem[]) => void }) {
  const [ocupado, setOcupado] = useState(false);
  const [erro, setErro] = useState("");
  if (!sugestao) return <section className="hermes-block" aria-label="Sugestão">
    <span className="micro-label">SUGESTÃO</span>
    <p className="hermes-empty">Nada a sugerir: parece uma nota. Classifique abaixo, se quiser — nada se perde.</p>
  </section>;
  async function aplicar() {
    if (!sugestao || ocupado) return;
    setOcupado(true);
    setErro("");
    try {
      const resultado = await api.inboxAplicar([sugestao]);
      if (resultado[0]?.ok) aplicado(resultado);
      else setErro(resultado[0]?.mensagem ?? "Não foi possível aplicar.");
    } catch (causa) {
      setErro(appError(causa).message);
    } finally {
      setOcupado(false);
    }
  }
  return <section className="hermes-block" aria-label="Sugestão">
    <span className="micro-label">SUGESTÃO{sugestao.confianca !== "high" ? " · CONFIRA" : ""}</span>
    <p>{rotuloDaSugestao(sugestao)}</p>
    <p className="support-copy">Por quê: {sugestao.razoes.join(" · ")}</p>
    {erro ? <p className="inline-error" role="alert">! {erro}</p> : null}
    <div className="button-line"><Button variant="primary" size="sm" disabled={ocupado} onClick={() => void aplicar()}>{ocupado ? "Aplicando…" : "Aplicar"}</Button></div>
  </section>;
}

/** Revisar e aplicar um lote. Confiança alta vem marcada; o resto, não. */
export function LoteDeSugestoes({ sugestoes, conteudoDe, fechar, aplicado }: {
  sugestoes: SugestaoDeTriagem[];
  conteudoDe: (captureId: string) => string;
  fechar: () => void;
  aplicado: (resultados: ResultadoDaTriagem[]) => void;
}) {
  const [marcadas, setMarcadas] = useState<Set<string>>(() => new Set(sugestoes.filter((s) => s.confianca === "high").map((s) => s.captureId)));
  const [ocupado, setOcupado] = useState(false);
  const [resultados, setResultados] = useState<ResultadoDaTriagem[] | null>(null);
  const [erro, setErro] = useState("");
  async function aplicar() {
    if (ocupado) return;
    setOcupado(true);
    setErro("");
    try {
      const r = await api.inboxAplicar(sugestoes.filter((s) => marcadas.has(s.captureId)));
      setResultados(r);
      aplicado(r);
    } catch (causa) {
      setErro(appError(causa).message);
    } finally {
      setOcupado(false);
    }
  }
  const falhas = resultados?.filter((r) => !r.ok) ?? [];
  return <section className="triagem-lote" aria-label="Sugestões para a Inbox">
    <header><span className="micro-label">SUGESTÕES · {sugestoes.length}</span><Button variant="ghost" size="sm" onClick={fechar}>Fechar</Button></header>
    {resultados ? <>
      <p className="support-copy">{resultados.length - falhas.length} {resultados.length - falhas.length === 1 ? "aplicada" : "aplicadas"}{falhas.length ? `, ${falhas.length} não` : ""}.</p>
      {falhas.length ? <ul className="triagem-falhas">{falhas.map((f) => <li key={f.captureId}>! {conteudoDe(f.captureId)} — {f.mensagem}</li>)}</ul> : null}
    </> : <>
      <ul className="triagem-itens">
        {sugestoes.map((s) => <li key={s.captureId}>
          <label>
            <input type="checkbox" checked={marcadas.has(s.captureId)} onChange={(e) => { const marcado = e.currentTarget.checked; setMarcadas((m) => { const n = new Set(m); if (marcado) n.add(s.captureId); else n.delete(s.captureId); return n; }); }} />
            <span className="triagem-texto"><span>{rotuloDaSugestao(s)}</span><span className="support-copy">{conteudoDe(s.captureId)} · {s.razoes.join(" · ")}</span></span>
          </label>
        </li>)}
      </ul>
      {erro ? <p className="inline-error" role="alert">! {erro}</p> : null}
      <div className="button-line"><Button variant="primary" size="sm" disabled={ocupado || marcadas.size === 0} onClick={() => void aplicar()}>{ocupado ? "Aplicando…" : `Aplicar ${marcadas.size}`}</Button></div>
    </>}
  </section>;
}
