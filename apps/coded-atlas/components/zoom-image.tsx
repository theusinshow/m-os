/* eslint-disable @next/next/no-img-element */
"use client";
import { useState, useEffect } from "react";

interface Props {
  src: string;
  alt: string;
  className?: string;
  style?: React.CSSProperties;
  loading?: "lazy" | "eager";
}

/**
 * Imagem que abre em tela cheia ao clicar (lightbox), com zoom 1×/real.
 * Esc, clique no fundo ou no × fecham. Trava o scroll do body enquanto aberta.
 */
export function ZoomImage({ src, alt, className, style, loading = "lazy" }: Props) {
  const [open, setOpen] = useState(false);
  const [zoom, setZoom] = useState(false);

  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") setOpen(false);
    }
    document.addEventListener("keydown", onKey);
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => {
      document.removeEventListener("keydown", onKey);
      document.body.style.overflow = prev;
    };
  }, [open]);

  return (
    <>
      <img
        src={src}
        alt={alt}
        loading={loading}
        style={style}
        className={`${className ?? ""} cursor-zoom-in`}
        onClick={() => {
          setZoom(false);
          setOpen(true);
        }}
      />

      {open && (
        <div
          role="dialog"
          aria-modal="true"
          aria-label={alt}
          onClick={() => setOpen(false)}
          className="fixed inset-0 z-[60] bg-base/95 backdrop-blur-sm flex items-center justify-center p-4 sm:p-8"
        >
          <button
            aria-label="Fechar"
            onClick={() => setOpen(false)}
            className="absolute top-4 right-5 text-zinc-300 hover:text-white text-2xl leading-none cursor-pointer"
          >
            ×
          </button>
          <span className="absolute top-5 left-5 text-[11px] font-mono text-zinc-400 uppercase tracking-wider max-w-[70vw] truncate">
            {alt}
          </span>

          <div
            className={
              zoom
                ? "max-h-full max-w-full overflow-auto"
                : "max-h-full max-w-full flex items-center justify-center"
            }
            onClick={(e) => e.stopPropagation()}
          >
            <img
              src={src}
              alt={alt}
              onClick={() => setZoom((z) => !z)}
              className={
                zoom
                  ? "max-w-none cursor-zoom-out border border-line"
                  : "max-h-[88vh] max-w-[92vw] object-contain cursor-zoom-in border border-line"
              }
            />
          </div>
        </div>
      )}
    </>
  );
}
