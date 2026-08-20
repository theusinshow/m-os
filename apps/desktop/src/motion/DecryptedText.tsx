import { useEffect, useRef, useState } from "react";
import { generateDecryptedStep } from "../motion";

export interface DecryptedTextProps {
  text: string;
  duration?: number;
  speed?: number;
  className?: string;
  glyphSet?: string;
}

/**
 * DecryptedText (React Bits Decrypted Text adaptado para M/OS)
 *
 * Utilizado com moderação para estados do Hermes (pensando, buscando, executando ações)
 * e linhas de sincronização de sistema.
 */
export function DecryptedText({
  text,
  duration = 240,
  speed = 24,
  className = "",
  glyphSet = "01_/*#[]<>:;=",
}: DecryptedTextProps) {
  const [displayText, setDisplayText] = useState(text);
  const startTimeRef = useRef<number>(0);
  const intervalRef = useRef<number | null>(null);

  useEffect(() => {
    const prefersReduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (prefersReduced || duration <= 0) {
      setDisplayText(text);
      return;
    }

    startTimeRef.current = performance.now();

    if (intervalRef.current) clearInterval(intervalRef.current);

    intervalRef.current = window.setInterval(() => {
      const now = performance.now();
      const progress = Math.min(1, (now - startTimeRef.current) / duration);
      const nextText = generateDecryptedStep(text, progress, glyphSet);

      setDisplayText(nextText);

      if (progress >= 1 && intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    }, speed);

    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [text, duration, speed, glyphSet]);

  return <span className={`decrypted-text ${className}`.trim()}>{displayText}</span>;
}
