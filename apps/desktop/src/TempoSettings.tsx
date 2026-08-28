import { useCallback, useEffect, useState } from "react";
import { api } from "./api";
import { Button } from "./Button";
import { EmptyState, Region } from "./Surface";
import type { Issuer, MonitoredApp, MonitoringSettings, SilencedApp, TrackingSettings } from "./types";

/** `3h20` de silêncio restante — a pergunta é "até quando", não "até que hora". */
function leftOf(minutes: number) {
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return hours ? `${hours}h${String(rest).padStart(2, "0")}` : `${rest}min`;
}

const MODES: { value: TrackingSettings["rounding"]["mode"]; label: string }[] = [
  { value: "nearest", label: "mais próximo" },
  { value: "up", label: "para cima" },
  { value: "down", label: "para baixo" },
];

/**
 * Arredondamento e inatividade.
 *
 * O aviso na tela não é decoração: o arredondamento **não** altera o que está
 * gravado. O banco guarda sempre o tempo real, e isto muda só o que se cobra.
 * Sem dizer isso, mudar de 15 para 30 minutos pareceria reescrever o passado.
 */
export function TempoSettings({ onChanged }: { onChanged?: () => void }) {
  const [settings, setSettings] = useState<TrackingSettings | null>(null);
  const [apps, setApps] = useState<MonitoredApp[]>([]);
  const [issuer, setIssuer] = useState<Issuer | null>(null);
  const [watch, setWatch] = useState<MonitoringSettings | null>(null);
  const [silenced, setSilenced] = useState<SilencedApp[]>([]);
  const [process, setProcess] = useState("");
  const [label, setLabel] = useState("");
  const [note, setNote] = useState("");

  const load = useCallback(async () => {
    const [next, list, who, observation, quiet] = await Promise.all([
      api.trackingSettings().catch(() => null),
      api.monitoredApps().catch(() => [] as MonitoredApp[]),
      api.trackingIssuer().catch(() => null),
      api.monitoringSettings().catch(() => null),
      api.reminderSilenced().catch(() => [] as SilencedApp[]),
    ]);
    setSettings(next);
    setApps(list);
    setIssuer(who);
    setWatch(observation);
    setSilenced(quiet);
  }, []);

  useEffect(() => { void load(); }, [load]);

  async function guard(run: () => Promise<unknown>) {
    setNote("");
    try {
      await run();
      await load();
      onChanged?.();
    } catch (error) {
      setNote(error instanceof Error ? error.message : String(error));
    }
  }

  function save(next: TrackingSettings) {
    setSettings(next);
    void guard(() => api.trackingSetSettings(next));
  }

  function observe(next: MonitoringSettings) {
    setWatch(next);
    void guard(() => api.monitoringSetSettings(next));
  }

  return (
    <>
      <Region label="ARREDONDAMENTO">
        {settings ? (
          <div className="tempo-form">
            {/* `tempo-check-governa`: esta caixa LIGA os dois campos ao lado, e
                dividir a linha com eles punha o rotulo dela encostado no
                "Intervalo" e os tres alinhados por baixo, como se fossem
                irmaos. Quem manda vem antes, e sozinho na linha. */}
            <label className="tempo-check tempo-check-governa">
              <input
                type="checkbox"
                checked={settings.rounding.enabled}
                onChange={(event) =>
                  save({ ...settings, rounding: { ...settings.rounding, enabled: event.currentTarget.checked } })
                }
              />
              Arredondar na cobrança
            </label>
            <div className="tempo-field">
              <label htmlFor="rounding-interval">Intervalo (min)</label>
              <input
                id="rounding-interval"
                type="number"
                min={1}
                max={120}
                step={5}
                value={settings.rounding.intervalMinutes}
                disabled={!settings.rounding.enabled}
                onChange={(event) =>
                  save({
                    ...settings,
                    rounding: {
                      ...settings.rounding,
                      intervalMinutes: Math.max(1, Number(event.currentTarget.value) || 1),
                    },
                  })
                }
              />
            </div>
            <div className="tempo-field">
              <label htmlFor="rounding-mode">Modo</label>
              <select
                id="rounding-mode"
                value={settings.rounding.mode}
                disabled={!settings.rounding.enabled}
                onChange={(event) =>
                  save({
                    ...settings,
                    rounding: {
                      ...settings.rounding,
                      mode: event.currentTarget.value as TrackingSettings["rounding"]["mode"],
                    },
                  })
                }
              >
                {MODES.map((mode) => <option key={mode.value} value={mode.value}>{mode.label}</option>)}
              </select>
            </div>
            <p className="support-copy">
              O arredondamento <strong>não</strong> altera o que está gravado. O banco guarda sempre o tempo real —
              isto muda só o número que você cobra, e o arredondamento é aplicado por sessão, não sobre a soma.
            </p>
          </div>
        ) : (
          <EmptyState>Configuração indisponível.</EmptyState>
        )}
      </Region>

      <Region label="OBSERVAÇÃO">
        {/* Dito de forma verificável, e não como promessa de marketing: o que
            é lido são nomes de executável e o tempo desde o último toque. Sem
            título de janela, sem conteúdo de arquivo, sem captura de tela. */}
        <p className="support-copy">
          O M/OS lê <strong>nomes de programas em execução</strong> e há quanto tempo você não toca no teclado
          ou no mouse. Nada além disso — sem título de janela, sem conteúdo de arquivo, sem captura de tela. E
          nenhuma hora é registrada sozinha: o que é observado vira sugestão na Linha do Tempo, e quem decide
          é você.
        </p>
        {watch ? (
          <div className="tempo-form">
            <label className="tempo-check">
              <input
                type="checkbox"
                checked={watch.processMonitoringEnabled}
                onChange={(event) => observe({ ...watch, processMonitoringEnabled: event.currentTarget.checked })}
              />
              Detectar programas abertos
            </label>
            <label className="tempo-check">
              <input
                type="checkbox"
                checked={watch.idleDetectionEnabled}
                onChange={(event) => observe({ ...watch, idleDetectionEnabled: event.currentTarget.checked })}
              />
              Detectar inatividade
            </label>
            <label className="tempo-check">
              <input
                type="checkbox"
                checked={watch.remindOnOpen}
                disabled={!watch.processMonitoringEnabled}
                onChange={(event) => observe({ ...watch, remindOnOpen: event.currentTarget.checked })}
              />
              Lembrar ao abrir o CAD sem cronômetro
            </label>
            <label className="tempo-check">
              <input
                type="checkbox"
                checked={watch.remindOnClose}
                disabled={!watch.processMonitoringEnabled}
                onChange={(event) => observe({ ...watch, remindOnClose: event.currentTarget.checked })}
              />
              Lembrar ao fechar o CAD com cronômetro rodando
            </label>
            <div className="tempo-field">
              <label htmlFor="idle-threshold">Parado a partir de (min)</label>
              <input
                id="idle-threshold"
                type="number"
                min={1}
                max={120}
                value={watch.idleThresholdMinutes}
                disabled={!watch.idleDetectionEnabled}
                onChange={(event) =>
                  observe({ ...watch, idleThresholdMinutes: Math.max(1, Number(event.currentTarget.value) || 1) })
                }
              />
            </div>
            <div className="tempo-field">
              <label htmlFor="check-interval">Verificar a cada (s)</label>
              <input
                id="check-interval"
                type="number"
                min={1}
                max={120}
                value={watch.checkIntervalSeconds}
                disabled={!watch.processMonitoringEnabled}
                onChange={(event) =>
                  observe({ ...watch, checkIntervalSeconds: Math.max(1, Number(event.currentTarget.value) || 1) })
                }
              />
            </div>
          </div>
        ) : (
          <EmptyState>Configuração indisponível.</EmptyState>
        )}
      </Region>

      <Region label="EMISSOR DA FATURA">
        {/* Só a fatura lê isto. Dito na tela para o campo não parecer um cadastro
            de perfil que vai aparecer em outro lugar. */}
        <p className="support-copy">
          Sai no cabeçalho do PDF de fatura, e em nenhum outro lugar. Em branco, a fatura sai sem identificar
          quem deve receber.
        </p>
        {issuer ? (
          <form
            className="tempo-form"
            onSubmit={(event) => {
              event.preventDefault();
              void guard(() => api.trackingSetIssuer(issuer));
            }}
          >
            <div className="tempo-field">
              <label htmlFor="issuer-name">Nome</label>
              <input
                id="issuer-name"
                value={issuer.name}
                onChange={(event) => setIssuer({ ...issuer, name: event.currentTarget.value })}
              />
            </div>
            <div className="tempo-field">
              <label htmlFor="issuer-document">CPF / CNPJ</label>
              <input
                id="issuer-document"
                value={issuer.document}
                onChange={(event) => setIssuer({ ...issuer, document: event.currentTarget.value })}
              />
            </div>
            <div className="tempo-field">
              <label htmlFor="issuer-contact">Contato</label>
              <input
                id="issuer-contact"
                value={issuer.contact}
                placeholder="e-mail ou telefone"
                onChange={(event) => setIssuer({ ...issuer, contact: event.currentTarget.value })}
              />
            </div>
            <Button variant="secondary" size="sm" type="submit">Salvar</Button>
          </form>
        ) : (
          <EmptyState>Configuração indisponível.</EmptyState>
        )}
      </Region>

      <Region label="PROGRAMAS MONITORADOS" count={apps.length ? String(apps.length) : undefined}>
        {/* Observar não é cronometrar: o programa aberto vira um evento na Linha
            do Tempo, e continua sendo você quem decide se aquilo foi trabalho. */}
        <p className="support-copy">
          Abrir um destes vira evento na Linha do Tempo. Nenhuma hora é registrada sozinha.
        </p>
        {apps.length ? (
          <div className="tempo-sessions">
            {apps.map((entry) => {
              // "Não lembrar hoje" é decidido na janelinha que aparece sobre o
              // CAD, longe daqui. Se o estado não aparecesse nesta lista, seria
              // um modo que se liga num lugar e não se desliga em lugar nenhum
              // — e semanas depois o usuário concluiria que o lembrete quebrou.
              const quiet = silenced.find(
                (item) => item.processName.toLowerCase() === entry.processName.toLowerCase(),
              );
              return (
              <div className="tempo-session" key={entry.id}>
                <span>
                  <strong>{entry.displayName}</strong>
                  <small>{entry.processName}{entry.enabled ? "" : " · desativado"}</small>
                </span>
                {quiet ? (
                  <span className="tempo-flag" title={`Volta a lembrar em ${leftOf(quiet.minutesLeft)}.`}>
                    Silenciado
                  </span>
                ) : null}
                <span className="tempo-session-actions" data-always={quiet ? "" : undefined}>
                  {quiet ? (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => void guard(() => api.reminderUnsilence(entry.processName))}
                    >
                      Voltar a lembrar
                    </Button>
                  ) : null}
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => void guard(() => api.saveMonitoredApp({ ...entry, enabled: !entry.enabled }))}
                  >
                    {entry.enabled ? "Desativar" : "Ativar"}
                  </Button>
                  <Button variant="ghost" size="sm" onClick={() => void guard(() => api.deleteMonitoredApp(entry.id))}>
                    Remover
                  </Button>
                </span>
              </div>
              );
            })}
          </div>
        ) : (
          <EmptyState>Nenhum programa monitorado.</EmptyState>
        )}

        <form
          className="tempo-form"
          onSubmit={(event) => {
            event.preventDefault();
            void guard(async () => {
              await api.saveMonitoredApp({
                id: crypto.randomUUID(),
                displayName: label.trim() || process.trim(),
                processName: process.trim(),
                enabled: true,
                remindOnOpen: true,
                remindOnClose: true,
              });
              setProcess("");
              setLabel("");
            });
          }}
        >
          <div className="tempo-field">
            <label htmlFor="app-label">Nome</label>
            <input id="app-label" value={label} placeholder="Revit" onChange={(event) => setLabel(event.currentTarget.value)} />
          </div>
          <div className="tempo-field">
            <label htmlFor="app-process">Executável</label>
            <input
              id="app-process"
              value={process}
              placeholder="revit.exe"
              onChange={(event) => setProcess(event.currentTarget.value)}
            />
          </div>
          <Button variant="secondary" size="sm" type="submit" disabled={!process.trim()}>Adicionar</Button>
        </form>
        {note ? <p className="support-copy" aria-live="polite">{note}</p> : null}
      </Region>
    </>
  );
}
