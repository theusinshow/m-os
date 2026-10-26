/**
 * Transforma as compras revisadas em linhas para gravar, sem duplicar o que já
 * está no banco.
 *
 * Uma parcela "4/10" vira as parcelas 4 a 10, uma por mês a partir do mês da
 * fatura: a previsão dos meses seguintes nasce certa, e a fatura de dezembro
 * já sabe que a parcela 5 vem. Quando a fatura seguinte for importada, a
 * parcela 5 que ela traz é reconhecida como a que já estava lá.
 */

export type ReviewedItem = {
  externalId: string;
  description: string;
  amountCents: number;
  purchaseDate: string | null;
  installmentNumber: number | null;
  installmentTotal: number | null;
  categoryId: string | null;
  isSubscription: boolean;
};

export type ExistingExpense = {
  externalId: string | null;
  description: string;
  amountCents: number;
  installmentNumber: number | null;
  installmentTotal: number | null;
};

export type PlannedRow = {
  /** 0 = mês da fatura; parcelas futuras somam um por mês. */
  monthOffset: number;
  /** Índice do item de origem: linhas do mesmo item formam um parcelamento. */
  itemIndex: number;
  externalId: string;
  description: string;
  amountCents: number;
  purchaseDate: string | null;
  installmentNumber: number | null;
  installmentTotal: number | null;
  categoryId: string | null;
  isSubscription: boolean;
};

function fold(text: string) {
  return text
    .trim()
    .toLocaleLowerCase("pt-BR")
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .replace(/\s+/g, " ");
}

/** A mesma compra, já gravada neste mês — por identidade, ou pela parcela. */
export function isAlreadyImported(item: ReviewedItem, existing: ExistingExpense[]) {
  return existing.some((row) => {
    if (row.externalId && row.externalId === item.externalId) return true;
    return (
      item.installmentNumber !== null &&
      row.installmentNumber === item.installmentNumber &&
      row.installmentTotal === item.installmentTotal &&
      row.amountCents === item.amountCents &&
      fold(row.description) === fold(item.description)
    );
  });
}

export function planImport(items: ReviewedItem[], existingInStartMonth: ExistingExpense[]) {
  const rows: PlannedRow[] = [];
  const skipped: ReviewedItem[] = [];

  items.forEach((item, itemIndex) => {
    if (isAlreadyImported(item, existingInStartMonth)) {
      skipped.push(item);
      return;
    }

    const number = item.installmentNumber;
    const total = item.installmentTotal;
    const count = number !== null && total !== null ? total - number + 1 : 1;

    for (let offset = 0; offset < count; offset += 1) {
      rows.push({
        monthOffset: offset,
        itemIndex,
        // `external_id` é único por usuário: as parcelas futuras ganham sufixo.
        externalId: offset === 0 ? item.externalId : `${item.externalId}#${(number ?? 0) + offset}`,
        description: item.description,
        amountCents: item.amountCents,
        purchaseDate: item.purchaseDate,
        installmentNumber: number === null ? null : number + offset,
        installmentTotal: total,
        categoryId: item.categoryId,
        isSubscription: item.isSubscription,
      });
    }
  });

  return { rows, skipped };
}
