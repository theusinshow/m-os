import { AtlasError } from "../errors";

const ALLOWED_PROTOCOLS = new Set(["http:", "https:"]);

export function validateUrl(url: string): void {
  if (!url || !url.trim()) {
    throw new AtlasError(
      "INVALID_URL",
      "A URL informada não é válida. Verifique e tente de novo.",
      "URL is empty"
    );
  }

  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    throw new AtlasError(
      "INVALID_URL",
      "A URL informada não é válida. Verifique e tente de novo.",
      `Failed to parse URL: "${url}"`
    );
  }

  if (!ALLOWED_PROTOCOLS.has(parsed.protocol)) {
    throw new AtlasError(
      "INVALID_URL",
      "A URL informada não é válida. Verifique e tente de novo.",
      `Protocol not allowed: "${parsed.protocol}". Only http: and https: are accepted.`
    );
  }

  if (!parsed.hostname) {
    throw new AtlasError(
      "INVALID_URL",
      "A URL informada não é válida. Verifique e tente de novo.",
      "URL has no hostname"
    );
  }
}
