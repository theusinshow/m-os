/**
 * A Daily Session do lado da tela: só o que dá para verificar.
 *
 * Não há teste de DOM neste repositório, por decisão registrada no
 * `vitest.config.ts`, e a consequência prática é esta: o que precisa ser
 * conferido tem de ser função pura. Então tudo que decide alguma coisa — em que
 * estado a Home está, o que o resumo do dia diz, quantas vagas de foco sobraram,
 * que ordem os objetivos têm, o que uma linha do dia deve dizer — mora aqui, e
 * os componentes só desenham o resultado.
 *
 * **Nenhuma regra de DOMÍNIO mora aqui.** Progresso, carry-over, conclusão
 * automática e unicidade do dia vivem em `mos-core::daily`, com teste. O que
 * este arquivo carrega é regra de APRESENTAÇÃO: como um número vira frase, o que
 * a tela mostra primeiro, e quando um controle aparece.
 */
import type {
  CarryOver,
  DailyContext,
  DailyObjective,
  DailySessionSummary,
  DailyToday,
  DayMood,
  ObjectiveDraft,
  ObjectiveStatus,
  ObjectiveLink,
} from "./types";

/**
 * Quantos secundários a interface incentiva.
 *
 * Espelha `SUGGESTED_SECONDARIES` do domínio. **Não é trava**: o banco aceita
 * mais, e o pedido pede exatamente isso — a UX incentiva foco, a estrutura não
 * bloqueia. Uma trava aqui transformaria um bom conselho num erro.
 */
export const SECUNDARIOS_SUGERIDOS = 3;

/** Em que estado a Home está, e é a única pergunta que ela faz primeiro. */
export type EstadoDoDia =
  /** Ninguém começou o dia, e não há dia anterior em aberto. */
  | { tipo: "nao_iniciado" }
  /** Não começou hoje, e ontem ficou aberto. O §24 do pedido por nome. */
  | { tipo: "ontem_aberto"; dia: string; pendentes: number }
  | { tipo: "ativo"; feitos: number; total: number }
  | { tipo: "encerrado"; feitos: number; total: number };

/**
 * Resolve o estado da Home a partir do dia.
 *
 * A ordem dos testes importa: **"ontem em aberto" ganha de "não iniciado"**,
 * porque ignorar a porta aberta é o que faria o histórico mentir. Mas ele NÃO
 * ganha de "hoje já começou": depois de começar hoje, a sessão velha já foi
 * fechada pelo backend e não há mais nada a oferecer.
 */
export function estadoDoDia(hoje: DailyToday | null): EstadoDoDia {
  if (!hoje || hoje.status === "not_started") {
    const stale = hoje?.stale;
    if (!stale) return { tipo: "nao_iniciado" };
    return {
      tipo: "ontem_aberto",
      dia: stale.day,
      pendentes: (hoje?.staleObjectives ?? []).filter((objetivo) => objetivo.status === "pending").length,
    };
  }
  const { feitos, total } = progresso(hoje.objectives);
  return hoje.status === "active" ? { tipo: "ativo", feitos, total } : { tipo: "encerrado", feitos, total };
}

/**
 * Quantos concluídos, de quantos que contavam.
 *
 * `dropped` sai dos DOIS lados da fração — espelha `DailyToday::progress` no
 * domínio. Abandonar um objetivo não pode piorar o placar do dia, senão o
 * sistema ensina a não abandonar nada, que é o oposto do que o End My Day
 * existe para permitir.
 */
export function progresso(objetivos: DailyObjective[]): { feitos: number; total: number } {
  const contados = objetivos.filter((objetivo) => objetivo.status !== "dropped");
  return {
    feitos: contados.filter((objetivo) => objetivo.status === "completed").length,
    total: contados.length,
  };
}

/**
 * A ordem em que a Home e a sessão leem os objetivos: principal, depois posição.
 *
 * O backend já devolve nessa ordem. A função existe mesmo assim porque a tela
 * reordena OTIMISTA — o arrasto move a linha antes de o banco confirmar — e
 * nesse intervalo a lista é montada aqui.
 */
export function emOrdem(objetivos: DailyObjective[]): DailyObjective[] {
  return [...objetivos].sort((esquerda, direita) => {
    const peso = (objetivo: DailyObjective) => (objetivo.priority === "main" ? 0 : 1);
    return peso(esquerda) - peso(direita) || esquerda.position - direita.position;
  });
}

/**
 * Move um objetivo para antes de `antes`, ou para o fim quando ele é nulo.
 *
 * A mira é um VIZINHO, e não um índice, pelo mesmo motivo do
 * `moveInArrangement` da Home: a lista muda de tamanho quando o item sai dela, e
 * todo índice calculado antes da remoção erra por um em metade dos casos.
 */
export function moverObjetivo(objetivos: DailyObjective[], id: string, antes: string | null): DailyObjective[] {
  const atual = objetivos.find((objetivo) => objetivo.id === id);
  if (!atual) return objetivos;
  const resto = objetivos.filter((objetivo) => objetivo.id !== id);
  const destino = resto.findIndex((objetivo) => objetivo.id === antes);
  if (antes !== null && destino >= 0) {
    resto.splice(destino, 0, atual);
    return resto;
  }
  return [...resto, atual];
}

/** Quantas vagas de foco ainda cabem, pelo conselho e não pela trava. */
export function vagasRestantes(objetivos: DailyObjective[]): number {
  const secundarios = objetivos.filter(
    (objetivo) => objetivo.priority === "secondary" && objetivo.status !== "dropped",
  ).length;
  return Math.max(0, SECUNDARIOS_SUGERIDOS - secundarios);
}

// ---------------------------------------------------------------- o resumo

/** Uma linha do resumo do dia: um número e o que ele é. */
export type LinhaDeContexto = { chave: string; valor: number; texto: string };

/**
 * O resumo do dia, em frases.
 *
 * **Zero não vira linha.** Um painel que diz "0 tarefas atrasadas · 0
 * compromissos · 0 pendências" é ansiedade com cara de informação: ele ocupa a
 * tela inteira para dizer que não há nada. O pedido é explícito — dar contexto,
 * não criar ansiedade visual.
 *
 * A ORDEM é a da urgência, e não a da grandeza: atrasado primeiro, porque é o
 * que já falhou; hoje depois; e o que só é volume — Inbox, Projects — por
 * último. Ordenar pelo número faria uma Inbox de quarenta capturas empurrar
 * duas entregas atrasadas para o fim da frase.
 */
export function resumoDoDia(contexto: DailyContext | null): LinhaDeContexto[] {
  if (!contexto) return [];
  const linhas: LinhaDeContexto[] = [
    { chave: "overdue", valor: contexto.overdue, texto: plural(contexto.overdue, "lembrete atrasado", "lembretes atrasados") },
    { chave: "dueToday", valor: contexto.dueToday, texto: plural(contexto.dueToday, "lembrete para hoje", "lembretes para hoje") },
    { chave: "highPriority", valor: contexto.highPriority, texto: plural(contexto.highPriority, "item de prioridade alta", "itens de prioridade alta") },
    { chave: "doing", valor: contexto.doing, texto: plural(contexto.doing, "task em andamento", "tasks em andamento") },
    { chave: "meetingsToday", valor: contexto.meetingsToday, texto: plural(contexto.meetingsToday, "reunião registrada hoje", "reuniões registradas hoje") },
    { chave: "inbox", valor: contexto.inbox, texto: plural(contexto.inbox, "capture por processar", "captures por processar") },
    { chave: "openTasks", valor: contexto.openTasks, texto: plural(contexto.openTasks, "task aberta", "tasks abertas") },
    { chave: "projects", valor: contexto.projects, texto: plural(contexto.projects, "project ativo", "projects ativos") },
  ];
  return linhas.filter((linha) => linha.valor > 0);
}

/**
 * A saudação da Home sem sessão. Curta, e sem tom motivacional.
 *
 * O M/OS é funcional, silencioso e objetivo — o §21 do pedido e o
 * `UX-PRINCIPLES` dizem a mesma coisa. Então é a hora do dia, e nada além.
 */
export function saudacao(agora: Date): string {
  const hora = agora.getHours();
  if (hora < 12) return "Bom dia.";
  if (hora < 18) return "Boa tarde.";
  return "Boa noite.";
}

function plural(valor: number, singular: string, plural: string): string {
  return `${valor} ${valor === 1 ? singular : plural}`;
}

// ------------------------------------------------------------- os objetivos

/**
 * O que uma linha de objetivo mostra, já resolvido.
 *
 * O `marcador` não é só cor: `DESIGN-FOUNDATIONS.md` §14 proíbe estado que
 * dependa apenas dela, e a mesma lógica vale aqui — um objetivo levado para
 * amanhã e um abandonado precisam ser distinguíveis sem enxergar a diferença
 * entre dois cinzas.
 */
export type LinhaDeObjetivo = {
  id: string;
  titulo: string;
  principal: boolean;
  marcador: "●" | "○" | "✓" | "→" | "×";
  /** Vazio quando não há nada a dizer além do título. */
  estado: string;
  /** Para onde clicar leva, quando o objetivo aponta para algo. */
  link: ObjectiveLink | null;
  concluivel: boolean;
};

const MARCADOR: Record<ObjectiveStatus, LinhaDeObjetivo["marcador"]> = {
  pending: "○",
  completed: "✓",
  carried_over: "→",
  dropped: "×",
};

const ESTADO: Record<ObjectiveStatus, string> = {
  pending: "",
  completed: "concluído",
  carried_over: "levado para amanhã",
  dropped: "abandonado",
};

export function linhaDeObjetivo(objetivo: DailyObjective): LinhaDeObjetivo {
  const principal = objetivo.priority === "main";
  return {
    id: objetivo.id,
    titulo: objetivo.title,
    principal,
    // O principal PENDENTE ganha o marcador cheio: ele é a âncora do dia, e a
    // diferença tem de aparecer antes de a pessoa ler o rótulo. Concluído
    // continua sendo ✓ nos dois pesos — o desfecho vale mais que o peso.
    marcador: principal && objetivo.status === "pending" ? "●" : MARCADOR[objetivo.status],
    estado: ESTADO[objetivo.status],
    link: objetivo.link,
    concluivel: objetivo.status === "pending",
  };
}

/**
 * O objetivo veio de um carry-over, e quantas vezes.
 *
 * Só fala a partir do segundo: "veio de ontem" é ruído — quase todo carry-over
 * veio de ontem. "Adiado 4 vezes" é a informação que faz a pessoa decidir
 * largar, e é ela que o §29 pede que o modelo consiga responder.
 */
export function avisoDeCarregado(vezes: number): string {
  return vezes >= 2 ? `adiado ${vezes} vezes` : "";
}

/**
 * Os carry-overs em ordem de decisão: o mais carregado primeiro.
 *
 * Quem já foi adiado quatro vezes é quem mais precisa de uma decisão — e não
 * quem foi escrito por último.
 */
export function carryOverEmOrdem(itens: CarryOver[]): CarryOver[] {
  return [...itens].sort((esquerda, direita) => direita.timesCarried - esquerda.timesCarried);
}

// -------------------------------------------------------------- os fluxos

/** Um rascunho pronto para o backend, a partir de uma escolha da tela. */
export function rascunho(titulo: string, link: ObjectiveLink | null, carriedFrom?: string): ObjectiveDraft {
  return {
    title: titulo.trim(),
    // Os dois vazios juntos é intenção livre; metade preenchida é recusada pelo
    // domínio. A tela nunca monta metade porque o link vem inteiro ou nulo.
    linkKind: link?.kind ?? "",
    linkId: link?.id ?? "",
    ...(carriedFrom ? { carriedFrom } : {}),
  };
}

/** O rascunho está pronto para virar objetivo? */
export function rascunhoValido(draft: ObjectiveDraft | null): boolean {
  if (!draft) return false;
  // Título vazio COM vínculo é válido: o backend preenche com o título da
  // entidade. Sem vínculo e sem título não há objetivo nenhum.
  return draft.title.trim().length > 0 || Boolean(draft.linkKind && draft.linkId);
}

/**
 * Dá para confirmar o Start My Day?
 *
 * Um objetivo basta, e ele pode ser secundário: um dia sem principal é uma
 * escolha legítima — o pedido diz "idealmente" e não "obrigatoriamente" —, e
 * travar o botão por isso transformaria um conselho em obstáculo.
 */
export function podeIniciar(principal: ObjectiveDraft | null, secundarios: ObjectiveDraft[]): boolean {
  return rascunhoValido(principal) || secundarios.some(rascunhoValido);
}

/** O destino de cada pendente, no formato que o backend espera. */
export function destinos(escolhas: Map<string, ObjectiveStatus>): { objectiveId: string; status: ObjectiveStatus }[] {
  return [...escolhas.entries()].map(([objectiveId, status]) => ({ objectiveId, status }));
}

export const HUMORES: { valor: DayMood; rotulo: string }[] = [
  { valor: "productive", rotulo: "Dia produtivo" },
  { valor: "normal", rotulo: "Dia normal" },
  { valor: "blocked", rotulo: "Dia travado" },
];

export const DESTINOS: { valor: ObjectiveStatus; rotulo: string; explica: string }[] = [
  { valor: "completed", rotulo: "Concluído", explica: "Entra no placar do dia." },
  { valor: "carried_over", rotulo: "Levar para amanhã", explica: "Aparece no próximo Start My Day." },
  { valor: "dropped", rotulo: "Abandonar", explica: "Sai do placar, fica no histórico." },
  { valor: "pending", rotulo: "Só registrar", explica: "Fica pendente, e volta amanhã." },
];

// ------------------------------------------------------------- o histórico

/** Uma linha do histórico: a data por extenso e o placar. */
export function linhaDeHistorico(resumo: DailySessionSummary, locale = "pt-BR"): { data: string; placar: string } {
  return {
    data: dataPorExtenso(resumo.session.day, locale),
    placar: `${resumo.done}/${resumo.total} ${resumo.total === 1 ? "objetivo" : "objetivos"}`,
  };
}

/**
 * `AAAA-MM-DD` por extenso, **sem passar por `new Date(texto)`**.
 *
 * `new Date("2026-08-21")` é lido como MEIA-NOITE UTC, e num fuso negativo isso
 * volta como o dia 20. É o mesmo erro que o resto do M/OS evita guardando UTC e
 * deixando o renderer decidir o dia — só que ao contrário: aqui a data já É
 * civil, e reinterpretá-la como instante é o que a estraga.
 */
export function dataPorExtenso(dia: string, locale = "pt-BR"): string {
  const [ano, mes, data] = dia.split("-").map(Number);
  if (!ano || !mes || !data) return dia;
  return new Intl.DateTimeFormat(locale, { day: "2-digit", month: "short" }).format(new Date(ano, mes - 1, data));
}

/** A hora de um instante, para "Dia iniciado 09:08". */
export function horaDe(instante: string | null, locale = "pt-BR"): string {
  if (!instante) return "";
  const quando = new Date(instante);
  if (Number.isNaN(quando.getTime())) return "";
  return new Intl.DateTimeFormat(locale, { hour: "2-digit", minute: "2-digit" }).format(quando);
}

export const HUMOR_ROTULO: Record<DayMood, string> = {
  productive: "produtivo",
  normal: "normal",
  blocked: "travado",
};
