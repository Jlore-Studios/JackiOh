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
  grainVariation: 3,
  grainSpreadPercent: 2,
  crunchVariants: 4,
} as const;

type SandMark = {
  id: number;
  x: number;
  y: number;
  trail: boolean;
  grain: boolean;
  build: number;
  variation: number;
  /** #185: a landing Unit's mark on the sand, sized to its zone (`slamSand`). */
  slam?: "cracks" | "crater";
  /** The zone's width, in % of the field, for a slam mark. */
  size?: number;
};

/** #185: what a landing leaves in the sand. Cracks are never drawn under Reduce motion. */
export type SlamSand = { marks: readonly ("cracks" | "crater")[] };

/** The DOM event a landing zone dispatches; it bubbles to the field this surface listens on. */
export const SLAM_SAND_EVENT = "jk-slam-sand";

/** #185: dispatch a landing's sand marks at `zone` (the field's surface draws them under the cards). */
export function slamSand(zone: Element, sand: SlamSand): void {
  zone.dispatchEvent(new CustomEvent<SlamSand>(SLAM_SAND_EVENT, { bubbles: true, detail: sand }));
}

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
      const nextMark = (markX: number, markY: number, grain: boolean): SandMark => {
        markId.current += 1;
        return {
          id: markId.current,
          x: markX,
          y: markY,
          trail,
          grain,
          build: build.current,
          variation: variationFor(markX, markY, markId.current),
        };
      };
      const dimple = nextMark(x, y, false);
      const grains: SandMark[] = [];
      const grainCount = SAND_FEEL.baseGrains + Math.floor(dimple.variation * SAND_FEEL.grainVariation);
      for (let grain = 0; grain < grainCount; grain += 1) {
        const angle = variationFor(x, y, dimple.id + grain) * Math.PI * 2;
        const distance = SAND_FEEL.grainSpreadPercent * (0.35 + variationFor(y, x, dimple.id + grain + 1));
        grains.push(nextMark(x + Math.cos(angle) * distance, y + Math.sin(angle) * distance, true));
      }
      setMarks((previous) => [...previous, dimple, ...grains].slice(-SAND_FEEL.maximumMarks));
      // Sand has its own low-gain, polyphonic crunch path: taps are never rate-limited by normal
      // UI SFX protection, and its variant/build values make a rapid pile sound fuller.
      try {
        getAudioEngine().playSfx("sand", {
          variation: dimple.variation,
          sandVariant: Math.floor(dimple.variation * SAND_FEEL.crunchVariants),
          sandBuild: build.current,
        });
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
    // #185: a slam's cracks and crater, centred on its zone and as wide as it, under the cards.
    const slam = (event: Event): void => {
      const detail = (event as CustomEvent<SlamSand>).detail;
      const zone = event.target;
      if (!(zone instanceof Element) || detail === undefined) return;
      const bounds = element.getBoundingClientRect();
      const box = zone.getBoundingClientRect();
      if (bounds.width === 0 || bounds.height === 0) return;
      const x = ((box.left + box.width / 2 - bounds.left) / bounds.width) * 100;
      const y = ((box.top + box.height / 2 - bounds.top) / bounds.height) * 100;
      const size = (box.width / bounds.width) * 100;
      const added = detail.marks.map((kind): SandMark => {
        markId.current += 1;
        return { id: markId.current, x, y, trail: false, grain: false, build: 1, variation: variationFor(x, y, markId.current), slam: kind, size };
      });
      setMarks((previous) => [...previous, ...added].slice(-SAND_FEEL.maximumMarks));
    };
    element.addEventListener(SLAM_SAND_EVENT, slam);
    element.addEventListener("pointerdown", down);
    element.addEventListener("pointermove", move);
    element.addEventListener("pointerup", up);
    element.addEventListener("pointercancel", up);
    return () => {
      element.removeEventListener(SLAM_SAND_EVENT, slam);
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
          className={
            mark.slam !== undefined
              ? `sand-mark sand-mark--${mark.slam}`
              : mark.grain
                ? "sand-mark sand-mark--grain"
                : mark.trail
                  ? "sand-mark sand-mark--trail"
                  : "sand-mark"
          }
          key={mark.id}
          style={{
            "--sand-x": `${String(mark.x)}%`,
            "--sand-y": `${String(mark.y)}%`,
            "--sand-build": String(mark.build),
            "--sand-variation": String(mark.variation),
            "--sand-fade": `${String(SAND_FEEL.fadeMs)}ms`,
            ...(mark.size === undefined ? {} : { "--sand-zone": `${String(mark.size)}%` }),
          } as CSSProperties}
        />
      ))}
    </span>
  );
}
