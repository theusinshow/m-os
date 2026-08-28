/**
 * Quando o lembrete vence — a parte que é só conta.
 *
 * # Por que isto é um módulo separado da folha que o mostra (`Quando.tsx`)
 *
 * Porque tudo aqui é aritmética de fuso, e aritmética de fuso é onde erro de
 * lembrete nasce: "amanhã 9h" resolvido errado não falha, ele **toca na hora
 * errada** — e um lembrete que toca na hora errada é pior que um que não toca,
 * porque ensina a não confiar no relógio.
 *
 * Separado, ele se confere com uma data fixa e sem navegador.
 *
 * # O instante é resolvido AQUI, e não no servidor
 *
 * "Amanhã de manhã" é um conceito local. O `mos-web` roda numa VPS cujo fuso não
 * é o de quem tocou no botão, e meia-noite em UTC é nove da noite no Brasil —
 * mandar o cálculo para lá faria "amanhã" começar hoje à noite. A regra é
 * normativa (`CORE-FOUNDATION.md` §5) e é a mesma que o `ReminderComposer` do
 * desktop segue.
 */

export type Escolha = {
  rotulo: string;
  /** Resolve contra `base`, que os testes fixam e a tela não passa. */
  resolver: (base: Date) => Date;
};

/**
 * As opções rápidas.
 *
 * Relógio apenas. Sugestão por agenda exigiria agenda, e as decisões D-1 e D-4
 * deixaram o M/OS sem prazo em Task e sem entidade Event — não há âncora de
 * tempo futuro a que se referir (`ATTENTION-SYSTEM.md` §35.1). Um chip
 * "no fim da reunião" desabilitado ensinaria que a capacidade existe e está
 * quebrada; a ausência é honesta.
 */
export const ESCOLHAS: ReadonlyArray<Escolha> = [
  { rotulo: "15 min", resolver: (base) => new Date(base.getTime() + 15 * 60_000) },
  { rotulo: "1 hora", resolver: (base) => new Date(base.getTime() + 60 * 60_000) },
  { rotulo: "3 horas", resolver: (base) => new Date(base.getTime() + 3 * 60 * 60_000) },
  {
    rotulo: "Hoje 18h",
    resolver: (base) => {
      const quando = new Date(base);
      quando.setHours(18, 0, 0, 0);
      return quando;
    },
  },
  {
    rotulo: "Amanhã 9h",
    resolver: (base) => {
      const quando = new Date(base);
      quando.setDate(quando.getDate() + 1);
      quando.setHours(9, 0, 0, 0);
      return quando;
    },
  },
  {
    rotulo: "Segunda 9h",
    resolver: (base) => {
      const quando = new Date(base);
      // 8 menos o dia da semana, com resto 7 quando hoje já é segunda: pedir
      // "segunda" numa segunda quer dizer a próxima, e não daqui a instante
      // nenhum.
      const adiante = ((8 - quando.getDay()) % 7) || 7;
      quando.setDate(quando.getDate() + adiante);
      quando.setHours(9, 0, 0, 0);
      return quando;
    },
  },
];

/**
 * As opções que ainda fazem sentido.
 *
 * "Hoje 18h" às 19h não quer dizer nada, e o servidor a recusaria de todo jeito
 * — oferecê-la seria oferecer um erro.
 */
export function disponiveis(base: Date = new Date()): Escolha[] {
  return ESCOLHAS.filter((escolha) => escolha.resolver(base).getTime() > base.getTime());
}

/** O primeiro chip que ainda serve, ou quinze minutos se nenhum servir. */
export function padrao(base: Date = new Date()): Date {
  return disponiveis(base)[0]?.resolver(base) ?? new Date(base.getTime() + 15 * 60_000);
}

/**
 * O instante por extenso, no fuso de quem lê.
 *
 * Sempre visível na folha de escolha: um lembrete que dispara em hora diferente
 * da que a pessoa achou que escolheu é pior que um lembrete que não dispara.
 */
export function porExtenso(quando: Date): string {
  return quando.toLocaleString("pt-BR", {
    weekday: "short",
    day: "2-digit",
    month: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/**
 * Quanto falta, ou quanto passou.
 *
 * Numa lista de lembretes é a única leitura que decide alguma coisa: "venceu há
 * 50 min" pede ação agora, "em 3 h" não pede nada. O relógio exato responde
 * depois, no detalhe.
 */
export function daquiA(iso: string | null, agora: Date = new Date()): string {
  if (!iso) return "sem hora";
  const alvo = new Date(iso).getTime();
  if (Number.isNaN(alvo)) return "";

  const minutos = Math.round((alvo - agora.getTime()) / 60_000);
  const atrasado = minutos < 0;
  const falta = Math.abs(minutos);

  const medida =
    falta < 1
      ? "agora"
      : falta < 60
        ? `${falta} min`
        : falta < 60 * 24
          ? `${Math.round(falta / 60)} h`
          : `${Math.round(falta / (60 * 24))} d`;

  if (medida === "agora") return "agora";
  return atrasado ? `venceu há ${medida}` : `em ${medida}`;
}

/**
 * O valor que um `datetime-local` aceita.
 *
 * Ele fala no fuso do usuário e recusa sufixo de zona, então o `toISOString`
 * cru — que é UTC — chegaria com as horas trocadas.
 */
export function paraCampoLocal(quando: Date): string {
  const deslocado = new Date(quando.getTime() - quando.getTimezoneOffset() * 60_000);
  return deslocado.toISOString().slice(0, 16);
}
