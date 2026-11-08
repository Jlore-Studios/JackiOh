// R3399000: Classic+ Frostspatula (`classicplus-012-8`, entrance "griddleFreeze", bespoke "spatulaFlip"), its signature (#3399):
// Entrance: a frozen griddle rises in the zone, frost spreading across it (early frost, a ring), and a
// rime-coated spatula slams down point first at the slam and stands upright (shards and a shake).
// Text: its "flip back" is its Death (the engine's `death` hook, then one `summoned` per Unit it destroyed),
// so on its destruction each such Unit flips back onto the caster's side, frosted: a frost burst and a ring
// on the zone it lands in. Nothing moves. R202: only zones are anchors.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardRecipe, type Entrance } from "../recipe.ts";

/** Every particle count the griddle and the flip-back throw, at intensity "normal" (rule 9). */
const TUNING = {
  rime: { count: 14, power: 0.7 },
  spatula: { count: 12, power: 1 },
  flip: { count: 14, power: 0.8 },
} as const;

/** The griddle's frost spreads early; each flipped-back Unit lands at this fraction, staggered. */
const FROST_AT = 0.2;
const FLIP_AT = 0.2;
const FLIP_STEP = 0.1;
/** The lanes the gallery's flipped-back Units land in. */
const FIXTURE_LANES = [2, 4] as const;
const LOOK = paletteLook("Spirit") ?? FLAVOR_LOOKS.Spirit;

export const griddleFreeze: Entrance = ({ D, tgt, intensity: i }) => {
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  return [
    { ...tunedBurst(i, "frost", tid(tgt, FOOT), "area", frac(FROST_AT, D), TUNING.rime), colors: LOOK.colors },
    ringCue(D, "frost", tid(tgt, FOOT), frac(FROST_AT, D)),
    { ...tunedBurst(i, "shard", at, "point", slam, TUNING.spatula), colors: LOOK.colors },
    ringCue(D, "frost", at, slam),
    ...shakeCues(i, ENTRANCE_TRAUMA.griddleFreeze, slam),
  ];
};

/** Its Death: every Unit it destroyed flips back, in order, frosted. */
export const spatulaFlip: CardRecipe = (event, p, play) => {
  if (event.type !== "summoned" || play.hook !== "death") return null;
  const zone = tid(testid.zone(sideOf(p.view, event.player), event.row, event.lane));
  const delay = Math.min(p.D, frac(FLIP_AT + (play.seen.summoned ?? 0) * FLIP_STEP, p.D));
  const cues: FxCue[] = [ringCue(p.D, "frost", zone, delay), { ...tunedBurst(p.env.intensity, "frost", zone, "area", delay, TUNING.flip), colors: LOOK.colors }];
  return { cues, row: "keep" };
};

/** Its Death flips back two of the Units it destroyed onto its caster's side. */
export function spatulaFlipFixture(_view: PlayerView, instanceId: string, player: PlayerId): GameEvent[] {
  return [
    { type: "hookResolving", sourceId: instanceId, defId: "classicplus-012-8", controller: player, hook: "death" },
    ...FIXTURE_LANES.map((lane, k): GameEvent => ({ type: "summoned", player, instanceId: `fx-flip-${String(k)}`, defId: "core-005", row: "units", lane })),
  ];
}
