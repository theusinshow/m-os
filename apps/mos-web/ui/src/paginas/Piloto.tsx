import { useState } from "react";
import { api, type Piloto as Panorama } from "../api";
import type { Pagina } from "../navegacao";
import { EncerrarDia } from "./EncerrarDia";
import { Resgate } from "./Resgate";

/**
 * O piloto no bolso: o que faço agora, o dia por começar, o que precisa de
 * atenção. Abre a Home — antes dos cartões —, porque é a pergunta que se faz
 * ao tirar o telefone do bolso.
 *
 * Só desenha. A recomendação, a proposta e a lista vêm prontas do `/api/piloto`,
 * do MESMO motor que o desktop usa. Se o celular e o PC discordassem sobre "o
 * que faço agora", um dos dois estaria errado; assim nenhum pode estar.
 */
export function Piloto({
  panorama,
  aoAbrirTask,
  aoAtualizar,
  aoIr,
  aoAbrirLembrete,
}: {
  panorama: Panorama | null;
  aoAbrirTask: (id: string) => void;
  aoAtualizar: () => Promise<void>;
  aoIr: (pagina: Pagina) => void;
  aoAbrirLembrete: (id: string) => void;
}) {
  const [ocupado, setOcupado] = useState<string | null>(null);
  const [porque, setPorque] = useState(false);
  const [erro, setErro] = useState("");
  const [encerrando, setEncerrando] = useState(false);
  /** Resgate aberto (plano na tela) ou adiado nesta visita. "Agora não" não
   *  grava nada: a retomada volta na próxima abertura, sem cobrança. */
  const [resgate, setResgate] = useState<"fechado" | "aberto" | "adiado">("fechado");
  const [diaDispensado, setDiaDispensado] = useState(() => {
    try { return localStorage.getItem("mos-dia-dispensado") ?? ""; } catch { return ""; }
  });
  if (!panorama) return null;

  async function correr(chave: string, acao: () => Promise<unknown>) {
    if (ocupado) return;
    setOcupado(chave);
    setErro("");
    try {
      await acao();
      try { await aoAtualizar(); } catch {
        setErro("Ação salva. Não foi possível atualizar o painel; atualize a página.");
      }
    } catch {
      setErro("Não foi possível completar a ação. Confira a conexão e tente novamente.");
    } finally {
      setOcupado(null);
    }
  }

  const diaNaoIniciado = panorama.estadoDoDia.kind === "not_started" || panorama.estadoDoDia.kind === "stale_open";
  const mostrarDia = diaNaoIniciado && diaDispensado !== panorama.day;
  const agora = panorama.agora.agora;
  const atencao = panorama.atencao.filter((i) => i.tipo !== "day_not_started").slice(0, 3);
  const diaAberto = panorama.estadoDoDia.kind === "active";

  function destino(item: Panorama["atencao"][number]): (() => void) | null {
    if (item.alvo.kind === "task") return () => aoAbrirTask(item.alvo.id);
    if (item.alvo.kind === "reminder") return () => aoAbrirLembrete(item.alvo.id);
    if (item.alvo.kind.startsWith("academic_")) return () => aoIr("academico");
    if (item.tipo === "unprocessed_capture") return () => aoIr("fazer");
    if (item.tipo === "unsynced_changes") return () => aoIr("mais");
    if (item.tipo === "unfinished_day" && diaAberto) return () => setEncerrando(true);
    return null;
  }

  return (
    <section className="piloto" aria-label="O piloto">
      {erro ? <p role="alert" className="piloto-cartao">{erro}</p> : null}
      {panorama.resgate && resgate === "aberto" ? (
        <Resgate
          dias={panorama.resgate.dias}
          aoFechar={() => setResgate("adiado")}
          aoAplicar={aoAtualizar}
          aoRevisarInbox={() => aoIr("fazer")}
        />
      ) : panorama.resgate && resgate === "fechado" ? (
        <div className="piloto-cartao">
          <span className="piloto-rotulo">{panorama.resgate.dias} {panorama.resgate.dias === 1 ? "DIA" : "DIAS"} FORA</span>
          <p className="piloto-frase">Retomar sem reorganizar tudo</p>
          <p className="piloto-mudo">Poucas decisões, a sugestão já escolhida. O resto fica como está.</p>
          <div className="piloto-acoes">
            <button type="button" className="botao" onClick={() => setResgate("aberto")}>Retomar</button>
            <button type="button" className="botao" data-variante="quieto" onClick={() => setResgate("adiado")}>Agora não</button>
          </div>
        </div>
      ) : null}
      {mostrarDia ? (
        <div className="piloto-cartao piloto-dia">
          <span className="piloto-rotulo">{panorama.estadoDoDia.kind === "stale_open" ? `O DIA ${panorama.estadoDoDia.day} FICOU ABERTO` : "UMA PROPOSTA PARA HOJE"}</span>
          <p className="piloto-frase">Hoje importa</p>
          {panorama.proposta ? (
            <ul className="piloto-lista">
              {panorama.proposta.principal ? <li>□ {panorama.proposta.principal.draft.title}</li> : null}
              {panorama.proposta.secundarios.map((s) => <li key={s.draft.title}>□ {s.draft.title}</li>)}
              {panorama.proposta.agenda.slice(0, 2).map((l) => <li key={l.at} className="piloto-mudo">{l.hora} — {l.titulo}</li>)}
            </ul>
          ) : null}
          <div className="piloto-acoes">
            <button type="button" className="botao" disabled={ocupado !== null} onClick={() => void correr("dia", () => api.iniciarDia())}>
              {ocupado === "dia" ? "Preparando..." : "Começar meu dia"}
            </button>
            <button type="button" className="botao" data-variante="quieto" onClick={() => { setDiaDispensado(panorama.day); try { localStorage.setItem("mos-dia-dispensado", panorama.day); } catch { /* sem armazenamento */ } }}>Agora não</button>
          </div>
        </div>
      ) : null}

      <div className="piloto-cartao">
        <span className="piloto-rotulo">AGORA</span>
        {agora ? (
          <>
            <button type="button" className="piloto-titulo" onClick={() => aoAbrirTask(agora.taskId)}>
              {agora.comecada ? <span className="piloto-ponto" aria-hidden="true">● </span> : null}{agora.titulo}
            </button>
            <p className="piloto-mudo">{[agora.projeto, agora.estimativa].filter(Boolean).join(" · ")}</p>
            <div className="piloto-acoes">
              {agora.comecada ? (
                <>
                  <button type="button" className="botao" disabled={ocupado !== null} onClick={() => void correr(agora.taskId, () => api.mudarEstado(agora.taskId, "done"))}>✓ Concluir</button>
                  <button type="button" className="botao" data-variante="quieto" disabled={ocupado !== null} onClick={() => void correr(agora.taskId, () => api.pararTask(agora.taskId))}>Continuar depois</button>
                </>
              ) : (
                <button type="button" className="botao" disabled={ocupado !== null} onClick={() => void correr(agora.taskId, () => api.comecarTask(agora.taskId))}>Começar</button>
              )}
              <button type="button" className="piloto-porque" aria-expanded={porque} onClick={() => setPorque(!porque)}>Por quê?</button>
            </div>
            {porque ? <ul className="piloto-lista piloto-mudo">{agora.razoes.map((r) => <li key={r}>• {r}</li>)}</ul> : null}
          </>
        ) : (
          <p className="piloto-mudo">{panorama.agora.vazio ?? "Nada precisa da sua atenção agora."}</p>
        )}
      </div>

      {encerrando && diaAberto ? (
        <EncerrarDia aoFechar={() => setEncerrando(false)} aoEncerrar={aoAtualizar} />
      ) : null}

      {atencao.length ? (
        <div className="piloto-cartao">
          <span className="piloto-rotulo">PRECISA DE ATENÇÃO</span>
          <ul className="piloto-itens">
            {atencao.map((item) => (
              <li key={`${item.tipo}:${item.alvo.id}:${item.titulo}`} data-severidade={item.severidade}>
                <div className="piloto-item-linha">
                  <button type="button" className="piloto-item" onClick={() => destino(item)?.()} disabled={!destino(item)}>
                    <span>{item.titulo}</span>
                    <span className="piloto-mudo">{item.descricao || item.razoes[0]}</span>
                  </button>
                  {item.chave ? (
                    <button type="button" className="piloto-dispensar" aria-label={`Dispensar: ${item.titulo}`} disabled={ocupado !== null} onClick={() => void correr(`dispensar:${item.chave}`, () => api.dispensarAtencao(item.chave!))}>×</button>
                  ) : null}
                </div>
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {diaAberto && !encerrando ? (
        <div className="piloto-acoes piloto-rodape">
          <button type="button" className="botao" data-variante="quieto" onClick={() => setEncerrando(true)}>Encerrar o dia</button>
        </div>
      ) : null}
    </section>
  );
}
