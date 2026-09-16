/**
 * O que a tela de Reuniões decide, fora do componente.
 *
 * Tudo aqui é função pura, porque não há teste de DOM neste repo — a mesma
 * razão escrita no topo do `lequePetalas.ts`. E a copy vive junto das regras,
 * porque na V1 foi a copy que enganou: "gravada" lida como "não gravou nada".
 *
 * A regra que organiza o arquivo: **a pessoa vê Reunião → Processando → Pronta,
 * e nunca o estado técnico.** `transcribed`, `analyzing` e `failed(analysis)`
 * não existem para quem usa.
 *
 * Spec: `docs/superpowers/specs/2026-09-16-meeting-agent-v2-design.md`.
 */
import type {
  GuardianView, InsightKind, MeetingBookmark, MeetingInsight, MeetingOverview, MeetingPhase,
  PipelineProgress, TranscriptSegment,
} from "./types";

// ---------------------------------------------------------------------------
// Tempo
// ---------------------------------------------------------------------------

/** `32:14`, ou `1:02:09` depois de uma hora. */
export function relogio(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const pad = (value: number) => String(value).padStart(2, "0");
  return h ? `${h}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`;
}

/** `48 min`, `1h12`, `menos de 1 min`. Nunca segundos: numa reunião são ruído. */
export function duracao(ms: number): string {
  const minutos = Math.round(ms / 60000);
  if (minutos < 1) return "menos de 1 min";
  const horas = Math.floor(minutos / 60);
  return horas ? `${horas}h${String(minutos % 60).padStart(2, "0")}` : `${minutos} min`;
}

// ---------------------------------------------------------------------------
// Fase
// ---------------------------------------------------------------------------

const ROTULO_DA_FASE: Record<MeetingPhase, string> = {
  recording: "Gravando",
  finalizing: "Salvando",
  processing: "Organizando",
  ready: "Pronta",
  partially_ready: "Transcrição pronta",
  recovered: "Recuperada",
  needs_attention: "Precisa de você",
  failed_recoverable: "Precisa de você",
  discarded: "Descartada",
};

export function rotuloDaFase(fase: MeetingPhase): string {
  return ROTULO_DA_FASE[fase];
}

/** Em qual seção da lista a reunião mora. */
export type SecaoDaLista = "em_andamento" | "atencao" | "recentes";

export function secaoDa(linha: MeetingOverview): SecaoDaLista {
  switch (linha.phase) {
    case "recording":
    case "finalizing":
    case "processing":
      return "em_andamento";
    case "needs_attention":
    case "failed_recoverable":
    case "recovered":
      return "atencao";
    default:
      return linha.pendingActions > 0 ? "atencao" : "recentes";
  }
}

/** A linha de contagem do cartão: `3 ações para revisar · 2 decisões · 1 aguardando`. */
export function linhaDeContagem(linha: MeetingOverview): string {
  const partes: string[] = [];
  const plural = (n: number, um: string, varios: string) => `${n} ${n === 1 ? um : varios}`;
  if (linha.pendingActions > 0) partes.push(plural(linha.pendingActions, "ação para revisar", "ações para revisar"));
  if (linha.tasksCreated > 0) partes.push(plural(linha.tasksCreated, "Task", "Tasks"));
  if (linha.decisions > 0) partes.push(plural(linha.decisions, "decisão", "decisões"));
  if (linha.waiting > 0) partes.push(`${linha.waiting} aguardando`);
  return partes.join(" · ");
}

/** O selo de estado do cartão. Pronta sem pendência não precisa de selo alto. */
export function seloDoCartao(linha: MeetingOverview, progressoAoVivo?: number | null): string {
  switch (linha.phase) {
    case "processing": {
      const fracao = progressoAoVivo ?? linha.progress.fraction;
      return fracao == null ? "Organizando" : `Organizando · ${Math.round(fracao * 100)}%`;
    }
    case "ready":
      return "✓ Pronta";
    default:
      return rotuloDaFase(linha.phase);
  }
}

// ---------------------------------------------------------------------------
// Progresso
// ---------------------------------------------------------------------------

const PASSO: Record<PipelineProgress["steps"][number]["key"], { feito: string; fazendo: string }> = {
  saved: { feito: "Gravação salva", fazendo: "Salvando a gravação" },
  audio: { feito: "Áudio preparado", fazendo: "Preparando o áudio" },
  transcription: { feito: "Transcrição concluída", fazendo: "Transcrevendo" },
  analysis: { feito: "Decisões e tarefas identificadas", fazendo: "Identificando decisões e tarefas" },
  ready: { feito: "Pronta", fazendo: "Finalizando" },
};

export type PassoVisivel = { chave: string; texto: string; estado: PipelineProgress["steps"][number]["state"] };

/** A lista `✓ Gravação salva / ● Transcrevendo / ○ Finalizando`. */
export function passosVisiveis(progresso: PipelineProgress): PassoVisivel[] {
  return progresso.steps.map((passo) => ({
    chave: passo.key,
    estado: passo.state,
    texto: passo.state === "done" ? PASSO[passo.key].feito
      : passo.state === "waiting" ? `${PASSO[passo.key].fazendo} — vai tentar de novo`
        : passo.state === "failed" ? `${PASSO[passo.key].fazendo} — pendente`
          : PASSO[passo.key].fazendo,
  }));
}

export function marcaDoPasso(estado: PassoVisivel["estado"]): string {
  switch (estado) {
    case "done": return "✓";
    case "active": return "●";
    case "waiting": return "◌";
    case "failed": return "!";
    default: return "○";
  }
}

// ---------------------------------------------------------------------------
// Itens
// ---------------------------------------------------------------------------

const MINHAS: InsightKind[] = ["my_action", "follow_up", "deadline"];
const DE_OUTROS: InsightKind[] = ["other_action", "commitment", "dependency"];

/** Uma linha da revisão em lote. */
export type LinhaDeRevisao = {
  item: MeetingInsight;
  /** Confiança baixa não tem caixa: é só um registro. */
  selecionavel: boolean;
  /** Alta vem marcada; média desmarcada, pedindo revisão. */
  marcadaDeInicio: boolean;
  revisar: boolean;
};

function linhaDeRevisao(item: MeetingInsight): LinhaDeRevisao {
  const temProcedencia = item.evidence.length > 0 || item.origin !== "spoken";
  const selecionavel = item.confidence !== "low" && temProcedencia;
  return {
    item,
    selecionavel,
    marcadaDeInicio: selecionavel && item.confidence === "high",
    revisar: selecionavel && item.confidence === "medium",
  };
}

/** "O que exige sua atenção": as ações minhas ainda propostas. */
export function exigeAtencao(itens: MeetingInsight[]): LinhaDeRevisao[] {
  return itens
    .filter((item) => item.status === "proposed" && MINHAS.includes(item.kind))
    .map(linhaDeRevisao);
}

/** "Aguardando outras pessoas". */
export function aguardandoOutros(itens: MeetingInsight[]): LinhaDeRevisao[] {
  return itens
    .filter((item) => item.status === "proposed" && DE_OUTROS.includes(item.kind))
    .map(linhaDeRevisao);
}

/** Os que já viraram Task, para a pessoa ver que não precisa fazer de novo. */
export function jaCriados(itens: MeetingInsight[]): MeetingInsight[] {
  return itens.filter((item) => item.status === "accepted" && item.createdTaskId);
}

export function doTipo(itens: MeetingInsight[], tipos: InsightKind[]): MeetingInsight[] {
  return itens.filter((item) => item.status !== "dismissed" && tipos.includes(item.kind));
}

/** `Criar 2 tarefas`. */
export function rotuloDoLote(quantas: number): string {
  if (quantas === 0) return "Nada marcado";
  return quantas === 1 ? "Criar 1 tarefa" : `Criar ${quantas} tarefas`;
}

const DIAS = ["domingo", "segunda", "terça", "quarta", "quinta", "sexta", "sábado"];

/**
 * `hoje`, `amanhã`, `sexta`, `20/09`, `sem prazo`.
 *
 * Até seis dias à frente, o nome do dia — é como se fala. Depois disso, a data:
 * "terça" a dez dias de distância seria ambíguo.
 */
export function rotuloDePrazo(iso: string | null, agora = new Date()): string {
  if (!iso) return "sem prazo";
  const prazo = new Date(iso);
  const inicio = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  const dias = Math.round((inicio(prazo) - inicio(agora)) / 86_400_000);
  if (dias < 0) return dias === -1 ? "ontem" : `venceu ${prazo.toLocaleDateString("pt-BR", { day: "2-digit", month: "2-digit" })}`;
  if (dias === 0) return "hoje";
  if (dias === 1) return "amanhã";
  if (dias < 7) return DIAS[prazo.getDay()];
  return prazo.toLocaleDateString("pt-BR", { day: "2-digit", month: "2-digit" });
}

/** `2026-09-18` para o `<input type="date">`, no fuso local. */
export function dataDoInput(iso: string | null): string {
  if (!iso) return "";
  const d = new Date(iso);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** Do `<input type="date">` para um instante às 18h locais (o fim do expediente). */
export function instanteDoInput(valor: string): Date | null {
  if (!valor) return null;
  const [ano, mes, dia] = valor.split("-").map(Number);
  if (!ano || !mes || !dia) return null;
  return new Date(ano, mes - 1, dia, 18, 0, 0, 0);
}

export function rotuloDaOrigem(item: MeetingInsight): string {
  switch (item.origin) {
    case "written": return "escrito por você";
    case "manual": return "marcado por você";
    default: return "";
  }
}

// ---------------------------------------------------------------------------
// Recording Guardian
// ---------------------------------------------------------------------------

export type CopyDoGuardian = {
  titulo: string;
  corpo: string;
  /** O gesto de encerrar. Quando há excesso relevante, ele já corta. */
  encerrar: string;
  continuar: string;
  /** O fim provável para cortar ao encerrar, quando vale. */
  cortarEm: number | null;
};

/** Excesso a partir do qual o encerrar oferece ignorar o fim. */
export const EXCESSO_PARA_CORTAR_MS = 5 * 60 * 1000;

/**
 * O que a barra e a janelinha dizem.
 *
 * A pergunta nunca bloqueia nada. "Encerrar" já leva o corte quando o Guardian
 * sabe onde a conversa acabou e sobrou mais de cinco minutos — é o caso da
 * gravação esquecida, e fazer a pessoa cortar depois seria um passo a mais.
 */
export function copyDoGuardian(view: GuardianView, duracaoMs: number): CopyDoGuardian | null {
  if (view.kind === "idle") return null;
  const cortarEm = view.suggestedEndMs != null && view.excessMs >= EXCESSO_PARA_CORTAR_MS
    ? view.suggestedEndMs
    : null;
  const encerrar = cortarEm != null
    ? `Encerrar e ignorar os últimos ${duracao(view.excessMs)}`
    : "Encerrar gravação";

  if (view.kind === "countdown") {
    return {
      titulo: "Parece que a reunião terminou.",
      corpo: `A gravação será encerrada em ${view.secondsLeft} s.`,
      encerrar: "Encerrar agora",
      continuar: "Continuar gravando",
      cortarEm,
    };
  }
  if (view.prompt === "long") {
    return {
      titulo: `Esta reunião está sendo gravada há ${duracao(duracaoMs)}.`,
      corpo: "Ela ainda está acontecendo?",
      encerrar: "Encerrar",
      continuar: "Sim, continuar",
      cortarEm,
    };
  }
  return {
    titulo: "Parece que sua reunião terminou.",
    corpo: cortarEm != null
      ? `A conversa parece ter acabado há ${duracao(view.excessMs)}.`
      : "A chamada deixou de usar o microfone.",
    encerrar,
    continuar: "Continuar gravando",
    cortarEm,
  };
}

// ---------------------------------------------------------------------------
// Transcrição
// ---------------------------------------------------------------------------

export type FiltroDaTranscricao = {
  busca: string;
  canal: "todos" | "mic" | "system";
  soMarcados: boolean;
  soComItens: boolean;
};

/** Um trecho é "marcado" quando um momento cai dentro dele (com 5 s de folga). */
export function trechoMarcado(trecho: TranscriptSegment, marcas: MeetingBookmark[]): boolean {
  return marcas.some((marca) => marca.atMs >= trecho.startMs - 5000 && marca.atMs <= trecho.endMs + 5000);
}

export function filtrarTranscricao(
  trechos: TranscriptSegment[],
  filtro: FiltroDaTranscricao,
  marcas: MeetingBookmark[],
  itens: MeetingInsight[],
): TranscriptSegment[] {
  const agulha = normalizar(filtro.busca.trim());
  const comItens = new Set(itens.flatMap((item) => item.evidence.map((e) => e.segmentId)));
  return trechos.filter((trecho) => {
    if (filtro.canal !== "todos" && trecho.channel !== filtro.canal) return false;
    if (filtro.soMarcados && !trechoMarcado(trecho, marcas)) return false;
    if (filtro.soComItens && !comItens.has(trecho.id)) return false;
    if (!agulha) return true;
    return normalizar(textoDoTrecho(trecho)).includes(agulha) || normalizar(trecho.text).includes(agulha);
  });
}

function normalizar(texto: string): string {
  return texto.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();
}

/** O texto que se lê: o normalizado quando existe, o cru quando não. */
export function textoDoTrecho(trecho: TranscriptSegment): string {
  return trecho.textNormalized ?? trecho.text;
}

export type PedacoDoTrecho = { texto: string; original?: string; incerto?: boolean };

/**
 * O texto em pedaços, para grifar as trocas do vocabulário.
 *
 * As posições das correções são em BYTES UTF-8 do texto normalizado (vêm do
 * Rust). A conversão para índice de string JS precisa passar pelos bytes — senão
 * cada "ã" antes da troca desloca o grifo uma letra.
 */
export function pedacosDoTrecho(trecho: TranscriptSegment): PedacoDoTrecho[] {
  const texto = textoDoTrecho(trecho);
  if (!trecho.textNormalized || trecho.corrections.length === 0) return [{ texto }];
  const bytes = new TextEncoder().encode(texto);
  const decoder = new TextDecoder();
  const pedacos: PedacoDoTrecho[] = [];
  let cursor = 0;
  for (const troca of [...trecho.corrections].sort((a, b) => a.start - b.start)) {
    if (troca.start < cursor || troca.end > bytes.length) continue;
    if (troca.start > cursor) pedacos.push({ texto: decoder.decode(bytes.slice(cursor, troca.start)) });
    pedacos.push({
      texto: decoder.decode(bytes.slice(troca.start, troca.end)),
      original: troca.original,
      incerto: troca.uncertain,
    });
    cursor = troca.end;
  }
  if (cursor < bytes.length) pedacos.push({ texto: decoder.decode(bytes.slice(cursor)) });
  return pedacos;
}

// ---------------------------------------------------------------------------
// Corte
// ---------------------------------------------------------------------------

/** A frase da sugestão de corte. */
export function fraseDoCorte(duracaoMs: number, fimProvavelMs: number): string {
  const sobra = Math.max(0, duracaoMs - fimProvavelMs);
  return `Detectamos ${duracao(sobra)} depois do provável fim da reunião.`;
}

/** Arredonda para o segundo, e mantém dentro da gravação. */
export function limitarCorte(inicio: number, fim: number, duracaoMs: number): [number, number] {
  const a = Math.max(0, Math.min(Math.round(inicio / 1000) * 1000, duracaoMs));
  const b = Math.max(0, Math.min(Math.round(fim / 1000) * 1000, duracaoMs));
  return a < b ? [a, b] : [Math.max(0, b - 1000), b];
}
