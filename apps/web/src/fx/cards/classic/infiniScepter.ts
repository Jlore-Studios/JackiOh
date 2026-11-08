// R3389001: Classic #7 InfiniScepter (`classic-007`), a Field Spell's signature: entrance "sceptrePlant",
// bespoke "sceptreLocket" and its aura "sceptreLoop" (R3389000). A crystal sceptre spins down into its
// zone and plants itself upright, its looped head opening like a locket. Its Cry draws the Spell it picks
// into the locket as it is exiled, a card back from its owner's hand to its zone ringed in violet light
// (the pick is its owner's alone until it moves, R202); each Activate (its own `activated`, in place of
// that row) pulses the sceptre in rings and a spectral copy, a card back, flies out of it to the middle
// of the board to be cast. Standing, the held Spell turns in its loop, a figure-eight trail round it.

import { animTestid } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_CENTER, FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook, type FxPalette } from "../../flavors.ts";
import { isFirstEntry, motifBurst, motifRing, timesOf } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPoint } from "../../types.ts";
import { cardAnchor, ENTRANCE_TRAUMA, heroOf, ownWindow, type CardRecipe, type Entrance } from "../recipe.ts";
import { flourishSpec, type FlourishFixture } from "./flourish.ts";

/** The look the locket and the copy shine in. */
const PALETTE: FxPalette = "Nerd";

/** Every particle count it throws, the entrance's at intensity "normal", the recipe's at scale 1 (rule 9). */
const TUNING = {
  spin: { count: 10, power: 0.6 },
  plant: { count: 20, power: 1 },
  locket: { count: 14, power: 0.8 },
  store: { count: 12, power: 0.7 },
} as const;

/** Where the sceptre spins down from: two of its zone's heights above it. */
const ABOVE: FxPoint = { x: 0.5, y: -2 };
/** The head of the sceptre, where its locket opens. */
const HEAD: FxPoint = { x: 0.5, y: 0.25 };
/** Where the copy flies to be cast: the middle of the board. */
const CENTER: FxAnchor = { kind: "viewport", at: { x: FX_CENTER.x, y: FX_CENTER.y } };

const EXILED = ["exiled"] as const;

export const sceptrePlant: Entrance = ({ D, tgt, intensity: i, landMs }) => {
  const at = tid(tgt);
  const land = landMs ?? frac(FX_SLAM_AT, D);
  const look = paletteLook(PALETTE) ?? FLAVOR_LOOKS.Nerd;
  return [
    { kind: "shot", shot: { motion: "bolt", trail: "sparkle", wave: true }, colors: look.colors, preset: "sparkle", from: tid(tgt, ABOVE), to: at, delayMs: 0, flightMs: land, density: i },
    tunedBurst(i, "sparkle", tid(tgt, ABOVE), "point", 0, TUNING.spin),
    ringCue(D, "arcane", at, land),
    tunedBurst(i, "arcane", at, "area", land, TUNING.plant),
    tunedBurst(i, "frost", tid(tgt, HEAD), "ring", land, TUNING.locket),
    { kind: "sheen", at, delayMs: land, durationMs: D - land },
    ...shakeCues(i, ENTRANCE_TRAUMA.sceptrePlant, land),
  ];
};

/** #7 InfiniScepter stores the Spell its Cry exiles and sends a copy out on each Activate. */
export const sceptreLocket: CardRecipe = (event, p, play) => {
  const spec = flourishSpec(PALETTE, play, p.view, p.env);
  if (!(p.D > 0) || !(spec.scale > 0)) return null;
  const t = timesOf(p.D);
  const zone = cardAnchor(p, play.instanceId) ?? heroOf(p.view, play);
  if (event.type === "exiled" && event.owner === play.player && ownWindow(p, play)) {
    // Once a play, on its Cry's first exile.
    const first = p.entry.events.find((candidate) => candidate.type === "exiled" && candidate.owner === play.player);
    if (first !== event || !isFirstEntry(p.env, p.entry, EXILED)) return null;
    const hand = tid(animTestid.hand(sideOf(p.view, play.player)));
    return {
      cues: [
        { kind: "ghost", from: hand, to: zone, delayMs: t.G, durationMs: t.L - t.G },
        motifRing(spec, p.D, zone, t.L),
        motifBurst(spec, zone, "ring", t.L, TUNING.store, "sparkle"),
      ],
      row: "keep",
    };
  }
  if (event.type === "activated" && event.instanceId === play.instanceId) {
    const cues: FxCue[] = [
      motifRing(spec, p.D, zone, t.G),
      { kind: "sheen", at: zone, delayMs: t.G, durationMs: p.D - t.G },
      { kind: "ghost", from: zone, to: CENTER, delayMs: t.T, durationMs: t.L - t.T },
      motifRing(spec, p.D, CENTER, t.L),
    ];
    return { cues, row: "replace" };
  }
  return null;
};

/** The gallery's InfiniScepter: its Cry exiles a Spell from its player's hand, then it activates. */
export const sceptreLocketFixture: FlourishFixture = (_view, instanceId, player) => [
  { type: "exiled", instanceId: "fx-gallery-stored", defId: "core-016", owner: player },
  { type: "activated", player, instanceId, defId: "classic-007", ability: "cast-copy" },
];
