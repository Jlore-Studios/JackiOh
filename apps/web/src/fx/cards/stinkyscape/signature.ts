// R3407000: what Stinkyscape part two's signatures share (#3407, the Legendaries and Mythics of
// `stinkyscape-091` on, the three MAGI tokens and the Narcors among them): the cards, the spec a beat is
// built with, the piles a hand or a deck is reached through, a card back flying between two anchors and a
// ballot dropping. No cue names a card: a hand or a deck is only ever its pile, a card moving is only ever
// its back, and an id the view cannot find draws nothing (R202).

import type { PlayerId, PlayerView } from "@jackioh/shared";

import { FLAVOR_FALLBACK } from "../../../cards/flavor.ts";
import { animTestid } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, tid } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook, type FxPalette } from "../../flavors.ts";
import { motifBurst, motifRing, motifScale, type CueSpec } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxGhostCue, FxPlay } from "../../types.ts";
import type { CardFxPlan, EntrancePlan } from "../recipe.ts";

/** R3407000: every card of part two's signature pass, in catalog order (the MAGI tokens first, `stinkyscape-1.json5`'s). */
export const STINKYSCAPE_TWO_SIGNATURES = [
  "stinkyscape-018-1",
  "stinkyscape-018-2",
  "stinkyscape-018-3",
  "stinkyscape-109",
  "stinkyscape-110",
  "stinkyscape-111",
  "stinkyscape-112",
  "stinkyscape-112-1",
  "stinkyscape-115",
  "stinkyscape-116",
  "stinkyscape-147",
  "stinkyscape-147-1",
  "stinkyscape-147-2",
  "stinkyscape-148",
  "stinkyscape-148-1",
  "stinkyscape-149",
  "stinkyscape-150",
  "stinkyscape-150-1",
  "stinkyscape-151",
  "stinkyscape-152",
  "stinkyscape-153",
  "stinkyscape-t-narcor",
  "stinkyscape-t-red-narcor",
] as const;

export type StinkyscapeTwoSignature = (typeof STINKYSCAPE_TWO_SIGNATURES)[number];

/** Every particle count the shared beats throw at scale 1 (rule 9). */
const TUNING = {
  ballot: { count: 12, power: 0.8 },
} as const;

/** The look `palette` names, else the fallback flavor's. */
export function lookOf(palette: FxPalette): CueSpec["look"] {
  return paletteLook(palette) ?? FLAVOR_LOOKS[FLAVOR_FALLBACK];
}

/** R3407000: a card recipe's beat in `palette`, at its card's motif scale, over the event rows (`keep`). */
export function sigSpec(p: CardFxPlan, play: FxPlay, palette: FxPalette): CueSpec {
  return { look: lookOf(palette), scale: motifScale(play.defId, p.view, p.env.intensity), row: "keep" };
}

/** R3407000: an entrance's beat in `palette` at the plan's intensity. */
export function entranceSpec(p: EntrancePlan, palette: FxPalette): CueSpec {
  return { look: lookOf(palette), scale: p.intensity, row: "keep" };
}

/**
 * R3407000: an entrance's moments: `wait`, its entry's hold before the motion (a Mythic's takeover), and
 * `at(x)`, x of the motion after it; `slam` is when its card lands, FX_SLAM_AT of the motion, with the
 * takeover's landing (takeover.ts `entranceLandMs`).
 */
export function entranceTimes(p: EntrancePlan): { wait: number; at: (x: number) => number; slam: number } {
  const wait = Math.min(Math.max(0, p.wait ?? 0), p.D);
  const at = (x: number): number => wait + frac(x, p.D - wait);
  return { wait, at, slam: at(FX_SLAM_AT) };
}

/** The piles a player's cards are reached through: never a card in them (R202). */
export type Pile = "hand" | "library" | "graveyard" | "mana";

/** R3407000: `player`'s `pile` as the board draws it, by side. */
export function pileOf(view: PlayerView, player: PlayerId, pile: Pile): FxAnchor {
  return tid(animTestid[pile](sideOf(view, player)));
}

/** R3407000: a card BACK flying from `from` to `to`; it carries no card (R202). */
export function backTo(from: FxAnchor, to: FxAnchor, delayMs: number, durationMs: number): FxGhostCue {
  return { kind: "ghost", from, to, delayMs, durationMs: Math.max(0, durationMs) };
}

/** R3407000: a ballot dropping onto `at`: a ring and a small burst, in the spec's look. */
export function ballot(spec: CueSpec, D: number, at: FxAnchor, delayMs: number): FxCue[] {
  return [motifRing(spec, D, at, delayMs), motifBurst(spec, at, "point", delayMs, TUNING.ballot)];
}

/** The other seat. */
export function otherPlayer(player: PlayerId): PlayerId {
  return player === "p1" ? "p2" : "p1";
}
