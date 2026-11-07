// Haptics (R669) buzz for a viewer's own plays and turns, plus damage.
// Public events only, so they neither enforce rules nor reveal information (CLAUDE.md rule 7).
// R3348000, R3348001: a touch press on a tile or a corner prop buzzes too (`buzz`, `tapHaptic`), on its
// own gap, so a tile never swallows the game's own drop.

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { reducedMotionNow } from "../game/animations.ts";
import { readHapticsSettings } from "./settings.ts";

export type HapticMoment = "drop" | "hit" | "turn" | "touch" | "tile" | "tileLegal" | "corner" | "cornerBig" | "cornerRare";

export const HAPTIC_PATTERNS: { readonly [K in HapticMoment]: readonly number[] } = {
  drop: [12],
  hit: [24],
  turn: [16, 70, 16],
  touch: [8],
  // A press on a tile (a legal one answers harder) and on a corner prop (a bigger payoff, a rare one).
  tile: [6],
  tileLegal: [12],
  corner: [8],
  cornerBig: [14, 40, 14],
  cornerRare: [10, 30, 10, 30, 24],
};

export const HAPTIC_MIN_GAP_MS = 120;

export function hapticFor(event: GameEvent, view: PlayerView): HapticMoment | null {
  switch (event.type) {
    case "cardPlayed":
      return event.player === view.viewer ? "drop" : null;
    case "damage":
      return event.amount > 0 ? "hit" : null;
    case "turnStarted":
      return event.player === view.viewer ? "turn" : null;
    default:
      return null;
  }
}

export type HapticsOptions = {
  vibrate?: ((pattern: number[]) => boolean) | null;
  now?: () => number;
  reducedMotion?: () => boolean;
};

export type Haptics = {
  onEvent(event: GameEvent, view: PlayerView): void;
  touch(): void;
  /** R3348000: a press's own buzz, behind the same switch, reduced motion and vibrate guards as `onEvent`, on its own gap. */
  buzz(moment: HapticMoment): void;
};

function browserVibrate(): ((pattern: number[]) => boolean) | null {
  try {
    const nav = typeof navigator === "undefined" ? undefined : navigator;
    return typeof nav?.vibrate === "function" ? (pattern) => nav.vibrate(pattern) : null;
  } catch {
    return null;
  }
}

export function createHaptics(options: HapticsOptions = {}): Haptics {
  const vibrate = options.vibrate === undefined ? browserVibrate() : options.vibrate;
  const now = options.now ?? (() => performance.now());
  const reduced = options.reducedMotion ?? (() => reducedMotionNow());
  let last: number | null = null;
  let lastTap: number | null = null;
  return {
    buzz(moment) {
      try {
        if (vibrate === null) return;
        if (!readHapticsSettings().vibration || reduced()) return;
        const t = now();
        if (lastTap !== null && t - lastTap < HAPTIC_MIN_GAP_MS) return;
        lastTap = t;
        vibrate([...HAPTIC_PATTERNS[moment]]);
      } catch {
        // Haptics must not interrupt the game.
      }
    },
    onEvent(event, view) {
      try {
        if (vibrate === null) return;
        const moment = hapticFor(event, view);
        if (moment === null) return;
        if (!readHapticsSettings().vibration || reduced()) return;
        const t = now();
        if (last !== null && t - last < HAPTIC_MIN_GAP_MS) return;
        last = t;
        vibrate([...HAPTIC_PATTERNS[moment]]);
      } catch {
        // Haptics must not interrupt the game.
      }
    },
    touch() {
      try {
        if (vibrate === null) return;
        if (!readHapticsSettings().vibration || reduced()) return;
        const t = now();
        if (last !== null && t - last < HAPTIC_MIN_GAP_MS) return;
        last = t;
        vibrate([...HAPTIC_PATTERNS.touch]);
      } catch {
        // Haptics must not interrupt the game.
      }
    },
  };
}

let shared: Haptics | null = null;

export function touchHaptic(): void {
  shared ??= createHaptics();
  shared.touch();
}

/** R3348000: a touch press's buzz, from the one player the arena shares. */
export function tapHaptic(moment: HapticMoment): void {
  shared ??= createHaptics();
  shared.buzz(moment);
}

export function resetTapHapticsForTests(): void {
  shared = null;
}
