/**
 * O que faz a pagina de Reunioes acordar — e por que isso mora fora dela.
 *
 * O defeito de 20/08 nasceu de duas omissoes que so aparecem quando a pagina JA
 * esta montada, que e exatamente o caso de quem iniciou a gravacao por ela:
 *
 * 1. `meeting_stop` nao emitia evento nenhum. O caminho gemeo, `stop_from_tray`,
 *    ja emitia — e a doc do proprio comando promete que "o caminho e o MESMO do
 *    clique na barra". Nao era.
 * 2. o `focus` que o shell manda depois de parar entrava so como valor inicial
 *    do `useState`, e prop que muda depois da montagem nao mexia em nada.
 *
 * O efeito somado: a barra sumia, a lista continuava dizendo "gravando", nada
 * ficava selecionado, e a leitura honesta de quem estava olhando era "nao gravou
 * nada" — com seis minutos de audio intactos no disco.
 *
 * Vive aqui, e nao dentro do componente, porque nao ha teste de DOM neste repo:
 * o que se verifica tem de ser funcao pura. Mesma razao do `lequePetalas.ts`.
 */

/**
 * Os eventos que mudam uma reuniao de estagio, e que por isso obrigam a lista a
 * recarregar.
 *
 * Sao os FINS de estagio, e nao os comecos: `meeting-transcribing` existe para
 * mover uma barra de progresso, e recarregar a lista a cada passo dela seria
 * pedir ao banco a mesma resposta dezenas de vezes.
 */
export const EVENTOS_DE_REUNIAO = [
  "meeting-stopped",
  "meeting-transcribed",
  "meeting-analyzed",
  "meeting-failed",
] as const;

/**
 * Qual reuniao mostrar quando o shell aponta para uma.
 *
 * Chamada QUANDO O FOCO MUDA — nao a cada render. A diferenca importa: se
 * rodasse sempre, clicar noutra reuniao da lista seria desfeito no render
 * seguinte, e a pessoa ficaria presa na ultima que o shell escolheu.
 */
export function selecaoAoFocar(
  foco: string | null | undefined,
  atual: string | null,
): string | null {
  return foco ?? atual;
}
