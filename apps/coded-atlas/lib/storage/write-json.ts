import { promises as fs } from "node:fs";
import { AtlasError } from "../errors";

export async function writeJson(filePath: string, data: unknown): Promise<void> {
  try {
    await fs.writeFile(filePath, JSON.stringify(data, null, 2), "utf-8");
  } catch (err) {
    throw new AtlasError(
      "STORAGE_FAILED",
      "Não foi possível salvar os arquivos gerados.",
      `Failed to write JSON to "${filePath}": ${err}`
    );
  }
}
