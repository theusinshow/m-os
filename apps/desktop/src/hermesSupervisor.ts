/**
 * O supervisor da ponte do Hermes: o que fazer depois de uma tentativa falhar.
 *
 * Vive fora do `App.tsx` para poder ser testado — nao ha teste de DOM neste
 * repositorio (`vitest.config.ts`), entao o que DECIDE vira funcao pura.
 *
 * # O defeito que originou este arquivo
 *
 * A decisao morava dentro do `catch` do supervisor, assim:
 *
 * ```ts
 * const failure = error as Partial<HermesFailure> | null;
 * if (!failure?.retriable) { stopped = true; return; }
 * ```
 *
 * E o `try` acima dela chamava `hermes.status()`, que e um comando do Tauri como
 * qualquer outro. Na abertura, esse comando bate no portao do backend e volta
 * **`O M/OS ainda esta abrindo`** — um `CoreError`, nao um `HermesFailure`.
 *
 * `CoreError` tem `retryable`. `HermesFailure` tem `retriable`. Uma letra.
 *
 * Entao `failure?.retriable` dava `undefined`, o supervisor marcava `stopped` e
 * **nunca mais tentava** — num PC onde tudo estava certo, e por causa de uma
 * corrida que se resolve sozinha em menos de um segundo. E a mesma corrida que
 * o `abertura.ts` ja trata com doze tentativas.
 *
 * O sintoma, do lado de quem usa: o Hermes aparecia Offline todo dia, e o unico
 * jeito de destravar era **redigitar usuario e senha em Settings** — nao porque
 * a credencial tivesse sumido (ela vive no Credential Manager e estava la), mas
 * porque salvar chama `hermes.connect()` direto, por fora do supervisor morto.
 *
 * # A regra, agora explicita
 *
 * Parar para sempre e uma decisao cara, e so duas causas a merecem:
 * `unauthorized` e `rate_limited`. As duas nao mudam por insistencia — e a
 * segunda PIORA, porque repetir foi o que causou o bloqueio.
 *
 * Todo o resto repete. **Inclusive o que nao se reconhece**: um erro sem forma
 * conhecida e, quase sempre, um erro de infraestrutura passageiro, e tratar o
 * desconhecido como fatal foi exatamente o defeito acima.
 */
import type { HermesFailure } from "./hermes";

export type DecisaoDoSupervisor =
  /** Agenda outra tentativa, com a espera crescendo. */
  | { acao: "repetir" }
  /** Para. So `unauthorized` e `rate_limited` chegam aqui. */
  | { acao: "parar"; causa: "unauthorized" | "rate_limited" };

/** As unicas causas que insistir nao resolve — e a segunda, piora. */
const DEFINITIVAS = ["unauthorized", "rate_limited"] as const;

/**
 * O que fazer depois de uma tentativa de conexao falhar.
 *
 * O erro chega como `unknown` de proposito: ele pode ser um `HermesFailure` da
 * ponte, um `CoreError` do portao de abertura, ou qualquer coisa que o IPC
 * resolva jogar. A funcao so para diante de uma causa que ela RECONHECE como
 * definitiva; na duvida, repete.
 */
export function decidirAposFalha(erro: unknown): DecisaoDoSupervisor {
  if (!erro || typeof erro !== "object") return { acao: "repetir" };
  const kind = (erro as Partial<HermesFailure>).kind;
  const definitiva = DEFINITIVAS.find((causa) => causa === kind);
  if (definitiva) return { acao: "parar", causa: definitiva };
  return { acao: "repetir" };
}

/**
 * Se uma parada pode ser desfeita por o usuario voltar ao app.
 *
 * Sempre `false` hoje, e o codigo existe para dizer por que: as duas causas que
 * param sao justamente as que voltar para a janela nao muda. Trazer o M/OS para
 * a frente com a senha errada so martelaria o login ate o gateway responder
 * 429, que e o defeito que a trava foi criada para impedir.
 *
 * Quem destrava e a acao em Settings — e agora ela e a UNICA coisa que precisa
 * destravar, porque o desconhecido deixou de parar o supervisor.
 */
export function podeRearmarNoPrimeiroPlano(causa: DecisaoDoSupervisor): boolean {
  return causa.acao !== "parar";
}
