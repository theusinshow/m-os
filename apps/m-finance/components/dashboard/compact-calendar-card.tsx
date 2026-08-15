import { DashboardCard } from "@/components/dashboard/dashboard-card";

type Event = {
  id: string;
  type: "bill" | "invoice";
  dueDate: string;
  status: "pending" | "paid" | "overdue";
};

function getDayFromDate(date: string) {
  return new Date(`${date}T12:00:00`).getDate();
}

export function CompactCalendarCard({ events }: { events: Event[] }) {
  const today = new Date();
  const daysInMonth = new Date(today.getFullYear(), today.getMonth() + 1, 0).getDate();
  const firstWeekday = new Date(today.getFullYear(), today.getMonth(), 1).getDay();
  const firstOffset = firstWeekday === 0 ? 6 : firstWeekday - 1;

  return (
    <DashboardCard title="Calendário resumido">
      <div className="grid grid-cols-7 gap-2 text-center text-xs text-text-muted">
        {["S", "T", "Q", "Q", "S", "S", "D"].map((day, index) => (
          <span className="pb-1 font-semibold text-text-muted" key={`weekday-${index}`}>
            {day}
          </span>
        ))}
        {Array.from({ length: firstOffset }, (_, index) => (
          <span key={`empty-${index}`} />
        ))}
        {Array.from({ length: daysInMonth }, (_, index) => {
          const day = index + 1;
          const dayEvents = events.filter((event) => getDayFromDate(event.dueDate) === day);
          const hasOverdue = dayEvents.some((event) => event.status === "overdue");
          const hasPending = dayEvents.some((event) => event.status === "pending");

          return (
            <div
              className="min-h-12 rounded-md border border-border-subtle bg-background-elevated p-2"
              key={day}
            >
              <span className="block text-text-secondary">{day}</span>
              {dayEvents.length > 0 ? (
                <span
                  className={
                    hasOverdue
                      ? "mx-auto mt-2 block h-1.5 w-1.5 rounded-full bg-accent"
                      : hasPending
                        ? "mx-auto mt-2 block h-1.5 w-1.5 rounded-full bg-status-fair"
                        : "mx-auto mt-2 block h-1.5 w-1.5 rounded-full bg-status-positive"
                  }
                />
              ) : null}
            </div>
          );
        })}
      </div>
    </DashboardCard>
  );
}
