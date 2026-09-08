import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import type { ParsedReminder } from "./types";

/**
 * Um campo, uma frase, Enter.
 *
 * `Ctrl+Shift+R` abre; a pessoa digita *"Enviar as bases para o Victor hoje
 * 20:30 e não me deixa esquecer"*; Enter cria e a janela some. É o §13 do
 * pedido: captura em segundos, sem formulário.
 *
 * # Por que a leitura da frase acontece no backend
 *
 * O parser é determinístico e vive no domínio (`resolve_when`, o mesmo que a voz
 * usa desde antes disto). Reimplementá-lo em TypeScript daria dois
 * entendedores de "sexta à tarde" no mesmo sistema, e dois entendedores
 * discordam — provavelmente na virada da noite, quando ninguém está olhando.
 *
 * # O que a pessoa vê antes de confirmar
 *
 * O instante resolvido, sempre. Um lembrete que dispara em hora diferente da
 * que se achou ter escolhido é pior que um lembrete que não dispara: o primeiro
 * ensina a não confiar. A prévia aparece enquanto se digita e o trecho que virou
 * hora fica dito por extenso ao lado.
 *
 * Sem tempo na frase, o lembrete nasce **sem data** — e isso é legítimo, não é
 * falha. "Comprar cabo HDMI" vai para Algum dia, e continua existindo.
 */
export function QuickReminder() {
  const [texto, setTexto] = useState("");
  const [lido, setLido] = useState<ParsedReminder | null>(null);
  const [estado, setEstado] = useState<"idle" | "saving" | "error">("idle");
  const [erro, setErro] = useState("");
  const campo = useRef<HTMLInputElement>(null);

  useEffect(() => {
    campo.current?.focus();
    const unlisten = listen("window-revealed", () => {
      // A janela é a mesma sempre: sem isto, reabrir traria o texto de ontem.
      setTexto("");
      setLido(null);
      setErro("");
      setEstado("idle");
      campo.current?.focus();
    });
    return () => {
      void unlisten.then((dispose) => dispose());
    };
  }, []);

  /* A leitura acompanha a digitação, com um respiro.
   *
   * O respiro não é economia de rede — é um comando local. Ele existe para a
   * prévia não piscar a cada tecla enquanto alguém escreve "amanhã": as formas
   * intermediárias resolvem para coisas diferentes, e ver a data dançar embaixo
   * do campo é pior que esperar 120 ms para vê-la certa. */
  useEffect(() => {
    if (!texto.trim()) {
      setLido(null);
      return;
    }
    const timer = window.setTimeout(() => {
      void api
        .parseReminder(texto)
        .then(setLido)
        .catch(() => setLido(null));
    }, 120);
    return () => window.clearTimeout(timer);
  }, [texto]);

  async function salvar(evento: React.FormEvent) {
    evento.preventDefault();
    const titulo = lido?.title?.trim() || texto.trim();
    if (!titulo || estado === "saving") return;

    setEstado("saving");
    try {
      await api.novoLembrete({
        title: titulo,
        at: lido?.at ? new Date(lido.at) : null,
        persistent: lido?.persistent ?? false,
      });
      setTexto("");
      setLido(null);
      setEstado("idle");
      await api.hideQuickReminder();
    } catch (falha) {
      setEstado("error");
      setErro(`${(falha as Error).message ?? String(falha)} O texto continua aqui.`);
    }
  }

  const quando = lido?.at ? new Date(lido.at) : null;

  return (
    <main className="quick-shell">
      <form className="quick-reminder" onSubmit={salvar}>
        <div className="capture-line">
          <span className="capture-bar" aria-hidden="true" />
          <input
            aria-label="Me lembre de"
            onChange={(evento) => {
              setTexto(evento.currentTarget.value);
              if (estado !== "idle") {
                setEstado("idle");
                setErro("");
              }
            }}
            onKeyDown={(evento) => {
              if (evento.key === "Escape") void api.hideQuickReminder();
            }}
            placeholder="Me lembre de..."
            ref={campo}
            value={texto}
          />
        </div>

        {/* A prévia. Ela é o contrato: o que está escrito aqui é o que vai ser
            criado, e nada além disso. */}
        <div aria-live="polite" className="quick-reminder-preview">
          {texto.trim() ? (
            <>
              <strong>{lido?.title || texto.trim()}</strong>
              <span className="attention-when">
                {quando ? porExtenso(quando) : "sem data · vai para Algum dia"}
              </span>
              {lido?.whenText ? (
                <span className="quick-reminder-echo">entendi “{lido.whenText}”</span>
              ) : null}
              {lido?.persistent ? (
                <span className="attention-mark" data-reason="persistent">
                  vou continuar cobrando até você resolver
                </span>
              ) : null}
            </>
          ) : (
            <span className="micro-label">⏎ CRIA · ESC FECHA</span>
          )}
        </div>

        {estado === "error" ? (
          <p className="inline-error" role="alert">
            ! {erro}
          </p>
        ) : null}
      </form>
    </main>
  );
}

function porExtenso(quando: Date): string {
  return quando.toLocaleString("pt-BR", {
    weekday: "short",
    day: "2-digit",
    month: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}
