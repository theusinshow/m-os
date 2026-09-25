import { useEffect, useMemo, useState } from "react";
import { api, type AcaoDeResgate, type PlanoDeResgate, type ResgateResultado } from "../api";

/**
 * O Rescue Mode no bolso (MOS-OVERHAUL-PLAN F2.2).
 *
 * O plano vem pronto do mesmo motor do desktop (`mos_core::plano_de_resgate`):
 * poucos itens por passo, a sugestão já escolhida e o motivo de cada um. Nada
 * é gravado até o fim — fechar no meio não muda nada —, e o que ficar de fora
 * não é arquivado por envelhecer: "Deixar" é uma resposta válida.
 *
 * A aplicação usa `mos_core::aplicar_resgate`. Falha parcial aparece apontando
 * o item, e o resto do lote continua aplicado.
 */
function rotulo(acao: AcaoDeResgate): string {
  switch (acao.acao) {
    case "planejar": return `Fazer ${acao.para.split("-").reverse().slice(0, 2).join("/")}`;
    case "backlog": return "Depois";
    case "arquivar": return "Arquivar";
    case "concluir": return "Já fiz";
    case "cobrar": return "Cobrar hoje";
    case "processar": return "Revisar";
    case "arquivar_capture": return "Arquivar";
    case "concluir_lembrete": return "Concluir";
    case "adiar_lembrete": return "Amanhã";
    case "abrir_academico": return "Abrir";
  }
}

const chaveDe = (acao: AcaoDeResgate) => JSON.stringify(acao);

export function Resgate({ dias, aoFechar, aoAplicar, aoRevisarInbox }: {
  dias: number;
  aoFechar: () => void;
  aoAplicar: () => Promise<void>;
  aoRevisarInbox: () => void;
}) {
  const [plano, setPlano] = useState<PlanoDeResgate | null | undefined>(undefined);
  const [erro, setErro] = useState("");
  const [passo, setPasso] = useState(0);
  /** Chave da sugestão → ação escolhida. Ausente = a sugerida; `null` = deixar. */
  const [escolhas, setEscolhas] = useState<Map<string, AcaoDeResgate | null>>(new Map());
  const [salvando, setSalvando] = useState(false);
  const [fim, setFim] = useState<ResgateResultado | null>(null);

  useEffect(() => {
    let vivo = true;
    api.resgate()
      .then((p) => { if (vivo) setPlano(p); })
      .catch(() => { if (vivo) setErro("Não deu para montar o plano. Confira a conexão e tente de novo."); });
    return () => { vivo = false; };
  }, []);

  /** As ações na ordem em que vão para o servidor, com o título de cada uma
   *  — é o que permite apontar QUAL item falhou pelo índice da resposta. */
  const lote = useMemo(() => {
    const lista: { acao: AcaoDeResgate; titulo: string }[] = [];
    for (const p of plano?.passos ?? []) {
      for (const item of p.itens) {
        const chave = chaveDe(item.sugerida);
        const escolha = escolhas.has(chave) ? escolhas.get(chave) : item.sugerida;
        if (escolha && escolha.acao !== "processar" && escolha.acao !== "abrir_academico") {
          lista.push({ acao: escolha, titulo: item.titulo });
        }
      }
    }
    return lista;
  }, [plano, escolhas]);

  async function aplicar() {
    if (salvando) return;
    setSalvando(true);
    setErro("");
    try {
      const resultado = await api.aplicarResgate(lote.map((l) => l.acao));
      await api.concluirResgate();
      setFim(resultado);
      try { await aoAplicar(); } catch { /* o painel mostra o próprio aviso */ }
    } catch {
      setErro("Não foi possível aplicar. Nada foi perdido; tente de novo.");
    } finally {
      setSalvando(false);
    }
  }

  async function dispensar() {
    try { await api.concluirResgate(); } catch { /* sem rede: volta a aparecer, o que é o certo */ }
    aoFechar();
  }

  const atual = plano?.passos[passo];
  const total = plano?.passos.length ?? 0;

  return (
    <div className="piloto-cartao piloto-resgate" role="region" aria-label="Retomar depois de dias fora">
      <span className="piloto-rotulo">
        {fim ? "PRONTO" : atual ? `${passo + 1}/${total} · ${atual.titulo.toUpperCase()}` : `${dias} ${dias === 1 ? "DIA" : "DIAS"} FORA`}
      </span>
      {erro ? <p role="alert" className="piloto-mudo">{erro}</p> : null}
      {plano === undefined && !erro ? <p className="piloto-mudo">Lendo o que acumulou...</p> : null}

      {fim ? (
        <>
          <p className="piloto-frase">{plano?.fecho || "Retomado."}</p>
          {fim.falhas.length ? (
            <ul className="piloto-lista">
              {fim.falhas.map((f) => <li key={f.indice}>! {lote[f.indice]?.titulo ?? "Um item"}: {f.motivo}</li>)}
            </ul>
          ) : null}
          <div className="piloto-acoes"><button type="button" className="botao" onClick={aoFechar}>Ver meu dia</button></div>
        </>
      ) : null}

      {!fim && plano === null ? (
        <>
          <p className="piloto-frase">Nada acumulou. Está tudo em ordem.</p>
          <div className="piloto-acoes"><button type="button" className="botao" onClick={() => void dispensar()}>Fechar</button></div>
        </>
      ) : null}

      {!fim && atual ? (
        <>
          <ul className="piloto-itens">
            {atual.itens.map((item) => {
              const chave = chaveDe(item.sugerida);
              const escolha = escolhas.has(chave) ? (escolhas.get(chave) ?? null) : item.sugerida;
              const opcoes = [item.sugerida, ...item.alternativas];
              return (
                <li key={chave} className="piloto-resgate-item" data-ignorado={escolha === null || undefined}>
                  <span>{item.titulo}</span>
                  <span className="piloto-mudo">{[item.descricao, ...item.razoes].filter(Boolean).join(" · ")}</span>
                  <div className="piloto-opcoes" role="group" aria-label={`O que fazer com ${item.titulo}`}>
                    {opcoes.map((o) => (
                      <button
                        key={chaveDe(o)}
                        type="button"
                        className="piloto-opcao"
                        aria-pressed={escolha !== null && chaveDe(escolha) === chaveDe(o)}
                        onClick={() => {
                          if (o.acao === "processar") { aoRevisarInbox(); return; }
                          setEscolhas((m) => new Map(m).set(chave, o));
                        }}
                      >{rotulo(o)}</button>
                    ))}
                    <button type="button" className="piloto-opcao" aria-pressed={escolha === null} onClick={() => setEscolhas((m) => new Map(m).set(chave, null))}>Deixar</button>
                  </div>
                </li>
              );
            })}
          </ul>
          {atual.restantes > 0 ? <p className="piloto-mudo">+ {atual.restantes} ficam para depois — poucas decisões por vez.</p> : null}
          <div className="piloto-acoes">
            {passo + 1 < total ? (
              <button type="button" className="botao" onClick={() => setPasso(passo + 1)}>Próximo</button>
            ) : (
              <button type="button" className="botao" disabled={salvando} onClick={() => void aplicar()}>
                {salvando ? "Aplicando..." : `Aplicar ${lote.length} ${lote.length === 1 ? "decisão" : "decisões"}`}
              </button>
            )}
            {passo > 0 ? (
              <button type="button" className="botao" data-variante="quieto" onClick={() => setPasso(passo - 1)}>Voltar</button>
            ) : (
              <button type="button" className="botao" data-variante="quieto" onClick={aoFechar}>Agora não</button>
            )}
          </div>
        </>
      ) : null}
    </div>
  );
}
