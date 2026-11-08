// R3389001: Classic #4 Palantir (`classic-004`), a Field Spell's signature: entrance "seeingOrb", bespoke
// "palantirGaze" and its aura "seeingIris" (R3389000). A black seeing-orb drops onto its zone and its iris
// opens in cyan light, and one scan sweeps the other side's unit row, lane by lane. When it steals, its
// base face's Tribute cracks the orb (its own `destroyed`, under its steal's marker: a crack, shards and
// the eye-beam locking onto the other hand), and the beam reels the announced Spell across from the
// other hand to its owner's (`countered` into a hand, by it), a card back ringed at both ends, in place
// of the counter's fizzle. Moved: the beam's lock to the crack, so the steal ends inside its budget; the
// draw limit's shutter over the other deck, since `drawLimited` names no card that set it; and the
// stolen Spell's cast, which plays its own entry as every cast does. No cue names a card (R202).

import { animTestid, targetFor } from "../../../game/animations.ts";
import { LANES, sideOf, testid } from "../../../game/contract.ts";
import { FX_CRACK_TAIL_MS, FX_SLAM_AT } from "../../constants.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FLAVOR_LOOKS, paletteLook, type FxPalette } from "../../flavors.ts";
import { motifBurst, motifRing, motifShot, otherSide, timesOf } from "../../motifs/build.ts";
import type { FxCue, FxPoint } from "../../types.ts";
import { cardAnchor, ENTRANCE_TRAUMA, heroOf, ownWindow, stepMs, type CardRecipe, type Entrance } from "../recipe.ts";
import { flourishSpec, type FlourishFixture } from "./flourish.ts";

/** The look its iris, its scan and its beam shine in. */
const PALETTE: FxPalette = "Tech/hologram";

/** Every particle count it throws, the entrance's at intensity "normal", the recipe's at scale 1 (rule 9). */
const TUNING = {
  orb: { count: 16, power: 0.6 },
  iris: { count: 20, power: 1 },
  scan: { count: 5, power: 0.5 },
  shards: { count: 18, power: 1.2 },
  reel: { count: 10, power: 0.8 },
} as const;

/** Where the orb drops from: two of its zone's heights above it. */
const ABOVE: FxPoint = { x: 0.5, y: -2 };

export const seeingOrb: Entrance = ({ D, tgt, intensity: i, view, landing, landMs }) => {
  const at = tid(tgt);
  const land = landMs ?? frac(FX_SLAM_AT, D);
  const look = paletteLook(PALETTE) ?? FLAVOR_LOOKS.Tech;
  const cues: FxCue[] = [
    { kind: "shot", shot: { motion: "bolt", trail: "void" }, colors: look.colors, preset: "void", from: tid(tgt, ABOVE), to: at, delayMs: 0, flightMs: land, density: i },
    tunedBurst(i, "void", at, "area", land, TUNING.orb),
    ringCue(D, "frost", at, land),
    { ...tunedBurst(i, "sparkle", at, "ring", land, TUNING.iris), colors: look.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.seeingOrb, land),
  ];
  if (view === undefined || landing === undefined) return cues;
  // The scan crosses the other side's unit row as the iris opens, lane by lane.
  const other = otherSide(sideOf(view, landing.player));
  LANES.forEach((lane, k) => {
    cues.push({ ...tunedBurst(i, "frost", tid(testid.zone(other, "units", lane)), "point", stepMs(D, land, k), TUNING.scan), colors: look.colors });
  });
  return cues;
};

/** #4 Palantir cracks as it pays its Tribute and reels the Spell it steals across. */
export const palantirGaze: CardRecipe = (event, p, play) => {
  const spec = flourishSpec(PALETTE, play, p.view, p.env);
  if (!(p.D > 0) || !(spec.scale > 0)) return null;
  const t = timesOf(p.D);
  const side = sideOf(p.view, play.player);
  const theirs = tid(animTestid.hand(otherSide(side)));
  if (event.type === "destroyed" && event.instanceId === play.instanceId && ownWindow(p, play)) {
    const tgt = targetFor(event, p.view);
    if (tgt === null) return null;
    const at = tid(tgt);
    return {
      cues: [
        { kind: "crack", at, delayMs: t.G, durationMs: p.D - t.G + FX_CRACK_TAIL_MS },
        motifBurst(spec, at, "ring", t.G, TUNING.shards, "shard"),
        motifShot(spec, "beam", at, theirs, t.T, 0),
        motifBurst(spec, theirs, "point", t.T, TUNING.reel),
      ],
      row: "keep",
    };
  }
  if (event.type === "countered" && event.byInstanceId === play.instanceId && event.to === "hand") {
    const mine = tid(animTestid.hand(side));
    const eye = cardAnchor(p, play.instanceId) ?? heroOf(p.view, play);
    return {
      cues: [
        motifRing(spec, p.D, eye, t.G),
        { kind: "sheen", at: theirs, delayMs: t.G, durationMs: p.D - t.G },
        { kind: "ghost", from: theirs, to: mine, delayMs: t.G, durationMs: t.L - t.G },
        motifRing(spec, p.D, mine, t.G),
      ],
      row: "replace",
    };
  }
  return null;
};

/** The gallery's Palantir: its steal's marker, its base face's Tribute and the Spell it reels into its owner's hand. */
export const palantirGazeFixture: FlourishFixture = (view, instanceId, player) => {
  const other = player === view.viewer ? view.opponent.player : view.you.player;
  return [
    { type: "hookResolving", sourceId: instanceId, defId: "classic-004", controller: player, hook: "whenever" },
    { type: "destroyed", instanceId, defId: "classic-004", owner: player, controller: player, attack: 0, maxHealth: 0, killerId: null },
    { type: "countered", player: other, instanceId: "fx-gallery-stolen", defId: "core-016", byInstanceId: instanceId, to: "hand" },
  ];
};
