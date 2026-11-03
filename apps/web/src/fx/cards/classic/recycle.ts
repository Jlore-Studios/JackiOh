// R2794000: Classic #30 Recycle (`classic-030`, bespoke "recycleLoop"). Its caster's graveyard lifts in
// Nature's leaves, a card back loops from the graveyard pile into the Deck pile, and a ring of leaves
// closes over the deck, once a play: on the entry that holds the play's first `shuffledIn`. The shuffles'
// own rows still play (`keep`); a shuffle of a whole pile is a zone entry, which plays its one wave.

import { animTestid } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { tid } from "../../build.ts";
import type { FxPalette } from "../../flavors.ts";
import { isFirstEntry, motifBurst, motifRing, timesOf } from "../../motifs/build.ts";
import type { CardRecipe } from "../recipe.ts";
import { flourishSpec, type FlourishFixture } from "./flourish.ts";

/** The look the loop turns in. */
const PALETTE: FxPalette = "Nature";

/** Every particle count the loop throws, at scale 1 (rule 9). */
const TUNING = {
  lift: { count: 12, power: 0.8 },
  loop: { count: 20, power: 1 },
} as const;

/** The gallery's Recycle shuffles this many cards in (two: fewer than its deck, so no zone entry), then draws one. */
const FIXTURE_SHUFFLED = 2;

const SHUFFLED_IN = ["shuffledIn"] as const;

/** #30 Recycle loops its caster's graveyard into the deck. */
export const recycleLoop: CardRecipe = (event, p, play) => {
  if (event.type !== "shuffledIn" || event.player !== play.player) return null;
  // Once a play, on the first of its shuffles, however many the entry holds.
  const first = p.entry.events.find((candidate) => candidate.type === "shuffledIn" && candidate.player === play.player);
  if (first !== event || !isFirstEntry(p.env, p.entry, SHUFFLED_IN)) return null;
  const spec = flourishSpec(PALETTE, play, p.view, p.env);
  if (!(p.D > 0) || !(spec.scale > 0)) return null;
  const t = timesOf(p.D);
  const side = sideOf(p.view, play.player);
  const grave = tid(animTestid.graveyard(side));
  const deck = tid(animTestid.library(side));
  return {
    cues: [
      motifBurst(spec, grave, "area", t.G, TUNING.lift),
      { kind: "ghost", from: grave, to: deck, delayMs: t.T, durationMs: t.L - t.T },
      motifRing(spec, p.D, deck, t.L),
      motifBurst(spec, deck, "ring", t.L, TUNING.loop, "leaf"),
    ],
    row: "keep",
  };
};

/** The gallery's Recycle: two cards shuffled into its player's deck, then a draw. */
export const recycleLoopFixture: FlourishFixture = (_view, instanceId, player) => [
  ...Array.from({ length: FIXTURE_SHUFFLED }, (_, k) => ({ type: "shuffledIn" as const, player, instanceId: `${instanceId}-shuffled-${String(k)}`, defId: "core-003", position: 0 })),
  { type: "drawn", player, instanceId: `${instanceId}-drawn`, defId: "core-003" },
];
