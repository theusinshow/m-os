/**
 * O que a barra de processamento pode prometer — e o que ela não pode.
 *
 * A regra que organiza este arquivo: **cada número mostrado tem de ser medido.**
 * Onde não há medida, não há número — há nome de estágio. Barra que inventa
 * porcentagem é pior que barra nenhuma, porque ela ensina a pessoa a não
 * acreditar na próxima.
 *
 * - **Transcrever tem fração**, e ela vem do próprio whisper, lido do stderr
 *   enquanto ele trabalha (ver `parse_progress` no `mos-transcribe`).
 * - **Analisar não tem.** É chamada de rede: ou voltou, ou não voltou. O que
 *   existe ali é contagem de janelas, e contagem é fato.
 *
 * Vive fora do componente porque não há teste de DOM neste repo — a mesma razão
 * escrita no topo do `lequePetalas.ts`.
 */

export type Processamento =
  | { tipo: "transcrevendo"; meetingId: string; canal: "mic" | "system"; progress: number }
  | { tipo: "analisando"; meetingId: string; window: number; windows: number }
  | { tipo: "falhou"; meetingId: string; detalhe: string };

export type RotuloDeProcessamento = {
  titulo: string;
  detalhe: string;
  /** `null` significa indeterminado: a barra pulsa em vez de encher. */
  fracao: number | null;
  erro: boolean;
};

export function rotuloDoProcessamento(estado: Processamento): RotuloDeProcessamento {
  switch (estado.tipo) {
    case "transcrevendo":
      return {
        titulo: "Transcrevendo",
        /* O canal aparece nomeado de propósito: são duas passadas do whisper, e
           escondê-las faria a barra ir até a metade, parecer travada e depois
           recomeçar do zero — o que se lê como defeito. */
        detalhe: estado.canal === "mic" ? "o que você falou" : "o que os outros falaram",
        fracao: estado.progress,
        erro: false,
      };
    case "analisando":
      return {
        titulo: "Analisando com o Hermes",
        detalhe:
          estado.window === 0
            ? "juntando as partes"
            : `parte ${estado.window} de ${estado.windows}`,
        fracao: null,
        erro: false,
      };
    case "falhou":
      return {
        titulo: "Não deu certo",
        detalhe: estado.detalhe,
        fracao: null,
        erro: true,
      };
  }
}
