import Link from "next/link";
import { notFound } from "next/navigation";
import { ArrowLeft } from "lucide-react";
import { StatementImport } from "@/components/cards/statement-import";
import { PageHeading } from "@/components/page-heading";
import { requireUser } from "@/lib/auth/guard";
import { getCardById } from "@/lib/card-expenses";
import { getAppUserBySupabaseId } from "@/lib/months";

export default async function CardStatementImportPage({
  params,
}: {
  params: Promise<{ cardId: string }>;
}) {
  const { cardId } = await params;
  const user = await requireUser();
  const appUser = await getAppUserBySupabaseId(user.id);
  if (!appUser) notFound();

  const card = await getCardById(appUser.id, cardId);
  if (!card) notFound();

  return (
    <div className="space-y-6">
      <PageHeading eyebrow="Cartão" title="Importar fatura">
        <Link
          className="focus-ring inline-flex min-h-11 items-center gap-2 rounded-md border border-border-subtle px-4 text-sm font-semibold text-text-secondary transition duration-200 hover:border-border-default hover:bg-background-hover hover:text-text-primary"
          href={`/app/cards/${card.id}`}
        >
          <ArrowLeft size={16} aria-hidden="true" />
          {card.name}
        </Link>
      </PageHeading>
      <StatementImport cardId={card.id} cardName={card.name} />
    </div>
  );
}
