// Haptics (R669) buzz for a viewer's own plays and turns, plus damage.
// They use only public events, so they neither enforce rules nor reveal information (CLAUDE.md rule 7).
// Skip unavailable devices, disabled or reduced-motion settings, and repeated calls inside HAPTIC_MIN_GAP_MS.

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { reducedMotionNow } from "../game/animations.ts";
import { readHapticsSettings } from "./settings.ts";

export type HapticMoment = "drop" | "hit" | "turn";

export const HAPTIC_PATTERNS: { readonly [K in HapticMoment]: readonly number[] } = {
  drop: [12],
  hit: [24],
  turn: [16, 70, 16],
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
  return {
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
  };
}
