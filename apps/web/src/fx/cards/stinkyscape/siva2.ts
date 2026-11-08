// R3395000: Stinkyscape #88 Siva 2 (`stinkyscape-088`, entrance "venomStep", bespoke "venomHand", toxic
// motes curling on her tile), a Legendary Unit's signature. Siva 2 steps out of the dark flicking a venom
// vial over her zone, and a drip of green venom runs to each friendly Human the side shows face up (her
// Aura's Poisonous edging their blades), landing as she does in a toxic ring. Her Radiant Cry fills the hand
// with Speciesists: each flies from her hero to the hand pile as a card back, a step after the last. It keys
// on her play and the board's public tags, never on a hidden card or an added card's definition (R202).
// Moved: the first Siva's red carpet cutting to black is her step out of the dark; each Human summoned
// later keys nothing of hers.

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { HIDDEN_ID } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { CATALOG } from "../../../wire/cards.ts";
import { ringCue, tid, tunedBurst } from "../../build.ts";
import type { FxCue, FxShotCue } from "../../types.ts";
import { FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { cardBack, entranceFigure, handOf, heroAt, leadOf, lookOf, replace, slamAt, slamShake } from "./signatureKit.ts";

/** Her venom: the curse's green. */
const VENOM = "Dark/curse";

/** The tag her Aura reads (§8). */
const HUMAN_TAG = "Human";

/** At most this many drips run, one to each Human. */
const DRIPS_MAX = 5;

/** Every particle count she throws, at intensity "normal" (rule 9). */
const TUNING = {
  venom: { count: 22, power: 1 },
} as const;

/** As shares of the entry: the first drip leaves at `dripAt`, each next `dripStep` later, all landing at the slam; the hand's backs fan `fanStep` apart, at most `fanMax` in. */
const TIMING = { dripAt: 0.1, dripStep: 0.06, fanStep: 0.15, fanMax: 0.5 } as const;

type Summoned = Extract<GameEvent, { type: "summoned" }>;

/** R202: the unit zones, in lane order, of every Human on her side the view shows face up, her own left out. */
export function humanZones(view: PlayerView, landing: Summoned): string[] {
  const side = sideOf(view, landing.player);
  return view[side].units.flatMap((unit, k) => {
    if (unit === null || unit.defId === HIDDEN_ID || unit.instanceId === landing.instanceId) return [];
    const tags = unit.tags ?? (CATALOG[unit.defId] ?? view.defs?.[unit.defId])?.tags ?? [];
    return tags.includes(HUMAN_TAG) ? [testid.zone(side, "units", k + 1)] : [];
  });
}

export const venomStep: Entrance = (plan) => {
  const look = lookOf(VENOM);
  const at = tid(plan.tgt);
  const slam = slamAt(plan);
  const humans = plan.view === undefined || plan.landing === undefined ? [] : humanZones(plan.view, plan.landing).slice(0, DRIPS_MAX);
  const drips = humans.map((zone, n): FxShotCue => {
    const delayMs = Math.min(slam, leadOf(plan) + Math.round((TIMING.dripAt + n * TIMING.dripStep) * (plan.D - leadOf(plan))));
    return { kind: "shot", shot: { motion: "bolt", trail: "poison" }, colors: look.colors, preset: "poison", from: at, to: tid(zone), delayMs, flightMs: slam - delayMs, density: plan.intensity };
  });
  const cues: FxCue[] = [
    entranceFigure("vial", VENOM, plan),
    ...drips,
    { ...tunedBurst(plan.intensity, "poison", tid(plan.tgt, FOOT), "area", slam, TUNING.venom), colors: look.colors },
    ringCue(plan.D, "poison", at, slam),
    ...slamShake(plan, "venomStep"),
  ];
  return cues;
};

export const venomHand: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined || event.type !== "addedToHand" || event.player !== play.player) return null;
  const n = play.seen.addedToHand ?? 1;
  const delayMs = Math.min(Math.round((n - 1) * TIMING.fanStep * p.D), Math.round(TIMING.fanMax * p.D));
  return replace([cardBack(heroAt(p.view, play.player), handOf(p.view, event.player), delayMs, p.D)]);
};
