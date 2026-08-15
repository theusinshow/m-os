"use client";
import { useState } from "react";
import { slugify } from "@/lib/validation/slugify";
import { PROJECT_CATEGORIES } from "@/lib/categories";
import type { ProjectInput } from "@/lib/types";

const INPUT =
  "w-full bg-surface border border-line text-zinc-100 text-sm px-3 py-2.5 " +
  "placeholder:text-zinc-500 focus:outline-none focus:border-accent transition-colors";

const LABEL =
  "block text-[11px] font-mono text-zinc-400 tracking-wider uppercase mb-1.5";

interface Props {
  onSubmit: (input: ProjectInput) => void;
}

/** Deriva um nome inicial a partir do domínio (só sugestão, editável). */
function nameFromUrl(url: string): string {
  try {
    const host = new URL(url).host.replace(/^www\./, "");
    const label = host.split(".")[0] ?? "";
    return label ? label.charAt(0).toUpperCase() + label.slice(1) : "";
  } catch {
    return "";
  }
}

export function UrlInput({ onSubmit }: Props) {
  const [url, setUrl] = useState("");
  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [category, setCategory] = useState<string>(PROJECT_CATEGORIES[0]);
  const [customCategory, setCustomCategory] = useState("");
  const [client, setClient] = useState("");
  const [description, setDescription] = useState("");
  const [pages, setPages] = useState("");
  const [states, setStates] = useState("");
  const [video, setVideo] = useState(true);
  const [sections, setSections] = useState(true);
  const [nameEdited, setNameEdited] = useState(false);
  const [slugEdited, setSlugEdited] = useState(false);
  const [showDetails, setShowDetails] = useState(false);
  const [urlError, setUrlError] = useState("");

  const isOther = category === "Outro";

  function setNameAndSlug(val: string) {
    setName(val);
    if (!slugEdited) setSlug(slugify(val));
  }

  function handleNameChange(val: string) {
    setNameEdited(true);
    setNameAndSlug(val);
  }

  function handleUrlBlur(val: string) {
    validateUrl(val);
    // Autofill: sugere nome a partir do domínio se o usuário ainda não digitou.
    if (!nameEdited && !name.trim()) {
      const suggested = nameFromUrl(val);
      if (suggested) setNameAndSlug(suggested);
    }
  }

  function validateUrl(val: string): boolean {
    if (!val.trim()) {
      setUrlError("Informe a URL do projeto.");
      return false;
    }
    try {
      const u = new URL(val.trim());
      if (u.protocol !== "http:" && u.protocol !== "https:") {
        setUrlError("A URL deve começar com http:// ou https://");
        return false;
      }
    } catch {
      setUrlError("URL inválida. Use o formato https://exemplo.com");
      return false;
    }
    setUrlError("");
    return true;
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!validateUrl(url)) return;
    const finalSlug = slug.trim() || slugify(name);
    const finalCategory = isOther ? customCategory.trim() : category;
    if (!name.trim() || !finalSlug || !finalCategory) return;
    const pageList = pages
      .split(/[\n,]/)
      .map((s) => s.trim())
      .filter(Boolean);
    const stateList = states
      .split("\n")
      .map((line) => {
        const [name, ...rest] = line.split("|");
        return { name: (name ?? "").trim(), selector: rest.join("|").trim() };
      })
      .filter((s) => s.name && s.selector);
    onSubmit({
      url: url.trim(),
      name: name.trim(),
      slug: finalSlug,
      category: finalCategory,
      client: client.trim() || undefined,
      description: description.trim() || undefined,
      options: { video, sections },
      pages: pageList.length ? pageList : undefined,
      states: stateList.length ? stateList : undefined,
    });
  }

  return (
    <form onSubmit={handleSubmit} noValidate className="space-y-5">
      {/* URL */}
      <div>
        <label className={LABEL}>URL do projeto</label>
        <input
          type="url"
          value={url}
          onChange={(e) => {
            setUrl(e.target.value);
            if (urlError) validateUrl(e.target.value);
          }}
          onBlur={(e) => handleUrlBlur(e.target.value)}
          placeholder="https://exemplo.com.br"
          required
          autoFocus
          className={`${INPUT} ${urlError ? "border-bad focus:border-bad" : ""}`}
        />
        {urlError ? (
          <p className="text-bad text-xs font-mono mt-1.5">{urlError}</p>
        ) : (
          <p className="text-zinc-500 text-xs mt-1.5">
            O nome é sugerido a partir do domínio. Você pode ajustar.
          </p>
        )}
      </div>

      {/* Nome + Slug */}
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label className={LABEL}>Nome do projeto</label>
          <input
            type="text"
            value={name}
            onChange={(e) => handleNameChange(e.target.value)}
            placeholder="Machado Plataformas"
            required
            className={INPUT}
          />
        </div>
        <div>
          <label className={LABEL}>Slug</label>
          <input
            type="text"
            value={slug}
            onChange={(e) => {
              setSlug(e.target.value);
              setSlugEdited(true);
            }}
            placeholder="machado-plataformas"
            required
            className={`${INPUT} font-mono text-zinc-300`}
          />
        </div>
      </div>

      {/* Categoria (dropdown) */}
      <div>
        <label className={LABEL}>Tipo de projeto</label>
        <div className={isOther ? "grid grid-cols-2 gap-4" : ""}>
          <div className="relative">
            <select
              value={category}
              onChange={(e) => setCategory(e.target.value)}
              className={`${INPUT} appearance-none pr-9 cursor-pointer`}
            >
              {PROJECT_CATEGORIES.map((c) => (
                <option key={c} value={c} className="bg-surface text-zinc-100">
                  {c}
                </option>
              ))}
            </select>
            <span
              className="pointer-events-none absolute right-3 top-1/2 -translate-y-1/2 text-zinc-500 text-[10px]"
              aria-hidden
            >
              ▼
            </span>
          </div>
          {isOther && (
            <input
              type="text"
              value={customCategory}
              onChange={(e) => setCustomCategory(e.target.value)}
              placeholder="Qual tipo?"
              required
              autoFocus
              className={INPUT}
            />
          )}
        </div>
      </div>

      {/* Capturas — controle rápido do que gerar */}
      <div>
        <label className={LABEL}>Capturas</label>
        <div className="flex flex-wrap gap-2">
          {[
            { on: sections, set: setSections, label: "Seções", hint: "fotografa cada bloco" },
            { on: video, set: setVideo, label: "Vídeo de scroll", hint: "grava a navegação" },
          ].map((t) => (
            <button
              key={t.label}
              type="button"
              onClick={() => t.set(!t.on)}
              aria-pressed={t.on}
              title={t.hint}
              className={[
                "flex items-center gap-2 px-3 py-2 text-[13px] font-medium border transition-colors cursor-pointer",
                t.on
                  ? "border-accent text-accent-bright bg-accent/[0.06]"
                  : "border-line text-zinc-500 hover:text-zinc-300",
              ].join(" ")}
            >
              <span
                className={[
                  "w-3.5 h-3.5 border flex items-center justify-center text-[9px] leading-none",
                  t.on ? "bg-accent border-accent text-zinc-950" : "border-line text-transparent",
                ].join(" ")}
                aria-hidden
              >
                ✓
              </span>
              {t.label}
            </button>
          ))}
        </div>
        <p className="text-zinc-500 text-xs mt-1.5">
          Desligar o vídeo deixa a geração bem mais rápida.
        </p>
      </div>

      {/* Detalhes opcionais — recolhidos por padrão (menos a preencher) */}
      <div className="border-t border-line pt-4">
        {!showDetails ? (
          <button
            type="button"
            onClick={() => setShowDetails(true)}
            className="flex items-center gap-2 text-[13px] text-zinc-400 hover:text-zinc-100 transition-colors"
          >
            <span className="text-accent" aria-hidden>
              +
            </span>
            Adicionar cliente e descrição (opcional)
          </button>
        ) : (
          <div className="space-y-4">
            <div>
              <label className={LABEL}>
                Cliente{" "}
                <span className="text-zinc-600 normal-case tracking-normal font-sans">(opcional)</span>
              </label>
              <input
                type="text"
                value={client}
                onChange={(e) => setClient(e.target.value)}
                placeholder="Nome do cliente"
                className={INPUT}
              />
            </div>
            <div>
              <label className={LABEL}>
                Descrição{" "}
                <span className="text-zinc-600 normal-case tracking-normal font-sans">(opcional)</span>
              </label>
              <textarea
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder="Breve descrição do projeto..."
                rows={3}
                className={`${INPUT} resize-none`}
              />
            </div>
            <div>
              <label className={LABEL}>
                Páginas adicionais{" "}
                <span className="text-zinc-600 normal-case tracking-normal font-sans">(opcional)</span>
              </label>
              <textarea
                value={pages}
                onChange={(e) => setPages(e.target.value)}
                placeholder={"/sobre\n/servicos\n/contato"}
                rows={3}
                className={`${INPUT} resize-none font-mono text-zinc-300`}
              />
              <p className="text-zinc-500 text-xs mt-1.5">
                Uma por linha. Paths (ex: /sobre) ou URLs completas. Capturadas em desktop e mobile.
              </p>
            </div>
            <div>
              <label className={LABEL}>
                Estados de interação{" "}
                <span className="text-zinc-600 normal-case tracking-normal font-sans">(opcional)</span>
              </label>
              <textarea
                value={states}
                onChange={(e) => setStates(e.target.value)}
                placeholder={"Menu aberto | .menu-toggle\nBusca | button[aria-label^='Buscar']"}
                rows={3}
                className={`${INPUT} resize-none font-mono text-zinc-300`}
              />
              <p className="text-zinc-500 text-xs mt-1.5">
                Formato <span className="font-mono text-zinc-400">Nome | seletor</span>, uma por linha. Clica o seletor e fotografa (desktop).
              </p>
            </div>
          </div>
        )}
      </div>

      <button
        type="submit"
        className="w-full py-3 bg-zinc-100 text-zinc-950 text-sm font-semibold hover:bg-white transition-colors"
      >
        Gerar catálogo →
      </button>
    </form>
  );
}
