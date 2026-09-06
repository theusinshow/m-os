import { describe, expect, it } from "vitest";
import { periodo, pontaCurta } from "./janelas";

/** Sábado, 5 de setembro de 2026, meio-dia. */
const AGORA = new Date(2026, 8, 5, 12, 0, 0);

function dia(quando: Date): string {
  return `${quando.getFullYear()}-${String(quando.getMonth() + 1).padStart(2, "0")}-${String(
    quando.getDate(),
  ).padStart(2, "0")}`;
}

describe("as janelas de horas", () => {
  it("a semana comeca na segunda", () => {
    const [de, ate] = periodo("semana", AGORA);
    expect(dia(de)).toBe("2026-08-31");
    expect(de.getHours()).toBe(0);
    expect(ate).toEqual(AGORA);
  });

  it("a semana passada e a anterior inteira", () => {
    const [de, ate] = periodo("passada", AGORA);
    expect(dia(de)).toBe("2026-08-24");
    expect(dia(ate)).toBe("2026-08-30");
  });

  // Sem o milissegundo de recuo, a semana passada e a atual dividiriam a
  // meia-noite de segunda, e uma hora lançada exatamente ali contaria duas
  // vezes — nas duas janelas.
  it("a semana passada termina antes de a atual comecar", () => {
    const [, fimDaPassada] = periodo("passada", AGORA);
    const [inicioDaAtual] = periodo("semana", AGORA);
    expect(fimDaPassada.getTime()).toBeLessThan(inicioDaAtual.getTime());
  });

  it("o mes comeca no dia 1", () => {
    const [de, ate] = periodo("mes", AGORA);
    expect(dia(de)).toBe("2026-09-01");
    expect(ate).toEqual(AGORA);
  });

  it("o mes passado vai do 1 ao ultimo dia dele", () => {
    const [de, ate] = periodo("mes-passado", AGORA);
    expect(dia(de)).toBe("2026-08-01");
    expect(dia(ate)).toBe("2026-08-31");
  });

  /// Fevereiro de ano bissexto é onde a conta de "último dia do mês" costuma
  /// quebrar. 2028 é bissexto: 29 dias.
  it("o mes passado acerta fevereiro de ano bissexto", () => {
    const marco = new Date(2028, 2, 10, 12, 0, 0);
    const [de, ate] = periodo("mes-passado", marco);
    expect(dia(de)).toBe("2028-02-01");
    expect(dia(ate)).toBe("2028-02-29");
  });

  it("a virada do ano nao quebra o mes passado", () => {
    const janeiro = new Date(2026, 0, 10, 12, 0, 0);
    const [de, ate] = periodo("mes-passado", janeiro);
    expect(dia(de)).toBe("2025-12-01");
    expect(dia(ate)).toBe("2025-12-31");
  });

  it("tudo comeca antes do M/OS existir e termina agora", () => {
    const [de, ate] = periodo("tudo", AGORA);
    expect(de.getFullYear()).toBe(2020);
    expect(ate).toEqual(AGORA);
  });
});

describe("o rotulo da ponta", () => {
  it("escreve dia e mes curto", () => {
    expect(pontaCurta(new Date(2026, 8, 5))).toBe("5 de set");
    expect(pontaCurta(new Date(2026, 0, 31))).toBe("31 de jan");
  });
});
