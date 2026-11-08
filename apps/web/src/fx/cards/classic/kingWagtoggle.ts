// R2794000: Classic #85 King Wagtoggle (`classic-085`, bespoke "crownSwap"). As its Cry swaps the two
// decks, each Deck pile lifts in Hustle's light, two card backs cross the board to the other pile, and a
// gold crown of sparks lands on each with a ring and a shake. The swap's own row still plays (`keep`).
// R3389001: grown into its signature (R3389000): entrance "royalCloak", bespoke "crownSwap" and its aura
// "cloakShimmer" on its own tile (R3400000). A velvet cloak sweeps in over its zone as it lands, and the King tips
// his crown to the far side, a glint over the zone across his lane. As the decks swap, the two piles bow
// and cross under a crown ringed at the middle of the board, card backs only (R202); on the Radiant face,
// the card it Recruits leaps from its new deck into play (a `summoned` of its own player inside its play).
// Standing, a slow crimson shimmer runs along its cloak's hem.

import { animTestid, targetFor } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_CENTER, FX_CROWN_SWAP_TRAUMA, FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook, type FxPalette } from "../../flavors.ts";
import { motifBurst, motifRing, otherSide, timesOf } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, ownWindow, type CardRecipe, type Entrance } from "../recipe.ts";
import { flourishSpec, type FlourishFixture } from "./flourish.ts";

/** The look the crowns land in. */
const PALETTE: FxPalette = "Hustle";

/** The look its cloak sweeps in. */
const CLOAK_PALETTE: FxPalette = "Dark/blood";

/** Every particle count it throws, the entrance's at intensity "normal", the recipe's at scale 1 (rule 9). */
const TUNING = {
  lift: { count: 12, power: 0.9 },
  crown: { count: 18, power: 1.1 },
  cloak: { count: 18, power: 0.8 },
  tip: { count: 10, power: 0.7 },
} as const;

/** Where its crown sits: the top of its zone. */
const CROWN: FxPoint = { x: 0.5, y: 0.1 };
/** The crown the piles cross under: the middle of the board. */
const CENTER: FxAnchor = { kind: "viewport", at: { x: FX_CENTER.x, y: FX_CENTER.y } };

export const royalCloak: Entrance = ({ D, tgt, intensity: i, view, landing, landMs }) => {
  const at = tid(tgt);
  const land = landMs ?? frac(FX_SLAM_AT, D);
  const cloak = paletteLook(CLOAK_PALETTE) ?? FLAVOR_LOOKS.Dark;
  const cues: FxCue[] = [
    { kind: "sheen", at, delayMs: 0, durationMs: D },
    { ...tunedBurst(i, "sparkle", at, "area", 0, TUNING.cloak), colors: cloak.colors },
    ringCue(D, "gold", at, land),
    tunedBurst(i, "gold", tid(tgt, CROWN), "ring", land, TUNING.crown),
    ...shakeCues(i, ENTRANCE_TRAUMA.royalCloak, land),
  ];
  // He tips his crown to the far side: a glint over the zone across his lane.
  if (view !== undefined && landing !== undefined) {
    cues.push(tunedBurst(i, "sparkle", tid(testid.zone(otherSide(sideOf(view, landing.player)), "units", landing.lane)), "point", land, TUNING.tip));
  }
  return cues;
};

/** #85 King Wagtoggle crowns the two decks as they change hands, and its Radiant Recruit leaps out of its new one. */
export const crownSwap: CardRecipe = (event, p, play) => {
  const spec = flourishSpec(PALETTE, play, p.view, p.env);
  if (!(p.D > 0) || !(spec.scale > 0)) return null;
  const t = timesOf(p.D);
  if (event.type === "swapped" && event.what === "library") {
    const mine = tid(animTestid.library("you"));
    const theirs = tid(animTestid.library("opponent"));
    const cues: FxCue[] = [
      motifBurst(spec, mine, "point", t.G, TUNING.lift),
      motifBurst(spec, theirs, "point", t.G, TUNING.lift),
      motifRing(spec, p.D, CENTER, t.G),
      motifBurst(spec, CENTER, "ring", t.G, TUNING.crown, "gold"),
      { kind: "ghost", from: mine, to: theirs, delayMs: t.T, durationMs: t.L - t.T },
      { kind: "ghost", from: theirs, to: mine, delayMs: t.T, durationMs: t.L - t.T },
    ];
    for (const pile of [mine, theirs]) cues.push(motifRing(spec, p.D, pile, t.L));
    cues.push(...shakeCues(spec.scale, FX_CROWN_SWAP_TRAUMA, t.L));
    return { cues, row: "keep" };
  }
  if (event.type === "summoned" && event.player === play.player && event.instanceId !== play.instanceId && ownWindow(p, play)) {
    const tgt = targetFor(event, p.view);
    if (tgt === null) return null;
    const deck = tid(animTestid.library(sideOf(p.view, play.player)));
    return { cues: [{ kind: "ghost", from: deck, to: tid(tgt), delayMs: t.G, durationMs: t.L - t.G }, motifRing(spec, p.D, tid(tgt), t.L)], row: "keep" };
  }
  return null;
};

/** The gallery's King Wagtoggle: its Cry swaps the decks. */
export const crownSwapFixture: FlourishFixture = () => [{ type: "swapped", what: "library" }];
