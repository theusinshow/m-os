/**
 * O que cada estado de uma reunião significa para quem está olhando.
 *
 * # Por que existe um "próximo passo" separado do rótulo
 *
 * Em 20/08 uma reunião de seis minutos ficou parada em `recorded`. A tela dizia
 * "gravada", e era verdade — mas "gravada" não diz que a transcrição espera um
 * clique, e a leitura honesta de quem olhou foi **"não gravou nada"**. O áudio
 * estava inteiro no disco o tempo todo.
 *
 * Um rótulo diz onde a coisa está. O próximo passo diz o que falta. São duas
 * perguntas diferentes, e responder só a primeira foi o que custou a confusão.
 *
 * `null` é resposta legítima e importante: significa "não há nada para você
 * fazer". Inventar um passo onde não há ensina a pessoa a ignorar a linha.
 */
import type { MeetingStatus } from "./types";

const ROTULO: Record<MeetingStatus, string> = {
  recording: "gravando",
  paused: "pausada",
  stopping: "encerrando",
  interrupted: "interrompida",
  recorded: "gravada",
  transcribing: "transcrevendo",
  transcribed: "transcrita",
  analyzing: "analisando",
  ready: "pronta",
  failed: "falhou",
  cancelled: "descartada",
};

export function rotuloDoEstado(status: MeetingStatus): string {
  return ROTULO[status];
}

/**
 * O que falta, em uma frase — ou `null` quando não falta nada de você.
 *
 * Os estados em curso (`transcribing`, `analyzing`, `recording`) devolvem `null`
 * de propósito: quem está esperando o computador não precisa de instrução, e a
 * barra de processamento já conta essa parte.
 */
export function proximoPasso(status: MeetingStatus): string | null {
  switch (status) {
    case "recorded":
      return "O áudio está salvo. Falta transcrever — use o botão Transcrever.";
    case "transcribed":
      return "A transcrição está pronta. Falta a análise do Hermes.";
    case "interrupted":
      return "Esta gravação foi cortada por uma queda. Decida se processa ou descarta.";
    case "failed":
      return "A gravação está segura. Você pode tentar de novo.";
    case "cancelled":
      return "Esta reunião foi descartada, e o áudio dela não existe mais.";
    default:
      return null;
  }
}
