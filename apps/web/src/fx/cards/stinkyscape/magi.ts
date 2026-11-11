// R3407000: the three MAGI, Imitated Instrumentality's Field Spells (`stinkyscape-018-1` to `-3`), each a
// signature of its own: an entrance as it boots in its zone, a recipe for its hooks (its Start of turn
// ballot, its Activate) and a standing look on its zone (stinkyscape-1.json5).
//
// - MELCHIOR (`melchiorBoot`, `melchiorTally`, aura `scanline`): a cyan core column boots in its zone, a
//   white scan line sweeps the board and a hologram figure blinks on over each of its side's lanes. Its
//   ballot ticks each face-up Unit of both sides, then stamps a cyan ballot on its zone; its Activate beams
//   from its zone to each Unit it grants a keyword (`keywordGranted`), whose own callout pins the icon.
// - BALTHASAR (`balthasarLamp`, `balthasarCradle`, aura `heartbeat`): a rose core warms on like a nursery
//   lamp and a heart-monitor line traces out to its owner's hero. Its ballot drops in rose with the line
//   back to the hero; its Activate cradles a card back from its zone into its owner's hand pile.
// - CASPER (`casperMirror`, `casperKiss`, aura `glint`): a violet core flickers on behind a mirror pane, a
//   rose glint sweeping the glass. Its ballot drops in violet; its Allure blows a kiss of rose light at the
//   Unit it marks (`marked`), and a Unit won over (`controlChanged`) bursts in it.
//
// No cue names a card: a hand is its pile, a card a back, and a Unit only ever the zone or card the view
// shows (R202). Each beat starts inside its entry and ends within FX_MAX_TAIL_MS of it (R200).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { ALLURE_MARK, ALLURE_MARK_COLOR } from "../../../cards/marks.ts";
import { HIDDEN_ID, locateInstance } from "../../../game/animations.ts";
import { LANES, sideOf, testid } from "../../../game/contract.ts";
import { frac, raysCue, shakeCues, tid } from "../../build.ts";
import { motifBurst, motifRing, motifShot, type CueSpec } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPlay, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardRecipe, type Entrance } from "../recipe.ts";
import { backTo, ballot, entranceSpec, entranceTimes, pileOf, sigSpec } from "./signature.ts";

/** Every particle count the MAGI throw at scale 1 (rule 9). */
const TUNING = {
  column: { count: 18, power: 1.1 },
  figure: { count: 4, power: 0.5 },
  tick: { count: 3, power: 0.4 },
  boot: { count: 16, power: 0.9 },
  lamp: { count: 12, power: 0.6 },
  flicker: { count: 6, power: 0.5 },
  won: { count: 14, power: 1 },
} as const;

/** Each beat's moments as fractions of its entry's D (an entrance's of its motion after its hold). */
const TIMING = {
  melchiorBoot: { columnAt: 0.1, scanAt: 0.2, figuresAt: 0.25, figureStep: 0.06 },
  melchiorTally: { tickAt: 0.05, tickStep: 0.03, ballotAt: 0.4, grantAt: 0.15 },
  balthasarLamp: { warmAt: 0.1, lineAt: 0.3 },
  balthasarCradle: { ballotAt: 0.2, lineAt: 0.25, sendAt: 0.15, sendFor: 0.7 },
  casperMirror: { flickerAt: 0.1, flickerStep: 0.2, glintAt: 0.3 },
  casperKiss: { ballotAt: 0.2, kissAt: 0.1, kissLand: 0.35, wonAt: 0.15 },
} as const;

/** The palettes: the MAGI's hologram, BALTHASAR's rose and CASPER's violet. */
const HOLOGRAM = "Tech/hologram";
const ROSE = "Clout";
const VIOLET = "Dark";

/** The top of a zone, where a hologram figure blinks on. */
const TOP: FxPoint = { x: 0.5, y: 0 };

/** The MAGI's ballot entry: their Start of turn hook, which runs inside no play (R2797000). */
const START_OF_TURN = "startOfTurn";

/** R3407000: the zone or card `instanceId` stands in, else its controller's backrow (a MAGI the view no longer shows). */
function standing(view: PlayerView, play: FxPlay): FxAnchor {
  const at = locateInstance(view, play.instanceId);
  return tid(at ?? `backrow-${sideOf(view, play.player)}`);
}

/** Each face-up Unit of both sides, as the view shows them, in lane order, your side first. */
function faceUpUnits(view: PlayerView): FxAnchor[] {
  return (["you", "opponent"] as const).flatMap((side) =>
    view[side].units.flatMap((unit) => (unit === null || unit.defId === HIDDEN_ID ? [] : [tid(testid.card(unit.instanceId))])),
  );
}

/** Whether `event` is the vote of the play's own MAGI. */
function ownVote(event: GameEvent, play: FxPlay): event is Extract<GameEvent, { type: "voteCast" }> {
  return event.type === "voteCast" && event.instanceId === play.instanceId;
}

/* MELCHIOR */

export const melchiorBoot: Entrance = (p) => {
  const spec = entranceSpec(p, HOLOGRAM);
  const { at, slam } = entranceTimes(p);
  const T = TIMING.melchiorBoot;
  const zone = tid(p.tgt);
  const cues: FxCue[] = [
    motifBurst(spec, tid(p.tgt, FOOT), "area", at(T.columnAt), TUNING.column),
    { kind: "sheen", at: tid(testid.board), delayMs: at(T.scanAt), durationMs: p.D - at(T.scanAt) },
    { kind: "sheen", at: zone, delayMs: at(0), durationMs: p.D - at(0) },
    motifRing(spec, p.D, zone, slam),
    motifBurst(spec, zone, "ring", slam, TUNING.boot),
  ];
  if (p.view !== undefined && p.landing !== undefined) {
    const side = sideOf(p.view, p.landing.player);
    LANES.forEach((lane, k) => cues.push(motifBurst(spec, tid(testid.zone(side, "units", lane), TOP), "point", at(T.figuresAt + k * T.figureStep), TUNING.figure)));
  }
  cues.push(...shakeCues(p.intensity, ENTRANCE_TRAUMA.melchiorBoot, slam));
  return cues;
};

export const melchiorTally: CardRecipe = (event, p, play) => {
  const spec = sigSpec(p, play, HOLOGRAM);
  const T = TIMING.melchiorTally;
  if (ownVote(event, play)) {
    const cues: FxCue[] = faceUpUnits(p.view).map((unit, k) => motifBurst(spec, unit, "point", frac(T.tickAt + k * T.tickStep, p.D), TUNING.tick));
    cues.push(...ballot(spec, p.D, standing(p.view, play), frac(T.ballotAt, p.D)));
    return { cues, row: "replace" };
  }
  if (event.type === "keywordGranted" && play.hook === "activate") {
    const at = locateInstance(p.view, event.instanceId);
    if (at === null) return null;
    // The core's scan pins the keyword: a hologram beam from its zone to the Unit, whose own callout still plays.
    const delayMs = frac(T.grantAt, p.D);
    return { cues: [motifShot(spec, "beam", standing(p.view, play), tid(at), 0, delayMs), motifRing(spec, p.D, tid(at), delayMs)], row: "keep" };
  }
  return null;
};

/* BALTHASAR */

export const balthasarLamp: Entrance = (p) => {
  const spec = entranceSpec(p, ROSE);
  const teal = entranceSpec(p, HOLOGRAM);
  const { at, slam } = entranceTimes(p);
  const T = TIMING.balthasarLamp;
  const zone = tid(p.tgt);
  const cues: FxCue[] = [
    raysCue(p.D, "holy", zone, at(T.warmAt)),
    motifBurst(spec, zone, "area", at(T.warmAt), TUNING.lamp),
    motifRing(teal, p.D, zone, slam),
  ];
  if (p.view !== undefined && p.landing !== undefined) {
    cues.push(motifShot(spec, "beam", zone, tid(testid.hero(sideOf(p.view, p.landing.player))), at(T.lineAt), Math.max(0, slam - at(T.lineAt))));
  }
  cues.push(...shakeCues(p.intensity, ENTRANCE_TRAUMA.balthasarLamp, slam));
  return cues;
};

export const balthasarCradle: CardRecipe = (event, p, play) => {
  const T = TIMING.balthasarCradle;
  if (ownVote(event, play)) {
    const spec = sigSpec(p, play, ROSE);
    const from = standing(p.view, play);
    const lineAt = frac(T.lineAt, p.D);
    return {
      cues: [...ballot(spec, p.D, from, frac(T.ballotAt, p.D)), motifShot(spec, "beam", from, tid(testid.hero(sideOf(p.view, play.player))), lineAt, p.D - lineAt)],
      row: "replace",
    };
  }
  if (event.type === "addedToHand" && play.hook === "activate") {
    const delayMs = frac(T.sendAt, p.D);
    return { cues: [backTo(standing(p.view, play), pileOf(p.view, event.player, "hand"), delayMs, frac(T.sendFor, p.D))], row: "keep" };
  }
  return null;
};

/* CASPER */

export const casperMirror: Entrance = (p) => {
  const spec = entranceSpec(p, VIOLET);
  const rose = entranceSpec(p, ROSE);
  const { at, slam } = entranceTimes(p);
  const T = TIMING.casperMirror;
  const zone = tid(p.tgt);
  return [
    motifBurst(spec, zone, "point", at(T.flickerAt), TUNING.flicker),
    motifBurst(spec, zone, "point", at(T.flickerAt + T.flickerStep), TUNING.flicker),
    { kind: "sheen", at: zone, delayMs: at(T.glintAt), durationMs: p.D - at(T.glintAt) },
    motifRing(rose, p.D, zone, slam),
    motifBurst(rose, zone, "ring", slam, TUNING.boot),
    ...shakeCues(p.intensity, ENTRANCE_TRAUMA.casperMirror, slam),
  ];
};

export const casperKiss: CardRecipe = (event, p, play) => {
  const T = TIMING.casperKiss;
  if (ownVote(event, play)) return { cues: ballot(sigSpec(p, play, VIOLET), p.D, standing(p.view, play), frac(T.ballotAt, p.D)), row: "replace" };
  const spec: CueSpec = sigSpec(p, play, ROSE);
  if (event.type === "marked" && event.added && event.mark === ALLURE_MARK) {
    const at = locateInstance(p.view, event.instanceId);
    if (at === null) return null;
    const leave = frac(T.kissAt, p.D);
    const land = frac(T.kissLand, p.D);
    return { cues: [motifShot(spec, "bolt", standing(p.view, play), tid(at), leave, land - leave), motifRing(spec, p.D, tid(at), land)], row: "keep" };
  }
  if (event.type === "controlChanged") {
    const at = locateInstance(p.view, event.instanceId);
    if (at === null) return null;
    const delayMs = frac(T.wonAt, p.D);
    return {
      cues: [motifShot(spec, "beam", standing(p.view, play), tid(at), 0, delayMs), motifBurst(spec, tid(at), "area", delayMs, TUNING.won)],
      row: "keep",
    };
  }
  return null;
};

/* The gallery's fixtures */

const marker = (instanceId: string, player: PlayerId, defId: string, hook: "startOfTurn" | "activate"): GameEvent => ({
  type: "hookResolving",
  sourceId: instanceId,
  defId,
  controller: player,
  hook,
});

const vote = (instanceId: string, player: PlayerId, defId: string): GameEvent => ({ type: "voteCast", player, instanceId, defId, votes: 1, needed: 3 });

/**
 * R3407000: MELCHIOR's Activate: its controller's first face-up Unit gains a combat keyword (one grant
 * stands for each, so the run fits its ceiling). Its ballot
 * (`magiBallotFixture`) is the module test's: the shared vote moment (systemMoments.ts) draws under it,
 * and the two do not fit after its entrance in one Legendary's ceiling.
 */
export function melchiorTallyFixture(view: PlayerView, instanceId: string, player: PlayerId, defId = "stinkyscape-018-1"): GameEvent[] {
  const own = player === view.viewer ? view.you : view.opponent;
  const unit = own.units.find((u) => u !== null && u.defId !== HIDDEN_ID);
  const granted: GameEvent[] = unit === undefined || unit === null ? [] : [{ type: "keywordGranted", instanceId: unit.instanceId, keyword: { kind: "Taunt" } }];
  return [marker(instanceId, player, defId, "activate"), ...granted];
}

/** R3407000: BALTHASAR's Activate: a copy of the last card played is added to its controller's hand. */
export function balthasarCradleFixture(_view: PlayerView, instanceId: string, player: PlayerId, defId = "stinkyscape-018-2"): GameEvent[] {
  return [marker(instanceId, player, defId, "activate"), { type: "addedToHand", player, instanceId: "fx-cradled", defId: "core-003" }];
}

/** R3407000: a MAGI's Start of turn ballot (its vote), the beat each module test proves beside its gallery play. */
export function magiBallotFixture(instanceId: string, player: PlayerId, defId: string): GameEvent[] {
  return [marker(instanceId, player, defId, START_OF_TURN), vote(instanceId, player, defId)];
}

/** R3407000: CASPER's Activate: it Allures the first enemy Unit. */
export function casperKissFixture(view: PlayerView, instanceId: string, player: PlayerId, defId = "stinkyscape-018-3"): GameEvent[] {
  const enemy = player === view.viewer ? view.opponent : view.you;
  const foe = enemy.units.find((unit) => unit !== null && unit.defId !== HIDDEN_ID);
  return [marker(instanceId, player, defId, "activate"), ...(foe === undefined || foe === null ? [] : [allured(foe.instanceId)])];
}

const allured = (instanceId: string): GameEvent => ({ type: "marked", instanceId, mark: ALLURE_MARK, color: ALLURE_MARK_COLOR, added: true });
