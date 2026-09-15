import { useState } from "react";
import { api, type Piloto as Panorama } from "../api";

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
}: {
  panorama: Panorama | null;
  aoAbrirTask: (id: string) => void;
  aoAtualizar: () => Promise<void>;
}) {
  const [ocupado, setOcupado] = useState<string | null>(null);
  const [porque, setPorque] = useState(false);
  const [diaDispensado, setDiaDispensado] = useState(() => {
    try { return localStorage.getItem("mos-dia-dispensado") ?? ""; } catch { return ""; }
  });
  if (!panorama) return null;

  async function correr(chave: string, acao: () => Promise<unknown>) {
    if (ocupado) return;
    setOcupado(chave);
    try { await acao(); } catch { /* o panorama conta na próxima leitura */ }
    setOcupado(null);
    await aoAtualizar();
  }

  const diaNaoIniciado = panorama.estadoDoDia.kind === "not_started" || panorama.estadoDoDia.kind === "stale_open";
  const mostrarDia = diaNaoIniciado && diaDispensado !== panorama.day;
  const agora = panorama.agora.agora;
  const atencao = panorama.atencao.filter((i) => i.tipo !== "day_not_started").slice(0, 3);

  return (
    <section className="piloto" aria-label="O piloto">
      {mostrarDia ? (
        <div className="piloto-cartao piloto-dia">
          <span className="piloto-rotulo">SEU DIA AINDA NÃO FOI INICIADO</span>
          <p className="piloto-frase">Posso montar tudo automaticamente.</p>
          {panorama.proposta ? (
            <ul className="piloto-lista">
              {panorama.proposta.principal ? <li>□ {panorama.proposta.principal.draft.title}</li> : null}
              {panorama.proposta.secundarios.map((s) => <li key={s.draft.title}>□ {s.draft.title}</li>)}
              {panorama.proposta.agenda.slice(0, 2).map((l) => <li key={l.at} className="piloto-mudo">{l.hora} — {l.titulo}</li>)}
            </ul>
          ) : null}
          <div className="piloto-acoes">
            <button type="button" className="botao" disabled={ocupado === "dia"} onClick={() => void correr("dia", () => api.iniciarDia())}>
              {ocupado === "dia" ? "Montando..." : "Montar meu dia"}
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
                  <button type="button" className="botao" disabled={ocupado === agora.taskId} onClick={() => void correr(agora.taskId, () => api.mudarEstado(agora.taskId, "done"))}>✓ Concluir</button>
                  <button type="button" className="botao" data-variante="quieto" disabled={ocupado === agora.taskId} onClick={() => void correr(agora.taskId, () => api.pararTask(agora.taskId))}>Continuar depois</button>
                </>
              ) : (
                <button type="button" className="botao" disabled={ocupado === agora.taskId} onClick={() => void correr(agora.taskId, () => api.comecarTask(agora.taskId))}>Começar</button>
              )}
              <button type="button" className="piloto-porque" aria-expanded={porque} onClick={() => setPorque(!porque)}>Por quê?</button>
            </div>
            {porque ? <ul className="piloto-lista piloto-mudo">{agora.razoes.map((r) => <li key={r}>• {r}</li>)}</ul> : null}
          </>
        ) : (
          <p className="piloto-mudo">{panorama.agora.vazio ?? "Nada precisa da sua atenção agora."}</p>
        )}
      </div>

      {atencao.length ? (
        <div className="piloto-cartao">
          <span className="piloto-rotulo">PRECISA DE ATENÇÃO</span>
          <ul className="piloto-itens">
            {atencao.map((item) => (
              <li key={`${item.tipo}:${item.alvo.id}:${item.titulo}`} data-severidade={item.severidade}>
                <button type="button" className="piloto-item" onClick={() => { if (item.alvo.kind === "task") aoAbrirTask(item.alvo.id); }} disabled={item.alvo.kind !== "task"}>
                  <span>{item.titulo}</span>
                  <span className="piloto-mudo">{item.descricao || item.razoes[0]}</span>
                </button>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </section>
  );
}
