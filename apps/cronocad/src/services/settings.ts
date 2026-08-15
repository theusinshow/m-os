/**
 * Servico de configuracoes: leitura e atualizacao (comandos Tauri tipados).
 */

import type { Settings } from "@/types/domain";
import { invokeCommand } from "./tauri";

export function getSettings(): Promise<Settings> {
  return invokeCommand<Settings>("get_settings");
}

export function updateSettings(settings: Settings): Promise<Settings> {
  return invokeCommand<Settings>("update_settings", { settings });
}
