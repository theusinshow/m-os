import { DeviceFrame } from "./device-frame";
import { ZoomImage } from "./zoom-image";
import type { DeviceCapture } from "@/lib/types";

interface Props {
  type: "desktop" | "mobile";
  capture: DeviceCapture;
  projectName: string;
}

export function GeneratedGallery({ type, capture, projectName }: Props) {
  const isDesktop = type === "desktop";
  const label     = isDesktop ? "Desktop" : "Mobile";

  return (
    <section aria-label={`Galeria ${label}`}>
      {/* Section header */}
      <div className="flex items-center gap-3 mb-5">
        <h2 className="text-[11px] font-mono text-zinc-300 uppercase tracking-widest">
          {label}
        </h2>
        <span className="text-[11px] font-mono text-zinc-500">{capture.viewport}</span>
      </div>

      {isDesktop ? (
        <div className="space-y-4">
          {/* Viewport */}
          <div>
            <p className="text-[11px] font-mono text-zinc-400 uppercase tracking-wider mb-2">
              Viewport
            </p>
            <DeviceFrame type="desktop">
              <ZoomImage
                src={capture.screenshot}
                alt={`${projectName} — desktop viewport`}
                className="w-full block"
              />
            </DeviceFrame>
          </div>

          {/* Full page — clipped with fade */}
          <div>
            <p className="text-[11px] font-mono text-zinc-400 uppercase tracking-wider mb-2">
              Full Page
            </p>
            <DeviceFrame type="desktop">
              <div className="relative max-h-72 overflow-hidden">
                <ZoomImage
                  src={capture.fullpage}
                  alt={`${projectName} — desktop full page`}
                  className="w-full block"
                />
                <div className="absolute inset-x-0 bottom-0 h-20 bg-linear-to-t from-surface to-transparent pointer-events-none" />
              </div>
            </DeviceFrame>
          </div>
        </div>
      ) : (
        /* Mobile — viewport + fullpage side by side */
        <div className="grid grid-cols-2 gap-4" style={{ maxWidth: "28rem" }}>
          <div>
            <p className="text-[11px] font-mono text-zinc-400 uppercase tracking-wider mb-2">
              Viewport
            </p>
            <DeviceFrame type="mobile">
              <ZoomImage
                src={capture.screenshot}
                alt={`${projectName} — mobile viewport`}
                className="w-full block"
              />
            </DeviceFrame>
          </div>

          <div>
            <p className="text-[11px] font-mono text-zinc-400 uppercase tracking-wider mb-2">
              Full Page
            </p>
            <DeviceFrame type="mobile">
              <div className="relative max-h-112 overflow-hidden">
                <ZoomImage
                  src={capture.fullpage}
                  alt={`${projectName} — mobile full page`}
                  className="w-full block"
                />
                <div className="absolute inset-x-0 bottom-0 h-16 bg-linear-to-t from-surface to-transparent pointer-events-none" />
              </div>
            </DeviceFrame>
          </div>
        </div>
      )}
    </section>
  );
}
