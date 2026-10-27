import type { Workspace } from "./types";

/**
 * O Workspace de uma Task ou Capture (ADR-093), editavel.
 *
 * So o PROPRIO: o que a Task herda do Project vem escrito embaixo, em
 * `herdados`, para a pessoa entender por que ela aparece num Workspace que o
 * campo nao mostra. Arquivado nao entra na lista; um item que ainda aponta para
 * um arquivado mostra o nome dele, para a escolha nao sumir em silencio.
 */
export function CampoDeWorkspace({ valor, workspaces, herdados = [], desabilitado = false, mudar }: {
  valor: string | null | undefined;
  workspaces: Workspace[];
  herdados?: string[];
  desabilitado?: boolean;
  mudar: (workspaceId: string | null) => void;
}) {
  const ativos = workspaces.filter((workspace) => workspace.lifecycleState === "active");
  const fora = valor && !ativos.some((workspace) => workspace.id === valor)
    ? workspaces.find((workspace) => workspace.id === valor) ?? null
    : null;
  return <>
    <select value={valor ?? ""} aria-label="Workspace" disabled={desabilitado} onChange={(evento) => mudar(evento.currentTarget.value || null)}>
      <option value="">Sem workspace</option>
      {fora ? <option value={fora.id}>{fora.name} (arquivado)</option> : null}
      {ativos.map((workspace) => <option key={workspace.id} value={workspace.id}>{workspace.name}</option>)}
    </select>
    {herdados.length ? <small className="campo-workspace-herdado">Também em {herdados.join(", ")}, pelo Project.</small> : null}
  </>;
}
