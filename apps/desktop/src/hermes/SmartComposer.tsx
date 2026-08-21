import { FormEvent, KeyboardEvent, useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { api } from "../api";
import { SmallIcon } from "../Icon";
import type { ContextInput, HermesStatus } from "../hermes";
import type { SearchItem } from "../types";
import { aplicarComando, comandosPara, type Comando } from "./comandos";
import { aplicarMencao, medirCampo, tokenDeMencao } from "./composer";
import { ContextChip } from "./ContextChip";
import { anexaveis, contextoDoItem, idDoItem, rotuloDoItem, TAG_DA_ENTIDADE } from "./entidades";

/**
 * O ponto de comando do Hermes.
 *
 * # Por que ele ganhou peso
 *
 * Antes era um campo de uma linha com um rótulo ao lado — indistinguível de uma
 * caixa de busca, e portanto lido como "mais um elemento da página". Ele é o
 * lugar de onde o M/OS é operado, e o desenho passou a dizer isso: moldura
 * própria, altura de duas linhas mesmo vazio, e o contexto anexado morando
 * DENTRO dele.
 *
 * # Por que o contexto mudou de lugar
 *
 * Os chips viviam numa régua no topo da tela, longe do campo. A pergunta que
 * eles respondem — "o que vai junto quando eu apertar enter?" — é sobre o que
 * está prestes a ser enviado, e ela se faz olhando para o campo. Agora eles
 * ficam na borda de cima do composer, onde o olho já está.
 *
 * # Os três seletores
 *
 * `@` para mencionar no meio da frase, `/` para atalho de digitação no começo,
 * `+` para procurar sem saber o nome. São três portas para a mesma lista, e
 * nenhuma delas é obrigatória: linguagem natural continua sendo a interface.
 */

type Seletor =
  | { modo: "nenhum" }
  | { modo: "mencao"; itens: SearchItem[]; indice: number }
  | { modo: "comando"; itens: Comando[]; indice: number }
  | { modo: "anexo"; busca: string; itens: SearchItem[]; indice: number };

const NENHUM: Seletor = { modo: "nenhum" };

export function SmartComposer({
  draft,
  setDraft,
  contexts,
  setContexts,
  running,
  online,
  status,
  onSubmit,
  onInterrupt,
  onEditarUltima,
  campo,
  offlinePanel,
}: {
  draft: string;
  setDraft: (next: string) => void;
  contexts: ContextInput[];
  setContexts: (next: ContextInput[]) => void;
  running: boolean;
  online: boolean;
  status: HermesStatus | null;
  onSubmit: () => void;
  onInterrupt: () => void;
  /** Seta para cima em campo vazio. Mora na página porque depende do histórico. */
  onEditarUltima: () => void;
  campo: React.RefObject<HTMLTextAreaElement | null>;
  offlinePanel?: React.ReactNode;
}) {
  const [seletor, setSeletor] = useState<Seletor>(NENHUM);
  const [rolando, setRolando] = useState(false);
  const buscaAnexo = useRef<HTMLInputElement>(null);
  const raiz = useRef<HTMLFormElement>(null);

  /**
   * A altura acompanha o texto.
   *
   * `useLayoutEffect` e não `useEffect`: medir depois da pintura faz o campo
   * saltar de altura à vista de quem digita.
   */
  useLayoutEffect(() => {
    const node = campo.current;
    if (!node) return;
    const estilo = window.getComputedStyle(node);
    const linha = Number.parseFloat(estilo.lineHeight) || 20;
    const moldura =
      Number.parseFloat(estilo.paddingTop) + Number.parseFloat(estilo.paddingBottom) || 0;
    node.style.height = "0px";
    const medida = medirCampo(node.scrollHeight, linha, moldura);
    node.style.height = `${medida.altura}px`;
    setRolando(medida.rolando);
  }, [draft, campo]);

  /** Menção: dois caracteres depois do `@`, em qualquer posição da frase. */
  useEffect(() => {
    if (seletor.modo === "anexo") return;
    const comandos = comandosPara(draft);
    if (comandos.length) {
      setSeletor((atual) => ({
        modo: "comando",
        itens: comandos,
        indice: atual.modo === "comando" ? Math.min(atual.indice, comandos.length - 1) : 0,
      }));
      return;
    }
    const token = tokenDeMencao(draft);
    if (!token) {
      setSeletor((atual) => (atual.modo === "nenhum" ? atual : NENHUM));
      return;
    }
    let cancelado = false;
    const tempo = window.setTimeout(() => {
      void api
        .search(token, false)
        .then((itens) => {
          if (cancelado) return;
          const lista = anexaveis(itens, contexts).slice(0, 6);
          setSeletor(lista.length ? { modo: "mencao", itens: lista, indice: 0 } : NENHUM);
        })
        .catch(() => setSeletor(NENHUM));
    }, 120);
    return () => { cancelado = true; window.clearTimeout(tempo); };
    // `contexts` de propósito fora: reanexar não deve refazer a busca em curso.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [draft, seletor.modo]);

  /** Busca do `+`. Separada da menção porque não depende do rascunho. */
  useEffect(() => {
    if (seletor.modo !== "anexo") return;
    const termo = seletor.busca.trim();
    if (termo.length < 2) return;
    let cancelado = false;
    const tempo = window.setTimeout(() => {
      void api
        .search(termo, false)
        .then((itens) => {
          if (cancelado) return;
          setSeletor((atual) =>
            atual.modo === "anexo" ? { ...atual, itens: anexaveis(itens, contexts).slice(0, 8), indice: 0 } : atual);
        })
        .catch(() => undefined);
    }, 120);
    return () => { cancelado = true; window.clearTimeout(tempo); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [seletor.modo, seletor.modo === "anexo" ? seletor.busca : ""]);

  /** Clique fora fecha o `+`. Sem isto o painel ficava aberto atrás da conversa. */
  useEffect(() => {
    if (seletor.modo !== "anexo") return;
    function fora(evento: MouseEvent) {
      if (!raiz.current?.contains(evento.target as Node)) setSeletor(NENHUM);
    }
    document.addEventListener("mousedown", fora);
    return () => document.removeEventListener("mousedown", fora);
  }, [seletor.modo]);

  const anexar = useCallback((item: SearchItem) => {
    const contexto = contextoDoItem(item);
    if (!contexts.some((atual) => atual.id === contexto.id)) setContexts([...contexts, contexto]);
  }, [contexts, setContexts]);

  function escolherMencao(item: SearchItem) {
    setDraft(aplicarMencao(draft, rotuloDoItem(item)));
    anexar(item);
    setSeletor(NENHUM);
    campo.current?.focus();
  }

  function escolherComando(comando: Comando) {
    setDraft(aplicarComando(comando));
    setSeletor(NENHUM);
    campo.current?.focus();
  }

  function escolherAnexo(item: SearchItem) {
    anexar(item);
    setSeletor(NENHUM);
    campo.current?.focus();
  }

  function abrirAnexos() {
    setSeletor({ modo: "anexo", busca: "", itens: [], indice: 0 });
    window.requestAnimationFrame(() => buscaAnexo.current?.focus());
  }

  /** Navegação comum aos três seletores. Devolve `true` quando consumiu a tecla. */
  function navegar(evento: KeyboardEvent, total: number, escolher: (indice: number) => void, indice: number) {
    if (evento.key === "ArrowDown" || evento.key === "ArrowUp") {
      evento.preventDefault();
      const passo = evento.key === "ArrowDown" ? 1 : -1;
      setSeletor((atual) =>
        atual.modo === "nenhum" ? atual : { ...atual, indice: (indice + passo + total) % total });
      return true;
    }
    if (evento.key === "Enter" && !evento.shiftKey) { evento.preventDefault(); escolher(indice); return true; }
    if (evento.key === "Tab") { evento.preventDefault(); escolher(indice); return true; }
    if (evento.key === "Escape") { evento.preventDefault(); setSeletor(NENHUM); return true; }
    return false;
  }

  function aoTeclar(evento: KeyboardEvent<HTMLTextAreaElement>) {
    if (seletor.modo === "mencao" && seletor.itens.length) {
      if (navegar(evento, seletor.itens.length, (i) => escolherMencao(seletor.itens[i]), seletor.indice)) return;
    }
    if (seletor.modo === "comando" && seletor.itens.length) {
      if (navegar(evento, seletor.itens.length, (i) => escolherComando(seletor.itens[i]), seletor.indice)) return;
    }
    // Campo vazio, seta para cima: editar a última pergunta.
    if (evento.key === "ArrowUp" && !draft) { evento.preventDefault(); onEditarUltima(); return; }
    // Backspace em campo vazio remove o último chip — o gesto que todo campo de
    // token tem, e cuja ausência obriga a mirar num `×` de 12px.
    if (evento.key === "Backspace" && !draft && contexts.length) {
      evento.preventDefault();
      setContexts(contexts.slice(0, -1));
      return;
    }
    if (evento.key === "Enter" && !evento.shiftKey) {
      evento.preventDefault();
      if (!running && online) onSubmit();
    }
  }

  function enviar(evento: FormEvent) {
    evento.preventDefault();
    if (running || !online) return;
    onSubmit();
  }

  const podeEnviar = Boolean(draft.trim()) && online && !running;

  return (
    <form className="hermes-composer" onSubmit={enviar} ref={raiz}>
      {seletor.modo === "mencao" ? (
        <div className="hermes-picker" role="listbox" aria-label="Contexto para anexar">
          {seletor.itens.map((item, indice) => (
            <button
              key={`${item.kind}-${idDoItem(item)}`}
              type="button"
              role="option"
              aria-selected={indice === seletor.indice}
              data-active={indice === seletor.indice || undefined}
              onMouseDown={(evento) => evento.preventDefault()}
              onClick={() => escolherMencao(item)}
            >
              <span className="hermes-picker-kind">{TAG_DA_ENTIDADE[item.kind as ContextInput["entity"]] ?? "ITEM"}</span>
              <span className="hermes-picker-name">{rotuloDoItem(item)}</span>
            </button>
          ))}
        </div>
      ) : null}

      {seletor.modo === "comando" ? (
        <div className="hermes-picker" role="listbox" aria-label="Comandos">
          {seletor.itens.map((comando, indice) => (
            <button
              key={comando.nome}
              type="button"
              role="option"
              aria-selected={indice === seletor.indice}
              data-active={indice === seletor.indice || undefined}
              onMouseDown={(evento) => evento.preventDefault()}
              onClick={() => escolherComando(comando)}
            >
              <span className="hermes-picker-kind">/{comando.nome}</span>
              <span className="hermes-picker-name">{comando.descricao}</span>
            </button>
          ))}
        </div>
      ) : null}

      {seletor.modo === "anexo" ? (
        <div className="hermes-picker hermes-picker-anexo" role="dialog" aria-label="Adicionar contexto">
          <input
            ref={buscaAnexo}
            value={seletor.busca}
            onChange={(evento) => setSeletor({ ...seletor, busca: evento.currentTarget.value })}
            onKeyDown={(evento) => {
              if (seletor.itens.length) {
                navegar(evento, seletor.itens.length, (i) => escolherAnexo(seletor.itens[i]), seletor.indice);
                return;
              }
              if (evento.key === "Escape") { setSeletor(NENHUM); campo.current?.focus(); }
            }}
            placeholder="Project, Task, Capture ou Resource"
            aria-label="Procurar no M/OS"
          />
          {seletor.itens.map((item, indice) => (
            <button
              key={`${item.kind}-${idDoItem(item)}`}
              type="button"
              data-active={indice === seletor.indice || undefined}
              onMouseDown={(evento) => evento.preventDefault()}
              onClick={() => escolherAnexo(item)}
            >
              <span className="hermes-picker-kind">{TAG_DA_ENTIDADE[item.kind as ContextInput["entity"]] ?? "ITEM"}</span>
              <span className="hermes-picker-name">{rotuloDoItem(item)}</span>
            </button>
          ))}
          {seletor.busca.trim().length >= 2 && !seletor.itens.length ? (
            <p className="hermes-quiet">Nada encontrado.</p>
          ) : null}
        </div>
      ) : null}

      <div className="hermes-trilho" data-running={running || undefined} data-offline={!online || undefined}>
        {contexts.length ? (
          <div className="hermes-trilho-contexto">
            <span className="micro-label">CONTEXTO</span>
            {contexts.map((contexto) => (
              <ContextChip
                key={contexto.id}
                compacto
                entity={contexto.entity}
                label={contexto.label}
                origin={contexto.origin}
                onRemove={() => setContexts(contexts.filter((entrada) => entrada.id !== contexto.id))}
              />
            ))}
          </div>
        ) : null}

        <textarea
          ref={campo}
          className="hermes-campo"
          value={draft}
          rows={2}
          data-rolando={rolando || undefined}
          onChange={(evento) => setDraft(evento.currentTarget.value)}
          onKeyDown={aoTeclar}
          placeholder={running ? "Hermes está escrevendo…" : "Pergunte ou mande o Hermes fazer alguma coisa"}
          aria-label="Perguntar ao Hermes"
          disabled={!online}
        />

        <div className="hermes-trilho-pe">
          <button
            type="button"
            className="hermes-mais"
            onClick={abrirAnexos}
            disabled={!online}
            aria-label="Adicionar contexto"
            aria-expanded={seletor.modo === "anexo"}
            title="Adicionar contexto do M/OS"
          >
            <SmallIcon name="plus" />
          </button>
          <span className="hermes-dica" aria-hidden="true">
            <b>@</b> contexto · <b>/</b> comandos
          </span>
          {/* O modo é a única promessa que o sistema faz sobre o que vai
              acontecer com seus dados. Hoje ela é garantida pela arquitetura:
              mos-hermes não compila com acesso ao banco. */}
          <span
            className="hermes-mode"
            title="O Hermes lê o M/OS e propõe ações, mas nada é executado sem a sua confirmação no cartão."
          >
            {status?.state === "online" && !status.sessionReady ? "ABRINDO SESSÃO" : "PROPÕE · VOCÊ CONFIRMA"}
          </span>
          {running ? (
            <button type="button" className="hermes-stop" onClick={onInterrupt}>
              Parar <kbd>Esc</kbd>
            </button>
          ) : (
            <button type="submit" className="hermes-send" disabled={!podeEnviar} aria-label="Enviar">
              <span aria-hidden="true">↑</span>
            </button>
          )}
        </div>
      </div>

      {offlinePanel}
    </form>
  );
}
