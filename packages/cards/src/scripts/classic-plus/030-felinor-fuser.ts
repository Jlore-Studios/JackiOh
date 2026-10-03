// C+ #30 Felinor Fuser (SPEC §8.7 row 30; §6.3 Discover and Fuse, R77, R102, R179, R352, R380,
// R387, R405). (3) Unit, Felinor, Epic, 3/3 → 6/6.
//   Base:    "Cry: Discover a Felinor Unit, then another. Fuse both into this."
//   Radiant: "Cry: Discover a Radiant Felinor Unit, then another. Fuse both into this."
//
// Two chained Discovers, the shape #98 Heroic Power's Stitching makes them (R352): the first answer
// re-enters this script with the pick carried in the second prompt's data, and the second answer
// fuses both picks onto this unit. Each Discover offers three different non-token Felinor-tagged
// Units (R405: "Felinors" are the creatures) of every set (R380), never Felinor Fuser itself, which
// `discoverFromCatalog` leaves out by id — every ingredient's id on a fused Fuser (R387).
//
// The Fuse is R77 with this unit as the target on the field (`fuseCards`): it keeps its instance,
// zone, damage, position and radiant flag, sums the stats, unions the keywords, joins the texts, costs
// min(sum, 4), and its id names the three (R179); the Discovered cards were never cards on a board
// and cease to exist, so their Cries never run. On the Radiant face the Discovered cards are Radiant:
// each goes in on its Radiant face, lending it to both of the fusion's forms (§6.3 Fuse), and the
// kept Radiant instance reads the Radiant form (R77). A Discover with no pool fizzles, and fewer than
// two picks fuse nothing (R352, R77).

import type { EffectContext, Script } from "@jackioh/engine";
import { chosenOptions, discoverFromCatalog, fuseCards } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-030");

/** R352's two Discovers: "Discover a Felinor Unit, then another". */
const PICKS = 2;

/** The resume step each Discover's answer re-enters (§10.6). */
const PICKED = "picked";

/** The data key the picks so far travel in, from the first prompt to the second. */
const PICKS_KEY = "picks";

/** R405, R380: a Felinor-tagged Unit of any set; `query` leaves tokens out (§5.1). */
const FELINOR_UNITS = { tags: ["Felinor" as const], type: "Unit" as const };

/** The picks a paused Cry has made so far, read back out of the prompt's data. */
function picksSoFar(ctx: EffectContext): string[] {
  const stored: unknown = ctx.data[PICKS_KEY];
  return Array.isArray(stored) ? stored.filter((pick): pick is string => typeof pick === "string") : [];
}

function felinorFuser(radiant: boolean): Script {
  const discover = (picks: readonly string[]) =>
    discoverFromCatalog({
      step: PICKED,
      query: FELINOR_UNITS,
      prompt: radiant ? "Discover a Radiant Felinor Unit" : "Discover a Felinor Unit",
      data: { [PICKS_KEY]: [...picks] },
    });
  return {
    cry: () => [discover([])],
    resume: {
      [PICKED]: (ctx) => {
        const picked = chosenOptions(ctx)[0];
        // A Discover is answered with one of its options (§10.6); an answer naming none adds nothing.
        if (picked === undefined) return [];
        const picks = [...picksSoFar(ctx), picked];
        if (picks.length < PICKS) return [discover(picks)];
        // The unit fused onto is this one, while it still stands on the field (R77).
        if (ctx.self === null) return [];
        return [fuseCards({ defIds: picks, targetInstanceId: ctx.self.id, ...(radiant ? { radiantIngredients: true } : {}) })];
      },
    },
  };
}

export const base: Script = felinorFuser(false);

export const radiant: Script = felinorFuser(true);
