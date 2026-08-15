import { describe, expect, it } from "vitest";
import { isoToLocalInput, localInputToIso } from "./datetime";

describe("datetime (round-trip local)", () => {
  it("ida e volta preserva o instante (precisao de minuto)", () => {
    const iso = "2026-07-11T08:30:00.000Z";
    const local = isoToLocalInput(iso);
    // Formato datetime-local: "YYYY-MM-DDTHH:MM".
    expect(local).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}$/);
    const back = localInputToIso(local);
    // Mesma marca temporal, truncada ao minuto.
    expect(new Date(back).getTime()).toBe(Date.parse(iso));
  });
});
