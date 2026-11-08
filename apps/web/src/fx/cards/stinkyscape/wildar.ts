// R3407000: Stinkyscape #149 Wildar (`stinkyscape-149`, bespoke "chaosOrb"), a Legendary Spell's signature.
//
// Its cast: a swirl of nebula smoke opens over the board's centre and a churning chaos orb flares in it,
// then cracks in two (a ring, a fracture, a light shake): a dull, sputtering half in grey smoke and a
// dazzling half in prismatic sparks. Its two `addedToHand` then send the halves on as card backs from the
// centre: the first to the hand pile of whoever gets it (the opponent's Call to Chaos, by the card's order),
// the caster's with a sheen on his pile as the dazzling half drops in.
//
// What moved from the issue's concept: Wildar's patchwork-sleeved hand is not drawn (no figure draws a
// hand, and a new one needs fx.css); the smoke and the orb stand for it. Which half is which is told by
// the seat a card goes to (`event.player`, public), never by the card.
//
// R202: the editions added, their Radiance and the (0) cost are their owners' alone, so nothing here reads
// an `addedToHand`'s `defId` or `instanceId` (both "hidden" on the other seat): a hand is only ever its
// pile and a card only ever its back, and the beat draws the same on both seats. R200: every cue starts
// inside its entry and ends within FX_MAX_TAIL_MS of it; the halves' ghosts replace the hand's glint,
// whose burst would outlast the Spell's ceiling (R3332000).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { HIDDEN_ID } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { frac, shakeCues, tid } from "../../build.ts";
import { FX_CENTER } from "../../constants.ts";
import { motifBurst, motifRing, type CueSpec } from "../../motifs/build.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";
import { backTo, otherPlayer, pileOf, sigSpec } from "./signature.ts";

/** Every particle count the cast throws at scale 1 (rule 9). */
const TUNING = {
  nebula: { count: 26, power: 0.7 },
  orb: { count: 18, power: 0.6 },
  dull: { count: 10, power: 0.5 },
  dazzle: { count: 22, power: 1.1 },
  /** The crack's shake, inside SIGNATURE_SPELL_MAX_TRAUMA. */
  crackTrauma: 0.35,
} as const;

/** The cast's moments as fractions of its entry's D, the crack's tail past it, and the halves' flight. */
const TIMING = {
  orbAt: 0.15,
  crackAt: 0.5,
  /** The crack lingers this long past the cast's entry, inside the Spell's ceiling. */
  fractureTailMs: 600,
  sendAt: 0,
  sheenAt: 0.5,
} as const;

/** The palettes: the nebula's dark smoke, the orb's chaos, the dull half's grey. */
const NEBULA = "Dark";
const CHAOS = "Chaos";
const DULL = "Dark/shadow";

/** The board's centre, where the orb cracks and both halves leave from. */
const CENTRE: FxAnchor = tid(testid.board, FX_CENTER);

export const chaosOrb: CardRecipe = (event, p, play) => {
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const nebula: CueSpec = sigSpec(p, play, NEBULA);
    const chaos: CueSpec = sigSpec(p, play, CHAOS);
    const crack = frac(TIMING.crackAt, p.D);
    const cues: FxCue[] = [
      motifBurst(nebula, CENTRE, "area", 0, TUNING.nebula, "smoke"),
      motifBurst(chaos, CENTRE, "point", frac(TIMING.orbAt, p.D), TUNING.orb),
      motifRing(chaos, p.D, CENTRE, crack),
      { kind: "fracture", at: CENTRE, delayMs: crack, durationMs: p.D - crack + TIMING.fractureTailMs },
      motifBurst(sigSpec(p, play, DULL), CENTRE, "point", crack, TUNING.dull),
      motifBurst(chaos, CENTRE, "area", crack, TUNING.dazzle),
      ...shakeCues(p.env.intensity, TUNING.crackTrauma, crack),
    ];
    return { cues, row: "replace" };
  }
  if (event.type === "addedToHand") {
    // R202: the seat it goes to is public; the card is not read.
    const hand = pileOf(p.view, event.player, "hand");
    const delayMs = frac(TIMING.sendAt, p.D);
    const cues: FxCue[] = [backTo(CENTRE, hand, delayMs, p.D - delayMs)];
    if (event.player === play.player) {
      const sheenAt = frac(TIMING.sheenAt, p.D);
      cues.push({ kind: "sheen", at: hand, delayMs: sheenAt, durationMs: p.D - sheenAt });
    }
    return { cues, row: "replace" };
  }
  return null;
};

/**
 * R3407000: Wildar's Cry as its caster's seat streams it: the opponent's Call to Chaos (its edition hidden
 * from the caster, R97: `HIDDEN_ID`), then the caster's own Radiant one.
 */
export function chaosOrbFixture(_view: PlayerView, _instanceId: string, player: PlayerId, _defId?: string): GameEvent[] {
  return [
    { type: "addedToHand", player: otherPlayer(player), instanceId: HIDDEN_ID, defId: HIDDEN_ID },
    { type: "addedToHand", player, instanceId: "fx-chaos-half", defId: "core-095" },
  ];
}
