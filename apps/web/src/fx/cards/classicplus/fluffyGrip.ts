// R3399000: Classic+ Fluffy Grip (`classicplus-012-3`, entrance none, bespoke "doughArm"), its signature (#3399):
// a Spell printed Legendary, so every burst starts by SPELL_BURST_LATEST_MS of its entry (1,200 ms ceiling).
// Cast (its `cardPlayed`): a long fluffy dough arm stretches out of the caster's hand pile and wobbles across
// the board, as a ghost back flying from the hand pile to the opponent's deck pile with flour puffs.
// Text, at its `stolen`: the arm dives into the opponent's deck pile, a puff of flour there, and yanks one
// card back, a ghost back reeling home from the deck pile to the caster's hand pile (backs and piles only,
// never a face, R202). The `addedToHand` and `costChanged` rows play as they are. Moved: the reel-home is
// drawn on `stolen`, the one beat the recipe claims, rather than on `addedToHand`.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { animTestid } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, tid } from "../../build.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import { motifBurst, motifScale, type CueSpec } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the arm throws at scale 1 (rule 9). */
const TUNING = {
  stretch: { count: 8, power: 0.6 },
  dive: { count: 16, power: 0.9 },
  reel: { count: 8, power: 0.6 },
} as const;

/** A Spell's bursts start no later than this into its entry, so they end inside its 1,200 ms ceiling (#3399). */
const SPELL_BURST_LATEST_MS = 300;
/** Where in its entry each beat starts, and how long the arm and the reel fly (of the entry). */
const AT = { stretch: 0, dive: 0.1, reel: 0.35, armFly: 0.7, reelFly: 0.6 } as const;
/** The card the gallery's Fluffy Grip steals: a core Unit, never named in a cue (R202). */
const FIXTURE_STOLEN = { id: "fx-stolen", defId: "core-019", cost: 0 } as const;

function at(fraction: number, D: number): number {
  return Math.min(frac(fraction, D), SPELL_BURST_LATEST_MS);
}

/** Fluffy Grip: the arm reaches over on its cast and reels the stolen card home at its `stolen`. */
export const doughArm: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined || !(p.D > 0)) return null;
  const spec: CueSpec = { look: FLAVOR_LOOKS.Snack, scale: motifScale(play.defId, p.view, p.env.intensity), row: "keep" };
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const home = tid(animTestid.hand(sideOf(p.view, play.player)));
    const deck = tid(animTestid.library(sideOf(p.view, play.player === "p1" ? "p2" : "p1")));
    const cues: FxCue[] = [
      motifBurst(spec, home, "area", at(AT.stretch, p.D), TUNING.stretch),
      { kind: "ghost", from: home, to: deck, delayMs: at(AT.stretch, p.D), durationMs: frac(AT.armFly, p.D) },
    ];
    return { cues, row: "keep" };
  }
  if (event.type === "stolen") {
    const from = tid(animTestid.library(sideOf(p.view, event.from)));
    const to = tid(animTestid.hand(sideOf(p.view, event.to)));
    const cues: FxCue[] = [
      motifBurst(spec, from, "area", at(AT.dive, p.D), TUNING.dive),
      { kind: "ghost", from, to, delayMs: at(AT.dive, p.D), durationMs: frac(AT.reelFly, p.D) },
      motifBurst(spec, to, "area", at(AT.reel, p.D), TUNING.reel),
    ];
    return { cues, row: "keep" };
  }
  return null;
};

/** The gallery's Fluffy Grip: one card from the other player's deck comes home to the player's hand, costing 0. */
export function doughArmFixture(_view: PlayerView, _instanceId: string, player: PlayerId): GameEvent[] {
  const other: PlayerId = player === "p1" ? "p2" : "p1";
  return [
    { type: "stolen", instanceId: FIXTURE_STOLEN.id, defId: FIXTURE_STOLEN.defId, from: other, to: player, zone: "library" },
    { type: "addedToHand", player, instanceId: FIXTURE_STOLEN.id, defId: FIXTURE_STOLEN.defId },
    { type: "costChanged", instanceId: FIXTURE_STOLEN.id, cost: FIXTURE_STOLEN.cost },
  ];
}
