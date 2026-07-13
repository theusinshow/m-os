interface StatProps {
  label: string;
  value: string;
  hint?: string;
}

/** Numero com rotulo, com numeros tabulares. Sem "numeros gigantes" soltos. */
export function Stat({ label, value, hint }: StatProps) {
  return (
    <div>
      <p className="text-2xs uppercase tracking-wide text-text-subtle">
        {label}
      </p>
      <p className="font-display tabular mt-1 text-2xl font-bold text-text">
        {value}
      </p>
      {hint && <p className="mt-0.5 text-xs text-text-muted">{hint}</p>}
    </div>
  );
}
