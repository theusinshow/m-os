import { describe, expect, it } from "vitest";
import { cookieDoEmbed } from "./cookies";

/**
 * O valor destas tres opcoes e o que separa "logo uma vez" de "logo toda vez
 * que troco de aba no M/OS". Elas nao aparecem em teste de fluxo — o cookie so
 * falha no navegador de verdade, dentro do iframe — entao ficam presas aqui.
 */
describe("cookie da sessao dentro do embed", () => {
  it("viaja em contexto cross-site", () => {
    expect(cookieDoEmbed.sameSite).toBe("none");
  });

  /** `SameSite=None` sem `Secure` e recusado pelo navegador, calado. */
  it("vem com Secure, que None exige", () => {
    expect(cookieDoEmbed.secure).toBe(true);
  });

  /** Sobrevive ao fim dos cookies de terceiros no Chrome. */
  it("e particionado por CHIPS", () => {
    expect(cookieDoEmbed.partitioned).toBe(true);
  });
});
