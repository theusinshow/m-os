/**
 * A conversa com o servidor.
 *
 * Um arquivo, e todas as chamadas nele — mesma disciplina do `api.ts` do
 * desktop. O resto da interface não sabe que existe rede.
 */

export type Capture = {
  id: string;
  content: string;
  capturedAt: string;
};

export type EstadoDaTask =
  | "inbox"
  | "backlog"
  | "planned"
  | "doing"
  | "review"
  | "done";

/** A mesma escala do lembrete, e não uma segunda. */
export type PrioridadeDaTask = "low" | "normal" | "high" | "urgent";

export type Task = {
  id: string;
  title: string;
  description: string;
  state: EstadoDaTask;
  projectId: string | null;
  lifecycleState: "active" | "archived" | "trashed";
  /** Quando o trabalho VENCE. Não é o lembrete — ver a ADR-066. */
  dueAt: string | null;
  priority: PrioridadeDaTask;
  estimateMinutes: number | null;
  parentTaskId: string | null;
  blockedByTaskId: string | null;
  waitingFor: string;
  followUpAt: string | null;
  /* Os dois números do progresso vêm DENTRO da Task. A lista de Fazer desenha
     `3/6` sem pedir o checklist de cada linha — no 4G da rua, uma chamada por
     task seria a diferença entre a tela abrir e a tela travar. */
  checklistTotal: number;
  checklistDone: number;
  createdAt: string;
  completedAt: string | null;
};

/** Um passo dentro de uma Task. */
export type ItemDeChecklist = {
  id: string;
  taskId: string;
  label: string;
  position: number;
  completedAt: string | null;
};

/** A Task com tudo que a tela de detalhe mostra, numa ida só. */
export type DetalheDaTask = {
  task: Task;
  checklist: ItemDeChecklist[];
  subtasks: Task[];
  blockedBy: Task | null;
  references: { id: string; title: string; url: string }[];
  reminders: Lembrete[];
};

/** O dia: o Start My Day visto do bolso. */
export type ODia = {
  status: "not_started" | "active" | "ended";
  objetivos: {
    id: string;
    titulo: string;
    status: "pending" | "done" | "dropped" | "carried";
    prioridade: string;
  }[];
  /** Quantos objetivos já foram resolvidos — o numerador do anel. */
  resolvidos: number;
  /** Tasks concluídas hoje. Não é o mesmo que objetivos. */
  feitasHoje: number;
  /** Dias seguidos com o dia encerrado. */
  sequencia: number;
};

/** Um projeto, só com o que a tela do bolso precisa saber dele. */
/**
 * O que se consulta, e nao o que se faz — um `Resource` do M/OS.
 *
 * `title` NUNCA vem vazio: o `mos-core` troca titulo em branco pela propria
 * URL, e so recusa o vazio quando nao ha URL para servir de fallback. Entao a
 * tela nao testa "tem titulo?", e sim "o titulo E a url?" — e nesse caso mostra
 * o dominio, que e mais curto e diz a mesma coisa.
 */
export type Referencia = {
  id: string;
  kind: "site" | "library" | "image" | "note" | "file";
  title: string;
  url: string;
  note: string;
  createdAt: string;
};

export type Projeto = {
  id: string;
  name: string;
};

/** O que se manda para editar uma Task. Ausente é "não mexi". */
export type EdicaoDeTask = {
  titulo?: string;
  descricao?: string;
  /** `null` desliga o projeto. Ausente deixa como está. */
  projectId?: string | null;
  /** `null` TIRA o prazo. Ausente deixa como está — é a mesma dupla-opção. */
  prazo?: string | null;
  prioridade?: PrioridadeDaTask;
  estimativaMinutos?: number | null;
  aguardando?: string;
  cobrarEm?: string | null;
};

/**
 * A entidade a que um lembrete se prende, quando se prende.
 *
 * So `task` aparece nesta superficie hoje, e o tipo continua largo de proposito:
 * o dominio tem sete bracos, e estreitar aqui faria a tela mentir sobre o que o
 * servidor devolve quando um lembrete criado no PC aponta para outra coisa.
 */
export type AlvoDoLembrete = {
  type: "task" | "project" | "capture" | "resource" | "conversation" | "app" | "meeting";
  id: string;
};

export type EstadoDoLembrete =
  | "scheduled"
  | "due"
  | "delivered"
  | "acknowledged"
  | "snoozed"
  | "completed"
  | "cancelled"
  | "missed"
  | "expired";

/** Que pergunta a tela faz sobre este lembrete. */
export type TipoDeLembrete = "standard" | "follow_up";

/* A regra de repeticao, como o dominio a guarda.

   Hora e minuto sao LOCAIS, mais o deslocamento em que a regra nasceu: "todo dia
   as 08:00" quer dizer oito da manha onde a pessoa esta. Ver
   `crates/mos-core/src/recurrence.rs`. */
export type RegraDeRepeticao =
  | { kind: "daily" }
  | { kind: "weekdays" }
  | { kind: "weekly"; days: number[] }
  | { kind: "monthly"; day: { kind: string; day?: number; weekday?: number; ordinal?: number } }
  | { kind: "yearly"; month: number; day: number }
  | { kind: "everyDays"; days: number }
  | { kind: "everyWeeks"; weeks: number };

export type Repeticao = {
  rule: RegraDeRepeticao;
  anchor: "fixed" | "completion";
  hour: number;
  minute: number;
  offsetMinutes: number;
};

export type Lembrete = {
  id: string;
  title: string;
  body: string;
  target: AlvoDoLembrete | null;
  status: EstadoDoLembrete;
  priority: "low" | "normal" | "high" | "urgent";
  /** Quando vence — ou quando venceu. `null` e "algum dia". RFC 3339. */
  nextDueAt: string | null;
  snoozeCount: number;
  kind: TipoDeLembrete;
  waitingFor: string;
  /** "Nao me deixa esquecer": volta a cobrar ate ser resolvido. */
  persistent: boolean;
  recurrence: Repeticao | null;
  createdAt: string;
  updatedAt: string;
  lifecycleState: "active" | "archived" | "trashed";
};

/** Por que um lembrete esta sendo esquecido. Vem decidido do dominio. */
export type MotivoDeAtencao =
  | "missed"
  | "overdue"
  | "ignored"
  | "snooze_fatigue"
  | "persistent"
  | "high_priority"
  | "carried_over";

export type LinhaDeAtencao = {
  reminder: Lembrete;
  reasons: MotivoDeAtencao[];
  weight: number;
};

/** Como cada motivo se le na tela. */
export const MOTIVO: Record<MotivoDeAtencao, string> = {
  missed: "perdido",
  overdue: "atrasado",
  ignored: "ignorado",
  snooze_fatigue: "adiado demais",
  persistent: "nao deixar esquecer",
  high_priority: "prioridade alta",
  carried_over: "veio de ontem",
};

/** O que se manda para editar. Campo ausente é "não mexi" — não "apague". */
export type EdicaoDeLembrete = {
  titulo?: string;
  nota?: string;
  quando?: Date;
  prioridade?: Lembrete["priority"];
};

/** O que ainda espera uma acao da pessoa. E o que o badge conta. */
export function pedeAtencao(lembrete: Lembrete): boolean {
  return (
    lembrete.status === "due" ||
    lembrete.status === "delivered" ||
    lembrete.status === "missed"
  );
}

/** O que a Home mostra além do que ela já tinha.
 *
 *  Uma chamada só, e não três: o celular abre no 4G, e cada ida à rede é um
 *  segundo de tela vazia. */
export type Panorama = {
  horas: {
    /** Segundos faturáveis da semana, já arredondados por sessão. */
    semanaSegundos: number;
    semanaValorCents: number;
    hojeSegundos: number;
    /** Os sete dias, de segunda a domingo. Servidor antigo não manda: por isso
     *  opcional, e a tela desenha o cartão sem a semana quando falta. */
    diasSegundos?: number[];
  };
  /** Até três, do mais próximo para o mais distante. */
  proximos: { titulo: string; disciplina: string; quando: string; tipo: string }[];
};

/** Um item da agenda, como o `mos_core::compose` o devolve.
 *
 *  `kind` é largo de propósito: o domínio tem doze tipos e o bolso desenha os
 *  que conhece, ignorando o resto. Estreitar aqui faria a tela quebrar no dia em
 *  que o desktop passasse a compor um tipo novo. */
export type ItemDaAgenda = {
  kind: string;
  /** RFC3339. */
  at: string;
  endsAt: string | null;
  title: string;
  projectId: string | null;
  /** Zero quando o item não tem duração. */
  seconds: number;
  /** Zero quando não é hora cobrável. */
  amountCents: number;
};

/** As horas de um projeto na janela pedida. */
export type HorasDeProjeto = {
  projeto: string;
  segundos: number;
  valorCents: number;
  /** Quantos lançamentos somaram isso — o número que separa "um dia inteiro" de
   *  "vinte visitas de dez minutos". */
  lancamentos: number;
};

/** Um compromisso do acadêmico. `urgencia`: `atrasado`, `hoje`, ou vazio. */
export type CompromissoDaLista = {
  titulo: string;
  disciplina: string;
  quando: string;
  tipo: string;
  urgencia: string;
};

export type EstadoDoAparelho = {
  pendentes: number;
  sincroniza: boolean;
  /** A chave pública VAPID, ou `null` quando este servidor não notifica. */
  chavePush: string | null;
  /** Quantos aparelhos já assinaram. É a prova de que "ativar" funcionou. */
  aparelhosAvisados: number;
};

/** O `PushSubscription.toJSON()` do navegador, repassado inteiro. */
export type AssinaturaPush = {
  endpoint: string;
  keys: { p256dh: string; auth: string };
};

/**
 * O erro chega como `{ erro }` do servidor; o que aparece na tela é essa
 * frase, e não "Failed to fetch". A pessoa está na rua com uma ideia na
 * cabeça — ela precisa saber se deve tentar de novo ou se perdeu.
 */
/**
 * O 401 tem tratamento próprio.
 *
 * Ele não é um erro para mostrar numa linha de recado: é a informação de que a
 * tela inteira deveria ser outra. Um `Error` comum aqui viraria "Entre para
 * continuar." escrito embaixo de um app que a pessoa não consegue usar.
 */
export class SemSessao extends Error {
  constructor() {
    super("Entre para continuar.");
    this.name = "SemSessao";
  }
}

async function pedir<T>(caminho: string, init?: RequestInit): Promise<T> {
  let resposta: Response;
  try {
    resposta = await fetch(caminho, {
      ...init,
      headers: { "content-type": "application/json", ...(init?.headers ?? {}) },
    });
  } catch {
    throw new Error("Sem conexão com o M/OS.");
  }
  if (resposta.status === 401) throw new SemSessao();
  if (!resposta.ok) {
    const corpo = await resposta.json().catch(() => null);
    throw new Error(corpo?.erro ?? `O servidor respondeu ${resposta.status}.`);
  }
  return (await resposta.json()) as T;
}

export const api = {
  capturar(texto: string) {
    return pedir<{ id: string }>("/api/capturar", {
      method: "POST",
      body: JSON.stringify({ texto }),
    });
  },
  inbox() {
    return pedir<Capture[]>("/api/inbox");
  },
  /** O panorama, com o instante DESTE aparelho.
   *
   *  O fuso viaja junto de propósito: o servidor roda em UTC, e cortar a semana
   *  pelo relógio dele terminaria a semana às 21h de sábado para quem lê. */
  /** O que o M/OS registrou entre dois instantes.
   *
   *  A janela vai em RFC3339 com o offset deste aparelho, pela mesma razão do
   *  panorama: onde um dia começa é decisão de quem olha. */
  agenda(desde: Date, ate: Date) {
    const parametros = new URLSearchParams({
      desde: comOffsetLocal(desde),
      ate: comOffsetLocal(ate),
    });
    return pedir<ItemDaAgenda[]>(`/api/agenda?${parametros}`);
  },
  /** As horas da janela, por projeto, do maior para o menor. */
  horas(desde: Date, ate: Date) {
    const parametros = new URLSearchParams({
      desde: comOffsetLocal(desde),
      ate: comOffsetLocal(ate),
    });
    return pedir<HorasDeProjeto[]>(`/api/horas?${parametros}`);
  },
  /** O que vem por aí no acadêmico. Atrasado primeiro. */
  academico() {
    return pedir<CompromissoDaLista[]>(
      `/api/academico?agora=${encodeURIComponent(comOffsetLocal(new Date()))}`,
    );
  },
  panorama() {
    return pedir<Panorama>(
      `/api/panorama?agora=${encodeURIComponent(comOffsetLocal(new Date()))}`,
    );
  },
  tasks() {
    return pedir<Task[]>("/api/tasks");
  },
  detalheDaTask(id: string) {
    return pedir<DetalheDaTask>(`/api/tasks/${id}/detalhe`);
  },
  /** Uma linha vira um passo; várias linhas coladas viram vários. */
  criarItem(taskId: string, texto: string) {
    return pedir<Task>(`/api/tasks/${taskId}/checklist`, {
      method: "POST",
      body: JSON.stringify({ texto }),
    });
  },
  marcarItem(id: string, feito: boolean) {
    return pedir<Task>(`/api/checklist/${id}`, {
      method: "PATCH",
      body: JSON.stringify({ feito }),
    });
  },
  renomearItem(id: string, texto: string) {
    return pedir<Task>(`/api/checklist/${id}`, {
      method: "PATCH",
      body: JSON.stringify({ texto }),
    });
  },
  apagarItem(id: string) {
    return pedir<Task>(`/api/checklist/${id}`, { method: "DELETE" });
  },
  reordenarChecklist(taskId: string, ids: string[]) {
    return pedir<ItemDeChecklist[]>(`/api/tasks/${taskId}/checklist/ordem`, {
      method: "POST",
      body: JSON.stringify({ ids }),
    });
  },
  criarTask(titulo: string) {
    return pedir<Task>("/api/tasks", {
      method: "POST",
      body: JSON.stringify({ titulo }),
    });
  },
  mudarEstado(id: string, estado: Task["state"]) {
    return pedir<Task>(`/api/tasks/${id}/estado`, {
      method: "POST",
      body: JSON.stringify({ estado }),
    });
  },
  task(id: string) {
    return pedir<Task>(`/api/tasks/${id}`);
  },
  editarTask(id: string, mudanca: EdicaoDeTask) {
    return pedir<Task>(`/api/tasks/${id}`, {
      method: "PATCH",
      // Só o que mudou. O servidor completa o resto com o que está gravado —
      // não com o que esta tela leu quando abriu.
      body: JSON.stringify({
        ...(mudanca.titulo !== undefined ? { titulo: mudanca.titulo } : {}),
        ...(mudanca.descricao !== undefined ? { descricao: mudanca.descricao } : {}),
        ...(mudanca.projectId !== undefined ? { projectId: mudanca.projectId } : {}),
      }),
    });
  },
  arquivarTask(id: string) {
    return pedir<Task>(`/api/tasks/${id}/arquivar`, { method: "POST" });
  },
  projetos() {
    return pedir<Projeto[]>("/api/projetos");
  },
  /** A Capture vira Task. Sai da inbox na mesma transação. */
  capturaParaTask(id: string, titulo?: string) {
    return pedir<Task>(`/api/capturas/${id}/task`, {
      method: "POST",
      body: JSON.stringify(titulo === undefined ? {} : { titulo }),
    });
  },
  /** A Capture vira referência — o que se consulta, e não o que se faz. */
  capturaParaReferencia(id: string) {
    return pedir<Referencia>(
      `/api/capturas/${id}/referencia`,
      { method: "POST", body: JSON.stringify({}) },
    );
  },
  arquivarCaptura(id: string) {
    return pedir<Capture>(`/api/capturas/${id}/arquivar`, { method: "POST" });
  },
  dia() {
    return pedir<ODia>(`/api/dia?agora=${encodeURIComponent(comOffsetLocal(new Date()))}`);
  },
  /**
   * Cria um lembrete.
   *
   * `quando` viaja como instante JA RESOLVIDO, e o calculo de "amanha de manha"
   * acontece AQUI de proposito: este servidor roda numa VPS cujo fuso nao e o de
   * quem tocou no botao, e meia-noite em UTC e nove da noite no Brasil. Mesmo
   * caminho que o `ReminderComposer` do desktop segue.
   */
  /**
   * Criar lembrete, com tudo que ele pode ser.
   *
   * `quando: null` e "algum dia" — o lembrete existe e nao interrompe.
   */
  novoLembrete(pedido: {
    titulo: string;
    quando: Date | null;
    nota?: string;
    alvo?: AlvoDoLembrete;
    persistente?: boolean;
    aguardando?: string;
    repeticao?: Repeticao | null;
    adiantamentos?: number[];
  }) {
    return pedir<Lembrete>("/api/lembretes", {
      method: "POST",
      body: JSON.stringify({
        titulo: pedido.titulo,
        nota: pedido.nota ?? "",
        quando: pedido.quando ? pedido.quando.toISOString() : null,
        alvo_tipo: pedido.alvo?.type,
        alvo_id: pedido.alvo?.id,
        persistente: pedido.persistente ?? false,
        aguardando: pedido.aguardando,
        repeticao: pedido.repeticao ?? null,
        adiantamentos: pedido.adiantamentos ?? [],
      }),
    });
  },
  /** A estante: o que voce guardou para consultar. Mais novo primeiro. */
  biblioteca() {
    return pedir<Referencia[]>("/api/biblioteca");
  },
  /** Cola o endereco e pronto. Titulo vazio e valido — o servidor cai na URL. */
  guardarNaBiblioteca(url: string, titulo = "", nota = "") {
    return pedir<Referencia>("/api/biblioteca", {
      method: "POST",
      body: JSON.stringify({ url, titulo, nota }),
    });
  },
  /** Tira da estante sem apagar. */
  arquivarDaBiblioteca(id: string) {
    return pedir<Referencia>(`/api/biblioteca/${id}/arquivar`, { method: "POST" });
  },

  /** O que esta sendo esquecido, com o motivo de cada um. */
  lembretesEmAtencao() {
    return pedir<LinhaDeAtencao[]>("/api/lembretes/atencao");
  },
  alertasDoLembrete(id: string) {
    return pedir<{ id: string; scheduledAt: string; kind: string; leadMinutes: number | null; status: string }[]>(
      `/api/lembretes/${id}/alertas`,
    );
  },
  criarLembrete(titulo: string, quando: Date, nota = "", alvo?: AlvoDoLembrete) {
    return pedir<Lembrete>("/api/lembretes", {
      method: "POST",
      body: JSON.stringify({
        titulo,
        nota,
        quando: quando.toISOString(),
        alvo_tipo: alvo?.type,
        alvo_id: alvo?.id,
      }),
    });
  },
  lembretes() {
    return pedir<Lembrete[]>("/api/lembretes");
  },
  concluirLembrete(id: string) {
    return pedir<Lembrete>(`/api/lembretes/${id}/concluir`, { method: "POST" });
  },
  cancelarLembrete(id: string) {
    return pedir<Lembrete>(`/api/lembretes/${id}/cancelar`, { method: "POST" });
  },
  lembrete(id: string) {
    return pedir<Lembrete>(`/api/lembretes/${id}`);
  },
  /** O histórico: o que já foi concluído, cancelado ou expirou. */
  lembretesResolvidos() {
    return pedir<Lembrete[]>("/api/lembretes/resolvidos");
  },
  editarLembrete(id: string, mudanca: EdicaoDeLembrete) {
    return pedir<Lembrete>(`/api/lembretes/${id}`, {
      method: "PATCH",
      // Só o que foi mexido viaja. Mandar o objeto inteiro faria a tela que
      // editou o título reescrever também a hora — com o valor que ela leu
      // antes — e o sync não teria como saber que aquilo não foi uma edição.
      body: JSON.stringify({
        ...(mudanca.titulo !== undefined ? { titulo: mudanca.titulo } : {}),
        ...(mudanca.nota !== undefined ? { nota: mudanca.nota } : {}),
        ...(mudanca.quando ? { quando: comOffsetLocal(mudanca.quando) } : {}),
        ...(mudanca.prioridade ? { prioridade: mudanca.prioridade } : {}),
      }),
    });
  },
  adiarLembrete(id: string, ate: Date) {
    return pedir<Lembrete>(`/api/lembretes/${id}/adiar`, {
      method: "POST",
      body: JSON.stringify({ ate: comOffsetLocal(ate) }),
    });
  },
  /** Arquivar é o "excluir" da tela: some da lista, a linha continua. */
  arquivarLembrete(id: string) {
    return pedir<Lembrete>(`/api/lembretes/${id}/arquivar`, { method: "POST" });
  },
  estado() {
    return pedir<EstadoDoAparelho>("/api/estado");
  },
  assinarPush(assinatura: AssinaturaPush) {
    // O servidor espera os três campos rasos; o navegador entrega as chaves
    // aninhadas em `keys`. Achatar aqui e não lá mantém o formato do servidor
    // igual ao que os testes usam, sem um nível de objeto que só existe porque
    // a API do navegador é assim.
    return pedir<{ ok: boolean }>("/api/push/assinar", {
      method: "POST",
      body: JSON.stringify({
        endpoint: assinatura.endpoint,
        p256dh: assinatura.keys.p256dh,
        auth: assinatura.keys.auth,
      }),
    });
  },
  testarPush() {
    return pedir<{ enviadas: number }>("/api/push/testar", { method: "POST" });
  },
};

/**
 * `2026-09-04T14:30:00-03:00` — o instante com o fuso DESTE aparelho.
 *
 * O `toISOString` devolve UTC, e é justamente o que não serve: o servidor roda
 * em UTC e precisa saber onde o dia começa para quem está olhando.
 */
function comOffsetLocal(quando: Date): string {
  const minutos = -quando.getTimezoneOffset();
  const sinal = minutos >= 0 ? "+" : "-";
  const doisDigitos = (numero: number) =>
    String(Math.floor(Math.abs(numero))).padStart(2, "0");
  const offset = `${sinal}${doisDigitos(minutos / 60)}:${doisDigitos(minutos % 60)}`;
  const local = new Date(quando.getTime() - quando.getTimezoneOffset() * 60_000)
    .toISOString()
    .slice(0, 19);
  return local + offset;
}
