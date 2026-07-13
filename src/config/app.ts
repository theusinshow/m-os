/**
 * Configuracao central da aplicacao.
 *
 * O nome do produto e outros valores globais ficam centralizados aqui para
 * permitir alteracao futura sem procurar textos espalhados pelo codigo
 * (requisito da secao 2 do documento do produto).
 */

export const APP = {
  /** Nome de exibicao do produto. Alterar aqui reflete em toda a UI. */
  name: "CronoCAD",
  /** Identificador tecnico (usado em bundle, paths, etc.). */
  slug: "cronocad",
  tagline: "Rastreador de horas para desenhistas e projetistas",
  version: "0.1.0",
} as const;

export const LOCALE = {
  language: "pt-BR",
  currency: "BRL",
  timeZoneNote: "Horarios sao calculados no backend a partir de timestamps.",
} as const;

/** Nome do arquivo do banco SQLite local (resolvido em app data dir). */
export const DATABASE_FILE = "cronocad.sqlite";
