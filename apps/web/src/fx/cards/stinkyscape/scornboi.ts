// R3407000: Stinkyscape #115 Scornboi (`stinkyscape-115`, a Legendary Unit: entrance "scornGate", recipe
// "scornSweep", aura "searchlight" in Dark/blood on his tile), a signature.
//
// - His entrance: a crimson gate slams down over his zone (a crimson bolt from above it to its foot, a ring,
//   a crack and crimson dust at the slam, a shake); he strides through it, his greatcoat snapping (a sheen
//   over the zone and a puff of crimson smoke), and unrolls a very short blank list (the `scroll` figure,
//   small) with a sneer.
// - His play: Tribute 3 pays for it, he lands, and the state check after his landing applies his exile aura
//   (R1850002): every Unit that is no Human, Starship Piece or Starship (on the Radiant face every enemy one)
//   is `exiled` while his play resolves. Each such `exiled` is picked out by a crimson beam from him and
//   dragged into a void hatch (a void ring and a ring of void where it stood); on the first, the Units his
//   aura spares glint (a sheen on each).
//
// Moved from the issue's concept: the table's old `cry: wipe` is gone; he has no Cry, and his sweep is his
// aura's, so it keys on the `exiled` of his own play. A Unit his aura exiles later (one landing at a later
// state check, in another card's play) runs in no entry of his and plays the plain exile row. The spared are
// read off the views, not the rules: the Units the view shows face up, him and the exiled aside, that the
// view after the action (`env.next`) still shows; with no next view (the gallery), those not yet exiled.
//
// R202: an exile of a face-up Unit is public; the beam and the hatch go only to a card the view shows face up
// on the board, and the glint only to the Units it shows face up. No cue names a card. He stands at his card
// while the view shows it, else in the zone his `summoned` named, else at his hero.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { HIDDEN_ID, locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid } from "../../build.ts";
import { FX_CRACK_TAIL_MS } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { motifBurst, motifRing, motifShot } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPlay, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardFxPlan, type CardRecipe, type Entrance } from "../recipe.ts";
import { entranceSpec, entranceTimes, otherPlayer, sigSpec } from "./signature.ts";

/** Every particle count his signature throws at scale 1 (rule 9). */
const TUNING = {
  gate: { count: 18, power: 1.1 },
  coat: { count: 8, power: 0.6 },
  hatch: { count: 16, power: 0.9 },
} as const;

/** Each beat's moments as fractions of its entry's D (an entrance's of its motion after its hold), and its tails in ms. */
const TIMING = {
  scornGate: { gateAt: 0.1, strideAt: 0.7, listAt: 0.75, tailMs: 300 },
  scornSweep: { beamAt: 0.05, beamFor: 0.3, glintAt: 0.4 },
} as const;

/** His crimson. */
const BLOOD = "Dark/blood";

/** Where the gate comes down from, above his zone, and where his list hangs, at its top (fractions of the zone). */
const GATE_FROM: FxPoint = { x: 0.5, y: -0.6 };
const LIST: FxPoint = { x: 0.5, y: 0.25 };

/** R3407000: where he stands: his card while the view shows it, else the zone his `summoned` named, else his hero. */
function standing(p: CardFxPlan, play: FxPlay): FxAnchor {
  const at = locateInstance(p.view, play.instanceId);
  if (at !== null && at.startsWith("card-")) return tid(at);
  const landed = [...p.env.memory.recent()].reverse().find((event) => event.type === "summoned" && event.instanceId === play.instanceId);
  if (landed?.type === "summoned") return tid(testid.zone(sideOf(p.view, landed.player), landed.row, landed.lane));
  return tid(testid.hero(sideOf(p.view, play.player)));
}

/** The card testid of a Unit `view` shows face up on the board, else null. */
function boardCard(view: PlayerView, instanceId: string): string | null {
  const at = locateInstance(view, instanceId);
  return at !== null && at.startsWith("card-") ? at : null;
}

/**
 * The Units his aura spares, as the views show them: each face-up Unit but him and those exiled so far, that
 * the view after the action still shows (with none, every one not exiled yet).
 */
function spared(p: CardFxPlan, play: FxPlay): FxAnchor[] {
  const gone = new Set(play.hits ?? []);
  const next = p.env.next;
  return (["you", "opponent"] as const).flatMap((side) =>
    p.view[side].units.flatMap((unit) => {
      if (unit === null || unit.defId === HIDDEN_ID || unit.instanceId === play.instanceId || gone.has(unit.instanceId)) return [];
      if (next !== undefined && locateInstance(next, unit.instanceId) === null) return [];
      return [tid(testid.card(unit.instanceId))];
    }),
  );
}

export const scornGate: Entrance = (p) => {
  const spec = entranceSpec(p, BLOOD);
  const { at, slam } = entranceTimes(p);
  const T = TIMING.scornGate;
  const zone = tid(p.tgt);
  const foot = tid(p.tgt, FOOT);
  const side = p.view === undefined || p.landing === undefined ? {} : { side: sideOf(p.view, p.landing.player) };
  return [
    motifShot(spec, "bolt", tid(p.tgt, GATE_FROM), foot, at(T.gateAt), slam - at(T.gateAt)),
    motifRing(spec, p.D, zone, slam),
    { kind: "crack", at: foot, delayMs: slam, durationMs: p.D - slam + FX_CRACK_TAIL_MS },
    motifBurst(spec, foot, "area", slam, TUNING.gate, "dust"),
    { kind: "sheen", at: zone, delayMs: at(T.strideAt), durationMs: p.D - at(T.strideAt) },
    motifBurst(spec, zone, "point", at(T.strideAt), TUNING.coat, "smoke"),
    figureCue("scroll", tid(p.tgt, LIST), figureTint(spec.look), at(T.listAt), p.D - at(T.listAt) + T.tailMs, { size: "small", ...side }),
    ...shakeCues(p.intensity, ENTRANCE_TRAUMA.scornGate, slam),
  ];
};

export const scornSweep: CardRecipe = (event, p, play) => {
  if (event.type !== "exiled" || play.hook !== undefined || event.instanceId === play.instanceId) return null;
  const target = boardCard(p.view, event.instanceId);
  if (target === null) return null;
  const T = TIMING.scornSweep;
  const spec = sigSpec(p, play, BLOOD);
  const at = tid(target);
  const leave = frac(T.beamAt, p.D);
  const land = leave + frac(T.beamFor, p.D);
  const cues: FxCue[] = [motifShot(spec, "beam", standing(p, play), at, leave, land - leave), ringCue(p.D, "void", at, land), motifBurst(spec, at, "ring", land, TUNING.hatch, "void")];
  // The first exile of his play: the spared glint.
  if ((play.seen.exiled ?? 0) - p.entry.events.filter((candidate) => candidate.type === "exiled").length === 0 && p.entry.events.find((candidate) => candidate.type === "exiled") === event) {
    const glint = frac(T.glintAt, p.D);
    for (const unit of spared(p, play)) cues.push({ kind: "sheen", at: unit, delayMs: glint, durationMs: p.D - glint });
  }
  return { cues, row: "replace" };
};

/** R3407000: his aura exiles the first face-up Unit of the other side (one exile stands for each, so the run fits its ceiling). */
export function scornSweepFixture(view: PlayerView, _instanceId: string, player: PlayerId): GameEvent[] {
  const enemy = otherPlayer(player) === view.viewer ? view.you : view.opponent;
  const foe = enemy.units.find((unit) => unit !== null && unit.defId !== HIDDEN_ID);
  return foe === undefined || foe === null ? [] : [{ type: "exiled", instanceId: foe.instanceId, defId: foe.defId, owner: foe.owner }];
}
