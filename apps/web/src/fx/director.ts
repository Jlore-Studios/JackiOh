// S8 effects director. R200: it only decorates; every visual stays within planned duration, FX_MAX_TAIL_MS and FX_MAX_PARTICLE_LIFE_MS.
// Cues measure anchors when firing and expire from scheduled due time; frames fire, update, shake, draw, expire/track, adapt, then loop only while active work remains.

import { createCanvasFx } from "./canvasFx.ts";
import {
  FX_ADAPT_DISPLAY_MAX_MS,
  FX_ADAPT_MIN_INTERVAL_MS,
  FX_ADAPT_RECOVER_WINDOWS,
  FX_ADAPT_SLOW_FACTOR,
  FX_ADAPT_SLOW_MS,
  FX_ADAPT_WINDOW,
  FX_LUNGE_MAX_PX,
  FX_LUNGE_MIN_PX,
  FX_LUNGE_STANDOFF,
  FX_MOBILE_WIDTH,
  FX_PARTICLE_CAP,
  FX_PARTICLE_CAP_MIN,
  FX_PARTICLE_CAP_MOBILE,
} from "./constants.ts";
import { landingBox, mountDomEffect, mountHold, type DomEffect, type DomEffectBoxes } from "./dom.ts";
import { createFrameLoop, type FxFrame } from "./loop.ts";
import { createParticleSystem } from "./particles.ts";
import { pointIn } from "./anchors.ts";
import { createRng } from "./rng.ts";
import { createShake } from "./shake.ts";
import type { FxSurface } from "./surface.ts";
import type {
  FxAnchor,
  FxBox,
  FxCue,
  FxDomCue,
  FxFrameSource,
  FxPoint,
  FxShakeSink,
  FxStageCue,
  FxVisibility,
} from "./types.ts";

export function capacityFor(viewportWidth: number): number {
  return viewportWidth < FX_MOBILE_WIDTH ? FX_PARTICLE_CAP_MOBILE : FX_PARTICLE_CAP;
}

export type FxDirectorOptions = {
  surface: FxSurface | null; // null: canvas cues (burst, projectile, crack, ring) are dropped
  domRoot: HTMLElement; // DOM cues mount here via mountDomEffect (S10)
  now: () => number;
  frames: FxFrameSource;
  visibility: FxVisibility;
  measure: (anchor: FxAnchor) => FxBox | null;
  shakeSink: FxShakeSink;
  seed: number;
  capacity: number;
  /** Stage-cue target lookup; defaults to a `document` query. */
  element?: (testid: string) => HTMLElement | null;
  /** Scroll/resize subscription for tracking parked stand-ins; defaults to window events. */
  viewport?: FxViewportEvents;
};

export type FxViewportEvents = { subscribe(listener: () => void): () => void };

export type FxDirector = {
  /** Schedules cues for their first due frame; drops them while hidden. */
  play(cues: readonly FxCue[]): void;
  /** Removes all effects and resets the shake sink. */
  clear(): void;
  /** Removes a pending or active turn banner and its rays once the player acts. */
  dismissBanner(): void;
  /** Clears stand-ins, concealment and lunges for a newer board view (B46–B48). */
  release(): void;
  /** Freezes visual time without changing the animation queue or game clock. */
  pause(): void;
  /** Resumes a hit-stop, shifting visual deadlines by the frozen span. */
  resume(): void;
  /** Pending, DOM, canvas and shake activity. */
  active(): number;
  particles(): number;
  capacity(): number;
  dispose(): void;
};

type PendingCue = { cue: FxCue; due: number };
type MountedEffect = { effect: DomEffect; expiresAt: number; kind: FxDomCue["kind"]; tone?: string };
/** Stage effect cleanup and optional movement tracking. */
type StagedEffect = { undo(): void; track?: () => void; expiresAt: number };

function defaultViewport(): FxViewportEvents {
  return {
    subscribe(listener) {
      if (typeof window === "undefined") return () => undefined;
      const options = { capture: true, passive: true } as const;
      window.addEventListener("scroll", listener, options);
      window.addEventListener("resize", listener, options);
      return () => {
        window.removeEventListener("scroll", listener, options);
        window.removeEventListener("resize", listener, options);
      };
    },
  };
}

const sameBox = (a: FxBox, b: FxBox): boolean =>
  a.x === b.x && a.y === b.y && a.width === b.width && a.height === b.height;

/** Mirror a hidden source card's animation onto its stand-in; `cardPlayed` remains on the source. */
function mirrorAnimating(source: Element, copy: Element): () => void {
  const sync = (): void => {
    const value = source.getAttribute("data-animating");
    if (value !== null && value !== "cardPlayed") copy.setAttribute("data-animating", value);
    else copy.removeAttribute("data-animating");
  };
  sync();
  if (typeof MutationObserver === "undefined") return () => undefined;
  const observer = new MutationObserver(sync);
  observer.observe(source, { attributes: true, attributeFilter: ["data-animating"] });
  return () => observer.disconnect();
}

function isStageCue(cue: FxCue): cue is FxStageCue {
  return cue.kind === "hold" || cue.kind === "conceal" || cue.kind === "lunge";
}

function defaultElement(testid: string): HTMLElement | null {
  if (typeof document === "undefined") return null;
  return document.querySelector<HTMLElement>(`[data-testid="${testid.replace(/["\\]/g, "\\$&")}"]`);
}

function cardIn(element: HTMLElement | null): HTMLElement | null {
  if (element === null) return null;
  if (element.classList.contains("card")) return element;
  const cards = element.querySelectorAll<HTMLElement>(".card");
  return cards.length > 0 ? (cards[cards.length - 1] ?? null) : null;
}

function boxOfElement(element: Element): FxBox | null {
  const rect = element.getBoundingClientRect();
  if (rect.width === 0 && rect.height === 0) return null;
  return { x: rect.left, y: rect.top, width: rect.width, height: rect.height };
}

function isCanvasCue(cue: FxCue): boolean {
  return cue.kind === "burst" || cue.kind === "projectile" || cue.kind === "crack" || cue.kind === "ring";
}

function atOf(anchor: FxAnchor): FxPoint | undefined {
  return anchor.kind === "testid" ? anchor.at : undefined;
}

export function createFxDirector(options: FxDirectorOptions): FxDirector {
  const { surface, domRoot, now, frames, visibility, shakeSink, seed } = options;
  const element = options.element ?? defaultElement;

  // Effects aimed at a carried card must target its stand-in, not the concealed source.
  const carried = new Map<string, HTMLElement>();
  const measure = (anchor: FxAnchor): FxBox | null => {
    if (anchor.kind === "testid") {
      const standIn = carried.get(anchor.testid);
      if (standIn !== undefined) return boxOfElement(standIn) ?? options.measure(anchor);
    }
    return options.measure(anchor);
  };

  let cap = options.capacity;
  const particles = createParticleSystem({ capacity: cap, rng: createRng(seed) });
  const canvasFx = createCanvasFx({ particles, rng: createRng(seed + 1) });
  const shake = createShake({ seed });

  let pending: PendingCue[] = [];
  let mounted: MountedEffect[] = [];
  let staged: StagedEffect[] = [];
  let shaking = false;
  let disposed = false;
  let painted = false;
  let layoutDirty = false;
  let pausedAt: number | null = null;

  // B34: compare each raw-frame window with learned display refresh so 30 Hz is not mistaken for load.
  // Check accumulated time every frame so a stall lowers capacity promptly.
  const initialCap = cap;
  let displayMs = Number.POSITIVE_INFINITY;
  let windowCount = 0;
  let windowSum = 0;
  let healthyWindows = 0;

  const resetWindow = (): void => {
    windowCount = 0;
    windowSum = 0;
  };

  const setCap = (next: number): void => {
    if (next === cap) return;
    cap = next;
    particles.setCapacity(cap);
  };

  const adapt = (frame: FxFrame): void => {
    // A wake-to-first-frame interval reveals no display refresh.
    if (frame.first) return;
    const raw = Math.max(0, frame.raw);
    if (raw >= FX_ADAPT_MIN_INTERVAL_MS) displayMs = Math.min(displayMs, raw);
    const refresh = Math.min(displayMs, FX_ADAPT_DISPLAY_MAX_MS);
    const slowMs = Math.max(FX_ADAPT_SLOW_MS, refresh * FX_ADAPT_SLOW_FACTOR);
    windowCount += 1;
    windowSum += raw;
    if (windowSum > FX_ADAPT_WINDOW * slowMs) {
      setCap(Math.max(FX_PARTICLE_CAP_MIN, Math.floor(cap / 2)));
      healthyWindows = 0;
      resetWindow();
      return;
    }
    if (windowCount < FX_ADAPT_WINDOW) return;
    resetWindow();
    healthyWindows += 1;
    if (healthyWindows >= FX_ADAPT_RECOVER_WINDOWS && cap < initialCap) {
      setCap(Math.min(initialCap, cap * 2));
      healthyWindows = 0;
    }
  };

  const mountDom = (cue: FxDomCue, due: number, at: number): void => {
    const expiresAt = due + cue.durationMs;
    // Do not mount an effect whose lifetime passed in a stalled tab.
    if (expiresAt <= at) return;
    let boxes: DomEffectBoxes = {};
    switch (cue.kind) {
      case "splat":
      case "rays":
      case "sheen":
      case "arrows":
      case "fracture":
      case "walls":
      case "brand":
      case "shield": {
        const box = measure(cue.at);
        if (box === null) return;
        boxes = { at: box };
        break;
      }
      case "zone": {
        const box = measure(cue.at);
        if (box === null) return;
        boxes = { at: box };
        break;
      }
      case "ghost":
      case "fog": {
        const from = measure(cue.from);
        if (from === null) return;
        const to = measure(cue.to);
        if (to === null) return;
        boxes = { from, to };
        break;
      }
      // Viewport effects need no anchor.
    }
    // Never overlap fading banners.
    if (cue.kind === "banner") {
      mounted = mounted.filter((item) => {
        if (item.kind !== "banner") return true;
        item.effect.remove();
        return false;
      });
    }
    const effect = mountDomEffect(domRoot, cue, boxes);
    if (effect !== null) mounted.push({ effect, expiresAt, kind: cue.kind, ...(cue.kind === "rays" ? { tone: cue.tone } : {}) });
  };

  const stage = (cue: FxStageCue, due: number, at: number): void => {
    const expiresAt = due + cue.durationMs;
    if (expiresAt <= at) return;
    switch (cue.kind) {
      case "hold": {
        const zone = measure(cue.to);
        if (zone === null) return;
        const source = cue.from !== null && cue.from.kind === "testid" ? cardIn(element(cue.from.testid)) : null;
        const sourceBox = source !== null ? boxOfElement(source) : null;
        const copied = sourceBox !== null ? source : null;
        // R502: an empty `from` still supplies the stand-in's origin.
        const from = sourceBox ?? (source === null && cue.from !== null ? measure(cue.from) : null);
        const parent = copied?.parentElement ?? null;
        const font = parent !== null ? (parent.ownerDocument.defaultView?.getComputedStyle(parent).fontSize ?? null) : null;
        // Track only board movement; translate same-size stand-ins without layout.
        let placed = landingBox(zone, sourceBox);
        let shown = placed;
        const hold = mountHold(domRoot, cue, { source: copied, from, land: placed, font });
        const carriedId = copied !== null && cue.from !== null && cue.from.kind === "testid" && copied === element(cue.from.testid)
          ? cue.from.testid
          : null;
        if (carriedId !== null) carried.set(carriedId, hold.el);
        const copy = hold.el.firstElementChild;
        const unmirror = carriedId !== null && copied !== null && copy !== null ? mirrorAnimating(copied, copy) : () => undefined;
        staged.push({
          undo: () => {
            unmirror();
            if (carriedId !== null && carried.get(carriedId) === hold.el) carried.delete(carriedId);
            hold.remove();
          },
          track: () => {
            const next = measure(cue.to);
            if (next === null) return;
            const land = landingBox(next, sourceBox);
            if (sameBox(land, shown)) return;
            shown = land;
            if (land.width !== placed.width || land.height !== placed.height) {
              placed = land;
              hold.place(land);
              hold.shift(0, 0);
            } else {
              hold.shift(land.x - placed.x, land.y - placed.y);
            }
          },
          expiresAt,
        });
        return;
      }
      case "conceal": {
        const target = element(cue.testid);
        if (target === null) return;
        target.setAttribute("data-fx-concealed", cue.mode);
        staged.push({
          undo: () => {
            if (target.getAttribute("data-fx-concealed") === cue.mode) target.removeAttribute("data-fx-concealed");
          },
          expiresAt,
        });
        return;
      }
      case "lunge": {
        const attacker = element(cue.attacker);
        const target = element(cue.target);
        if (attacker === null || target === null) return;
        const a = boxOfElement(attacker);
        const b = boxOfElement(target);
        if (a === null || b === null) return;
        const dx = b.x + b.width / 2 - (a.x + a.width / 2);
        const dy = b.y + b.height / 2 - (a.y + a.height / 2);
        const distance = Math.hypot(dx, dy);
        if (!(distance > 0)) return;
        // Stop at the target's near edge.
        const ux = Math.abs(dx) / distance;
        const uy = Math.abs(dy) / distance;
        const extents = (ux * (a.width + b.width) + uy * (a.height + b.height)) / 2;
        const reach = Math.min(FX_LUNGE_MAX_PX, Math.max(FX_LUNGE_MIN_PX, distance - extents * FX_LUNGE_STANDOFF));
        attacker.style.setProperty("--fx-lunge-x", `${((dx / distance) * reach).toFixed(1)}px`);
        attacker.style.setProperty("--fx-lunge-y", `${((dy / distance) * reach).toFixed(1)}px`);
        staged.push({
          undo: () => {
            attacker.style.removeProperty("--fx-lunge-x");
            attacker.style.removeProperty("--fx-lunge-y");
          },
          expiresAt,
        });
        return;
      }
    }
  };

  const releaseStaged = (): void => {
    const undone = staged;
    staged = [];
    for (const item of undone) item.undo();
  };

  /** R200 safety expiry for staged effects. */
  const expireStaged = (at: number): void => {
    if (staged.length === 0) return;
    const kept: StagedEffect[] = [];
    for (const item of staged) {
      if (item.expiresAt <= at) item.undo();
      else kept.push(item);
    }
    staged = kept;
  };

  const fire = (cue: FxCue, due: number, at: number): void => {
    if (isStageCue(cue)) {
      stage(cue, due, at);
      return;
    }
    switch (cue.kind) {
      case "burst": {
        const box = measure(cue.at);
        if (box === null) return;
        const origin = pointIn(box, atOf(cue.at));
        particles.emit(cue.preset, origin.x, origin.y, {
          count: cue.count,
          spread: cue.spread,
          box,
          power: cue.power,
          ...(cue.scale === undefined ? {} : { scale: cue.scale }),
        });
        canvasFx.flash(cue.preset, origin, box, cue.count);
        return;
      }
      case "projectile": {
        const from = measure(cue.from);
        if (from === null) return;
        const to = measure(cue.to);
        if (to === null) return;
        canvasFx.projectile(cue.preset, pointIn(from, atOf(cue.from)), pointIn(to, atOf(cue.to)), cue.flightMs, cue.density);
        return;
      }
      case "crack": {
        const box = measure(cue.at);
        if (box === null) return;
        canvasFx.crack(box, cue.durationMs);
        return;
      }
      case "ring": {
        const box = measure(cue.at);
        if (box === null) return;
        canvasFx.ring(cue.preset, box, cue.durationMs);
        return;
      }
      case "shake":
        shake.add(cue.trauma);
        return;
      default:
        mountDom(cue, due, at);
        return;
    }
  };

  const hasWork = (): boolean =>
    pending.length > 0 ||
    mounted.length > 0 ||
    layoutDirty ||
    particles.alive() > 0 ||
    canvasFx.alive() > 0 ||
    shake.active();

  const onFrame = (frame: FxFrame): boolean => {
    if (pausedAt !== null) return false;
    const at = frame.now;

    if (pending.length > 0) {
      const due: PendingCue[] = [];
      const later: PendingCue[] = [];
      for (const item of pending) (item.due <= at ? due : later).push(item);
      pending = later;
      for (const item of due) fire(item.cue, item.due, at);
    }

    const age = Math.max(0, frame.raw);
    particles.step(frame.dt, age);
    canvasFx.step(frame.dt, age);
    shake.step(age);

    // A final shake clear also moves the board back for stand-in tracking.
    const boardMoved = shake.active() || shaking;
    if (shake.active()) {
      shakeSink.apply(shake.sample(at));
      shaking = true;
    } else if (shaking) {
      shakeSink.clear();
      shaking = false;
    }

    if (surface !== null) {
      const live = particles.alive() > 0 || canvasFx.alive() > 0;
      if (live || painted) {
        surface.clear();
        particles.draw(surface.ctx, surface.dpr());
        canvasFx.draw(surface.ctx);
        painted = live;
      }
    }

    if (mounted.length > 0) {
      const kept: MountedEffect[] = [];
      for (const item of mounted) {
        if (item.expiresAt <= at) item.effect.remove();
        else kept.push(item);
      }
      mounted = kept;
    }
    expireStaged(at);
    if (boardMoved || layoutDirty) for (const item of staged) item.track?.();
    layoutDirty = false;

    adapt(frame);

    return hasWork();
  };

  const clear = (): void => {
    pending = [];
    for (const item of mounted) item.effect.remove();
    mounted = [];
    releaseStaged();
    particles.clear();
    canvasFx.clear();
    shake.reset();
    shaking = false;
    shakeSink.clear();
    surface?.clear();
    painted = false;
  };

  const loop = createFrameLoop({
    frames,
    visibility,
    now,
    onFrame,
    // Drop stale hidden-tab cues rather than replay a visual backlog.
    onResume: clear,
  });

  const unsubscribeViewport = (options.viewport ?? defaultViewport()).subscribe(() => {
    if (disposed || staged.length === 0) return;
    layoutDirty = true;
    loop.wake();
  });

  return {
    play(cues: readonly FxCue[]): void {
      if (disposed || cues.length === 0) return;
      if (visibility.hidden()) return;
      const playTime = now();
      expireStaged(playTime);
      let added = 0;
      for (const cue of cues) {
        if (surface === null && isCanvasCue(cue)) continue;
        // Due-now stage cues must see this task's board state.
        if (isStageCue(cue) && cue.delayMs <= 0) stage(cue, playTime, playTime);
        else pending.push({ cue, due: playTime + cue.delayMs });
        added += 1;
      }
      if (added > 0) loop.wake();
    },
    clear,
    dismissBanner(): void {
      pending = pending.filter((item) => item.cue.kind !== "banner");
      if (!mounted.some((item) => item.kind === "banner")) return;
      // Only turn-banner rays share its `victory` tone.
      mounted = mounted.filter((item) => {
        const goes = item.kind === "banner" || (item.kind === "rays" && item.tone === "victory");
        if (goes) item.effect.remove();
        return !goes;
      });
    },
    release: releaseStaged,
    pause(): void {
      if (disposed || pausedAt !== null) return;
      pausedAt = now();
    },
    resume(): void {
      if (disposed || pausedAt === null) return;
      const elapsed = Math.max(0, now() - pausedAt);
      pausedAt = null;
      for (const item of pending) item.due += elapsed;
      for (const item of mounted) item.expiresAt += elapsed;
      for (const item of staged) item.expiresAt += elapsed;
      if (hasWork()) loop.wake();
    },
    active(): number {
      return pending.length + mounted.length + staged.length + canvasFx.alive() + (shake.active() ? 1 : 0);
    },
    particles(): number {
      return particles.alive();
    },
    capacity(): number {
      return cap;
    },
    dispose(): void {
      if (disposed) return;
      clear();
      disposed = true;
      unsubscribeViewport();
      loop.dispose();
    },
  };
}
