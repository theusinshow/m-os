type PayableStatus = "pending" | "paid" | "overdue";

function normalizeDate(date: string | Date) {
  return typeof date === "string" ? new Date(`${date}T12:00:00`) : date;
}

function isPastDate(date: string | Date, today = new Date()) {
  const due = normalizeDate(date);
  const normalizedDue = new Date(due.getFullYear(), due.getMonth(), due.getDate());
  const normalizedToday = new Date(today.getFullYear(), today.getMonth(), today.getDate());

  return normalizedDue.getTime() < normalizedToday.getTime();
}

export function derivePayableStatus(status: PayableStatus, dueDate: string | Date): PayableStatus {
  if (status === "paid") {
    return "paid";
  }

  if (isPastDate(dueDate)) {
    return "overdue";
  }

  return status;
}
