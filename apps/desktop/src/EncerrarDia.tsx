/**
 * O End My Day 2.0: um clique resolve a maior parte.
 *
 * O cartão lê a proposta de encerramento — concluídas, abertas, vencidas, e o
 * que mover para amanhã — e oferece "Encerrar dia". Mover muda o dia
 * PLANEJADO, nunca o prazo. Quem quer o fluxo completo (humor, resumo,
 * destino de cada objetivo) abre o `EndMyDayFlow` pelo botão "Detalhar".
 */
import { useEffect, useRef, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { Icon } from "./Icon";
import { StateMessage } from "./Surface";
import type { DailyToday, PropostaDeEncerramento } from "./types";

export function EncerrarDia({ close, concluido, detalhar }: { close: () => void; concluido: (dia: DailyToday) => void; detalhar: () => void }) {
  const [proposta, setProposta] = useState<PropostaDeEncerramento | null>(null);
  const [erro, setErro] = useState("");
  const [mover, setMover] = useState<Set<string>>(new Set());
  const [salvando, setSalvando] = useState(false);
  const painel = useRef<HTMLDivElement>(null);

  useEffect(() => {
    void api.pilotoPropostaDeEncerramento()
      .then((p) => { setProposta(p); setMover(new Set(p.mover.map((m) => m.taskId))); })
      .catch((e) => setErro(String((e as { message?: string })?.message ?? e)));
  }, []);

  useEffect(() => {
    painel.current?.focus();
    function aoTeclar(evento: KeyboardEvent) { if (evento.key === "Escape") { evento.preventDefault(); close(); } }
    document.addEventListener("keydown", aoTeclar);
    return () => document.removeEventListener("keydown", aoTeclar);
  }, [close]);

  async function encerrar() {
    if (!proposta || salvando) return;
    setSalvando(true);
    try {
      const dia = await api.pilotoEncerrarDia({ mover: [...mover], resolutions: proposta.resolutions });
      concluido(dia);
      close();
    } catch (e) {
      setErro(String((e as { message?: string })?.message ?? e));
      setSalvando(false);
    }
  }

  return <>
    <button aria-hidden="true" className="attention-scrim" onClick={close} tabIndex={-1} type="button" />
    <div aria-label="Encerrar o dia" className="daily-flow encerrar-dia" ref={painel} role="dialog" tabIndex={-1}>
      <header className="daily-flow-head">
        <span className="micro-label">HOJE</span>
        <button type="button" className="icon-button" aria-label="Fechar" onClick={close}><Icon name="close" /></button>
      </header>
      <div className="daily-flow-body">
        {erro ? <StateMessage state="error" label="Não deu para ler o dia." detail={erro} /> : null}
        {!proposta && !erro ? <StateMessage state="loading" label="Lendo o dia..." /> : null}
        {proposta ? <>
          <ul className="piloto-linhas encerrar-placar">
            <li>✓ {proposta.concluidas} {proposta.concluidas === 1 ? "concluída" : "concluídas"}</li>
            <li>→ {proposta.abertas} {proposta.abertas === 1 ? "ficou aberta" : "ficaram abertas"}</li>
            {proposta.vencidas ? <li>⚠ {proposta.vencidas} {proposta.vencidas === 1 ? "vencida" : "vencidas"}</li> : null}
          </ul>
          {proposta.mover.length ? <>
            <span className="micro-label">SUGESTÃO · {proposta.sugestao.toUpperCase()}</span>
            <ul className="encerrar-mover">
              {proposta.mover.map((m) => <li key={m.taskId}>
                <label>
                  <input type="checkbox" checked={mover.has(m.taskId)} onChange={(e) => setMover((s) => { const n = new Set(s); if (e.currentTarget.checked) n.add(m.taskId); else n.delete(m.taskId); return n; })} />
                  <span>{m.titulo}</span>
                  <span className="piloto-mudo">{m.razao}</span>
                </label>
              </li>)}
            </ul>
            <p className="support-copy">Mover muda o dia planejado. O prazo de cada uma continua o mesmo.</p>
          </> : <p className="support-copy">{proposta.sugestao}</p>}
          <div className="form-actions">
            <Button variant="ghost" onClick={close}>Agora não</Button>
            <Button variant="ghost" onClick={detalhar}>Detalhar</Button>
            <Button variant="primary" disabled={salvando} onClick={() => void encerrar()}>{salvando ? "Encerrando..." : "Encerrar dia"}</Button>
          </div>
        </> : null}
      </div>
    </div>
  </>;
}
