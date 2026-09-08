import { useRef, useState } from "react";
import type { ItemDeChecklist } from "../api";

/**
 * O checklist no bolso.
 *
 * # Por que ele não é o componente do desktop
 *
 * A lógica é a mesma — e ela vive no servidor, que é onde `mos-web` e desktop
 * de fato compartilham. O que muda aqui é só a MANIFESTAÇÃO, e a diferença não
 * é de tamanho:
 *
 * - **o alvo tem 44px**, e não 30. Um checkbox de 14px cercado por 8px de
 *   respiro funciona com um mouse e falha com um polegar num ônibus;
 * - **não há hover.** O item de remover mora atrás de um toque longo no
 *   desktop; aqui ele é um botão sempre visível, porque não existe estado
 *   intermediário para revelá-lo;
 * - **não há arrastar.** Reordenar por toque exige `touch-action` e um
 *   long-press que compete com a rolagem da página. Quem reordena está no PC;
 *   quem está na rua está executando.
 *
 * A escrita é otimista pelo mesmo motivo do desktop, e aqui ela pesa mais: no
 * 4G a resposta demora, e um checkbox que só risca depois do servidor parece
 * um app quebrado.
 */
export function Checklist({
  itens,
  ocupado,
  aoMarcar,
  aoCriar,
  aoApagar,
}: {
  itens: ItemDeChecklist[];
  ocupado: boolean;
  aoMarcar: (id: string, feito: boolean) => void;
  aoCriar: (texto: string) => void;
  aoApagar: (id: string) => void;
}) {
  const [rascunho, setRascunho] = useState("");
  const [otimista, setOtimista] = useState<Record<string, boolean>>({});
  const campo = useRef<HTMLInputElement>(null);

  const feito = (item: ItemDeChecklist) => otimista[item.id] ?? item.completedAt !== null;
  const concluidos = itens.filter(feito).length;

  function marcar(item: ItemDeChecklist) {
    const proximo = !feito(item);
    setOtimista((atual) => ({ ...atual, [item.id]: proximo }));
    aoMarcar(item.id, proximo);
  }

  function criar() {
    const texto = rascunho.trim();
    if (!texto) return;
    setRascunho("");
    aoCriar(texto);
    /* O foco fica: escrever cinco passos é cinco linhas e cinco Enters, e não
       cinco viagens até o campo. */
    campo.current?.focus();
  }

  return (
    <section className="detalhe-bloco">
      <h3>
        CHECKLIST
        {itens.length ? (
          <span className="checklist-contagem">
            {concluidos}/{itens.length}
          </span>
        ) : null}
      </h3>

      {itens.length ? (
        <ul className="checklist">
          {itens.map((item) => (
            <li key={item.id} data-feito={feito(item) || undefined}>
              {/* O alvo é a LINHA inteira, e não só a caixinha: 44px de altura
                  por toda a largura é o que faz marcar um passo funcionar em
                  movimento. */}
              <button
                type="button"
                className="checklist-linha"
                aria-pressed={feito(item)}
                disabled={ocupado}
                onClick={() => marcar(item)}
              >
                <span className="checklist-caixa" aria-hidden="true" />
                <span className="checklist-texto">{item.label}</span>
              </button>
              <button
                type="button"
                className="checklist-remover"
                aria-label={`Remover ${item.label}`}
                disabled={ocupado}
                onClick={() => aoApagar(item.id)}
              >
                ×
              </button>
            </li>
          ))}
        </ul>
      ) : null}

      <div className="checklist-novo">
        <input
          ref={campo}
          value={rascunho}
          placeholder={itens.length ? "próximo passo" : "adicionar primeiro item"}
          aria-label="Novo item de checklist"
          enterKeyHint="done"
          onChange={(evento) => setRascunho(evento.currentTarget.value)}
          onKeyDown={(evento) => {
            if (evento.key === "Enter") {
              evento.preventDefault();
              criar();
            }
          }}
          onPaste={(evento) => {
            /* Colar várias linhas cria vários itens — quem divide é o domínio,
               no servidor. A tela só decide não interromper com uma pergunta
               cuja resposta a própria ação já deu. */
            const colado = evento.clipboardData.getData("text");
            if (colado.split("\n").filter((linha) => linha.trim()).length < 2) return;
            evento.preventDefault();
            setRascunho("");
            aoCriar(colado);
          }}
        />
        {rascunho.trim() ? (
          <button type="button" className="botao" data-variante="quieto" onClick={criar}>
            Adicionar
          </button>
        ) : null}
      </div>
    </section>
  );
}

/** `3/6` e uma barra, para a linha da lista. */
export function Progresso({ feitos, total }: { feitos: number; total: number }) {
  if (!total) return null;
  return (
    <span className="progresso" title={`${feitos} de ${total}`}>
      <span className="progresso-numero">
        {feitos}/{total}
      </span>
      <span className="progresso-barra" aria-hidden="true">
        <span style={{ transform: `scaleX(${feitos / total})` }} />
      </span>
    </span>
  );
}
