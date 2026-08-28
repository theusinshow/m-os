// @vitest-environment jsdom

import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { CategoryChips } from "@/components/bills/category-chips";
import { ValidatedForm } from "@/components/ui/validated-form";
import { ToastProvider } from "@/components/ui/toast";
import { successState, type FormState } from "@/lib/form-state";

afterEach(cleanup);

const categories = [
  { id: "cat-faculdade", name: "Faculdade" },
  { id: "cat-moradia", name: "Moradia" },
];

/** Guarda o que cada submit enviou, que e onde o bug aparecia de verdade. */
function setup() {
  const enviados: (string | null)[] = [];

  async function action(_prev: FormState, data: FormData): Promise<FormState> {
    enviados.push(data.get("categoryId") as string | null);
    return successState("Conta adicionada.");
  }

  render(
    <ToastProvider>
      <ValidatedForm action={action} successMessage="Conta adicionada." resetOnSuccess>
        <CategoryChips categories={categories} />
        <button type="submit">Adicionar conta</button>
      </ValidatedForm>
    </ToastProvider>,
  );

  return { enviados };
}

const chip = (nome: string) => screen.getByRole("button", { name: nome });
const aceso = (nome: string) => chip(nome).getAttribute("aria-pressed") === "true";

describe("CategoryChips dentro de um form com resetOnSuccess", () => {
  it("nao carrega a categoria da conta anterior para a proxima", async () => {
    const user = userEvent.setup();
    const { enviados } = setup();

    await user.click(chip("Faculdade"));
    expect(aceso("Faculdade")).toBe(true);

    await user.click(screen.getByRole("button", { name: "Adicionar conta" }));
    await waitFor(() => expect(enviados).toHaveLength(1));

    // O reset tem de apagar o chip na tela...
    await waitFor(() => expect(aceso("Faculdade")).toBe(false));

    // ...e, o que importa, no que o proximo submit envia. Era aqui que quatro
    // contas seguidas nasciam em Faculdade sem ninguem escolher.
    await user.click(screen.getByRole("button", { name: "Adicionar conta" }));
    await waitFor(() => expect(enviados).toHaveLength(2));

    expect(enviados[0]).toBe("cat-faculdade");
    expect(enviados[1]).toBe("");
  });

  it("continua deixando escolher uma categoria diferente depois do reset", async () => {
    const user = userEvent.setup();
    const { enviados } = setup();

    await user.click(chip("Faculdade"));
    await user.click(screen.getByRole("button", { name: "Adicionar conta" }));
    await waitFor(() => expect(enviados).toHaveLength(1));

    await user.click(chip("Moradia"));
    expect(aceso("Moradia")).toBe(true);
    expect(aceso("Faculdade")).toBe(false);

    await user.click(screen.getByRole("button", { name: "Adicionar conta" }));
    await waitFor(() => expect(enviados).toHaveLength(2));

    expect(enviados[1]).toBe("cat-moradia");
  });
});
