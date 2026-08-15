import type { Catalog } from "@/lib/types";

interface Props {
  catalog: Catalog;
}

const FILES = (catalog: Catalog) => [
  { label: "Desktop — Viewport",  href: catalog.captures.desktop.screenshot, ext: "PNG" },
  { label: "Desktop — Full Page", href: catalog.captures.desktop.fullpage,   ext: "PNG" },
  { label: "Mobile — Viewport",   href: catalog.captures.mobile.screenshot,  ext: "PNG" },
  { label: "Mobile — Full Page",  href: catalog.captures.mobile.fullpage,    ext: "PNG" },
  { label: "Thumbnail Principal", href: catalog.thumbnails.main,             ext: "WEBP" },
  { label: "Thumbnail Mobile",    href: catalog.thumbnails.mobile,           ext: "WEBP" },
];

export function AssetDownloadItems({ catalog }: Props) {
  return (
    <>
      {FILES(catalog).map(({ label, href, ext }) => (
        <li key={href}>
          <a
            href={href}
            download
            className="flex items-center justify-between px-4 py-3 text-zinc-300 hover:bg-surface hover:text-zinc-50 transition-colors group cursor-pointer"
          >
            <span className="text-sm">{label}</span>
            <span className="flex items-center gap-2 text-[11px] font-mono text-zinc-500 group-hover:text-accent transition-colors">
              <span>{ext}</span>
              <span>↓</span>
            </span>
          </a>
        </li>
      ))}
    </>
  );
}

export function AssetDownloads({ catalog }: Props) {
  return (
    <section aria-label="Downloads">
      <h2 className="text-[11px] font-mono text-zinc-300 uppercase tracking-widest mb-4">
        Downloads
      </h2>
      <ul className="divide-y divide-line border border-line">
        <AssetDownloadItems catalog={catalog} />
      </ul>
    </section>
  );
}
