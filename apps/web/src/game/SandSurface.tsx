// A cosmetic, input-transparent fidget layer for the empty playmat. It reads neither rules nor
// hidden state: the board owns the cards above it, and this layer only records pointer marks in
// gaps which did not belong to a card or a board control.

import { useEffect, useRef, useState, type CSSProperties, type ReactElement, type RefObject } from "react";

import { getAudioEngine } from "../audio/engine.ts";

/** Tunables kept together so the surface can be retuned without changing input code. */
export const SAND_FEEL = {
  fadeMs: 8_000,
  maximumMarks: 180,
  minimumTrailDistancePx: 7,
  settleMs: 650,
  maximumBuild: 6,
  baseGrains: 4,
} as const;

type SandMark = { id: number; x: number; y: number; trail: boolean; build: number; variation: number };

export type SandSurfaceProps = {
  field: RefObject<HTMLElement | null>;
  /** The action builder owns this signal: while it has a card lifted or targeted, sand is inert. */
  disabled: boolean;
};

function isEmptySurfaceTarget(target: EventTarget | null): target is Element {
  if (!(target instanceof Element)) return false;
  if (target.closest(".card, button, input, select, textarea, [role=button], .hero, .seat, .control-bar") !== null) return false;
  const zone = target.closest<HTMLElement>("[data-testid^='zone-']");
  // A blank lane slot is part of the sand; a slot containing a card always belongs to that card.
  return zone === null || zone.querySelector(".card") === null;
}

function variationFor(x: number, y: number, sequence: number): number {
  const value = Math.abs(Math.sin(x * 13.17 + y * 7.31 + sequence * 0.91));
  return value;
}

export default function SandSurface({ field, disabled }: SandSurfaceProps): ReactElement {
  const [marks, setMarks] = useState<readonly SandMark[]>([]);
  const disabledRef = useRef(disabled);
  const markId = useRef(0);
  const pointer = useRef<{ id: number; x: number; y: number } | null>(null);
  const lastTap = useRef<number | null>(null);
  const build = useRef(0);
  disabledRef.current = disabled;

  useEffect(() => {
    const element = field.current;
    if (element === null) return undefined;

    const addMark = (event: PointerEvent, trail: boolean): void => {
      if (disabledRef.current || !isEmptySurfaceTarget(event.target)) return;
      if (document.documentElement.hasAttribute("data-dragging")) return;
      const bounds = element.getBoundingClientRect();
      if (bounds.width === 0 || bounds.height === 0) return;
      const x = ((event.clientX - bounds.left) / bounds.width) * 100;
      const y = ((event.clientY - bounds.top) / bounds.height) * 100;
      const now = performance.now();
      build.current = lastTap.current === null || now - lastTap.current > SAND_FEEL.settleMs
        ? 1
        : Math.min(SAND_FEEL.maximumBuild, build.current + 1);
      lastTap.current = now;
      markId.current += 1;
      const variation = variationFor(x, y, markId.current);
      const mark: SandMark = { id: markId.current, x, y, trail, build: build.current, variation };
      setMarks((previous) => [...previous, mark].slice(-SAND_FEEL.maximumMarks));
      // `poof` is the existing low, filtered-grain recipe. Variation makes rapid taps feel less
      // mechanical without touching gameplay RNG; the visual mark is never rate limited.
      try {
        getAudioEngine().playSfx("poof", { variation });
      } catch {
        // Web Audio can be unavailable before a gesture unlock; the cosmetic mark still appears.
      }
    };

    const down = (event: PointerEvent): void => {
      addMark(event, false);
      if (disabledRef.current || !isEmptySurfaceTarget(event.target)) return;
      pointer.current = { id: event.pointerId, x: event.clientX, y: event.clientY };
    };
    const move = (event: PointerEvent): void => {
      const active = pointer.current;
      if (active === null || active.id !== event.pointerId || event.buttons === 0) return;
      const distance = Math.hypot(event.clientX - active.x, event.clientY - active.y);
      if (distance < SAND_FEEL.minimumTrailDistancePx) return;
      pointer.current = { id: event.pointerId, x: event.clientX, y: event.clientY };
      addMark(event, true);
    };
    const up = (event: PointerEvent): void => {
      if (pointer.current?.id === event.pointerId) pointer.current = null;
    };
    element.addEventListener("pointerdown", down);
    element.addEventListener("pointermove", move);
    element.addEventListener("pointerup", up);
    element.addEventListener("pointercancel", up);
    return () => {
      element.removeEventListener("pointerdown", down);
      element.removeEventListener("pointermove", move);
      element.removeEventListener("pointerup", up);
      element.removeEventListener("pointercancel", up);
    };
  }, [field]);

  return (
    <span className="sand-surface" aria-hidden="true">
      {marks.map((mark) => (
        <span
          className={mark.trail ? "sand-mark sand-mark--trail" : "sand-mark"}
          key={mark.id}
          style={{
            "--sand-x": `${String(mark.x)}%`,
            "--sand-y": `${String(mark.y)}%`,
            "--sand-build": String(mark.build),
            "--sand-variation": String(mark.variation),
            "--sand-fade": `${String(SAND_FEEL.fadeMs)}ms`,
          } as CSSProperties}
        />
      ))}
    </span>
  );
}
