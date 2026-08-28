import { describe, expect, it } from "vitest";
import { decidirAposFalha } from "./hermesSupervisor";

describe("o que para o supervisor", () => {
  it("credencial recusada para: insistir não muda uma senha errada", () => {
    expect(decidirAposFalha({ kind: "unauthorized", message: "401", retriable: false }))
      .toEqual({ acao: "parar", causa: "unauthorized" });
  });

  it("rate limit para: repetir foi o que causou o bloqueio", () => {
    expect(decidirAposFalha({ kind: "rate_limited", message: "429", retriable: false }))
      .toEqual({ acao: "parar", causa: "rate_limited" });
  });
});

describe("o que NÃO para o supervisor", () => {
  it("túnel fechado repete — ele abre depois", () => {
    expect(decidirAposFalha({ kind: "unreachable", message: "sem túnel", retriable: true }).acao)
      .toBe("repetir");
  });

  it("o erro da abertura repete, e é o defeito que este arquivo existe para consertar", () => {
    // `CoreError` tem `retryable` (com Y); `HermesFailure` tem `retriable`.
    // A versão antiga lia `retriable` num CoreError, achava `undefined`, e
    // parava para sempre — num PC onde tudo estava certo.
    const doPortao = { message: "O M/OS ainda esta abrindo.", retryable: true };
    expect(decidirAposFalha(doPortao).acao).toBe("repetir");
  });

  it("erro sem forma nenhuma repete: desconhecido não é fatal", () => {
    expect(decidirAposFalha("caiu a rede").acao).toBe("repetir");
    expect(decidirAposFalha(null).acao).toBe("repetir");
    expect(decidirAposFalha({}).acao).toBe("repetir");
  });

  it("um kind que esta versão não conhece repete, em vez de matar a ponte", () => {
    expect(decidirAposFalha({ kind: "gateway", message: "502" }).acao).toBe("repetir");
    expect(decidirAposFalha({ kind: "motivo_do_futuro", message: "?" }).acao).toBe("repetir");
  });
});
