/**
 * Script de verificação da Fase 8 — streaming SSE do POST /api/generate.
 * Requer o servidor rodando: npm run dev
 * Execute com: npx tsx scripts/test-phase8.ts [url] [slug]
 */
import { existsSync } from "node:fs";
import { catalogPath } from "../lib/storage/paths";
import type { ProgressEvent, ResultEvent, AtlasErrorPayload } from "../lib/types";

const BASE = "http://localhost:3000";
const url  = process.argv[2] ?? "https://example.com";
const slug = process.argv[3] ?? "test-phase8";

type StreamEvent = ProgressEvent | ResultEvent | AtlasErrorPayload;

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
    console.error(`\n✗  Servidor não está em ${BASE}. Execute: npm run dev\n`);
    process.exit(1);
  }
}

async function consumeStream(url: string, body: object): Promise<StreamEvent[]> {
  const res = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
    signal: AbortSignal.timeout(90_000),
  });

  const reader = res.body!.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  const events: StreamEvent[] = [];

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    const lines = buffer.split("\n\n");
    buffer = lines.pop() ?? "";
    for (const line of lines) {
      if (!line.startsWith("data: ")) continue;
      const event = JSON.parse(line.slice(6)) as StreamEvent;
      events.push(event);
    }
  }
  return events;
}

async function main() {
  console.log(`\n[atlas:${slug}] SSE /api/generate — Fase 8`);
  console.log(`  URL: ${url}\n`);

  await checkServer();
  console.log("  ✓  Servidor detectado\n");

  // ── Caso 1: POST válido — consumir stream completo ────────────────────────
  console.log("── stream válido ────────────────────────────────────");

  const t0 = Date.now();
  const events = await consumeStream(`${BASE}/api/generate`, {
    url, name: "Fase 8 Test", slug, category: "Site Institucional",
  });
  const elapsed = ((Date.now() - t0) / 1000).toFixed(1);

  console.log(`\n  Eventos recebidos (${events.length}) em ${elapsed}s:`);
  for (const ev of events) {
    const step = ev.step.padEnd(28);
    const prog = "progress" in ev ? `[${String(ev.progress).padStart(3)}%]` : "[---]";
    const msg  = "message" in ev ? ev.message : "";
    console.log(`    ${prog} ${step} ${msg}`);
  }
  console.log();

  ok("Pelo menos 8 eventos recebidos", () => {
    if (events.length < 8) throw new Error(`${events.length} eventos`);
  });

  const steps = events.map(e => e.step);

  ok("Primeiro evento: 'validating'", () => {
    if (steps[0] !== "validating") throw new Error(`"${steps[0]}"`);
  });

  ok("'launching' presente", () => {
    if (!steps.includes("launching")) throw new Error("ausente");
  });

  ok("'capturing-desktop' presente", () => {
    if (!steps.includes("capturing-desktop")) throw new Error("ausente");
  });

  ok("'capturing-fullpage-desktop' presente", () => {
    if (!steps.includes("capturing-fullpage-desktop")) throw new Error("ausente");
  });

  ok("'capturing-mobile' presente", () => {
    if (!steps.includes("capturing-mobile")) throw new Error("ausente");
  });

  ok("'capturing-fullpage-mobile' presente", () => {
    if (!steps.includes("capturing-fullpage-mobile")) throw new Error("ausente");
  });

  ok("'generating-thumbnails' presente", () => {
    if (!steps.includes("generating-thumbnails")) throw new Error("ausente");
  });

  ok("'writing-catalog' presente", () => {
    if (!steps.includes("writing-catalog")) throw new Error("ausente");
  });

  ok("Último evento: 'done'", () => {
    if (steps.at(-1) !== "done") throw new Error(`último = "${steps.at(-1)}"`);
  });

  // Verificar evento final (ResultEvent)
  const lastEvent = events.at(-1) as ResultEvent;

  ok("ResultEvent tem catalog", () => {
    if (!("catalog" in lastEvent)) throw new Error("catalog ausente");
  });

  ok("ResultEvent tem projectUrl", () => {
    if (!("projectUrl" in lastEvent)) throw new Error("projectUrl ausente");
    if (lastEvent.projectUrl !== `/projects/${slug}`)
      throw new Error(`"${lastEvent.projectUrl}"`);
  });

  ok("Progress cresce monotonicamente", () => {
    const progEvents = events.filter(e => "progress" in e) as ProgressEvent[];
    for (let i = 1; i < progEvents.length; i++) {
      if (progEvents[i].progress < progEvents[i - 1].progress) {
        throw new Error(
          `${progEvents[i].step}(${progEvents[i].progress}) < ${progEvents[i-1].step}(${progEvents[i-1].progress})`
        );
      }
    }
  });

  ok("catalog.json gravado no disco", () => {
    if (!existsSync(catalogPath(slug)))
      throw new Error(`catalog.json não encontrado: ${catalogPath(slug)}`);
  });

  // ── Caso 2: URL inválida → evento de erro no stream ───────────────────────
  console.log("\n── stream com URL inválida ──────────────────────────");

  const errEvents = await consumeStream(`${BASE}/api/generate`, {
    url: "nao-e-uma-url", name: "T", slug: "t", category: "Site",
  });

  ok("Recebe pelo menos 1 evento de erro", () => {
    if (errEvents.length === 0) throw new Error("nenhum evento");
  });

  const errEvent = errEvents.find(e => e.step === "error") as AtlasErrorPayload | undefined;

  ok("step = 'error'", () => {
    if (!errEvent) throw new Error("evento error não encontrado");
  });

  ok("code = 'INVALID_URL'", () => {
    if (errEvent?.code !== "INVALID_URL") throw new Error(`code = "${errEvent?.code}"`);
  });

  ok("message amigável (sem stack trace)", () => {
    const msg = errEvent?.message ?? "";
    if (msg.includes("Error:") || msg.includes("  at "))
      throw new Error(`message técnica: "${msg}"`);
  });

  // ── Resumo ─────────────────────────────────────────────────────────────────
  console.log(`\n${"─".repeat(52)}`);
  console.log(`  ${passed} passou  |  ${failed} falhou\n`);
  if (failed > 0) process.exit(1);
}

main().catch(err => {
  console.error("\n✗ Erro inesperado:", err instanceof Error ? err.message : err);
  process.exit(1);
});
