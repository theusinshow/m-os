/**
 * O Sync Health: diagnóstico, não administração de banco.
 *
 * O selo do cabeçalho (`SyncChip`) é a resposta calma; esta folha é o que abre
 * ao clicar nele. Mostra estado, última rodada, fila, retry, conflitos e os
 * aparelhos conhecidos — e oferece "Tentar agora" e o reparo. O que decide a
 * frase de cada estado é o `piloto.ts`, com teste.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { Icon } from "./Icon";
import { fraseDeErroDeSync, proximaTentativa, seloDeSync } from "./piloto";
import { relativeTime } from "./relativeTime";
import type { AparelhoNaMalha, SaudeDoSync } from "./types";

export function SyncChip({ saude, abrir }: { saude: SaudeDoSync | null; abrir: () => void }) {
  const selo = seloDeSync(saude?.estado ?? null, saude?.registro.ultimoOkEm ?? null);
  if (selo.tom === "mudo") return null;
  return <button type="button" className="sync-chip" data-tom={selo.tom} title={selo.detalhe} onClick={abrir} aria-label={`Sincronização: ${selo.texto}. Abrir diagnóstico.`}>
    <span className="sync-chip-icone" aria-hidden="true">{selo.icone}</span>
    <span>{selo.texto}</span>
  </button>;
}

export function SyncHealth({ close, abrirAjustes, inicial = null }: { close: () => void; abrirAjustes: () => void; /** Só a bancada passa: evita a ida ao Rust. */ inicial?: SaudeDoSync | null }) {
  const [saude, setSaude] = useState<SaudeDoSync | null>(inicial);
  const [malha, setMalha] = useState<AparelhoNaMalha[]>([]);
  const [ocupado, setOcupado] = useState<"" | "sync" | "reparo">("");
  const [mensagem, setMensagem] = useState("");
  const painel = useRef<HTMLDivElement>(null);

  const recarregar = useCallback(() => {
    if (inicial) return;
    void api.syncHealth().then(setSaude).catch(() => undefined);
    void api.syncMalha().then(setMalha).catch(() => setMalha([]));
  }, [inicial]);

  useEffect(() => {
    recarregar();
    painel.current?.focus();
    function aoTeclar(evento: KeyboardEvent) {
      if (evento.key === "Escape") { evento.preventDefault(); close(); }
    }
    document.addEventListener("keydown", aoTeclar);
    return () => document.removeEventListener("keydown", aoTeclar);
  }, [close, recarregar]);

  async function tentarAgora() {
    setOcupado("sync");
    setMensagem("");
    try {
      const r = await api.syncNow();
      setMensagem(r.error ? `Parou em: ${r.error}` : `${r.sent} enviadas · ${r.received} recebidas${r.conflicts ? ` · ${r.conflicts} conflitos` : ""}`);
    } catch (e) {
      setMensagem(String((e as { message?: string })?.message ?? e));
    }
    setOcupado("");
    recarregar();
  }

  async function reparar() {
    setOcupado("reparo");
    try {
      const r = await api.syncReparar();
      setMensagem(`Reparo: ${r.reparadas} materializadas, ${r.falharam.length} ainda dependem de algo, ${r.abandonadas.length} abandonadas.`);
    } catch (e) {
      setMensagem(String((e as { message?: string })?.message ?? e));
    }
    setOcupado("");
    recarregar();
  }

  const selo = seloDeSync(saude?.estado ?? null, saude?.registro.ultimoOkEm ?? null);
  const erro = saude ? fraseDeErroDeSync(saude) : "";
  const proxima = saude ? proximaTentativa(saude.registro) : "";
  const vistoNoHub = (id: string) => malha.find((a) => a.id === id)?.vistoEm;

  return <>
    <button aria-hidden="true" className="attention-scrim" onClick={close} tabIndex={-1} type="button" />
    <div aria-label="Sync Health" className="daily-flow sync-health" ref={painel} role="dialog" tabIndex={-1}>
      <header className="daily-flow-head">
        <span className="micro-label">SYNC HEALTH</span>
        <button type="button" className="icon-button" aria-label="Fechar" onClick={close}><Icon name="close" /></button>
      </header>
      <div className="daily-flow-body">
        <p className="sync-health-estado" data-tom={selo.tom}><span aria-hidden="true">{selo.icone}</span> {selo.texto}</p>
        {selo.tom === "mudo" ? <>
          <p className="support-copy">O M/OS funciona inteiro sem sincronizar. Ligue quando quiser que o bolso e o PC se encontrem.</p>
          <div className="form-actions"><Button variant="primary" onClick={abrirAjustes}>Configurar em Ajustes</Button></div>
        </> : null}
        {erro ? <p className="support-copy sync-health-erro">{erro}{proxima && saude?.estado.kind === "offline" ? ` Próxima tentativa ${proxima}.` : ""}</p> : null}

        {saude && selo.tom !== "mudo" ? <dl className="fact-grid">
          <div><dt>ÚLTIMA OK</dt><dd>{saude.registro.ultimoOkEm ? relativeTime(saude.registro.ultimoOkEm) : <span className="fact-empty">Nunca</span>}</dd></div>
          <div><dt>PENDENTES</dt><dd>{saude.pendentes}</dd></div>
          <div><dt>EM RETRY</dt><dd>{saude.emRetry}</dd></div>
          <div><dt>CONFLITOS</dt><dd>{saude.conflitosAbertos}</dd></div>
          {saude.registro.falhasSeguidas > 0 ? <div><dt>FALHAS SEGUIDAS</dt><dd>{saude.registro.falhasSeguidas}</dd></div> : null}
          {saude.registro.ultimoErro && saude.registro.tipoDoErro ? <div><dt>ÚLTIMO ERRO</dt><dd className="sync-health-cru">{saude.registro.ultimoErro}</dd></div> : null}
        </dl> : null}

        {saude && saude.conflitosAbertos > 0 ? <p className="support-copy">Conflitos são edições do mesmo campo em dois aparelhos. A mais recente venceu; a outra ficou guardada. <button type="button" className="piloto-link" onClick={() => void api.syncReconhecerConflitos().then(recarregar)}>Marcar como vistos</button></p> : null}

        {saude && saude.dispositivos.length > 0 ? <>
          <span className="micro-label">APARELHOS</span>
          <ul className="malha">
            {saude.dispositivos.map((d) => {
              const noHub = vistoNoHub(d.id);
              return <li key={d.id} data-divergente={d.appVersion !== saude.appVersion || undefined}>
                <span className="malha-nome">{d.name}</span>
                <span className="malha-versao">{d.appVersion || "—"}</span>
                <span className="malha-visto">{d.isThisDevice ? (d.lastSyncAt ? `este aparelho · sync ${relativeTime(d.lastSyncAt)}` : "este aparelho") : noHub ? `visto ${relativeTime(noHub)}` : d.lastSyncAt ? `sync ${relativeTime(d.lastSyncAt)}` : "nunca visto"}</span>
              </li>;
            })}
            {malha.filter((a) => !saude.dispositivos.some((d) => d.id === a.id)).map((a) => <li key={a.id}>
              <span className="malha-nome">{a.nome}</span>
              <span className="malha-versao">{a.versao}</span>
              <span className="malha-visto">visto {relativeTime(a.vistoEm)}</span>
            </li>)}
          </ul>
        </> : null}

        {mensagem ? <p className="support-copy" role="status">{mensagem}</p> : null}
        {selo.tom !== "mudo" ? <div className="form-actions">
          <Button variant="ghost" disabled={ocupado !== ""} onClick={() => void reparar()}>{ocupado === "reparo" ? "Reparando..." : "Reparar"}</Button>
          <Button variant="ghost" onClick={abrirAjustes}>Diagnóstico</Button>
          <Button variant="primary" disabled={ocupado !== ""} onClick={() => void tentarAgora()}>{ocupado === "sync" ? "Sincronizando..." : "Tentar agora"}</Button>
        </div> : null}
      </div>
    </div>
  </>;
}
