/**
 * As secoes do Settings: quais existem, como se agrupam e qual abre.
 *
 * Vive fora da pagina para poder ser testado — nao ha teste de DOM neste
 * repositorio (`vitest.config.ts`), entao o que DECIDE vira funcao pura. Mesma
 * forma do `HOME_SECTIONS` no `homeLayout.ts`.
 *
 * ESTA E A UNICA COPIA DA ORDEM. A pagina itera daqui; nao ha uma segunda lista
 * escrita no JSX. A licao veio do `arrange_widgets`, que existiu em Rust e em
 * TypeScript ao mesmo tempo e ficou para tras em silencio, com os testes dele
 * passando.
 *
 * # Uma secao por vez (2026-09-24)
 *
 * A pagina era uma coluna so com vinte paineis e um menu que rolava ate a
 * ancora. Achar o botao do tunel do Hermes era rolar por Sincronizacao inteira
 * e depois procurar dentro de "Conexoes", entre OpenAI, Univirtus e M-Finance.
 * Agora o menu troca a secao mostrada, cada secao diz para que serve, e o
 * Hermes tem a propria — ele e a unica conexao que cai no dia a dia.
 */

export type SettingsGroup = "conexoes" | "uso" | "dados";

export type SettingsSection = {
  /** Vira alvo de navegacao e chave salva. Renomear quebra a secao lembrada. */
  id: string;
  /** O que a navegacao e o `<h2>` mostram. */
  title: string;
  /** Uma frase: para que a secao serve. Aparece embaixo do titulo. */
  descricao: string;
  grupo: SettingsGroup;
};

export const SETTINGS_GROUPS: { id: SettingsGroup; title: string }[] = [
  { id: "conexoes", title: "Conexões" },
  { id: "uso", title: "Uso" },
  { id: "dados", title: "Dados" },
];

/* A ordem e o desenho, e nao acaso.

   Sincronizacao primeiro porque e a unica que alguem PROCURA, em vez de
   encontrar por acaso; o Hermes logo depois, porque e a conexao que cai. O
   resto desce do que fala com FORA para o que so importa quando algo deu
   errado (saude, avancado). */
export const SETTINGS_SECTIONS: SettingsSection[] = [
  { id: "sync", title: "Sincronização", descricao: "Os aparelhos que compartilham este M/OS e a saúde da fila.", grupo: "conexoes" },
  { id: "hermes", title: "Hermes", descricao: "O agente na sua VPS: túnel, conexão e credencial.", grupo: "conexoes" },
  { id: "integracoes", title: "Integrações", descricao: "Serviços de fora que o M/OS lê ou aciona.", grupo: "conexoes" },
  { id: "aparencia", title: "Aparência e entrada", descricao: "Tema, atalhos globais e captura por voz.", grupo: "uso" },
  { id: "reunioes", title: "Reuniões", descricao: "Gravação, transcrição e o Recording Guardian.", grupo: "uso" },
  { id: "inicio", title: "Início e atualizações", descricao: "Abrir com o Windows e versões novas.", grupo: "uso" },
  { id: "dados", title: "Backup e arquivo", descricao: "Backup, restauração, export, arquivados e lixeira.", grupo: "dados" },
  { id: "saude", title: "Saúde", descricao: "Integridade do banco, ocorrências e sinais de uso.", grupo: "dados" },
  { id: "avancado", title: "Avançado", descricao: "Registro de functions e importação do CronoCAD.", grupo: "dados" },
];

/** O que o chamador pediu para abrir, quando pediu. */
export type AlvoDoSettings = "function_registry" | "updates_check" | "hermes" | "sync" | undefined;

const SECAO_DO_ALVO: Record<Exclude<AlvoDoSettings, undefined>, string> = {
  function_registry: "avancado",
  updates_check: "inicio",
  hermes: "hermes",
  sync: "sync",
};

/**
 * Qual secao abre.
 *
 * Um pedido explicito (vindo de uma function, do aviso do Hermes, da saude do
 * sync) vence; depois, a ultima secao que a pessoa viu — voltar ao Settings e
 * cair de novo no topo seria refazer o caminho; e so entao a primeira. Secao
 * salva que deixou de existir e ignorada.
 */
export function secaoInicial(alvo: AlvoDoSettings, salva: string | null): string {
  if (alvo) return SECAO_DO_ALVO[alvo];
  if (salva && SETTINGS_SECTIONS.some((secao) => secao.id === salva)) return salva;
  return SETTINGS_SECTIONS[0].id;
}
