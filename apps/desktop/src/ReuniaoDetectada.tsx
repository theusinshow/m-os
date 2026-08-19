import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import { Button } from "./Button";

type Detectada = { processo: string; nome: string };

/**
 * A oferta que aparece quando um microfone abre.
 *
 * Ela **não rouba o foco** — quem está entrando numa reunião está clicando em
 * "entrar", não aqui, e capturar o teclado nesse instante é um acidente.
 *
 * E ela **não diz "IA"**. O que se inicia é uma GRAVAÇÃO; a análise vem depois,
 * por botão separado e com consentimento próprio. O botão do Notion diz "Iniciar
 * Anotações IA" e promete na hora errada.
 */
export function ReuniaoDetectada() {
  const [alvo, setAlvo] = useState<Detectada | null>(null);
  const [erro, setErro] = useState("");

  useEffect(() => {
    const off = listen<Detectada>("reuniao-detectada", (evento) => {
      setAlvo(evento.payload);
      setErro("");
    });
    return () => { void off.then((fn) => fn()); };
  }, []);

  async function agir(run: () => Promise<unknown>) {
    try {
      await run();
      await api.fecharReuniaoDetectada();
    } catch (causa) {
      // O erro fica AQUI. Mandar procurar o motivo no M/OS desfaz o motivo de a
      // janelinha existir.
      setErro(causa instanceof Error ? causa.message : String(causa));
    }
  }

  if (!alvo) return null;

  return (
    <main className="oferta-shell">
      <header className="oferta-head">
        <span className="micro-label">M/OS · REUNIÕES</span>
        <strong>{alvo.nome} abriu o microfone</strong>
      </header>

      {erro ? <p className="support-copy" role="alert">{erro}</p> : null}

      <div className="oferta-acoes">
        <Button variant="primary" size="sm" onClick={() => void agir(() => api.meetingStart("", null))}>
          Gravar reunião
        </Button>
        <Button variant="ghost" size="sm" onClick={() => void agir(async () => undefined)}>
          Agora não
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => void agir(() => api.silenciarDeteccao(alvo.processo))}
        >
          Não neste app
        </Button>
      </div>
    </main>
  );
}
