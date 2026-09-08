import { pedeAtencao, type Lembrete } from "../api";
import { daquiA } from "../instantes";
import { Esqueleto } from "../componentes/Esqueleto";
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
  carregando,
  aoTrocarVista,
  aoAbrir,
  aoResolver,
}: {
  lembretes: Lembrete[];
  resolvidos: Lembrete[];
  vista: VistaDosLembretes;
  ocupado: boolean;
  carregando?: boolean;
  aoTrocarVista: (vista: VistaDosLembretes) => void;
  aoAbrir: (lembrete: Lembrete) => void;
  aoResolver: (lembrete: Lembrete, como: "concluir" | "cancelar") => void;
}) {
  const cobrando = lembretes.filter(pedeAtencao);
  const adiados = lembretes.filter((l) => l.status === "snoozed");
  // Sem data e um grupo proprio, e no fim: "algum dia" nao e um lembrete
  // atrasado nem um que vem — e misturado com os que tem hora, ele so faria a
  // lista parecer maior do que o que ela cobra.
  const algumDia = lembretes.filter(
    (l) => !pedeAtencao(l) && l.status !== "snoozed" && !l.nextDueAt,
  );
  const proximos = lembretes.filter(
    (l) => !pedeAtencao(l) && l.status !== "snoozed" && l.nextDueAt,
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

      {carregando && lembretes.length === 0 && resolvidos.length === 0 ? (
        <Esqueleto />
      ) : vista === "resolvidos" ? (
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
          <Grupo
            titulo="ALGUM DIA"
            itens={algumDia}
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
                {/* Um follow-up faz outra PERGUNTA: "concluir" nao e a resposta
                    que ele quer, e a linha nao pode fingir que e. */}
                {lembrete.kind === "follow_up" ? (
                  <p className="item-pergunta">
                    {lembrete.waitingFor.trim()
                      ? `${lembrete.waitingFor.trim()} respondeu?`
                      : "Já respondeu?"}
                  </p>
                ) : null}
                <small>
                  {lembrete.nextDueAt ? daquiA(lembrete.nextDueAt) : "sem data"}
                  {PALAVRA[lembrete.status] ? ` · ${PALAVRA[lembrete.status]}` : ""}
                  {lembrete.target?.type === "task" ? " · task" : ""}
                  {lembrete.snoozeCount > 0 ? ` · adiado ${lembrete.snoozeCount}×` : ""}
                  {/* Nunca so cor: `DESIGN-FOUNDATIONS.md` §14 pede que nenhum
                      estado dependa dela, e por isso a insistencia e a
                      repeticao entram como PALAVRA nesta linha. */}
                  {lembrete.persistent ? " · não deixar esquecer" : ""}
                  {lembrete.recurrence ? " · repete" : ""}
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
                  {lembrete.kind === "follow_up" ? "Respondeu" : "Feito"}
                </button>
              </div>
            ) : null}
          </li>
        ))}
      </ul>
    </section>
  );
}
