"use client";
import { useState } from "react";
import { useRouter } from "next/navigation";

interface Props {
  slug: string;
  name: string;
}

export function DeleteProject({ slug, name }: Props) {
  const router = useRouter();
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  async function remove() {
    setBusy(true);
    setError("");
    try {
      const res = await fetch(`/api/projects/${slug}`, { method: "DELETE" });
      if (!res.ok) {
        const data = (await res.json().catch(() => ({}))) as { error?: string };
        throw new Error(data.error ?? "Falha ao remover.");
      }
      router.push("/projects");
      router.refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Falha ao remover.");
      setBusy(false);
      setConfirming(false);
    }
  }

  return (
    <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
      <div className="min-w-0">
        <p className="text-sm font-medium text-zinc-200">Remover projeto</p>
        <p className="text-[13px] text-zinc-400 mt-0.5">
          Apaga todos os arquivos gerados deste catálogo. Não dá para desfazer.
        </p>
        {error && <p className="text-bad text-[13px] mt-1.5">{error}</p>}
      </div>

      {!confirming ? (
        <button
          onClick={() => setConfirming(true)}
          className="shrink-0 px-4 py-2 border border-bad/40 text-bad text-[13px] font-medium hover:bg-bad/10 transition-colors cursor-pointer"
        >
          Remover
        </button>
      ) : (
        <div className="shrink-0 flex items-center gap-2">
          <span className="text-[13px] text-zinc-300 mr-1">Remover &ldquo;{name}&rdquo;?</span>
          <button
            onClick={remove}
            disabled={busy}
            className="px-4 py-2 bg-bad text-zinc-50 text-[13px] font-semibold hover:opacity-90 transition-opacity cursor-pointer disabled:opacity-60"
          >
            {busy ? "Removendo..." : "Sim, remover"}
          </button>
          <button
            onClick={() => setConfirming(false)}
            disabled={busy}
            className="px-4 py-2 border border-line text-zinc-300 text-[13px] hover:text-zinc-100 transition-colors cursor-pointer"
          >
            Cancelar
          </button>
        </div>
      )}
    </div>
  );
}
