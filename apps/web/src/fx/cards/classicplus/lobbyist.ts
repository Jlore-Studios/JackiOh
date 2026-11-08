// R3399000: Classic+ Jlockeed's Lobbyist (`classicplus-048`, entrance "briefcaseStride", bespoke "sealedEnvelope"),
// its signature (#3399): his entrance and his death.
// Entrance: he strides in from his hero (a bolt), briefcase first; at the landing it snaps open (ring, circuit burst,
// shake), blueprints fan out like a hand of cards (a ring-spread of sparks), and a red laser dot sweeps the enemy
// board, a beam from the briefcase to each enemy unit zone in lane order.
// Death (`addedToHand`): the briefcase pops where he stood (his old zone if the view shows it, else the graveyard)
// and a sealed envelope, a card back, zips to the owner's hand pile. The `costChanged` that stamps it zero is drawn
// only for the owner (the engine hides it from the opponent): a stamp of sparks at the hand pile.
// R202: nothing here names a card or shows the envelope's face.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { animTestid } from "../../../game/animations.ts";
import { LANES, sideOf, testid } from "../../../game/contract.ts";
import { frac, shakeCues, tid } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { hitAnchor, motifBurst, motifRing, motifShot, otherSide, timesOf, type CueSpec } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, type CardRecipe, type Entrance } from "../recipe.ts";

/** Every particle count the briefcase throws, at intensity "normal" (rule 9). */
const TUNING = {
  snap: { count: 20, power: 1.1 },
  fan: { count: 12, power: 1 },
  dot: { count: 4, power: 0.5 },
  pop: { count: 14, power: 1 },
  stamp: { count: 10, power: 0.8 },
} as const;

/** Entrance beats, as fractions of D. */
const STRIDE_AT = 0.05;
const FAN_AT = 0.7;
const SWEEP_AT = 0.72;
const SWEEP_STEP = 0.05;
const SWEEP_FLIGHT = 0.08;
/** The stamp's own beat, after the envelope has arrived. */
const STAMP_AFTER = 0.1;

const look = paletteLook("Tech/laser") ?? FLAVOR_LOOKS.Tech;

export const briefcaseStride: Entrance = ({ D, tgt, intensity: i, view, landing }) => {
  const spec: CueSpec = { look, scale: i, row: "keep" };
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  const cues: FxCue[] = [
    motifRing(spec, D, at, slam),
    motifBurst(spec, at, "area", slam, TUNING.snap),
    motifBurst(spec, at, "ring", frac(FAN_AT, D), TUNING.fan, "spark"),
    ...shakeCues(i, ENTRANCE_TRAUMA.briefcaseStride, slam),
  ];
  if (view === undefined || landing === undefined) return cues;
  const side = sideOf(view, landing.player);
  const sprint = frac(STRIDE_AT, D);
  cues.push(motifShot(spec, "bolt", tid(testid.hero(side)), at, sprint, slam - sprint));
  LANES.forEach((lane, k) => {
    const zone = tid(testid.zone(otherSide(side), "units", lane));
    const delay = frac(SWEEP_AT + k * SWEEP_STEP, D);
    cues.push(motifShot(spec, "beam", at, zone, delay, frac(SWEEP_FLIGHT, D)), motifBurst(spec, zone, "point", delay, TUNING.dot, "spark"));
  });
  return cues;
};

/** #48: the briefcase pops and a sealed envelope zips to its owner's hand. */
export const sealedEnvelope: CardRecipe = (event, p, play) => {
  if (play.hook !== "death") return null;
  const t = timesOf(p.D);
  const spec: CueSpec = { look, scale: p.env.intensity, row: "keep" };
  if (event.type === "addedToHand") {
    const side = sideOf(p.view, event.player);
    const from = hitAnchor(p.view, play.instanceId) ?? tid(animTestid.graveyard(side));
    const cues: FxCue[] = [
      motifBurst(spec, from, "area", t.G, TUNING.pop),
      { kind: "ghost", from, to: tid(animTestid.hand(side)), delayMs: t.T, durationMs: t.L - t.T },
    ];
    return { cues, row: "keep" };
  }
  if (event.type === "costChanged" && play.player === p.view.viewer) {
    const hand = tid(animTestid.hand(sideOf(p.view, play.player)));
    return { cues: [motifBurst(spec, hand, "point", Math.min(p.D, t.L + frac(STAMP_AFTER, p.D)), TUNING.stamp, "spark")], row: "keep" };
  }
  return null;
};

/** The gallery's death: an envelope reaches the hand, its cost stamped to zero. */
export function sealedEnvelopeFixture(_view: PlayerView, instanceId: string, player: PlayerId): GameEvent[] {
  return [
    { type: "hookResolving", sourceId: instanceId, defId: "classicplus-048", controller: player, hook: "death" },
    { type: "addedToHand", player, instanceId: "fx-envelope", defId: "core-005" },
    { type: "costChanged", instanceId: "fx-envelope", cost: 0 },
  ];
}
