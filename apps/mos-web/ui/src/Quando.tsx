import { useEffect, useRef, useState } from "react";
import { disponiveis, padrao, paraCampoLocal, porExtenso } from "./instantes";

/**
 * A folha que pergunta *quando*, e nada além disso.
 *
 * # Por que uma folha por baixo, e não um diálogo no meio
 *
 * Porque ela é acionada pelo polegar, numa lista, e volta para a mesma lista. Um
 * diálogo centralizado tira os controles do alcance da mão e cobre a linha que a
 * pessoa acabou de tocar — e cobrir o item é justamente perder a referência do
 * que se está agendando.
 *
 * # O que ela NÃO pergunta
 *
 * Prioridade, canal, repetição, nota. Quem toca no sino de uma Task quer ser
 * lembrado dela, não configurar um lembrete: o título já existe — é a Task. O
 * `UX-PRINCIPLES.md` §88 mede a experiência por decisões desnecessárias, e todas
 * essas seriam.
 *
 * # As duas que ela passou a perguntar, e por quê
 *
 * **"Algum dia"** é um chip ao lado dos outros. Sem ele, a única forma de
 * guardar "comprar cabo HDMI" seria inventar uma hora — e uma hora inventada é
 * uma notificação que se aprende a ignorar.
 *
 * **"Não me deixe esquecer"** é um interruptor, e não um chip: ele não escolhe
 * *quando*, escolhe *como*. Fica no primeiro nível porque é a escolha que muda o
 * comportamento do lembrete, e escondê-la atrás de "mais opções" faria a
 * capacidade central do sistema depender de alguém descobrir um link.
 */
export function Quando({
  titulo,
  descricao,
  ocupado,
  aoEscolher,
  aoFechar,
}: {
  /** O que está sendo agendado, mostrado para a escolha não ficar às cegas. */
  titulo: string;
  /** A linha acima do título. "LEMBRAR DESTA TASK", por exemplo. */
  descricao: string;
  ocupado: boolean;
  /** `null` em `quando` é "algum dia": existe, e não interrompe. */
  aoEscolher: (quando: Date | null, persistente: boolean) => void;
  aoFechar: () => void;
}) {
  const [quando, setQuando] = useState<Date>(() => padrao());
  const [aberto, setAberto] = useState(false);
  const [semData, setSemData] = useState(false);
  const [persistente, setPersistente] = useState(false);
  const folha = useRef<HTMLDivElement>(null);
  // A base do cálculo é congelada na abertura. Sem isto, cada `render` moveria
  // "15 min" um pouco para a frente, e o chip aceso pararia de bater com o
  // instante escolhido no meio da escolha.
  const base = useRef(new Date());

  useEffect(() => {
    folha.current?.focus();
    function tecla(evento: KeyboardEvent) {
      if (evento.key === "Escape") aoFechar();
    }
    document.addEventListener("keydown", tecla);
    return () => document.removeEventListener("keydown", tecla);
  }, [aoFechar]);

  const escolhas = disponiveis(base.current);

  return (
    <div className="folha-fundo" onPointerDown={(evento) => {
      // Só o fundo fecha. Um toque que começou dentro da folha e terminou fora
      // — arrastar para rolar os chips — não é uma desistência.
      if (evento.target === evento.currentTarget) aoFechar();
    }}>
      <div
        className="folha"
        role="dialog"
        aria-modal="true"
        aria-label={descricao}
        ref={folha}
        tabIndex={-1}
      >
        <p className="rotulo">{descricao}</p>
        <p className="folha-alvo">{titulo}</p>

        <div className="chips" role="group" aria-label="Quando">
          {escolhas.map((escolha) => {
            const instante = escolha.resolver(base.current);
            // Um minuto de tolerância: o chip aceso é o que a pessoa tocou, e
            // comparar milissegundos apagaria todos eles.
            const aceso =
              !aberto && Math.abs(instante.getTime() - quando.getTime()) < 60_000;
            return (
              <button
                key={escolha.rotulo}
                type="button"
                className="chip"
                aria-pressed={aceso}
                onClick={() => {
                  setAberto(false);
                  setQuando(escolha.resolver(base.current));
                }}
              >
                {escolha.rotulo}
              </button>
            );
          })}
          <button
            type="button"
            className="chip"
            aria-pressed={aberto}
            onClick={() => {
              setSemData(false);
              setAberto(true);
            }}
          >
            Escolher
          </button>
          <button
            type="button"
            className="chip"
            aria-pressed={semData}
            onClick={() => {
              setAberto(false);
              setSemData((atual) => !atual);
            }}
          >
            Algum dia
          </button>
        </div>

        {aberto && !semData ? (
          <input
            aria-label="Data e hora"
            className="campo-hora"
            min={paraCampoLocal(new Date())}
            onChange={(evento) => {
              const lido = new Date(evento.currentTarget.value);
              if (!Number.isNaN(lido.getTime())) setQuando(lido);
            }}
            type="datetime-local"
            value={paraCampoLocal(quando)}
          />
        ) : null}

        {/* O instante resolvido, sempre visível. Ver `instantes.ts`. */}
        <p className="folha-instante" aria-live="polite">
          {semData ? "Sem data — não vai interromper você" : porExtenso(quando)}
        </p>

        {/* Interruptor de verdade, e não um chip que finge ser um: ligado ou
            desligado precisa chegar ao leitor de tela como o que é, e
            `aria-pressed` num botão diz outra coisa. */}
        <label className="folha-interruptor">
          <input
            checked={persistente}
            onChange={(evento) => setPersistente(evento.currentTarget.checked)}
            type="checkbox"
          />
          <span>
            Não me deixe esquecer
            <small>continua cobrando até você resolver</small>
          </span>
        </label>

        <div className="folha-acoes">
          <button
            type="button"
            className="botao"
            data-variante="quieto"
            disabled={ocupado}
            onClick={aoFechar}
          >
            Cancelar
          </button>
          <button
            type="button"
            className="botao"
            disabled={ocupado}
            onClick={() => aoEscolher(semData ? null : quando, persistente)}
          >
            {ocupado ? "Criando" : "Criar lembrete"}
          </button>
        </div>
      </div>
    </div>
  );
}
