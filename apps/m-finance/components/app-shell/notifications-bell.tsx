"use client";

import { useState, useTransition } from "react";
import { useRouter } from "next/navigation";
import { Bell } from "lucide-react";
import { markAllNotificationsRead } from "@/app/actions/notifications";
import type { NotificationItem } from "@/lib/notifications";
import { cn } from "@/lib/utils";

const severityDot = {
  info: "bg-text-muted",
  warning: "bg-status-fair",
  danger: "bg-accent",
};

export function NotificationsBell({
  items,
  unreadCount,
}: {
  items: NotificationItem[];
  unreadCount: number;
}) {
  const [open, setOpen] = useState(false);
  const [pending, startTransition] = useTransition();
  const router = useRouter();

  function markAll() {
    if (pending || unreadCount === 0) return;
    startTransition(async () => {
      await markAllNotificationsRead();
      router.refresh();
    });
  }

  return (
    <div className="relative">
      <button
        aria-label={unreadCount > 0 ? `Notificações, ${unreadCount} não lidas` : "Notificações"}
        aria-expanded={open}
        aria-haspopup="dialog"
        className="focus-ring relative inline-flex h-11 w-11 items-center justify-center rounded-md border border-border-subtle bg-background-secondary text-text-secondary transition duration-150 hover:border-border-default hover:text-text-primary"
        onClick={() => setOpen((value) => !value)}
        type="button"
      >
        <Bell size={18} aria-hidden="true" />
        {unreadCount > 0 ? (
          <span className="num absolute -right-1 -top-1 flex h-5 min-w-5 items-center justify-center rounded-full bg-accent px-1 text-[10px] font-bold text-text-primary">
            {unreadCount > 9 ? "9+" : unreadCount}
          </span>
        ) : null}
      </button>

      {open ? (
        <>
          <button
            aria-hidden="true"
            className="fixed inset-0 z-30 cursor-default"
            onClick={() => setOpen(false)}
            tabIndex={-1}
            type="button"
          />
          <div
            className="absolute right-0 z-40 mt-2 w-80 max-w-[calc(100vw-2rem)] overflow-hidden rounded-lg border border-border-default bg-background-card shadow-xl shadow-black/30"
            role="dialog"
            aria-label="Notificações"
          >
            <div className="flex items-center justify-between gap-2 border-b border-border-subtle px-4 py-3">
              <p className="text-sm font-semibold text-text-primary">Notificações</p>
              {unreadCount > 0 ? (
                <button
                  className="focus-ring rounded-md px-2 py-1 text-xs font-semibold text-accent transition duration-150 hover:bg-accent-soft disabled:opacity-50"
                  disabled={pending}
                  onClick={markAll}
                  type="button"
                >
                  Marcar todas como lidas
                </button>
              ) : null}
            </div>

            <ul className="max-h-80 overflow-auto p-1.5">
              {items.length === 0 ? (
                <li className="px-3 py-6 text-center text-sm text-text-muted">
                  Nenhum alerta ativo para este mês.
                </li>
              ) : (
                items.map((item) => (
                  <li key={item.id}>
                    <div
                      className={cn(
                        "flex items-start gap-3 rounded-md px-3 py-2.5",
                        item.read ? "opacity-60" : "bg-background-elevated",
                      )}
                    >
                      <span
                        aria-hidden="true"
                        className={cn("mt-1.5 h-2 w-2 shrink-0 rounded-full", severityDot[item.severity])}
                      />
                      <div className="min-w-0">
                        <p className="text-sm font-semibold text-text-primary">{item.title}</p>
                        <p className="mt-0.5 text-sm text-text-muted">{item.message}</p>
                      </div>
                    </div>
                  </li>
                ))
              )}
            </ul>
          </div>
        </>
      ) : null}
    </div>
  );
}
