type Brand = { match: RegExp; color: string };

// Brand accent per card, matched loosely on the card name.
const BRANDS: Brand[] = [
  { match: /mercado\s*pago/i, color: "#3483FA" },
  { match: /ita[uú]/i, color: "#FF6200" },
  { match: /nubank|\bnu\b/i, color: "#820AD1" },
];

export function getCardBrandColor(name: string): string | null {
  return BRANDS.find((brand) => brand.match.test(name))?.color ?? null;
}
