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

export type Task = {
  id: string;
  title: string;
  description: string;
  state: "inbox" | "backlog" | "planned" | "doing" | "review" | "done";
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

export type Lembrete = {
  id: string;
  title: string;
  body: string;
  target: AlvoDoLembrete | null;
  status: EstadoDoLembrete;
  priority: "low" | "normal" | "high" | "urgent";
  /** Quando vence — ou quando venceu. RFC 3339. */
  nextDueAt: string | null;
  snoozeCount: number;
  createdAt: string;
  updatedAt: string;
  lifecycleState: "active" | "archived" | "trashed";
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
  /**
   * Cria um lembrete.
   *
   * `quando` viaja como instante JA RESOLVIDO, e o calculo de "amanha de manha"
   * acontece AQUI de proposito: este servidor roda numa VPS cujo fuso nao e o de
   * quem tocou no botao, e meia-noite em UTC e nove da noite no Brasil. Mesmo
   * caminho que o `ReminderComposer` do desktop segue.
   */
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
