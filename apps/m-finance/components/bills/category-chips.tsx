"use client";

import { useState } from "react";
import { Check } from "lucide-react";
import { cn } from "@/lib/utils";

type Category = { id: string; name: string };

/**
 * Visual one-tap category picker. Keeps the selected id in a hidden input so it
 * rides along with the surrounding form (createBill reads `categoryId`).
 */
export function CategoryChips({
  categories,
  name = "categoryId",
  defaultValue = "",
}: {
  categories: Category[];
  name?: string;
  defaultValue?: string;
}) {
  const [selected, setSelected] = useState(defaultValue);

  if (categories.length === 0) {
    return null;
  }

  const options = [{ id: "", name: "Sem categoria" }, ...categories];

  return (
    <div className="flex flex-wrap gap-2">
      <input name={name} type="hidden" value={selected} />
      {options.map((option) => {
        const active = selected === option.id;
        return (
          <button
            aria-pressed={active}
            className={cn(
              "focus-ring inline-flex min-h-9 items-center gap-1.5 rounded-full border px-3 text-sm font-medium transition duration-150",
              active
                ? "border-accent-border bg-accent-soft text-accent"
                : "border-border-subtle bg-background-elevated text-text-muted hover:border-border-default hover:text-text-secondary",
            )}
            key={option.id || "none"}
            onClick={() => setSelected(option.id)}
            type="button"
          >
            {active ? <Check size={13} aria-hidden="true" /> : null}
            {option.name}
          </button>
        );
      })}
    </div>
  );
}
