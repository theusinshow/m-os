import { describe, expect, it } from "vitest";
import { alinhamento } from "./malha";
import type { AparelhoNaMalha } from "./types";

function aparelho(id: string, manifesto: AparelhoNaMalha["manifesto"]): AparelhoNaMalha {
  return {
    id,
    nome: id,
    plataforma: "windows",
    versao: "0.4.0",
    contrato: 1,
    vistoEm: "2026-09-04T12:00:00Z",
    manifesto,
  };
}

describe("o alinhamento da malha", () => {
  it("diz alinhado quando contagem e hash batem", () => {
    const eu = aparelho("eu", [{ familia: "task", contagem: 17, hash: "aa" }]);
    const outro = aparelho("outro", [{ familia: "task", contagem: 17, hash: "aa" }]);
    expect(alinhamento([eu, outro], "eu").map((a) => a.estado)).toEqual(["alinhado", "alinhado"]);
  });

  // Atrás e divergente pedem ações diferentes: o primeiro passa sozinho na
  // próxima rodada, o segundo precisa de reparo.
  it("diz atras quando o outro tem menos", () => {
    const eu = aparelho("eu", [{ familia: "task", contagem: 17, hash: "aa" }]);
    const outro = aparelho("outro", [{ familia: "task", contagem: 12, hash: "bb" }]);
    const dele = alinhamento([eu, outro], "eu").find((a) => a.id === "outro");
    expect(dele?.estado).toBe("atras");
    expect(dele?.detalhe).toBe("task: 12 de 17");
  });

  it("diz atras quando o outro nem tem a familia", () => {
    const eu = aparelho("eu", [{ familia: "time_entry", contagem: 26, hash: "aa" }]);
    const outro = aparelho("outro", [{ familia: "task", contagem: 1, hash: "bb" }]);
    const dele = alinhamento([eu, outro], "eu").find((a) => a.id === "outro");
    expect(dele?.estado).toBe("atras");
    expect(dele?.detalhe).toBe("time_entry: 0 de 26");
  });

  it("diz divergente quando a contagem bate e o hash nao", () => {
    const eu = aparelho("eu", [{ familia: "task", contagem: 17, hash: "aa" }]);
    const outro = aparelho("outro", [{ familia: "task", contagem: 17, hash: "zz" }]);
    const dele = alinhamento([eu, outro], "eu").find((a) => a.id === "outro");
    expect(dele?.estado).toBe("divergente");
    expect(dele?.detalhe).toBe("task: mesma contagem, conteúdo diferente");
  });

  // Sem manifesto não é divergência: é um aparelho que ainda não atualizou.
  it("nao acusa quem ainda nao manda manifesto", () => {
    const eu = aparelho("eu", [{ familia: "task", contagem: 17, hash: "aa" }]);
    const outro = aparelho("outro", []);
    const dele = alinhamento([eu, outro], "eu").find((a) => a.id === "outro");
    expect(dele?.estado).toBe("alinhado");
    expect(dele?.detalhe).toBe("sem manifesto");
  });

  // O que o outro tem a MAIS não é problema deste aparelho: ele descobre na
  // própria linha, e acusar aqui faria toda malha parecer quebrada.
  it("nao acusa quando o outro tem a mais", () => {
    const eu = aparelho("eu", [{ familia: "task", contagem: 17, hash: "aa" }]);
    const outro = aparelho("outro", [
      { familia: "task", contagem: 17, hash: "aa" },
      { familia: "capture", contagem: 3, hash: "cc" },
    ]);
    const dele = alinhamento([eu, outro], "eu").find((a) => a.id === "outro");
    expect(dele?.estado).toBe("alinhado");
  });
});
