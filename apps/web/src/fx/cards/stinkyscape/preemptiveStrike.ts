// R3334004: Stinkyscape #176 Preemptive Strike (`stinkyscape-176`, bespoke "preemptiveStrike"), its
// signature (R3332000): it sounds the alarm, rings of fire on its player's hero and a red spark strike
// with a shake; then the Dormant Starship it wakes for its launch is branded red, with a ring and a softer
// shake. The launch and the War Doctrine it casts are the Starship's moment (fx/starship.ts) and the
// Doctrine's own entry. Both beats draw over the event rows (`keep`). A hidden play keys none (R202,
// cardFx.ts), and no cue names a card. Every cue starts inside its entry and ends within FX_MAX_TAIL_MS
// of it (R200), within a Legendary Spell's ceiling (R3332000). It imports nothing that reads the effects
// tables (fx/starship.ts, fx/setPieces.ts): bespoke.ts imports it, and the tables read bespoke.ts.

import type { GameEvent } from "@jackioh/shared";

import { MARK_PALETTES } from "../../../cards/marks.ts";
import { targetFor } from "../../../game/animations.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_BRAND_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxBrandCue, FxPlay, FxPreset } from "../../types.ts";
import { heroOf, type CardFxPlan, type CardFxResult } from "../recipe.ts";

/** Every particle count the strike throws, at intensity "normal" (rule 9). */
export const PREEMPTIVE_TUNING = { strike: { count: 30, power: 1.3 } } as const;

/** R3334004: when the alarm rings on its player's hero, and when the strike lands, as fractions of its play's entry (rule 9). */
export const PREEMPTIVE_BEATS = { alarmAt: [0, 0.3], strikeAt: 0.25 } as const;

/** R3334004: its shakes: the strike, and the brand on the ship it wakes, softer. */
export const PREEMPTIVE_TRAUMA = { strike: 0.5, brand: 0.3 } as const;

/**
 * R3334004: the red brand on the woken ship lasts its entry and this much more, shorter than a mark's
 * (FX_BRAND_TAIL_MS), so a Spell's signature ends within its ceiling (R3332000), inside FX_MAX_TAIL_MS (R200).
 */
export const PREEMPTIVE_BRAND_TAIL_MS = 300;

/** The brand's red: the red mark's (cards/marks.ts), as a mark's brand wears its color (brand.ts). */
const BRAND_RED = MARK_PALETTES.red;

/** The preset its alarm rings and its brand's ring burn in. */
const ALARM_RING: FxPreset = "fire";

/**
 * R3334004: on its own `cardPlayed`, the alarm on its player's hero and the strike in Tech's laser red
 * (its own flavor's); on the first `awakened` of its play (the Dormant Starship it launches), the red
 * brand on the ship (its player's hero when the board cannot place it), a ring and a softer shake. Its
 * other events, the War Doctrine's among them (a play of its own), draw their own rows.
 */
export function preemptiveStrike(event: GameEvent, p: CardFxPlan, play: FxPlay): CardFxResult | null {
  const i = p.env.intensity;
  const hero = heroOf(p.view, play);
  if (event.type === "cardPlayed") {
    if (event.instanceId !== play.instanceId) return null;
    const look = paletteLook("Tech/laser") ?? FLAVOR_LOOKS.Tech;
    const strike = frac(PREEMPTIVE_BEATS.strikeAt, p.D);
    return {
      cues: [
        ...PREEMPTIVE_BEATS.alarmAt.map((share) => ringCue(p.D, ALARM_RING, hero, frac(share, p.D))),
        { ...tunedBurst(i, look.particle, hero, "area", strike, PREEMPTIVE_TUNING.strike), colors: look.colors },
        ...shakeCues(i, PREEMPTIVE_TRAUMA.strike, strike),
      ],
      row: "keep",
    };
  }
  if (event.type !== "awakened") return null;
  const own = p.entry.events.filter((candidate) => candidate.type === "awakened");
  if (own[0] !== event || play.seen.awakened !== own.length) return null;
  const ship = targetFor(event, p.view);
  const at = ship === null ? hero : tid(ship);
  const slam = frac(FX_BRAND_SLAM_AT, p.D);
  const brand: FxBrandCue = {
    kind: "brand",
    at,
    tint: { rim: BRAND_RED.rim, core: BRAND_RED.core, glow: BRAND_RED.glow },
    delayMs: 0,
    durationMs: p.D + PREEMPTIVE_BRAND_TAIL_MS,
  };
  return { cues: [brand, ringCue(p.D, ALARM_RING, at, slam), ...shakeCues(i, PREEMPTIVE_TRAUMA.brand, slam)], row: "keep" };
}
