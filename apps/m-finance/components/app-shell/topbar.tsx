import type { User } from "@supabase/supabase-js";
import Link from "next/link";
import { History } from "lucide-react";
import { signOut } from "@/app/actions/auth";
import { MonthSwitcher } from "@/components/app-shell/month-switcher";
import { NotificationsBell } from "@/components/app-shell/notifications-bell";
import { SignOutButton } from "@/components/sign-out-button";
import type { NotificationData } from "@/lib/notifications";

type MonthOption = { value: string; label: string; isCurrent: boolean };

export function Topbar({
  user,
  monthOptions,
  activeMonthValue,
  notifications,
}: {
  user: User;
  monthOptions: MonthOption[];
  activeMonthValue: string;
  notifications: NotificationData;
}) {
  return (
    <header className="sticky top-0 z-20 border-b border-border-subtle bg-background-primary/90 px-4 py-3 backdrop-blur-xl md:px-6 lg:px-8">
      <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
        <MonthSwitcher activeValue={activeMonthValue} options={monthOptions} />

        <div className="flex items-center gap-2">
          <Link
            className="clip-notch sheen group focus-ring hidden min-h-11 items-center gap-2 border border-border-default bg-background-secondary px-3.5 text-sm font-semibold text-text-secondary transition duration-200 hover:border-border-strong hover:bg-background-hover hover:text-text-primary md:flex"
            href="/app/history"
          >
            <History size={16} aria-hidden="true" />
            Salvar histórico
          </Link>
          <NotificationsBell items={notifications.items} unreadCount={notifications.unreadCount} />
          <div className="flex min-h-11 items-center gap-3 rounded-md border border-border-subtle bg-background-secondary px-3 py-2">
            <div className="flex h-8 w-8 items-center justify-center rounded-md bg-background-elevated text-xs font-semibold text-text-secondary">
              {user.email?.charAt(0).toUpperCase() ?? "M"}
            </div>
            <div className="hidden sm:block">
              <p className="max-w-44 truncate text-sm font-medium text-text-primary">
                {user.user_metadata?.name ?? "Matheus Mendes"}
              </p>
              <p className="max-w-44 truncate text-xs text-text-muted">{user.email}</p>
            </div>
          </div>
          <form action={signOut}>
            <SignOutButton />
          </form>
        </div>
      </div>
    </header>
  );
}
