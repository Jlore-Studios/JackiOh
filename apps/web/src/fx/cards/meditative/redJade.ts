// R3405001: Meditative #39.4 Red Jade (`meditative-039-4`, bespoke "redJade", takeover "mine"), a token's
// signature as the Mythic Spell it prints: its takeover is the mine's red seam (takeoverArt.ts), and as the
// hold ends a red jade shard glints out of it at the board's centre in a spray of red splinters. As the
// caster's Jade Counter rises (`jadeChanged`) the shard drops into the counter by the caster's hero and jade
// light rings it, the counter's own tick and sparkle beside it (systemMoments.ts). At five the Jade Beauty
// it summons lands with its own entrance, which this recipe never claims. It keys on its own play, never on
// a card's definition (R202).

import { sideOf, testid } from "../../../game/contract.ts";
import { ringCue, tid } from "../../build.ts";
import { FX_CENTER } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { motifBurst, motifScale } from "../../motifs/build.ts";
import type { CardRecipe, RecipeFixture } from "../recipe.ts";

/** The sub-look its shard is cut from: the red of the mine's seam. */
const SHARD_PALETTE = "Dark/blood";

/** The red splinters it throws as the hold ends, at scale 1 (rule 9). */
const TUNING = { splinters: { count: 22, power: 1.2 } } as const;

/** The shard glints `tailMs` past the cast's entry and drops into the counter `dropTailMs` past the rise's. */
const TIMING = { tailMs: 400, dropTailMs: 300 } as const;

export const redJade: CardRecipe = (event, p, play) => {
  const look = paletteLook(SHARD_PALETTE) ?? FLAVOR_LOOKS.Dark;
  const tint = figureTint(look);
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const centre = tid(testid.board, FX_CENTER);
    const hold = p.entry.setPiece?.holdMs ?? 0;
    const spec = { look, scale: motifScale(play.defId, p.view, p.env.intensity), row: "replace" } as const;
    return { cues: [figureCue("jadeShard", centre, tint, hold, p.D - hold + TIMING.tailMs), motifBurst(spec, centre, "area", hold, TUNING.splinters, "shard")], row: "replace" };
  }
  if (event.type === "jadeChanged" && event.player === play.player) {
    const side = sideOf(p.view, play.player);
    const hero = tid(testid.hero(side));
    // Mirrored, it drops toward the hero from the board's centre on either seat.
    return { cues: [figureCue("jadeShard", hero, tint, 0, p.D + TIMING.dropTailMs, { side }), ringCue(p.D, "poison", hero, 0)], row: "replace" };
  }
  return null;
};

/**
 * The gallery's Red Jade: its cast alone. Its rise (`jadeChanged`) also plays the counter's own system
 * moment (systemMoments.ts), whose sparkle outlives a Mythic Spell's ceiling after the takeover's entry
 * (R3332000), so the gallery leaves it out.
 */
export const redJadeFixture: RecipeFixture = () => [];
