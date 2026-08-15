"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  CalendarDays,
  CreditCard,
  LayoutDashboard,
  MoreHorizontal,
  ReceiptText,
} from "lucide-react";
import { TriangleMark } from "@/components/brand/triangle-mark";
import { cn } from "@/lib/utils";

// Five fixed tabs for the daily-glance surfaces; everything else lives under
// "Mais" so the bar never scrolls or overflows working memory.
const items = [
  { href: "/app/dashboard", label: "Início", icon: LayoutDashboard },
  { href: "/app/bills", label: "Despesas", icon: ReceiptText },
  { href: "/app/cards", label: "Cartões", icon: CreditCard },
  { href: "/app/calendar", label: "Agenda", icon: CalendarDays },
  { href: "/app/more", label: "Mais", icon: MoreHorizontal },
] as const;

export function MobileNav() {
  const pathname = usePathname();

  return (
    <nav className="fixed inset-x-0 bottom-0 z-40 grid grid-cols-5 border-t border-border-subtle bg-background-primary/95 px-1 pb-[calc(0.5rem+env(safe-area-inset-bottom))] pt-2 backdrop-blur-xl lg:hidden">
      {items.map((item) => {
        const Icon = item.icon;
        const active =
          pathname === item.href ||
          (item.href === "/app/more" && pathname.startsWith("/app/more"));

        return (
          <Link
            className={cn(
              "focus-ring relative flex min-h-14 flex-col items-center justify-center gap-1 rounded-md px-1 text-[11px] font-medium text-text-muted transition duration-200",
              active && "bg-background-elevated text-text-primary shadow-lg shadow-black/10",
            )}
            href={item.href}
            key={item.href}
          >
            {active ? (
              <TriangleMark
                className="absolute top-1 text-accent"
                size={7}
                variant="solid"
              />
            ) : null}
            <Icon size={18} aria-hidden="true" />
            {item.label}
          </Link>
        );
      })}
    </nav>
  );
}
