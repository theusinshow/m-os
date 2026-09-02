import { pedeAtencao, type Lembrete } from "../api";
import { daquiA } from "../instantes";
import { Vazio } from "../componentes/Vazio";

export function Lembretes({
  lembretes,
  ocupado,
  aoResolver,
}: {
  lembretes: Lembrete[];
  ocupado: boolean;
  aoResolver: (lembrete: Lembrete, como: "concluir" | "cancelar") => void;
}) {
  if (lembretes.length === 0) {
    return (
      <Vazio frase="Nenhum lembrete esperando. Escreva embaixo, ou toque no sino de uma Task." />
    );
  }
  return (
    <ul className="lista">
      {lembretes.map((lembrete) => {
        const cobra = pedeAtencao(lembrete);
        return (
          <li className="item" key={lembrete.id} data-cobra={cobra || undefined}>
            <div className="item-corpo">
              <p>{lembrete.title}</p>
              <small>
                {daquiA(lembrete.nextDueAt)}
                {lembrete.target?.type === "task" ? " · task" : ""}
                {lembrete.snoozeCount > 0 ? ` · adiado ${lembrete.snoozeCount}×` : ""}
              </small>
            </div>
            {/* Concluir e cancelar, e nao adiar: adiar mexe na hora do
                vencimento, que e a coluna que o agendador do PC le — ver
                `api.rs`. As duas daqui levam o lembrete para estado terminal, e
                depois delas nao ha o que disputar. */}
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
              <button
                className="acao"
                type="button"
                data-variante="quieto"
                disabled={ocupado}
                aria-label={`Cancelar ${lembrete.title}`}
                onClick={() => aoResolver(lembrete, "cancelar")}
              >
                Cancelar
              </button>
            </div>
          </li>
        );
      })}
    </ul>
  );
}
