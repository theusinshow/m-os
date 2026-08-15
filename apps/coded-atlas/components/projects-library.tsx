"use client";
import { useMemo, useState } from "react";
import { useRouter } from "next/navigation";
import type { ProjectSummary } from "@/lib/storage/list-projects";
import { ProjectCatalogCard } from "@/components/project-catalog-card";

interface Props {
  projects: ProjectSummary[];
}

type SortKey = "recent" | "name";

export function ProjectsLibrary({ projects }: Props) {
  const router = useRouter();
  const [query, setQuery] = useState("");
  const [category, setCategory] = useState<string>("Todas");
  const [sort, setSort] = useState<SortKey>("recent");

  // Seleção em lote
  const [selectMode, setSelectMode] = useState(false);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const categories = useMemo(() => {
    const set = new Set(projects.map((p) => p.category));
    return ["Todas", ...Array.from(set).sort()];
  }, [projects]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    let list = projects.filter((p) => {
      if (category !== "Todas" && p.category !== category) return false;
      if (!q) return true;
      return (
        p.name.toLowerCase().includes(q) ||
        p.category.toLowerCase().includes(q) ||
        p.url.toLowerCase().includes(q) ||
        (p.client?.toLowerCase().includes(q) ?? false) ||
        p.slug.toLowerCase().includes(q)
      );
    });
    if (sort === "name") {
      list = [...list].sort((a, b) => a.name.localeCompare(b.name, "pt-BR"));
    }
    return list;
  }, [projects, query, category, sort]);

  function toggle(slug: string) {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(slug)) next.delete(slug);
      else next.add(slug);
      return next;
    });
  }

  function exitSelect() {
    setSelectMode(false);
    setSelected(new Set());
    setConfirming(false);
    setError("");
  }

  function selectAllVisible() {
    setSelected(new Set(filtered.map((p) => p.slug)));
  }

  async function bulkDelete() {
    setBusy(true);
    setError("");
    const slugs = [...selected];
    const failed: string[] = [];
    for (const slug of slugs) {
      try {
        const res = await fetch(`/api/projects/${slug}`, { method: "DELETE" });
        if (!res.ok) failed.push(slug);
      } catch {
        failed.push(slug);
      }
    }
    setBusy(false);
    if (failed.length) {
      setError(`Falha ao remover ${failed.length} projeto(s).`);
      setSelected(new Set(failed));
      setConfirming(false);
    } else {
      exitSelect();
      router.refresh();
    }
  }

  function bulkDownload() {
    [...selected].forEach((slug, i) => {
      setTimeout(() => {
        const a = document.createElement("a");
        a.href = `/api/zip/${slug}`;
        a.download = `coded-atlas-${slug}.zip`;
        document.body.appendChild(a);
        a.click();
        a.remove();
      }, i * 400);
    });
  }

  const count = selected.size;

  return (
    <div className="space-y-6">
      {/* Controles */}
      <div className="flex flex-col md:flex-row md:items-center gap-3">
        <div className="relative flex-1">
          <span className="absolute left-3 top-1/2 -translate-y-1/2 text-zinc-500 text-sm" aria-hidden>⌕</span>
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Buscar por nome, cliente, URL ou categoria..."
            className="w-full bg-surface border border-line text-zinc-100 text-sm pl-9 pr-3 py-2.5 placeholder:text-zinc-500 focus:outline-none focus:border-accent transition-colors"
          />
        </div>

        <div className="flex items-center gap-1 shrink-0">
          {(["recent", "name"] as SortKey[]).map((k) => (
            <button
              key={k}
              onClick={() => setSort(k)}
              className={[
                "px-3 py-2.5 text-[13px] font-medium border transition-colors",
                sort === k ? "border-line-soft bg-surface text-zinc-100" : "border-line text-zinc-400 hover:text-zinc-200",
              ].join(" ")}
            >
              {k === "recent" ? "Recentes" : "A→Z"}
            </button>
          ))}
          <button
            onClick={() => (selectMode ? exitSelect() : setSelectMode(true))}
            className={[
              "px-3 py-2.5 text-[13px] font-medium border transition-colors",
              selectMode ? "border-accent text-accent-bright" : "border-line text-zinc-400 hover:text-zinc-200",
            ].join(" ")}
          >
            {selectMode ? "Concluir" : "Selecionar"}
          </button>
        </div>
      </div>

      {/* Filtro por categoria */}
      {categories.length > 2 && (
        <div className="flex flex-wrap gap-2">
          {categories.map((c) => {
            const active = category === c;
            return (
              <button
                key={c}
                onClick={() => setCategory(c)}
                className={[
                  "px-3 py-1.5 text-[12px] font-medium border transition-colors",
                  active ? "border-accent text-accent-bright bg-accent/10" : "border-line text-zinc-400 hover:text-zinc-200 hover:border-line-soft",
                ].join(" ")}
              >
                {c}
              </button>
            );
          })}
        </div>
      )}

      {/* Contagem */}
      <p className="text-[12px] font-mono text-zinc-500">
        {filtered.length} de {projects.length} projeto{projects.length !== 1 ? "s" : ""}
        {query && ` para "${query}"`}
      </p>

      {/* Grid */}
      {filtered.length > 0 ? (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {filtered.map((p) => (
            <ProjectCatalogCard
              key={p.slug}
              project={p}
              selectable={selectMode}
              selected={selected.has(p.slug)}
              onToggle={toggle}
            />
          ))}
        </div>
      ) : (
        <div className="border border-line bg-surface/30 p-10 text-center space-y-3">
          <p className="text-sm text-zinc-300">Nenhum projeto encontrado com esse filtro.</p>
          <button
            onClick={() => {
              setQuery("");
              setCategory("Todas");
            }}
            className="text-[13px] font-medium text-accent hover:text-accent-bright transition-colors"
          >
            Limpar filtros
          </button>
        </div>
      )}

      {/* Barra de ações em lote (fixa no rodapé quando há seleção) */}
      {selectMode && count > 0 && (
        <div className="fixed bottom-5 left-1/2 -translate-x-1/2 z-40 w-[min(92vw,40rem)]">
          <div className="border border-line-soft bg-surface shadow-2xl px-4 py-3 flex flex-wrap items-center justify-between gap-3">
            <div className="flex items-center gap-3 min-w-0">
              <span className="text-sm font-medium text-zinc-100">
                {count} selecionado{count !== 1 ? "s" : ""}
              </span>
              <button onClick={selectAllVisible} className="text-[12px] font-medium text-zinc-400 hover:text-zinc-100 transition-colors">
                Selecionar todos ({filtered.length})
              </button>
              <button onClick={() => setSelected(new Set())} className="text-[12px] font-medium text-zinc-400 hover:text-zinc-100 transition-colors">
                Limpar
              </button>
            </div>

            <div className="flex items-center gap-2">
              <button
                onClick={bulkDownload}
                className="px-3 py-1.5 border border-line text-zinc-200 text-[13px] font-medium hover:border-line-soft hover:text-zinc-50 transition-colors"
              >
                Baixar ZIPs
              </button>
              {!confirming ? (
                <button
                  onClick={() => setConfirming(true)}
                  className="px-3 py-1.5 border border-bad/40 text-bad text-[13px] font-medium hover:bg-bad/10 transition-colors"
                >
                  Excluir
                </button>
              ) : (
                <>
                  <button
                    onClick={bulkDelete}
                    disabled={busy}
                    className="px-3 py-1.5 bg-bad text-zinc-50 text-[13px] font-semibold hover:opacity-90 transition-opacity disabled:opacity-60"
                  >
                    {busy ? "Removendo..." : `Confirmar (${count})`}
                  </button>
                  <button
                    onClick={() => setConfirming(false)}
                    disabled={busy}
                    className="px-3 py-1.5 border border-line text-zinc-300 text-[13px] hover:text-zinc-100 transition-colors"
                  >
                    Cancelar
                  </button>
                </>
              )}
            </div>
          </div>
          {error && <p className="text-bad text-[12px] mt-2 text-center">{error}</p>}
        </div>
      )}
    </div>
  );
}
