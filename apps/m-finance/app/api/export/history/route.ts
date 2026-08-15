import { centsToAmount, csvResponse, toCsv } from "@/lib/csv";
import { requireUser } from "@/lib/auth/guard";
import { getMonthlySnapshots } from "@/lib/history";
import { getAppUserBySupabaseId } from "@/lib/months";

const HEALTH_LABEL: Record<string, string> = {
  positive: "Positivo",
  fair: "Justo",
  tight: "Apertado",
  negative: "Negativo",
};

export async function GET() {
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  if (!appUser) {
    return new Response("Usuário interno não configurado.", { status: 400 });
  }

  const snapshots = await getMonthlySnapshots(appUser.id);

  const rows: (string | number | null)[][] = [
    [
      "Ano",
      "Mês",
      "Receita",
      "Contas",
      "Faturas",
      "Pago",
      "Pendente",
      "Vencido",
      "Sobra estimada",
      "Saúde",
    ],
  ];

  for (const snap of snapshots) {
    rows.push([
      snap.year,
      String(snap.month).padStart(2, "0"),
      centsToAmount(snap.totalIncomeCents),
      centsToAmount(snap.totalBillsCents),
      centsToAmount(snap.totalInvoicesCents),
      centsToAmount(snap.totalPaidCents),
      centsToAmount(snap.totalPendingCents),
      centsToAmount(snap.totalOverdueCents),
      centsToAmount(snap.estimatedRemainingCents),
      HEALTH_LABEL[snap.monthHealth] ?? snap.monthHealth,
    ]);
  }

  return csvResponse(toCsv(rows), "m-finance-historico.csv");
}
