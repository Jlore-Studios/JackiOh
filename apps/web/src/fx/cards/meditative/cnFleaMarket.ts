// R3405001: Meditative #42 CN Flea Market (`meditative-042`, bespoke "fleaMarket"), a Legendary Spell's
// signature in Hustle's cash look: as it is cast red lanterns pop alight in a wave along the board's top
// edge and a striped stall awning unfurls over the caster's backrow, food-cart steam curling up from it.
// The stall and its yuan are the caster's prompt alone, so nothing more plays on the other seat; each lot
// bought drops a card back from the stall into the caster's hand pile, its face's sheen the caster's alone,
// and a Radiant barter carries the hand card it exiles, in public, from the hand pile to the stall as a
// card back. It keys on its own play and what that play adds or exiles, never on a card's definition, and
// draws card backs alone (R202).

import { animTestid } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, tid } from "../../build.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { motifBurst, motifScale } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import type { CardRecipe, RecipeFixture } from "../recipe.ts";

/** The sub-look its market trades in: Hustle's cash. */
const MARKET_PALETTE = "Hustle/cash";

/** The food-cart steam it curls up, at scale 1 (rule 9). */
const TUNING = { steam: { count: 16, power: 0.5 } } as const;

/** As fractions of the cast: the awning unfurls from `awningAt`, the steam rises at `steamAt`; both figures last `tailMs` past it. */
const TIMING = { tailMs: 400, awningAt: 0.15, steamAt: 0.45 } as const;

/** The gallery's two lots bought. */
const FIXTURE_LOTS = ["core-055", "core-002"] as const;

export const fleaMarket: CardRecipe = (event, p, play) => {
  const look = paletteLook(MARKET_PALETTE) ?? FLAVOR_LOOKS.Hustle;
  const side = sideOf(p.view, play.player);
  const stall = tid(animTestid.backrow(side));
  const hand = tid(animTestid.hand(side));
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const tint = figureTint(look);
    const unfurl = frac(TIMING.awningAt, p.D);
    const spec = { look, scale: motifScale(play.defId, p.view, p.env.intensity), row: "replace" } as const;
    return {
      cues: [
        figureCue("lanterns", tid(testid.board), tint, 0, p.D + TIMING.tailMs),
        figureCue("awning", stall, tint, unfurl, p.D - unfurl + TIMING.tailMs, { side }),
        motifBurst(spec, stall, "area", frac(TIMING.steamAt, p.D), TUNING.steam, "smoke"),
      ],
      row: "replace",
    };
  }
  if (event.type === "addedToHand" && event.player === play.player) {
    const cues: FxCue[] = [{ kind: "ghost", from: stall, to: hand, delayMs: 0, durationMs: p.D }];
    if (p.view.viewer === play.player) cues.push({ kind: "sheen", at: hand, delayMs: 0, durationMs: p.D });
    return { cues, row: "replace" };
  }
  // R1001: the Radiant face's barter, a hand card exiled in public for its price.
  if (event.type === "exiled" && event.owner === play.player && event.instanceId !== play.instanceId) {
    return { cues: [{ kind: "ghost", from: hand, to: stall, delayMs: 0, durationMs: p.D }], row: "replace" };
  }
  return null;
};

/** The gallery's CN Flea Market: two lots bought into the caster's hand. */
export const fleaMarketFixture: RecipeFixture = (_view, _instanceId, player) =>
  FIXTURE_LOTS.map((defId, k) => ({ type: "addedToHand", player, instanceId: `fx-lot-${String(k + 1)}`, defId }));
