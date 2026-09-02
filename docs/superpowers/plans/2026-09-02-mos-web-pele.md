# A pele do bolso — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dar ao `mos-web` a marca do M/OS, uma Home que abre o app, uma barra de cinco destinos e o movimento que o brief permite — sem acrescentar uma única rota no servidor.

**Architecture:** O `App.tsx` de 666 linhas vira uma casca que guarda o estado e escolhe a página; cada página é um arquivo em `ui/src/paginas/`, e o que elas têm em comum vira componente em `ui/src/componentes/`. A decisão de cada tela sai do JSX e vira função pura testável (o padrão que `instantes.ts` + `instantes.test.ts` já usam neste app). Uma bancada servida só pelo Vite monta todas as páginas com dados falsos, para a aparência ser conferida por foto sem passar pela porta com passkey.

**Tech Stack:** React 19, TypeScript, Vite 6, vitest 3 (sem testing-library — os testes são de função pura), CSS puro, Web Animations API. Nenhuma dependência nova.

**Spec:** `docs/superpowers/specs/2026-09-02-mos-web-pele-design.md`

## Global Constraints

- **Nenhuma dependência nova em `apps/mos-web/ui/package.json`.** O CI roda `npm ci` nessa pasta para montar a PWA.
- **Nenhuma rota nova no servidor.** Esta entrega usa só `/api/estado`, `/api/inbox`, `/api/tasks`, `/api/lembretes`, `/api/push/*`, `/api/porta/*`.
- **Um acento só: o sódio `#E7C24E`.** Tinta sobre ele: `#0A0C0E`. Nenhuma cor de marca nova entra — nem uma por página.
- **Tipografia:** Schibsted Grotesk (400/500/700) e JetBrains Mono (400/500). Nada mais.
- **A barra dá meia-volta (180°), e é o único spinner do sistema.** Não existe círculo girando, não existem três pontos.
- **Nada é hover.** O feedback de toque mora em `:active`.
- **Todo movimento morre dentro de `@media (prefers-reduced-motion: reduce)`.**
- **Alvo de toque mínimo 44px** (`--toque`), já definido em `estilo.css`.
- Comentários e nomes em português, como todo o app. Comentário explica **por quê**, não o quê.
- Os comandos rodam de `apps/mos-web/ui` salvo onde estiver dito o contrário.

---

## File Structure

**Criados:**

| Arquivo | Responsabilidade |
| --- | --- |
| `ui/src/componentes/Marca.tsx` | A barra do M/OS como SVG, com o polígono certo por tamanho e a meia-volta |
| `ui/src/componentes/marca.ts` | `poligonoPara(tamanho)` — a geometria, pura e testável |
| `ui/src/componentes/Cartao.tsx` | Cartão da Home: rótulo, número grande, legenda, destino |
| `ui/src/componentes/Vazio.tsx` | Estado vazio com frase e, quando faz sentido, uma ação |
| `ui/src/componentes/Barra.tsx` | A barra inferior de cinco destinos, com badges |
| `ui/src/paginas/Home.tsx` | O hub |
| `ui/src/paginas/home.ts` | `cartoesDaHome(...)` — que cartões existem e o que dizem |
| `ui/src/paginas/Capturar.tsx` | Compositor + últimas três capturas |
| `ui/src/paginas/Inbox.tsx` | A lista de capturas |
| `ui/src/paginas/Tasks.tsx` | A lista de tasks, com marcar e sino |
| `ui/src/paginas/Lembretes.tsx` | Lembretes + o canal de notificação |
| `ui/src/paginas/Mais.tsx` | Índice: Lembretes, notificação, sessão, sync |
| `ui/src/telas.css` | Componentes e páginas |
| `ui/bancada.html` + `ui/src/bancada.tsx` | A bancada de desenvolvimento |
| `ui/src/paginas/home.test.ts`, `ui/src/componentes/marca.test.ts`, `ui/src/navegacao.test.ts` | Os testes |

**Modificados:** `ui/src/App.tsx` (vira casca), `ui/src/estilo.css` (só tokens e base), `ui/index.html` (nada além do ícone), `ui/public/*` e `static/*` (ícones), `scripts/gerar-icones-web.py`, `apps/mos-web/README.md`.

**Intocados:** `Porta.tsx`, `Quando.tsx`, `api.ts`, `instantes.ts`, `notificacoes.ts`, `cerimonia.ts`, e todo o Rust.

---

### Task 1: A marca

O ícone de bolso é um "M" desenhado à mão; a marca do M/OS é a barra. Esta task troca o desenho em todos os lugares e cria o componente que o app vai usar por dentro.

**Files:**
- Create: `apps/mos-web/ui/src/componentes/marca.ts`
- Create: `apps/mos-web/ui/src/componentes/Marca.tsx`
- Create: `apps/mos-web/ui/src/componentes/marca.test.ts`
- Modify: `scripts/gerar-icones-web.py` (troca o traço do M pelo polígono da barra)
- Modify: `apps/mos-web/ui/public/icone.svg`, `apps/mos-web/ui/public/icone-maskable.svg`
- Regenerated: `apps/mos-web/ui/public/icone-180.png`, `icone-192.png`, `icone-512.png`

**Interfaces:**
- Produces: `poligonoPara(tamanho: number): ReadonlyArray<readonly [number, number]>` e `pontosDoPoligono(tamanho: number): string` (o `points` do SVG, ex. `"38,8 53,8 26,56 11,56"`); `<Marca tamanho={number} girando?: boolean className?: string />`.

- [ ] **Step 1: Write the failing test**

Criar `apps/mos-web/ui/src/componentes/marca.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { pontosDoPoligono, poligonoPara } from "./marca";

// O brief (`docs/BRIEF-SISTEMA-DE-LOGOS.md`) manda corrigir o ANGULO por
// escala, e proibe escalar um SVG so. A mesma inclinacao geometrica le mais
// fina conforme o desenho encolhe; o angulo abre para compensar.
describe("a barra escolhe o poligono pelo tamanho", () => {
  it("usa 22 graus de 128 para cima", () => {
    expect(pontosDoPoligono(512)).toBe("38,8 53,8 26,56 11,56");
    expect(pontosDoPoligono(128)).toBe("38,8 53,8 26,56 11,56");
  });

  it("usa 18 graus entre 48 e 127", () => {
    expect(pontosDoPoligono(64)).toBe("40,10 54,10 24,54 10,54");
    expect(pontosDoPoligono(48)).toBe("40,10 54,10 24,54 10,54");
  });

  it("usa 14 graus abaixo de 48", () => {
    expect(pontosDoPoligono(32)).toBe("42,12 56,12 22,52 8,52");
    expect(pontosDoPoligono(16)).toBe("42,12 56,12 22,52 8,52");
  });

  it("mantem os quatro vertices no viewBox de 64", () => {
    for (const tamanho of [512, 64, 16]) {
      const pontos = poligonoPara(tamanho);
      expect(pontos).toHaveLength(4);
      for (const [x, y] of pontos) {
        expect(x).toBeGreaterThanOrEqual(0);
        expect(x).toBeLessThanOrEqual(64);
        expect(y).toBeGreaterThanOrEqual(0);
        expect(y).toBeLessThanOrEqual(64);
      }
    }
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- marca`
Expected: FAIL — `Failed to resolve import "./marca"`.

- [ ] **Step 3: Write the minimal implementation**

Criar `apps/mos-web/ui/src/componentes/marca.ts`:

```ts
/**
 * A geometria da marca, e o motivo de ela nao ser um SVG so.
 *
 * O simbolo do M/OS e uma barra solida inclinada. O `BRIEF-SISTEMA-DE-LOGOS.md`
 * proibe escalar um desenho unico: a mesma inclinacao le mais fina conforme o
 * icone encolhe, entao o angulo ABRE para compensar. Sao tres desenhos, no
 * mesmo viewBox de 64, com centroide em (32,32).
 */
type Ponto = readonly [number, number];

const BARRAS: Record<"grande" | "media" | "pequena", ReadonlyArray<Ponto>> = {
  // 22 graus — 128 para cima
  grande: [[38, 8], [53, 8], [26, 56], [11, 56]],
  // 18 graus — 48 a 127
  media: [[40, 10], [54, 10], [24, 54], [10, 54]],
  // 14 graus — abaixo de 48
  pequena: [[42, 12], [56, 12], [22, 52], [8, 52]],
};

export function poligonoPara(tamanho: number): ReadonlyArray<Ponto> {
  if (tamanho >= 128) return BARRAS.grande;
  if (tamanho >= 48) return BARRAS.media;
  return BARRAS.pequena;
}

/** O mesmo poligono no formato que o atributo `points` do SVG espera. */
export function pontosDoPoligono(tamanho: number): string {
  return poligonoPara(tamanho)
    .map(([x, y]) => `${x},${y}`)
    .join(" ");
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- marca`
Expected: PASS — 4 testes.

- [ ] **Step 5: Write the component**

Criar `apps/mos-web/ui/src/componentes/Marca.tsx`:

```tsx
import { pontosDoPoligono } from "./marca";

/**
 * A barra do M/OS.
 *
 * `girando` e o unico spinner deste app: a barra da meia-volta. O brief e
 * taxativo — nao existe circulo girando, nao existem tres pontos. A animacao
 * mora no CSS (`.marca-barra[data-girando]`), e nao aqui, para o
 * `prefers-reduced-motion` poder desliga-la sem a tela saber.
 */
export function Marca({
  tamanho = 24,
  girando = false,
  className,
}: {
  tamanho?: number;
  girando?: boolean;
  className?: string;
}) {
  return (
    <svg
      className={className ? `marca-barra ${className}` : "marca-barra"}
      data-girando={girando || undefined}
      viewBox="0 0 64 64"
      width={tamanho}
      height={tamanho}
      role="img"
      aria-label="M/OS"
    >
      <polygon points={pontosDoPoligono(tamanho)} fill="currentColor" />
    </svg>
  );
}
```

- [ ] **Step 6: Trocar o SVG do ícone**

Substituir `apps/mos-web/ui/public/icone.svg` inteiro por:

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" role="img" aria-label="M/OS">
  <!-- Campo sodio com a barra em tinta: e a marca do M/OS, e nao um "M"
       desenhado. Raio 13.4 = 21% de 64, o mesmo do icone do desktop
       (`scripts/gerar-icones.py`), para os dois aparecerem iguais lado a lado
       na tela do celular. -->
  <rect width="64" height="64" rx="13.4" fill="#E7C24E"/>
  <polygon points="38,8 53,8 26,56 11,56" fill="#0A0C0E"/>
</svg>
```

E `apps/mos-web/ui/public/icone-maskable.svg` por:

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" role="img" aria-label="M/OS">
  <!-- Maskable: o Android recorta em circulo, entao o campo vai ate a borda
       (sem raio) e a barra encolhe para caber na zona segura de 80%. Uma marca
       desenhada ate a margem sairia com as pontas cortadas. -->
  <rect width="64" height="64" fill="#E7C24E"/>
  <polygon points="36.4,14.4 48.4,14.4 27.6,52.8 15.6,52.8" fill="#0A0C0E"/>
</svg>
```

- [ ] **Step 7: Trocar a geometria no gerador de PNG**

Em `scripts/gerar-icones-web.py`, substituir o bloco `CAMINHO`/`TRACO` e a função `desenhar` por:

```python
# Os tres poligonos do brief, no viewBox de 64 — os MESMOS de
# `scripts/gerar-icones.py`. Aqui so o "grande" e usado: 180, 192 e 512 estao
# todos acima de 128.
BARRAS = {
    "large": [(38, 8), (53, 8), (26, 56), (11, 56)],
    "medium": [(40, 10), (54, 10), (24, 54), (10, 54)],
    "small": [(42, 12), (56, 12), (22, 52), (8, 52)],
}


def barra_para(tamanho):
    if tamanho >= 128:
        return BARRAS["large"]
    if tamanho >= 48:
        return BARRAS["medium"]
    return BARRAS["small"]


def desenhar(tamanho):
    lado = tamanho * SUPER
    escala = lado / 64.0
    # Quadrado CHEIO de sodio, sem cantos arredondados: o iOS arredonda sozinho
    # por cima. Um PNG que ja chega arredondado ganha cantos PRETOS depois da
    # mascara do sistema — moldura escura que ninguem desenhou.
    imagem = Image.new("RGBA", (lado, lado), SODIO)
    pincel = ImageDraw.Draw(imagem)
    pincel.polygon([(x * escala, y * escala) for x, y in barra_para(tamanho)], fill=TINTA)
    return imagem.resize((tamanho, tamanho), FILTRO)
```

E trocar as constantes de cor no topo do arquivo:

```python
SODIO = (231, 194, 78, 255)    # #E7C24E — o campo
TINTA = (10, 12, 14, 255)      # #0A0C0E — a barra
```

Remover `FUNDO`, `CAMINHO` e `TRACO`, que deixam de ser usados.

- [ ] **Step 8: Regenerar os PNGs e olhar**

Run (da raiz do repositório): `python scripts/gerar-icones-web.py`
Expected: imprime `gerado apps/mos-web/ui/public/icone-180.png` e as outras duas.

Abrir os três PNGs com a ferramenta Read e confirmar: campo sódio cheio, barra escura inclinada para a direita, sem cantos transparentes.

- [ ] **Step 9: Conferir que o binário passa a servir o ícone novo**

Run: `npm run build`
Expected: `vite build` escreve em `../static`, copiando `public/` junto.

`static/` é **gitignored** (`apps/mos-web/.gitignore`): ela é gerada, e o
`rust-embed` a lê em tempo de compilação. Não há nada a versionar ali — o que se
confere é só que o build passou.

- [ ] **Step 10: Commit**

```bash
git add apps/mos-web/ui/src/componentes apps/mos-web/ui/public scripts/gerar-icones-web.py
git commit -m "feat(mos-web): a barra vira a marca do bolso

O icone era um M desenhado a mao, campo escuro com traco ambar. A marca do M/OS
e a barra: paralelogramo solido em campo sodio. Na tela de inicio do iPhone o
app nao se identificava como sendo do mesmo sistema.

O angulo e corrigido por escala e nunca por SVG escalado, como o brief manda: a
mesma inclinacao le mais fina conforme o icone encolhe."
```

---

### Task 2: A bancada

Sem ela, cada olhada na tela custa a cerimônia da passkey. Com ela, toda task seguinte é conferível por foto.

**Files:**
- Create: `apps/mos-web/ui/bancada.html`
- Create: `apps/mos-web/ui/src/bancada.tsx`
- Create: `apps/mos-web/ui/src/falso.ts`

**Interfaces:**
- Produces: `FALSO` — objeto com `capturas: Capture[]`, `tasks: Task[]`, `lembretes: Lembrete[]`, `estado: EstadoDoAparelho`, usado por toda foto daqui em diante.

- [ ] **Step 1: Dados falsos**

Criar `apps/mos-web/ui/src/falso.ts`:

```ts
import type { Capture, EstadoDoAparelho, Lembrete, Task } from "./api";

/**
 * O banco de mentira da bancada.
 *
 * Os textos sao realistas de proposito: "task 1" cabe em qualquer largura, e e
 * exatamente por isso que ela nao prova nada. Titulo longo, acento e numero
 * grande sao o que quebra layout de verdade.
 */
const AGORA = new Date("2026-09-02T14:30:00Z");

function ha(minutos: number): string {
  return new Date(AGORA.getTime() - minutos * 60_000).toISOString();
}

function daqui(minutos: number): string {
  return new Date(AGORA.getTime() + minutos * 60_000).toISOString();
}

export const FALSO: {
  capturas: Capture[];
  tasks: Task[];
  lembretes: Lembrete[];
  estado: EstadoDoAparelho;
} = {
  capturas: [
    { id: "c1", content: "Ligar para o cliente do Rancho Queimado sobre a prancha 04", capturedAt: ha(12) },
    { id: "c2", content: "Comprar cabo HDMI", capturedAt: ha(180) },
    { id: "c3", content: "Ideia: o CronoCAD podia sugerir a hora esquecida", capturedAt: ha(1500) },
  ],
  tasks: [
    { id: "t1", title: "Fechar o levantamento do Rancho Queimado", description: "", state: "doing" },
    { id: "t2", title: "Revisar a planta do quiosque", description: "", state: "planned" },
    { id: "t3", title: "Mandar a fatura de agosto", description: "", state: "done" },
  ],
  lembretes: [
    {
      id: "l1", title: "Mandar a fatura de agosto", body: "",
      target: { type: "task", id: "t3" }, status: "due", priority: "high",
      nextDueAt: ha(30), snoozeCount: 0, createdAt: ha(600),
    },
    {
      id: "l2", title: "Reuniao com o Juliano", body: "",
      target: null, status: "scheduled", priority: "normal",
      nextDueAt: daqui(240), snoozeCount: 1, createdAt: ha(2000),
    },
  ],
  estado: { pendentes: 3, sincroniza: true, chavePush: "chave-falsa", aparelhosAvisados: 1 },
};
```

Se algum campo obrigatório de `Capture`, `Task` ou `Lembrete` faltar, o `tsc` da Step 4 acusa — conferir os tipos em `ui/src/api.ts` e completar.

- [ ] **Step 2: A página da bancada**

Criar `apps/mos-web/ui/bancada.html`:

```html
<!doctype html>
<html lang="pt-BR">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
    <title>M/OS — bancada</title>
  </head>
  <body>
    <!-- A bancada existe SO em desenvolvimento: o `vite build` tem uma entrada
         so (`index.html`), entao nada disto vai para o binario. -->
    <div id="raiz"></div>
    <script type="module" src="/src/bancada.tsx"></script>
  </body>
</html>
```

Criar `apps/mos-web/ui/src/bancada.tsx`:

```tsx
import { createRoot } from "react-dom/client";
import "./estilo.css";
import { FALSO } from "./falso";

/**
 * A bancada.
 *
 * O `mos-web` de verdade exige sessao — `/app/*` sem passkey cai na porta —,
 * entao fotografar o app real custa a cerimonia inteira antes de cada olhada.
 * Aqui as telas nascem com dado falso e o CSS de verdade, lado a lado, nas duas
 * larguras que importam.
 *
 * O que ela NAO prova: nada que dependa do servidor. Comportamento continua
 * sendo assunto dos testes, e do app de verdade.
 */
const LARGURAS = [390, 430];

function Moldura({ titulo, largura, children }: { titulo: string; largura: number; children: React.ReactNode }) {
  return (
    <figure className="bancada-moldura">
      <figcaption>{titulo} · {largura}px</figcaption>
      <div className="bancada-tela" style={{ width: largura, height: 780 }}>
        {children}
      </div>
    </figure>
  );
}

function Bancada() {
  return (
    <div className="bancada">
      {LARGURAS.map((largura) => (
        <Moldura key={largura} titulo="Nada ainda" largura={largura}>
          <p style={{ padding: 20 }}>
            As paginas entram aqui conforme as tasks seguintes as criarem.
            Capturas falsas: {FALSO.capturas.length}.
          </p>
        </Moldura>
      ))}
    </div>
  );
}

createRoot(document.getElementById("raiz")!).render(<Bancada />);
```

- [ ] **Step 3: O estilo da bancada**

Acrescentar ao fim de `apps/mos-web/ui/src/estilo.css`:

```css
/* A BANCADA — só existe em desenvolvimento.
   Fundo claro de propósito: o app é escuro, e sobre fundo escuro a silhueta do
   cartão some. Contra o claro, dá para ver onde a borda termina. */
.bancada {
  display: flex;
  flex-wrap: wrap;
  gap: 32px;
  padding: 24px;
  background: #cfd4d8;
  min-height: 100%;
}

.bancada-moldura {
  margin: 0;
}

.bancada-moldura figcaption {
  font: 500 11px/1.4 var(--font-system);
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #3b4247;
  padding-bottom: 8px;
}

.bancada-tela {
  overflow: hidden;
  background: var(--canvas);
  border-radius: 12px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
}
```

- [ ] **Step 4: Conferir que compila e que abre**

Run: `npm run build`
Expected: `tsc --noEmit` passa e o `vite build` escreve em `../static`.

Run: `ls ../static`
Expected: **nada de `bancada`** ali — a bancada não entra no binário.

- [ ] **Step 5: Subir o Vite e fotografar**

Run: `npm run dev` (em segundo plano)
Abrir `http://localhost:9131/bancada.html` no navegador e tirar uma foto.
Expected: duas molduras escuras sobre fundo claro, com a frase provisória e "Capturas falsas: 3".

- [ ] **Step 6: Commit**

```bash
git add apps/mos-web/ui/bancada.html apps/mos-web/ui/src/bancada.tsx apps/mos-web/ui/src/falso.ts apps/mos-web/ui/src/estilo.css
git commit -m "chore(mos-web): a bancada, para a tela ser vista antes de entregue

O app de verdade exige passkey, entao conferir aparencia custava a cerimonia
inteira a cada olhada — e o que custa caro se faz pouco. A bancada monta as
telas com dado falso e o CSS real, e nao entra no build: o vite tem uma entrada
so."
```

---

### Task 3: A casca e a barra de cinco

Quebrar `App.tsx` sem mudar comportamento nenhum, e trocar a navegação de quatro abas no topo por cinco destinos embaixo.

**Files:**
- Modify: `apps/mos-web/ui/src/App.tsx` (de 666 linhas para a casca)
- Create: `apps/mos-web/ui/src/paginas/Capturar.tsx`, `Inbox.tsx`, `Tasks.tsx`, `Lembretes.tsx`, `Mais.tsx`
- Create: `apps/mos-web/ui/src/componentes/Barra.tsx`
- Create: `apps/mos-web/ui/src/navegacao.ts`, `ui/src/navegacao.test.ts`

**Interfaces:**
- Consumes: `Marca` (Task 1), `FALSO` (Task 2).
- Produces: `type Pagina = "home" | "capturar" | "inbox" | "tasks" | "lembretes" | "mais"`; `DESTINOS: ReadonlyArray<{ pagina: Pagina; rotulo: string }>` (os cinco da barra, sem `lembretes`); `contagemDe(pagina, dados)`.

- [ ] **Step 1: Write the failing test**

Criar `apps/mos-web/ui/src/navegacao.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { contagemDe, DESTINOS } from "./navegacao";
import { FALSO } from "./falso";

describe("a barra de baixo", () => {
  it("tem cinco destinos, e lembretes nao e um deles", () => {
    expect(DESTINOS.map((d) => d.pagina)).toEqual([
      "home",
      "capturar",
      "inbox",
      "tasks",
      "mais",
    ]);
  });

  it("conta a inbox e as tasks abertas, e nao as feitas", () => {
    // A task "done" do banco falso nao entra: um badge que sobe com coisa
    // resolvida e um badge que se aprende a ignorar.
    expect(contagemDe("inbox", FALSO)).toBe(3);
    expect(contagemDe("tasks", FALSO)).toBe(2);
  });

  it("poe em `mais` o que cobra acao — o lembrete vencido", () => {
    expect(contagemDe("mais", FALSO)).toBe(1);
  });

  it("nao conta nada em home nem em capturar", () => {
    expect(contagemDe("home", FALSO)).toBe(0);
    expect(contagemDe("capturar", FALSO)).toBe(0);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- navegacao`
Expected: FAIL — `Failed to resolve import "./navegacao"`.

- [ ] **Step 3: Implement**

Criar `apps/mos-web/ui/src/navegacao.ts`:

```ts
import { pedeAtencao, type Capture, type Lembrete, type Task } from "./api";

export type Pagina = "home" | "capturar" | "inbox" | "tasks" | "lembretes" | "mais";

/**
 * Os cinco destinos da barra de baixo.
 *
 * Lembretes NAO esta aqui, e a ausencia e a decisao: ele e destino de
 * notificacao — chega-se nele pelo aviso que tocou, ou pelo cartao da Home —,
 * e a barra guarda os cinco alvos que o polegar procura sem motivo externo.
 */
export const DESTINOS: ReadonlyArray<{ pagina: Pagina; rotulo: string }> = [
  { pagina: "home", rotulo: "Home" },
  { pagina: "capturar", rotulo: "Capturar" },
  { pagina: "inbox", rotulo: "Inbox" },
  { pagina: "tasks", rotulo: "Tasks" },
  { pagina: "mais", rotulo: "Mais" },
];

export type Dados = {
  capturas: Capture[];
  tasks: Task[];
  lembretes: Lembrete[];
};

/** O numero do badge. Zero significa "nao desenhe nada". */
export function contagemDe(pagina: Pagina, dados: Dados): number {
  switch (pagina) {
    case "inbox":
      return dados.capturas.length;
    case "tasks":
      return dados.tasks.filter((task) => task.state !== "done").length;
    case "mais":
    case "lembretes":
      // So o que cobra acao. `scheduled` nao entra: um badge que sobe com coisa
      // que ainda nao e hora e um badge que se aprende a ignorar
      // (`ATTENTION-SYSTEM.md` §21.1).
      return dados.lembretes.filter(pedeAtencao).length;
    default:
      return 0;
  }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- navegacao`
Expected: PASS — 4 testes.

- [ ] **Step 5: A barra**

Criar `apps/mos-web/ui/src/componentes/Barra.tsx`:

```tsx
import { contagemDe, DESTINOS, type Dados, type Pagina } from "../navegacao";

/**
 * A barra de baixo.
 *
 * Ela mora no alcance do polegar, e cada alvo tem `--toque` de altura: abaixo
 * de 44px o dedo erra, e errar aqui significa abrir a pagina errada com o
 * aparelho na mao no meio da rua.
 */
export function Barra({
  atual,
  dados,
  aoIr,
}: {
  atual: Pagina;
  dados: Dados;
  aoIr: (pagina: Pagina) => void;
}) {
  return (
    <nav className="barra" aria-label="Seções">
      {DESTINOS.map(({ pagina, rotulo }) => {
        const conta = contagemDe(pagina, dados);
        return (
          <button
            key={pagina}
            type="button"
            aria-current={atual === pagina ? "page" : undefined}
            onClick={() => aoIr(pagina)}
          >
            <span>{rotulo}</span>
            {conta > 0 ? <b className="conta">{conta}</b> : null}
          </button>
        );
      })}
    </nav>
  );
}
```

- [ ] **Step 6: Mover as telas para `paginas/`**

Recortar de `App.tsx`, **sem mudar uma linha do JSX**, para arquivos próprios. Cada página recebe por props o que hoje lê do escopo:

- `Capturar.tsx` — props `{ capturas: Capture[] }`; o bloco `aba === "capturar"`.
- `Inbox.tsx` — props `{ capturas: Capture[] }`; o bloco `aba === "inbox"`.
- `Tasks.tsx` — props `{ tasks: Task[]; tasksLembradas: Set<string>; aoAlternar: (t: Task) => void; aoLembrar: (t: Task, jaTem: boolean) => void }`; o bloco `aba === "tasks"` mais o `SinoIcone` (que se muda junto, por ser usado só aqui).
- `Lembretes.tsx` — props `{ lembretes: Lembrete[]; ocupado: boolean; aoResolver: (l: Lembrete, como: "concluir" | "cancelar") => void }`; a lista do bloco `aba === "lembretes"`, **sem** a seção `.canal`.
- `Mais.tsx` — props `{ estado: EstadoDoAparelho | null; avisos: Situacao | null; ocupado: boolean; aoAtivar: () => void; aoTestar: () => void; aoAbrirLembretes: () => void }`; recebe a seção `.canal` inteira, mais uma linha que leva a Lembretes.

`porQueNaoAtivo` vai junto para `Mais.tsx`; `quando` vai para `Inbox.tsx` e é importado por `Capturar.tsx`.

- [ ] **Step 7: A casca**

Em `App.tsx`: trocar `type Aba` por `Pagina` de `./navegacao`, trocar `<nav className="abas">` por `<Barra …/>`, e o `<main>` passa a escolher a página:

```tsx
<main className="conteudo">
  {pagina === "capturar" ? <Capturar capturas={capturas} /> : null}
  {pagina === "inbox" ? <Inbox capturas={capturas} /> : null}
  {pagina === "tasks" ? (
    <Tasks
      tasks={tasks}
      tasksLembradas={tasksLembradas}
      aoAlternar={(task) => void alternar(task)}
      aoLembrar={(task, jaTem) =>
        setAgendando({
          titulo: task.title,
          descricao: jaTem ? "JÁ HÁ UM LEMBRETE PARA ESTA TASK" : "LEMBRAR DESTA TASK",
          alvo: { type: "task", id: task.id },
        })
      }
    />
  ) : null}
  {pagina === "lembretes" ? (
    <Lembretes
      lembretes={lembretes}
      ocupado={ocupado}
      aoResolver={(lembrete, como) => void resolverLembrete(lembrete, como)}
    />
  ) : null}
  {pagina === "mais" ? (
    <Mais
      estado={estado}
      avisos={avisos}
      ocupado={ocupado}
      aoAtivar={() => void ativarAvisos()}
      aoTestar={() => void testarAvisos()}
      aoAbrirLembretes={() => setPagina("lembretes")}
    />
  ) : null}
</main>
```

O compositor continua na casca, e some em `home` e `mais` — nessas duas não há o que compor:

```tsx
{pagina === "capturar" || pagina === "tasks" || pagina === "lembretes" ? (
  <form className="compositor" onSubmit={/* como hoje */}>…</form>
) : null}
```

- [ ] **Step 8: O CSS da barra**

Mover as regras `.abas`, `.abas button`, `.conta` de `estilo.css` para o novo `telas.css`, renomeando `.abas` para `.barra`, e trocar posição: `position: sticky; bottom: 0;` com `padding-bottom: env(safe-area-inset-bottom)`. Importar `telas.css` em `main.tsx` e em `bancada.tsx`, depois de `estilo.css`.

- [ ] **Step 9: Conferir comportamento e aparência**

Run: `npm test`
Expected: todos passam, incluindo `instantes.test.ts`.

Run: `npm run build`
Expected: `tsc --noEmit` limpo.

Na bancada, trocar a `Moldura` provisória por `<Inbox capturas={FALSO.capturas} />` e `<Tasks …/>` com `FALSO`, e fotografar as duas larguras.
Expected: as listas aparecem iguais às de hoje, com a barra embaixo.

- [ ] **Step 10: Commit**

```bash
git add apps/mos-web/ui/src
git commit -m "refactor(mos-web): a casca, as paginas e a barra de cinco

App.tsx tinha 666 linhas e fazia navegacao, estado, quatro telas e a folha de
agendamento. Cada pagina vira arquivo, e a decisao de badge vira funcao pura com
teste — era JSX antes, e JSX nao se testa sem montar o app inteiro.

A barra desce para o alcance do polegar e ganha Home e Mais. Lembretes sai dela:
ele e destino de notificacao, e nao de varredura."
```

---

### Task 4: A Home

**Files:**
- Create: `apps/mos-web/ui/src/paginas/home.ts`, `home.test.ts`, `Home.tsx`
- Create: `apps/mos-web/ui/src/componentes/Cartao.tsx`
- Modify: `apps/mos-web/ui/src/App.tsx` (rota `home`, que passa a ser a inicial)

**Interfaces:**
- Consumes: `Dados`, `Pagina` (Task 3); `EstadoDoAparelho` (`api.ts`).
- Produces: `cartoesDaHome(estado, dados, agora): CartaoDaHome[]`, onde `type CartaoDaHome = { chave: string; rotulo: string; numero: string; legenda: string; destino: Pagina; urgente?: boolean }`.

- [ ] **Step 1: Write the failing test**

Criar `apps/mos-web/ui/src/paginas/home.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { cartoesDaHome } from "./home";
import { FALSO } from "../falso";

const AGORA = new Date("2026-09-02T14:30:00Z");

describe("os cartoes da Home", () => {
  it("mostra a fila do sync, a inbox, as tasks abertas e o que vence", () => {
    const chaves = cartoesDaHome(FALSO.estado, FALSO, AGORA).map((c) => c.chave);
    expect(chaves).toEqual(["sync", "hoje", "inbox", "tasks", "ultima"]);
  });

  it("conta como task aberta o que nao esta feito", () => {
    const tasks = cartoesDaHome(FALSO.estado, FALSO, AGORA).find((c) => c.chave === "tasks");
    expect(tasks?.numero).toBe("2");
    expect(tasks?.destino).toBe("tasks");
  });

  it("marca urgente o cartao de hoje quando ha lembrete cobrando", () => {
    const hoje = cartoesDaHome(FALSO.estado, FALSO, AGORA).find((c) => c.chave === "hoje");
    expect(hoje?.urgente).toBe(true);
  });

  // Cartao vazio prometendo conteudo e pior que a ausencia dele: ele ensina que
  // a Home tem lugares que nunca dizem nada.
  it("omite o cartao que nao tem o que dizer", () => {
    const vazio = { capturas: [], tasks: [], lembretes: [] };
    const chaves = cartoesDaHome(
      { ...FALSO.estado, pendentes: 0 },
      vazio,
      AGORA,
    ).map((c) => c.chave);
    expect(chaves).toEqual(["sync"]);
  });

  it("diz EM DIA quando nao ha nada na fila", () => {
    const sync = cartoesDaHome({ ...FALSO.estado, pendentes: 0 }, FALSO, AGORA)
      .find((c) => c.chave === "sync");
    expect(sync?.numero).toBe("EM DIA");
  });

  it("avisa quando o aparelho nao tem hub", () => {
    const sync = cartoesDaHome({ ...FALSO.estado, sincroniza: false }, FALSO, AGORA)
      .find((c) => c.chave === "sync");
    expect(sync?.numero).toBe("SEM HUB");
    expect(sync?.urgente).toBe(true);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- home`
Expected: FAIL — `Failed to resolve import "./home"`.

- [ ] **Step 3: Implement**

Criar `apps/mos-web/ui/src/paginas/home.ts`:

```ts
import { pedeAtencao, type EstadoDoAparelho } from "../api";
import type { Dados, Pagina } from "../navegacao";

export type CartaoDaHome = {
  chave: string;
  rotulo: string;
  /** O conteudo do cartao. Texto, e nao numero: "EM DIA" e uma resposta. */
  numero: string;
  legenda: string;
  destino: Pagina;
  urgente?: boolean;
};

/**
 * O que a Home mostra, e em que ordem.
 *
 * Regra que vale para todos: **cartao sem o que dizer nao aparece**. Um cartao
 * vazio prometendo conteudo ensina que a Home tem lugares que nunca dizem nada,
 * e depois disso ela deixa de ser lida.
 *
 * O sync e a excecao, e de proposito: "em dia" e informacao — e a resposta a
 * pergunta que se faz ao abrir o app na rua.
 */
export function cartoesDaHome(
  estado: EstadoDoAparelho | null,
  dados: Dados,
  agora: Date = new Date(),
): CartaoDaHome[] {
  const cartoes: CartaoDaHome[] = [];

  const semHub = estado?.sincroniza === false;
  const pendentes = estado?.pendentes ?? 0;
  cartoes.push({
    chave: "sync",
    rotulo: "SYNC",
    numero: semHub ? "SEM HUB" : pendentes > 0 ? String(pendentes) : "EM DIA",
    legenda: semHub
      ? "este aparelho não alcança o hub"
      : pendentes > 0
        ? "esperando para subir"
        : "tudo o que você escreveu já atravessou",
    destino: "mais",
    urgente: semHub || undefined,
  });

  const cobrando = dados.lembretes.filter(pedeAtencao);
  const proximos = dados.lembretes.filter(
    (l) => l.status === "scheduled" && l.nextDueAt !== null && new Date(l.nextDueAt) > agora,
  );
  if (cobrando.length > 0 || proximos.length > 0) {
    cartoes.push({
      chave: "hoje",
      rotulo: "HOJE",
      numero: String(cobrando.length + proximos.length),
      legenda:
        cobrando.length > 0
          ? `${cobrando.length} cobrando agora`
          : "agendados, nenhum vencido",
      destino: "lembretes",
      urgente: cobrando.length > 0 || undefined,
    });
  }

  if (dados.capturas.length > 0) {
    cartoes.push({
      chave: "inbox",
      rotulo: "INBOX",
      numero: String(dados.capturas.length),
      legenda: dados.capturas.length === 1 ? "captura esperando" : "capturas esperando",
      destino: "inbox",
    });
  }

  const abertas = dados.tasks.filter((task) => task.state !== "done");
  if (abertas.length > 0) {
    const andando = abertas.filter((task) => task.state === "doing").length;
    cartoes.push({
      chave: "tasks",
      rotulo: "TASKS",
      numero: String(abertas.length),
      legenda: andando > 0 ? `${andando} em andamento` : "abertas",
      destino: "tasks",
    });
  }

  const ultima = dados.capturas[0];
  if (ultima) {
    cartoes.push({
      chave: "ultima",
      rotulo: "ÚLTIMA CAPTURA",
      numero: ultima.content,
      legenda: "",
      destino: "inbox",
    });
  }

  return cartoes;
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- home`
Expected: PASS — 6 testes.

- [ ] **Step 5: O cartão e a página**

Criar `apps/mos-web/ui/src/componentes/Cartao.tsx`:

```tsx
/**
 * O cartao da Home.
 *
 * O numero e o conteudo; o rotulo e legenda. Por isso ele vem primeiro na
 * ordem visual e maior — quem abre o app quer o numero, e le o resto so se o
 * numero surpreender.
 *
 * `largo` e o cartao de texto (a ultima captura), que ocupa a linha inteira: um
 * texto de duas linhas espremido em meia coluna vira tres linhas cortadas.
 */
export function Cartao({
  rotulo,
  numero,
  legenda,
  urgente,
  largo,
  aoTocar,
}: {
  rotulo: string;
  numero: string;
  legenda: string;
  urgente?: boolean;
  largo?: boolean;
  aoTocar: () => void;
}) {
  return (
    <button
      type="button"
      className="cartao"
      data-urgente={urgente || undefined}
      data-largo={largo || undefined}
      onClick={aoTocar}
    >
      <span className="cartao-rotulo">{rotulo}</span>
      <span className="cartao-numero">{numero}</span>
      {legenda ? <span className="cartao-legenda">{legenda}</span> : null}
    </button>
  );
}
```

Criar `apps/mos-web/ui/src/paginas/Home.tsx`:

```tsx
import { Cartao } from "../componentes/Cartao";
import type { EstadoDoAparelho } from "../api";
import type { Dados, Pagina } from "../navegacao";
import { cartoesDaHome } from "./home";

export function Home({
  estado,
  dados,
  aoIr,
}: {
  estado: EstadoDoAparelho | null;
  dados: Dados;
  aoIr: (pagina: Pagina) => void;
}) {
  const cartoes = cartoesDaHome(estado, dados);
  return (
    <div className="home">
      {cartoes.map((cartao) => (
        <Cartao
          key={cartao.chave}
          rotulo={cartao.rotulo}
          numero={cartao.numero}
          legenda={cartao.legenda}
          urgente={cartao.urgente}
          largo={cartao.chave === "ultima"}
          aoTocar={() => aoIr(cartao.destino)}
        />
      ))}
    </div>
  );
}
```

- [ ] **Step 6: Ligar na casca**

Em `App.tsx`: `const [pagina, setPagina] = useState<Pagina>("home")` — a Home passa a ser a tela de abertura — e acrescentar ao `<main>`:

```tsx
{pagina === "home" ? (
  <Home estado={estado} dados={{ capturas, tasks, lembretes }} aoIr={setPagina} />
) : null}
```

- [ ] **Step 7: O CSS**

Em `telas.css`:

```css
/* A HOME.
   Duas colunas: o polegar alcança as duas sem reposicionar o aparelho, e três
   deixariam o número pequeno demais para ser lido de relance. */
.home {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-3);
  padding: var(--space-4) var(--space-4) var(--space-5);
}

.cartao {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  min-height: 104px;
  padding: var(--space-3);
  text-align: left;
  background: var(--surface-raised);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.4);
  color: inherit;
  font: inherit;
  cursor: pointer;
}

.cartao:active {
  background: var(--surface-active);
}

.cartao[data-largo] {
  grid-column: 1 / -1;
  min-height: 0;
}

.cartao[data-urgente] {
  border-color: var(--signal);
}

.cartao-rotulo {
  font: 500 11px/1.4 var(--font-system);
  letter-spacing: 0.08em;
  color: var(--text-system);
}

.cartao-numero {
  font: 500 32px/1.1 var(--font-product);
  font-variant-numeric: tabular-nums;
  letter-spacing: -0.02em;
}

/* O cartão de texto não usa 32px: a última captura é uma frase, e uma frase
   nesse corpo vira três linhas e empurra o resto da Home para fora da tela. */
.cartao[data-largo] .cartao-numero {
  font-size: 15px;
  font-weight: 400;
  line-height: 1.45;
  letter-spacing: -0.008em;
}

.cartao-legenda {
  font-size: 13px;
  color: var(--text-secondary);
}
```

- [ ] **Step 8: Fotografar**

Na bancada, montar `<Home estado={FALSO.estado} dados={FALSO} aoIr={() => {}} />` nas duas larguras e fotografar.
Expected: cinco cartões, o de sync mostrando `3`, o de hoje com borda sódio, e a última captura ocupando a linha inteira sem cortar palavra.

- [ ] **Step 9: Commit**

```bash
git add apps/mos-web/ui/src
git commit -m "feat(mos-web): a Home, que responde antes de perguntarem

O app abria no compositor e nunca dizia como estao as coisas. Agora abre num
panorama do que ja existe — fila do sync, o que vence, inbox, tasks abertas e a
ultima captura.

Cartao sem o que dizer nao aparece. O sync e a excecao: em dia e informacao, e e
a pergunta que se faz ao abrir o app na rua."
```

---

### Task 5: A pele

**Files:**
- Modify: `apps/mos-web/ui/src/estilo.css` (tokens novos)
- Modify: `apps/mos-web/ui/src/telas.css` (listas, topo, vazios)
- Modify: `apps/mos-web/ui/src/componentes/Vazio.tsx` (criado aqui), e as páginas que usam `.vazio`

**Interfaces:**
- Produces: `<Vazio frase={string} acao?: { rotulo: string; aoTocar: () => void } />`.

- [ ] **Step 1: Tokens**

Em `estilo.css`, dentro de `:root`, acrescentar:

```css
  /* Elevação, em dois níveis e não cinco: o app tem cartão e folha, e uma
     escala maior que a quantidade de superfícies vira decisão sem critério. */
  --elev-1: 0 1px 2px rgba(0, 0, 0, 0.4);
  --elev-2: 0 8px 28px rgba(0, 0, 0, 0.55);

  /* O único gradiente do app, no topo da Home. 6% é o limite em que ele lê
     como luz e não como cor: acima disso o sódio vira fundo, e o sódio é o
     acento — a única coisa com direito de puxar o olho. */
  --brilho-topo: linear-gradient(180deg, rgba(231, 194, 78, 0.06), transparent 120px);
```

- [ ] **Step 2: O topo e o brilho**

Em `telas.css`:

```css
/* O topo carrega a marca de verdade — a barra, não a palavra. O texto "M/OS"
   fica ao lado dela em mono, do mesmo jeito que o desktop faz no trilho. */
.topo {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
}

.topo .marca-barra {
  color: var(--signal);
}

.home::before {
  content: "";
  position: absolute;
  inset: 0 0 auto 0;
  height: 120px;
  background: var(--brilho-topo);
  pointer-events: none;
}

.conteudo {
  position: relative;
}
```

- [ ] **Step 3: O estado vazio**

Criar `apps/mos-web/ui/src/componentes/Vazio.tsx`:

```tsx
/**
 * O estado vazio.
 *
 * Ele diz o que a tela FARIA, e nao que ela esta vazia. "Inbox vazia" descreve
 * o pixel; "o que estiver na cabeca vai para a Inbox" descreve o proposito — e
 * so a segunda ensina alguma coisa a quem abriu o app pela primeira vez.
 */
export function Vazio({
  frase,
  acao,
}: {
  frase: string;
  acao?: { rotulo: string; aoTocar: () => void };
}) {
  return (
    <div className="vazio">
      <p>{frase}</p>
      {acao ? (
        <button type="button" className="botao" data-variante="quieto" onClick={acao.aoTocar}>
          {acao.rotulo}
        </button>
      ) : null}
    </div>
  );
}
```

Trocar os quatro `<p className="vazio">…</p>` das páginas por `<Vazio …/>`, com as frases:

- Capturar: `"O que estiver na cabeça vai para a Inbox. Organizar é depois."`
- Inbox: `"Nada esperando. O que você capturar aparece aqui."` + ação `"Capturar agora"` que vai para `capturar`.
- Tasks: `"Nenhuma task aberta."` + ação `"Criar uma task"` que vai para `tasks` com foco no compositor.
- Lembretes: `"Nenhum lembrete esperando. Escreva embaixo, ou toque no sino de uma Task."`

- [ ] **Step 4: A lista, com a barra como marcador**

Em `telas.css`, acrescentar ao `.item`:

```css
/* A barra reaparece como marcador do item que cobra ação — é o mesmo papel que
   ela tem nas listas do desktop. Um filete, e não um fundo colorido: o fundo
   competiria com o cartão urgente da Home. */
.item[data-cobra]::before {
  content: "";
  position: absolute;
  left: 0;
  top: 50%;
  width: 3px;
  height: 20px;
  transform: translateY(-50%) skewX(-14deg);
  background: var(--signal);
}

.item {
  position: relative;
}
```

- [ ] **Step 5: Fotografar antes e depois**

Montar na bancada as seis páginas com `FALSO`, nas duas larguras, e fotografar.
Expected: cartões com borda e elevação visíveis, número de 32px dominando o cartão, o filete sódio no lembrete vencido, e o brilho do topo perceptível sem virar cor de fundo.

- [ ] **Step 6: Commit**

```bash
git add apps/mos-web/ui/src
git commit -m "feat(mos-web): a pele — camada, numero grande e o vazio com proposito

Mesma paleta e mesma tipografia; a riqueza vem de camada. O estado vazio deixa
de descrever o pixel (inbox vazia) e passa a dizer o que a tela faz — que e a
unica frase util para quem abriu o app pela primeira vez.

A barra reaparece como marcador do item que cobra acao, como no desktop."
```

---

### Task 6: O movimento

**Files:**
- Modify: `apps/mos-web/ui/src/telas.css`
- Modify: `apps/mos-web/ui/src/App.tsx` (a chave de animação da troca de página)
- Modify: `apps/mos-web/ui/src/componentes/Marca.tsx` (nada de código — só confirmar o `data-girando`)

- [ ] **Step 1: As transições**

Em `telas.css`:

```css
/* O MOVIMENTO.
   Uma regra manda em todas as outras, e vem do `BRIEF-SISTEMA-DE-LOGOS.md`: a
   barra dá meia-volta, e é o único spinner do sistema. Não existe círculo
   girando, não existem três pontos. */

@keyframes meia-volta {
  from { transform: rotate(0deg); }
  to { transform: rotate(180deg); }
}

.marca-barra[data-girando] {
  animation: meia-volta 600ms cubic-bezier(0.65, 0, 0.35, 1) infinite;
  transform-origin: 50% 50%;
}

@keyframes entrar-pagina {
  from { opacity: 0; transform: translateX(12px); }
  to { opacity: 1; transform: none; }
}

.conteudo > * {
  animation: entrar-pagina 140ms ease-out;
}

@keyframes entrar-item {
  from { opacity: 0; transform: translateY(6px); }
  to { opacity: 1; transform: none; }
}

/* Escalonado, e no máximo seis: além disso a última linha entra depois de a
   pessoa já ter começado a ler a primeira, e o efeito vira espera. */
.lista .item {
  animation: entrar-item 160ms ease-out backwards;
}
.lista .item:nth-child(1) { animation-delay: 0ms; }
.lista .item:nth-child(2) { animation-delay: 30ms; }
.lista .item:nth-child(3) { animation-delay: 60ms; }
.lista .item:nth-child(4) { animation-delay: 90ms; }
.lista .item:nth-child(5) { animation-delay: 120ms; }
.lista .item:nth-child(6) { animation-delay: 150ms; }
.lista .item:nth-child(n + 7) { animation-delay: 150ms; }

/* Concluir: risca, esmaece e recolhe. A linha some por um caminho que explica
   o que aconteceu — sumir de um quadro para o outro parece perda de dado. */
.item[data-saindo] {
  animation: sair-item 200ms ease-in forwards;
}

@keyframes sair-item {
  from { opacity: 1; }
  to { opacity: 0; transform: translateX(-8px); }
}

/* Quem liga esta opção costuma ligá-la por enjoo de movimento. Corte seco em
   tudo — inclusive no spinner, que vira um símbolo parado. */
@media (prefers-reduced-motion: reduce) {
  .marca-barra[data-girando],
  .conteudo > *,
  .lista .item,
  .item[data-saindo] {
    animation: none;
  }
}
```

- [ ] **Step 2: A chave de animação**

Em `App.tsx`, dar ao `<main>` uma `key` que muda com a página, para o React remontar e a animação de entrada rodar a cada troca:

```tsx
<main className="conteudo" key={pagina}>
```

- [ ] **Step 3: O spinner onde ele faz sentido**

Trocar, no topo, o texto de fila por `<Marca tamanho={16} girando={ocupado} />` ao lado do rótulo — a barra gira só enquanto uma ação está em curso, e para quando ela termina.

- [ ] **Step 4: Conferir os dois estados**

Fotografar a bancada duas vezes: uma normal, outra com movimento reduzido (`prefers-reduced-motion: reduce` — no Chrome, via emulação em DevTools, ou `Emulate CSS media feature`).
Expected: layout idêntico nos dois; nada de animação no segundo.

- [ ] **Step 5: Commit**

```bash
git add apps/mos-web/ui/src
git commit -m "feat(mos-web): o movimento, com a barra como unico spinner

O brief e taxativo, e isso podou metade das escolhas: a barra da meia-volta, e
nao existe circulo girando nem tres pontos. Troca de pagina desliza, lista entra
escalonada em no maximo seis, item concluido risca e recolhe.

Tudo morre dentro de prefers-reduced-motion. Nao e acessibilidade decorativa:
quem liga essa opcao costuma liga-la por enjoo de movimento."
```

---

### Task 7: A fronteira escrita, e o fechamento

**Files:**
- Modify: `apps/mos-web/README.md`
- Rebuild: `apps/mos-web/static/` (gerada, e gitignored)

- [ ] **Step 1: Atualizar o README**

Em `apps/mos-web/README.md`, na seção "O que ele é", trocar o parágrafo da fronteira por:

```markdown
Uma **porta** para o M/OS, e não um segundo M/OS. Capturar, ver a inbox, mexer
em tasks, receber lembretes — e um panorama do que está acontecendo. O trabalho
de verdade continua no desktop, com o CAD aberto.

O critério que separa os dois é esse: o que se faz com o desenho na frente fica
no PC. Ver quanto se trabalhou, o que vence hoje e o que caiu na inbox é
justamente o que se pergunta longe da mesa — e por isso entra aqui.

Essa fronteira está escrita porque é ela que costuma escapar: o desktop expõe
280 comandos, e crescer até lá seria uma decisão, não uma deriva.
```

E na tabela "Estado", acrescentar a linha:

```markdown
| Marca, Home e movimento (entrega 1 da pele) | pronto |
```

- [ ] **Step 2: Rodar tudo**

Run: `npm test`
Expected: todos os arquivos de teste passam.

Run: `npm run build`
Expected: `tsc --noEmit` limpo, saída escrita em `../static`.

Run (da raiz): `cargo test -p mos-web`
Expected: os testes de servidor continuam passando — nenhum deles depende do HTML, mas o `rust-embed` lê `static/` em tempo de compilação, e um build quebrado apareceria aqui.

- [ ] **Step 3: Fotografar as seis páginas, uma última vez**

Na bancada, as seis páginas × duas larguras, e o ícone (abrir `public/icone-512.png` com Read).
Expected: nada de layout quebrado em 390px; o ícone é a barra em campo sódio.

- [ ] **Step 4: Commit**

```bash
git add apps/mos-web
git commit -m "docs(mos-web): a fronteira nova, escrita onde a antiga estava

O README dizia porta e listava quatro coisas. A entrega da pele acrescentou
panorama, e uma fronteira que muda sem o texto mudar junto e uma fronteira que
ninguem sabe onde esta.

O criterio ficou escrito: o que se faz com o desenho na frente fica no PC."
```

---

## Self-Review

**Cobertura do spec:**

| Seção do spec | Task |
| --- | --- |
| 1. A marca (ícone, ângulo por escala, maskable) | 1 |
| 2. A carta de navegação (barra de 5, Mais como índice, sem router) | 3 |
| 3. A Home (cinco cartões com o dado que já existe) | 4 |
| 4. A pele (cartão, gradiente, números, rótulos, nada de hover) | 4 e 5 |
| 5. O movimento (cinco momentos, reduced-motion) | 6 |
| 6. O código (paginas/, componentes/, dois CSS, sem router) | 3 |
| 7. Como é conferido (vitest + bancada + fotos) | 2, e uma foto ao fim de cada task |
| Fronteira do README | 7 |

**Nomes usados entre tasks:** `poligonoPara`/`pontosDoPoligono` (Task 1) → usados por `Marca` (Task 1) e pelo topo (Task 6). `Pagina`, `DESTINOS`, `contagemDe`, `Dados` (Task 3) → usados por `Barra` (3), `Home`/`cartoesDaHome` (4). `FALSO` (Task 2) → usado pelos testes de 3 e 4 e por todas as fotos. `CartaoDaHome` (4) → consumido só por `Home.tsx`.

**O que este plano NÃO faz:** rota nova no servidor, horas do CronoCAD, acadêmico, diário, escrita nova, tema claro, router, biblioteca de animação. Tudo isso está nas entregas 2 a 4 do spec.
