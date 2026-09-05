import { useEffect, useState } from "react";
import { pedeAtencao, type EdicaoDeLembrete, type Lembrete as Item } from "../api";
import { paraCampoLocal, porExtenso } from "../instantes";

/** A palavra que a tela usa para cada estado. */
const PALAVRA: Record<Item["status"], string> = {
  scheduled: "AGENDADO",
  due: "VENCEU",
  delivered: "AVISADO",
  acknowledged: "VISTO",
  snoozed: "ADIADO",
  completed: "CONCLUÍDO",
  cancelled: "CANCELADO",
  missed: "PERDIDO",
  expired: "EXPIROU",
};

/** Os adiamentos que a folha oferece, em minutos. */
const ADIAMENTOS = [
  { rotulo: "10 min", minutos: 10 },
  { rotulo: "1 h", minutos: 60 },
  { rotulo: "3 h", minutos: 180 },
  { rotulo: "amanhã", minutos: 60 * 24 },
] as const;

/**
 * Um lembrete, inteiro.
 *
 * # Por que uma tela, e não uma linha com mais botões
 *
 * A lista respondia *o que falta* e oferecia duas ações. Tudo o mais — corrigir
 * a hora que se digitou errado, ler a nota inteira, saber há quanto tempo isto
 * está adiado — exigia o PC. Um lembrete criado no ônibus com a hora errada
 * ficava errado até chegar em casa.
 *
 * # O que é edição e o que é transição
 *
 * **Salvar** grava título, nota e hora. **Concluir**, **cancelar** e **adiar**
 * são transições de estado, e por isso não passam pelo formulário: elas não
 * dependem do que está escrito nos campos, e misturá-las faria "concluir"
 * salvar de passagem um texto que a pessoa estava só experimentando.
 *
 * Adiar e remarcar são coisas diferentes, e a tela mostra as duas: adiar conta
 * fadiga — depois do quinto o sistema oferece ajuda —, remarcar é corrigir.
 */
export function Lembrete({
  lembrete,
  ocupado,
  aoSalvar,
  aoConcluir,
  aoCancelar,
  aoAdiar,
  aoArquivar,
  aoVoltar,
}: {
  lembrete: Item;
  ocupado: boolean;
  aoSalvar: (mudanca: EdicaoDeLembrete) => void;
  aoConcluir: () => void;
  aoCancelar: () => void;
  aoAdiar: (ate: Date) => void;
  aoArquivar: () => void;
  aoVoltar: () => void;
}) {
  const [titulo, setTitulo] = useState(lembrete.title);
  const [nota, setNota] = useState(lembrete.body);
  const [quando, setQuando] = useState(
    lembrete.nextDueAt ? paraCampoLocal(new Date(lembrete.nextDueAt)) : "",
  );
  const [confirmando, setConfirmando] = useState(false);

  // O lembrete pode mudar por baixo — o laço de 30 s traz o que o PC editou. Os
  // campos seguem, mas só quando é OUTRO lembrete: sobrescrever o que a pessoa
  // está digitando porque o servidor respondeu seria perder o texto no meio da
  // frase.
  useEffect(() => {
    setTitulo(lembrete.title);
    setNota(lembrete.body);
    setQuando(lembrete.nextDueAt ? paraCampoLocal(new Date(lembrete.nextDueAt)) : "");
    setConfirmando(false);
  }, [lembrete.id]);

  const resolvido =
    lembrete.status === "completed" ||
    lembrete.status === "cancelled" ||
    lembrete.status === "expired";

  const original = {
    titulo: lembrete.title,
    nota: lembrete.body,
    quando: lembrete.nextDueAt ? paraCampoLocal(new Date(lembrete.nextDueAt)) : "",
  };
  const mexeu =
    titulo !== original.titulo || nota !== original.nota || quando !== original.quando;

  function salvar() {
    const mudanca: EdicaoDeLembrete = {};
    if (titulo !== original.titulo) mudanca.titulo = titulo;
    if (nota !== original.nota) mudanca.nota = nota;
    if (quando !== original.quando && quando) mudanca.quando = new Date(quando);
    aoSalvar(mudanca);
  }

  return (
    <div className="detalhe">
      <header className="detalhe-topo">
        <button type="button" className="voltar" onClick={aoVoltar}>
          ← Lembretes
        </button>
        <span className="etiqueta" data-cobra={pedeAtencao(lembrete) || undefined}>
          {PALAVRA[lembrete.status]}
        </span>
      </header>

      {lembrete.nextDueAt ? (
        <p className="detalhe-quando">{porExtenso(new Date(lembrete.nextDueAt))}</p>
      ) : null}

      {resolvido ? (
        // Resolvido não se edita: é uma resposta já dada, e o núcleo recusa.
        // Mostrar os campos desabilitados seria oferecer o que não existe.
        <>
          <h2 className="detalhe-titulo">{lembrete.title}</h2>
          {lembrete.body ? <p className="detalhe-nota">{lembrete.body}</p> : null}
          <p className="detalhe-aviso">
            Este lembrete já foi resolvido. Para mexer nele, crie um novo.
          </p>
        </>
      ) : (
        <>
          <label className="campo">
            <span>TÍTULO</span>
            <input
              value={titulo}
              onChange={(evento) => setTitulo(evento.currentTarget.value)}
              enterKeyHint="done"
            />
          </label>

          <label className="campo">
            <span>NOTA</span>
            <textarea
              value={nota}
              rows={3}
              placeholder="o que mais importa lembrar"
              onChange={(evento) => setNota(evento.currentTarget.value)}
            />
          </label>

          <label className="campo">
            <span>QUANDO</span>
            <input
              type="datetime-local"
              value={quando}
              onChange={(evento) => setQuando(evento.currentTarget.value)}
            />
          </label>

          {/* O botão de salvar só existe quando há o que salvar. Um "Salvar"
              sempre aceso convida a tocar sem ter mudado nada, e cada toque
              desses é uma escrita que atravessa para o PC por nada. */}
          {mexeu ? (
            <button
              type="button"
              className="botao"
              disabled={ocupado || !titulo.trim()}
              onClick={salvar}
            >
              Salvar
            </button>
          ) : null}

          <section className="detalhe-bloco">
            <h3>ADIAR</h3>
            <div className="detalhe-adiar">
              {ADIAMENTOS.map((opcao) => (
                <button
                  key={opcao.minutos}
                  type="button"
                  disabled={ocupado}
                  onClick={() => aoAdiar(new Date(Date.now() + opcao.minutos * 60_000))}
                >
                  {opcao.rotulo}
                </button>
              ))}
            </div>
            {lembrete.snoozeCount >= 5 ? (
              // §13 do sistema de atenção: a partir do quinto, adiar de novo
              // deixa de ser ajuda. A frase não bloqueia nada — só diz o que
              // está acontecendo, que é o que a pessoa não vê sozinha.
              <p className="detalhe-aviso">
                Adiado {lembrete.snoozeCount}× — talvez o problema não seja a
                hora. Remarcar ou cancelar pode resolver melhor.
              </p>
            ) : lembrete.snoozeCount > 0 ? (
              <p className="detalhe-nota">Adiado {lembrete.snoozeCount}×.</p>
            ) : null}
          </section>

          <div className="detalhe-acoes">
            <button
              type="button"
              className="botao"
              disabled={ocupado}
              onClick={aoConcluir}
            >
              Concluir
            </button>
            <button
              type="button"
              className="botao"
              data-variante="quieto"
              disabled={ocupado}
              onClick={aoCancelar}
            >
              Cancelar lembrete
            </button>
          </div>
        </>
      )}

      {/* Excluir pede confirmação, e só ele. É a única ação daqui que a pessoa
          não desfaz sozinha na mesma tela — concluir e cancelar continuam
          visíveis no histórico. */}
      <section className="detalhe-bloco">
        {confirmando ? (
          <div className="detalhe-acoes">
            <button
              type="button"
              className="botao"
              data-variante="perigo"
              disabled={ocupado}
              onClick={aoArquivar}
            >
              Excluir mesmo
            </button>
            <button
              type="button"
              className="botao"
              data-variante="quieto"
              onClick={() => setConfirmando(false)}
            >
              Deixa
            </button>
          </div>
        ) : (
          <button
            type="button"
            className="detalhe-excluir"
            onClick={() => setConfirmando(true)}
          >
            Excluir
          </button>
        )}
      </section>
    </div>
  );
}
