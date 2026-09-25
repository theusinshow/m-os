import { describe, expect, it } from "vitest";
import { SemConexao, SemSessao } from "./api";
import { esvaziar, guardar, pendentes, type Armazem, type Pendente } from "./filaDeCaptura";

function memoria(): Armazem & { dados: Map<string, string> } {
  const dados = new Map<string, string>();
  return { dados, getItem: (k) => dados.get(k) ?? null, setItem: (k, v) => { dados.set(k, v); } };
}

const item = (chave: string, texto = `texto ${chave}`): Pendente => ({ chave, texto, capturadaEm: "2026-09-24T08:00:00-03:00" });

describe("fila de captura do aparelho", () => {
  it("guarda e reenvia na ordem, e esvazia o que chegou", async () => {
    const armazem = memoria();
    guardar(item("a"), armazem);
    guardar(item("b"), armazem);
    const enviados: string[] = [];
    const r = await esvaziar(async (p) => { enviados.push(p.chave); }, armazem);
    expect(enviados).toEqual(["a", "b"]);
    expect(r).toEqual({ enviadas: 2, restantes: 0, parou: null });
  });

  it("guardar a mesma chave de novo não cria duas", () => {
    const armazem = memoria();
    guardar(item("a"), armazem);
    guardar(item("a", "editado"), armazem);
    expect(pendentes(armazem)).toHaveLength(1);
  });

  it("sem rede para e mantém tudo", async () => {
    const armazem = memoria();
    guardar(item("a"), armazem);
    guardar(item("b"), armazem);
    const r = await esvaziar(async () => { throw new SemConexao(); }, armazem);
    expect(r).toEqual({ enviadas: 0, restantes: 2, parou: "rede" });
  });

  it("sessão expirada não perde nem envia o resto", async () => {
    const armazem = memoria();
    guardar(item("a"), armazem);
    guardar(item("b"), armazem);
    const tentados: string[] = [];
    const r = await esvaziar(async (p) => { tentados.push(p.chave); throw new SemSessao(); }, armazem);
    expect(tentados).toEqual(["a"]);
    expect(r.parou).toBe("sessao");
    expect(pendentes(armazem).map((p) => p.chave)).toEqual(["a", "b"]);
  });

  it("recusa do servidor fica marcada e a fila segue", async () => {
    const armazem = memoria();
    guardar(item("ruim"), armazem);
    guardar(item("bom"), armazem);
    const r = await esvaziar(async (p) => { if (p.chave === "ruim") throw new Error("A captura nao pode estar vazia."); }, armazem);
    expect(r.enviadas).toBe(1);
    expect(pendentes(armazem)).toEqual([{ ...item("ruim"), recusa: "A captura nao pode estar vazia." }]);
  });

  it("armazenamento que recusa escrever faz guardar lançar", () => {
    const cheio: Armazem = { getItem: () => null, setItem: () => { throw new Error("QuotaExceededError"); } };
    expect(() => guardar(item("a"), cheio)).toThrow();
  });

  it("lista corrompida vira fila vazia, e não um app quebrado", () => {
    const armazem = memoria();
    armazem.setItem("mos-capturas-pendentes", "{não é json");
    expect(pendentes(armazem)).toEqual([]);
  });
});
