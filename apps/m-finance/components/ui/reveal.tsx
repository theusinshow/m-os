"use client";

import { useEffect, useRef, useState } from "react";
import { cn } from "@/lib/utils";

/**
 * Reveals children on first scroll into view. Defaults to visible if
 * IntersectionObserver is unavailable so content is never trapped hidden.
 */
export function Reveal({
  children,
  className,
  delay = 0,
  as: Tag = "div",
}: {
  children: React.ReactNode;
  className?: string;
  delay?: number;
  as?: "div" | "section" | "li";
}) {
  const ref = useRef<HTMLElement | null>(null);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const node = ref.current;
    if (!node || typeof IntersectionObserver === "undefined") {
      setVisible(true);
      return;
    }

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setVisible(true);
          observer.disconnect();
        }
      },
      { threshold: 0.12, rootMargin: "0px 0px -8% 0px" },
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, []);

  return (
    <Tag
      className={cn("reveal", visible && "is-visible", className)}
      ref={ref as never}
      style={{ "--reveal-delay": `${delay}ms` } as React.CSSProperties}
    >
      {children}
    </Tag>
  );
}
