import { describe, expect, it } from "vitest";
import {
  easeOutQuart,
  generateDecryptedStep,
  interpolateNumber,
  MOTION_DURATIONS,
  MOTION_EASINGS,
  safeVariants,
} from "./motion";

describe("motion foundation", () => {
  it("defines standard durations compliant with M/OS speed principles", () => {
    expect(MOTION_DURATIONS.instant).toBe(0.075);
    expect(MOTION_DURATIONS.micro).toBe(0.12);
    expect(MOTION_DURATIONS.state).toBe(0.14);
    expect(MOTION_DURATIONS.enter).toBe(0.18);
    expect(MOTION_DURATIONS.exit).toBe(0.09);
    expect(MOTION_DURATIONS.move).toBe(0.20);
    expect(MOTION_DURATIONS.context).toBe(0.26);
    expect(MOTION_DURATIONS.slow).toBe(0.40);
  });

  it("defines spring curves", () => {
    expect(MOTION_EASINGS.tactileSpring.type).toBe("spring");
    expect(MOTION_EASINGS.tactileSpring.stiffness).toBeGreaterThan(500);
    expect(MOTION_EASINGS.defaultSpring.damping).toBeGreaterThan(20);
  });

  it("calculates decrypted text steps", () => {
    const text = "SINCRONIZANDO DADOS";
    const initial = generateDecryptedStep(text, 0);
    expect(initial.length).toBe(text.length);
    expect(initial[13]).toBe(" "); // preserves spaces

    const mid = generateDecryptedStep(text, 0.5);
    expect(mid.length).toBe(text.length);
    expect(mid.startsWith("SINCRONIZ")).toBe(true);

    const complete = generateDecryptedStep(text, 1);
    expect(complete).toBe(text);
  });

  it("interpolates numbers smoothly with easeOutQuart", () => {
    expect(easeOutQuart(0)).toBe(0);
    expect(easeOutQuart(1)).toBe(1);
    expect(easeOutQuart(0.5)).toBeGreaterThan(0.5); // Ease out starts faster

    expect(interpolateNumber(0, 100, 0)).toBe(0);
    expect(interpolateNumber(0, 100, 1)).toBe(100);
    expect(interpolateNumber(10, 20, 0.5)).toBeCloseTo(10 + 10 * easeOutQuart(0.5));
  });

  it("transforms variants safely for reduced-motion", () => {
    const original = {
      dialog: {
        initial: { opacity: 0, scale: 0.95, y: 10 },
        animate: { opacity: 1, scale: 1, y: 0 },
        exit: { opacity: 0, scale: 0.95, y: -10 },
      },
    };

    const normal = safeVariants(original, false);
    expect(normal.dialog.initial.scale).toBe(0.95);

    const reduced = safeVariants(original, true);
    expect(reduced.dialog.initial.opacity).toBe(0);
    expect(reduced.dialog.initial).not.toHaveProperty("scale");
    expect(reduced.dialog.initial).not.toHaveProperty("y");
    expect(reduced.dialog.animate.opacity).toBe(1);
    expect(reduced.dialog.animate).not.toHaveProperty("scale");
  });
});
