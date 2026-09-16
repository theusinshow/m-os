/**
 * O que a barra de processamento pode prometer — e o que ela não pode.
 *
 * A regra que organiza este arquivo: **cada número mostrado tem de ser medido.**
 * A fração global vem do pipeline, ponderada 70/30 entre transcrever e
 * organizar (`meeting_pipeline::TRANSCRIPTION_WEIGHT`): a transcrição tem
 * fração do próprio whisper, e a organização conta janelas que voltaram.
 *
 * A V2 muda o que a barra É: ela deixou de acompanhar botões apertados e passou
 * a acompanhar o que o M/OS faz sozinho. Por isso ela ganhou um estado que a V1
 * não tinha — **pronta** — e é nele que ela fica até a pessoa olhar.
 *
 * Vive fora do componente porque não há teste de DOM neste repo.
 */
import type { MeetingOverview, MeetingProgressEvent } from "./types";

export type Processamento =
  | { tipo: "progresso"; meetingId: string; evento: MeetingProgressEvent }
  | { tipo: "aguardando"; meetingId: string }
  | { tipo: "pronta"; meetingId: string; titulo: string; acoes: number; decisoes: number }
  | { tipo: "falhou"; meetingId: string; detalhe: string };

export type RotuloDeProcessamento = {
  titulo: string;
  detalhe: string;
  /** `null` significa sem barra: nada a medir. */
  fracao: number | null;
  erro: boolean;
  /** O texto do botão ao lado, quando há gesto. */
  acao: string | null;
};

export function prontaDe(overview: MeetingOverview): Processamento {
  return {
    tipo: "pronta",
    meetingId: overview.meeting.id,
    titulo: overview.meeting.title,
    acoes: overview.pendingActions,
    decisoes: overview.decisions,
  };
}

export function rotuloDoProcessamento(estado: Processamento): RotuloDeProcessamento {
  switch (estado.tipo) {
    case "progresso": {
      const { evento } = estado;
      const transcrevendo = evento.stage === "transcription";
      return {
        titulo: "Organizando reunião",
        detalhe: transcrevendo
          /* O canal aparece nomeado: são duas passadas do whisper, e escondê-las
             faria o detalhe parecer travado no meio. */
          ? evento.detail === "system" ? "transcrevendo o que os outros falaram" : "transcrevendo o que você falou"
          : evento.detail === "juntando" ? "juntando as partes" : "identificando decisões e tarefas",
        fracao: evento.overall,
        erro: false,
        acao: null,
      };
    }
    case "aguardando":
      return {
        titulo: "Organização pendente",
        detalhe: "o M/OS vai tentar de novo sozinho",
        fracao: null,
        erro: false,
        acao: null,
      };
    case "pronta": {
      const partes: string[] = [];
      if (estado.acoes > 0) partes.push(estado.acoes === 1 ? "1 ação" : `${estado.acoes} ações`);
      if (estado.decisoes > 0) partes.push(estado.decisoes === 1 ? "1 decisão" : `${estado.decisoes} decisões`);
      return {
        titulo: "Reunião pronta",
        detalhe: partes.length ? partes.join(" · ") : estado.titulo,
        fracao: null,
        erro: false,
        acao: estado.acoes > 0 ? "Revisar" : "Abrir",
      };
    }
    case "falhou":
      return {
        titulo: "Uma reunião precisa de você",
        detalhe: estado.detalhe,
        fracao: null,
        erro: true,
        acao: "Abrir",
      };
  }
}
