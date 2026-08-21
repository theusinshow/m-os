/**
 * Argos como presença do Hermes — a cor diz se ele está lá.
 *
 * # O conflito, e como ele foi reconciliado
 *
 * O `argosPose.ts` registra, com todas as letras, que **a conexão não entra**:
 * "Hermes nunca configurado é `offline`, que é um estado perfeitamente normal —
 * tratá-la como falha deixaria Argos permanentemente aterrorizado em qualquer
 * instalação sem gateway."
 *
 * Aquela objeção é sobre **pose**, e ela continua de pé: nada aqui muda a cara do
 * bicho, e ninguém fica com cara de susto por falta de gateway. O que muda é a
 * **cor**, que é outro canal e não carrega medo — carrega presença.
 *
 * # O que a cor deixou de dizer
 *
 * Antes, a cor codificava o PESO (`weightFor`): amarelo quando algo esperava
 * você. Esse sinal não sumiu — ele mudou de canal, e agora vive no movimento,
 * pelo `data-peso` no botão. Cor é presença; movimento é urgência. Dois fatos
 * ortogonais, dois canais.
 */
import type { HermesConnectionState } from "./hermes";

export type ArgosPresenca = "conectado" | "conectando" | "desconectado";

/**
 * Quanto tempo depois de abrir o app um `offline` ainda conta como "conectando".
 *
 * O túnel não sobe junto com a janela. Nos primeiros segundos o gateway responde
 * `offline` porque ele AINDA não subiu, e não porque caiu — e um balão de queda
 * que aparece na abertura e some sozinho três segundos depois é exatamente o
 * "pisca e some" que o próprio balão foi desenhado para não ser.
 *
 * Quatro segundos porque é mais do que a subida costuma levar e menos do que
 * alguém leva para reparar na ausência do aviso. Passado isso, `offline` é
 * queda de verdade e o balão acende.
 */
export const CARENCIA_DE_ABERTURA_MS = 4000;

/**
 * O que o estado da conexão significa para o bicho.
 *
 * `null` é "ainda não perguntamos", e ele vira `conectando` de propósito:
 * nascer `desconectado` acenderia o balão de queda em toda abertura do app,
 * antes de o Hermes ter tido a chance de responder.
 *
 * `emAbertura` estende essa mesma cortesia para a PRIMEIRA RESPOSTA: um
 * `offline` dentro da carência ainda se lê como `conectando`. Sem isso o balão
 * subia em toda abertura, dizia que o chat não responde, e se desdizia sozinho
 * quando o túnel terminava de subir.
 */
export function presencaDe(
  state: HermesConnectionState | null,
  emAbertura = false,
): ArgosPresenca {
  switch (state) {
    case "online":
      return "conectado";
    case "offline":
      return emAbertura ? "conectando" : "desconectado";
    default:
      return "conectando";
  }
}

/**
 * O token de cor do corpo.
 *
 * Token, e nunca literal: o design system continua sendo a fonte, e a troca de
 * tema precisa alcançar o bicho.
 */
export function corDaPresenca(presenca: ArgosPresenca): string {
  switch (presenca) {
    case "conectado":
      return "--signal-ink";
    case "conectando":
      return "--text";
    case "desconectado":
      return "--text-system";
  }
}

/** O que o leitor de tela ouve, junto da pose. */
export function rotuloDaPresenca(presenca: ArgosPresenca): string {
  switch (presenca) {
    case "conectado":
      return "Hermes conectado";
    case "conectando":
      return "Hermes conectando";
    case "desconectado":
      return "Hermes desconectado";
  }
}
