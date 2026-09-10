import { useEffect, useState } from "react";

import { api, type Referencia } from "../api";
import { dominioDe } from "./links";

/**
 * A estante do bolso.
 *
 * # Por que um campo só, e no topo
 *
 * O gesto que esta tela existe para servir é curto e acontece de pé: você viu
 * uma biblioteca de CSS, copiou o endereço, e quer que ele esteja no PC quando
 * sentar. Qualquer campo a mais entre colar e guardar é atrito cobrado
 * justamente no momento em que a pessoa está com o dedo no vidro e a atenção em
 * outro lugar — e o link não guardado é o link perdido.
 *
 * Título e nota ficam para o desktop, com teclado de verdade. O `mos-core` já
 * decidiu o que fazer com o título em branco: ele vira a própria URL.
 *
 * # Não é share sheet, e não dá para ser
 *
 * O iOS não implementa Web Share Target para PWA — não existe "compartilhar do
 * Safari direto para o M/OS". É colar, e isso não tem contorno deste lado.
 */
export function Biblioteca({ aoVoltar }: { aoVoltar: () => void }) {
  const [itens, setItens] = useState<Referencia[] | null>(null);
  const [url, setUrl] = useState("");
  const [salvando, setSalvando] = useState(false);
  const [erro, setErro] = useState<string | null>(null);

  useEffect(() => {
    api
      .biblioteca()
      .then(setItens)
      .catch(() => setItens([]));
  }, []);

  async function guardar() {
    const endereco = url.trim();
    if (!endereco || salvando) return;
    setSalvando(true);
    setErro(null);
    try {
      const novo = await api.guardarNaBiblioteca(endereco);
      // Na frente da lista, e não recarregando tudo: a estante já está na tela,
      // e uma ida ao servidor aqui trocaria o item aparecendo na hora por meio
      // segundo de nada.
      setItens((atual) => [novo, ...(atual ?? [])]);
      setUrl("");
    } catch (causa) {
      setErro(causa instanceof Error ? causa.message : "Não consegui guardar.");
    } finally {
      setSalvando(false);
    }
  }

  async function arquivar(id: string) {
    setItens((atual) => (atual ?? []).filter((item) => item.id !== id));
    try {
      await api.arquivarDaBiblioteca(id);
    } catch {
      // Voltou a lista do servidor: sumir da tela e continuar no banco é a
      // única divergência que o usuário não tem como perceber sozinho.
      api.biblioteca().then(setItens).catch(() => undefined);
    }
  }

  return (
    <div className="biblioteca">
      <button className="voltar" type="button" onClick={aoVoltar}>
        ← Mais
      </button>

      <form
        className="guardar"
        onSubmit={(evento) => {
          evento.preventDefault();
          void guardar();
        }}
      >
        <input
          // `url` abre o teclado com `.com` e `/`, e desliga a correção
          // automática — que num endereço só atrapalha.
          type="url"
          inputMode="url"
          autoCapitalize="off"
          autoCorrect="off"
          spellCheck={false}
          placeholder="Cole o endereço"
          value={url}
          onChange={(evento) => setUrl(evento.target.value)}
          aria-label="Endereço para guardar"
        />
        <button className="botao" type="submit" disabled={!url.trim() || salvando}>
          {salvando ? "Guardando" : "Guardar"}
        </button>
      </form>
      {/* `.recado` e o padrao da casa para dizer o que aconteceu: fixo, acima
          da barra, e some sozinho de vista quando a proxima acao acontece. */}
      {erro ? (
        <p className="recado" data-estado="erro" aria-live="polite">
          {erro}
        </p>
      ) : null}

      {itens === null ? null : itens.length === 0 ? (
        <p className="vazio">
          Nada guardado ainda. Cole um endereço aí em cima — ele aparece no
          Library do PC na próxima sincronização.
        </p>
      ) : (
        <ul className="lista">
          {itens.map((item) => (
            <li className="item" key={item.id}>
              <a className="linha-destino" href={item.url} target="_blank" rel="noreferrer">
                <div className="item-corpo">
                  <p>{rotulo(item)}</p>
                  <small>{dominioDe(item.url) ?? item.kind}</small>
                </div>
              </a>
              <button
                className="arquivar"
                type="button"
                onClick={() => void arquivar(item.id)}
                aria-label={`Arquivar ${rotulo(item)}`}
              >
                Arquivar
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

/**
 * O que a linha mostra como nome.
 *
 * O servidor nunca devolve título vazio — ele cai na URL. Então a pergunta não
 * é "tem título?", e sim "o título É a url?": nesse caso o domínio diz a mesma
 * coisa em muito menos largura, e sobra espaço para o que foi digitado de
 * verdade quando alguém digitou.
 */
export function rotulo(item: Pick<Referencia, "title" | "url">): string {
  if (item.title && item.title !== item.url) return item.title;
  return dominioDe(item.url) ?? item.url;
}
