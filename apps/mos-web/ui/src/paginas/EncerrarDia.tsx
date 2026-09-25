import { useEffect, useState } from "react";
import { api, type PropostaDeEncerramento } from "../api";

/**
 * Encerrar o dia no bolso (MOS-OVERHAUL-PLAN F2.1).
 *
 * A mesma proposta do desktop — `mos_core::propor_encerramento` —, desenhada
 * para o toque: o placar, o que mover para amanhã já marcado, e um botão.
 * Encerrar não pede texto; humor e resumo continuam opcionais no desktop.
 * Mover muda o dia PLANEJADO, nunca o prazo.
 *
 * Repetir depois de um timeout é seguro: o servidor devolve o dia já
 * encerrado em vez de adiar de novo.
 */
export function EncerrarDia({ aoFechar, aoEncerrar }: { aoFechar: () => void; aoEncerrar: () => Promise<void> }) {
  const [proposta, setProposta] = useState<PropostaDeEncerramento | null>(null);
  const [mover, setMover] = useState<Set<string>>(new Set());
  const [erro, setErro] = useState("");
  const [salvando, setSalvando] = useState(false);

  useEffect(() => {
    let vivo = true;
    api.propostaDeEncerramento()
      .then((p) => { if (vivo) { setProposta(p); setMover(new Set(p.mover.map((m) => m.taskId))); } })
      .catch(() => { if (vivo) setErro("Não deu para ler o dia. Confira a conexão e tente de novo."); });
    return () => { vivo = false; };
  }, []);

  async function encerrar() {
    if (!proposta || salvando) return;
    setSalvando(true);
    setErro("");
    try {
      await api.encerrarDia([...mover], proposta.resolutions);
    } catch {
      setErro("Não foi possível encerrar o dia. Tentar de novo não adia nada duas vezes.");
      setSalvando(false);
      return;
    }
    try { await aoEncerrar(); } catch { /* o painel mostra o próprio aviso */ }
    aoFechar();
  }

  return (
    <div className="piloto-cartao piloto-encerrar" role="region" aria-label="Encerrar o dia">
      <span className="piloto-rotulo">ENCERRAR O DIA</span>
      {erro ? <p role="alert" className="piloto-mudo">{erro}</p> : null}
      {!proposta && !erro ? <p className="piloto-mudo">Lendo o dia...</p> : null}
      {proposta ? (
        <>
          <ul className="piloto-lista">
            <li>✓ {proposta.concluidas} {proposta.concluidas === 1 ? "concluída" : "concluídas"}</li>
            <li>→ {proposta.abertas} {proposta.abertas === 1 ? "ficou aberta" : "ficaram abertas"}</li>
            {proposta.vencidas ? <li>⚠ {proposta.vencidas} {proposta.vencidas === 1 ? "vencida" : "vencidas"}</li> : null}
          </ul>
          {proposta.mover.length ? (
            <>
              <p className="piloto-mudo">{proposta.sugestao}</p>
              <ul className="piloto-escolhas">
                {proposta.mover.map((m) => (
                  <li key={m.taskId}>
                    <label>
                      <input
                        type="checkbox"
                        checked={mover.has(m.taskId)}
                        onChange={(e) => {
                          const marcado = e.currentTarget.checked;
                          setMover((atual) => { const novo = new Set(atual); if (marcado) novo.add(m.taskId); else novo.delete(m.taskId); return novo; });
                        }}
                      />
                      <span>{m.titulo}</span>
                    </label>
                  </li>
                ))}
              </ul>
              <p className="piloto-mudo">Mover muda o dia planejado. O prazo continua o mesmo.</p>
            </>
          ) : <p className="piloto-mudo">{proposta.sugestao}</p>}
          <div className="piloto-acoes">
            <button type="button" className="botao" disabled={salvando} onClick={() => void encerrar()}>
              {salvando ? "Encerrando..." : "Encerrar dia"}
            </button>
            <button type="button" className="botao" data-variante="quieto" disabled={salvando} onClick={aoFechar}>Agora não</button>
          </div>
        </>
      ) : erro ? (
        <div className="piloto-acoes">
          <button type="button" className="botao" data-variante="quieto" onClick={aoFechar}>Fechar</button>
        </div>
      ) : null}
    </div>
  );
}
