// R3395000: Stinkyscape #6 Tylannt (`stinkyscape-006`, entrance "vaultStride", bespoke "vaultHeist",
// takeover "vault", steel motes on his tile), a Mythic Unit's signature. His takeover is a giant round
// vault door filling the dark, its tumblers clicking open (fx/takeoverArt.ts); then a small vault door
// swings open over his zone and he drops through it in a spray of steel and a ring of cash. His Cry
// completes his Quests' stages: at each `questCompleted` a combination dial turns and slams home over
// that Quest's zone, and the rewards follow, a card back to the hand for each card added and a ring where
// a Unit is summoned; the Radiant face's The Greatest Heist lands first with its own entrance. It keys on
// his play and the Quest's public id, never on a card's definition (R202). Moved: his duffel is the cash
// ring, and the steel dial on his tile is a slow drift of steel motes (an aura draws motes and an edge).

import { ringCue, tid, tunedBurst } from "../../build.ts";
import { figureCue, figureTint } from "../../figures.ts";
import type { FxCue } from "../../types.ts";
import { FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { cardBack, entranceFigure, handOf, heroAt, lookOf, placeOf, replace, replaceMoment, slamAt, slamShake, summonZone } from "./signatureKit.ts";

/** The steel his vault is made of, and the cash it holds. */
const STEEL = "Physical/steel";
const CASH = "Hustle/cash";

/** Every particle count his signature throws, at intensity "normal" (rule 9). */
const TUNING = {
  drop: { count: 24, power: 1.1 },
  cash: { count: 18, power: 1 },
  reward: { count: 14, power: 0.9 },
} as const;

export const vaultStride: Entrance = (plan) => {
  const at = tid(plan.tgt);
  const slam = slamAt(plan);
  const i = plan.intensity;
  return [
    entranceFigure("vaultDoor", STEEL, plan),
    { ...tunedBurst(i, "shard", tid(plan.tgt, FOOT), "area", slam, TUNING.drop), colors: lookOf(STEEL).colors },
    ringCue(plan.D, "spark", at, slam),
    { ...tunedBurst(i, "confetti", at, "ring", slam, TUNING.cash), colors: lookOf(CASH).colors },
    ...slamShake(plan, "vaultStride"),
  ];
};

export const vaultHeist: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  if (event.type === "questCompleted" && event.player === play.player) {
    // The dial slams home over the Quest it completed, the first with a burst of cash.
    const quest = placeOf(p.view, event.instanceId);
    const at = quest === null ? heroAt(p.view, play.player) : tid(quest);
    const cues: FxCue[] = [figureCue("tumbler", at, figureTint(lookOf(STEEL)), 0, p.D), ringCue(p.D, "spark", at, 0)];
    if ((play.seen.questCompleted ?? 0) <= 1) cues.push({ ...tunedBurst(p.env.intensity, "confetti", at, "area", 0, TUNING.reward), colors: lookOf(CASH).colors });
    // R3395001: in place of the shared quest moment too, whose banner alone overran his Mythic ceiling.
    return replaceMoment(cues);
  }
  if (event.type === "addedToHand") {
    // A reward's card, as a back from his hero: its face is its owner's alone.
    return replace([cardBack(heroAt(p.view, play.player), handOf(p.view, event.player), 0, p.D)]);
  }
  if (event.type === "summoned" && event.row === "units" && event.instanceId !== play.instanceId) {
    const at = tid(summonZone(p.view, event));
    return replace([ringCue(p.D, "gold", at, 0), { kind: "sheen", at, delayMs: 0, durationMs: p.D }]);
  }
  return null;
};
