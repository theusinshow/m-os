/**
 * O Rescue Mode: a sequência guiada de quando a pessoa ficou dias fora.
 *
 * Um passo por vez, poucas decisões por passo, a sugestão já marcada. O plano
 * vem pronto do `piloto_resgate`; aqui só se escolhe entre a ação sugerida e
 * as alternativas, e no fim tudo é aplicado de uma vez. Nada é gravado até o
 * último passo — fechar no meio não muda nada.
 */
import { useEffect, useMemo, useRef, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { Icon } from "./Icon";
import { StateMessage } from "./Surface";
import type { AcaoDeResgate, PlanoDeResgate } from "./types";

function rotulo(acao: AcaoDeResgate): string {
  switch (acao.acao) {
    case "planejar": return `Planejar para ${acao.para.split("-").reverse().slice(0, 2).join("/")}`;
    case "backlog": return "Voltar ao backlog";
    case "arquivar": return "Arquivar";
    case "concluir": return "Já fiz";
    case "cobrar": return "Cobrar hoje";
    case "processar": return "Processar";
    case "arquivar_capture": return "Arquivar";
    case "concluir_lembrete": return "Concluir";
    case "adiar_lembrete": return "Adiar para amanhã";
    case "abrir_academico": return "Abrir";
  }
}

const chaveDe = (acao: AcaoDeResgate) => JSON.stringify(acao);

export function RescueMode({ close, concluido, abrirInbox }: { close: () => void; concluido: () => void; abrirInbox: () => void }) {
  const [plano, setPlano] = useState<PlanoDeResgate | null>(null);
  const [erro, setErro] = useState("");
  const [passo, setPasso] = useState(0);
  /** A escolha por item: a chave do item → a ação escolhida. Ausente = sugerida. */
  const [escolhas, setEscolhas] = useState<Map<string, AcaoDeResgate | null>>(new Map());
  const [salvando, setSalvando] = useState(false);
  const [fim, setFim] = useState<{ aplicadas: number; falharam: string[] } | null>(null);
  const painel = useRef<HTMLDivElement>(null);

  useEffect(() => {
    void api.pilotoResgate().then((p) => { setPlano(p); if (!p) close(); }).catch((e) => setErro(String((e as { message?: string })?.message ?? e)));
  }, [close]);

  useEffect(() => {
    painel.current?.focus();
    function aoTeclar(evento: KeyboardEvent) { if (evento.key === "Escape") { evento.preventDefault(); close(); } }
    document.addEventListener("keydown", aoTeclar);
    return () => document.removeEventListener("keydown", aoTeclar);
  }, [close]);

  const atual = plano?.passos[passo];
  const total = plano?.passos.length ?? 0;

  const acoesFinais = useMemo(() => {
    if (!plano) return [] as AcaoDeResgate[];
    const lista: AcaoDeResgate[] = [];
    for (const p of plano.passos) {
      for (const item of p.itens) {
        const chave = chaveDe(item.sugerida);
        const escolha = escolhas.has(chave) ? escolhas.get(chave) : item.sugerida;
        if (escolha) lista.push(escolha);
      }
    }
    return lista;
  }, [plano, escolhas]);

  async function aplicar() {
    if (salvando) return;
    setSalvando(true);
    try {
      const r = await api.pilotoResgateAplicar(acoesFinais);
      await api.pilotoResgateConcluir();
      setFim(r);
      concluido();
    } catch (e) {
      setErro(String((e as { message?: string })?.message ?? e));
      setSalvando(false);
    }
  }

  return <>
    <button aria-hidden="true" className="attention-scrim" onClick={close} tabIndex={-1} type="button" />
    <div aria-label="Organizar para mim" className="daily-flow rescue" ref={painel} role="dialog" tabIndex={-1}>
      <header className="daily-flow-head">
        <span className="micro-label">{fim ? "PRONTO" : atual ? `${passo + 1}/${total} — ${atual.titulo.toUpperCase()}` : "ORGANIZAR PARA MIM"}</span>
        <button type="button" className="icon-button" aria-label="Fechar" onClick={close}><Icon name="close" /></button>
      </header>
      <div className="daily-flow-body">
        {erro ? <StateMessage state="error" label="Não deu para montar o plano." detail={erro} /> : null}
        {!plano && !erro ? <StateMessage state="loading" label="Lendo o que acumulou..." /> : null}

        {fim ? <>
          <p className="piloto-titulo">{plano?.fecho}</p>
          {fim.falharam.length ? <p className="support-copy">{fim.falharam.length} não puderam ser aplicadas: {fim.falharam.join("; ")}</p> : null}
          <div className="form-actions"><Button variant="primary" onClick={close}>Ver meu dia</Button></div>
        </> : null}

        {!fim && atual ? <>
          <ul className="rescue-itens">
            {atual.itens.map((item) => {
              const chave = chaveDe(item.sugerida);
              const escolha: AcaoDeResgate | null = escolhas.has(chave) ? (escolhas.get(chave) ?? null) : item.sugerida;
              const opcoes = [item.sugerida, ...item.alternativas];
              return <li key={chave} data-ignorado={escolha === null || undefined}>
                <div className="piloto-item-texto">
                  <span className="piloto-item-titulo">{item.titulo}</span>
                  <span className="piloto-mudo">{[item.descricao, ...item.razoes].filter(Boolean).join(" · ")}</span>
                </div>
                <div className="rescue-opcoes" role="group" aria-label={`O que fazer com ${item.titulo}`}>
                  {opcoes.map((o) => <button key={chaveDe(o)} type="button" className="rescue-opcao" aria-pressed={escolha !== null && chaveDe(escolha) === chaveDe(o)} onClick={() => {
                    if (o.acao === "processar") { abrirInbox(); return; }
                    setEscolhas((m) => new Map(m).set(chave, o));
                  }}>{rotulo(o)}</button>)}
                  <button type="button" className="rescue-opcao" aria-pressed={escolha === null} onClick={() => setEscolhas((m) => new Map(m).set(chave, null))}>Deixar</button>
                </div>
              </li>;
            })}
          </ul>
          {atual.restantes > 0 ? <p className="piloto-mudo">+ {atual.restantes} ficam para a próxima rodada — poucas decisões por vez.</p> : null}
          <div className="form-actions">
            {passo > 0 ? <Button variant="ghost" onClick={() => setPasso(passo - 1)}>Voltar</Button> : <Button variant="ghost" onClick={close}>Agora não</Button>}
            {passo + 1 < total ? <Button variant="primary" onClick={() => setPasso(passo + 1)}>Próximo</Button> : <Button variant="primary" disabled={salvando} onClick={() => void aplicar()}>{salvando ? "Organizando..." : `Aplicar ${acoesFinais.length} ${acoesFinais.length === 1 ? "decisão" : "decisões"}`}</Button>}
          </div>
        </> : null}

        {!fim && plano && total === 0 ? <>
          <p className="piloto-titulo">Nada acumulou. Está tudo em ordem.</p>
          <div className="form-actions"><Button variant="primary" onClick={() => void api.pilotoResgateConcluir().then(() => { concluido(); close(); })}>Fechar</Button></div>
        </> : null}
      </div>
    </div>
  </>;
}
