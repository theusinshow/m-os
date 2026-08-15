interface CheckboxProps {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  /** Nome acessivel quando o rotulo visivel esta vazio. */
  ariaLabel?: string;
}

/** Checkbox com rotulo, estilo consistente com os tokens. */
export function Checkbox({
  label,
  checked,
  onChange,
  disabled,
  ariaLabel,
}: CheckboxProps) {
  return (
    <label className="flex cursor-pointer items-center gap-2 text-sm text-text">
      <input
        type="checkbox"
        checked={checked}
        disabled={disabled}
        aria-label={label ? undefined : ariaLabel}
        onChange={(e) => onChange(e.target.checked)}
        className="h-4 w-4 rounded border-border accent-accent"
      />
      {label}
    </label>
  );
}
