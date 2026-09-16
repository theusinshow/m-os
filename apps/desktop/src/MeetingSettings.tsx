import { useCallback, useEffect, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { Panel, StateMessage } from "./Surface";
import type { AnalysisConsent, AudioRetention, AudioTest, MeetingPreferences, MonitoringSettings, TranscriberStatus } from "./types";

/**
 * Settings → Reuniões.
 *
 * Organizado pelo que a pessoa decide, e não pelo que o código tem: gravar,
 * automatizar, proteger contra gravação esquecida, privacidade e vocabulário.
 * **Nenhum limiar técnico aparece** — tolerância, cooldown e pesos do Recording
 * Guardian são constantes do domínio. O transcritor fica no fim, recolhido: é
 * configuração de uma vez.
 */

function Interruptor({ titulo, descricao, marcado, desabilitado, mudar }: {
  titulo: string;
  descricao: string;
  marcado: boolean;
  desabilitado?: boolean;
  mudar: (valor: boolean) => void;
}) {
  return (
    <div className="setting-row">
      <div>
        <strong>{titulo}</strong>
        <p>{descricao}</p>
      </div>
      <label className="switch">
        <input
          aria-label={titulo}
          type="checkbox"
          checked={marcado}
          disabled={desabilitado}
          onChange={(evento) => mudar(evento.currentTarget.checked)}
        />
        <span />
      </label>
    </div>
  );
}

const ROTULO_DO_GUARDIAN: Record<string, string> = {
  suggested: "vezes sugeriu encerrar",
  continued: "vezes você quis continuar",
  auto_stopped: "gravações encerradas sozinhas",
  trim_applied: "cortes de excesso aplicados",
};

export function MeetingSettings() {
  const [prefs, setPrefs] = useState<MeetingPreferences | null>(null);
  const [transcriber, setTranscriber] = useState<TranscriberStatus | null>(null);
  const [consent, setConsent] = useState<AnalysisConsent | null>(null);
  const [observacao, setObservacao] = useState<MonitoringSettings | null>(null);
  const [teste, setTeste] = useState<AudioTest | null>(null);
  const [testando, setTestando] = useState(false);
  const [vocabulario, setVocabulario] = useState("");
  const [estatisticas, setEstatisticas] = useState<[string, number][]>([]);
  const [binary, setBinary] = useState("");
  const [model, setModel] = useState("");
  const [threads, setThreads] = useState("0");
  const [vadModel, setVadModel] = useState("");
  const [note, setNote] = useState("");
  const [saved, setSaved] = useState("");

  const load = useCallback(async () => {
    try {
      const [preferencias, status, granted, monitoramento, stats] = await Promise.all([
        api.meetingPreferences(),
        api.meetingTranscriberStatus(),
        api.meetingAnalysisConsent(),
        api.monitoringSettings(),
        api.meetingGuardianStats().catch(() => ({ counts: [] as [string, number][] })),
      ]);
      setPrefs(preferencias);
      setVocabulario(preferencias.vocabulary.join("\n"));
      setTranscriber(status);
      setConsent(granted);
      setObservacao(monitoramento);
      setEstatisticas(stats.counts);
      setBinary(status.binary);
      setModel(status.model);
      setThreads(String(status.threads));
      setVadModel(status.vadModel);
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    }
  }, []);

  useEffect(() => { void load(); }, [load]);

  const avisar = (texto: string) => {
    setSaved(texto);
    window.setTimeout(() => setSaved(""), 3000);
  };

  const salvarPrefs = async (proximas: MeetingPreferences) => {
    setPrefs(proximas);
    try {
      setPrefs(await api.meetingSetPreferences(proximas));
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    }
  };

  const testar = async () => {
    setTestando(true);
    setTeste(null);
    setNote("");
    try {
      setTeste(await api.meetingAudioTest());
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    } finally {
      setTestando(false);
    }
  };

  const salvarTranscritor = async () => {
    setNote("");
    try {
      setTranscriber(await api.meetingSetTranscriber(binary, model, Number(threads) || 0, vadModel));
      avisar("Transcritor atualizado");
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    }
  };

  const guardian = prefs?.guardian;
  const numeros = estatisticas.filter(([tipo]) => ROTULO_DO_GUARDIAN[tipo]);

  return (
    <>
      <Panel label="GRAVAÇÃO">
        <Interruptor
          titulo="Oferecer gravação quando uma reunião começa"
          descricao="O M/OS observa qual programa abriu o microfone — nunca o título da janela, o conteúdo da tela ou o áudio."
          marcado={Boolean(observacao?.meetingDetectionEnabled)}
          desabilitado={!observacao}
          mudar={(valor) => {
            if (!observacao) return;
            void api.monitoringSetSettings({ ...observacao, meetingDetectionEnabled: valor })
              .then(setObservacao)
              .catch((error) => setNote(String(error)));
          }}
        />
        <div className="setting-row">
          <div>
            <strong>Teste de áudio</strong>
            <p>Grava três segundos e diz se o microfone e o áudio do computador chegam. Fale algo durante o teste.</p>
            {teste ? (
              <ul className="reuniao-teste">
                <li data-ok={teste.micOk || undefined}>Seu microfone {teste.micOk ? "✓" : "— nenhum som chegou"}</li>
                <li data-ok={teste.systemOk || undefined}>
                  Áudio do computador {teste.systemOk ? (teste.systemSilent ? "✓ (nada tocando agora)" : "✓") : "— não abriu"}
                </li>
              </ul>
            ) : null}
          </div>
          <Button variant="outline" size="sm" disabled={testando} onClick={() => void testar()}>
            {testando ? "Testando…" : "Testar"}
          </Button>
        </div>
        <p className="support-copy">Atalhos: Ctrl+Alt+M inicia (ou marca um momento, se já grava); Ctrl+Alt+Shift+M encerra.</p>
      </Panel>

      <Panel label="AUTOMAÇÃO">
        <Interruptor
          titulo="Processar automaticamente"
          descricao="Ao encerrar, a transcrição e a organização começam sozinhas. Desligado, a reunião espera você pedir."
          marcado={Boolean(prefs?.autoProcess)}
          desabilitado={!prefs}
          mudar={(valor) => prefs && void salvarPrefs({ ...prefs, autoProcess: valor })}
        />
        <Interruptor
          titulo="Organizar com o Hermes"
          descricao={consent?.granted
            ? `A transcrição vai ao Hermes para virar resumo, decisões e tarefas. O áudio nunca sai. Autorizado em ${new Date(consent.grantedAt).toLocaleDateString("pt-BR")}.`
            : "Ainda não autorizado. Sem isto, as reuniões ficam só com a transcrição."}
          marcado={Boolean(consent?.granted)}
          mudar={(valor) => void api.meetingSetAnalysisConsent(valor).then(setConsent).catch((error) => setNote(String(error)))}
        />
      </Panel>

      <Panel label="PROTEÇÃO CONTRA GRAVAÇÃO ESQUECIDA">
        <Interruptor
          titulo="Avisar quando a reunião parecer ter terminado"
          descricao="Quando o programa da chamada larga o microfone e o som some, o M/OS pergunta se pode encerrar. Silêncio sozinho não conta."
          marcado={Boolean(guardian?.warnOnEnd)}
          desabilitado={!prefs}
          mudar={(valor) => prefs && void salvarPrefs({ ...prefs, guardian: { ...prefs.guardian, warnOnEnd: valor } })}
        />
        <Interruptor
          titulo="Encerrar automaticamente com alta confiança"
          descricao="Só quando vários sinais concordam. Antes de encerrar, mostra 20 segundos para você continuar."
          marcado={Boolean(guardian?.autoStop)}
          desabilitado={!prefs}
          mudar={(valor) => prefs && void salvarPrefs({ ...prefs, guardian: { ...prefs.guardian, autoStop: valor } })}
        />
        <Interruptor
          titulo="Avisar sobre gravações anormalmente longas"
          descricao="Depois de duas horas, e só se não houver atividade, pergunta se a reunião ainda está acontecendo."
          marcado={Boolean(guardian?.warnLong)}
          desabilitado={!prefs}
          mudar={(valor) => prefs && void salvarPrefs({ ...prefs, guardian: { ...prefs.guardian, warnLong: valor } })}
        />
        {numeros.length ? (
          <p className="support-copy">
            Até agora: {numeros.map(([tipo, n]) => `${n} ${ROTULO_DO_GUARDIAN[tipo]}`).join(" · ")}.
          </p>
        ) : null}
      </Panel>

      <Panel label="PRIVACIDADE">
        <div className="setting-row">
          <div>
            <strong>Áudio das reuniões</strong>
            <p>A transcrição e o resumo ficam. O áudio é só o insumo — e fica neste computador.</p>
          </div>
          <select
            aria-label="Retenção do áudio"
            value={prefs?.retention ?? "delete_after_processing"}
            disabled={!prefs}
            onChange={(evento) => prefs && void salvarPrefs({ ...prefs, retention: evento.currentTarget.value as AudioRetention })}
          >
            <option value="delete_after_processing">Apagar depois de processar</option>
            <option value="keep_24h">Manter por 24 horas</option>
            <option value="keep">Manter</option>
          </select>
        </div>
      </Panel>

      <Panel label="VOCABULÁRIO">
        <p className="support-copy">
          Nomes, siglas e códigos que a transcrição costuma errar — um por linha. Os nomes dos seus
          Projects já entram sozinhos. O texto original nunca é apagado: a correção aparece ao lado,
          e as incertas ficam marcadas.
        </p>
        <textarea
          className="card-gravacao-notas"
          aria-label="Vocabulário"
          value={vocabulario}
          placeholder={"Criciúma\nNexoDoc\nEE-04"}
          onChange={(evento) => setVocabulario(evento.currentTarget.value)}
        />
        <div className="form-actions">
          <Button
            variant="outline"
            disabled={!prefs}
            onClick={() => {
              if (!prefs) return;
              void salvarPrefs({ ...prefs, vocabulary: vocabulario.split("\n") }).then(() => avisar("Vocabulário salvo"));
            }}
          >Salvar vocabulário</Button>
        </div>
      </Panel>

      <Panel label="TRANSCRIÇÃO LOCAL">
        {transcriber ? (
          <StateMessage
            state={transcriber.ready ? "saved" : "error"}
            label={transcriber.ready ? `Pronto · ${transcriber.name}` : "Não está pronto"}
            detail={transcriber.ready ? undefined : transcriber.problem}
          />
        ) : null}
        <details className="reuniao-avancado">
          <summary>Avançado</summary>
          <p className="support-copy">
            A transcrição roda nesta máquina, por um binário do <code>whisper.cpp</code>. O modelo precisa
            ser multilíngue: as variantes <code>.en</code> não servem para português.
          </p>
          <label className="meeting-field">
            <span>Executável</span>
            <input value={binary} placeholder="C:\\whisper\\whisper-cli.exe" onChange={(e) => setBinary(e.target.value)} />
          </label>
          <label className="meeting-field">
            <span>Modelo</span>
            <input value={model} placeholder="C:\\whisper\\ggml-large-v3-turbo-q5_0.bin" onChange={(e) => setModel(e.target.value)} />
          </label>
          <label className="meeting-field">
            <span>Modelo de VAD (opcional)</span>
            <input value={vadModel} onChange={(e) => setVadModel(e.target.value)} placeholder="C:\whisper\ggml-silero-v5.1.2.bin" />
          </label>
          <label className="meeting-field">
            <span>Threads (0 deixa o binário decidir)</span>
            <input type="number" min={0} max={64} value={threads} onChange={(e) => setThreads(e.target.value)} />
          </label>
          <div className="form-actions">
            <Button variant="primary" onClick={() => void salvarTranscritor()}>Aplicar</Button>
          </div>
        </details>
      </Panel>

      {saved ? <StateMessage state="saved" label={saved} /> : null}
      {note ? <StateMessage state="error" label="Não foi possível salvar" detail={note} /> : null}
    </>
  );
}
