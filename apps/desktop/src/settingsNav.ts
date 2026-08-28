/**
 * As secoes do Settings: quais existem, como se chamam, e qual esta a vista.
 *
 * Vive fora da pagina para poder ser testado — nao ha teste de DOM neste
 * repositorio (`vitest.config.ts`), entao o que DECIDE vira funcao pura. Mesma
 * forma do `HOME_SECTIONS` no `homeLayout.ts`.
 *
 * ESTA E A UNICA COPIA DA ORDEM. A pagina itera daqui; nao ha uma segunda lista
 * escrita no JSX. A licao veio do `arrange_widgets`, que existiu em Rust e em
 * TypeScript ao mesmo tempo e ficou para tras em silencio, com os testes dele
 * passando.
 */

export type SettingsSection = {
  /** Vira ancora e alvo de rolagem. Renomear quebra link salvo. */
  id: string;
  /** O que a navegacao e o `<h2>` mostram. */
  title: string;
};

/* A ordem e o desenho, e nao acaso.

   Sincronizacao primeiro porque e a unica que alguem PROCURA, em vez de
   encontrar por acaso. O resto desce do que fala com FORA (conexoes) para o que
   so importa quando algo deu errado (avancado).

   O agrupamento anterior era "Conexao e aparencia", e o "e" no meio do titulo
   era a confissao de que nunca houve criterio: ele juntava Hermes, Univirtus,
   sync, a ponte do M-Finance e o tema claro. */
export const SETTINGS_SECTIONS: SettingsSection[] = [
  { id: "sync", title: "Sincronização" },
  { id: "conexoes", title: "Conexões" },
  { id: "aparencia", title: "Aparência e entrada" },
  { id: "inicio", title: "Início e atualizações" },
  { id: "reunioes", title: "Reuniões" },
  { id: "dados", title: "Dados" },
  { id: "avancado", title: "Avançado" },
];

/**
 * Quanto ANTES do topo de uma secao ela ja conta como a visivel.
 *
 * Sem esta margem o titulo chega colado no topo e a navegacao ainda marca a
 * secao anterior — a marca fica sempre um passo atras do olho.
 */
const MARGEM = 24;

/**
 * Qual secao esta a vista, dado onde cada uma comeca e o quanto se rolou.
 *
 * A ULTIMA que ja passou, e nao a mais proxima: uma secao curta no fim da pagina
 * nunca encheria a tela, e a regra da proximidade deixaria a marca presa na
 * penultima para sempre.
 */
export function secaoVisivel(
  posicoes: { id: string; top: number }[],
  scrollTop: number,
): string {
  let atual = "";
  for (const posicao of posicoes) {
    if (scrollTop >= posicao.top - MARGEM) atual = posicao.id;
  }
  // Antes da primeira, a primeira. "Nenhuma marcada" faria a navegacao parecer
  // quebrada logo na chegada, que e quando ninguem ainda rolou nada.
  return atual || posicoes[0]?.id || "";
}
