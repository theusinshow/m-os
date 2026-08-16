"use client";

import { useState } from "react";
import { MarkPaidButton } from "@/components/payable/mark-paid-button";
import { DashboardCard } from "@/components/dashboard/dashboard-card";
import { InlineEmpty } from "@/components/ui/inline-empty";
import { StatusBadge } from "@/components/status-badge";
import { formatCurrency } from "@/lib/formatters/currency";
import { cn } from "@/lib/utils";

export type CalendarEvent = {
  id: string;
  type: "bill" | "invoice";
  title: string;
  amountCents: number;
  day: number;
  status: "pending" | "paid" | "overdue";
  label: string;
};

const WEEKDAYS = ["Seg", "Ter", "Qua", "Qui", "Sex", "Sáb", "Dom"];

const STATUS_DOT: Record<CalendarEvent["status"], string> = {
  overdue: "bg-accent",
  pending: "bg-status-fair",
  paid: "bg-status-positive",
};

/**
 * Calendário financeiro interativo: cada dia é um botão que filtra o painel
 * lateral. No mobile, os eventos viram pontos de status (os títulos ficam
 * para o painel), resolvendo a perda de informação da versão anterior.
 */
export function FinancialCalendar({
  daysInMonth,
  firstOffset,
  todayDay,
  events,
}: {
  daysInMonth: number;
  firstOffset: number;
  todayDay: number;
  events: CalendarEvent[];
}) {
  const [selectedDay, setSelectedDay] = useState<number | null>(null);

  const eventsByDay = new Map<number, CalendarEvent[]>();
  for (const event of events) {
    const list = eventsByDay.get(event.day) ?? [];
    list.push(event);
    eventsByDay.set(event.day, list);
  }

  const weekEvents = events.filter((event) => {
    if (todayDay < 1) return false;
    return event.status !== "paid" && event.day >= todayDay && event.day <= todayDay + 7;
  });
  const visibleEvents =
    selectedDay === null
      ? weekEvents.length > 0
        ? weekEvents
        : events.filter((event) => event.status !== "paid")
      : (eventsByDay.get(selectedDay) ?? []);
  const panelTitle =
    selectedDay === null
      ? weekEvents.length > 0
        ? "Hoje e próximos 7 dias"
        : "Pendências do mês"
      : `Dia ${selectedDay}`;

  return (
    <section className="grid gap-4 xl:grid-cols-[1fr_380px]">
      <DashboardCard>
        <div className="grid grid-cols-7 gap-1.5 sm:gap-2" role="grid" aria-label="Dias do mês">
          {WEEKDAYS.map((day) => (
            <div
              className="pb-2 text-center text-[10px] font-semibold uppercase tracking-tight text-text-muted sm:text-xs"
              key={day}
              role="columnheader"
            >
              <span className="sm:hidden">{day.slice(0, 1)}</span>
              <span className="hidden sm:inline">{day}</span>
            </div>
          ))}
          {Array.from({ length: firstOffset }, (_, index) => (
            <div key={`empty-${index}`} />
          ))}
          {Array.from({ length: daysInMonth }, (_, index) => {
            const day = index + 1;
            const dayEvents = eventsByDay.get(day) ?? [];
            const hasOverdue = dayEvents.some((event) => event.status === "overdue");
            const hasPending = dayEvents.some((event) => event.status === "pending");
            const isToday = day === todayDay;
            const isSelected = day === selectedDay;

            return (
              <button
                aria-label={
                  dayEvents.length > 0
                    ? `Dia ${day}, ${dayEvents.length} ${dayEvents.length === 1 ? "evento" : "eventos"}`
                    : `Dia ${day}, sem eventos`
                }
                aria-pressed={isSelected}
                className={cn(
                  "focus-ring min-h-14 rounded-md border border-border-subtle bg-background-elevated p-1.5 text-left transition duration-200 hover:border-border-default hover:bg-background-hover sm:min-h-28 sm:p-3",
                  isToday && "border-accent-border bg-accent-soft",
                  isSelected && "border-border-strong bg-background-hover ring-1 ring-accent/40",
                )}
                key={day}
                onClick={() => setSelectedDay(isSelected ? null : day)}
                type="button"
              >
                <div className="flex items-center justify-between gap-2">
                  <span
                    className={cn(
                      "text-sm font-semibold text-text-secondary",
                      isToday && "text-accent",
                    )}
                  >
                    {day}
                  </span>
                  {dayEvents.length > 0 ? (
                    <span className="rounded-sm border border-border-subtle px-1.5 py-0.5 text-[10px] font-semibold text-text-muted">
                      {dayEvents.length}
                    </span>
                  ) : null}
                </div>
                {/* Mobile: pontos de status por evento (até 3). */}
                {dayEvents.length > 0 ? (
                  <div className="mt-2 flex flex-wrap gap-1 sm:hidden" aria-hidden="true">
                    {dayEvents.slice(0, 3).map((event) => (
                      <span
                        className={cn("h-1.5 w-1.5 rounded-full", STATUS_DOT[event.status])}
                        key={`${event.type}-${event.id}`}
                      />
                    ))}
                    {dayEvents.length > 3 ? (
                      <span className="text-[9px] leading-none text-text-muted">
                        +{dayEvents.length - 3}
                      </span>
                    ) : null}
                  </div>
                ) : null}
                {/* Desktop: até 2 chips com título. */}
                <div className="mt-3 hidden space-y-1.5 sm:block">
                  {dayEvents.slice(0, 2).map((event) => (
                    <div
                      className="truncate rounded-sm border border-border-subtle bg-background-card px-2 py-1 text-xs text-text-secondary"
                      key={`${event.type}-${event.id}`}
                      title={event.title}
                    >
                      {event.type === "invoice" ? "Fatura" : "Conta"}: {event.title}
                    </div>
                  ))}
                  {dayEvents.length > 2 ? (
                    <p className="text-xs text-text-muted">+{dayEvents.length - 2} eventos</p>
                  ) : null}
                </div>
                {hasOverdue ? (
                  <span className="mt-1.5 block h-1 rounded-full bg-accent sm:mt-3" />
                ) : hasPending ? (
                  <span className="mt-1.5 block h-1 rounded-full bg-status-fair sm:mt-3" />
                ) : null}
              </button>
            );
          })}
        </div>
      </DashboardCard>

      <DashboardCard
        action={
          selectedDay !== null ? (
            <button
              className="focus-ring rounded-md px-2 py-1 text-xs font-semibold text-accent transition hover:bg-accent-soft"
              onClick={() => setSelectedDay(null)}
              type="button"
            >
              Ver mês todo
            </button>
          ) : undefined
        }
        title={panelTitle}
      >
        <div className="space-y-3">
          {visibleEvents.length === 0 ? (
            <InlineEmpty>
              {selectedDay === null
                ? "Nenhum vencimento em aberto para este mês."
                : "Nada vence neste dia."}
            </InlineEmpty>
          ) : (
            visibleEvents.map((event) => (
              <div
                className="rounded-lg border border-border-subtle bg-background-elevated p-4"
                key={`${event.type}-${event.id}`}
              >
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <p className="font-semibold text-text-primary">{event.title}</p>
                    <p className="mt-1 text-sm text-text-muted">
                      {event.label} · dia {event.day}
                    </p>
                  </div>
                  <StatusBadge status={event.status} />
                </div>
                <div className="mt-4 flex items-center justify-between gap-3">
                  <p className="num font-semibold text-text-primary">
                    {formatCurrency(event.amountCents)}
                  </p>
                  {event.status !== "paid" ? (
                    <MarkPaidButton payableId={event.id} payableType={event.type}>
                      Marcar como pago
                    </MarkPaidButton>
                  ) : null}
                </div>
              </div>
            ))
          )}
        </div>
      </DashboardCard>
    </section>
  );
}
