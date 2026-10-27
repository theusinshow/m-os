import { describe, expect, it } from "vitest";
import { shouldReload } from "@/lib/version-check";

describe("shouldReload", () => {
  it("recarrega quando o servidor está noutra versão", () => {
    expect(shouldReload({ running: "a", served: "b", alreadyReloadedFor: null })).toBe(true);
  });

  it("não recarrega na mesma versão, nem sem resposta", () => {
    expect(shouldReload({ running: "a", served: "a", alreadyReloadedFor: null })).toBe(false);
    expect(shouldReload({ running: "a", served: null, alreadyReloadedFor: null })).toBe(false);
  });

  it("recarrega uma vez por versão — sem laço se a CDN ainda serve a velha", () => {
    expect(shouldReload({ running: "a", served: "b", alreadyReloadedFor: "b" })).toBe(false);
    expect(shouldReload({ running: "a", served: "c", alreadyReloadedFor: "b" })).toBe(true);
  });

  it("em desenvolvimento nunca recarrega", () => {
    expect(shouldReload({ running: "dev", served: "b", alreadyReloadedFor: null })).toBe(false);
  });
});
