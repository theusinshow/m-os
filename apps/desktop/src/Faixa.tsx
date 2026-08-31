import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { Button } from "./Button";
import { MarcaDeIA } from "./MarcaDeIA";
import { Ring } from "./Ring";
import type { AnelDaFaixa, Faixa, JanelaDaFaixa } from "./types";

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
 * o cartão some da tela e sobra só ela, um clique de distância. A janela
 * continua onde estava, porque movê-la mata a entrada dela (ver `usage.rs`) —
 * mas os 84px que o cartão ocupava deixaram de engolir clique do desktop: o
 * `vigiar_o_cursor` só deixa a janela receber clique onde ela pinta (ADR-061).
 * O item "Faixa de uso" no tray **desliga**: a janela some inteira. A escolha
 * fica gravada nos dois casos.
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

/**
 * "reseta em 51 min", "reseta em 2h13", "reseta em 6d15h".
 *
 * O degrau de DIAS existe por causa da janela de sete dias, que chegou com a
 * ADR-062. Sem ele o rótulo dizia "reseta em 159h50" — aritmeticamente certo e
 * ilegível: ninguém converte 159 horas em "quinta-feira" de cabeça.
 *
 * Os minutos somem junto com o degrau, e é de propósito: num prazo de seis dias
 * eles são precisão que ninguém usa e dois dígitos que mudam a cada minuto.
 */
export function faltaPara(quando: string, agora: number) {
  const restante = new Date(quando).getTime() - agora;
  if (restante <= 0) return "reseta agora";
  const minutos = Math.round(restante / 60_000);
  if (minutos < 60) return `reseta em ${minutos} min`;
  const horas = Math.floor(minutos / 60);
  if (horas < 24) return `reseta em ${horas}h${String(minutos % 60).padStart(2, "0")}`;
  return `reseta em ${Math.floor(horas / 24)}d${horas % 24}h`;
}

/** Há régua de PICO para calcular proporção? */
export function temRegua(anel: AnelDaFaixa, calibrando: boolean) {
  // `janelasConhecidas > 1` é o que impede o 100% do primeiro dia: com uma
  // janela só, o pico e a sessão são a mesma coisa.
  return !calibrando && anel.pico > 0 && anel.janelasConhecidas > 1;
}

/**
 * Contra o que este anel está medindo.
 *
 * São três estados, e a ordem entre eles é a decisão inteira da ADR-062:
 *
 * 1. **`cota`** — o teto de verdade, dito pelo servidor da Anthropic. Quando
 *    ele responde, é ele que manda: um "23% da sessão" com denominador real
 *    vale mais que qualquer proporção contra o histórico desta máquina;
 * 2. **`pico`** — a régua da ADR-059, contra o maior consumo já observado aqui.
 *    Ela não sumiu: é o que responde sem credencial, com o token vencido, sem
 *    rede, ou quando a Anthropic mudar o formato da resposta;
 * 3. **`nenhuma`** — nem uma nem outra, e aí o anel mostra o trilho e o número
 *    absoluto. É o que o `Ring.tsx` manda fazer, e é melhor que um anel bonito
 *    preenchido com número inventado.
 */
export type Regua =
  | { tipo: "cota"; janela: JanelaDaFaixa }
  | { tipo: "pico"; fracao: number }
  | { tipo: "nenhuma" };

export function regua(anel: AnelDaFaixa, calibrando: boolean): Regua {
  if (anel.cotaSessao) return { tipo: "cota", janela: anel.cotaSessao };
  if (temRegua(anel, calibrando)) {
    return { tipo: "pico", fracao: proporcao(anel.peso, anel.pico) };
  }
  return { tipo: "nenhuma" };
}

/**
 * O número que vai dentro do anel.
 *
 * O `~` é o que marca um valor que não conseguiu renovar. Ele não é decoração:
 * um número de quatro minutos atrás continua útil numa janela de cinco horas, e
 * apagá-lo trocaria informação velha por nenhuma — mas mostrá-lo como se fosse
 * de agora seria a mentira que este desenho recusa.
 */
export function rotuloDaRegua(r: Regua, anel: AnelDaFaixa): string {
  if (r.tipo === "cota") {
    return `${r.janela.obsoleta ? "~" : ""}${r.janela.percentual}%`;
  }
  if (r.tipo === "pico") return `${Math.round(r.fracao * 100)}%`;
  return curto(anel.peso);
}

/** O que a régua se chama. Vai no rótulo acessível e no painel, não na tira. */
export function nomeDaRegua(r: Regua, calibrando: boolean): string {
  if (r.tipo === "cota") return "DA SESSÃO";
  if (r.tipo === "pico") return "DO PICO";
  return calibrando ? "LENDO" : "SEM RÉGUA";
}

/**
 * O semáforo do anel.
 *
 * # Por que a cor entra aqui, e só aqui
 *
 * O design system diz que "cor no M/OS significa atenção", e é por isso que a
 * faixa nasceu toda em sódio: um anel colorido a mais seria uma segunda cor de
 * sinal por acidente. Relendo a regra ao contrário: **um anel em 95% É
 * atenção**, e negar cor a ele é justamente esconder o único momento em que a
 * faixa tem algo urgente a dizer.
 *
 * O que a regra continua proibindo — e continua valendo — é cor sem
 * significado. Aqui cada degrau significa uma coisa que muda o que fazer:
 *
 * - **calmo** (< 50%) — a janela aguenta o que você planejou;
 * - **atenção** (50–80%) — dá para terminar, não dá para recomeçar;
 * - **limite** (≥ 80%) — o resto da janela é contado.
 *
 * Os cortes são os do `agent-notch`, e não inventados aqui: 50 e 80.
 *
 * Sem régua não há degrau — sem denominador, "alto" não quer dizer nada — e o
 * anel volta ao sódio, que é a cor de "isto é uma medida, não um alarme".
 */
export function degrau(r: Regua): "calmo" | "atencao" | "limite" | undefined {
  const percentual =
    r.tipo === "cota" ? r.janela.percentual : r.tipo === "pico" ? r.fracao * 100 : undefined;
  if (percentual === undefined) return undefined;
  if (percentual >= 80) return "limite";
  if (percentual >= 50) return "atencao";
  return "calmo";
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
    demonstracao: faixa?.demonstracao ?? false,
    agora,
  };
}

/** O degrau de uma barra, a partir da fração que ela pinta. */
function degrauDaFracao(fracao: number): "calmo" | "atencao" | "limite" {
  if (fracao >= 0.8) return "limite";
  if (fracao >= 0.5) return "atencao";
  return "calmo";
}

function Barra({ rotulo, valor, teto, nota, regua, exato }: {
  rotulo: string;
  valor: number;
  teto: number;
  nota: string;
  regua: boolean;
  /** Sobrepõe o texto do valor. Existe por causa do percentual acima de 100 — a
   *  barra satura em 1 e o texto não pode saturar junto — e do `~` do valor que
   *  não conseguiu renovar. */
  exato?: string;
}) {
  const fracao = proporcao(valor, teto);
  return (
    <div className="faixa-barra">
      <div className="faixa-barra-head">
        <span className="micro-label">{rotulo}</span>
        <span className="faixa-barra-nota">{nota}</span>
      </div>
      {/* O mesmo semáforo do anel, pelo mesmo motivo: a barra e o anel medem a
          MESMA coisa, e duas cores diferentes para o mesmo número fariam a
          pessoa procurar a diferença que não existe. */}
      <div className="faixa-trilho" data-degrau={regua ? degrauDaFracao(fracao) : undefined}>
        {/* Sem régua não se pinta nada: uma barra cheia contra um teto que não
            existe é a mentira que o trilho vazio evita. */}
        {regua ? <div className="faixa-carga" style={{ width: `${fracao * 100}%` }} /> : null}
      </div>
      {/* Só o símbolo, e igual nas três barras.

          A versão anterior escrevia a régua no valor — "27% da sessão", "15% do
          maior dia" — para que duas réguas diferentes não parecessem a mesma.
          A preocupação continua certa e a resposta mudou de lugar: quem diz a
          régua agora é o parágrafo embaixo, uma vez, em vez de três frases
          repetindo o que o próprio rótulo da barra já diz. */}
      <span className="faixa-barra-valor">
        {regua ? (exato ?? `${Math.round(fracao * 100)}%`) : `${curto(valor)} tokens`}
      </span>
    </div>
  );
}

/**
 * Mede o que a tira PINTOU e conta ao Rust.
 *
 * A janela da tira é alta o bastante para três anéis e **nunca muda de
 * tamanho** — redimensioná-la mata a entrada dela no Windows (ADR-059). Com um
 * anel só, dois terços dela são pixel transparente, e pixel transparente
 * engoliria o clique do desktop se o Rust não soubesse onde ela para.
 *
 * `useLayoutEffect` e não `useEffect`: a medida sai antes de o quadro aparecer,
 * senão existe um instante em que a tira está desenhada reivindicando área que
 * ela não pinta.
 *
 * O `ResizeObserver` cobre o resto: um anel que chega, a lingueta que recolhe,
 * uma fonte que muda de altura com o tema.
 */
function useMedirZona(deps: unknown[]) {
  const conteudo = useRef<HTMLDivElement | null>(null);

  useLayoutEffect(() => {
    const medir = () => {
      const alvo = conteudo.current;
      if (!alvo) return;
      const caixa = alvo.getBoundingClientRect();
      if (caixa.width <= 0 || caixa.height <= 0) return;
      void api
        .medirFaixa(caixa.left, caixa.top, caixa.width, caixa.height)
        .catch(() => undefined);
    };

    medir();
    const observador = new ResizeObserver(medir);
    if (conteudo.current) observador.observe(conteudo.current);
    return () => observador.disconnect();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  return conteudo;
}

/** A tira de anéis, na janela `faixa`. */
export function FaixaDeUso() {
  const { aneis, calibrando, recolhida, demonstracao } = useFaixa();
  const [aberto, setAberto] = useState(false);
  const conteudo = useMedirZona([aneis.length, recolhida]);

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
    <div
      className="faixa-shell"
      data-recolhida={recolhida || undefined}
      /* A tira fica na borda da tela o dia inteiro, e é ela que alguém
         fotografa. O aviso mora aqui também, e não só no painel. */
      data-demonstracao={demonstracao || undefined}
    >
      {/* O invólucro tem a altura do CONTEÚDO, e é ele que é medido.
          A janela cabe três anéis e não muda de tamanho; sem esta camada, a
          união do cartão com a lingueta daria a janela inteira, porque a
          lingueta precisa esticar-se até a altura do cartão — e "esticar" num
          flex é esticar até o container. */}
      <div className="faixa-conteudo" ref={conteudo}>
      {/* Primeiro no DOM, e à direita na tela: o invólucro é `row-reverse`,
          para a lingueta ficar colada na borda mesmo quando o cartão some. */}
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
        {aneis.map((anel, indice) => {
          const r = regua(anel, calibrando);
          const valor = rotuloDaRegua(r, anel);
          /* O anel satura em 1 e o número NÃO: acima de 100% o arco não tem
             para onde crescer, e o número é justamente o que importa ali. */
          const fracao =
            r.tipo === "cota" ? Math.min(1, r.janela.percentual / 100)
            : r.tipo === "pico" ? r.fracao
            : 0;
          return (
            <button
              type="button"
              className="faixa-anel"
              key={anel.nome}
              data-degrau={degrau(r)}
              /* A cascata é por anel, e não por quadro: três anéis entrando
                 juntos leem como um bloco; entrando em 40ms de diferença, leem
                 como três coisas. */
              style={{ ["--faixa-atraso" as string]: `${indice * 70}ms` }}
              aria-expanded={aberto}
              aria-label={
                r.tipo === "cota"
                  ? `${anel.nome}: ${r.janela.percentual}% da sessão de 5h${r.janela.obsoleta ? ", valor desatualizado" : ""}`
                  : r.tipo === "pico"
                    ? `${anel.nome}: ${valor} do pico de consumo`
                    : `${anel.nome}: ${valor} tokens`
              }
              onClick={alternar}
            >
              {/* O anel ABRAÇA a marca, em vez de conter o número. É a inversão
                  que o desenho de referência faz e que resolve o problema de
                  três anéis empilhados: com o número dentro, três anéis são
                  três números e nenhuma identidade — não dá para saber QUEM
                  está em 73%. */}
              <Ring size={44} segments={r.tipo === "nenhuma" ? [] : [{ value: fracao }]}>
                <span className="faixa-marca">
                  <MarcaDeIA nome={anel.nome} />
                </span>
              </Ring>
              <span className="faixa-valor">{valor}</span>
            </button>
          );
        })}
      </div>
      </div>
    </div>
  );
}

/** O painel, na janela `faixa-painel`. Ele lê o mesmo evento que a tira. */
export function PainelDaFaixa() {
  const { aneis, calibrando, demonstracao, agora } = useFaixa();

  const conteudo = useMemo(() => aneis.map((anel) => {
    const temPico = temRegua(anel, calibrando);
    const cota = anel.cotaSessao;
    const semana = anel.cotaSemana;
    /* A nota da sessão: o prazo do SERVIDOR quando ele existe, e só então o que
       esta máquina calcula a partir do início da janela. */
    const prazo = cota?.resetaEm ?? anel.resetaEm;
    return (
      <div className="faixa-painel-fonte" key={anel.nome}>
        <span className="faixa-painel-nome">{anel.nome}</span>
        <Barra
          rotulo="SESSÃO · 5H"
          valor={cota ? cota.percentual : anel.peso}
          teto={cota ? 100 : anel.pico}
          regua={cota ? true : temPico}
          exato={cota ? `${cota.obsoleta ? "~" : ""}${cota.percentual}%` : undefined}
          /* "sem janela aberta" é uma frase sobre TRANSCRIPT: ela diz que
             nenhum request caiu na janela de cinco horas corrente. Uma fonte
             externa não conta isso — ela só não mandou o prazo. */
          nota={
            prazo
              ? faltaPara(prazo, agora)
              : anel.temHistorico
                ? "sem janela aberta"
                : "sem prazo"
          }
        />
        {/* A semana só aparece com cota. Ela não tem versão de reserva: o
            transcript não sabe onde a semana começa nem quanto ela aguenta, e
            uma barra de semana calculada daqui seria um denominador inventado. */}
        {semana ? (
          <Barra
            rotulo="SEMANA · 7D"
            valor={semana.percentual}
            teto={100}
            regua
            exato={`${semana.obsoleta ? "~" : ""}${semana.percentual}%`}
            nota={semana.resetaEm ? faltaPara(semana.resetaEm, agora) : "sem prazo"}
          />
        ) : null}
        {/* HOJE só existe para quem conta o próprio histórico. Numa fonte
            externa `peso` e `pico` vêm zerados por construção, e uma barra vazia
            rotulada HOJE diria "não consumiu nada hoje" — frase diferente de
            "esta fonte não me conta isso". */}
        {anel.temHistorico ? (
          <Barra
            rotulo="HOJE · MAIOR DIA"
            valor={anel.pesoHoje}
            teto={anel.picoDia}
            regua={temPico}
            nota={`${anel.requisicoesHoje} ${anel.requisicoesHoje === 1 ? "request" : "requests"}`}
          />
        ) : null}
        {/* A régua é dita em voz alta, sempre. Um "73%" sem denominador seria
            exatamente o número que este desenho recusa — e agora que há DUAS
            réguas possíveis, dizer qual delas está valendo passou a importar
            mais, não menos. */}
        <p className="faixa-regua" data-demonstracao={demonstracao || undefined}>
          {demonstracao
            ? "DEMONSTRAÇÃO. Estes números vêm do MOS_FAIXA_DEMO e não são o seu consumo."
            : !anel.temHistorico
              ? "Cota dita pelo comando que você apontou em faixaFontes. O M/OS repassa o número; ele não o confere."
              : cota
            ? cota.obsoleta
              ? "Cota real do seu plano, dita pela Anthropic. O ~ marca a última leitura que deu certo: a renovação está falhando."
              : "Cota real do seu plano, dita pela Anthropic. HOJE continua medindo contra o seu maior dia."
            : calibrando
              ? "Lendo o histórico pela primeira vez. Sem régua ainda."
              : temPico
                ? "Proporção contra o seu maior consumo já observado — não contra o limite do plano, que não respondeu agora."
                : "Ainda não há histórico suficiente para comparar. O número é absoluto."}
        </p>
      </div>
    );
  }), [aneis, calibrando, agora]);

  if (!aneis.length) return null;

  return (
    <div className="faixa-painel" role="group" aria-label="Consumo de IA">
      {/* O que ROLA é o corpo, e não o cartão.
          A seta que aponta para a calha é um pseudo-elemento que sai para fora
          do cartão, e `overflow` num elemento recorta o que sai dele — a seta
          sumia. Duas camadas: o cartão desenha a seta, o corpo rola. */}
      <div className="faixa-painel-corpo">
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
    </div>
  );
}
