import { useEffect } from "react";
import { listenEvent } from "@/services/tauri";
import { TauriEvent } from "@/types/events";
import { useTimerStore } from "@/stores/timerStore";
import { useEntriesStore } from "@/stores/entriesStore";
import { useCatalogStore } from "@/stores/catalogStore";
import { useSettingsStore } from "@/stores/settingsStore";
import { useMonitorStore } from "@/stores/monitorStore";

/**
 * Sincronizacao inicial e reativa do estado com o backend.
 *
 * - Na montagem: carrega catalogo, sessoes, cronometro (com recuperacao) e
 *   configuracoes/programas monitorados.
 * - Enquanto montado: escuta eventos do backend (sem polling — secao 21):
 *   `timer-state-changed` re-sincroniza cronometro e sessoes;
 *   `monitored-app-opened`/`closed` abrem os lembretes conforme o estado.
 */
export function useAppSync(): void {
  useEffect(() => {
    void useCatalogStore.getState().loadAll();
    void useEntriesStore.getState().load();
    void useTimerStore.getState().loadActive(true);
    void useSettingsStore.getState().load();

    const unlisteners: Array<() => void> = [];

    void listenEvent(TauriEvent.timerStateChanged, () => {
      void useTimerStore.getState().loadActive(false);
      void useEntriesStore.getState().load();
      void useCatalogStore.getState().loadTotals();
    }).then((fn) => unlisteners.push(fn));

    // O lembrete de "programa aberto" e tratado pelo widget flutuante (janela
    // "reminder"), sobre o proprio CAD. Aqui cuidamos apenas de fechamento,
    // inatividade e saida.

    void listenEvent(TauriEvent.monitoredAppClosed, (payload) => {
      // Lembrete de encerramento apenas quando ha cronometro ativo (secao 10).
      if (payload.hasActiveTimer) {
        useMonitorStore.getState().setClose({
          processName: payload.processName,
          displayName: payload.displayName,
        });
      }
    }).then((fn) => unlisteners.push(fn));

    void listenEvent(TauriEvent.idleEnded, (payload) => {
      // Decisao sobre a inatividade so faz sentido com cronometro ativo (secao 11).
      if (payload.hasActiveTimer && payload.idleSeconds > 0) {
        useMonitorStore.getState().setIdle(payload.idleSeconds);
      }
    }).then((fn) => unlisteners.push(fn));

    void listenEvent(TauriEvent.requestQuit, () => {
      // Sair com cronometro ativo: pede confirmacao (secao 15).
      useMonitorStore.getState().requestQuit();
    }).then((fn) => unlisteners.push(fn));

    return () => unlisteners.forEach((fn) => fn());
  }, []);
}
