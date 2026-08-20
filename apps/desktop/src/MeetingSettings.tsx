import { useCallback, useEffect, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { Panel, StateMessage } from "./Surface";
import type { AnalysisConsent, MonitoringSettings, TranscriberStatus } from "./types";

/**
 * As duas configurações do Meeting Agent.
 *
 * Elas moram juntas por serem os dois pontos onde a pessoa decide o que sai da
 * máquina: o transcritor decide se o ÁUDIO precisa sair (não precisa — ele é
 * local), e o consentimento decide se a TRANSCRIÇÃO sai.
 */
export function MeetingSettings() {
  const [transcriber, setTranscriber] = useState<TranscriberStatus | null>(null);
  const [consent, setConsent] = useState<AnalysisConsent | null>(null);
  /* As configurações de observação vivem em `tracking_settings`, junto do
     monitoramento de processos — a detecção de reunião é a mesma família. */
  const [observacao, setObservacao] = useState<MonitoringSettings | null>(null);
  const [binary, setBinary] = useState("");
  const [model, setModel] = useState("");
  const [threads, setThreads] = useState("0");
  const [vadModel, setVadModel] = useState("");
  const [note, setNote] = useState("");
  const [saved, setSaved] = useState(false);

  const load = useCallback(async () => {
    try {
      const [status, granted, monitoramento] = await Promise.all([
        api.meetingTranscriberStatus(),
        api.meetingAnalysisConsent(),
        api.monitoringSettings(),
      ]);
      setTranscriber(status);
      setConsent(granted);
      setObservacao(monitoramento);
      setBinary(status.binary);
      setModel(status.model);
      setThreads(String(status.threads));
      setVadModel(status.vadModel);
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    }
  }, []);

  useEffect(() => { void load(); }, [load]);

  const save = async () => {
    setNote("");
    setSaved(false);
    try {
      const status = await api.meetingSetTranscriber(binary, model, Number(threads) || 0, vadModel);
      setTranscriber(status);
      setSaved(true);
      window.setTimeout(() => setSaved(false), 4000);
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <>
      <Panel label="TRANSCRIÇÃO LOCAL">
        <p className="support-copy">
          A transcrição roda nesta máquina, por um binário do <code>whisper.cpp</code> que
          você aponta aqui. O áudio da reunião <b>não sai do computador</b> em
          nenhum momento.
        </p>
        <p className="support-copy">
          O binário fica de fora do M/OS de propósito: trocar uma build de CPU por
          uma de GPU passa a ser trocar um caminho, e não recompilar o aplicativo.
        </p>

        {transcriber ? (
          <StateMessage
            state={transcriber.ready ? "saved" : "error"}
            label={transcriber.ready ? `Pronto · ${transcriber.name}` : "Não está pronto"}
            detail={transcriber.ready ? undefined : transcriber.problem}
          />
        ) : null}

        <label className="meeting-field">
          <span>Executável</span>
          <input
            value={binary}
            placeholder="C:\\whisper\\whisper-cli.exe"
            onChange={(event) => setBinary(event.target.value)}
          />
        </label>

        <label className="meeting-field">
          <span>Modelo</span>
          <input
            value={model}
            placeholder="C:\\whisper\\ggml-large-v3-turbo-q5_0.bin"
            onChange={(event) => setModel(event.target.value)}
          />
        </label>
        <p className="support-copy">
          O modelo precisa ser <b>multilíngue</b>: as reuniões são em português, e as
          variantes <code>.en</code> não servem.
        </p>

        <label className="meeting-field">
          <span>Modelo de VAD (opcional)</span>
          <input
            value={vadModel}
            onChange={(event) => setVadModel(event.target.value)}
            placeholder="C:\Dev\whisper\ggml-silero-v5.1.2.bin"
          />
        </label>
        <p className="support-copy">
          O VAD faz o transcritor <b>não ver o silêncio</b>, que é onde nascem as
          repetições em laço. Vazio, a transcrição funciona como antes — sem VAD, e
          não com erro.
        </p>

        <label className="meeting-field">
          <span>Threads</span>
          <input
            type="number"
            min={0}
            max={64}
            value={threads}
            onChange={(event) => setThreads(event.target.value)}
          />
        </label>
        <p className="support-copy">
          Zero deixa o binário decidir. Mais threads é mais rápido e ocupa mais a
          máquina enquanto processa.
        </p>

        <div className="form-actions">
          <Button variant="primary" onClick={() => void save()}>Aplicar</Button>
        </div>
        {saved ? <StateMessage state="saved" label="Transcritor atualizado" /> : null}
      </Panel>

      <Panel label="ANÁLISE COM O HERMES">
        <p className="support-copy">
          Depois de transcrever, o M/OS pode enviar a <b>transcrição</b> ao Hermes
          para extrair resumo, decisões e compromissos. O áudio nunca é enviado.
        </p>
        <p className="support-copy">
          A autorização é dada uma vez, e não a cada reunião — confirmação repetida
          ensina a clicar sem ler. Desligando aqui, as reuniões param depois da
          transcrição e nada mais sai da máquina.
        </p>

        <div className="setting-row">
          <div>
            <strong>Oferecer gravação quando uma reunião começa</strong>
            {/* Este parágrafo não é decoração: é o que a pessoa precisa para
                decidir, e ele diz o que a feature NÃO faz. Sem ele o toggle pede
                confiança em vez de informar. */}
            <p>
              O M/OS observa qual programa abriu o microfone — nunca o título da janela,
              o conteúdo da tela ou o áudio.
            </p>
          </div>
          <label className="switch">
            <input
              aria-label="Oferecer gravação quando uma reunião começa"
              type="checkbox"
              checked={Boolean(observacao?.meetingDetectionEnabled)}
              disabled={!observacao}
              onChange={(event) => {
                if (!observacao) return;
                const proxima = { ...observacao, meetingDetectionEnabled: event.currentTarget.checked };
                void api.monitoringSetSettings(proxima)
                  .then(setObservacao)
                  .catch((error) => setNote(String(error)));
              }}
            />
            <span />
          </label>
        </div>

        <div className="setting-row">
          <div>
            <strong>Enviar transcrição ao Hermes</strong>
            <p>
              {consent?.granted
                ? `Autorizado em ${new Date(consent.grantedAt).toLocaleDateString("pt-BR")}.`
                : "Ainda não autorizado. A primeira gravação vai perguntar."}
            </p>
          </div>
          <label className="switch">
            <input
              aria-label="Enviar transcrição ao Hermes"
              type="checkbox"
              checked={Boolean(consent?.granted)}
              onChange={(event) => {
                void api.meetingSetAnalysisConsent(event.currentTarget.checked)
                  .then(setConsent)
                  .catch((error) => setNote(String(error)));
              }}
            />
            <span />
          </label>
        </div>
      </Panel>

      {note ? <StateMessage state="error" label="Não foi possível salvar" detail={note} /> : null}
    </>
  );
}
