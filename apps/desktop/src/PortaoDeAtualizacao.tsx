import { useEffect, useRef, useState, type ReactNode } from "react";
import { api, appError } from "./api";
import { Button } from "./Button";
import {
  AVISO_ANTES_DE_REINICIAR_S,
  comPrazo,
  esquecerFalhas,
  FALHAS_ANTES_DE_LIBERAR,
  INTERVALO_COM_APP_ABERTO_MS,
  liberadaPorFalhas,
  podeReiniciar,
  PRAZO_DA_ABERTURA_MS,
  registrarFalha,
  rotuloDoProgresso,
} from "./regraDaAtualizacaoObrigatoria";
import type { UpdateProgress } from "./types";

type Fase =
  | { tipo: "conferindo" }
  | { tipo: "livre"; aviso: string | null }
  | { tipo: "contagem"; versao: string; segundos: number }
  | { tipo: "instalando"; versao: string; progresso: UpdateProgress }
  | { tipo: "falhou"; versao: string; motivo: string }
  | { tipo: "sem-reiniciar"; versao: string };

function armazenamento(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

/** Ninguém conferindo em `tauri dev`: o build de desenvolvimento seria "trocado" pelo release. */
const ATIVO = !import.meta.env.DEV;

/**
 * A porta da janela principal: só deixa entrar na versão mais nova (ADR-076).
 * As regras — e as três exceções — moram em `regraDaAtualizacaoObrigatoria.ts`.
 */
export function PortaoDeAtualizacao({ children }: { children: ReactNode }) {
  const [fase, setFase] = useState<Fase>(ATIVO ? { tipo: "conferindo" } : { tipo: "livre", aviso: null });
  const pendente = useRef<string | null>(null);

  async function instalar(versao: string) {
    setFase({ tipo: "instalando", versao, progresso: { downloaded: 0, total: null } });
    void api.registrarOcorrencia("info", "atualizacao", `obrigatoria: instalando ${versao}`);
    try {
      const fim = await api.installUpdate((progresso) =>
        setFase({ tipo: "instalando", versao, progresso }),
      );
      esquecerFalhas(armazenamento());
      // No Windows o instalador encerra o M/OS e nem se chega aqui. Se chegou,
      // a versão nova está no disco e falta abrir de novo.
      if (fim === "instalada") setFase({ tipo: "sem-reiniciar", versao });
    } catch (erro) {
      const motivo = appError(erro).message;
      const vezes = registrarFalha(versao, armazenamento());
      void api.registrarOcorrencia("erro", "atualizacao", `obrigatoria: ${versao} falhou (${vezes}x): ${motivo}`);
      setFase(
        vezes >= FALHAS_ANTES_DE_LIBERAR
          ? {
              tipo: "livre",
              aviso: `Não consegui instalar a versão ${versao}. O M/OS abriu na versão atual e tenta de novo quando sair outra.`,
            }
          : { tipo: "falhou", versao, motivo },
      );
    }
  }

  /** A versão a instalar, ou `null` — em dia, sem resposta, ou liberada por falhas. */
  async function conferir(prazo: number | null): Promise<string | null> {
    const consulta = api.checkForUpdate().catch(() => null);
    const info = prazo === null ? await consulta : await comPrazo(consulta, prazo);
    if (!info) return null;
    return liberadaPorFalhas(info.version, armazenamento()) ? null : info.version;
  }

  // Ao abrir: confere antes de mostrar qualquer coisa.
  useEffect(() => {
    if (!ATIVO) return;
    let vivo = true;
    void conferir(PRAZO_DA_ABERTURA_MS).then((versao) => {
      if (!vivo) return;
      if (versao) void instalar(versao);
      else setFase({ tipo: "livre", aviso: null });
    });
    return () => {
      vivo = false;
    };
  }, []);

  // Aberto: confere de tempos em tempos e, achando, espera não haver gravação.
  useEffect(() => {
    if (!ATIVO) return;
    const conferencia = setInterval(() => {
      void conferir(null).then((versao) => {
        if (versao) pendente.current = versao;
      });
    }, INTERVALO_COM_APP_ABERTO_MS);
    const vigia = setInterval(() => {
      const versao = pendente.current;
      if (!versao) return;
      void Promise.all([
        api.meetingRecording().catch(() => null),
        api.voiceRecording().catch(() => null),
      ]).then(([reuniao, voz]) => {
        if (!podeReiniciar({ reuniao, voz })) return;
        pendente.current = null;
        setFase((atual) =>
          atual.tipo === "livre" ? { tipo: "contagem", versao, segundos: AVISO_ANTES_DE_REINICIAR_S } : atual,
        );
      });
    }, 60_000);
    return () => {
      clearInterval(conferencia);
      clearInterval(vigia);
    };
  }, []);

  // A contagem regressiva antes do reinício.
  useEffect(() => {
    if (fase.tipo !== "contagem") return;
    if (fase.segundos <= 0) {
      void instalar(fase.versao);
      return;
    }
    const passo = setTimeout(() => setFase({ ...fase, segundos: fase.segundos - 1 }), 1_000);
    return () => clearTimeout(passo);
  }, [fase]);

  if (fase.tipo === "livre") {
    return (
      <>
        {children}
        {fase.aviso ? (
          <div className="portao-aviso" role="status">
            <span>{fase.aviso}</span>
            <Button variant="ghost" onClick={() => setFase({ tipo: "livre", aviso: null })}>
              Ok
            </Button>
          </div>
        ) : null}
      </>
    );
  }

  return (
    <div className="portao-atualizacao" role="alertdialog" aria-live="polite">
      <div className="portao-cartao">
        <p className="micro-label">M/OS</p>
        {fase.tipo === "conferindo" ? <h1>Conferindo se há versão nova…</h1> : null}
        {fase.tipo === "contagem" ? (
          <>
            <h1>A versão {fase.versao} está pronta</h1>
            <p className="support-copy">
              O M/OS reinicia em <span className="tabular">{fase.segundos}s</span> para atualizar. O que
              está salvo continua salvo; timers seguem rodando.
            </p>
            <div className="button-line">
              <Button variant="primary" onClick={() => void instalar(fase.versao)}>
                Atualizar agora
              </Button>
            </div>
          </>
        ) : null}
        {fase.tipo === "instalando" ? (
          <>
            <h1>Atualizando para a versão {fase.versao}</h1>
            <p className="support-copy tabular">{rotuloDoProgresso(fase.progresso)}</p>
            <div className="portao-barra" aria-hidden="true">
              <span
                style={{
                  width: fase.progresso.total
                    ? `${Math.min(100, (fase.progresso.downloaded / fase.progresso.total) * 100)}%`
                    : "30%",
                }}
              />
            </div>
            <p className="support-copy">O M/OS reabre sozinho quando terminar.</p>
          </>
        ) : null}
        {fase.tipo === "falhou" ? (
          <>
            <h1>A atualização para a {fase.versao} não terminou</h1>
            <p className="support-copy">{fase.motivo}</p>
            <div className="button-line">
              <Button variant="primary" onClick={() => void instalar(fase.versao)}>
                Tentar de novo
              </Button>
            </div>
          </>
        ) : null}
        {fase.tipo === "sem-reiniciar" ? (
          <>
            <h1>Versão {fase.versao} instalada</h1>
            <p className="support-copy">Feche e abra o M/OS para começar a usar.</p>
          </>
        ) : null}
      </div>
    </div>
  );
}
