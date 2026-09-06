/**
 * O que a tela mostra enquanto o dado não chegou.
 *
 * # A confusão que ele conserta
 *
 * Sem ele, uma lista que ainda está na rede aparece com a frase do vazio:
 * *"Nenhuma hora nesta janela"*. Isso é uma **afirmação falsa** — a tela não
 * sabe se há horas, ela sabe que ainda não perguntou. E é a pior mentira
 * possível numa tela de consulta, porque ela é indistinguível da verdade: quem
 * lê acredita, fecha o app, e vai conferir no PC.
 *
 * A regra que sai daqui: **estado vazio só depois de uma resposta.** Antes
 * disso, esqueleto.
 *
 * # Por que não um spinner
 *
 * O `BRIEF-SISTEMA-DE-LOGOS.md` é taxativo: o único spinner do M/OS é a barra da
 * marca dando meia-volta, e ela já mora no topo. Um segundo indicador no meio da
 * tela competiria com ele dizendo a mesma coisa.
 *
 * O esqueleto também diz mais: ele mostra a FORMA do que vem, e a tela não
 * salta quando o conteúdo chega no lugar de um vazio de altura diferente.
 */
export function Esqueleto({ linhas = 3 }: { linhas?: number }) {
  return (
    <ul className="esqueleto" aria-hidden="true">
      {Array.from({ length: linhas }).map((_, indice) => (
        <li key={indice} style={{ animationDelay: `${indice * 90}ms` }}>
          <span />
          <b />
        </li>
      ))}
    </ul>
  );
}
