import { describe, expect, it } from "vitest";

import { rotulo } from "./Biblioteca";

describe("o nome que a linha da estante mostra", () => {
  it("usa o título quando alguém digitou um", () => {
    expect(rotulo({ title: "Open Props", url: "https://open-props.style" })).toBe("Open Props");
  });

  /* O caso que fez esta função existir: o `mos-core` troca título em branco pela
     própria URL, então a linha nunca recebe vazio — ela recebe a URL inteira, que
     estoura a largura do celular e não diz mais que o domínio. */
  it("encurta para o domínio quando o título É a url", () => {
    expect(rotulo({ title: "https://utopia.fyi/type/", url: "https://utopia.fyi/type/" })).toBe(
      "utopia.fyi",
    );
  });

  it("tira o www, que não distingue nada", () => {
    expect(rotulo({ title: "https://www.radix-ui.com", url: "https://www.radix-ui.com" })).toBe(
      "radix-ui.com",
    );
  });

  /* Endereço malformado não pode apagar a linha da tela: melhor mostrar o texto
     cru que uma lista com um item em branco. */
  it("cai no texto cru quando o endereço não parseia", () => {
    expect(rotulo({ title: "isso nao e url", url: "isso nao e url" })).toBe("isso nao e url");
  });
});
