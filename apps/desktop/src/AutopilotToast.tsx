/**
 * O aviso in-app do Autopilot, com as opções de adiar.
 *
 * Chega pelo evento `autopilot-aviso` — o backend já decidiu que vale um
 * aviso (dedupe, cooldown, silêncio, teto). Aqui só se desenha e se responde:
 * a ação principal, adiar por uma das opções, ou fechar. Fechar não resolve
 * nada; a coisa continua no "Precisa de atenção" da Home.
 */
import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { Button } from "./Button";
import type { AvisoDoAutopilot } from "./types";

export function useAutopilotAvisos() {
  const [aviso, setAviso] = useState<AvisoDoAutopilot | null>(null);
  useEffect(() => {
    const parar = listen<AvisoDoAutopilot>("autopilot-aviso", (evento) => setAviso(evento.payload));
    return () => { void parar.then((dispose) => dispose()); };
  }, []);
  return { aviso, fechar: () => setAviso(null) };
}

export function AutopilotToast({ aviso, fechar, agir }: { aviso: AvisoDoAutopilot; fechar: () => void; agir: (aviso: AvisoDoAutopilot) => void }) {
  const [adiando, setAdiando] = useState(false);

  useEffect(() => {
    // Some sozinho em meio minuto, SEM resolver: a coisa continua na Home. O
    // que não some é o que pede decisão — e mesmo esse cede ao Escape.
    const timer = window.setTimeout(fechar, 30_000);
    return () => window.clearTimeout(timer);
  }, [aviso, fechar]);

  async function adiar(minutos: number | null) {
    if (minutos === null) { setAdiando(true); return; }
    try { await api.autopilotAdiar(aviso.chave, minutos); } catch { /* o próximo tick reavalia */ }
    fechar();
  }

  return <div className="attention-toast autopilot-toast" role="status" data-tipo={aviso.tipo}>
    <div>
      <span className="micro-label">AUTOPILOT</span>
      <strong>{aviso.titulo}</strong>
      {aviso.corpo ? <p>{aviso.corpo}</p> : null}
    </div>
    {adiando ? <div className="autopilot-adiar" role="group" aria-label="Adiar por">
      {aviso.adiar.filter((o) => o.minutos !== null).map((o) => <button key={o.rotulo} type="button" className="rescue-opcao" onClick={() => void adiar(o.minutos)}>{o.rotulo}</button>)}
      <button type="button" className="rescue-opcao" onClick={() => setAdiando(false)}>Voltar</button>
    </div> : <div className="form-actions">
      <Button variant="ghost" size="sm" onClick={fechar}>Fechar</Button>
      {aviso.adiar.length ? <Button variant="ghost" size="sm" onClick={() => { if (aviso.adiar.length === 1) void adiar(aviso.adiar[0].minutos); else setAdiando(true); }}>Mais tarde</Button> : null}
      {aviso.acaoPrincipal ? <Button variant="primary" size="sm" onClick={() => { agir(aviso); fechar(); }}>{aviso.acaoPrincipal}</Button> : null}
    </div>}
  </div>;
}
