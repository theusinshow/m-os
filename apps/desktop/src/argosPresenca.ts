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
 * O que o estado da conexão significa para o bicho.
 *
 * `null` é "ainda não perguntamos", e ele vira `conectando` de propósito:
 * nascer `desconectado` acenderia o balão de queda em toda abertura do app,
 * antes de o Hermes ter tido a chance de responder.
 */
export function presencaDe(state: HermesConnectionState | null): ArgosPresenca {
  switch (state) {
    case "online":
      return "conectado";
    case "offline":
      return "desconectado";
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
