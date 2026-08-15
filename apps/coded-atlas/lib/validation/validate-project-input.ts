import type { ProjectInput } from "../types";
import { AtlasError } from "../errors";
import { validateUrl } from "./validate-url";

const SLUG_PATTERN = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

export function validateProjectInput(input: ProjectInput): void {
  if (!input.name?.trim()) {
    throw new AtlasError(
      "UNKNOWN",
      "Nome do projeto é obrigatório.",
      "Missing required field: name"
    );
  }

  if (!input.slug?.trim()) {
    throw new AtlasError(
      "UNKNOWN",
      "Slug do projeto é obrigatório.",
      "Missing required field: slug"
    );
  }

  if (!SLUG_PATTERN.test(input.slug)) {
    throw new AtlasError(
      "UNKNOWN",
      "Slug inválido. Use apenas letras minúsculas, números e hífens (ex: meu-projeto).",
      `Slug does not match pattern ^[a-z0-9]+(?:-[a-z0-9]+)*$: "${input.slug}"`
    );
  }

  if (!input.category?.trim()) {
    throw new AtlasError(
      "UNKNOWN",
      "Categoria é obrigatória.",
      "Missing required field: category"
    );
  }

  validateUrl(input.url);
}
