import type { SocialKit } from "@/lib/types";
import { ZoomImage } from "@/components/zoom-image";

interface Props {
  slug: string;
  social: SocialKit;
  projectName: string;
}

/**
 * Seção "Redes sociais" (v2.0): o kit pronto para postar, com imagens e vídeos
 * separados. Cada asset tem preview + download individual; o bloco todo tem um
 * atalho para baixar o ZIP só dos assets de rede.
 */
export function SocialKitSection({ slug, social, projectName }: Props) {
  const { images, videos } = social;
  if (images.length === 0 && videos.length === 0) return null;

  return (
    <div className="border-t border-line pt-16">
      <div className="flex items-baseline justify-between mb-6">
        <p className="text-[11px] font-mono text-zinc-300 uppercase tracking-widest">
          Redes sociais
        </p>
        <a
          href={`/api/zip/${slug}?only=social`}
          className="text-[11px] font-mono text-accent hover:text-accent-bright transition-colors uppercase tracking-wider cursor-pointer"
        >
          Baixar kit (.zip) ↓
        </a>
      </div>

      {/* ── Imagens ── */}
      {images.length > 0 && (
        <div className="mb-10">
          <p className="text-[11px] font-mono text-zinc-400 uppercase tracking-wider mb-3">
            Imagens · {images.length}
          </p>
          <div className="grid sm:grid-cols-3 gap-4">
            {images.map((img) => (
              <div key={img.name} className="space-y-2">
                <div className="border border-line bg-surface h-64 flex items-center justify-center p-3">
                  <ZoomImage
                    src={img.image}
                    alt={`${projectName} — ${img.label}`}
                    className="max-h-full max-w-full object-contain"
                  />
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-[11px] font-mono text-zinc-400">
                    {img.label}{" "}
                    <span className="text-zinc-600">{img.width}×{img.height}</span>
                  </span>
                  <a
                    href={img.image}
                    download
                    className="text-[11px] font-mono text-accent hover:text-accent-bright transition-colors uppercase tracking-wider"
                  >
                    Baixar →
                  </a>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* ── Vídeos ── */}
      {videos.length > 0 && (
        <div>
          <p className="text-[11px] font-mono text-zinc-400 uppercase tracking-wider mb-3">
            Vídeos · {videos.length}
          </p>
          <div className="grid sm:grid-cols-2 gap-6">
            {videos.map((vid) => (
              <div key={vid.name} className="space-y-2">
                <div className="border border-line bg-black/40 flex items-center justify-center">
                  <video
                    controls
                    preload="metadata"
                    poster={vid.poster}
                    className="max-h-96 w-auto block"
                    style={{ aspectRatio: `${vid.width}/${vid.height}` }}
                  >
                    <source src={vid.video} type="video/mp4" />
                  </video>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-[11px] font-mono text-zinc-400">
                    {vid.label}{" "}
                    <span className="text-zinc-600">{vid.width}×{vid.height} · MP4</span>
                  </span>
                  <a
                    href={vid.video}
                    download
                    className="text-[11px] font-mono text-accent hover:text-accent-bright transition-colors uppercase tracking-wider"
                  >
                    Baixar →
                  </a>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
