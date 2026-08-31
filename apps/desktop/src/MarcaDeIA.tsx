/**
 * A marca de cada provedor de IA, para dentro do anel da faixa.
 *
 * # Por que ícone e não texto
 *
 * A faixa mora na borda da tela e é lida de relance, com o olho de passagem.
 * "Claude Code" em 9px numa tira de 96 pixels não é lido — é reconhecido pela
 * forma da palavra, que é o mesmo trabalho que um ícone faz melhor e em um
 * terço do espaço. Com três provedores empilhados, três nomes escritos viram
 * três linhas de texto pequeno; três marcas viram três formas distinguíveis.
 *
 * # Desenhadas aqui, e não baixadas
 *
 * São traços simplificados, em `currentColor`, num viewBox de 24 — a mesma
 * gramática dos `marks/` do design system. Não são os logotipos oficiais e não
 * tentam ser: o que a faixa precisa é de uma forma que a pessoa associe ao
 * provedor em meio segundo, monocromática, legível a 18 pixels sobre preto.
 * Um SVG oficial de marca costuma trazer cor fixa, gradiente e detalhe que
 * some nesse tamanho.
 *
 * # O nome é a chave, e o desconhecido tem resposta
 *
 * Uma fonte externa vem do `settings.json` com o nome que o dono escolheu.
 * Quando ele não bate com nenhuma marca conhecida, o anel recebe a inicial —
 * que distingue "Codex" de "Cursor" sem fingir saber quem é.
 */

type Props = { nome: string; tamanho?: number };

/** O nome, reduzido ao que dá para casar. */
function chave(nome: string) {
  return nome.toLowerCase().normalize("NFD").replace(/[^a-z]/g, "");
}

/** A rajada do Claude: oito raios afilados a partir do centro. */
function Claude({ tamanho }: { tamanho: number }) {
  // Oito raios, e não doze: a 18px os doze encostam um no outro e a estrela
  // vira um disco.
  const raios = Array.from({ length: 8 }, (_, indice) => (indice * 360) / 8);
  return (
    <svg viewBox="0 0 24 24" width={tamanho} height={tamanho} aria-hidden="true" focusable="false">
      {raios.map((angulo) => (
        <line
          key={angulo}
          x1="12"
          y1="12"
          x2="12"
          y2="2.5"
          stroke="currentColor"
          strokeWidth="1.9"
          strokeLinecap="round"
          transform={`rotate(${angulo} 12 12)`}
        />
      ))}
    </svg>
  );
}

/** O nó da OpenAI, reduzido a uma hexafólia de traço. */
function OpenAI({ tamanho }: { tamanho: number }) {
  const petalas = Array.from({ length: 6 }, (_, indice) => (indice * 360) / 6);
  return (
    <svg viewBox="0 0 24 24" width={tamanho} height={tamanho} aria-hidden="true" focusable="false">
      {petalas.map((angulo) => (
        <ellipse
          key={angulo}
          cx="12"
          cy="8"
          rx="3.6"
          ry="5.6"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.15"
          transform={`rotate(${angulo} 12 12)`}
        />
      ))}
    </svg>
  );
}

/** O cubo do Cursor: três faces de um prisma isométrico. */
function Cursor({ tamanho }: { tamanho: number }) {
  return (
    <svg viewBox="0 0 24 24" width={tamanho} height={tamanho} aria-hidden="true" focusable="false">
      <path
        d="M12 2.6 21 7.7v8.6L12 21.4 3 16.3V7.7z"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinejoin="round"
      />
      <path
        d="M3 7.7 12 12.8l9-5.1M12 12.8v8.6"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinejoin="round"
        opacity="0.62"
      />
    </svg>
  );
}

/** A faísca de quatro pontas do Gemini. */
function Gemini({ tamanho }: { tamanho: number }) {
  return (
    <svg viewBox="0 0 24 24" width={tamanho} height={tamanho} aria-hidden="true" focusable="false">
      <path
        d="M12 1.8c0 5.6 4.6 10.2 10.2 10.2C16.6 12 12 16.6 12 22.2 12 16.6 7.4 12 1.8 12 7.4 12 12 7.4 12 1.8z"
        fill="currentColor"
      />
    </svg>
  );
}

/** O xis do Grok. */
function Grok({ tamanho }: { tamanho: number }) {
  return (
    <svg viewBox="0 0 24 24" width={tamanho} height={tamanho} aria-hidden="true" focusable="false">
      <path
        d="M4 3.5 20 20.5M20 3.5 4 20.5"
        stroke="currentColor"
        strokeWidth="2.1"
        strokeLinecap="round"
      />
    </svg>
  );
}

/** Quem não tem marca fica com a inicial. */
function Inicial({ nome, tamanho }: { nome: string; tamanho: number }) {
  const letra = nome.trim().charAt(0).toUpperCase() || "?";
  return (
    <svg viewBox="0 0 24 24" width={tamanho} height={tamanho} aria-hidden="true" focusable="false">
      <text
        x="12"
        y="12"
        textAnchor="middle"
        dominantBaseline="central"
        fill="currentColor"
        fontSize="13"
        fontWeight="600"
        fontFamily="var(--font-mono, monospace)"
      >
        {letra}
      </text>
    </svg>
  );
}

export function MarcaDeIA({ nome, tamanho = 18 }: Props) {
  const k = chave(nome);
  // Ordem por especificidade: "claudecode" contém "claude", e um `includes`
  // solto casaria os dois na ordem errada dependendo de quem viesse antes.
  if (k.includes("claude") || k.includes("anthropic")) return <Claude tamanho={tamanho} />;
  if (k.includes("cursor")) return <Cursor tamanho={tamanho} />;
  if (k.includes("codex") || k.includes("openai") || k.includes("chatgpt") || k.includes("gpt"))
    return <OpenAI tamanho={tamanho} />;
  if (k.includes("gemini") || k.includes("antigravity") || k.includes("google"))
    return <Gemini tamanho={tamanho} />;
  if (k.includes("grok") || k.includes("xai")) return <Grok tamanho={tamanho} />;
  return <Inicial nome={nome} tamanho={tamanho} />;
}
