// #2793: #99 Craft a Card (`core-099`, bespoke "fusionForge"), bespoke because its Discovered Units fuse
// off the board and no motif reads `fused`: at the fuse two halves fly in from the board's edges and
// forge in a nova at its centre, and the crafted card flies on into the caster's hand pile. The engine
// adds the card to the hand before it reports the fuse, so the forge carries the card's flight. It
// anchors on the board and the hand pile alone, the same on both seats (R202); the rows still play.

import { animTestid } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { shakeCues, tid } from "../../build.ts";
import { FX_CENTER, FX_FORGE_REACH, FX_FORGE_TRAUMA } from "../../constants.ts";
import { MOTIF_TUNING, motifBurst, motifRing, motifShot, timesOf } from "../../motifs/build.ts";
import type { FxAnchor } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";
import { flourishSpec } from "./flourish.ts";

export const fusionForge: CardRecipe = (event, p, play) => {
  if (event.type !== "fused") return null;
  const spec = flourishSpec(p, play);
  const t = timesOf(p.D);
  const centre = tid(testid.board, FX_CENTER);
  const edge = (x: number): FxAnchor => tid(testid.board, { x, y: FX_CENTER.y });
  const left = edge(FX_FORGE_REACH);
  const right = edge(1 - FX_FORGE_REACH);
  const hand = tid(animTestid.hand(sideOf(p.view, play.player)));
  return {
    cues: [
      motifBurst(spec, left, "point", t.G, MOTIF_TUNING.gather),
      motifBurst(spec, right, "point", t.G, MOTIF_TUNING.gather),
      motifShot(spec, "bolt", left, centre, t.T, t.L - t.T),
      motifShot(spec, "bolt", right, centre, t.T, t.L - t.T),
      motifBurst(spec, centre, "area", t.L, MOTIF_TUNING.nova),
      motifRing(spec, p.D, centre, t.L),
      ...shakeCues(spec.scale, FX_FORGE_TRAUMA, t.L),
      { kind: "ghost", from: centre, to: hand, delayMs: t.Li, durationMs: t.D - t.Li },
      motifBurst(spec, hand, "point", t.D, MOTIF_TUNING.small, "sparkle"),
    ],
    row: "keep",
  };
};
