// How much of the card-motion flourishes of #3347 a player's settings allow: the flight and
// spring of a hand card, the deck's riffle, the opening deal and the drag's tilt and return.
// View state only, from the player's own settings (CLAUDE.md rule 7).
//
//   "still": reduced motion. Every move snaps to its end, with no flight and no deal.
//   "calm": fx intensity off. The sounds stay; the flourishes (riffle leaves, deal flights, tilt,
//           return spring, overshoot) go.
//   "full": everything.

import { getFxSettings, type FxIntensity } from "../fx/settings.ts";
import { reducedMotionNow } from "../settings/motion.ts";

export type CardMotion = "full" | "calm" | "still";

export function cardMotionLevel(reduced: boolean, intensity: FxIntensity): CardMotion {
  if (reduced) return "still";
  return intensity === "off" ? "calm" : "full";
}

/** The level the settings give right now, read at each call. */
export function currentCardMotion(): CardMotion {
  const fx = getFxSettings();
  return cardMotionLevel(reducedMotionNow(fx), fx.intensity);
}
