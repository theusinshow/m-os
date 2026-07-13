import { describe, expect, it } from "vitest";
import { roundDuration } from "./rounding";

describe("roundDuration", () => {
  it("retorna a duracao original quando desativado", () => {
    expect(
      roundDuration(4020, { enabled: false, intervalMinutes: 15, mode: "up" }),
    ).toBe(4020);
  });

  it("modo 'up' arredonda 1h07 para 1h15 com intervalo de 15min", () => {
    // 4020s = 1h07 -> 4500s = 1h15
    expect(
      roundDuration(4020, { enabled: true, intervalMinutes: 15, mode: "up" }),
    ).toBe(4500);
  });

  it("modo 'down' arredonda 1h07 para 1h00", () => {
    expect(
      roundDuration(4020, { enabled: true, intervalMinutes: 15, mode: "down" }),
    ).toBe(3600);
  });

  it("modo 'nearest' arredonda para o intervalo mais proximo", () => {
    // 1h07 (4020s) -> mais proximo de 1h00 (3600) do que de 1h15 (4500)
    expect(
      roundDuration(4020, {
        enabled: true,
        intervalMinutes: 15,
        mode: "nearest",
      }),
    ).toBe(3600);
    // 1h08 (4080s) -> mais proximo de 1h15
    expect(
      roundDuration(4080, {
        enabled: true,
        intervalMinutes: 15,
        mode: "nearest",
      }),
    ).toBe(4500);
  });

  it("nao arredonda quando o valor ja e multiplo do intervalo", () => {
    expect(
      roundDuration(3600, { enabled: true, intervalMinutes: 15, mode: "up" }),
    ).toBe(3600);
  });

  it("trata intervalo invalido retornando o valor original", () => {
    expect(
      roundDuration(4020, { enabled: true, intervalMinutes: 0, mode: "up" }),
    ).toBe(4020);
  });
});
