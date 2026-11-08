// #2793: #99 Craft a Card (`core-099`, bespoke "fusionForge"), bespoke because its Discovered Units fuse
// off the board and no motif reads `fused`. R3402001: its signature (R3332000), a Mythic Spell's: its
// takeover is a floating crystal forge, two glowing rails converging on an anvil under a giant hammer,
// gears turning overhead (takeover look "forge", takeoverArt.ts), the hammer falling as the card lands.
// Through its hold the two Discovered Units ride the rails in from the board's edges as card backs and
// meet in frost sparks at its centre (the caster picks them in a prompt only they see). The engine adds
// the crafted card to the hand before it reports the fuse: the hand pile rings as it does, and on the
// `fused` the crafted card flies on from the forge into the hand pile as a card back, with the strike's
// shake, both in place of their rows, inside a Spell's budget. It anchors on the board and the hand pile
// alone, the same on both seats (R202).

import { animTestid, leadMs } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { ringCue, shakeCues, tid } from "../../build.ts";
import { FX_CENTER, FX_FORGE_REACH, FX_FORGE_TRAUMA } from "../../constants.ts";
import { MOTIF_TUNING, motifBurst, motifRing, timesOf } from "../../motifs/build.ts";
import type { FxAnchor } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";
import { flourishSpec, isFirstBeat } from "./flourish.ts";

export const fusionForge: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  const spec = flourishSpec(p, play);
  const t = timesOf(p.D);
  const centre = tid(testid.board, FX_CENTER);
  const hand = tid(animTestid.hand(sideOf(p.view, play.player)));
  if (isFirstBeat(event, play)) {
    const edge = (x: number): FxAnchor => tid(testid.board, { x, y: FX_CENTER.y });
    // The halves ride in through the takeover's hold and meet before its card lands; with Big moments
    // off there is no hold, and they meet as the entry lands.
    const hold = Math.max(leadMs(p.entry), t.L);
    const ride = Math.round(hold / 2);
    return {
      cues: [
        motifBurst(spec, edge(FX_FORGE_REACH), "point", t.G, MOTIF_TUNING.gather),
        motifBurst(spec, edge(1 - FX_FORGE_REACH), "point", t.G, MOTIF_TUNING.gather),
        { kind: "ghost", from: edge(FX_FORGE_REACH), to: centre, delayMs: ride, durationMs: hold - ride },
        { kind: "ghost", from: edge(1 - FX_FORGE_REACH), to: centre, delayMs: ride, durationMs: hold - ride },
        motifBurst(spec, centre, "area", hold, MOTIF_TUNING.land, "frost"),
        motifRing(spec, p.D, centre, hold),
      ],
      row: "keep",
    };
  }
  if (event.type === "addedToHand" && event.player === play.player) {
    return { cues: [ringCue(p.D, "frost", hand, t.G)], row: "replace" };
  }
  if (event.type !== "fused") return null;
  return {
    cues: [{ kind: "ghost", from: centre, to: hand, delayMs: t.G, durationMs: Math.min(p.D, t.L) }, ...shakeCues(p.env.intensity, FX_FORGE_TRAUMA, t.G)],
    row: "replace",
  };
};
