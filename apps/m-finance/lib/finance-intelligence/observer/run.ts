import { and, desc, eq, inArray } from "drizzle-orm";
import { db } from "@/db/client";
import {
  financialInsights,
  financialObserverRuns,
  type FinancialInsightStatus,
} from "@/db/schema";
import { narrateObservation } from "@/lib/finance-intelligence/ai/analyst";
import { safeToSpend } from "@/lib/finance-intelligence/kernel/safe-to-spend";
import { runDetectors, severityRank } from "@/lib/finance-intelligence/observer/detectors";
import { decideInsightUpdate, staleInsights } from "@/lib/finance-intelligence/observer/lifecycle";
import { loadFinanceSnapshot } from "@/lib/finance-intelligence/snapshot/load";
import { sendPushToUser } from "@/lib/push/web-push";

const LIVE: FinancialInsightStatus[] = ["open", "acknowledged"];

export async function getPreviousSafeToSpend(userId: string) {
  if (!db) return null;
  const [row] = await db
    .select({ safeToSpendCents: financialObserverRuns.safeToSpendCents })
    .from(financialObserverRuns)
    .where(eq(financialObserverRuns.userId, userId))
    .orderBy(desc(financialObserverRuns.ranAt))
    .limit(1);
  return row?.safeToSpendCents ?? null;
}

/**
 * Uma rodada do Observer: detectores determinísticos → dedupe/cooldown →
 * persistência. A LLM só entra depois, e só para narrar o que o código já
 * achou material (quando `FINANCE_AI_NARRATE_INSIGHTS=true`).
 */
export async function runFinancialObserver(userId: string, now = new Date()) {
  if (!db) throw new Error("Banco de dados indisponível.");
  const snapshot = await loadFinanceSnapshot(userId, now);
  const previous = await getPreviousSafeToSpend(userId);
  const observations = runDetectors({ snapshot, previousSafeToSpendCents: previous });

  const live = await db
    .select()
    .from(financialInsights)
    .where(and(eq(financialInsights.userId, userId), inArray(financialInsights.status, LIVE)));
  const byKey = new Map(live.map((row) => [row.dedupeKey, row]));

  const counts = { created: 0, touched: 0, escalated: 0, reopened: 0, resolved: 0, notified: 0 };
  const toNotify: { title: string; body: string }[] = [];

  for (const observation of observations) {
    const existing = byKey.get(observation.dedupeKey) ?? null;
    const decision = decideInsightUpdate(existing, observation, now);

    if (decision.action === "create") {
      const narrative = await narrateObservation(observation);
      await db.insert(financialInsights).values({
        userId,
        type: observation.detector,
        severity: observation.severity,
        status: "open",
        title: observation.title,
        summary: observation.summary,
        narrative,
        facts: observation.facts,
        evidence: observation.entityRefs,
        dedupeKey: observation.dedupeKey,
        materialityScore: observation.materialityScore,
        firstSeenAt: now,
        lastSeenAt: now,
      });
      counts.created += 1;
    } else if (existing) {
      const base = { lastSeenAt: now, facts: observation.facts, updatedAt: now };
      if (decision.action === "touch") {
        await db.update(financialInsights).set(base).where(eq(financialInsights.id, existing.id));
        counts.touched += 1;
      } else {
        await db
          .update(financialInsights)
          .set({
            ...base,
            severity: observation.severity,
            title: observation.title,
            summary: observation.summary,
            materialityScore: observation.materialityScore,
            evidence: observation.entityRefs,
            status: "open",
            acknowledgedAt: null,
          })
          .where(eq(financialInsights.id, existing.id));
        counts[decision.action === "reopen" ? "reopened" : "escalated"] += 1;
      }
    }

    if ("notify" in decision && decision.notify) {
      toNotify.push({ title: observation.title, body: observation.summary });
    }
  }

  const stale = staleInsights(live, new Set(observations.map((row) => row.dedupeKey)));
  for (const item of stale) {
    await db
      .update(financialInsights)
      .set({ status: item.status, resolvedAt: now, updatedAt: now })
      .where(eq(financialInsights.id, item.id));
    counts.resolved += 1;
  }

  const current = safeToSpend(snapshot);
  await db.insert(financialObserverRuns).values({
    userId,
    ranAt: now,
    safeToSpendCents: current.safeToSpendCents,
    observations: observations.length,
    facts: { counts, month: current.key, previousSafeToSpendCents: previous },
  });

  // Push só para o crítico que nasceu ou piorou — nunca para todo insight.
  for (const push of toNotify) {
    try {
      counts.notified += await sendPushToUser(userId, {
        title: push.title,
        body: push.body,
        url: "/app/dashboard#insights",
        tag: "finance-insight",
      });
    } catch {
      // Push falhar não desfaz o insight gravado.
    }
  }

  return { observations: observations.length, ...counts, safeToSpendCents: current.safeToSpendCents };
}

export async function getStoredInsights(userId: string, limit = 20) {
  if (!db) return [];
  const rows = await db
    .select()
    .from(financialInsights)
    .where(and(eq(financialInsights.userId, userId), inArray(financialInsights.status, LIVE)))
    .orderBy(desc(financialInsights.lastSeenAt))
    .limit(50);
  return rows
    .sort(
      (a, b) =>
        (a.status === "open" ? 0 : 1) - (b.status === "open" ? 0 : 1) ||
        severityRank(b.severity) - severityRank(a.severity) ||
        b.materialityScore - a.materialityScore,
    )
    .slice(0, limit);
}

export async function setInsightStatus(
  userId: string,
  insightId: string,
  status: "acknowledged" | "resolved",
  now = new Date(),
) {
  if (!db) return false;
  const [row] = await db
    .update(financialInsights)
    .set(
      status === "acknowledged"
        ? { status, acknowledgedAt: now, updatedAt: now }
        : { status, resolvedAt: now, updatedAt: now },
    )
    .where(
      and(
        eq(financialInsights.id, insightId),
        eq(financialInsights.userId, userId),
        inArray(financialInsights.status, LIVE),
      ),
    )
    .returning({ id: financialInsights.id });
  return Boolean(row);
}
