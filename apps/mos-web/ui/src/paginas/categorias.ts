import type { ItemDaAgenda } from "../api";

/**
 * As categorias do calendário, e o que cai em cada uma.
 *
 * # Por que agrupar os `kind` em vez de filtrar por eles
 *
 * O domínio tem catorze tipos de item, e a pergunta que alguém faz diante de um
 * calendário nunca é "quero ver `academic_planned` mas não `assignment_due`" —
 * é *quero ver só a faculdade*. Uma lista de catorze interruptores obrigaria a
 * conhecer o vocabulário interno do M/OS para usar a agenda.
 *
 * Oito grupos, e cada `kind` novo do domínio precisa ser posto num deles — o
 * teste garante que nenhum fique órfão, porque item sem categoria seria item que
 * some quando qualquer filtro é ligado.
 */
export type Categoria =
  | "cronocad"
  | "faculdade"
  | "tasks"
  | "lembretes"
  | "capturas"
  | "dia"
  | "feriados"
  | "outros";

export const CATEGORIAS: { chave: Categoria; rotulo: string }[] = [
  { chave: "cronocad", rotulo: "CronoCAD" },
  { chave: "faculdade", rotulo: "Faculdade" },
  { chave: "tasks", rotulo: "Tasks" },
  { chave: "lembretes", rotulo: "Lembretes" },
  { chave: "capturas", rotulo: "Capturas" },
  { chave: "dia", rotulo: "Dia" },
  { chave: "feriados", rotulo: "Feriados" },
  { chave: "outros", rotulo: "Outros" },
];

const DE_CADA_TIPO: Record<string, Categoria> = {
  session: "cronocad",
  assignment_due: "faculdade",
  exam_scheduled: "faculdade",
  academic_planned: "faculdade",
  task_created: "tasks",
  task_done: "tasks",
  reminder: "lembretes",
  capture: "capturas",
  day_started: "dia",
  day_ended: "dia",
  objective_done: "dia",
  holiday: "feriados",
  meeting: "outros",
  app_opened: "outros",
};

/**
 * A categoria de um item.
 *
 * Tipo desconhecido — de um servidor mais novo que esta tela — cai em `outros`,
 * e não some: sumir seria a tela escondendo o que ela não entende, que é o
 * jeito mais silencioso de um calendário mentir.
 */
export function categoriaDe(item: ItemDaAgenda): Categoria {
  return DE_CADA_TIPO[item.kind] ?? "outros";
}

/** O que está ligado. Ausente da lista significa DESLIGADO. */
export type Filtro = Categoria[];

/** Tudo ligado — o estado em que a agenda abre. */
export const TUDO: Filtro = CATEGORIAS.map((categoria) => categoria.chave);

export function alternar(filtro: Filtro, categoria: Categoria): Filtro {
  return filtro.includes(categoria)
    ? filtro.filter((atual) => atual !== categoria)
    : [...filtro, categoria];
}

/**
 * Aplica o filtro.
 *
 * Filtro VAZIO devolve tudo, e não nada.
 *
 * É a decisão menos óbvia deste arquivo: literalmente, nenhuma categoria ligada
 * deveria dar zero itens. Mas ninguém desliga as oito querendo uma tela em
 * branco — desliga-se a última por engano, ou tateando. Uma agenda vazia sem
 * explicação parece defeito; devolver tudo é o mesmo que dizer "sem filtro".
 */
export function aplicar(itens: ItemDaAgenda[], filtro: Filtro): ItemDaAgenda[] {
  if (filtro.length === 0) return itens;
  return itens.filter((item) => filtro.includes(categoriaDe(item)));
}

const CHAVE = "mos.agenda.filtro";

/** Lê o filtro guardado. Falha em silêncio, como o arranjo da Home. */
export function lerFiltro(): Filtro {
  try {
    const cru = window.localStorage.getItem(CHAVE);
    if (!cru) return TUDO;
    const lido = JSON.parse(cru) as unknown;
    if (!Array.isArray(lido)) return TUDO;
    const validas = lido.filter((c): c is Categoria =>
      CATEGORIAS.some((categoria) => categoria.chave === c),
    );
    return validas.length > 0 ? validas : TUDO;
  } catch {
    return TUDO;
  }
}

export function gravarFiltro(filtro: Filtro): void {
  try {
    window.localStorage.setItem(CHAVE, JSON.stringify(filtro));
  } catch {
    // Sem onde guardar, o filtro vale para esta sessão.
  }
}
