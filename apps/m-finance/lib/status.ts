type PayableStatus = "pending" | "paid" | "overdue";

function isPastDate(date: string, today = new Date()) {
  const due = new Date(`${date}T12:00:00`);
  const normalizedDue = new Date(due.getFullYear(), due.getMonth(), due.getDate());
  const normalizedToday = new Date(today.getFullYear(), today.getMonth(), today.getDate());

  return normalizedDue.getTime() < normalizedToday.getTime();
}

export function derivePayableStatus(status: PayableStatus, dueDate: string): PayableStatus {
  if (status === "paid") {
    return "paid";
  }

  if (isPastDate(dueDate)) {
    return "overdue";
  }

  return status;
}
