/**
 * O cartao da Home.
 *
 * O numero e o conteudo; o rotulo e legenda. Por isso ele vem primeiro na ordem
 * visual e maior — quem abre o app quer o numero, e le o resto so se o numero
 * surpreender.
 *
 * `largo` e o cartao de texto (a ultima captura), que ocupa a linha inteira: uma
 * frase espremida em meia coluna vira tres linhas cortadas.
 */
export function Cartao({
  rotulo,
  numero,
  legenda,
  urgente,
  largo,
  aoTocar,
}: {
  rotulo: string;
  numero: string;
  legenda: string;
  urgente?: boolean;
  largo?: boolean;
  aoTocar: () => void;
}) {
  return (
    <button
      type="button"
      className="cartao"
      data-urgente={urgente || undefined}
      data-largo={largo || undefined}
      onClick={aoTocar}
    >
      <span className="cartao-rotulo">{rotulo}</span>
      <span className="cartao-numero">{numero}</span>
      {legenda ? <span className="cartao-legenda">{legenda}</span> : null}
    </button>
  );
}
