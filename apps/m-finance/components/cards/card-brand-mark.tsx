import { TriangleMark } from "@/components/brand/triangle-mark";
import { getCardBrandColor } from "@/lib/card-brand";

/**
 * Small triangle chip tinted with the card's brand color (Mercado Pago blue,
 * Itaú orange, Nubank purple), falling back to a muted mark for unknown cards.
 */
export function CardBrandMark({ name, size = 14 }: { name: string; size?: number }) {
  const color = getCardBrandColor(name);

  return (
    <span
      className="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-border-subtle bg-background-elevated"
      style={color ? { color } : undefined}
    >
      <TriangleMark
        className={color ? undefined : "text-text-muted"}
        size={size}
        variant="solid"
      />
    </span>
  );
}
