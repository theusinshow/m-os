import { describe, expect, it } from "vitest";
import { contagemDe, DESTINOS } from "./navegacao";
import { FALSO } from "./falso";

describe("a barra de baixo", () => {
  it("tem cinco destinos, e lembretes nao e um deles", () => {
    expect(DESTINOS.map((d) => d.pagina)).toEqual([
      "home",
      "capturar",
      "inbox",
      "tasks",
      "mais",
    ]);
  });

  it("conta a inbox e as tasks abertas, e nao as feitas", () => {
    // A task "done" do banco falso nao entra: um badge que sobe com coisa
    // resolvida e um badge que se aprende a ignorar.
    expect(contagemDe("inbox", FALSO)).toBe(3);
    expect(contagemDe("tasks", FALSO)).toBe(2);
  });

  it("poe em `mais` o que cobra acao — o lembrete vencido", () => {
    expect(contagemDe("mais", FALSO)).toBe(1);
  });

  it("nao conta nada em home nem em capturar", () => {
    expect(contagemDe("home", FALSO)).toBe(0);
    expect(contagemDe("capturar", FALSO)).toBe(0);
  });
});
