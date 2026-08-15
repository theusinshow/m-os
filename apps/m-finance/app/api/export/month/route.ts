import { centsToAmount, csvResponse, toCsv } from "@/lib/csv";
import { requireUser } from "@/lib/auth/guard";
import { getActiveMonthForUser } from "@/lib/active-month";
import { getBillsByMonth } from "@/lib/bills";
import { getInvoicesByMonth } from "@/lib/cards";
import { getIncomesByMonth } from "@/lib/incomes";
import { getAppUserBySupabaseId } from "@/lib/months";

const STATUS_LABEL: Record<string, string> = {
  pending: "Pendente",
  paid: "Pago",
  overdue: "Vencido",
};

export async function GET() {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  if (!appUser) {
    return new Response("Usuário interno não configurado.", { status: 400 });
  }

  const month = await getActiveMonthForUser(appUser.id);
  if (!month) {
    return new Response("Nenhum mês para exportar.", { status: 404 });
  }

  const [incomes, bills, invoices] = await Promise.all([
    getIncomesByMonth(month.id),
    getBillsByMonth(month.id),
    getInvoicesByMonth(month.id),
  ]);

  const rows: (string | number | null)[][] = [
    ["Tipo", "Nome", "Categoria", "Valor", "Data", "Status"],
  ];

  for (const income of incomes) {
    rows.push([
      "Receita",
      income.name,
      income.incomeType,
      centsToAmount(income.amountCents),
      income.expectedDate ?? "",
      income.received ? "Recebido" : "Previsto",
    ]);
  }

  for (const bill of bills) {
    rows.push([
      "Conta",
      bill.name,
      bill.categoryName ?? "",
      centsToAmount(bill.amountCents),
      bill.dueDate,
      STATUS_LABEL[bill.status] ?? bill.status,
    ]);
  }

  for (const invoice of invoices) {
    rows.push([
      "Fatura",
      invoice.name,
      invoice.cardType === "business" ? "PJ" : "Pessoal",
      centsToAmount(invoice.amountCents),
      invoice.dueDate,
      STATUS_LABEL[invoice.status] ?? invoice.status,
    ]);
  }

  const filename = `m-finance-${month.year}-${String(month.month).padStart(2, "0")}.csv`;
  return csvResponse(toCsv(rows), filename);
}
