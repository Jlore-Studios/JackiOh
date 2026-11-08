// #2798: Stinkyscape #110 iKwzki (`stinkyscape-110`, bespoke "duoBond"), bespoke because its Duo is a
// bond between two cards and no motif reads `marked`: a gather on iKwzki (its caster's hero when it is
// off the board), a beam to the permanent it Duos with, and there the Radiant rays it is promised, a
// ring and a burst, all in the Duo mark's own gold. It anchors on the shown ids and the heroes alone,
// the same on both seats (R202); the mark's own brand row still plays.
// R3407000: her signature (ikwzki.ts draws her entrance): the beam is the ribbon of starlight and the burst
// the knot of motes it ties there, as before; and in a hook entry of hers (`trigger`) the knot blooms on
// `radiantSet`, the card she Duoed turning Radiant at her next turn: the Radiant rays, a gold burst, a burst
// of rose motes and a ring on the bloomed card, in place of the radiant row's own. The bloomed card is its
// card where the view shows it on the field, else its zone (a face-down one), and a card off the field draws
// nothing. The engine marks no hook for a delayed effect (hook_marks.rs), so the bloom draws only inside a
// hook entry of hers.

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { DUO_MARK, DUO_MARK_COLOR } from "../../../cards/marks.ts";
import { frac, raysCue, shakeCues, tid } from "../../build.ts";
import { FX_DUO_BOND_TRAUMA } from "../../constants.ts";
import { MOTIF_TUNING, motifBurst, motifRing, motifShot, timesOf, type CueSpec } from "../../motifs/build.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import { flourishSpec } from "../core/flourish.ts";
import type { CardRecipe } from "../recipe.ts";
import { markLook } from "./markLook.ts";
import { lookOf } from "./signature.ts";

/** R3407000: the bloom's motes, in Clout's rose (rule 9). */
const BLOOM_ROSE = "Clout";

/** R3407000: the bloom's particle counts at scale 1 (rule 9). */
const TUNING = { motes: { count: 16, power: 0.8 } } as const;

/** R3407000: the bloom's moments as fractions of its entry's D. */
const TIMING = { bloomAt: 0.1, motesAt: 0.2 } as const;

/** R3407000: where a card turning Radiant on the field stands: its card where the view shows it, else its zone. */
function bloomed(view: PlayerView, event: Extract<GameEvent, { type: "radiantSet" }>): FxAnchor | null {
  const zone = event.zone;
  if (zone.z !== "field") return null;
  const card = testid.card(event.instanceId);
  return tid(locateInstance(view, event.instanceId) === card ? card : testid.zone(sideOf(view, zone.player), zone.row, zone.lane));
}

export const duoBond: CardRecipe = (event, p, play) => {
  if (event.type === "radiantSet" && play.hook === "trigger") {
    const at = bloomed(p.view, event);
    if (at === null) return null;
    const base = flourishSpec(p, play);
    const gold: CueSpec = { ...base, look: markLook(DUO_MARK_COLOR, base.look) };
    const rose: CueSpec = { ...base, look: { ...lookOf(BLOOM_ROSE), particle: gold.look.particle } };
    const bloom = frac(TIMING.bloomAt, p.D);
    const motes = frac(TIMING.motesAt, p.D);
    return {
      cues: [raysCue(p.D, "radiant", at, bloom), motifBurst(gold, at, "area", bloom, MOTIF_TUNING.land), motifBurst(rose, at, "ring", motes, TUNING.motes), motifRing(gold, p.D, at, bloom)],
      row: "replace",
    };
  }
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
