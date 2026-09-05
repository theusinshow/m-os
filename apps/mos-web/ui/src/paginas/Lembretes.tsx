import { pedeAtencao, type Lembrete } from "../api";
import { daquiA } from "../instantes";
import { Vazio } from "../componentes/Vazio";

export type VistaDosLembretes = "abertos" | "resolvidos";

/** A palavra curta do estado, para a linha da lista. */
const PALAVRA: Partial<Record<Lembrete["status"], string>> = {
  snoozed: "adiado",
  missed: "perdido",
  delivered: "avisado",
  completed: "concluído",
  cancelled: "cancelado",
  expired: "expirou",
};

/**
 * Os lembretes, em três grupos.
 *
 * # Por que agrupar, e não só ordenar
 *
 * Uma lista ordenada por hora responde *quando*, e a pergunta que traz alguém
 * aqui é *o que cobra*. Com o vencido no meio da fila, entre o de ontem e o de
 * quinta, ele lê como mais um item — e é o único que exige ação agora.
 *
 * O terceiro grupo é o que ainda vai vencer. Ele fica por último e apagado: é
 * informação, não chamado.
 */
export function Lembretes({
  lembretes,
  resolvidos,
  vista,
  ocupado,
  aoTrocarVista,
  aoAbrir,
  aoResolver,
}: {
  lembretes: Lembrete[];
  resolvidos: Lembrete[];
  vista: VistaDosLembretes;
  ocupado: boolean;
  aoTrocarVista: (vista: VistaDosLembretes) => void;
  aoAbrir: (lembrete: Lembrete) => void;
  aoResolver: (lembrete: Lembrete, como: "concluir" | "cancelar") => void;
}) {
  const cobrando = lembretes.filter(pedeAtencao);
  const adiados = lembretes.filter((l) => l.status === "snoozed");
  const proximos = lembretes.filter(
    (l) => !pedeAtencao(l) && l.status !== "snoozed",
  );

  return (
    <div className="lembretes">
      <div className="agenda-vista">
        {(["abertos", "resolvidos"] as const).map((opcao) => (
          <button
            key={opcao}
            type="button"
            aria-pressed={vista === opcao}
            onClick={() => aoTrocarVista(opcao)}
          >
            {opcao === "abertos" ? "ABERTOS" : "RESOLVIDOS"}
          </button>
        ))}
      </div>

      {vista === "resolvidos" ? (
        resolvidos.length === 0 ? (
          <Vazio frase="Nada resolvido ainda. O que você concluir ou cancelar fica guardado aqui." />
        ) : (
          <Grupo
            titulo="JÁ RESOLVIDOS"
            itens={resolvidos}
            ocupado={ocupado}
            aoAbrir={aoAbrir}
            aoResolver={aoResolver}
            comAcoes={false}
          />
        )
      ) : lembretes.length === 0 ? (
        <Vazio frase="Nenhum lembrete esperando. Escreva embaixo, ou toque no sino de uma Task." />
      ) : (
        <>
          <Grupo
            titulo="COBRANDO AGORA"
            itens={cobrando}
            ocupado={ocupado}
            aoAbrir={aoAbrir}
            aoResolver={aoResolver}
          />
          <Grupo
            titulo="ADIADOS"
            itens={adiados}
            ocupado={ocupado}
            aoAbrir={aoAbrir}
            aoResolver={aoResolver}
          />
          <Grupo
            titulo="AINDA VÊM"
            itens={proximos}
            ocupado={ocupado}
            aoAbrir={aoAbrir}
            aoResolver={aoResolver}
            apagado
          />
        </>
      )}
    </div>
  );
}

/** Um grupo. Vazio, ele não aparece: título de seção sem itens é ruído. */
function Grupo({
  titulo,
  itens,
  ocupado,
  apagado,
  comAcoes = true,
  aoAbrir,
  aoResolver,
}: {
  titulo: string;
  itens: Lembrete[];
  ocupado: boolean;
  apagado?: boolean;
  comAcoes?: boolean;
  aoAbrir: (lembrete: Lembrete) => void;
  aoResolver: (lembrete: Lembrete, como: "concluir" | "cancelar") => void;
}) {
  if (itens.length === 0) return null;
  return (
    <section data-apagado={apagado || undefined}>
      <h2 className="secao">
        <span>{titulo}</span>
        <b>{itens.length}</b>
      </h2>
      <ul className="lista">
        {itens.map((lembrete) => (
          <li
            className="item"
            key={lembrete.id}
            data-cobra={(comAcoes && pedeAtencao(lembrete)) || undefined}
          >
            {/* A linha inteira abre o detalhe. Os botões ficam fora dela, e não
                dentro: um botão dentro de um alvo maior faz o toque na borda
                cair no alvo errado — e aqui o alvo errado conclui um lembrete
                que a pessoa só queria ler. */}
            <button className="linha-destino" type="button" onClick={() => aoAbrir(lembrete)}>
              <div className="item-corpo">
                <p>{lembrete.title}</p>
                <small>
                  {daquiA(lembrete.nextDueAt)}
                  {PALAVRA[lembrete.status] ? ` · ${PALAVRA[lembrete.status]}` : ""}
                  {lembrete.target?.type === "task" ? " · task" : ""}
                  {lembrete.snoozeCount > 0 ? ` · adiado ${lembrete.snoozeCount}×` : ""}
                </small>
              </div>
            </button>
            {comAcoes ? (
              <div className="item-acoes">
                <button
                  className="acao"
                  type="button"
                  disabled={ocupado}
                  aria-label={`Concluir ${lembrete.title}`}
                  onClick={() => aoResolver(lembrete, "concluir")}
                >
                  Feito
                </button>
              </div>
            ) : null}
          </li>
        ))}
      </ul>
    </section>
  );
}
