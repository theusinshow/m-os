import { describe, expect, it } from "vitest";
import { buildCsv } from "./csv";

describe("buildCsv", () => {
  it("usa ';' como separador e junta linhas com CRLF, com BOM", () => {
    const csv = buildCsv(["A", "B"], [["1", "2"]]);
    expect(csv.charCodeAt(0)).toBe(0xfeff); // BOM
    const body = csv.slice(1);
    expect(body).toBe("A;B\r\n1;2");
  });

  it("escapa campos com separador, aspas ou quebra de linha", () => {
    const csv = buildCsv(["X"], [['a;b'], ['diz "oi"'], ["linha\nnova"]]);
    const body = csv.slice(1);
    expect(body).toContain('"a;b"');
    expect(body).toContain('"diz ""oi"""');
    expect(body).toContain('"linha\nnova"');
  });
});
