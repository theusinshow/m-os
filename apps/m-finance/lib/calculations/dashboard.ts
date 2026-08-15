import { classifyMonthHealth } from "@/lib/calculations/month-health";

export type DashboardItemStatus = "pending" | "paid" | "overdue";

export type DashboardBill = {
  id: string;
  name: string;
  amountCents: number;
  dueDate: string;
  status: DashboardItemStatus;
};

export type DashboardInvoice = DashboardBill & {
  cardType?: "personal" | "business";
};

export type DashboardIncome = {
  id: string;
  amountCents: number;
  received: boolean;
};

export function getDashboardSummary({
  incomes,
  bills,
  invoices,
}: {
  incomes: DashboardIncome[];
  bills: DashboardBill[];
  invoices: DashboardInvoice[];
}) {
  const totalIncomeCents = incomes.reduce((total, income) => total + income.amountCents, 0);
  const totalBillsCents = bills.reduce((total, bill) => total + bill.amountCents, 0);
  const totalInvoicesCents = invoices.reduce((total, invoice) => total + invoice.amountCents, 0);
  const allPayables = [...bills, ...invoices];
  const totalPaidCents = allPayables
    .filter((item) => item.status === "paid")
    .reduce((total, item) => total + item.amountCents, 0);
  const totalPendingCents = allPayables
    .filter((item) => item.status === "pending")
    .reduce((total, item) => total + item.amountCents, 0);
  const totalOverdueCents = allPayables
    .filter((item) => item.status === "overdue")
    .reduce((total, item) => total + item.amountCents, 0);
  const estimatedRemainingCents = totalIncomeCents - totalBillsCents - totalInvoicesCents;

  return {
    totalIncomeCents,
    totalBillsCents,
    totalInvoicesCents,
    totalPaidCents,
    totalPendingCents,
    totalOverdueCents,
    estimatedRemainingCents,
    monthHealth: classifyMonthHealth({
      estimatedRemainingCents,
      overdueCents: totalOverdueCents,
      dueSoonCount: allPayables.filter((item) => item.status !== "paid").length,
    }),
  };
}
