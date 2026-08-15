"use client";

import { useEffect, useRef, useState } from "react";

// Measures a container so Recharts gets an explicit pixel width instead of
// ResponsiveContainer, which measures to -1 during SSR/first paint in this app.
export function useChartWidth() {
  const ref = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    const update = () => setWidth(el.clientWidth);
    update();

    const observer = new ResizeObserver(update);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  return { ref, width };
}
