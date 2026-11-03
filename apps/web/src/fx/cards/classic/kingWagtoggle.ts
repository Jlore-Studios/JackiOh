// R2794000: Classic #85 King Wagtoggle (`classic-085`, bespoke "crownSwap"). As its Cry swaps the two
// decks, each Deck pile lifts in Hustle's light, two card backs cross the board to the other pile, and a
// gold crown of sparks lands on each with a ring and a shake. The swap's own row still plays (`keep`).
// Its Legendary entrance and its music stay the shared ones (R2722000, R2722001).

import { animTestid } from "../../../game/animations.ts";
import { shakeCues, tid } from "../../build.ts";
import { FX_CROWN_SWAP_TRAUMA } from "../../constants.ts";
import type { FxPalette } from "../../flavors.ts";
import { motifBurst, motifRing, timesOf } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";
import { flourishSpec, type FlourishFixture } from "./flourish.ts";

/** The look the crowns land in. */
const PALETTE: FxPalette = "Hustle";

/** Every particle count the swap throws, at scale 1 (rule 9). */
const TUNING = {
  lift: { count: 14, power: 0.9 },
  crown: { count: 22, power: 1.1 },
} as const;

/** #85 King Wagtoggle crowns the two decks as they change hands. */
export const crownSwap: CardRecipe = (event, p, play) => {
  if (event.type !== "swapped" || event.what !== "library") return null;
  const spec = flourishSpec(PALETTE, play, p.view, p.env);
  if (!(p.D > 0) || !(spec.scale > 0)) return null;
  const t = timesOf(p.D);
  const mine = tid(animTestid.library("you"));
  const theirs = tid(animTestid.library("opponent"));
  const cues: FxCue[] = [
    motifBurst(spec, mine, "point", t.G, TUNING.lift),
    motifBurst(spec, theirs, "point", t.G, TUNING.lift),
    { kind: "ghost", from: mine, to: theirs, delayMs: t.T, durationMs: t.L - t.T },
    { kind: "ghost", from: theirs, to: mine, delayMs: t.T, durationMs: t.L - t.T },
  ];
  for (const pile of [mine, theirs]) cues.push(motifRing(spec, p.D, pile, t.L), motifBurst(spec, pile, "ring", t.L, TUNING.crown, "gold"));
  cues.push(...shakeCues(spec.scale, FX_CROWN_SWAP_TRAUMA, t.L));
  return { cues, row: "keep" };
};

/** The gallery's King Wagtoggle: its Cry swaps the decks. */
export const crownSwapFixture: FlourishFixture = () => [{ type: "swapped", what: "library" }];
