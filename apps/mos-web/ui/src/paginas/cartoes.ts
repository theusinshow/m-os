import { pedeAtencao, type EstadoDoAparelho, type Panorama } from "../api";
import type { Dados, Pagina } from "../navegacao";
import { idade } from "./idade";
import { emHoras, emReais } from "./numeros";

/**
 * O desenho que o cartão carrega além do número.
 *
 * É o que responde à queixa de que a Home era pobre: o número diz *quanto*, e o
 * enfeite diz *como foi* — e a segunda pergunta não cabe em mais um número.
 */
export type Enfeite =
  /** Sete barras, de segunda a domingo. `hoje` é o índice em sódio; o que vem
   *  depois dele é futuro e sai em traço apagado. */
  | { tipo: "semana"; dias: number[]; hoje: number }
  /** Uma barra de 0 a 1. */
  | { tipo: "progresso"; fracao: number };

export type CartaoDaHome = {
  chave: string;
  rotulo: string;
  /** O conteudo do cartao. Texto, e nao numero: "EM DIA" e uma resposta. */
  numero: string;
  legenda: string;
  destino: Pagina;
  urgente?: boolean;
  /** Canto superior direito, em mono. Idade, distância — o que situa o número. */
  aposto?: string;
  enfeite?: Enfeite;
  /** O número é uma resposta em palavra ("EM DIA"), e não um algarismo: não sobe
   *  de zero, e não fica em corpo 34. */
  palavra?: boolean;
};

/**
 * O que a Home mostra, e em que ordem.
 *
 * Regra que vale para todos: **cartao sem o que dizer nao aparece**. Um cartao
 * vazio prometendo conteudo ensina que a Home tem lugares que nunca dizem nada,
 * e depois disso ela deixa de ser lida.
 *
 * O sync e a excecao, e de proposito: "em dia" e informacao — e a resposta a
 * pergunta que se faz ao abrir o app na rua.
 */
export function cartoesDaHome(
  estado: EstadoDoAparelho | null,
  dados: Dados,
  agora: Date = new Date(),
  panorama: Panorama | null = null,
): CartaoDaHome[] {
  const cartoes: CartaoDaHome[] = [];

  const semHub = estado?.sincroniza === false;
  const pendentes = estado?.pendentes ?? 0;
  cartoes.push({
    chave: "sync",
    rotulo: "SYNC",
    numero: semHub ? "SEM HUB" : pendentes > 0 ? String(pendentes) : "EM DIA",
    legenda: semHub
      ? "este aparelho não alcança o hub"
      : pendentes > 0
        ? "esperando para subir"
        : "tudo já atravessou",
    destino: "mais",
    urgente: semHub || undefined,
    palavra: pendentes === 0 || semHub || undefined,
  });

  const cobrando = dados.lembretes.filter(pedeAtencao);
  const proximos = dados.lembretes.filter(
    (l) => l.status === "scheduled" && l.nextDueAt !== null && new Date(l.nextDueAt) > agora,
  );
  if (cobrando.length > 0 || proximos.length > 0) {
    cartoes.push({
      chave: "hoje",
      rotulo: "HOJE",
      numero: String(cobrando.length + proximos.length),
      legenda:
        cobrando.length > 0
          ? `${cobrando.length} cobrando agora`
          : "agendados, nenhum vencido",
      destino: "lembretes",
      urgente: cobrando.length > 0 || undefined,
    });
  }

  // As horas vêm do panorama, e não do banco local: o cálculo é o mesmo do
  // desktop — arredondamento por sessão —, e refazê-lo aqui daria um segundo
  // número que diverge do primeiro no dia em que a regra mudar.
  if (panorama && panorama.horas.semanaSegundos > 0) {
    const dias = panorama.horas.diasSegundos;
    cartoes.push({
      chave: "horas",
      rotulo: "HORAS",
      numero: emHoras(panorama.horas.semanaSegundos),
      legenda: `${emReais(panorama.horas.semanaValorCents)} nesta semana`,
      destino: "horas",
      // Servidor antigo não manda os dias: o cartão continua inteiro sem a
      // semana, com o número e o valor, que é o que ele sempre teve.
      enfeite:
        dias && dias.length === 7
          ? { tipo: "semana", dias, hoje: (agora.getDay() + 6) % 7 }
          : undefined,
    });
  }

  if (panorama && panorama.proximos.length > 0) {
    const primeiro = panorama.proximos[0];
    cartoes.push({
      chave: "academico",
      rotulo: "ACADÊMICO",
      numero: String(panorama.proximos.length),
      legenda: primeiro.titulo,
      // A distância, e não a data: "2d" decide se isso é problema de hoje; "11
      // de setembro" obriga a fazer a conta de cabeça.
      aposto: emDias(primeiro.quando, agora),
      destino: "academico",
    });
  }

  if (dados.capturas.length > 0) {
    cartoes.push({
      chave: "inbox",
      rotulo: "INBOX",
      numero: String(dados.capturas.length),
      legenda: dados.capturas.length === 1 ? "captura esperando" : "capturas esperando",
      destino: "fazer",
    });
  }

  const abertas = dados.tasks.filter((task) => task.state !== "done");
  if (abertas.length > 0) {
    const andando = abertas.filter((task) => task.state === "doing").length;
    const feitas = dados.tasks.length - abertas.length;
    cartoes.push({
      chave: "tasks",
      rotulo: "TASKS",
      numero: String(abertas.length),
      legenda: andando > 0 ? `${andando} em andamento` : "abertas",
      destino: "fazer",
      enfeite:
        dados.tasks.length > 0
          ? { tipo: "progresso", fracao: feitas / dados.tasks.length }
          : undefined,
    });
  }

  const ultima = dados.capturas[0];
  if (ultima) {
    cartoes.push({
      chave: "ultima",
      rotulo: "ÚLTIMA CAPTURA",
      numero: ultima.content,
      legenda: "",
      // Sem a idade, a última captura parece sempre recente — e a pergunta que
      // se faz diante dela é justamente *isto ainda importa?*.
      aposto: idade(ultima.capturedAt, agora),
      destino: "fazer",
      palavra: true,
    });
  }

  return cartoes;
}

/** `2d`, `hoje`, `atrasado`. Vazio quando a data não é legível. */
function emDias(iso: string, agora: Date): string {
  const quando = new Date(iso).getTime();
  if (Number.isNaN(quando)) return "";
  const dias = Math.ceil((quando - agora.getTime()) / 86_400_000);
  if (dias < 0) return "atrasado";
  if (dias === 0) return "hoje";
  if (dias === 1) return "amanhã";
  return `${dias}d`;
}
