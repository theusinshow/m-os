import { useCallback, useEffect, useState } from "react";
import { api, appError } from "./api";
import { Button } from "./Button";
import { Panel, StateMessage } from "./Surface";
import { relativeTime } from "./relativeTime";
import {
  deveConferirSozinho,
  linhaDaVerificacao,
  linhaDaVersao,
  rotulo,
  situacao,
  type EstadoDaAtualizacao,
} from "./atualizacao";
import type { UpdateInfo, UpdateProgress } from "./types";

/**
 * O painel de atualizações.
 *
 * # O que ele conserta
 *
 * Ele respondia "estou atualizado?" apenas nos segundos seguintes ao clique. O
 * resultado morava em `useState` dentro do Settings, e sair da página apagava a
 * prova de que a verificação tinha acontecido. Duas consequências, e as duas
 * geravam a queixa de que a atualização "às vezes não funciona":
 *
 * 1. **Não havia indicador.** Para saber se estava em dia era preciso entrar em
 *    Settings, achar o painel, clicar e ficar olhando. Fora dessa janela de
 *    segundos, o M/OS não sabia dizer.
 * 2. **Falhar e estar em dia tinham a mesma cara: nenhuma.** Uma verificação que
 *    caiu por falta de rede deixava a tela igual a uma que deu certo. O app
 *    parecia ter conferido quando não tinha.
 *
 * Agora o resultado é gravado pelo Rust (`atualizacao.rs`), o painel o lê ao
 * montar, e o M/OS confere sozinho quando a última resposta está velha. O selo
 * diz qual das cinco situações é a atual — ver `atualizacao.ts`.
 *
 * # E o caderno
 *
 * Toda verificação e toda instalação deixam uma linha no `diagnostico`, inclusive
 * as que dão certo. Um caderno que só guarda falha responde "o que quebrou?" e
 * não responde "ele chegou a tentar?" — e a segunda é a pergunta de quem diz que
 * às vezes não funciona.
 */
export function AtualizacaoPanel({ verificarAoAbrir }: { verificarAoAbrir: boolean }) {
  const [estado, setEstado] = useState<EstadoDaAtualizacao | null>(null);
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [fase, setFase] = useState<"parado" | "verificando" | "instalando">("parado");
  /** O recado da última ação, ao lado do botão que a provocou. */
  const [recado, setRecado] = useState("");
  const [recadoRuim, setRecadoRuim] = useState(false);
  const [progresso, setProgresso] = useState<UpdateProgress>({ downloaded: 0, total: null });

  const ler = useCallback(async () => {
    const proximo = await api.updateStatus().catch(() => null);
    setEstado(proximo);
    return proximo;
  }, []);

  const verificar = useCallback(async () => {
    setFase("verificando");
    setRecado("");
    setProgresso({ downloaded: 0, total: null });
    try {
      const achado = await api.checkForUpdate();
      setInfo(achado);
      // A anotação vem antes da leitura de propósito: ler primeiro devolveria o
      // estado anterior, e a tela piscaria o resultado velho antes do novo.
      await api.noteUpdateCheck(achado?.version ?? "", achado?.date ?? "");
      void api.registrarOcorrencia(
        "info",
        "atualizacao",
        achado ? `verificacao encontrou ${achado.version}` : "verificacao nao encontrou versao nova",
      );
    } catch (erro) {
      const motivo = appError(erro).message;
      setInfo(null);
      await api.noteUpdateFailure(motivo).catch(() => undefined);
      void api.registrarOcorrencia("erro", "atualizacao", `verificacao falhou: ${motivo}`);
    }
    await ler();
    setFase("parado");
  }, [ler]);

  useEffect(() => {
    void (async () => {
      const inicial = await ler();
      // Confere sozinho quando a última resposta está velha. Sem isto o selo
      // mostraria o passado: um indicador que só se move quando alguém entra
      // aqui e clica não é um indicador, é um histórico.
      if (deveConferirSozinho(inicial)) void verificar();
    })();
  }, [ler, verificar]);

  useEffect(() => {
    if (verificarAoAbrir) void verificar();
    // A intenção vinda da Search dispara uma vez, na chegada.
  }, [verificarAoAbrir]);

  async function instalar() {
    setFase("instalando");
    setRecado("");
    void api.registrarOcorrencia("info", "atualizacao", `instalando ${info?.version ?? "?"}`);
    try {
      const fim = await api.installUpdate(setProgresso);
      // Instalada mas sem reiniciar não é erro: a versão nova está no disco, e o
      // que falta é um passo do usuário. Dizer "reiniciando" quando o reinício
      // não aconteceu deixa a pessoa esperando por uma janela que não volta.
      setRecadoRuim(false);
      setRecado(
        fim === "reiniciando"
          ? "Atualização instalada. Reiniciando M/OS…"
          : "Atualização instalada. Feche e abra o M/OS para usar a versão nova.",
      );
      void api.registrarOcorrencia("info", "atualizacao", `instalacao terminou: ${fim}`);
      if (fim === "instalada") setInfo(null);
    } catch (erro) {
      const motivo = appError(erro).message;
      setRecadoRuim(true);
      setRecado(motivo);
      void api.registrarOcorrencia("erro", "atualizacao", `instalacao falhou: ${motivo}`);
    }
    setFase("parado");
  }

  const qual = situacao(estado, fase !== "parado");
  const trabalhando = fase !== "parado";

  /** Uma linha só, sempre no mesmo lugar: o progresso enquanto baixa, o recado
   *  nos demais estados. */
  function statusDaAcao(): string | null {
    if (recado) return recado;
    if (fase !== "instalando") return null;
    if (!progresso.total) return "Baixando pacote de atualização…";
    const parte = Math.min(100, Math.round((progresso.downloaded / progresso.total) * 100));
    return `Baixando atualização: ${parte}%`;
  }

  const acao = statusDaAcao();

  return (
    <Panel label="ATUALIZAÇÕES">
      <div className="setting-row">
        <div>
          <strong>Atualizar M/OS</strong>

          {/* O SELO E AS DUAS LINHAS.
              Eles ficam sempre na tela, e não só depois de um clique: a pergunta
              "estou atualizado?" tem resposta o tempo todo, e um painel que só
              responde enquanto você olha para ele não responde. */}
          <p className="update-badge" data-situacao={qual}>
            <i aria-hidden="true" />
            {rotulo(qual)}
          </p>
          <p className="tabular">{linhaDaVersao(estado)}</p>
          {trabalhando ? null : (
            <p className="support-copy">{linhaDaVerificacao(estado, relativeTime)}</p>
          )}

          {/* As notas da versão nova, quando o release traz alguma. */}
          {info?.body ? <p className="support-copy">{info.body}</p> : null}

          {/* De onde ele busca. Fica visível SEMPRE que a verificação falhou,
              porque a primeira pergunta útil depois de "não consegui" é "falar
              com quem?" — e a resposta costuma explicar sozinha (rede da empresa,
              GitHub fora, release publicado sem o latest.json). */}
          {qual === "sem-resposta" && estado?.endpoint ? (
            <p className="support-copy tabular">{estado.endpoint}</p>
          ) : null}

          {acao ? (
            <StateMessage
              state={recadoRuim ? "error" : trabalhando ? "loading" : "saved"}
              label={acao}
            />
          ) : null}
        </div>

        <div className="button-line">
          <Button variant="secondary" onClick={() => void verificar()} disabled={trabalhando}>
            {fase === "verificando" ? "Verificando" : "Verificar agora"}
          </Button>
          {/* O botão aparece por FATO, e não por estado: existe uma atualização
              encontrada, então existe o que instalar. Ele aparecia por estado, e
              um erro no meio do download o fazia sumir — a única saída era
              verificar de novo para chegar ao mesmo lugar. */}
          {info ? (
            <Button variant="primary" onClick={() => void instalar()} disabled={trabalhando}>
              {fase === "instalando" ? "Instalando" : recadoRuim ? "Tentar de novo" : "Atualizar agora"}
            </Button>
          ) : null}
        </div>
      </div>
    </Panel>
  );
}
