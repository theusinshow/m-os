import { AppShell } from "@/components/app-shell/app-shell";
import { ensureAppUser } from "@/lib/auth/bootstrap";
import { requireUser } from "@/lib/auth/guard";
import { getActiveMonthForUser, getMonthSwitcherData } from "@/lib/active-month";
import { getNotifications } from "@/lib/notifications";

export default async function ProtectedLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  const user = await requireUser();
  const appUser = await ensureAppUser(user);
  const { options, activeValue } = appUser
    ? await getMonthSwitcherData(appUser.id)
    : { options: [], activeValue: "" };
  const activeMonth = appUser ? await getActiveMonthForUser(appUser.id) : null;
  const notifications = appUser
    ? await getNotifications(appUser.id, activeMonth)
    : { items: [], unreadCount: 0 };

  return (
    <AppShell
      activeMonthValue={activeValue}
      monthOptions={options}
      notifications={notifications}
      user={user}
    >
      {children}
    </AppShell>
  );
}
