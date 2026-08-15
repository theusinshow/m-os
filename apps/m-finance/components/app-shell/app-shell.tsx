import type { User } from "@supabase/supabase-js";
import { MobileNav } from "@/components/app-shell/mobile-nav";
import { Sidebar } from "@/components/app-shell/sidebar";
import { Topbar } from "@/components/app-shell/topbar";
import { TriangleMark } from "@/components/brand/triangle-mark";
import { ToastProvider } from "@/components/ui/toast";
import type { NotificationData } from "@/lib/notifications";

type MonthOption = { value: string; label: string; isCurrent: boolean };

export function AppShell({
  children,
  user,
  monthOptions,
  activeMonthValue,
  notifications,
}: {
  children: React.ReactNode;
  user: User;
  monthOptions: MonthOption[];
  activeMonthValue: string;
  notifications: NotificationData;
}) {
  return (
    <ToastProvider>
      <div className="relative min-h-screen bg-background-primary text-text-primary">
        <div aria-hidden="true" className="pointer-events-none fixed inset-0 -z-10 overflow-hidden">
          <TriangleMark
            className="float-y absolute -right-16 top-24 text-text-primary/[0.03]"
            size={340}
            variant="outline"
          />
          <TriangleMark
            className="absolute -left-10 bottom-10 rotate-180 text-accent/[0.04]"
            size={220}
            variant="outline"
          />
        </div>
        <Sidebar />
        <div className="min-h-screen lg:pl-72">
          <Topbar
          activeMonthValue={activeMonthValue}
          monthOptions={monthOptions}
          notifications={notifications}
          user={user}
        />
          <main className="px-4 pb-24 pt-4 md:px-6 lg:px-8 lg:pb-8">{children}</main>
        </div>
        <MobileNav />
      </div>
    </ToastProvider>
  );
}
