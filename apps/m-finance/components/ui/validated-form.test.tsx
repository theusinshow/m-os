// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { orphanFieldErrors } from "@/components/ui/validated-form";

function formWith(html: string) {
  const form = document.createElement("form");
  form.innerHTML = html;
  document.body.append(form);
  return form;
}

describe("orphanFieldErrors", () => {
  it("erro num campo visível tem onde aparecer", () => {
    const form = formWith('<input name="amount" />');

    expect(orphanFieldErrors(form, { amount: "Informe um valor." })).toEqual([]);
  });

  it("erro num campo escondido não tem onde aparecer — foi assim que a compra sumia", () => {
    const form = formWith('<input name="installments" type="hidden" value="1" />');

    expect(orphanFieldErrors(form, { installments: "Mínimo de 2 parcelas." })).toEqual([
      "installments",
    ]);
  });

  it("erro num campo que nem existe no form também é órfão", () => {
    const form = formWith('<input name="amount" />');

    expect(orphanFieldErrors(form, { cardId: "Cartão não encontrado." })).toEqual(["cardId"]);
  });

  it("separa o que tem tela do que não tem", () => {
    const form = formWith('<input name="amount" /><input name="installments" type="hidden" />');

    expect(
      orphanFieldErrors(form, { amount: "a", installments: "b" }),
    ).toEqual(["installments"]);
  });

  it("sem erros, nada é órfão", () => {
    expect(orphanFieldErrors(formWith('<input name="amount" />'), undefined)).toEqual([]);
  });

  it("sem form montado, todo erro é órfão", () => {
    expect(orphanFieldErrors(null, { amount: "a" })).toEqual(["amount"]);
  });
});
