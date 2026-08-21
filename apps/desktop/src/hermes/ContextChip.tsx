import type { ContextInput } from "../hermes";
import { TAG_DA_ENTIDADE } from "./entidades";

/**
 * Um pedaço do M/OS que saiu junto com a pergunta.
 *
 * Borda sólida é manual, tracejada é automático — sem cor extra, sem ícone, sem
 * legenda. A distinção importa porque só uma das duas o usuário pode remover: o
 * contexto automático é recalculado a cada envio contra o estado de agora, e um
 * `×` nele prometeria um controle que não existe.
 *
 * O `title` carrega o que efetivamente saiu, em campos e bytes. É a resposta
 * literal para "o que o Hermes está vendo?", e ela não ocupa pixel nenhum até
 * alguém perguntar.
 */
export function ContextChip({ entity, label, origin, onRemove, detail, compacto }: {
  entity: ContextInput["entity"];
  label: string;
  origin: ContextInput["origin"];
  onRemove?: () => void;
  detail?: string;
  /** Dentro do composer o chip é menor: ele divide a linha com o campo. */
  compacto?: boolean;
}) {
  return (
    <span className="hermes-chip" data-origin={origin} data-compacto={compacto || undefined} title={detail}>
      <span className="hermes-chip-kind">{TAG_DA_ENTIDADE[entity]}</span>
      <span className="hermes-chip-label">{label}</span>
      {onRemove ? (
        <button type="button" aria-label={`Remover ${label} do contexto`} onClick={onRemove}>✕</button>
      ) : null}
    </span>
  );
}
