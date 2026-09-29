import { Badge } from "@/components/ui/badge";

/** O selo de "isto é palpite": fatura ou NF que ainda não foi lançada. */
export function EstimateBadge({ label = "Estimada" }: { label?: string }) {
  return (
    <Badge
      className="border-dashed border-border-strong bg-transparent text-text-secondary"
      label={label}
    />
  );
}
