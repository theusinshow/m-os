"use client";

import { useEffect, useRef } from "react";
import { cn } from "@/lib/utils";

type Triangle = {
  x: number;
  y: number;
  size: number;
  rot: number;
  spin: number;
  vx: number;
  vy: number;
  depth: number;
  accent: boolean;
};

/**
 * Animated triangular constellation rendered on a raw 2D canvas — no library,
 * DPR-aware, pointer parallax, paused when offscreen or under reduced motion.
 * The "Coded by M" hero ornament translated to the M Finance dark cockpit.
 */
export function TriangleField({
  className,
  density = 0.00007,
  interactive = true,
}: {
  className?: string;
  density?: number;
  interactive?: boolean;
}) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const dpr = Math.min(window.devicePixelRatio || 1, 2);

    let width = 0;
    let height = 0;
    let triangles: Triangle[] = [];
    let raf = 0;
    let running = true;
    const pointer = { x: 0, y: 0, tx: 0, ty: 0 };

    const build = () => {
      const rect = canvas.getBoundingClientRect();
      width = rect.width;
      height = rect.height;
      canvas.width = Math.max(1, Math.round(width * dpr));
      canvas.height = Math.max(1, Math.round(height * dpr));
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

      const count = Math.max(10, Math.min(64, Math.round(width * height * density)));
      triangles = Array.from({ length: count }, () => {
        const depth = 0.35 + Math.random() * 0.65;
        return {
          x: Math.random() * width,
          y: Math.random() * height,
          size: (8 + Math.random() * 26) * depth,
          rot: Math.random() * Math.PI * 2,
          spin: (Math.random() - 0.5) * 0.0022,
          vx: (Math.random() - 0.5) * 0.12 * depth,
          vy: (Math.random() - 0.5) * 0.12 * depth,
          depth,
          accent: Math.random() < 0.16,
        };
      });
    };

    const drawTriangle = (t: Triangle, ox: number, oy: number) => {
      const x = t.x + ox * t.depth;
      const y = t.y + oy * t.depth;
      ctx.save();
      ctx.translate(x, y);
      ctx.rotate(t.rot);
      ctx.beginPath();
      ctx.moveTo(0, -t.size * 0.62);
      ctx.lineTo(t.size * 0.55, t.size * 0.42);
      ctx.lineTo(-t.size * 0.55, t.size * 0.42);
      ctx.closePath();
      if (t.accent) {
        ctx.strokeStyle = `rgba(251, 54, 64, ${0.18 + t.depth * 0.45})`;
        ctx.lineWidth = 1.1;
        ctx.stroke();
      } else {
        ctx.strokeStyle = `rgba(245, 242, 237, ${0.05 + t.depth * 0.14})`;
        ctx.lineWidth = 1;
        ctx.stroke();
      }
      ctx.restore();
      return { x, y };
    };

    const render = () => {
      ctx.clearRect(0, 0, width, height);
      pointer.x += (pointer.tx - pointer.x) * 0.06;
      pointer.y += (pointer.ty - pointer.y) * 0.06;
      const ox = (pointer.x - width / 2) * 0.04;
      const oy = (pointer.y - height / 2) * 0.04;

      const points: { x: number; y: number }[] = [];
      for (const t of triangles) {
        if (running) {
          t.x += t.vx;
          t.y += t.vy;
          t.rot += t.spin;
          if (t.x < -40) t.x = width + 40;
          if (t.x > width + 40) t.x = -40;
          if (t.y < -40) t.y = height + 40;
          if (t.y > height + 40) t.y = -40;
        }
        points.push(drawTriangle(t, ox, oy));
      }

      // Constellation lines between near triangles.
      for (let i = 0; i < points.length; i++) {
        for (let j = i + 1; j < points.length; j++) {
          const dx = points[i].x - points[j].x;
          const dy = points[i].y - points[j].y;
          const dist = Math.hypot(dx, dy);
          if (dist < 132) {
            ctx.strokeStyle = `rgba(245, 242, 237, ${0.05 * (1 - dist / 132)})`;
            ctx.lineWidth = 0.6;
            ctx.beginPath();
            ctx.moveTo(points[i].x, points[i].y);
            ctx.lineTo(points[j].x, points[j].y);
            ctx.stroke();
          }
        }
      }

      if (running && !reduced) raf = requestAnimationFrame(render);
    };

    const onPointer = (event: PointerEvent) => {
      const rect = canvas.getBoundingClientRect();
      pointer.tx = event.clientX - rect.left;
      pointer.ty = event.clientY - rect.top;
    };

    build();
    pointer.x = pointer.tx = width / 2;
    pointer.y = pointer.ty = height / 2;
    render();

    const resizeObserver = new ResizeObserver(() => {
      build();
      if (reduced) render();
    });
    resizeObserver.observe(canvas);

    const visibility = new IntersectionObserver(
      ([entry]) => {
        running = entry.isIntersecting && !reduced;
        if (running) {
          cancelAnimationFrame(raf);
          raf = requestAnimationFrame(render);
        }
      },
      { threshold: 0 },
    );
    visibility.observe(canvas);

    if (interactive && !reduced) {
      window.addEventListener("pointermove", onPointer, { passive: true });
    }

    return () => {
      cancelAnimationFrame(raf);
      resizeObserver.disconnect();
      visibility.disconnect();
      window.removeEventListener("pointermove", onPointer);
    };
  }, [density, interactive]);

  return (
    <canvas
      aria-hidden="true"
      className={cn("h-full w-full", className)}
      ref={canvasRef}
    />
  );
}
