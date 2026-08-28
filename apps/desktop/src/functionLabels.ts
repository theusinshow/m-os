/**
 * Como cada Function se chama na tela.
 *
 * Mora fora do `App.tsx` porque DOIS lugares precisam: a busca, que mostra o
 * risco na linha do resultado, e o painel FUNCTIONS do Settings. Com os rotulos
 * no `App.tsx`, a pagina de Settings extraida teria de importar do `App.tsx`
 * que a importa — um ciclo. Um modulo terceiro e a saida, e e a mesma razao
 * pela qual o `Surface.tsx` existe.
 */
import type { FunctionDefinition } from "./types";

export const functionCategoryLabels: Record<FunctionDefinition["category"], string> = { capture: "CAPTURE", daily: "DIA", work: "WORK", time: "TEMPO", attention: "ATENÇÃO", memory: "MEMORY", meeting: "REUNIÕES", app: "APP", data: "DATA", system: "SYSTEM" };
export const functionRiskLabels: Record<FunctionDefinition["risk"], string> = { low: "baixo", medium: "medio", high: "alto" };
export const functionConfirmationLabels: Record<FunctionDefinition["confirmation"], string> = { none: "sem confirmacao", explicit: "confirmacao explicita" };
