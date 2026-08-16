/**
 * Verificação da Fase 2 (kit de redes — vídeos).
 * Cria gravações de scroll sintéticas (ffmpeg testsrc) para desktop e mobile,
 * roda generateSocialVideos e confere que cada formato saiu como MP4 nas
 * dimensões exatas do config, com poster.
 */
import path from "node:path";
import { promises as fs } from "node:fs";
import { spawn } from "node:child_process";
import { config } from "../lib/config";
import { generateSocialVideos } from "../lib/social/generate-videos";
import { ffmpegAvailable } from "../lib/social/ffmpeg";
import { socialVideoDir } from "../lib/storage/paths";
import type { DeviceCaptureResult } from "../lib/capture/capture-device";

const SLUG = "__test-social-vid__";

function fakeDevice(videoAbsPath: string): DeviceCaptureResult {
  return {
    viewport: "x", screenshot: "/x", fullpage: "/x",
    screenshotAbsPath: videoAbsPath, fullpageAbsPath: videoAbsPath,
    sections: [], videoAbsPath,
  };
}

function run(bin: string, args: string[]): Promise<string> {
  return new Promise((resolve, reject) => {
    const p = spawn(bin, args, { stdio: ["ignore", "pipe", "pipe"] });
    let out = ""; let err = "";
    p.stdout.on("data", (c) => (out += c));
    p.stderr.on("data", (c) => (err += c));
    p.on("error", reject);
    p.on("close", (code) => (code === 0 ? resolve(out) : reject(new Error(err))));
  });
}

async function probeDims(file: string): Promise<{ w: number; h: number; codec: string }> {
  const out = await run("ffprobe", [
    "-v", "error", "-select_streams", "v:0",
    "-show_entries", "stream=width,height,codec_name",
    "-of", "json", file,
  ]);
  const s = JSON.parse(out).streams[0];
  return { w: s.width, h: s.height, codec: s.codec_name };
}

async function main() {
  let pass = 0, fail = 0;
  const ok = (c: boolean, m: string) => { if (c) { pass++; console.log(`  ✓ ${m}`); } else { fail++; console.error(`  ✗ ${m}`); } };

  if (!(await ffmpegAvailable())) {
    console.error("ffmpeg indisponível — não é possível verificar a Fase 2 nesta máquina.");
    process.exit(1);
  }

  const dir = path.join(config.outputDir, SLUG);
  await fs.rm(dir, { recursive: true, force: true });
  await fs.mkdir(dir, { recursive: true });

  // Gravações sintéticas: desktop 1440×900, mobile 390×844, 3s cada.
  const deskVid = path.join(dir, "src-desktop.webm");
  const mobVid = path.join(dir, "src-mobile.webm");
  await run("ffmpeg", ["-y", "-f", "lavfi", "-i", "testsrc=size=1440x900:rate=30:duration=3", "-c:v", "libvpx", "-b:v", "1M", deskVid]);
  await run("ffmpeg", ["-y", "-f", "lavfi", "-i", "testsrc=size=390x844:rate=30:duration=3", "-c:v", "libvpx", "-b:v", "1M", mobVid]);

  const results = await generateSocialVideos(SLUG, fakeDevice(deskVid), fakeDevice(mobVid));

  ok(results.length === config.social.videos.length, `gerou ${results.length}/${config.social.videos.length} vídeos`);

  for (const fmt of config.social.videos) {
    const asset = results.find((r) => r.name === fmt.name);
    ok(!!asset, `formato "${fmt.name}" presente`);
    if (!asset) continue;
    ok(asset.video === `/generated/${SLUG}/social/videos/${fmt.name}.mp4`, `caminho público de "${fmt.name}" correto`);
    ok(asset.video.startsWith("/generated/") && !asset.video.includes("\\"), `caminho de "${fmt.name}" é público`);
    ok(!!asset.poster && asset.poster.endsWith(".jpg"), `poster de "${fmt.name}" gerado`);

    const file = path.join(socialVideoDir(SLUG), `${fmt.name}.mp4`);
    const { w, h, codec } = await probeDims(file);
    ok(w === fmt.width && h === fmt.height, `"${fmt.name}" = ${w}×${h} (esperado ${fmt.width}×${fmt.height})`);
    ok(codec === "h264", `"${fmt.name}" codec = ${codec} (esperado h264)`);
  }

  await fs.rm(dir, { recursive: true, force: true });
  console.log(`\n${pass} passaram, ${fail} falharam`);
  process.exit(fail ? 1 : 0);
}

main().catch((err) => { console.error(err); process.exit(1); });
