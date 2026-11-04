// #2798: Stinkyscape #110 iKwzki (`stinkyscape-110`, bespoke "duoBond"), bespoke because its Duo is a
// bond between two cards and no motif reads `marked`: a gather on iKwzki (its caster's hero when it is
// off the board), a beam to the permanent it Duos with, and there the Radiant rays it is promised, a
// ring and a burst, all in the Duo mark's own gold. It anchors on the shown ids and the heroes alone,
// the same on both seats (R202); the mark's own brand row still plays.

import { locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { DUO_MARK } from "../../../cards/marks.ts";
import { raysCue, shakeCues, tid } from "../../build.ts";
import { FX_DUO_BOND_TRAUMA } from "../../constants.ts";
import { MOTIF_TUNING, motifBurst, motifRing, motifShot, timesOf, type CueSpec } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import { flourishSpec } from "../core/flourish.ts";
import type { CardRecipe } from "../recipe.ts";
import { markLook } from "./markLook.ts";

export const duoBond: CardRecipe = (event, p, play) => {
  if (event.type !== "marked" || !event.added || event.mark !== DUO_MARK) return null;
  const at = locateInstance(p.view, event.instanceId);
  if (at === null) return null;
  const base = flourishSpec(p, play);
  const spec: CueSpec = { ...base, look: markLook(event.color, base.look) };
  const t = timesOf(p.D);
  const to = tid(at);
  const from = locateInstance(p.view, play.instanceId) ?? testid.hero(sideOf(p.view, play.player));
  const cues: FxCue[] = [];
  if (from !== at) cues.push(motifBurst(spec, tid(from), "ring", t.G, MOTIF_TUNING.gather), motifShot(spec, "beam", tid(from), to, t.T, t.L - t.T));
  cues.push(
    raysCue(p.D, "radiant", to, t.L),
    motifBurst(spec, to, "area", t.L, MOTIF_TUNING.land),
    motifRing(spec, p.D, to, t.L),
    ...shakeCues(spec.scale, FX_DUO_BOND_TRAUMA, t.L),
  );
  return { cues, row: "keep" };
};
