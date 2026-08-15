export const runtime = "nodejs";

import { NextRequest } from "next/server";
import { promises as fs } from "node:fs";
import { ZipArchive } from "archiver";
import {
  projectDir,
  catalogPath,
  caseDraftPath,
} from "@/lib/storage/paths";

interface Params {
  params: Promise<{ slug: string }>;
}

export async function GET(_req: NextRequest, { params }: Params): Promise<Response> {
  const { slug } = await params;

  if (!slug || typeof slug !== "string") {
    return new Response("slug inválido", { status: 400 });
  }

  try {
    await fs.access(catalogPath(slug));
  } catch {
    return new Response("Projeto não encontrado.", { status: 404 });
  }

  const dir = projectDir(slug);
  const archive = new ZipArchive({ zlib: { level: 6 } });

  archive.file(catalogPath(slug), { name: "catalog.json" });

  const screenshotFiles = [
    "desktop-1440x900.png",
    "desktop-fullpage.png",
    "mobile-390x844.png",
    "mobile-fullpage.png",
  ];
  for (const file of screenshotFiles) {
    const absPath = `${dir}/screenshots/${file}`;
    try {
      await fs.access(absPath);
      archive.file(absPath, { name: `screenshots/${file}` });
    } catch { /* arquivo opcional */ }
  }

  const thumbFiles = ["thumb-main.webp", "thumb-mobile.webp", "cover.webp"];
  for (const file of thumbFiles) {
    const absPath = `${dir}/thumbnails/${file}`;
    try {
      await fs.access(absPath);
      archive.file(absPath, { name: `thumbnails/${file}` });
    } catch { /* arquivo opcional */ }
  }

  const videoFiles = ["desktop-scroll.webm", "mobile-scroll.webm"];
  for (const file of videoFiles) {
    const absPath = `${dir}/videos/${file}`;
    try {
      await fs.access(absPath);
      archive.file(absPath, { name: `videos/${file}` });
    } catch { /* arquivo opcional */ }
  }

  try {
    await fs.access(caseDraftPath(slug));
    archive.file(caseDraftPath(slug), { name: "case-draft.mdx" });
  } catch { /* arquivo opcional */ }

  for (const device of ["desktop", "mobile"]) {
    const sectionsDir = `${dir}/screenshots/sections-${device}`;
    try {
      await fs.access(sectionsDir);
      archive.directory(sectionsDir, `screenshots/sections-${device}`);
    } catch { /* opcional */ }
  }

  const compositionsDir = `${dir}/compositions`;
  try {
    await fs.access(compositionsDir);
    archive.directory(compositionsDir, "compositions");
  } catch { /* opcional */ }

  const mockupsDir = `${dir}/mockups`;
  try {
    await fs.access(mockupsDir);
    archive.directory(mockupsDir, "mockups");
  } catch { /* opcional */ }

  const pagesDir = `${dir}/screenshots/pages`;
  try {
    await fs.access(pagesDir);
    archive.directory(pagesDir, "screenshots/pages");
  } catch { /* opcional */ }

  const statesDir = `${dir}/screenshots/states`;
  try {
    await fs.access(statesDir);
    archive.directory(statesDir, "screenshots/states");
  } catch { /* opcional */ }

  archive.finalize();

  const readable = new ReadableStream({
    start(controller) {
      archive.on("data", (chunk: Buffer) => controller.enqueue(chunk));
      archive.on("end", () => controller.close());
      archive.on("error", (err) => controller.error(err));
    },
  });

  return new Response(readable, {
    headers: {
      "Content-Type": "application/zip",
      "Content-Disposition": `attachment; filename="coded-atlas-${slug}.zip"`,
      "Cache-Control": "no-store",
    },
  });
}
