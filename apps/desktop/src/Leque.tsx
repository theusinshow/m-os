import type { CSSProperties } from "react";
import { useCallback, useEffect, useRef, useState } from "react";
import { Icon, type IconName } from "./Icon";
import { posicaoDaPetala, resolverPetalas, type Petala } from "./lequePetalas";
import type { Page, RadialPin, RegisteredApp } from "./types";

/** O raio do arco, em pixels.
 *
 *  Subiu de 96 para 112 quando as pétalas cresceram: a corda entre duas pétalas
 *  vizinhas é `2·R·sen(15°)`, então a 96 sobravam 6px de folga entre discos de
 *  44 — perto demais para o olho separar, e perto demais para o mouse errar sem
 *  consequência. A 112 sobram 14. Continua cabendo acima da âncora em 840×600,
 *  onde o alto do arco fica a 134px da borda. */
const RAIO = 112;

/**
 * O leque — cinco pétalas fixas, no rodapé ao centro.
 *
 * Existe apesar de o Ctrl+K já ser um lançador universal, e a diferença é
 * **evocar contra reconhecer**: o Command exige saber o nome e digitá-lo, o
 * leque é memória muscular. Daí sai a única regra que ele não pode quebrar — as
 * pétalas não se reordenam sozinhas, nunca. Um leque que se reorganiza é um
 * Ctrl+K pior, com menos alcance e sem busca.
 *
 * A âncora é `position: absolute` e NÃO ocupa espaço no fluxo: uma faixa
 * permanente roubaria altura de toda página para servir a um gesto. Em troca, a
 * `.page-surface` ganha `padding-bottom`, senão "sobrepor" viraria "esconder".
 *
 * Este componente DESENHA e não decide: a resolução inteira vem de `lequePetalas.ts`,
 * que é a única cópia da regra.
 */
export function Leque({ pins, workspaceId, apps, onNavegar, onAbrirApp, onAcao, onFixar }: {
  pins: RadialPin[];
  workspaceId: string | null;
  apps: RegisteredApp[];
  onNavegar: (page: Page) => void;
  onAbrirApp: (app: RegisteredApp) => void;
  onAcao: (target: string) => void;
  onFixar: (slot: number) => void;
}) {
  const [aberto, setAberto] = useState(false);
  const raiz = useRef<HTMLDivElement>(null);
  const ancora = useRef<HTMLButtonElement>(null);
  const petalas = resolverPetalas(pins, workspaceId);

  const fechar = useCallback(() => {
    setAberto(false);
    ancora.current?.focus();
  }, []);

  // Esc e clique fora fecham. Só o Esc devolve o foco à âncora: quem fechou com
  // o mouse já está olhando para outro lugar, e roubar o foco de volta seria a
  // interrupção que fechar existe para evitar.
  useEffect(() => {
    if (!aberto) return;
    const tecla = (evento: KeyboardEvent) => { if (evento.key === "Escape") fechar(); };
    const fora = (evento: MouseEvent) => {
      if (!raiz.current?.contains(evento.target as Node)) setAberto(false);
    };
    document.addEventListener("keydown", tecla);
    document.addEventListener("mousedown", fora);
    return () => {
      document.removeEventListener("keydown", tecla);
      document.removeEventListener("mousedown", fora);
    };
  }, [aberto, fechar]);

  function disparar(petala: Petala) {
    setAberto(false);
    if (petala.kind === "pagina") { onNavegar(petala.target as Page); return; }
    if (petala.kind === "acao") { onAcao(petala.target); return; }
    const app = apps.find((candidato) => candidato.id === petala.target);
    // App apagado, arquivado ou sem alvo de abertura não vira erro: o slot passa
    // a pedir o que fixar, que é a única saída que RESOLVE em vez de avisar.
    if (app && app.canOpen) onAbrirApp(app); else onFixar(petala.slot);
  }

  return (
    <div className="leque" ref={raiz} data-aberto={aberto || undefined}>
      <div className="leque-petalas" role="menu" aria-label="Leque" aria-hidden={!aberto}>
        {petalas.map((petala) => {
          const { x, y } = posicaoDaPetala(petala.slot, RAIO);
          return (
            <button
              key={petala.slot}
              type="button"
              role="menuitem"
              className="leque-petala"
              tabIndex={aberto ? 0 : -1}
              style={{ "--x": `${x}px`, "--y": `${y}px`, "--ordem": petala.slot } as CSSProperties}
              aria-label={rotuloDaPetala(petala, apps)}
              onClick={() => disparar(petala)}
            >
              <Icon name={iconeDaPetala(petala)} />
              {/* O rótulo é ELEMENTO, e não o `title` nativo. O `title` do
                  Windows demora ~500ms, aparece onde o sistema quer e ignora o
                  tema — três coisas que num leque de memória muscular atrapalham
                  em vez de ajudar. Este é o mesmo desenho do `.rail-tooltip`, e
                  responde a foco de teclado além do mouse. */}
              <span className="leque-rotulo" aria-hidden="true">{rotuloDaPetala(petala, apps)}</span>
            </button>
          );
        })}
      </div>
      <button
        ref={ancora}
        type="button"
        className="leque-ancora"
        aria-expanded={aberto}
        aria-label={aberto ? "Fechar o leque" : "Abrir o leque"}
        onClick={() => setAberto((estava) => !estava)}
      >
        <Icon name="more" />
      </button>
    </div>
  );
}

/** O ícone é do DESTINO, e não do tipo: quem olha o leque procura "Calendário",
 *  não "uma página". O tipo só decide quando o destino não tem desenho próprio. */
function iconeDaPetala(petala: Petala): IconName {
  if (petala.kind === "app") return "apps";
  if (petala.kind === "acao") return petala.target === "attention_create" ? "attention" : "capture";
  const porPagina: Partial<Record<string, IconName>> = {
    calendario: "calendar",
    finance: "finance",
    reunioes: "meetings",
    tempo: "tempo",
    apps: "apps",
    library: "library",
  };
  return porPagina[petala.target] ?? "more";
}

function rotuloDaPetala(petala: Petala, apps: RegisteredApp[]) {
  if (petala.kind === "app") {
    return apps.find((candidato) => candidato.id === petala.target)?.name ?? "Escolher o que fixar";
  }
  if (petala.kind === "acao") {
    return petala.target === "attention_create" ? "Novo lembrete" : "Quick Capture";
  }
  const nomes: Record<string, string> = {
    calendario: "Calendário",
    finance: "Finance",
    reunioes: "Reuniões",
    tempo: "CronoCAD",
    apps: "Apps",
    library: "Library",
  };
  return nomes[petala.target] ?? petala.target;
}
