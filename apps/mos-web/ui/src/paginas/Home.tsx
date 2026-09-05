import { useLayoutEffect, useRef, useState, type PointerEvent } from "react";
import type { EstadoDoAparelho, Panorama } from "../api";
import { Cartao } from "../componentes/Cartao";
import type { Dados, Pagina } from "../navegacao";
import { alternarOculto, aplicarArranjo, ordenar, reordenar, type Arranjo } from "./arranjo";
import { cartoesDaHome } from "./cartoes";
import { alvoDoDedo, animarDe, medir } from "./flip";

/** Quanto tempo o dedo fica parado antes da Home entrar em modo de arrumar. */
const PRESSAO = 300;
/** Quantos pixels de movimento cancelam a pressão. Acima disso é rolagem. */
const TOLERANCIA = 8;

/**
 * O hub.
 *
 * O app abria no compositor e nunca dizia como estao as coisas. A Home responde
 * isso antes de qualquer lista: o dia, e depois os cartoes.
 *
 * # Por que pressionar, e não um botão
 *
 * Arrumar era um botão cinza de 11px no canto, e ninguém nunca o viu. O gesto
 * que substitui é o que o iOS já ensinou a todo mundo para arrumar a tela de
 * início: **segure um cartão**. Não precisa ser descoberto porque já foi.
 *
 * Os 300 ms não são arbitrários: abaixo disso o gesto colide com o toque
 * comum — abrir a página do cartão — e a Home passaria a entrar em modo de
 * arrumar sozinha. E os 8 px de tolerância separam segurar de rolar: quem rola
 * a lista com o polegar apoia o dedo primeiro, e sem a tolerância cada rolagem
 * viraria um arrasto.
 */
export function Home({
  estado,
  dados,
  panorama,
  arranjo,
  arrumando,
  aoArrumando,
  aoArranjar,
  aoIr,
}: {
  estado: EstadoDoAparelho | null;
  dados: Dados;
  /** Nulo enquanto não chegou, ou quando o servidor é antigo demais para ter a
   *  rota. A Home continua inteira nos dois casos. */
  panorama: Panorama | null;
  arranjo: Arranjo;
  arrumando: boolean;
  aoArrumando: (arrumando: boolean) => void;
  aoArranjar: (arranjo: Arranjo) => void;
  aoIr: (pagina: Pagina) => void;
}) {
  const agora = new Date();
  const todos = cartoesDaHome(estado, dados, agora, panorama);
  // Dentro do modo, o escondido continua na grade — apagado. Fora dele, some.
  // É a única forma de descobrir que ele existe para trazê-lo de volta.
  const cartoes = arrumando ? ordenar(todos, arranjo) : aplicarArranjo(todos, arranjo);
  const chaves = cartoes.map((cartao) => cartao.chave);

  const [pegado, setPegado] = useState<string | null>(null);
  const nos = useRef(new Map<string, HTMLElement>());
  const medidas = useRef(new Map<string, DOMRect>());
  const gesto = useRef<{
    chave: string;
    x: number;
    y: number;
    relogio: number | null;
    arrastando: boolean;
  } | null>(null);

  // Depois de cada pintura: leva cada cartão do lugar onde ele estava até o
  // lugar onde ele está. Roda sempre, e não só ao reordenar, porque o cartão
  // também muda de lugar quando outro é escondido ou quando o sync some.
  useLayoutEffect(() => {
    animarDe(medidas.current, nos.current, pegado, 180);
    medidas.current = medir(nos.current);
  });

  function guardarNo(chave: string, no: HTMLElement | null) {
    if (no) nos.current.set(chave, no);
    else nos.current.delete(chave);
  }

  function comecar(evento: PointerEvent<HTMLDivElement>, chave: string) {
    // O botão de esconder é um alvo dentro do cartão: sem isto, tocá-lo também
    // começaria um arrasto e o cartão sairia do lugar ao ser escondido.
    if ((evento.target as HTMLElement).closest(".cartao-esconder")) return;
    const alvo = evento.currentTarget;
    gesto.current = {
      chave,
      x: evento.clientX,
      y: evento.clientY,
      arrastando: false,
      relogio: null,
    };
    if (arrumando) {
      pegar(alvo, evento.pointerId, chave);
      return;
    }
    gesto.current.relogio = window.setTimeout(() => {
      aoArrumando(true);
      pegar(alvo, evento.pointerId, chave);
      // O aparelho responde antes da tela: o toque curto vibrando é o que
      // ensina que a pressão FUNCIONOU, sem precisar de aviso escrito.
      navigator.vibrate?.(8);
    }, PRESSAO);
  }

  function pegar(alvo: HTMLElement, ponteiro: number, chave: string) {
    if (!gesto.current) return;
    gesto.current.arrastando = true;
    // Sem a captura, tirar o dedo de cima do cartão entrega os eventos ao
    // elemento de baixo e o arrasto morre no meio do caminho.
    alvo.setPointerCapture?.(ponteiro);
    setPegado(chave);
  }

  function mover(evento: PointerEvent<HTMLDivElement>) {
    const atual = gesto.current;
    if (!atual) return;

    if (!atual.arrastando) {
      const andou =
        Math.abs(evento.clientX - atual.x) + Math.abs(evento.clientY - atual.y);
      if (andou > TOLERANCIA) cancelar();
      return;
    }

    const no = nos.current.get(atual.chave);
    if (no) {
      no.style.transform = `translate(${evento.clientX - atual.x}px, ${
        evento.clientY - atual.y
      }px) scale(1.02) rotate(-1.2deg)`;
    }

    const de = chaves.indexOf(atual.chave);
    const para = alvoDoDedo(nos.current, chaves, evento.clientX, evento.clientY);
    if (para >= 0 && para !== de) {
      // A origem do arrasto anda junto com o cartão: sem isto, o deslocamento
      // passaria a ser medido a partir de um lugar onde o cartão não está mais,
      // e ele saltaria para longe do dedo na troca seguinte.
      const antes = no?.getBoundingClientRect();
      aoArranjar(reordenar(arranjo, chaves, de, para));
      if (no && antes) {
        const depois = no.getBoundingClientRect();
        atual.x += depois.left - antes.left;
        atual.y += depois.top - antes.top;
      }
    }
  }

  function largar() {
    const atual = gesto.current;
    if (atual) {
      const no = nos.current.get(atual.chave);
      if (no) no.style.transform = "";
    }
    cancelar();
    setPegado(null);
  }

  function cancelar() {
    if (gesto.current?.relogio) window.clearTimeout(gesto.current.relogio);
    if (gesto.current) gesto.current.arrastando = false;
    gesto.current = null;
  }

  return (
    <div className="home" data-arrumando={arrumando || undefined}>
      {arrumando ? null : (
        <header className="home-dia">
          <h2>{porExtenso(agora)}</h2>
          <span>SEM {semanaDoAno(agora)}</span>
        </header>
      )}

      <div className="home-grade">
        {cartoes.map((cartao) => {
          const oculto = arranjo.ocultos.includes(cartao.chave);
          return (
            <div
              className="home-slot"
              key={cartao.chave}
              ref={(no) => guardarNo(cartao.chave, no)}
              data-largo={cartao.chave === "ultima" || undefined}
              data-pegado={pegado === cartao.chave || undefined}
              data-oculto={(arrumando && oculto) || undefined}
              onPointerDown={(evento) => comecar(evento, cartao.chave)}
              onPointerMove={mover}
              onPointerUp={largar}
              onPointerCancel={largar}
            >
              <Cartao
                rotulo={cartao.rotulo}
                numero={cartao.numero}
                legenda={arrumando && oculto ? "escondido" : cartao.legenda}
                aposto={arrumando ? undefined : cartao.aposto}
                enfeite={arrumando ? undefined : cartao.enfeite}
                urgente={cartao.urgente}
                largo={cartao.chave === "ultima"}
                palavra={cartao.palavra}
                // Arrumando, o cartão não leva a lugar nenhum: quem está mexendo
                // na ordem erra o alvo o tempo todo, e sair da Home a cada erro
                // faria recomeçar do zero.
                aoTocar={() => (arrumando ? undefined : aoIr(cartao.destino))}
              />
              {arrumando ? (
                <button
                  type="button"
                  className="cartao-esconder"
                  aria-label={
                    oculto ? `Mostrar ${cartao.rotulo}` : `Esconder ${cartao.rotulo}`
                  }
                  aria-pressed={oculto}
                  onClick={() => aoArranjar(alternarOculto(arranjo, cartao.chave))}
                >
                  <span aria-hidden="true" />
                </button>
              ) : null}
            </div>
          );
        })}
      </div>
    </div>
  );
}

const DIAS = [
  "Domingo",
  "Segunda",
  "Terça",
  "Quarta",
  "Quinta",
  "Sexta",
  "Sábado",
] as const;

const MESES = [
  "jan",
  "fev",
  "mar",
  "abr",
  "mai",
  "jun",
  "jul",
  "ago",
  "set",
  "out",
  "nov",
  "dez",
] as const;

/**
 * `Quarta, 9 de set`.
 *
 * A Home abria em números sem dizer de quando eles são — e "9h08 nesta semana"
 * só quer dizer alguma coisa para quem sabe que dia é hoje.
 */
export function porExtenso(quando: Date): string {
  return `${DIAS[quando.getDay()]}, ${quando.getDate()} de ${MESES[quando.getMonth()]}`;
}

/**
 * A semana ISO do ano.
 *
 * Ela está ali porque o M/OS conta horas por semana, e "SEM 37" é como se fala
 * de uma semana de trabalho — não "a semana de 7 a 13". A regra ISO diz que a
 * semana 1 é a que contém a primeira quinta-feira do ano.
 */
export function semanaDoAno(quando: Date): number {
  const dia = new Date(Date.UTC(quando.getFullYear(), quando.getMonth(), quando.getDate()));
  // Empurra até a quinta-feira da mesma semana: é ela que decide de que ano a
  // semana é, e sem esse passo a virada de ano dá 53 onde deveria dar 1.
  dia.setUTCDate(dia.getUTCDate() + 4 - (dia.getUTCDay() || 7));
  const inicio = new Date(Date.UTC(dia.getUTCFullYear(), 0, 1));
  return Math.ceil(((dia.getTime() - inicio.getTime()) / 86_400_000 + 1) / 7);
}
