// R3399000: Classic+ J-lease J-Jungle EX-plorer Pack (`classicplus-075-1`, bespoke "relicSpill"), its signature (#3399):
// Cast (it casts when drawn): a battered explorer's pack bursts out of the caster's deck pile (leaf and dust
// there, a ghost flying out), lands open on the caster's middle unit zone with a thump (dust, a ring, a
// shake). Text: shiny relics spill out of it and each flies to the caster's hand as a glinting card back
// (`addedToHand`: a ghost and a gold glint each, one after another), a jungle bird swooping after the last
// (a leaf bolt from the board's upper edge to the hand pile).
// R202: no cue names a card; the relics are ghost backs, the opponent sees the same.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { animTestid } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import { motifShot } from "../../motifs/build.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the pack throws, at intensity "normal" (rule 9). */
const TUNING = {
  burst: { count: 10, power: 0.9 },
  thump: { count: 14, power: 1 },
  glint: { count: 6, power: 0.6 },
  bird: { count: 6, power: 0.5 },
} as const;

const PACK_LANE = 3;
const BURST_AT = 0;
const THUMP_AT = 0.2;
const THUMP_SHAKE = 0.5;
const RELIC_AT = 0.05;
const RELIC_STEP = 0.05;
const RELIC_LAND_AT = 0.3;
const BIRD_FROM: FxAnchor = { kind: "viewport", at: { x: 0.9, y: 0.1 } };
const FIXTURE_RELICS = 5;

/** Classic+ #75.1 Pack: the burst and thump on its cast, a relic's flight on each card it adds. */
export const relicSpill: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  const i = p.env.intensity;
  const look = FLAVOR_LOOKS.Nature;
  const mine = sideOf(p.view, play.player);
  const pack = tid(testid.zone(mine, "units", PACK_LANE));
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const deck = tid(animTestid.library(mine));
    const cues: FxCue[] = [
      { ...tunedBurst(i, "leaf", deck, "area", frac(BURST_AT, p.D), TUNING.burst), colors: look.colors },
      { kind: "ghost", from: deck, to: pack, delayMs: frac(BURST_AT, p.D), durationMs: frac(THUMP_AT - BURST_AT, p.D) },
      ringCue(p.D, "dust", pack, frac(THUMP_AT, p.D)),
      { ...tunedBurst(i, "dust", pack, "area", frac(THUMP_AT, p.D), TUNING.thump), colors: look.colors },
      ...shakeCues(i, THUMP_SHAKE, frac(THUMP_AT, p.D)),
    ];
    return { cues, row: "keep" };
  }
  if (event.type === "addedToHand") {
    const relics = p.entry.events.filter((e) => e.type === "addedToHand");
    const k = Math.max(0, relics.indexOf(event));
    const start = frac(RELIC_AT + k * RELIC_STEP, p.D);
    const land = frac(RELIC_LAND_AT + k * RELIC_STEP, p.D);
    const hand = tid(animTestid.hand(sideOf(p.view, event.player)));
    const cues: FxCue[] = [
      { kind: "ghost", from: pack, to: hand, delayMs: start, durationMs: land - start },
      { ...tunedBurst(i, "gold", hand, "point", land, TUNING.glint), colors: look.colors },
    ];
    if (k === relics.length - 1) {
      cues.push(motifShot({ look, scale: i, row: "keep" }, "bolt", BIRD_FROM, hand, start, land - start), { ...tunedBurst(i, "leaf", hand, "point", land, TUNING.bird), colors: look.colors });
    }
    return { cues, row: "keep" };
  }
  return null;
};

/** The gallery's Pack: five relic cards reach its player's hand. */
export function relicSpillFixture(_view: PlayerView, _instanceId: string, player: PlayerId): GameEvent[] {
  return Array.from({ length: FIXTURE_RELICS }, (_, k): GameEvent => ({ type: "addedToHand", player, instanceId: `fx-relic-${String(k)}`, defId: "core-005" }));
}
