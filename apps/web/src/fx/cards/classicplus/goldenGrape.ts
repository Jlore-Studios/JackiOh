// R3399000: Classic+ Golden Grape (`classicplus-065-4`, bespoke "grapeBounce"), its signature (#3399):
// Cast (its own `cardPlayed`): a single golden grape drops from the board's top edge (a gold projectile),
// bounces once on the board's centre (a gold ring and splash, then a smaller hop) and glows there like a
// jelly bean held up to the light (radiant rays and a soft sparkle).
// Text: it rolls to the chosen card and bursts in a glossy splash (`radiantSet`: a gold projectile from the
// centre, a ring and a gold and sparkle splash on the card), and the row's Radiant pulse still plays; each
// card beside it the Radiant face also turns gets a smaller splash of its own. A card in hand is no board
// target: its owner sees the grape roll to and burst on the hand pile, the opponent only a shimmer over the
// pile. Nothing moves from the concept. R202: no cue names a card, a hand card is drawn only as its pile,
// and the opponent never sees which.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { animTestid, targetFor } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, raysCue, ringCue, tid, tunedBurst, type BurstTuning } from "../../build.ts";
import { FX_CENTER } from "../../constants.ts";
import type { FxAnchor, FxCue, FxPreset, FxSpread } from "../../types.ts";
import { projectile, type CardRecipe } from "../recipe.ts";

/** Every particle count the grape throws, at intensity "normal" (rule 9). */
const TUNING = {
  bounce: { count: 12, power: 1 },
  hop: { count: 6, power: 0.6 },
  glow: { count: 10, power: 0.5 },
  splash: { count: 16, power: 1.1 },
  gloss: { count: 8, power: 0.6 },
  droplet: { count: 8, power: 0.8 },
} as const;

/** The grape's gold, hot first. */
const GOLD = ["#fff7cc", "#facc15", "#b45309"] as const;

/** Where it drops from (the board's top edge) and where it bounces (the board's centre). */
const SKY: FxAnchor = { kind: "viewport", at: { x: FX_CENTER.x, y: 0 } };
const CENTER: FxAnchor = { kind: "viewport", at: { x: FX_CENTER.x, y: FX_CENTER.y } };

/**
 * The beats as shares of D, every burst inside a Legendary Spell's first 300 ms at the cast's 400 ms: it
 * drops at DROP_AT and lands at BOUNCE_AT, hops at HOP_AT and glows from GLOW_AT; on its target it rolls
 * from ROLL_AT and bursts at SPLASH_AT.
 */
const DROP_AT = 0;
const BOUNCE_AT = 0.25;
const HOP_AT = 0.45;
const GLOW_AT = 0.6;
const ROLL_AT = 0;
const SPLASH_AT = 0.4;

/** Classic+ #65.4 Golden Grape: the drop and bounce on its cast, the roll and splash on each card it turns Radiant. */
export const grapeBounce: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  const i = p.env.intensity;
  const burst = (preset: FxPreset, at: FxAnchor, spread: FxSpread, delay: number, tuning: BurstTuning): FxCue => ({
    ...tunedBurst(i, preset, at, spread, delay, tuning),
    colors: GOLD,
  });
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const bounce = frac(BOUNCE_AT, p.D);
    const cues: FxCue[] = [
      projectile(i, "gold", SKY, CENTER, frac(DROP_AT, p.D), bounce - frac(DROP_AT, p.D)),
      ringCue(p.D, "gold", CENTER, bounce),
      burst("gold", CENTER, "point", bounce, TUNING.bounce),
      burst("gold", CENTER, "point", frac(HOP_AT, p.D), TUNING.hop),
      raysCue(p.D, "radiant", CENTER, frac(GLOW_AT, p.D)),
      burst("sparkle", CENTER, "area", frac(GLOW_AT, p.D), TUNING.glow),
    ];
    return { cues, row: "keep" };
  }
  if (event.type !== "radiantSet") return null;
  // The play's first card turned is the one it rolls to; an entry may hold several.
  const same = p.entry.events.filter((e) => e.type === "radiantSet");
  const first = (play.seen.radiantSet ?? same.length) - same.length + Math.max(0, same.indexOf(event)) === 0;
  const splash = frac(SPLASH_AT, p.D);
  const roll = frac(ROLL_AT, p.D);
  if (event.zone.z === "hand") {
    const hand = tid(animTestid.hand(sideOf(p.view, event.zone.player)));
    // R202: the opponent sees a shimmer over the pile and nothing of the card.
    if (event.zone.player !== p.view.viewer) return { cues: [{ kind: "sheen", at: hand, delayMs: splash, durationMs: p.D - splash }], row: "keep" };
    const cues: FxCue[] = [burst("gold", hand, "point", splash, first ? TUNING.splash : TUNING.droplet), burst("sparkle", hand, "area", splash, TUNING.gloss)];
    if (first) cues.unshift(projectile(i, "gold", CENTER, hand, roll, splash - roll));
    return { cues, row: "keep" };
  }
  const tgt = targetFor(event, p.view);
  if (tgt === null || event.zone.z !== "field") return null;
  const at = tid(tgt);
  if (!first) return { cues: [burst("gold", at, "point", splash, TUNING.droplet)], row: "keep" };
  return {
    cues: [projectile(i, "gold", CENTER, at, roll, splash - roll), ringCue(p.D, "gold", at, splash), burst("gold", at, "area", splash, TUNING.splash), burst("sparkle", at, "point", splash, TUNING.gloss)],
    row: "keep",
  };
};

/** The gallery's Golden Grape: it turns its player's first unit Radiant. */
export function grapeBounceFixture(view: PlayerView, _instanceId: string, player: PlayerId): GameEvent[] {
  const units = view[sideOf(view, player)].units;
  const lane = units.findIndex((unit) => unit !== null);
  const unit = units[lane];
  if (unit === null || unit === undefined) return [];
  return [{ type: "radiantSet", instanceId: unit.instanceId, defId: unit.defId, zone: { z: "field", player, row: "units", lane: lane + 1 } }];
}
