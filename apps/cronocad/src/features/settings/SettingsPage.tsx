import { useState } from "react";
import type { RoundingMode, Settings } from "@/types/domain";
import { useSettingsStore } from "@/stores/settingsStore";
import { PageHeader } from "@/components/ui/PageHeader";
import { Panel, PanelHeader } from "@/components/ui/Panel";
import { Button } from "@/components/ui/Button";
import { Checkbox } from "@/components/ui/Checkbox";
import { Field, Input, Select } from "@/components/ui/Field";
import { MonitoredAppsPanel } from "./MonitoredAppsPanel";

const ROUNDING_INTERVALS = [5, 10, 15, 30];
const ROUNDING_MODES: { value: RoundingMode; label: string }[] = [
  { value: "nearest", label: "Mais proximo" },
  { value: "up", label: "Para cima" },
  { value: "down", label: "Para baixo" },
];

/**
 * Configuracoes (secao 13). Edita a linha unica de configuracoes e gerencia os
 * programas monitorados. Alteracoes de monitoramento/intervalo passam a valer no
 * proximo ciclo do servico de monitoramento (secao 10).
 */
export function SettingsPage() {
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.saveSettings);
  const error = useSettingsStore((s) => s.error);

  const [form, setForm] = useState<Settings | null>(null);
  const [initialized, setInitialized] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  if (!initialized && settings) {
    setForm(settings);
    setInitialized(true);
  }

  if (!form) {
    return (
      <div>
        <PageHeader title="Configuracoes" />
        <Panel className="p-6 text-sm text-text-muted">
          {error ? `Nao foi possivel carregar: ${error}` : "Carregando…"}
        </Panel>
      </div>
    );
  }

  function set<K extends keyof Settings>(key: K, value: Settings[K]) {
    setForm((prev) => (prev ? { ...prev, [key]: value } : prev));
    setSaved(false);
  }

  async function handleSave() {
    if (!form) return;
    setSaving(true);
    setSaveError(null);
    try {
      await saveSettings(form);
      setSaved(true);
    } catch (err) {
      setSaveError(typeof err === "string" ? err : "Falha ao salvar.");
    } finally {
      setSaving(false);
    }
  }

  return (
    <div>
      <PageHeader
        title="Configuracoes"
        description="Monitoramento, inatividade, arredondamento e comportamento."
        action={
          <div className="flex items-center gap-3">
            {saved && <span className="text-xs text-success">Salvo</span>}
            <Button variant="primary" onClick={() => void handleSave()} disabled={saving}>
              {saving ? "Salvando…" : "Salvar"}
            </Button>
          </div>
        }
      />

      <div className="space-y-4">
        <Panel>
          <PanelHeader title="Monitoramento de processos" />
          <div className="space-y-4 p-4">
            <Checkbox
              label="Monitoramento ativo"
              checked={form.processMonitoringEnabled}
              onChange={(v) => set("processMonitoringEnabled", v)}
            />
            <Field
              label="Intervalo de verificacao (segundos)"
              htmlFor="s-interval"
            >
              <Input
                id="s-interval"
                type="number"
                min={1}
                className="max-w-[10rem]"
                value={form.processCheckIntervalSeconds}
                onChange={(e) =>
                  set(
                    "processCheckIntervalSeconds",
                    Math.max(1, Number(e.target.value) || 1),
                  )
                }
              />
            </Field>
            <Checkbox
              label="Avisar ao abrir programa monitorado"
              checked={form.remindWhenMonitoredAppOpens}
              onChange={(v) => set("remindWhenMonitoredAppOpens", v)}
            />
            <Checkbox
              label="Avisar ao fechar programa monitorado"
              checked={form.remindWhenMonitoredAppCloses}
              onChange={(v) => set("remindWhenMonitoredAppCloses", v)}
            />
          </div>
        </Panel>

        <MonitoredAppsPanel />

        <Panel>
          <PanelHeader title="Inatividade" />
          <div className="space-y-4 p-4">
            <Checkbox
              label="Detectar inatividade"
              checked={form.idleDetectionEnabled}
              onChange={(v) => set("idleDetectionEnabled", v)}
            />
            <Field label="Limite de inatividade (minutos)" htmlFor="s-idle">
              <Input
                id="s-idle"
                type="number"
                min={1}
                className="max-w-[10rem]"
                value={form.idleThresholdMinutes}
                onChange={(e) =>
                  set(
                    "idleThresholdMinutes",
                    Math.max(1, Number(e.target.value) || 1),
                  )
                }
              />
            </Field>
            <p className="text-2xs text-text-subtle">
              A acao sobre o periodo inativo sera implementada na Fase 5.
            </p>
          </div>
        </Panel>

        <Panel>
          <PanelHeader title="Arredondamento" />
          <div className="space-y-4 p-4">
            <Checkbox
              label="Arredondar na visualizacao/cobranca"
              checked={form.roundingEnabled}
              onChange={(v) => set("roundingEnabled", v)}
            />
            <div className="grid gap-3 sm:grid-cols-2">
              <Field label="Intervalo (minutos)" htmlFor="s-round-int">
                <Select
                  id="s-round-int"
                  value={form.roundingIntervalMinutes}
                  onChange={(e) =>
                    set("roundingIntervalMinutes", Number(e.target.value))
                  }
                >
                  {ROUNDING_INTERVALS.map((m) => (
                    <option key={m} value={m}>
                      {m} min
                    </option>
                  ))}
                </Select>
              </Field>
              <Field label="Modo" htmlFor="s-round-mode">
                <Select
                  id="s-round-mode"
                  value={form.roundingMode}
                  onChange={(e) =>
                    set("roundingMode", e.target.value as RoundingMode)
                  }
                >
                  {ROUNDING_MODES.map((m) => (
                    <option key={m.value} value={m.value}>
                      {m.label}
                    </option>
                  ))}
                </Select>
              </Field>
            </div>
            <p className="text-2xs text-text-subtle">
              O banco preserva sempre o tempo real; o arredondamento afeta apenas
              a exibicao e o calculo de cobranca.
            </p>
          </div>
        </Panel>

        <Panel>
          <PanelHeader title="Emissor (para faturas)" />
          <div className="space-y-4 p-4">
            <Field label="Nome / empresa" htmlFor="s-issuer-name">
              <Input
                id="s-issuer-name"
                value={form.issuerName}
                onChange={(e) => set("issuerName", e.target.value)}
                placeholder="Seu nome ou razao social"
              />
            </Field>
            <div className="grid gap-3 sm:grid-cols-2">
              <Field label="Documento (CPF/CNPJ)" htmlFor="s-issuer-doc">
                <Input
                  id="s-issuer-doc"
                  value={form.issuerDocument}
                  onChange={(e) => set("issuerDocument", e.target.value)}
                />
              </Field>
              <Field label="Contato (e-mail/telefone)" htmlFor="s-issuer-contact">
                <Input
                  id="s-issuer-contact"
                  value={form.issuerContact}
                  onChange={(e) => set("issuerContact", e.target.value)}
                />
              </Field>
            </div>
            <p className="text-2xs text-text-subtle">
              Aparece no cabecalho da fatura em PDF (Relatorios → Gerar fatura).
            </p>
          </div>
        </Panel>

        <Panel>
          <PanelHeader title="Comportamento e regiao" />
          <div className="space-y-4 p-4">
            <Checkbox
              label="Minimizar para a bandeja"
              checked={form.minimizeToTray}
              onChange={(v) => set("minimizeToTray", v)}
            />
            <Checkbox
              label="Fechar para a bandeja"
              checked={form.closeToTray}
              onChange={(v) => set("closeToTray", v)}
            />
            <Checkbox
              label="Iniciar com o Windows"
              checked={form.startWithWindows}
              onChange={(v) => set("startWithWindows", v)}
            />
            <div className="grid gap-3 sm:grid-cols-2">
              <Field label="Moeda" htmlFor="s-currency">
                <Input
                  id="s-currency"
                  value={form.currency}
                  onChange={(e) => set("currency", e.target.value)}
                />
              </Field>
              <Field label="Idioma" htmlFor="s-locale">
                <Input
                  id="s-locale"
                  value={form.locale}
                  onChange={(e) => set("locale", e.target.value)}
                />
              </Field>
            </div>
          </div>
        </Panel>

        {saveError && <p className="text-sm text-danger">{saveError}</p>}
      </div>
    </div>
  );
}
