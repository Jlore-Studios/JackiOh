// R3407000: Stinkyscape #153 Stimmycheck (`stinkyscape-153`, recipe "stimmyDrop"), a Legendary Spell's
// signature, in Hustle's cash mint and gold.
//
// Its cast: a delivery drone swoops in from the board's top-left edge over its caster's hero (a cash bolt
// with a trail of sparks) and drops a fat mint-green envelope there, which bursts in a shower of credit chips
// (gold and a ring) with a small shake. Its mana: the crystals fill (`manaChanged`, a gold ring on its
// caster's mana pile, in place of the mana row).
//
// Moved from the issue's concept: the rising-bar glow on the hand and deck piles is dropped, Inflation
// announcing nothing (R1848006: the raised costs are its owner's and no event carries them), and so is the
// fresh copy at the end of its turn: that add is a delayed effect, which opens no hook entry of the card's
// (hook_marks.rs), so the hand's own row plays it. The mana's beat is a ring with no burst: a burst lasts
// FX_MAX_PARTICLE_LIFE_MS past its start, past the Spell's ceiling from any entry after its cast (R3332000).
//
// R202: no cue names a card; the mana is only ever its pile. Each beat starts inside its entry and ends
// within FX_MAX_TAIL_MS of it (R200).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid } from "../../build.ts";
import { motifBurst, motifRing, motifShot } from "../../motifs/build.ts";
import type { FxPoint } from "../../types.ts";
import { heroOf, type CardRecipe } from "../recipe.ts";
import { pileOf, sigSpec } from "./signature.ts";

/** Its palette: Hustle's cash. */
const CASH = "Hustle/cash";

/** Every particle count its cast throws at scale 1 (rule 9). */
const TUNING = {
  chips: { count: 22, power: 1 },
  /** The envelope bursting, within SIGNATURE_SPELL_MAX_TRAUMA. */
  dropTrauma: 0.25,
} as const;

/** Its beats' moments as fractions of their entry's D. */
const TIMING = { swoopAt: 0, dropAt: 0.4, fillAt: 0.1 } as const;

/** Where the drone swoops in from: the board's top-left edge. */
const EDGE: FxPoint = { x: 0, y: 0 };

export const stimmyDrop: CardRecipe = (event, p, play) => {
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const spec = sigSpec(p, play, CASH);
    const hero = heroOf(p.view, play);
    const swoopAt = frac(TIMING.swoopAt, p.D);
    const dropAt = frac(TIMING.dropAt, p.D);
    return {
      cues: [
        motifShot(spec, "bolt", tid(testid.board, EDGE), hero, swoopAt, dropAt - swoopAt),
        motifRing(spec, p.D, hero, dropAt),
        motifBurst(spec, hero, "area", dropAt, TUNING.chips, "gold"),
        ...shakeCues(p.env.intensity, TUNING.dropTrauma, dropAt),
      ],
      row: "replace",
    };
  }
  if (event.type === "manaChanged" && event.player === play.player) {
    return { cues: [ringCue(p.D, "gold", pileOf(p.view, event.player, "mana"), frac(TIMING.fillAt, p.D))], row: "replace" };
  }
  return null;
};

/** R3407000: Stimmycheck's play as the gallery plays it: the mana it gains. */
export function stimmyDropFixture(_view: PlayerView, _instanceId: string, player: PlayerId, _defId?: string): GameEvent[] {
  return [{ type: "manaChanged", player, current: 4, max: 3 }];
}
