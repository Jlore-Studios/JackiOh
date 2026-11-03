// R2794000: what Classic's bespoke Flourishes share (Book of Heal's pageMend, Recycle's recycleLoop and
// King Wagtoggle's crownSwap): a motif's look and scale for the card's play, so a Flourish draws with the
// motif primitives (fx/motifs/build.ts) at the card's own cost and rarity, and the fixture the /dev/fx
// gallery plays it on (fx/motifs/gallery.ts). Presentation only (CLAUDE.md rule 7).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { FLAVOR_FALLBACK } from "../../../cards/flavor.ts";
import { FLAVOR_LOOKS, paletteLook, type FxPalette } from "../../flavors.ts";
import { motifScale, type CueSpec } from "../../motifs/build.ts";
import type { FxPlanEnv, FxPlay } from "../../types.ts";

/** A Flourish's fixture: the events its play resolves with in the gallery, as `player` would play it on `view`. */
export type FlourishFixture = (view: PlayerView, instanceId: string, player: PlayerId) => GameEvent[];

/** A Flourish's look (its palette's, else the fallback flavor's) and scale (the card's motif scale), over the event's own row. */
export function flourishSpec(palette: FxPalette, play: FxPlay, view: PlayerView, env: FxPlanEnv): CueSpec {
  return { look: paletteLook(palette) ?? FLAVOR_LOOKS[FLAVOR_FALLBACK], scale: motifScale(play.defId, view, env.intensity), row: "keep" };
}
