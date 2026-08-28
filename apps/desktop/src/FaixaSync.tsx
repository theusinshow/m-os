/**
 * A faixa de sincronizacao da Home. So desenha.
 *
 * Quem DECIDE — se aparece, qual dos estados, e que frase — e o `syncFaixa.ts`,
 * onde tambem mora a justificativa de esta faixa ser a unica excecao ao
 * principio de que tudo na Home se arruma.
 */
import { useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { MosSymbol } from "./Symbol";
import { estadoDaFaixa } from "./syncFaixa";
import type { SyncStatus } from "./types";

export function FaixaSync({ status, onChanged }: { status: SyncStatus | null; onChanged: () => void }) {
  const [ocupado, setOcupado] = useState(false);
  const faixa = estadoDaFaixa(status);
  if (!faixa) return null;

  async function agora() {
    setOcupado(true);
    // O erro nao e engolido: ele volta pelo `sync-changed` e vira a faixa de
    // erro, com o motivo cru. Capturar aqui so evita o unhandled rejection.
    try { await api.syncNow(); } catch { /* a faixa conta */ }
    setOcupado(false);
    onChanged();
  }

  async function dispensar() {
    try { await api.syncDismissSummary(); } catch { /* nada a fazer */ }
    onChanged();
  }

  const girando = faixa.girando || ocupado;

  return <section className="sync-faixa" data-tipo={faixa.tipo} aria-live="polite">
    <div className="sync-faixa-texto">
      <span className="micro-label">{faixa.titulo}</span>
      <p>{faixa.corpo}</p>
    </div>
    <div className="sync-faixa-acoes">
      {girando ? <MosSymbol size={16} spinning /> : null}
      {faixa.dispensavel
        ? <Button variant="ghost" size="sm" onClick={() => void dispensar()}>Dispensar</Button>
        : <Button variant="secondary" size="sm" disabled={girando} onClick={() => void agora()}>Tentar agora</Button>}
    </div>
  </section>;
}
