"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  CalendarDays,
  Calculator,
  CreditCard,
  History,
  LayoutDashboard,
  ReceiptText,
  RefreshCw,
  Settings,
  Target,
} from "lucide-react";
import { TriangleMark } from "@/components/brand/triangle-mark";
import { cn } from "@/lib/utils";

const navGroups = [
  {
    label: null,
    items: [
      { href: "/app/dashboard", label: "Dashboard", icon: LayoutDashboard },
      { href: "/app/bills", label: "Despesas", icon: ReceiptText },
      { href: "/app/cards", label: "Cartões", icon: CreditCard },
      { href: "/app/calendar", label: "Calendário", icon: CalendarDays },
      { href: "/app/subscriptions", label: "Assinaturas", icon: RefreshCw },
    ],
  },
  {
    label: "Planejamento",
    items: [
      { href: "/app/simulator", label: "Simulador", icon: Calculator },
      { href: "/app/goals", label: "Metas", icon: Target },
      { href: "/app/history", label: "Histórico", icon: History },
    ],
  },
  {
    label: null,
    items: [{ href: "/app/settings", label: "Configurações", icon: Settings }],
  },
] as const;

export function Sidebar() {
  const pathname = usePathname();

  return (
    <aside className="fixed inset-y-0 left-0 z-30 hidden w-72 border-r border-border-subtle bg-background-primary/95 px-4 py-5 backdrop-blur-xl lg:block">
      <Link className="focus-ring group mb-8 flex min-h-11 items-center gap-3 rounded-md px-2" href="/app/dashboard">
        <span className="clip-notch flex h-10 w-10 items-center justify-center border border-accent-border bg-accent-soft text-accent">
          <TriangleMark
            className="transition-transform duration-500 group-hover:rotate-[120deg]"
            size={18}
            variant="nested"
          />
        </span>
        <span>
          <span className="block font-display text-lg font-semibold leading-none tracking-tight">
            M Finance
          </span>
          <span className="mt-1 block text-[11px] uppercase tracking-[0.18em] text-text-muted">
            Coded by M
          </span>
        </span>
      </Link>

      <nav className="space-y-6">
        {navGroups.map((group, groupIndex) => (
          <div className="space-y-1" key={group.label ?? `group-${groupIndex}`}>
            {group.label ? (
              <p className="px-3 pb-1 text-[10px] font-semibold uppercase tracking-[0.2em] text-text-muted/70">
                {group.label}
              </p>
            ) : null}
            {group.items.map((item) => {
              const active = pathname === item.href;
              const Icon = item.icon;

              return (
                <Link
                  className={cn(
                    "focus-ring group relative flex min-h-11 items-center gap-3 rounded-md px-3 text-sm font-medium text-text-muted transition duration-200 hover:bg-background-elevated hover:text-text-secondary",
                    active && "border border-border-subtle bg-background-elevated text-text-primary shadow-lg shadow-black/10",
                  )}
                  href={item.href}
                  key={item.href}
                >
                  <TriangleMark
                    className={cn(
                      "shrink-0 transition-all duration-200",
                      active ? "text-accent" : "text-text-muted opacity-50 group-hover:opacity-80",
                    )}
                    size={9}
                    variant={active ? "solid" : "outline"}
                  />
                  <Icon className="shrink-0" size={18} aria-hidden="true" />
                  {item.label}
                </Link>
              );
            })}
          </div>
        ))}
      </nav>

      <div className="clip-notch brand-grid absolute bottom-5 left-4 right-4 border border-border-subtle bg-background-secondary p-4">
        <div className="flex items-center gap-2">
          <TriangleMark className="text-accent" size={12} variant="solid" />
          <p className="text-xs font-semibold uppercase tracking-[0.16em] text-text-muted">MVP 1</p>
        </div>
        <p className="mt-2 text-sm leading-5 text-text-secondary">
          Cockpit de contas, faturas e vencimentos mensais.
        </p>
      </div>
    </aside>
  );
}
