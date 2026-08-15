import { useEffect, useRef } from "react";

/**
 * Dot Field — camada atmosferica da Home.
 *
 * AVISO: contraria DESIGN-FOUNDATIONS secao 13 ("sem particulas") e
 * UX-PRINCIPLES secao 16 ("animacoes de particulas"). Decisao do dono do
 * produto, registrada no commit e no bloco de CSS desta rodada.
 *
 * Adaptado de Backgrounds/DotField. O original roda um requestAnimationFrame
 * permanente MAIS um setInterval de 20ms so para medir a velocidade do cursor,
 * e move cada ponto conforme o mouse. Duas coisas o desqualificam aqui:
 *
 *   1. o M/OS e uma janela sempre aberta. Um RAF permanente e consumo de CPU e
 *      bateria em idle pelo resto do dia, sem contrapartida;
 *   2. a especificacao pedia "movimento muito sutil ou ate quase estatico".
 *      Levada a serio, ela dispensa o loop: se o campo nao deve ser notado, o
 *      que ele ganha reagindo ao ponteiro e exatamente ser notado.
 *
 * Entao este pinta UMA vez. Repinta so quando a superficie muda de tamanho ou
 * o tema troca — dois eventos raros e observados, nunca um cronometro. Em
 * repouso o custo e zero, que e o mesmo custo de nao existir.
 *
 * A cor vem da propriedade `color` resolvida no proprio canvas, entao quem
 * manda e o CSS: o componente nao conhece token nenhum e segue o tema sozinho.
 */
export function DotField({ spacing = 28, radius = 1 }: { spacing?: number; radius?: number }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const element = canvas.current;
    const parent = element?.parentElement;
    if (!element || !parent) return;

    function paint() {
      if (!element || !parent) return;
      const ratio = window.devicePixelRatio || 1;
      const width = parent.clientWidth;
      const height = parent.clientHeight;
      if (!width || !height) return;
      // O buffer segue o devicePixelRatio: sem isto o ponto de 1px vira uma
      // mancha borrada em tela 125% ou 150%, que sao as escalas que o proprio
      // quality gate do design exige testar.
      element.width = Math.round(width * ratio);
      element.height = Math.round(height * ratio);
      element.style.width = `${width}px`;
      element.style.height = `${height}px`;
      const context = element.getContext("2d");
      if (!context) return;
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
      context.clearRect(0, 0, width, height);
      context.fillStyle = window.getComputedStyle(element).color;
      for (let y = spacing; y < height; y += spacing) {
        for (let x = spacing; x < width; x += spacing) {
          context.beginPath();
          context.arc(x, y, radius, 0, Math.PI * 2);
          context.fill();
        }
      }
    }

    paint();
    const resize = new ResizeObserver(paint);
    resize.observe(parent);
    const theme = new MutationObserver(paint);
    theme.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    return () => { resize.disconnect(); theme.disconnect(); };
  }, [spacing, radius]);

  return <canvas ref={canvas} className="dot-field" aria-hidden="true" />;
}
