// R3399000: Classic+ Support Loser (`classicplus-019-4`, entrance "staffShuffle", bespoke "sighMend"), his
// signature (#3399): Entrance: he shuffles into the zone staff first (two small puffs at his foot), sighs
// (a soft smoke at mid-entry) and a shield bubble blooms around him at the slam (the Divine Shield callout,
// holy light, a ring, the one shake). Text: at the end of turn a ring of soft white light spreads from
// his staff (his zone) and a streak of it runs to the caster's hero and to every ally, each glowing as it
// heals (`healed`, a holy burst and a ring). The heal's own number still plays. R202: zones, heroes and
// public board targets anchor; his own zone is read off the board, else the caster's hero stands for it.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { targetFor } from "../../../game/animations.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { hitAnchor, motifRing, timesOf, type CueSpec } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, heroOf, projectile, type CardRecipe, type Entrance } from "../recipe.ts";

/** Every particle count the shuffle and the mend throw, at intensity "normal" (rule 9). */
const TUNING = {
  shuffle: { count: 6, power: 0.4 },
  sigh: { count: 8, power: 0.3 },
  bubble: { count: 14, power: 0.9 },
  staff: { count: 8, power: 0.6 },
  mend: { count: 10, power: 0.6 },
} as const;

const LOOK = paletteLook("Holy") ?? FLAVOR_LOOKS.Holy;

const SHUFFLE_AT = [0.15, 0.35] as const;
const SIGH_AT = 0.45;
const CALLOUT_MS = 600;
const SHIELD_MS = 500;

export const staffShuffle: Entrance = ({ D, tgt, intensity: i }) => {
  const zone = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  const steps = SHUFFLE_AT.map((at): FxCue => tunedBurst(i, "dust", tid(tgt, FOOT), "point", frac(at, D), TUNING.shuffle));
  return [
    ...steps,
    tunedBurst(i, "smoke", zone, "area", frac(SIGH_AT, D), TUNING.sigh),
    { ...tunedBurst(i, "holy", zone, "ring", slam, TUNING.bubble), colors: LOOK.colors },
    ringCue(D, "holy", zone, slam),
    { kind: "callout", keyword: "Divine Shield", at: zone, delayMs: slam, durationMs: CALLOUT_MS },
    { kind: "shield", size: "small", at: zone, delayMs: slam, durationMs: SHIELD_MS },
    ...shakeCues(i, ENTRANCE_TRAUMA.staffShuffle, slam),
  ];
};

/** The end-of-turn mend: his staff's ring once, then a streak of light to each one it heals, glowing as it lands. */
export const sighMend: CardRecipe = (event, p, play) => {
  if (event.type !== "healed" || play.hook !== "trigger") return null;
  const tgt = targetFor(event, p.view);
  if (tgt === null) return null;
  const t = timesOf(p.D);
  const i = p.env.intensity;
  const spec: CueSpec = { look: LOOK, scale: i, row: "keep" };
  const to = tid(tgt);
  const staff = hitAnchor(p.view, play.instanceId) ?? heroOf(p.view, play);
  const cues: FxCue[] = [];
  if (play.seen.healed === 1) cues.push(motifRing(spec, p.D, staff, t.G), { ...tunedBurst(i, "holy", staff, "ring", t.G, TUNING.staff), colors: LOOK.colors });
  cues.push(projectile(i, "holy", staff, to, t.T, t.L - t.T), { ...tunedBurst(i, "sparkle", to, "area", t.L, TUNING.mend), colors: LOOK.colors }, ringCue(p.D, "holy", to, t.L));
  return { cues, row: "keep" };
};

/** The gallery's end of turn: the caster's hero and its first Unit mend. */
const FIXTURE_HEAL = 2;

export function sighMendFixture(view: PlayerView, instanceId: string, player: PlayerId): GameEvent[] {
  const own = player === view.viewer ? view.you : view.opponent;
  const ally = own.units.find((candidate) => candidate !== null);
  const heal = (targetId: string): GameEvent => ({ type: "healed", targetId, amount: FIXTURE_HEAL });
  return [
    { type: "hookResolving", sourceId: instanceId, defId: "classicplus-019-4", controller: player, hook: "endOfTurn" },
    heal(`hero-${player}`),
    ...(ally === undefined || ally === null ? [] : [heal(ally.instanceId)]),
  ];
}
