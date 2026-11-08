// #2793: what Core's bespoke flourishes (transmogulate.ts, zephrys.ts, craftACard.ts) share, and
// Stinkyscape's (#2798, ../stinkyscape/) and Training Grounds' (#2799, ../traininggrounds/): the caster's
// look and scale, as a motif reads them (motifs/build.ts), so each is built from the motif primitives at
// the scale its cost and rarity give it. R3402001: and what Core's signatures share: whether an event is
// their play's own first beat, and the cap on the follow-up beats one entry draws.

import type { GameEvent } from "@jackioh/shared";

import { FLAVOR_FALLBACK } from "../../../cards/flavor.ts";
import { FX_SIGNATURE_BURSTS_MAX } from "../../constants.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import { flavorLookOf } from "../../flavorSource.ts";
import { motifScale, type CueSpec } from "../../motifs/build.ts";
import type { FxPlay } from "../../types.ts";
import type { CardFxPlan } from "../recipe.ts";

/** The resolving play's own look (its flavor's, a Chaos card's pick) and its motif scale, the intensity in it. */
export function flourishSpec(p: CardFxPlan, play: FxPlay): CueSpec {
  return {
    look: flavorLookOf(play.defId, play.instanceId, p.view) ?? FLAVOR_LOOKS[FLAVOR_FALLBACK],
    scale: motifScale(play.defId, p.view, p.env.intensity),
    row: "keep",
  };
}

/**
 * R3402001: whether `event` is the first beat of `play`: its own `cardPlayed` (a Spell's cast, a Field
 * Spell's play), or a Trap's own first `trapFired` (its firing, never its face-down set). R2722002: a
 * Cloaked card's face-down set (its fresh id's `formerId`, R227) is no first beat, and its reveal, a
 * `trapFired` like a Trap's, is.
 */
export function isFirstBeat(event: GameEvent, play: FxPlay): boolean {
  if (event.type === "trapFired") return play.hook === "cast" && event.again !== true && event.instanceId === play.instanceId;
  return event.type === "cardPlayed" && event.formerId === undefined && play.hook === undefined && event.instanceId === play.instanceId;
}

/** R3402001: whether `event` is among the first FX_SIGNATURE_BURSTS_MAX of its type in `p`'s entry, so a run of them draws a capped few. */
export function withinBursts(event: GameEvent, p: CardFxPlan): boolean {
  const same = p.entry.events.filter((candidate) => candidate.type === event.type);
  return same.indexOf(event) < FX_SIGNATURE_BURSTS_MAX;
}
