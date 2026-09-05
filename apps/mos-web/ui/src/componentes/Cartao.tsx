import type { CSSProperties } from "react";
import type { Enfeite } from "../paginas/cartoes";

/**
 * O cartao da Home.
 *
 * O numero e o conteudo; o rotulo e legenda. Por isso ele vem primeiro na ordem
 * visual e maior — quem abre o app quer o numero, e le o resto so se o numero
 * surpreender.
 *
 * `largo` e o cartao de texto (a ultima captura), que ocupa a linha inteira: uma
 * frase espremida em meia coluna vira tres linhas cortadas.
 *
 * # O enfeite não é decoração
 *
 * A semana e o progresso respondem a pergunta que o número não responde: *como
 * chegou até aqui*. Eles moram entre o número e a legenda porque é ali que o
 * olho passa de qualquer jeito — embaixo da legenda seriam vistos só por quem
 * já tivesse decidido ler o cartão inteiro.
 */
export function Cartao({
  rotulo,
  numero,
  legenda,
  aposto,
  enfeite,
  urgente,
  largo,
  palavra,
  aoTocar,
}: {
  rotulo: string;
  numero: string;
  legenda: string;
  aposto?: string;
  enfeite?: Enfeite;
  urgente?: boolean;
  largo?: boolean;
  palavra?: boolean;
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
      <span className="cartao-rotulo">
        {rotulo}
        {aposto ? <b>{aposto}</b> : null}
      </span>
      <span className="cartao-numero" data-palavra={palavra || undefined}>
        {numero}
      </span>
      {enfeite ? <Desenho enfeite={enfeite} /> : null}
      {legenda ? <span className="cartao-legenda">{legenda}</span> : null}
    </button>
  );
}

function Desenho({ enfeite }: { enfeite: Enfeite }) {
  if (enfeite.tipo === "progresso") {
    return (
      <span className="cartao-progresso" aria-hidden="true">
        <i style={{ width: `${Math.round(enfeite.fracao * 100)}%` }} />
      </span>
    );
  }

  // A escala é o MAIOR dia da semana, e não a jornada de oito horas: quem
  // trabalhou duas horas na semana inteira veria sete tracinhos idênticos
  // contra uma escala fixa, e a forma da semana é justamente o que se quer ver.
  const maior = Math.max(...enfeite.dias, 1);
  return (
    <span className="cartao-semana" aria-hidden="true">
      {enfeite.dias.map((segundos, dia) => (
        <i
          key={dia}
          data-hoje={dia === enfeite.hoje || undefined}
          data-futuro={dia > enfeite.hoje || undefined}
          style={
            {
              height: dia > enfeite.hoje ? "8%" : `${Math.max(8, (segundos / maior) * 100)}%`,
              // A escada de 40 ms faz a semana crescer da esquerda para a
              // direita, que é a direção em que ela foi vivida.
              "--degrau": `${dia * 40}ms`,
            } as CSSProperties
          }
        />
      ))}
    </span>
  );
}
