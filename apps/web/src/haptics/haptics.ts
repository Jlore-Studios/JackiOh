// Haptics (R669) buzz for a viewer's own plays and turns, plus damage.
// Public events only, so they neither enforce rules nor reveal information (CLAUDE.md rule 7).

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { reducedMotionNow } from "../game/animations.ts";
import { readHapticsSettings } from "./settings.ts";

export type HapticMoment = "drop" | "hit" | "turn" | "pickup";

export const HAPTIC_PATTERNS: { readonly [K in HapticMoment]: readonly number[] } = {
  drop: [12],
  hit: [24],
  turn: [16, 70, 16],
  /** #3347: lifting a card from the hand, lighter than the drop that ends the drag. */
  pickup: [8],
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
  /** #3347: a buzz for a moment the player's own input makes (a pick-up), through the same gates as an event's. */
  moment(moment: HapticMoment): void;
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
  function play(moment: HapticMoment): void {
    try {
      if (vibrate === null) return;
      if (!readHapticsSettings().vibration || reduced()) return;
      const t = now();
      if (last !== null && t - last < HAPTIC_MIN_GAP_MS) return;
      last = t;
      vibrate([...HAPTIC_PATTERNS[moment]]);
    } catch {
      // Haptics must not interrupt the game.
    }
  }
  return {
    onEvent(event, view) {
      try {
        const moment = hapticFor(event, view);
        if (moment !== null) play(moment);
      } catch {
        // Haptics must not interrupt the game.
      }
    },
    moment: play,
  };
}

let shared: Haptics | null = null;

/**
 * #3347: the one `Haptics` of the page, so the game's events and the drag's pick-up share a gap clock
 * (`HAPTIC_MIN_GAP_MS`) and never buzz over each other.
 */
export function sharedHaptics(): Haptics {
  // The vibrate hook looks `navigator.vibrate` up at each buzz, as a device may gain it after this is made.
  shared ??= createHaptics({
    vibrate: (pattern) => {
      const nav = typeof navigator === "undefined" ? undefined : navigator;
      return typeof nav?.vibrate === "function" ? nav.vibrate(pattern) : false;
    },
  });
  return shared;
}

export function resetSharedHapticsForTests(): void {
  shared = null;
}
