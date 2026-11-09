// R3395000: Stinkyscape #39 The Narcor Project (`stinkyscape-039`, entrance "labVault", bespoke
// "narcorCases", a green lab haze in its zone), a Legendary Field Spell's signature. A sealed lab vault
// sinks into its zone with a hiss of green gas, its glass cases lighting one by one with crystals. Each End
// of turn a Narcor flies from a case to its owner's hand pile as a card back. Its Activate feeds one back:
// the discarded Narcor flies from the hand pile into the vault, and as its Brittle drops a crack splits
// wider across its zone; when Brittle destroys it the vault bursts open in green gas. It keys on its hooks
// and its own public id, never on a card's definition (R202). Moved: the rest-of-game gains its Death
// leaves come at later turns' ends with no hook of the vault's, and the haze and glowing cracks on its tile
// are slow green haze motes.

import { animTestid } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { ringCue, tid, tunedBurst } from "../../build.ts";
import type { FxCue } from "../../types.ts";
import { FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { cardAt, cardBack, entranceFigure, handOf, lookOf, replace, slamAt, slamShake } from "./signatureKit.ts";

/** Its gas and crystals: the curse's green and violet. */
const LAB = "Dark/curse";

/** The counter its Activate lowers. */
const BRITTLE = "brittle";

/** Every particle count the vault throws, at intensity "normal" (rule 9). */
const TUNING = {
  gas: { count: 24, power: 0.9 },
  burst: { count: 26, power: 1.2 },
} as const;

export const labVault: Entrance = (plan) => {
  const at = tid(plan.tgt);
  const slam = slamAt(plan);
  return [
    entranceFigure("labCases", LAB, plan),
    { ...tunedBurst(plan.intensity, "poison", tid(plan.tgt, FOOT), "area", slam, TUNING.gas), colors: lookOf(LAB).colors },
    ringCue(plan.D, "poison", at, slam),
    ...slamShake(plan, "labVault"),
  ];
};

export const narcorCases: CardRecipe = (event, p, play) => {
  const vault = cardAt(p.view, play);
  if (event.type === "addedToHand" && event.player === play.player) {
    // A Narcor from its case, as a card back.
    return replace([cardBack(vault, handOf(p.view, event.player), 0, p.D)]);
  }
  if (event.type === "discarded" && event.owner === play.player) {
    // Fed back: from the hand pile into the vault.
    return replace([cardBack(tid(animTestid.hand(sideOf(p.view, event.owner))), vault, 0, p.D)]);
  }
  if (event.type === "counterChanged" && event.instanceId === play.instanceId && event.counter === BRITTLE) {
    return replace([{ kind: "crack", at: vault, delayMs: 0, durationMs: p.D }]);
  }
  if (event.type === "destroyed" && event.instanceId === play.instanceId) {
    const cues: FxCue[] = [{ ...tunedBurst(p.env.intensity, "poison", vault, "area", 0, TUNING.burst), colors: lookOf(LAB).colors }, ringCue(p.D, "poison", vault, 0)];
    return replace(cues);
  }
  return null;
};
