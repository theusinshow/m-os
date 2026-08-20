import { useEffect, useRef, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import type { PetalaKind } from "./lequePetalas";
import type { RadialPin, RegisteredApp } from "./types";

/** Os destinos que o leque sabe abrir. Fica AQUI, e não em `lequePetalas.ts`,
 *  porque é vocabulário de interface: o módulo guarda a regra, esta lista o
 *  cardápio. */
const PAGINAS: { target: string; nome: string }[] = [
  { target: "calendario", nome: "Calendário" },
  { target: "finance", nome: "Finance" },
  { target: "reunioes", nome: "Reuniões" },
  { target: "tempo", nome: "CronoCAD" },
  { target: "apps", nome: "Apps" },
  { target: "library", nome: "Library" },
];

const ACOES: { target: string; nome: string }[] = [
  { target: "quick_capture", nome: "Quick Capture" },
  { target: "attention_create", nome: "Novo lembrete" },
];

/**
 * O que fixar num slot.
 *
 * Só troca o CONTEÚDO de um slot; a posição não se mexe, e isso é a feature —
 * mover pétala moveria o alvo debaixo da mão, que é o que o leque existe para
 * não fazer.
 */
export function LequeSeletor({ slot, workspaceId, apps, onGravado, onFechar }: {
  slot: number;
  workspaceId: string | null;
  apps: RegisteredApp[];
  onGravado: (pins: RadialPin[]) => void;
  onFechar: () => void;
}) {
  const [erro, setErro] = useState("");
  const [gravando, setGravando] = useState(false);
  const corpo = useRef<HTMLDivElement>(null);

  // O foco entra no primeiro botão do corpo. Não uso `ref` no `Button` porque
  // ele é função simples e não repassa ref — e envolvê-lo em `forwardRef` seria
  // mexer num componente compartilhado por uma necessidade local desta tela.
  useEffect(() => { corpo.current?.querySelector("button")?.focus(); }, []);
  useEffect(() => {
    const tecla = (evento: KeyboardEvent) => { if (evento.key === "Escape") onFechar(); };
    document.addEventListener("keydown", tecla);
    return () => document.removeEventListener("keydown", tecla);
  }, [onFechar]);

  async function fixar(kind: PetalaKind, target: string) {
    setGravando(true);
    setErro("");
    try {
      onGravado(await api.setRadialPin(workspaceId, { slot, kind, target }));
      onFechar();
    } catch (causa) {
      // O erro fica NO seletor: mandar a pessoa procurar o motivo em outra tela
      // desfaz o motivo de o seletor existir.
      setErro(causa instanceof Error ? causa.message : String(causa));
      setGravando(false);
    }
  }

  async function devolverAoDesenho() {
    setGravando(true);
    setErro("");
    try {
      // APAGA a linha em vez de gravar vazio: é a inversão da 0021, e é o que
      // faz o slot voltar a seguir o padrão em vez de congelar no de hoje.
      onGravado(await api.clearRadialPin(workspaceId, slot));
      onFechar();
    } catch (causa) {
      setErro(causa instanceof Error ? causa.message : String(causa));
      setGravando(false);
    }
  }

  const abriveis = apps.filter((app) => app.lifecycleState === "active" && app.canOpen);

  return (
    <div
      className="meeting-scrim"
      role="dialog"
      aria-modal="true"
      aria-label={"Fixar na posição " + String(slot + 1)}
      onMouseDown={(evento) => { if (evento.target === evento.currentTarget) onFechar(); }}
    >
      <div className="leque-seletor" ref={corpo}>
        <span className="micro-label">POSIÇÃO {slot + 1} DE 5</span>

        <span className="micro-label">PÁGINAS</span>
        <div className="leque-seletor-grade">
          {PAGINAS.map((pagina) => (
            <Button key={pagina.target} variant="outline" size="sm" disabled={gravando}
                    onClick={() => void fixar("pagina", pagina.target)}>{pagina.nome}</Button>
          ))}
        </div>

        <span className="micro-label">AÇÕES</span>
        <div className="leque-seletor-grade">
          {ACOES.map((acao) => (
            <Button key={acao.target} variant="outline" size="sm" disabled={gravando}
                    onClick={() => void fixar("acao", acao.target)}>{acao.nome}</Button>
          ))}
        </div>

        <span className="micro-label">APPS</span>
        <div className="leque-seletor-grade">
          {abriveis.map((app) => (
            <Button key={app.id} variant="outline" size="sm" disabled={gravando}
                    onClick={() => void fixar("app", app.id)}>{app.name}</Button>
          ))}
          {/* Um app sem `canOpen` daria uma pétala que não faz nada quando
              clicada, então ele não entra na lista — e a lista vazia diz por quê
              em vez de simplesmente não aparecer. */}
          {!abriveis.length ? <p className="support-copy">Nenhum app com abertura configurada. Cadastre um alvo em Apps para ele aparecer aqui.</p> : null}
        </div>

        {erro ? <p className="support-copy" role="alert">{erro}</p> : null}

        <div className="form-actions">
          <Button variant="ghost" disabled={gravando} onClick={() => void devolverAoDesenho()}>Voltar ao padrão</Button>
          <Button variant="ghost" disabled={gravando} onClick={onFechar}>Cancelar</Button>
        </div>
      </div>
    </div>
  );
}
