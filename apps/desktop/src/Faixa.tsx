import { useCallback, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { Button } from "./Button";
import { Ring, RingLabel } from "./Ring";
import type { AnelDaFaixa, Faixa } from "./types";

/**
 * A faixa de uso, colada na borda direita da tela.
 *
 * Um anel por fonte de IA — hoje só o Claude Code — com quanto da janela de
 * cinco horas já foi. O clique no anel abre o painel ao lado.
 *
 * # A regra que este componente existe para não quebrar
 *
 * O `Ring.tsx` diz por escrito que "um anel bonito preenchido com número
 * inventado é pior que a ausência". O número que falta aqui é o TETO: o Claude
 * Code não grava cota nem hora de reset, e portanto não existe "73% do plano".
 *
 * O que existe é o pico observado — a maior janela de cinco horas que já
 * aconteceu nesta máquina —, e é contra ele que o anel mede. É a mesma régua
 * que o `WeekRings` usa contra o melhor dia da semana.
 *
 * E há um caso em que nem essa régua serve: quando o banco conhece UMA janela
 * só, o pico É a sessão corrente, e a proporção daria 100% por falta de
 * comparação. Aí o anel mostra o trilho e o valor absoluto, que é o que
 * `Ring.tsx` já manda fazer no zero.
 *
 * # Esconder
 *
 * Dois gestos, e eles não são o mesmo. A lingueta de 12px na borda **recolhe**:
 * o cartão some da tela e sobra só ela, um clique de distância — mas a janela
 * continua onde estava, porque movê-la mata a entrada dela (ver `usage.rs`), e
 * os 84px que o cartão ocupava seguem engolindo clique do desktop. O item
 * "Faixa de uso" no tray **desliga**: a janela some inteira, e aí não sobra
 * pixel nenhum no caminho. A escolha fica gravada nos dois casos.
 *
 * O laço de leitura continua rodando nos dois casos. Parar de contar enquanto a
 * faixa está escondida perderia o consumo do período, e ao trazê-la de volta o
 * pico — que é o único número que ela tem — estaria errado.
 *
 * # Clique, e não hover
 *
 * O desenho de origem abria no hover. Ele não sobreviveu à tela: crescer a
 * janela exige `resizable: true`, e uma janela sem decoração e redimensionável
 * **para de receber evento de mouse** no Windows. Ver `usage.rs`.
 *
 * O que sobrou é melhor para uma tira que fica sempre por cima: ela não abre
 * sozinha quando o ponteiro passa a caminho de outra coisa.
 */

/** Milésimos de token-equivalente-de-input viram algo que cabe num anel. */
export function curto(peso: number) {
  const tokens = peso / 1_000;
  if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(1).replace(".", ",")}M`;
  if (tokens >= 1_000) return `${Math.round(tokens / 1_000)}k`;
  return String(Math.round(tokens));
}

/** "reseta em 51 min", "reseta em 2h13". O prazo é exato: sai do início da janela. */
export function faltaPara(quando: string, agora: number) {
  const restante = new Date(quando).getTime() - agora;
  if (restante <= 0) return "reseta agora";
  const minutos = Math.round(restante / 60_000);
  if (minutos < 60) return `reseta em ${minutos} min`;
  const horas = Math.floor(minutos / 60);
  return `reseta em ${horas}h${String(minutos % 60).padStart(2, "0")}`;
}

/** Há régua para calcular proporção? */
export function temRegua(anel: AnelDaFaixa, calibrando: boolean) {
  // `janelasConhecidas > 1` é o que impede o 100% do primeiro dia: com uma
  // janela só, o pico e a sessão são a mesma coisa.
  return !calibrando && anel.pico > 0 && anel.janelasConhecidas > 1;
}

export function proporcao(valor: number, teto: number) {
  if (teto <= 0) return 0;
  return Math.max(0, Math.min(1, valor / teto));
}

/** O dado da faixa, e o relógio que faz a contagem regressiva andar. */
function useFaixa() {
  const [faixa, setFaixa] = useState<Faixa | null>(null);
  const [agora, setAgora] = useState(() => Date.now());

  useEffect(() => {
    // Uma faixa que não sabe o número não inventa um: ela fica como estava, e a
    // passada seguinte do laço do Rust corrige.
    void api.faixaDeUso().then(setFaixa).catch(() => undefined);
    const parar = listen<Faixa>("usage", (evento) => setFaixa(evento.payload));
    return () => { void parar.then((remover) => remover()); };
  }, []);

  /* Um minuto de granularidade porque o rótulo é em minutos — acordar mais
     rápido redesenharia o mesmo texto. */
  useEffect(() => {
    const relogio = window.setInterval(() => setAgora(Date.now()), 60_000);
    return () => window.clearInterval(relogio);
  }, []);

  return {
    aneis: faixa?.aneis ?? [],
    calibrando: faixa?.calibrando ?? false,
    recolhida: faixa?.recolhida ?? false,
    agora,
  };
}

function Barra({ rotulo, valor, teto, nota, regua, contra }: {
  rotulo: string;
  valor: number;
  teto: number;
  nota: string;
  regua: boolean;
  /** Contra o que a proporção é medida. Dito por extenso na barra: "% do pico"
   *  na sessão e "% do maior dia" no dia medem réguas DIFERENTES, e uma etiqueta
   *  só para as duas faria a segunda parecer a primeira. */
  contra: string;
}) {
  const fracao = proporcao(valor, teto);
  return (
    <div className="faixa-barra">
      <div className="faixa-barra-head">
        <span className="micro-label">{rotulo}</span>
        <span className="faixa-barra-nota">{nota}</span>
      </div>
      <div className="faixa-trilho">
        {/* Sem régua não se pinta nada: uma barra cheia contra um teto que não
            existe é a mentira que o trilho vazio evita. */}
        {regua ? <div className="faixa-carga" style={{ width: `${fracao * 100}%` }} /> : null}
      </div>
      <span className="faixa-barra-valor">
        {regua ? `${Math.round(fracao * 100)}% ${contra}` : `${curto(valor)} tokens`}
      </span>
    </div>
  );
}

/** A tira de anéis, na janela `faixa`. */
export function FaixaDeUso() {
  const { aneis, calibrando, recolhida } = useFaixa();
  const [aberto, setAberto] = useState(false);

  /* Quem decide se o painel está aberto é a visibilidade da janela dele, do
     lado do Rust — o painel se fecha pelo próprio botão, e um booleano guardado
     aqui ficaria invertido no clique seguinte. */
  const alternar = useCallback(() => {
    void api.alternarPainelDaFaixa().then(setAberto).catch(() => undefined);
  }, []);

  const recolher = useCallback((proxima: boolean) => {
    void api.recolherFaixa(proxima).catch(() => undefined);
  }, []);

  // Sem fonte a faixa não desenha nada. Ela não aparece vazia esperando um dado
  // que nunca virá — a janela some do caminho de quem está trabalhando.
  if (!aneis.length) return null;

  return (
    <div className="faixa-shell" data-recolhida={recolhida || undefined}>
      {/* Primeiro no DOM, e à direita na tela: o shell é `row-reverse`, para a
          lingueta ficar colada na borda mesmo quando o cartão some. */}
      <button
        type="button"
        className="faixa-lingueta"
        aria-label={recolhida ? "Mostrar a faixa de uso" : "Recolher a faixa de uso"}
        title={recolhida ? "Mostrar a faixa de uso" : "Recolher a faixa de uso"}
        onClick={() => recolher(!recolhida)}
      >
        <span className="faixa-lingueta-marca" aria-hidden="true" />
      </button>

      <div className="faixa-tira">
        {aneis.map((anel) => {
          const regua = temRegua(anel, calibrando);
          const fracao = proporcao(anel.peso, anel.pico);
          const valor = regua ? `${Math.round(fracao * 100)}%` : curto(anel.peso);
          return (
            <button
              type="button"
              className="faixa-anel"
              key={anel.nome}
              aria-expanded={aberto}
              aria-label={`${anel.nome}: ${regua ? `${valor} do pico de consumo` : `${valor} tokens`}`}
              onClick={alternar}
            >
              <Ring size={56} segments={regua ? [{ value: fracao }] : []}>
                <RingLabel value={valor} />
              </Ring>
              <span className="micro-label">{regua ? "DO PICO" : calibrando ? "LENDO" : "SEM RÉGUA"}</span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

/** O painel, na janela `faixa-painel`. Ele lê o mesmo evento que a tira. */
export function PainelDaFaixa() {
  const { aneis, calibrando, agora } = useFaixa();

  const conteudo = useMemo(() => aneis.map((anel) => {
    const regua = temRegua(anel, calibrando);
    return (
      <div className="faixa-painel-fonte" key={anel.nome}>
        <span className="faixa-painel-nome">{anel.nome}</span>
        <Barra
          rotulo="SESSÃO"
          valor={anel.peso}
          teto={anel.pico}
          regua={regua}
          contra="do pico"
          nota={anel.resetaEm ? faltaPara(anel.resetaEm, agora) : "sem janela aberta"}
        />
        <Barra
          rotulo="HOJE"
          valor={anel.pesoHoje}
          teto={anel.picoDia}
          regua={regua}
          contra="do maior dia"
          nota={`${anel.requisicoesHoje} ${anel.requisicoesHoje === 1 ? "request" : "requests"}`}
        />
        {/* A régua é dita em voz alta. Um "73%" sem denominador seria
            exatamente o número que este desenho recusa. */}
        <p className="faixa-regua">
          {calibrando
            ? "Lendo o histórico pela primeira vez. Sem régua ainda."
            : regua
              ? "Proporção contra o seu maior consumo já observado — não contra o limite do plano, que o Claude Code não grava."
              : "Ainda não há histórico suficiente para comparar. O número é absoluto."}
        </p>
      </div>
    );
  }), [aneis, calibrando, agora]);

  if (!aneis.length) return null;

  return (
    <div className="faixa-painel" role="group" aria-label="Consumo de IA">
      {conteudo}
      <div className="button-line">
        <Button variant="secondary" onClick={() => void api.abrirApp().catch(() => undefined)}>
          Abrir o M/OS
        </Button>
        <Button variant="ghost" onClick={() => void api.fecharPainelDaFaixa().catch(() => undefined)}>
          Fechar
        </Button>
      </div>
    </div>
  );
}
