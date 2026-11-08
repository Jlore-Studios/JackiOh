// R3389001: Classic #28 Second Wind (`classic-028`), a Field Spell's signature: entrance "galeBurst",
// bespoke "secondGust" and its aura "graveBreeze" (R3389000). A gale bursts from its zone and the card
// lands upright in a whirl of sky-blue wind. Its Cry blows the deck away back by back (its first
// `exiled` of its owner's cards: card backs from the deck pile to the exile pile, one after another),
// then scatters the hand into the graveyard (its first `discarded`: backs from the hand to the graveyard
// pile, ringed there, in place of the discard's own row). A whole deck or hand going at once is its
// pile's one wave, and the backs fly beside it (R3389000). Standing, a soft breeze ruffles the
// graveyard, leaf-motes lifting off it. Moved: a card later played from the graveyard, and one bound
// there veering into exile, which open no play of Second Wind's and play the shared rows.

import { animTestid } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook, type FxPalette } from "../../flavors.ts";
import { isFirstEntry, motifBurst, motifRing, timesOf } from "../../motifs/build.ts";
import type { FxCue, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, ownWindow, SIGNATURE_RUN_MAX, stepMs, type CardRecipe, type Entrance } from "../recipe.ts";
import { flourishSpec, type FlourishFixture } from "./flourish.ts";

/** The look its wind blows in. */
const PALETTE: FxPalette = "Physical/wind";

/** Every particle count it throws, the entrance's at intensity "normal", the recipe's at scale 1 (rule 9). */
const TUNING = {
  gust: { count: 18, power: 1.3 },
  whirl: { count: 22, power: 0.9 },
  leaves: { count: 10, power: 0.7 },
  scatter: { count: 10, power: 0.8 },
} as const;

/** The gale's side: where it bursts from, beside its zone. */
const SIDEWAYS: FxPoint = { x: -0.2, y: 0.5 };

export const galeBurst: Entrance = ({ D, tgt, intensity: i, landMs }) => {
  const at = tid(tgt);
  const land = landMs ?? frac(FX_SLAM_AT, D);
  const look = paletteLook(PALETTE) ?? FLAVOR_LOOKS.Physical;
  return [
    { ...tunedBurst(i, "dust", tid(tgt, SIDEWAYS), "point", 0, TUNING.gust), colors: look.colors },
    { kind: "sheen", at, delayMs: 0, durationMs: land },
    ringCue(D, "dust", at, land),
    { ...tunedBurst(i, "dust", at, "ring", land, TUNING.whirl), colors: look.colors },
    tunedBurst(i, "leaf", tid(tgt, FOOT), "area", land, TUNING.leaves),
    ...shakeCues(i, ENTRANCE_TRAUMA.galeBurst, land),
  ];
};

/** #28 Second Wind blows its deck into exile and its hand into the graveyard, once each a play. */
export const secondGust: CardRecipe = (event, p, play) => {
  if ((event.type !== "exiled" && event.type !== "discarded") || event.owner !== play.player || !ownWindow(p, play)) return null;
  const type = event.type;
  const run = p.entry.events.filter((candidate) => candidate.type === type && candidate.owner === play.player);
  if (run[0] !== event || !isFirstEntry(p.env, p.entry, [type])) return null;
  const spec = flourishSpec(PALETTE, play, p.view, p.env);
  if (!(p.D > 0) || !(spec.scale > 0)) return null;
  const t = timesOf(p.D);
  const side = sideOf(p.view, play.player);
  const from = tid(type === "exiled" ? animTestid.library(side) : animTestid.hand(side));
  const to = tid(type === "exiled" ? animTestid.exile(side) : animTestid.graveyard(side));
  const ghosts = run.slice(0, SIGNATURE_RUN_MAX).map((_, k): FxCue => {
    const at = stepMs(p.D, t.G, k);
    return { kind: "ghost", from, to, delayMs: at, durationMs: Math.max(0, t.L - at) };
  });
  // The deck blows away in a gust; the hand scatters after it, ringed where it lands.
  if (type === "exiled") return { cues: [motifBurst(spec, from, "point", t.G, TUNING.scatter), ...ghosts, motifBurst(spec, to, "area", t.L, TUNING.scatter, "leaf")], row: "keep" };
  return { cues: [...ghosts, motifRing(spec, p.D, to, t.G)], row: "replace" };
};

/** The gallery's Second Wind: its Cry exiles a card off its deck and discards one from its hand. */
export const secondGustFixture: FlourishFixture = (_view, instanceId, player) => [
  { type: "exiled", instanceId: `${instanceId}-blown`, defId: "core-003", owner: player },
  { type: "discarded", instanceId: `${instanceId}-scattered`, defId: "core-055", owner: player },
];
