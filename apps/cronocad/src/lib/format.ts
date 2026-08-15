/**
 * Formatacao localizada (pt-BR) para tempo, dinheiro e datas.
 * Centralizada para facilitar internacionalizacao futura (secao 18).
 */

import type { Cents, Seconds } from "@/types/domain";

/**
 * Formata uma duracao em segundos como "2h 35min" (ou "35min", "45s").
 * Sempre exibe pelo menos os minutos quando ha horas.
 */
export function formatDuration(totalSeconds: Seconds): string {
  const safe = Math.max(0, Math.floor(totalSeconds));
  const hours = Math.floor(safe / 3600);
  const minutes = Math.floor((safe % 3600) / 60);
  const seconds = safe % 60;

  if (hours > 0) {
    return `${hours}h ${String(minutes).padStart(2, "0")}min`;
  }
  if (minutes > 0) {
    return `${minutes}min`;
  }
  return `${seconds}s`;
}

/**
 * Formata uma duracao como cronometro "HH:MM:SS" (numeros tabulares).
 * Usado no elemento principal do dashboard.
 */
export function formatClock(totalSeconds: Seconds): string {
  const safe = Math.max(0, Math.floor(totalSeconds));
  const hours = Math.floor(safe / 3600);
  const minutes = Math.floor((safe % 3600) / 60);
  const seconds = safe % 60;
  return [hours, minutes, seconds]
    .map((n) => String(n).padStart(2, "0"))
    .join(":");
}

/** Formata centavos como moeda BRL: 12345 -> "R$ 123,45". */
export function formatCurrency(cents: Cents, currency = "BRL"): string {
  return new Intl.NumberFormat("pt-BR", {
    style: "currency",
    currency,
  }).format(cents / 100);
}

/** Data ISO -> "DD/MM/YYYY". */
export function formatDate(iso: string): string {
  return new Intl.DateTimeFormat("pt-BR", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
  }).format(new Date(iso));
}

/** Data ISO -> "HH:MM" (24h). */
export function formatTime(iso: string): string {
  return new Intl.DateTimeFormat("pt-BR", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(new Date(iso));
}
