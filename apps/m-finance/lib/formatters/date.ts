export function formatShortDate(date: string | Date) {
  return new Intl.DateTimeFormat("pt-BR", {
    day: "2-digit",
    month: "short",
  }).format(typeof date === "string" ? new Date(`${date}T12:00:00`) : date);
}

export function formatMonthLabel(date = new Date()) {
  const label = new Intl.DateTimeFormat("pt-BR", {
    month: "long",
    year: "numeric",
  }).format(date);

  return label.charAt(0).toUpperCase() + label.slice(1);
}
