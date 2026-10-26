"use client";

import { useRef, useState, useTransition } from "react";
import { useRouter } from "next/navigation";
import { ChevronLeft, ChevronRight, FileUp, Repeat, RotateCcw, X } from "lucide-react";
import { commitStatementAction, previewStatementAction } from "@/app/actions/statement-import";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import { formatCurrency } from "@/lib/formatters/currency";
import { formatMonthLabel, formatShortDate } from "@/lib/formatters/date";
import { centsToInput, parseCurrencyToCents } from "@/lib/money";
import { merchantKey } from "@/lib/statement-import/classify";
import type { PreviewItem, StatementPreview } from "@/lib/statement-import/service";
import { cn } from "@/lib/utils";

type Row = PreviewItem & { included: boolean; touched: boolean };
type Filter = "all" | "uncategorized" | "subscriptions" | "installments";

function shiftMonth(parts: { month: number; year: number }, delta: number) {
  const index = parts.year * 12 + (parts.month - 1) + delta;
  return { year: Math.floor(index / 12), month: (index % 12) + 1 };
}

export function StatementImport({ cardId, cardName }: { cardId: string; cardName: string }) {
  const router = useRouter();
  const { addToast } = useToast();
  const [pending, startTransition] = useTransition();
  const [mode, setMode] = useState<"file" | "text">("file");
  const [raw, setRaw] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [preview, setPreview] = useState<StatementPreview | null>(null);
  const [rows, setRows] = useState<Row[]>([]);
  const [owed, setOwed] = useState("");
  const [filter, setFilter] = useState<Filter>("all");
  const fileInput = useRef<HTMLInputElement>(null);

  function read(content: string, month?: { month: number; year: number }) {
    setError(null);
    startTransition(async () => {
      const result = await previewStatementAction({ cardId, raw: content, month });
      if (!result.ok) {
        setError(result.message);
        return;
      }
      // Trocar o mês relê a fatura (o que já foi importado muda), mas o que a
      // pessoa já escolheu na revisão continua escolhido.
      const previous = new Map(rows.map((row) => [row.externalId, row]));
      setPreview(result.preview);
      setRows(
        result.preview.items.map((item) => {
          const kept = previous.get(item.externalId);
          return kept
            ? { ...item, categoryId: kept.categoryId, isSubscription: kept.isSubscription, included: kept.included, touched: kept.touched }
            : { ...item, included: true, touched: false };
        }),
      );
      if (!month) setOwed(centsToInput(result.preview.owedCents));
    });
  }

  async function onFile(file: File | undefined) {
    if (!file) return;
    const content = await file.text();
    setRaw(content);
    read(content);
  }

  function update(externalId: string, change: Partial<Pick<Row, "categoryId" | "isSubscription" | "included">>) {
    setRows((current) => {
      const target = current.find((row) => row.externalId === externalId);
      if (!target) return current;
      const key = merchantKey(target.description);
      return current.map((row) => {
        if (row.externalId === externalId) return { ...row, ...change, touched: true };
        // A mesma origem segue a escolha — nove "iFood" não pedem nove toques —
        // a menos que alguém já tenha mexido nela.
        if ("categoryId" in change && !row.touched && merchantKey(row.description) === key) {
          return { ...row, categoryId: change.categoryId ?? null };
        }
        return row;
      });
    });
  }

  const categoryName = new Map(preview?.categories.map((category) => [category.id, category.name]) ?? []);

  const active = rows.filter((row) => row.included);
  const fresh = active.filter((row) => !row.duplicate);
  const chargesCents = active.reduce((total, row) => total + row.amountCents, 0);
  const owedCents = parseCurrencyToCents(owed);
  const futureInstallments = fresh.reduce(
    (total, row) => total + (row.installmentNumber && row.installmentTotal ? row.installmentTotal - row.installmentNumber : 0),
    0,
  );

  const totals = new Map<string, number>();
  for (const row of active) {
    const name = row.categoryId ? (categoryName.get(row.categoryId) ?? "Outros") : "Sem categoria";
    totals.set(name, (totals.get(name) ?? 0) + row.amountCents);
  }
  const byCategory = [...totals.entries()].sort((a, b) => b[1] - a[1]);

  const counts = {
    all: rows.length,
    uncategorized: rows.filter((row) => !row.categoryId).length,
    subscriptions: rows.filter((row) => row.isSubscription).length,
    installments: rows.filter((row) => row.installmentNumber).length,
  };
  const visible = rows.filter((row) =>
    filter === "uncategorized"
      ? !row.categoryId
      : filter === "subscriptions"
        ? row.isSubscription
        : filter === "installments"
          ? row.installmentNumber
          : true,
  );

  function save() {
    if (!preview) return;
    startTransition(async () => {
      const result = await commitStatementAction({
        cardId,
        month: preview.month,
        owedCents,
        creditsCents: preview.creditsCents,
        items: active.map((row) => ({
          externalId: row.externalId,
          description: row.description,
          amountCents: row.amountCents,
          purchaseDate: row.purchaseDate,
          installmentNumber: row.installmentNumber,
          installmentTotal: row.installmentTotal,
          categoryId: row.categoryId,
          isSubscription: row.isSubscription,
        })),
      });
      if (!result.ok) {
        addToast(result.message, "error");
        return;
      }
      addToast(
        result.created === 0
          ? "Nada novo: essa fatura já estava importada. Categorias atualizadas."
          : `${result.created} lançamento${result.created === 1 ? "" : "s"} gravado${result.created === 1 ? "" : "s"}.`,
        "success",
      );
      router.push(`/app/cards/${cardId}`);
      router.refresh();
    });
  }

  if (!preview) {
    return (
      <DashboardCard
        description="O app lê a fatura, separa compras de pagamentos, reconhece parcelas e sugere a categoria de cada compra. Nada é gravado antes da sua revisão."
        title={`Importar fatura · ${cardName}`}
      >
        <div className="mb-4 inline-flex rounded-md border border-border-subtle p-1">
          {(["file", "text"] as const).map((option) => (
            <button
              aria-pressed={mode === option}
              className={cn(
                "focus-ring min-h-9 rounded px-3 text-sm font-medium transition duration-150",
                mode === option ? "bg-accent-soft text-accent" : "text-text-muted hover:text-text-secondary",
              )}
              key={option}
              onClick={() => setMode(option)}
              type="button"
            >
              {option === "file" ? "Arquivo .ofx" : "Colar texto"}
            </button>
          ))}
        </div>

        {mode === "file" ? (
          <button
            className="focus-ring flex w-full flex-col items-center justify-center gap-2 rounded-lg border border-dashed border-border-default bg-background-elevated px-4 py-10 text-center transition duration-200 hover:border-accent-border hover:bg-background-hover"
            disabled={pending}
            onClick={() => fileInput.current?.click()}
            type="button"
          >
            <FileUp aria-hidden="true" className="text-accent" size={22} />
            <span className="text-sm font-semibold text-text-primary">
              {pending ? "Lendo a fatura..." : "Escolher o arquivo da fatura"}
            </span>
            <span className="text-xs text-text-muted">
              No Nubank: fatura → ⋯ → Exportar fatura → OFX
            </span>
            <input
              accept=".ofx,.qfx,.txt,application/x-ofx"
              className="sr-only"
              onChange={(event) => onFile(event.target.files?.[0])}
              ref={fileInput}
              type="file"
            />
          </button>
        ) : (
          <div className="space-y-3">
            <textarea
              className="field-input min-h-48 w-full font-mono text-xs leading-5"
              onChange={(event) => setRaw(event.target.value)}
              placeholder={"Cole aqui a fatura copiada do app do banco. Exemplo:\nMercado Livre    4 de 10    R$ 62,00\nAmazon    Única    R$ 29,90"}
              value={raw}
            />
            <Button disabled={pending || !raw.trim()} onClick={() => read(raw)} type="button">
              {pending ? "Lendo..." : "Ler fatura"}
            </Button>
          </div>
        )}

        {error ? (
          <p className="mt-4 rounded-lg border border-accent-border bg-accent-soft px-4 py-3 text-sm text-accent">
            {error}
          </p>
        ) : null}
      </DashboardCard>
    );
  }

  return (
    <div className="space-y-6 pb-28">
      <DashboardCard accent>
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">
              Fatura que vence em
            </p>
            <div className="mt-1 flex items-center gap-1">
              <button
                aria-label="Mês anterior"
                className="focus-ring rounded p-1.5 text-text-muted hover:bg-background-hover hover:text-text-primary"
                disabled={pending}
                onClick={() => read(raw, shiftMonth(preview.month, -1))}
                type="button"
              >
                <ChevronLeft size={18} />
              </button>
              <h2 className="min-w-40 text-center text-lg font-semibold text-text-primary">
                {formatMonthLabel(new Date(preview.month.year, preview.month.month - 1, 1))}
              </h2>
              <button
                aria-label="Próximo mês"
                className="focus-ring rounded p-1.5 text-text-muted hover:bg-background-hover hover:text-text-primary"
                disabled={pending}
                onClick={() => read(raw, shiftMonth(preview.month, 1))}
                type="button"
              >
                <ChevronRight size={18} />
              </button>
            </div>
          </div>
          <button
            className="focus-ring inline-flex items-center gap-1.5 text-sm text-text-muted hover:text-text-primary"
            onClick={() => {
              setPreview(null);
              setRows([]);
            }}
            type="button"
          >
            <RotateCcw size={14} aria-hidden="true" /> Outra fatura
          </button>
        </div>

        <div className="mt-5 grid gap-4 sm:grid-cols-3">
          <label className="block">
            <span className="text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">A pagar</span>
            <input
              className="field-input num mt-1 w-full text-xl font-semibold"
              inputMode="decimal"
              onChange={(event) => setOwed(event.target.value)}
              value={owed}
            />
          </label>
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">Compras</p>
            <p className="num mt-2 text-xl font-semibold text-text-primary">{formatCurrency(chargesCents)}</p>
            <p className="text-xs text-text-muted">{active.length} lançamentos</p>
          </div>
          {preview.creditsCents > 0 ? (
            <div>
              <p className="text-xs font-semibold uppercase tracking-[0.14em] text-text-muted">Já abatido</p>
              <p className="num mt-2 text-xl font-semibold text-status-positive">
                {formatCurrency(preview.creditsCents)}
              </p>
              <p className="text-xs text-text-muted">pagamentos antecipados e créditos</p>
            </div>
          ) : null}
        </div>

        {futureInstallments > 0 ? (
          <p className="mt-4 text-sm text-text-muted">
            As parcelas desta fatura também criam {futureInstallments} lançamentos nos próximos meses — a
            previsão das faturas seguintes já nasce certa.
          </p>
        ) : null}
      </DashboardCard>

      <DashboardCard title="Por categoria">
        <ul className="space-y-2">
          {byCategory.map(([name, cents]) => (
            <li key={name}>
              <div className="flex items-baseline justify-between gap-4">
                <span className={cn("text-sm", name === "Sem categoria" ? "text-text-muted" : "font-medium text-text-primary")}>
                  {name}
                </span>
                <span className="num text-sm text-text-secondary">{formatCurrency(cents)}</span>
              </div>
              <div aria-hidden="true" className="mt-1.5 h-1.5 overflow-hidden rounded-full bg-background-elevated">
                <span
                  className={cn("block h-full rounded-full", name === "Sem categoria" ? "bg-text-muted/40" : "bg-accent/70")}
                  style={{ width: `${Math.max(Math.round((cents / Math.max(chargesCents, 1)) * 100), 1)}%` }}
                />
              </div>
            </li>
          ))}
        </ul>
      </DashboardCard>

      <DashboardCard title="Revisar compras">
        <div className="mb-4 flex flex-wrap gap-2">
          {(
            [
              ["all", "Todas"],
              ["uncategorized", "Sem categoria"],
              ["subscriptions", "Assinaturas"],
              ["installments", "Parceladas"],
            ] as const
          ).map(([value, label]) => (
            <button
              aria-pressed={filter === value}
              className={cn(
                "focus-ring inline-flex min-h-9 items-center gap-1.5 rounded-full border px-3 text-sm font-medium transition duration-150",
                filter === value
                  ? "border-accent-border bg-accent-soft text-accent"
                  : "border-border-subtle bg-background-elevated text-text-muted hover:border-border-default hover:text-text-secondary",
              )}
              key={value}
              onClick={() => setFilter(value)}
              type="button"
            >
              {label} <span className="num text-xs opacity-70">{counts[value]}</span>
            </button>
          ))}
        </div>

        <ul className="space-y-2">
          {visible.map((row) => (
            <li
              className={cn(
                "rounded-lg border border-border-subtle bg-background-elevated px-4 py-3 transition duration-150",
                !row.included && "opacity-45",
              )}
              key={row.externalId}
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <p className="truncate font-medium text-text-primary">{row.description}</p>
                  <p className="mt-0.5 flex flex-wrap gap-x-2 text-xs text-text-muted">
                    {row.purchaseDate ? <span>{formatShortDate(row.purchaseDate)}</span> : null}
                    {row.installmentNumber ? (
                      <span>
                        Parcela {row.installmentNumber}/{row.installmentTotal}
                      </span>
                    ) : (
                      <span>À vista</span>
                    )}
                    {row.duplicate ? <span className="text-status-fair">já importada</span> : null}
                    {row.suggestionSource === "learned" && !row.touched ? <span>como da última vez</span> : null}
                  </p>
                </div>
                <div className="flex shrink-0 items-center gap-1">
                  <p className="num font-semibold text-text-primary">{formatCurrency(row.amountCents)}</p>
                  <button
                    aria-label={row.included ? "Deixar de fora" : "Incluir de volta"}
                    className="focus-ring rounded p-1.5 text-text-muted hover:bg-background-hover hover:text-text-primary"
                    onClick={() => update(row.externalId, { included: !row.included })}
                    type="button"
                  >
                    {row.included ? <X size={15} /> : <RotateCcw size={15} />}
                  </button>
                </div>
              </div>

              {row.included ? (
                <div className="mt-3 flex flex-wrap items-center gap-2">
                  <select
                    aria-label={`Categoria de ${row.description}`}
                    className={cn("field-input min-h-9 flex-1 py-1 text-sm sm:max-w-56", !row.categoryId && "text-text-muted")}
                    onChange={(event) => update(row.externalId, { categoryId: event.target.value || null })}
                    value={row.categoryId ?? ""}
                  >
                    <option value="">Sem categoria</option>
                    {preview.categories.map((category) => (
                      <option key={category.id} value={category.id}>
                        {category.name}
                      </option>
                    ))}
                  </select>
                  <button
                    aria-pressed={row.isSubscription}
                    className={cn(
                      "focus-ring inline-flex min-h-9 items-center gap-1.5 rounded-full border px-3 text-sm font-medium transition duration-150",
                      row.isSubscription
                        ? "border-accent-border bg-accent-soft text-accent"
                        : "border-border-subtle text-text-muted hover:border-border-default hover:text-text-secondary",
                    )}
                    onClick={() => update(row.externalId, { isSubscription: !row.isSubscription })}
                    type="button"
                  >
                    <Repeat size={13} aria-hidden="true" />
                    Assinatura
                  </button>
                </div>
              ) : null}
            </li>
          ))}
        </ul>

        {preview.ignored.length > 0 ? (
          <details className="mt-5 text-sm text-text-muted">
            <summary className="cursor-pointer">
              {preview.ignored.length} lançamento{preview.ignored.length === 1 ? "" : "s"} que não são compra
            </summary>
            <ul className="mt-2 space-y-1">
              {preview.ignored.map((item, index) => (
                <li className="flex justify-between gap-4" key={`${item.description}-${index}`}>
                  <span>{item.description}</span>
                  <span className="num">{formatCurrency(item.amountCents)}</span>
                </li>
              ))}
            </ul>
          </details>
        ) : null}
      </DashboardCard>

      <div className="fixed inset-x-0 bottom-[calc(4.5rem+env(safe-area-inset-bottom))] z-30 px-4 lg:bottom-6 lg:left-auto lg:right-8 lg:px-0">
        <div className="mx-auto flex max-w-xl items-center justify-between gap-3 rounded-xl border border-border-default bg-background-card/95 p-3 shadow-2xl shadow-black/30 backdrop-blur">
          <p className="min-w-0 text-sm text-text-secondary">
            <span className="num font-semibold text-text-primary">{fresh.length}</span> novas
            {active.length - fresh.length > 0 ? ` · ${active.length - fresh.length} já importadas` : ""}
          </p>
          <Button disabled={pending || active.length === 0} onClick={save} type="button">
            {pending ? "Gravando..." : "Gravar fatura"}
          </Button>
        </div>
      </div>
    </div>
  );
}
