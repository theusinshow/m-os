/**
 * Script de verificação da Fase 7 — POST /api/generate (modo síncrono).
 * Requer o servidor rodando: npm run dev
 * Execute com: npx tsx scripts/test-phase7.ts [url] [slug]
 */
import { promises as fs } from "node:fs";
import { catalogPath } from "../lib/storage/paths";
import type { ResultEvent, AtlasErrorPayload } from "../lib/types";

const BASE = "http://localhost:3000";
const url  = process.argv[2] ?? "https://example.com";
const slug = process.argv[3] ?? "test-phase7";

let passed = 0;
let failed = 0;

function ok(name: string, fn: () => void): void {
  try { fn(); console.log(`  ✓  ${name}`); passed++; }
  catch (e) { console.error(`  ✗  ${name} — ${e instanceof Error ? e.message : e}`); failed++; }
}

async function checkServer(): Promise<void> {
  try {
    await fetch(BASE, { signal: AbortSignal.timeout(3000) });
  } catch {
    console.error(`\n✗  Servidor não está rodando em ${BASE}.`);
    console.error(`   Execute em outro terminal: npm run dev\n`);
    process.exit(1);
  }
}

async function main() {
  console.log(`\n[atlas:${slug}] POST /api/generate — Fase 7`);
  console.log(`  URL: ${url}\n`);

  await checkServer();
  console.log("  ✓  Servidor detectado em", BASE);

  // ── Caso 1: POST com dados válidos ─────────────────────────────────────────
  console.log("\n── POST válido ──────────────────────────────────────");

  const t0 = Date.now();
  const res = await fetch(`${BASE}/api/generate`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      url,
      name: "Fase 7 Test",
      slug,
      category: "Site Institucional",
      client: "Test Client",
      description: "Verificação da Fase 7.",
    }),
    // Timeout generoso para a captura + thumbnails
    signal: AbortSignal.timeout(90_000),
  });

  const elapsed = ((Date.now() - t0) / 1000).toFixed(1);
  console.log(`  Resposta recebida em ${elapsed}s  (HTTP ${res.status})`);

  ok("HTTP 200", () => {
    if (res.status !== 200) throw new Error(`status = ${res.status}`);
  });

  const body = (await res.json()) as ResultEvent | AtlasErrorPayload;

  ok("step = 'done'", () => {
    if (body.step !== "done") throw new Error(`step = "${body.step}"`);
  });

  const result = body as ResultEvent;

  ok("projectUrl = /projects/[slug]", () => {
    if (result.projectUrl !== `/projects/${slug}`)
      throw new Error(`"${result.projectUrl}"`);
  });

  ok("catalog.version presente", () => {
    if (!result.catalog?.version) throw new Error("version ausente");
  });

  ok("catalog.project.slug correto", () => {
    if (result.catalog.project.slug !== slug)
      throw new Error(`"${result.catalog.project.slug}"`);
  });

  ok("captures.desktop.screenshot começa com /generated/", () => {
    const p = result.catalog.captures.desktop.screenshot;
    if (!p.startsWith("/generated/")) throw new Error(`"${p}"`);
  });

  ok("captures.mobile.screenshot começa com /generated/", () => {
    const p = result.catalog.captures.mobile.screenshot;
    if (!p.startsWith("/generated/")) throw new Error(`"${p}"`);
  });

  ok("thumbnails.main começa com /generated/", () => {
    const p = result.catalog.thumbnails.main;
    if (!p.startsWith("/generated/")) throw new Error(`"${p}"`);
  });

  ok("sections = []", () => {
    if (!Array.isArray(result.catalog.sections) || result.catalog.sections.length !== 0)
      throw new Error(JSON.stringify(result.catalog.sections));
  });

  ok("meta.durationMs > 0", () => {
    if (result.catalog.meta.durationMs <= 0)
      throw new Error(`durationMs = ${result.catalog.meta.durationMs}`);
  });

  // catalog.json no disco (check síncrono via existsSync)
  const { existsSync } = await import("node:fs");
  ok("catalog.json gravado no disco", () => {
    if (!existsSync(catalogPath(slug)))
      throw new Error(`catalog.json não encontrado: ${catalogPath(slug)}`);
  });

  // ── Caso 2: URL inválida retorna 400 ──────────────────────────────────────
  console.log("\n── POST com URL inválida ────────────────────────────");

  const badRes = await fetch(`${BASE}/api/generate`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ url: "not-a-url", name: "T", slug: "t", category: "Site" }),
    signal: AbortSignal.timeout(10_000),
  });

  ok("URL inválida retorna HTTP 4xx", () => {
    if (badRes.status < 400 || badRes.status >= 500)
      throw new Error(`status = ${badRes.status}`);
  });

  const badBody = (await badRes.json()) as AtlasErrorPayload;

  ok("step = 'error'", () => {
    if (badBody.step !== "error") throw new Error(`step = "${badBody.step}"`);
  });

  ok("code = 'INVALID_URL'", () => {
    if (badBody.code !== "INVALID_URL") throw new Error(`code = "${badBody.code}"`);
  });

  ok("message amigável em PT-BR (sem stack trace)", () => {
    if (!badBody.message || badBody.message.includes("Error:") || badBody.message.includes("  at "))
      throw new Error(`message expõe detalhes técnicos: "${badBody.message}"`);
  });

  // ── Resumo ─────────────────────────────────────────────────────────────────
  console.log(`\n${"─".repeat(52)}`);
  console.log(`  ${passed} passou  |  ${failed} falhou\n`);
  if (failed > 0) process.exit(1);
}

main().catch((err) => {
  console.error("\n✗ Erro inesperado:", err instanceof Error ? err.message : err);
  process.exit(1);
});
