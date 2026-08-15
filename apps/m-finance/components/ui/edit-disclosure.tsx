import { ChevronDown, Pencil } from "lucide-react";
import { cn } from "@/lib/utils";

/**
 * Collapsible "edit" affordance that reads as an actual button instead of a
 * bare text link. Used for the inline edit + delete forms on list items.
 */
export function EditDisclosure({
  children,
  className,
  label = "Editar",
}: {
  children: React.ReactNode;
  className?: string;
  label?: string;
}) {
  return (
    <details className={cn("group/edit", className)}>
      <summary className="focus-ring flex min-h-10 w-fit list-none items-center gap-2 rounded-md border border-border-subtle bg-background-elevated px-3 text-sm font-semibold text-text-secondary transition duration-200 hover:border-border-default hover:text-text-primary [&::-webkit-details-marker]:hidden">
        <Pencil size={14} aria-hidden="true" />
        {label}
        <ChevronDown
          aria-hidden="true"
          className="transition-transform duration-200 group-open/edit:rotate-180"
          size={14}
        />
      </summary>
      <div className="mt-4">{children}</div>
    </details>
  );
}
