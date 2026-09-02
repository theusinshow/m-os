/**
 * O estado vazio.
 *
 * Ele diz o que a tela FARIA, e nao que ela esta vazia. "Inbox vazia" descreve o
 * pixel; "o que voce capturar aparece aqui" descreve o proposito — e so a
 * segunda ensina alguma coisa a quem abriu o app pela primeira vez.
 *
 * A acao e opcional porque nem todo vazio tem saida daqui: em Capturar, o que
 * falta fazer ja esta na tela, no compositor logo abaixo.
 */
export function Vazio({
  frase,
  acao,
}: {
  frase: string;
  acao?: { rotulo: string; aoTocar: () => void };
}) {
  return (
    <div className="vazio">
      <p>{frase}</p>
      {acao ? (
        <button type="button" className="botao" data-variante="quieto" onClick={acao.aoTocar}>
          {acao.rotulo}
        </button>
      ) : null}
    </div>
  );
}
