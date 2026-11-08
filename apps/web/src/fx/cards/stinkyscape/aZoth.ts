// R3395000: Stinkyscape #71 A'Zoth (`stinkyscape-071`, entrance "heraldHorn", bespoke "echoRings", a red
// storm on its tile), a Legendary Unit's signature. A horn-mawed herald of polished wreckage unfolds in its
// zone and lets out a crimson shockwave, one ring at its zone and one rippling across both sides from the
// board's middle. Its Cry calls the fallen back: each Cry Unit bursts from the caster's graveyard pile (a card
// back) onto a red echo ring as its `summoned` lands, and as each one's own Cry replays its ring pulses again
// over that Unit and over each target the replayed Cry hits, the Cry's own events drawing their rows
// beneath. It keys on its play and the Units' public ids, never on a card's definition (R202). Moved: the
// shockwave pulsing from its maw on its tile is a red storm of motes.

import { animTestid } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { ringCue, tid, tunedBurst } from "../../build.ts";
import { FX_CENTER } from "../../constants.ts";
import { FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { cardBack, entranceFigure, lookOf, placeOf, replace, slamAt, slamShake, summonZone } from "./signatureKit.ts";

/** Its crimson. */
const CRIMSON = "Dark/blood";

/** The hook a replayed Cry's marker names. */
const CRY = "cry";

/** Every particle count it throws, at intensity "normal" (rule 9). */
const TUNING = {
  blast: { count: 22, power: 1.1 },
} as const;

/** As a share of the entry, when a called Unit lands on its ring. */
const TIMING = { land: 0.6 } as const;

export const heraldHorn: Entrance = (plan) => {
  const at = tid(plan.tgt);
  const slam = slamAt(plan);
  return [
    entranceFigure("horn", CRIMSON, plan),
    { ...tunedBurst(plan.intensity, "ember", tid(plan.tgt, FOOT), "area", slam, TUNING.blast), colors: lookOf(CRIMSON).colors },
    ringCue(plan.D, "blood", at, slam),
    ringCue(plan.D, "blood", tid(testid.board, FX_CENTER), slam),
    ...slamShake(plan, "heraldHorn"),
  ];
};

export const echoRings: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  if (event.type === "summoned" && event.player === play.player && event.instanceId !== play.instanceId) {
    const zone = tid(summonZone(p.view, event));
    const land = Math.round(TIMING.land * p.D);
    return replace([cardBack(tid(animTestid.graveyard(sideOf(p.view, event.player))), zone, 0, land), ringCue(p.D, "blood", zone, land)]);
  }
  if (event.type === "hookResolving" && event.hook === CRY && event.sourceId !== play.instanceId) {
    // A called Unit's Cry replays: its ring pulses again over it; the Cry's own events draw their rows.
    const unit = placeOf(p.view, event.sourceId);
    return unit === null ? null : { cues: [ringCue(p.D, "blood", tid(unit), 0)], row: "keep" };
  }
  if (event.type === "damage" && (play.seen.hookResolving ?? 0) > 0) {
    // A replayed Cry's hit on its random target pulses the echo there, over the hit's own row (A'Zoth deals none).
    const target = placeOf(p.view, event.targetId);
    return target === null ? null : { cues: [ringCue(p.D, "blood", tid(target), 0)], row: "keep" };
  }
  return null;
};
