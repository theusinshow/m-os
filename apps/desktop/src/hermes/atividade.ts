/**
 * A atividade operacional de um turno, resumida para leitura humana.
 *
 * # O que isto NÃO é
 *
 * Não é chain-of-thought. O raciocínio do modelo continua atrás de um
 * `<details>` fechado, porque é dele e não do usuário. O que passa por aqui é
 * só o que o sistema FEZ: quais ferramentas rodaram, quantas, se alguma falhou.
 *
 * # Por que colapsa
 *
 * Durante a resposta, os passos são informação — dizem que algo está
 * acontecendo. Depois que ela assenta, viram histórico, e histórico aberto por
 * padrão é ruído que empurra a prosa para baixo em toda releitura. O recibo de
 * uma linha guarda o mesmo fato e devolve o espaço.
 */
import type { ToolRunState } from "../hermes";

export type Passo = { name: string; state: ToolRunState };

/** Marcador textual do estado. Nenhum estado depende só de cor. */
export const MARCA_DO_PASSO: Record<ToolRunState, string> = {
  queued: "·",
  running: "→",
  success: "✓",
  error: "!",
  cancelled: "×",
  waiting_permission: "?",
};

export const NOME_DO_ESTADO: Record<ToolRunState, string> = {
  queued: "na fila",
  running: "executando",
  success: "concluído",
  error: "falhou",
  cancelled: "cancelado",
  waiting_permission: "aguardando permissão",
};

/**
 * A linha única que resume os passos depois que o turno assenta.
 *
 * Fala em FONTES, e não em ferramentas: "3 fontes consultadas" diz o que o
 * usuário ganhou; "3 tool calls" diz como o backend trabalhou. A falha nunca é
 * escondida atrás do número — ela sobe para o resumo, porque é o único caso em
 * que a linha colapsada precisa provocar um clique.
 */
export function reciboDosPassos(passos: Passo[], decorrido = ""): string {
  if (!passos.length) return "";
  const falhas = passos.filter((passo) => passo.state === "error").length;
  const total = passos.length;
  const contagem = `${total} ${total === 1 ? "fonte consultada" : "fontes consultadas"}`;
  const tempo = decorrido ? ` · ${decorrido}` : "";
  // Com falha a linha encurta: ela mora numa margem de 132px, e "N fontes
  // consultadas · 1 falhou" quebrava em tres linhas de mono maiuscula ao lado
  // de um paragrafo. O que precisa sobreviver ao corte e o NUMERO e a palavra
  // "falhou" — e o resto o clique conta.
  if (falhas) return `${total} ${total === 1 ? "fonte" : "fontes"} · ${falhas} ${falhas === 1 ? "falhou" : "falharam"}`;
  return `${contagem}${tempo}`;
}

/** Se o recibo colapsado precisa chamar atenção. Só falha faz isso. */
export function reciboAlerta(passos: Passo[]): boolean {
  return passos.some((passo) => passo.state === "error");
}

/**
 * Quanto tempo o turno levou, em segundos com uma casa.
 *
 * Vazio abaixo de meio segundo: "0.2s" não informa nada que "pronto" já não
 * diga, e um número minúsculo ao lado do texto vira decoração.
 */
export function decorridoDe(inicio: number, agora: number): string {
  if (!inicio) return "";
  const segundos = (agora - inicio) / 1000;
  if (segundos < 0.5) return "";
  return `${segundos.toFixed(1)}s`;
}

/**
 * A mensagem já traz uma linha explicando como o turno acabou?
 *
 * Desde 2026-08-22 o `settle_turn` grava o motivo do fim como parte de status —
 * "A conexão caiu.", "Interrompido por você." — e ela é desenhada como qualquer
 * outra linha de sistema. Esta função existe para o componente não empilhar um
 * segundo "Interrompido." embaixo dela.
 *
 * O teste é POSICIONAL, e não por texto: comparar com a lista de frases exigiria
 * uma segunda cópia delas aqui, e duas cópias de uma frase divergem na primeira
 * vez que alguém corrige uma vírgula. Se a última coisa dita no turno já é uma
 * linha de sistema, ela ocupa esse lugar — venha do fim do turno ou de um
 * `status.update` que por acaso foi o último.
 *
 * O erro possível é benigno nos dois sentidos: no máximo uma mensagem antiga
 * deixa de mostrar "Interrompido.", ou mostra quando não precisava.
 */
export function explicaOFim(message: { parts: { body: { kind: string } }[] }): boolean {
  const ultima = message.parts[message.parts.length - 1];
  return ultima?.body.kind === "status";
}
