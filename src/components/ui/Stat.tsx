interface StatProps {
  label: string;
  value: string;
  hint?: string;
}

/**
 * Numero com rotulo, com numeros tabulares. Sem "numeros gigantes" soltos.
 *
 * `min-w-0`: valores de moeda vem do `Intl.NumberFormat`, que separa o simbolo
 * do numero com um espaco NAO-QUEBRAVEL (U+00A0). "R$ 12.480,83" e portanto um
 * token unico e indivisivel — sem isto, o item forca a trilha do grid a ficar
 * do tamanho do texto e o valor vaza por cima da borda do card. Quem usa o Stat
 * precisa garantir largura suficiente; o `min-w-0` evita que o vazamento
 * arraste o layout inteiro junto.
 */
export function Stat({ label, value, hint }: StatProps) {
  return (
    <div className="min-w-0">
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
