/**
 * Comandos de barra do composer.
 *
 * # Por que eles NÃO são um protocolo
 *
 * O Hermes entende linguagem natural, e essa é a interface principal. Um `/task`
 * que virasse um comando estruturado criaria um segundo idioma para o usuário
 * decorar, e um segundo caminho para o backend manter — exatamente o "sistema
 * paralelo" que o redesign proíbe.
 *
 * Então `/` aqui é ATALHO DE DIGITAÇÃO, e não sintaxe: ele expande para a frase
 * em português que o usuário teria escrito, e o cursor fica no fim para
 * completar. O que chega ao gateway é sempre prosa.
 *
 * # Por que a lista é curta
 *
 * Só entra comando que corresponde a uma capacidade real do M/OS. Um `/deploy`
 * bonito na lista e inerte no backend ensina uma promessa falsa na primeira vez
 * que alguém tenta.
 */

export type Comando = {
  /** O que se digita, sem a barra. */
  nome: string;
  /** O que a lista mostra à direita do nome. */
  descricao: string;
  /** A frase que substitui o token. Termina com espaço: o cursor continua. */
  expansao: string;
};

export const COMANDOS: Comando[] = [
  { nome: "task", descricao: "criar uma Task", expansao: "crie uma task para " },
  { nome: "project", descricao: "abrir ou criar um Project", expansao: "sobre o project " },
  { nome: "capture", descricao: "guardar na Inbox", expansao: "guarde na inbox: " },
  { nome: "find", descricao: "procurar no M/OS", expansao: "encontre " },
  { nome: "calendar", descricao: "ver a agenda", expansao: "o que tem na agenda " },
  { nome: "atencao", descricao: "o que pede atenção agora", expansao: "o que precisa da minha atenção agora?" },
];

/**
 * O token de barra em digitação, se houver.
 *
 * Só vale no COMEÇO do rascunho: uma barra no meio de uma frase é barra — data,
 * caminho de arquivo, "e/ou" — e abrir um menu ali interromperia a escrita
 * normal. Essa é a diferença entre `/` e `@`, que vale em qualquer posição
 * porque menção no meio da frase é o uso comum dela.
 */
export function tokenDeComando(rascunho: string): string | null {
  const encontrado = /^\/([\wÀ-ú-]*)$/.exec(rascunho);
  return encontrado ? encontrado[1] : null;
}

/** Os comandos que casam com o que já foi digitado, na ordem do catálogo. */
export function comandosPara(rascunho: string): Comando[] {
  const token = tokenDeComando(rascunho);
  if (token === null) return [];
  const alvo = token.toLowerCase();
  return COMANDOS.filter((comando) => comando.nome.startsWith(alvo));
}

/**
 * O rascunho depois de escolher um comando.
 *
 * Troca o token inteiro pela expansão. Nada do que veio antes é preservado
 * porque o token de comando só existe quando ele É o rascunho inteiro.
 */
export function aplicarComando(comando: Comando): string {
  return comando.expansao;
}
