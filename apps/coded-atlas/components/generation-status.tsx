"use client";
import type { ProgressEvent } from "@/lib/types";

type StepDef = { step: string; phase: string; label: string; desc: string };

// Ordem espelha exatamente a emissão da engine (capture-device + route).
const STEPS: StepDef[] = [
  { step: "validating", phase: "Preparando", label: "Validando URL e dados", desc: "Conferindo se a URL responde e se os campos estão corretos." },
  { step: "launching", phase: "Preparando", label: "Abrindo navegador", desc: "Iniciando um Chromium isolado para visitar o site." },

  { step: "capturing-desktop", phase: "Desktop", label: "Capturando viewport", desc: "Foto da primeira dobra em 1440×900." },
  { step: "capturing-sections-desktop", phase: "Desktop", label: "Fotografando seções", desc: "Detectando blocos da página e capturando cada um." },
  { step: "capturing-fullpage-desktop", phase: "Desktop", label: "Full page", desc: "Rolando até o fim e capturando a página inteira." },
  { step: "recording-video-desktop", phase: "Desktop", label: "Gravando scroll", desc: "Vídeo curto da navegação em desktop." },

  { step: "capturing-mobile", phase: "Mobile", label: "Capturando viewport", desc: "Foto da primeira dobra em 390×844." },
  { step: "capturing-sections-mobile", phase: "Mobile", label: "Fotografando seções", desc: "Mesmos blocos, agora na largura mobile." },
  { step: "capturing-fullpage-mobile", phase: "Mobile", label: "Full page", desc: "Página inteira em mobile." },
  { step: "recording-video-mobile", phase: "Mobile", label: "Gravando scroll", desc: "Vídeo curto da navegação em mobile." },

  { step: "capturing-pages", phase: "Finalizando", label: "Páginas extras", desc: "Capturando as outras páginas do site." },
  { step: "capturing-states", phase: "Finalizando", label: "Estados de interação", desc: "Clicando os seletores e fotografando." },
  { step: "generating-thumbnails", phase: "Finalizando", label: "Gerando thumbnails e capa", desc: "Redimensionando com Sharp para o portfólio." },
  { step: "writing-catalog", phase: "Finalizando", label: "Montando catálogo", desc: "Escrevendo o catalog.json com tudo organizado." },
  { step: "done", phase: "Finalizando", label: "Catálogo pronto", desc: "" },
];

interface Props {
  events: ProgressEvent[];
  done?: boolean;
}

export function GenerationStatus({ events, done = false }: Props) {
  const lastEvent = events.at(-1);
  const progress = done ? 100 : lastEvent?.progress ?? 0;

  const lastIdx = done
    ? STEPS.length - 1
    : Math.max(-1, ...events.map((e) => STEPS.findIndex((s) => s.step === e.step)));

  const activeStep = lastIdx >= 0 && !done ? STEPS[lastIdx] : undefined;

  return (
    <div className="space-y-7">
      {/* Cabeçalho + barra de progresso */}
      <div>
        <div className="flex items-baseline justify-between mb-2.5">
          <span className="text-sm font-medium text-zinc-100" aria-live="polite" aria-atomic="true">
            {done ? "Catálogo gerado." : activeStep ? `${activeStep.phase} · ${activeStep.label}` : "Construindo catálogo visual..."}
          </span>
          <span className="text-[13px] font-mono text-accent tabular-nums">{progress}%</span>
        </div>
        <div className="h-1.5 bg-surface-2 relative rounded-full overflow-hidden">
          <div
            className="absolute inset-y-0 left-0 bg-accent transition-[width] duration-500 ease-out rounded-full"
            style={{ width: `${progress}%` }}
          />
        </div>
        {!done && (
          <p className="text-[12px] text-zinc-400 mt-2.5 min-h-[1.25rem]">
            {lastEvent?.message ?? "Iniciando..."}
          </p>
        )}
      </div>

      {/* Lista de etapas, agrupadas por fase */}
      <ol className="space-y-0.5">
        {STEPS.map((s, idx) => {
          const isDone = idx < lastIdx || (idx === lastIdx && done);
          const isActive = idx === lastIdx && !done;
          const isPending = idx > lastIdx;
          const firstOfPhase = idx === 0 || STEPS[idx - 1].phase !== s.phase;

          return (
            <li key={s.step}>
              {firstOfPhase && (
                <p className="text-[10px] font-mono uppercase tracking-widest text-zinc-500 mt-4 first:mt-0 mb-1">
                  {s.phase}
                </p>
              )}
              <div className="flex items-start gap-3 py-1.5">
                {/* Indicador */}
                <span
                  className={[
                    "w-4 shrink-0 mt-0.5 text-center font-mono text-sm leading-none select-none",
                    isDone ? "text-accent" : isActive ? "text-accent animate-atlas-pulse" : "text-zinc-600",
                  ].join(" ")}
                  aria-hidden
                >
                  {isDone ? "✓" : isActive ? "▶" : "○"}
                </span>

                {/* Texto */}
                <div className="min-w-0">
                  <span
                    className={[
                      "text-[14px]",
                      isActive ? "text-zinc-50 font-medium" : isDone ? "text-zinc-300" : "text-zinc-500",
                    ].join(" ")}
                  >
                    {s.label}
                  </span>
                  {(isActive || isPending) && s.desc && (
                    <p className={["text-[12px] mt-0.5", isActive ? "text-zinc-400" : "text-zinc-600"].join(" ")}>
                      {s.desc}
                    </p>
                  )}
                </div>
              </div>
            </li>
          );
        })}
      </ol>
    </div>
  );
}
