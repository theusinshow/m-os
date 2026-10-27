/**
 * O resultado de um serviço de domínio financeiro.
 *
 * Serviço não lança por regra de negócio: "conta já paga" é uma resposta, não
 * uma exceção. Cada canal traduz o `code` para a sua língua — o WhatsApp numa
 * frase, a web num `FormState`, a Action API num JSON — e a regra continua uma
 * só.
 */
export type DomainErrorCode =
  | "db_unavailable"
  | "not_found"
  | "already_paid"
  | "stale_preview"
  | "invalid"
  | "write_failed";

export type DomainResult<T> =
  | { ok: true; value: T }
  | { ok: false; code: DomainErrorCode; message: string };

export function ok<T>(value: T): DomainResult<T> {
  return { ok: true, value };
}

export function fail<T = never>(code: DomainErrorCode, message: string): DomainResult<T> {
  return { ok: false, code, message };
}

export type MonthRecord = { id: string; month: number; year: number };
