/**
 * Categorias/tipos de projeto pré-configurados — alimentam o dropdown do
 * formulário (menos digitação) e os filtros da biblioteca. "Outro" permite
 * texto livre quando nenhum tipo encaixa.
 */
export const PROJECT_CATEGORIES = [
  "Site Institucional",
  "Landing Page",
  "One Page",
  "E-commerce",
  "Portfólio",
  "Blog / Conteúdo",
  "Aplicação Web",
  "Dashboard / SaaS",
  "Hotsite / Campanha",
  "Outro",
] as const;

export type ProjectCategory = (typeof PROJECT_CATEGORIES)[number];
