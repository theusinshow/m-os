/**
 * As janelas de tempo que a tela de Horas oferece.
 *
 * # Por que aqui, e não dentro do componente
 *
 * Porque são contas de calendário, e conta de calendário erra em silêncio: o
 * mês que começa no dia 1 mas termina no 28 em fevereiro, a semana passada que
 * inclui o domingo errado, o "tudo" que corta 2019 fora. Fora do componente
 * elas têm teste; dentro dele teriam um print no console.
 *
 * O servidor já aceita qualquer janela — `report(desde, até)` sempre aceitou. A
 * limitação era só de tela.
 */
export type Janela =
  | "semana"
  | "passada"
  | "mes"
  | "mes-passado"
  | "tudo"
  | "personalizado";

export const JANELAS: { chave: Janela; rotulo: string }[] = [
  { chave: "semana", rotulo: "Semana" },
  { chave: "passada", rotulo: "Passada" },
  { chave: "mes", rotulo: "Mês" },
  { chave: "mes-passado", rotulo: "Mês passado" },
  { chave: "tudo", rotulo: "Tudo" },
];

/** A segunda-feira da semana de `agora`, à meia-noite local. */
function inicioDaSemana(agora: Date): Date {
  const inicio = new Date(agora);
  // A semana começa na segunda, como em todo o M/OS — `getDay()` conta a partir
  // do domingo, e o `+6 % 7` é o que corrige isso.
  inicio.setDate(inicio.getDate() - ((inicio.getDay() + 6) % 7));
  inicio.setHours(0, 0, 0, 0);
  return inicio;
}

/**
 * De quando até quando cada janela vai.
 *
 * O fim é sempre `agora` para as janelas que incluem hoje, e não a meia-noite
 * do fim do período: pedir horas até o fim de uma semana que ainda não acabou
 * seria pedir dado do futuro, e o servidor devolveria o mesmo — mas a janela
 * ficaria mentindo sobre o que ela abrange.
 */
export function periodo(janela: Janela, agora: Date = new Date()): [Date, Date] {
  switch (janela) {
    case "semana": {
      return [inicioDaSemana(agora), agora];
    }
    case "passada": {
      const inicio = inicioDaSemana(agora);
      const fim = new Date(inicio);
      // Um milissegundo antes da segunda desta semana: sem isso, a semana
      // passada e a atual dividiriam a meia-noite de segunda e uma hora
      // lançada exatamente ali contaria duas vezes.
      fim.setMilliseconds(fim.getMilliseconds() - 1);
      inicio.setDate(inicio.getDate() - 7);
      return [inicio, fim];
    }
    case "mes": {
      return [new Date(agora.getFullYear(), agora.getMonth(), 1), agora];
    }
    case "mes-passado": {
      const inicio = new Date(agora.getFullYear(), agora.getMonth() - 1, 1);
      // Dia zero deste mês é o último do anterior, sem precisar saber quantos
      // dias ele tem — e sem errar em fevereiro de ano bissexto.
      const fim = new Date(agora.getFullYear(), agora.getMonth(), 0, 23, 59, 59, 999);
      return [inicio, fim];
    }
    case "tudo": {
      // 2020 e não "o começo dos tempos": o M/OS não existia antes disso, e uma
      // data absurda como 1970 faria o servidor varrer um índice inteiro para
      // achar o mesmo nada.
      return [new Date(2020, 0, 1), agora];
    }
    case "personalizado": {
      // Quem escolheu período próprio manda as datas; esta é só a de partida.
      return [inicioDaSemana(agora), agora];
    }
  }
}

/** `5 de set` — o rótulo curto de uma ponta do período. */
export function pontaCurta(quando: Date): string {
  const meses = [
    "jan", "fev", "mar", "abr", "mai", "jun",
    "jul", "ago", "set", "out", "nov", "dez",
  ];
  return `${quando.getDate()} de ${meses[quando.getMonth()]}`;
}
