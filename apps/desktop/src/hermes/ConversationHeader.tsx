import { forwardRef, useEffect, useRef, useState } from "react";
import { SmallIcon } from "../Icon";
import type { Conversation, HermesStatus } from "../hermes";

/**
 * O cabeçalho da conversa.
 *
 * # O que ele carrega, e por quê
 *
 * Três coisas, nesta ordem de leitura: onde estou (o botão de conversas, quando
 * a lista está fechada), o que estou lendo (o título, clicável para renomear) e
 * se o Hermes está lá (o estado).
 *
 * O contexto anexado NÃO mora mais aqui — ele desceu para dentro do composer.
 * A pergunta que os chips respondem é "o que vai junto quando eu enviar?", e
 * ela se faz olhando para o campo, não para o topo da tela. A régua antiga
 * ficava a quase mil pixels da ação que descrevia.
 *
 * # O estado à direita
 *
 * Palavra, não bolinha colorida. "ONLINE" sobrevive a daltonismo, a tema claro
 * e a captura de tela em preto e branco; um ponto verde não sobrevive a nenhum
 * dos três.
 *
 * # O menu
 *
 * Duas entradas, e nenhuma inventada: arquivar e excluir são as duas coisas que
 * o backend já sabe fazer com uma conversa e que nenhuma tela chamava —
 * `conversation_set_archived` existia sem porta. Renomear não entra na lista
 * porque já tem o gesto direto: clicar no título.
 */
export const ConversationHeader = forwardRef<HTMLButtonElement, {
  conversation: Conversation | null;
  status: HermesStatus | null;
  railAberto: boolean;
  renomeando: boolean;
  setRenomeando: (next: boolean) => void;
  onRenomear: (titulo: string) => void;
  onAbrirRail: () => void;
  onArquivar: () => void;
  onExcluir: () => void;
}>(function ConversationHeader(
  { conversation, status, railAberto, renomeando, setRenomeando, onRenomear, onAbrirRail, onArquivar, onExcluir },
  toggleRef,
) {
  const [menuAberto, setMenuAberto] = useState(false);
  const menu = useRef<HTMLDivElement>(null);

  /** Clique fora e Esc fecham. Um menu que só fecha pelo próprio botão é uma
   *  armadilha para quem abriu sem querer. */
  useEffect(() => {
    if (!menuAberto) return;
    function fora(evento: MouseEvent) {
      if (!menu.current?.contains(evento.target as Node)) setMenuAberto(false);
    }
    function tecla(evento: globalThis.KeyboardEvent) {
      if (evento.key === "Escape") setMenuAberto(false);
    }
    document.addEventListener("mousedown", fora);
    document.addEventListener("keydown", tecla);
    return () => {
      document.removeEventListener("mousedown", fora);
      document.removeEventListener("keydown", tecla);
    };
  }, [menuAberto]);

  const estado = status?.state === "online"
    ? (status.sessionReady ? "ONLINE" : "ABRINDO SESSÃO")
    : status?.state === "connecting" ? "CONECTANDO" : "OFFLINE";

  return (
    <header className="hermes-header">
      {!railAberto ? (
        <button ref={toggleRef} type="button" className="hermes-rail-toggle" onClick={onAbrirRail} title="Conversas · Ctrl+/">
          Conversas
        </button>
      ) : null}

      <div className="hermes-header-nome">
        <span className="micro-label">HERMES</span>
        {renomeando ? (
          <form
            onSubmit={(evento) => {
              evento.preventDefault();
              onRenomear(new FormData(evento.currentTarget).get("title") as string);
            }}
          >
            <input name="title" defaultValue={conversation?.title ?? ""} aria-label="Título da conversa" autoFocus />
          </form>
        ) : (
          // O nome acessível diz a AÇÃO e contém o texto visível. Só com o
          // título, o leitor de tela anunciava o nome da conversa e nada
          // indicava que clicar renomeia; só com "Renomear", o nome deixaria de
          // conter o rótulo visível (WCAG 2.5.3).
          <button
            type="button"
            className="hermes-title"
            onClick={() => setRenomeando(true)}
            aria-label={`Renomear conversa: ${conversation?.title || "sem título"}`}
            title="Renomear conversa"
          >
            {conversation?.title || "sem título"}
          </button>
        )}
      </div>

      <span className="hermes-estado micro-label" data-state={status?.state}>{estado}</span>

      <div className="hermes-menu" ref={menu}>
        <button
          type="button"
          className="hermes-menu-abrir"
          onClick={() => setMenuAberto((atual) => !atual)}
          aria-expanded={menuAberto}
          aria-haspopup="menu"
          aria-label="Ações da conversa"
          title="Ações da conversa"
        >
          <SmallIcon name="more" />
        </button>
        {menuAberto ? (
          <div className="hermes-menu-lista" role="menu">
            <button type="button" role="menuitem" onClick={() => { setMenuAberto(false); setRenomeando(true); }}>
              Renomear
            </button>
            <button type="button" role="menuitem" onClick={() => { setMenuAberto(false); onArquivar(); }}>
              Arquivar
            </button>
            <button type="button" role="menuitem" data-perigo onClick={() => { setMenuAberto(false); onExcluir(); }}>
              Excluir
            </button>
          </div>
        ) : null}
      </div>
    </header>
  );
});
