import { describe, expect, it } from "vitest";
import {
  CHUNK_BYTES,
  arrastoInterno,
  contextoDoDrop,
  conteudoDoDrop,
  fatias,
  loteTerminou,
  painelEspera,
  pareceUrl,
  primeiraUrl,
  recibo,
  type ItemDoLote,
} from "./dropIngest";

function item(parcial: Partial<ItemDoLote>): ItemDoLote {
  return { chave: "a", nome: "memorial.pdf", status: "guardado", ...parcial };
}

describe("o que acorda a Drop Zone", () => {
  it("arrastar coisa do próprio M/OS não acorda nada", () => {
    // O widget da Home e o card do Kanban carimbam tipos próprios. Se a Drop
    // Zone reagisse a eles, arrumar a Home viraria uma tentativa de ingestão.
    expect(arrastoInterno(["text/mos-widget"])).toBe(true);
    expect(conteudoDoDrop(["text/mos-widget"])).toBe("nenhum");
    expect(conteudoDoDrop(["text/task-id", "text/plain"])).toBe("nenhum");
  });

  it("arquivo vence texto", () => {
    // O Explorer manda os dois ao arrastar um arquivo. Tratar como texto
    // guardaria o nome e jogaria fora o conteúdo.
    expect(conteudoDoDrop(["Files", "text/plain"])).toBe("arquivos");
  });

  it("url vence texto puro, e texto puro ainda entra", () => {
    expect(conteudoDoDrop(["text/uri-list", "text/plain"])).toBe("url");
    expect(conteudoDoDrop(["text/plain"])).toBe("texto");
    expect(conteudoDoDrop([])).toBe("nenhum");
  });
});

describe("url", () => {
  it("só um endereço sozinho é um endereço", () => {
    expect(pareceUrl("https://motion.dev")).toBe(true);
    expect(pareceUrl("  http://localhost:1420/x  ")).toBe(true);
    expect(pareceUrl("olha isto: https://motion.dev")).toBe(false);
    expect(pareceUrl("motion.dev")).toBe(false);
    expect(pareceUrl("file:///C:/segredos.txt")).toBe(false);
  });

  it("a lista de uris ignora comentário e título", () => {
    expect(primeiraUrl("# comentário\r\nhttps://motion.dev\r\nMotion")).toBe("https://motion.dev");
    expect(primeiraUrl("nenhuma url aqui")).toBe("");
  });
});

describe("fatias", () => {
  it("cobre o arquivo inteiro sem sobreposição nem buraco", () => {
    const tamanho = CHUNK_BYTES * 2 + 17;
    const partes = fatias(tamanho);
    expect(partes).toHaveLength(3);
    expect(partes[0][0]).toBe(0);
    expect(partes[partes.length - 1][1]).toBe(tamanho);
    partes.slice(1).forEach(([inicio], indice) => expect(inicio).toBe(partes[indice][1]));
  });

  it("arquivo vazio não gera pedaço nenhum", () => {
    expect(fatias(0)).toEqual([]);
  });

  it("arquivo menor que o pedaço vai numa fatia só", () => {
    expect(fatias(10)).toEqual([[0, 10]]);
  });
});

describe("o painel", () => {
  it("segura quando há erro ou sugestão, e some quando não há", () => {
    expect(painelEspera([item({}), item({ chave: "b", status: "repetido" })])).toBe(false);
    expect(painelEspera([item({ status: "erro" })])).toBe(true);
    expect(painelEspera([item({ sugestao: { projectId: "p", nome: "NexoDoc" } })])).toBe(true);
    // Depois de desfeito, a sugestão deixa de fazer sentido e o painel libera.
    expect(painelEspera([item({ status: "desfeito", sugestao: { projectId: "p", nome: "NexoDoc" } })])).toBe(false);
  });

  it("o lote só termina quando nenhum item está andando", () => {
    expect(loteTerminou([item({}), item({ chave: "b", status: "lendo" })])).toBe(false);
    expect(loteTerminou([item({}), item({ chave: "b", status: "erro" })])).toBe(true);
  });
});

describe("o recibo", () => {
  it("um arquivo com destino diz o destino", () => {
    expect(recibo([item({ destino: "NexoDoc" })])).toBe("Guardado em NexoDoc");
  });

  it("sem contexto, o destino é a própria Library", () => {
    expect(recibo([item({ destino: "Library" })])).toBe("Guardado no M/OS");
  });

  it("um lote misto conta cada desfecho, e nenhum some", () => {
    expect(
      recibo([
        item({ chave: "a" }),
        item({ chave: "b" }),
        item({ chave: "c", status: "repetido" }),
        item({ chave: "d", status: "erro" }),
      ]),
    ).toBe("2 itens guardados · 1 já estava aqui · 1 falhou");
  });

  it("um lote inteiro que falha não anuncia sucesso nenhum", () => {
    expect(recibo([item({ status: "erro" }), item({ chave: "b", status: "erro" })])).toBe("2 falharam");
  });
});

describe("o contexto do drop", () => {
  it("a Task aberta entrega o Project dela, e registra a si mesma", () => {
    // Resource ainda não se relaciona com Task. O `taskId` viaja assim mesmo:
    // é dele que a relação sai no dia em que ela existir.
    expect(
      contextoDoDrop({ pagina: "tasks", taskId: "t1", taskProjectId: "p1", workspaceId: "w1" }),
    ).toEqual({ page: "tasks", projectId: "p1", workspaceId: "w1", taskId: "t1" });
  });

  it("o Project aberto ganha do Project da Task", () => {
    expect(contextoDoDrop({ pagina: "projects", projectId: "p1", taskProjectId: "p2" }).projectId).toBe("p1");
  });

  it("sem contexto nenhum, nada é inventado", () => {
    expect(contextoDoDrop({ pagina: "home" })).toEqual({
      page: "home",
      projectId: null,
      workspaceId: null,
      taskId: null,
    });
  });
});
