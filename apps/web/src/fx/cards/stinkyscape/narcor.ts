// R3407000: the two Narcors The Narcor Project mines (#39), each a token Spell's signature:
//
// - Narcor (`stinkyscape-t-narcor`, printed Legendary, recipe "narcorDust"): a pinch of silver-blue narcor
//   dust blows from its caster's hero across the board (a bolt to the hand pile, dust at the hero) and
//   shimmers into a slow spiral over the hand pile (a ring and a ring of sparkles); the Legendary card it
//   adds condenses into a card back that settles into the hand pile (`addedToHand`).
// - Red Narcor (`stinkyscape-t-red-narcor`, printed Mythic, recipe "redNarcorHum", takeover "redNarcor":
//   takeover.ts draws its own picture, the humming crimson crystal in a prospector's tent): as the hold ends
//   the crystal streaks into its caster's hero (a fire projectile from the board's centre, a ring and a burst
//   of sparks there);
//   its mana fills with crimson sparks (`manaChanged`, a ring on the mana pile), then the card it draws slides
//   into the hand as a back (`drawn`, from the deck pile).
//
// Moved from the issue's concept: the cloaked palm is the hero (no figure draws a hand). The beats after the
// cast are rings and backs alone, in place of the mana's and the hand's rows, no bursts: a burst lasts
// FX_MAX_PARTICLE_LIFE_MS past its start, past a Spell's ceiling from any entry after its cast; and Red
// Narcor's strike is a ring, not rays, which outlast its held entry by FX_RAYS_TAIL_MS (R3332000).
//
// R202: no cue names a card. A hand, a deck and the mana are only ever their piles; `addedToHand` and
// `drawn` are read for their `player` alone (their ids are "hidden" on the other seat), so the beat draws the
// same on both seats. Each beat starts inside its entry and ends within FX_MAX_TAIL_MS of it (R200).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid } from "../../build.ts";
import { FX_CENTER } from "../../constants.ts";
import { motifBurst, motifRing, motifShot } from "../../motifs/build.ts";
import type { FxCue, FxPlay } from "../../types.ts";
import { heroOf, projectile, type CardRecipe } from "../recipe.ts";
import { backTo, pileOf, sigSpec } from "./signature.ts";

/** The palettes: the narcor's silver-blue ore, the red crystal's fire. */
const ORE = "Hustle/ore";
const CRIMSON = "Fire";

/** Every particle count the two throw at scale 1 (rule 9). */
const TUNING = {
  pinch: { count: 10, power: 0.5 },
  spiral: { count: 14, power: 0.6 },
  streak: { count: 18, power: 1.1 },
  /** Red Narcor's crystal striking its hero, within SIGNATURE_SPELL_MAX_TRAUMA. */
  crystalTrauma: 0.45,
} as const;

/** Each beat's moments as fractions of its entry's D. */
const TIMING = {
  narcorDust: { blowAt: 0, blowFor: 0.45, spiralAt: 0.45, sendAt: 0, sendFor: 0.8 },
  redNarcorHum: { streakAt: 0, streakFor: 0.35, fillAt: 0.1, sendAt: 0, sendFor: 0.8 },
} as const;

/** Whether `event` is the play's own cast. */
function ownCast(event: GameEvent, play: FxPlay): boolean {
  return event.type === "cardPlayed" && event.instanceId === play.instanceId;
}

/* Narcor */

export const narcorDust: CardRecipe = (event, p, play) => {
  const T = TIMING.narcorDust;
  if (ownCast(event, play)) {
    const spec = sigSpec(p, play, ORE);
    const hero = heroOf(p.view, play);
    const hand = pileOf(p.view, play.player, "hand");
    const blowAt = frac(T.blowAt, p.D);
    const spiralAt = frac(T.spiralAt, p.D);
    const cues: FxCue[] = [
      motifBurst(spec, hero, "point", blowAt, TUNING.pinch, "dust"),
      motifShot(spec, "bolt", hero, hand, blowAt, spiralAt - blowAt),
      motifRing(spec, p.D, hand, spiralAt),
      motifBurst(spec, hand, "ring", spiralAt, TUNING.spiral, "sparkle"),
    ];
    return { cues, row: "replace" };
  }
  if (event.type === "addedToHand") {
    // R202: the seat it goes to is public; the card is not read.
    const hand = pileOf(p.view, event.player, "hand");
    const delayMs = frac(T.sendAt, p.D);
    return { cues: [backTo(tid(testid.board, FX_CENTER), hand, delayMs, frac(T.sendFor, p.D)), ringCue(p.D, "sparkle", hand, delayMs)], row: "replace" };
  }
  return null;
};

/** R3407000: Narcor's play as its caster's seat streams it: the Legendary card it adds. */
export function narcorDustFixture(_view: PlayerView, _instanceId: string, player: PlayerId, _defId?: string): GameEvent[] {
  return [{ type: "addedToHand", player, instanceId: "fx-narcor-legend", defId: "core-097" }];
}

/* Red Narcor */

export const redNarcorHum: CardRecipe = (event, p, play) => {
  const T = TIMING.redNarcorHum;
  if (ownCast(event, play)) {
    const spec = sigSpec(p, play, CRIMSON);
    const hero = heroOf(p.view, play);
    // R3400000: the crystal strikes as its takeover's hold ends (its streak flying in under the hold's end),
    // so its bursts end within the Mythic Spell's ceiling; with no hold, after its streak.
    const hold = Math.min(p.D, p.entry.setPiece?.holdMs ?? 0);
    const flight = frac(T.streakFor, p.D);
    const strikeAt = hold > 0 ? hold : Math.min(p.D, frac(T.streakAt, p.D) + flight);
    const streakAt = Math.max(0, strikeAt - flight);
    return {
      cues: [
        projectile(spec.scale, "fire", tid(testid.board, FX_CENTER), hero, streakAt, strikeAt - streakAt),
        ringCue(p.D, "fire", hero, strikeAt),
        motifBurst(spec, hero, "area", strikeAt, TUNING.streak),
        ...shakeCues(p.env.intensity, TUNING.crystalTrauma, strikeAt),
      ],
      row: "replace",
    };
  }
  if (event.type === "manaChanged" && event.player === play.player) {
    return { cues: [ringCue(p.D, "fire", pileOf(p.view, event.player, "mana"), frac(T.fillAt, p.D))], row: "replace" };
  }
  if (event.type === "drawn") {
    // R202: the seat that draws is public; the card is not read.
    const delayMs = frac(T.sendAt, p.D);
    return { cues: [backTo(pileOf(p.view, event.player, "library"), pileOf(p.view, event.player, "hand"), delayMs, frac(T.sendFor, p.D))], row: "replace" };
  }
  return null;
};

/** R3407000: Red Narcor's play as the gallery plays it: its mana; its draw is proven by its test from its cast. */
export function redNarcorHumFixture(_view: PlayerView, _instanceId: string, player: PlayerId, _defId?: string): GameEvent[] {
  return [{ type: "manaChanged", player, current: 5, max: 3 }];
}
