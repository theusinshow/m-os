import { spawn } from "node:child_process";
import { config } from "../config";

/**
 * Verifica se o binário do ffmpeg está disponível (`ffmpeg -version`).
 * Cacheado por processo — o kit de vídeo é opcional e não pode custar uma
 * verificação a cada formato.
 */
let availability: Promise<boolean> | undefined;

export function ffmpegAvailable(): Promise<boolean> {
  if (!availability) {
    availability = new Promise<boolean>((resolve) => {
      let settled = false;
      const done = (v: boolean) => { if (!settled) { settled = true; resolve(v); } };
      try {
        const proc = spawn(config.social.ffmpegPath, ["-version"], { stdio: "ignore" });
        proc.on("error", () => done(false));
        proc.on("close", (code) => done(code === 0));
      } catch {
        done(false);
      }
    });
  }
  return availability;
}

/**
 * Executa o ffmpeg com os argumentos dados. Rejeita com o stderr capturado
 * quando o código de saída não é 0 — o chamador decide se pula o formato.
 */
export function runFfmpeg(args: string[]): Promise<void> {
  return new Promise<void>((resolve, reject) => {
    const proc = spawn(config.social.ffmpegPath, args, { stdio: ["ignore", "ignore", "pipe"] });
    let stderr = "";
    proc.stderr.on("data", (chunk: Buffer) => { stderr += chunk.toString(); });
    proc.on("error", (err) => reject(err));
    proc.on("close", (code) => {
      if (code === 0) resolve();
      else reject(new Error(`ffmpeg saiu com código ${code}: ${stderr.slice(-500)}`));
    });
  });
}
