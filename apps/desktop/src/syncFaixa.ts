/**
 * A faixa de sincronizacao da Home: quando aparece, e o que diz.
 *
 * Vive fora do `App.tsx` para poder ser testada. Nao ha teste de DOM neste
 * repositorio, por decisao registrada no `vitest.config.ts`, e a consequencia
 * pratica e a de sempre: o que DECIDE alguma coisa tem de ser funcao pura, e o
 * componente so desenha o resultado.
 *
 * # Por que esta faixa e uma excecao, e por que ela pode ser
 *
 * O `App.tsx` registra o principio da Home ao apresentar o widget do dia:
 * "tudo que mora na Home do M/OS e um widget arrumavel, e uma excecao seria a
 * unica coisa da tela que nao se pode mover nem esconder."
 *
 * A faixa contradiz isso, e a contradicao precisa ficar escrita ao lado da
 * razao que ela contradiz — senao vira precedente, e o proximo card fixo aponta
 * para este.
 *
 * A defesa: aquele principio protege a Home de ter um MORADOR permanente que
 * nao se arruma. A faixa nao mora aqui. Ela so existe quando ha noticia ou
 * quando algo esta errado, e some quando e lida ou quando a causa some. Na
 * maioria dos dias nao ocupa espaco nenhum, entao nao compete com os widgets
 * pelo espaco que o principio protege.
 *
 * O widget arrumavel foi considerado e RECUSADO: um widget se esconde, e um
 * widget de sync escondido e um sync quebrado que ninguem descobre.
 */
import type { SyncStatus } from "./types";

export type TipoDaFaixa = "chegou" | "pendente" | "erro";

export type FaixaDeSync = {
  tipo: TipoDaFaixa;
  titulo: string;
  corpo: string;
  /** Uma rodada corre agora, POR CIMA desta faixa. */
  girando: boolean;
  /**
   * Se o botao de dispensar aparece.
   *
   * Falso para erro e pendente, e essa e a regra que amarra os seis estados: um
   * aviso que se pode calar sem consertar a causa e um aviso que se cala
   * sempre. Eles somem quando a causa some, e nao quando incomodam.
   */
  dispensavel: boolean;
};

/* Como cada tipo se chama na tela, no singular e no plural.

   So os tipos que o M/OS mostra por nome. O que nao esta aqui aparece pelo
   proprio id — feio, e MUITO melhor que sumir: `EntityKind` e texto e nao enum
   fechado (SYNC.md §9), justamente para um cliente antigo guardar e reenviar um
   tipo que ele nao conhece. Sumir com ele faria a faixa dizer que nada chegou
   quando algo chegou. */
const NOMES: Record<string, [string, string]> = {
  task: ["task", "tasks"],
  capture: ["capture", "captures"],
  project: ["project", "projects"],
  resource: ["resource", "resources"],
  reminder: ["lembrete", "lembretes"],
  workspace: ["contexto", "contextos"],
  daily_session: ["dia", "dias"],
  daily_objective: ["objetivo do dia", "objetivos do dia"],
  daily_reflection: ["reflexão", "reflexões"],
  weekly_review: ["fecho de semana", "fechos de semana"],
  academic_semester: ["semestre", "semestres"],
  academic_subject: ["disciplina", "disciplinas"],
  academic_assignment: ["entrega", "entregas"],
  academic_exam: ["prova", "provas"],
  academic_study_session: ["sessão de estudo", "sessões de estudo"],
};

/**
 * "3 tasks · 1 capture".
 *
 * Ordena pelo NUMERO e nao pelo nome: a noticia grande vem primeiro, e a ordem
 * alfabetica poria "academic_exam" na frente de vinte tasks.
 */
export function frasePorTipo(porTipo: Record<string, number>): string {
  return Object.entries(porTipo)
    .filter(([, quantas]) => quantas > 0)
    .sort(([aNome, a], [bNome, b]) => b - a || aNome.localeCompare(bNome))
    .map(([tipo, quantas]) => {
      const nomes = NOMES[tipo];
      if (!nomes) return `${quantas} ${tipo}`;
      return `${quantas} ${quantas === 1 ? nomes[0] : nomes[1]}`;
    })
    .join(" · ");
}

/**
 * Qual faixa desenhar, ou nenhuma.
 *
 * A ORDEM das perguntas e o desenho, e nao acaso:
 *
 * 1. desligado sai primeiro, e sai CALADO. Quem nao ligou o sync nao tem um
 *    problema, tem uma feature desligada — transformar isso em aviso diario na
 *    Home seria propaganda dentro do proprio produto;
 * 2. a NOTICIA ganha do erro, porque uma rodada que trouxe coisa funcionou, e o
 *    erro que sobrou e de antes dela;
 * 3. o erro ganha da fila, porque a fila e consequencia e o erro e a causa.
 *    Mostrar "47 esperando" sem dizer por que manda consertar as cegas.
 *
 * E o que NAO abre faixa: uma rodada silenciosa. Ela roda a cada quinze minutos,
 * e piscar uma faixa na Home a cada quarto de hora seria pior que o ruido que
 * este desenho existe para evitar. Rodada so troca a CARA de uma faixa que ja
 * estava la; o estado calmo vive no cabecalho.
 */
export function estadoDaFaixa(status: SyncStatus | null): FaixaDeSync | null {
  if (!status) return null;
  // Desligado: sem endereco ou sem segredo, nao ha o que sincronizar.
  if (!status.endpoint || !status.hasToken) return null;

  const girando = status.running;

  const resumo = status.daySummary;
  const chegou = resumo ? Object.values(resumo.byKind).reduce((a, b) => a + b, 0) : 0;
  if (resumo && chegou > 0) {
    return {
      tipo: "chegou",
      titulo: "CHEGOU ENQUANTO VOCÊ ESTAVA FORA",
      corpo: frasePorTipo(resumo.byKind),
      girando,
      dispensavel: true,
    };
  }

  if (status.lastError) {
    return {
      tipo: "erro",
      titulo: "A SINCRONIZAÇÃO PAROU",
      // O motivo CRU, e nao "algo deu errado". A causa quase sempre esta fora
      // do M/OS — tunel caido, hub fora — e so o texto de verdade diz onde ir.
      corpo: status.lastError,
      girando,
      dispensavel: false,
    };
  }

  if (status.pending > 0) {
    return {
      tipo: "pendente",
      titulo: `${status.pending} ${status.pending === 1 ? "MUDANÇA ESPERANDO" : "MUDANÇAS ESPERANDO"}`,
      corpo: "Ainda não subiram. Vou tentar sozinho; o botão adianta.",
      girando,
      dispensavel: false,
    };
  }

  // Em dia. A Home nao muda — o horario da ultima rodada vive no cabecalho.
  return null;
}
