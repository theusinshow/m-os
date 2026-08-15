export function slugify(text: string): string {
  return text
    .toLowerCase()
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")  // remove diacritics (á→a, ê→e, ç→c, etc.)
    .replace(/[^a-z0-9\s-]/g, "")     // mantém só alfanumérico, espaços e hífens
    .trim()
    .replace(/\s+/g, "-")             // espaços → hífens
    .replace(/-{2,}/g, "-")           // colapsa múltiplos hífens
    .replace(/^-|-$/g, "");           // remove hífens nas bordas
}
