/**
 * A corrida da abertura, e por que esperar é a resposta certa.
 *
 * O Tauri cria a janela declarada no `tauri.conf.json` **antes** de rodar o
 * `setup`, e a webview já dispara IPC enquanto o banco ainda está abrindo. O
 * backend tem um portão para isso: comando que chega cedo demais é recusado com
 * `O M/OS ainda esta abrindo.`
 *
 * O que faltava era do lado de cá. O boot tratava essa recusa como falha
 * definitiva e parava numa tela dizendo *"M/OS não abriu os dados locais com
 * segurança"* — que além de travar o app, **é mentira**: os dados estão
 * intactos, e o que aconteceu foi só chegar cedo demais.
 *
 * Uma condição que se resolve sozinha em menos de um segundo se espera. A que
 * não se resolve, não — e é por isso que a decisão olha `retryable`, e não só o
 * texto: falha de banco de verdade continua parando na primeira tentativa.
 */

/** Quantas vezes tentar antes de admitir que não é corrida. */
export const TENTATIVAS_DE_ABERTURA = 12;

/** O teto da espera entre tentativas, em ms. */
const ESPERA_MAXIMA = 500;

/**
 * Quanto esperar antes da próxima tentativa.
 *
 * Começa curta porque a abertura normal termina em menos de um segundo — esperar
 * meio segundo já na primeira faria o app parecer lento em todo boot saudável.
 * Cresce e para de crescer: espera que dobra sem teto vira travamento.
 */
export function esperaDaTentativa(tentativa: number): number {
  return Math.min(60 * 2 ** tentativa, ESPERA_MAXIMA);
}

/** Se vale esperar e tentar de novo, em vez de mostrar a tela de erro. */
export function deveEsperarAbertura(
  erro: { message: string; retryable: boolean },
  tentativa: number,
): boolean {
  if (tentativa >= TENTATIVAS_DE_ABERTURA) return false;
  if (!erro.retryable) return false;
  return /ainda est[aá] abrindo/i.test(erro.message);
}
