import { pedeAtencao, type EstadoDoAparelho, type Panorama } from "../api";
import type { Dados, Pagina } from "../navegacao";
import { emHoras, emReais } from "./numeros";

export type CartaoDaHome = {
  chave: string;
  rotulo: string;
  /** O conteudo do cartao. Texto, e nao numero: "EM DIA" e uma resposta. */
  numero: string;
  legenda: string;
  destino: Pagina;
  urgente?: boolean;
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
        : "tudo o que você escreveu já atravessou",
    destino: "mais",
    urgente: semHub || undefined,
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
    cartoes.push({
      chave: "horas",
      rotulo: "HORAS",
      numero: emHoras(panorama.horas.semanaSegundos),
      legenda: `${emReais(panorama.horas.semanaValorCents)} nesta semana`,
      destino: "horas",
    });
  }

  if (panorama && panorama.proximos.length > 0) {
    cartoes.push({
      chave: "academico",
      rotulo: "ACADÊMICO",
      numero: String(panorama.proximos.length),
      legenda: panorama.proximos[0].titulo,
      destino: "academico",
    });
  }

  if (dados.capturas.length > 0) {
    cartoes.push({
      chave: "inbox",
      rotulo: "INBOX",
      numero: String(dados.capturas.length),
      legenda: dados.capturas.length === 1 ? "captura esperando" : "capturas esperando",
      destino: "inbox",
    });
  }

  const abertas = dados.tasks.filter((task) => task.state !== "done");
  if (abertas.length > 0) {
    const andando = abertas.filter((task) => task.state === "doing").length;
    cartoes.push({
      chave: "tasks",
      rotulo: "TASKS",
      numero: String(abertas.length),
      legenda: andando > 0 ? `${andando} em andamento` : "abertas",
      destino: "tasks",
    });
  }

  const ultima = dados.capturas[0];
  if (ultima) {
    cartoes.push({
      chave: "ultima",
      rotulo: "ÚLTIMA CAPTURA",
      numero: ultima.content,
      legenda: "",
      destino: "inbox",
    });
  }

  return cartoes;
}
