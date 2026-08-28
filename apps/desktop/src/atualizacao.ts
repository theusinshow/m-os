/**
 * O que dizer embaixo do botão de atualizar.
 *
 * # Por que isto é um módulo, e não três ternários dentro do painel
 *
 * Porque a pergunta "estou atualizado?" tem **cinco** respostas, e quatro delas
 * eram invisíveis. O painel antigo mostrava recado só nos segundos seguintes ao
 * clique, e tratava dois estados muito diferentes como um só nada:
 *
 * - *conferi e você está em dia* — o caso comum, e o que a pessoa quer ver;
 * - *não consegui conferir* — rede fora, GitHub fora, release sem `latest.json`.
 *
 * Com a mesma cara para os dois, um M/OS que nunca conseguiu falar com o
 * servidor parecia um M/OS atualizado. É daí que sai a impressão de que a
 * atualização "às vezes não funciona": ela às vezes não ACONTECIA, e nunca
 * dizia isso.
 *
 * Separado, o mapa de estado para frase se confere sem abrir o app — e cada
 * estado tem um teste com a data fixa que o produz.
 */

/** O que o Rust grava e devolve. Fatos, nenhuma frase — ver `atualizacao.rs`. */
export type EstadoDaAtualizacao = {
  /** A versão que está rodando agora. */
  versao: string;
  /** Quando esta versão chegou NESTE computador. RFC 3339, ou vazio. */
  instaladaEm: string;
  /** A última verificação que deu certo. Vazio significa nunca. */
  verificadaEm: string;
  /** A versão nova que ela encontrou. Vazio significa que não havia. */
  disponivel: string;
  /** Quando essa versão foi publicada. */
  publicadaEm: string;
  /** O motivo da última tentativa, quando a mais recente falhou. */
  falha: string;
  falhaEm: string;
  endpoint: string;
};

/**
 * As cinco respostas possíveis.
 *
 * `trabalhando` cobre verificar e instalar: nos dois a resposta honesta é "ainda
 * não sei", e inventar um estado separado para cada um dobraria o mapa sem
 * mudar uma palavra da tela.
 */
export type Situacao = "em-dia" | "atrasado" | "sem-resposta" | "nunca" | "trabalhando";

/**
 * Qual das cinco, e **a ordem das perguntas é a decisão**.
 *
 * `disponivel` vem antes de `falha` de propósito: se sabemos que existe uma
 * versão nova, isso continua sendo verdade mesmo que a tentativa de hoje tenha
 * caído — e é a informação sobre a qual dá para agir. Dizer "não consegui
 * verificar" escondendo uma atualização já conhecida seria trocar um fato por
 * uma queixa.
 */
export function situacao(estado: EstadoDaAtualizacao | null, ocupado: boolean): Situacao {
  if (ocupado) return "trabalhando";
  if (!estado) return "nunca";
  // Igual à instalada quer dizer que ela já foi instalada desde a verificação —
  // a anotação é que ficou velha, e não o M/OS.
  if (estado.disponivel && estado.disponivel !== estado.versao) return "atrasado";
  if (estado.falha) return "sem-resposta";
  if (estado.verificadaEm) return "em-dia";
  return "nunca";
}

/** O selo. Curto porque ele é lido de relance, e não lido. */
export function rotulo(situacao: Situacao): string {
  switch (situacao) {
    case "em-dia":
      return "EM DIA";
    case "atrasado":
      return "DESATUALIZADO";
    case "sem-resposta":
      return "NÃO CONFERIDO";
    case "nunca":
      return "NUNCA CONFERI";
    case "trabalhando":
      return "CONFERINDO";
  }
}

/** "26/08, 02:31". Sem ano: a versão que roda é sempre recente. */
function dia(iso: string): string {
  const quando = new Date(iso);
  if (Number.isNaN(quando.getTime())) return "";
  return quando.toLocaleString("pt-BR", {
    day: "2-digit",
    month: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/** Só o dia, para a data de publicação — a hora de um release não decide nada. */
function data(iso: string): string {
  const quando = new Date(iso);
  if (Number.isNaN(quando.getTime())) return "";
  return quando.toLocaleDateString("pt-BR", { day: "2-digit", month: "2-digit" });
}

/**
 * A primeira linha: o que está rodando, e desde quando.
 *
 * A data vem do carimbo do executável, então ela responde "desde quando este
 * computador está nesta versão" — que é a pergunta —, e não "quando isto foi
 * compilado".
 */
export function linhaDaVersao(estado: EstadoDaAtualizacao | null): string {
  if (!estado?.versao) return "Versão —";
  const quando = dia(estado.instaladaEm);
  return quando ? `Versão ${estado.versao} · instalada em ${quando}` : `Versão ${estado.versao}`;
}

/**
 * A segunda linha: o que a última verificação descobriu.
 *
 * `relativa` é injetada em vez de importada para o teste poder fixar o tempo. O
 * painel passa a `relativeTime` de sempre.
 */
export function linhaDaVerificacao(
  estado: EstadoDaAtualizacao | null,
  relativa: (iso: string) => string,
): string {
  const qual = situacao(estado, false);
  if (!estado) return "";

  switch (qual) {
    case "atrasado": {
      const publicada = data(estado.publicadaEm);
      return publicada
        ? `Há uma versão nova: ${estado.disponivel}, publicada em ${publicada}.`
        : `Há uma versão nova: ${estado.disponivel}.`;
    }
    case "em-dia":
      return `Conferido ${relativa(estado.verificadaEm)}. Nenhuma versão nova.`;
    case "sem-resposta": {
      // As duas metades, e não uma: o que se soube antes continua valendo, e
      // apagá-lo transformaria uma queda de rede em "você nunca verificou".
      const queixa = `Não consegui conferir ${relativa(estado.falhaEm)}: ${estado.falha}`;
      return estado.verificadaEm
        ? `${queixa} A última vez que deu certo foi ${relativa(estado.verificadaEm)}.`
        : queixa;
    }
    case "nunca":
      return "Ainda não conferi se existe versão nova.";
    case "trabalhando":
      return "";
  }
}

/**
 * Já passou tempo bastante para conferir sozinho?
 *
 * O M/OS confere na abertura para o selo significar alguma coisa: um indicador
 * que só se atualiza quando alguém entra em Settings e clica é um indicador que
 * mostra o passado. Mas conferir a cada abertura seria uma ida à rede toda vez
 * que a janela nasce — e o M/OS abre no logon.
 *
 * Seis horas: mais de uma vez por dia de trabalho, e longe de uma por abertura.
 * Uma falha NÃO reinicia a contagem por si só; ela conta como tentativa, senão
 * um GitHub fora do ar viraria uma tentativa por minuto.
 */
export function deveConferirSozinho(
  estado: EstadoDaAtualizacao | null,
  agora: Date = new Date(),
  horas = 6,
): boolean {
  if (!estado) return false;
  // Já sabemos que há uma nova: não há o que descobrir, e a tela já diz.
  if (estado.disponivel && estado.disponivel !== estado.versao) return false;

  const ultima = [estado.verificadaEm, estado.falhaEm]
    .map((iso) => new Date(iso).getTime())
    .filter((instante) => !Number.isNaN(instante));
  if (!ultima.length) return true;

  return agora.getTime() - Math.max(...ultima) > horas * 3600_000;
}
