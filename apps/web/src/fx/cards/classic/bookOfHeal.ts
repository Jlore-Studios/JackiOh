// R2794000: Classic #3 Book of Heal (`classic-003`, bespoke "pageMend"). Its pages open in arcane light
// over the caster's hero, a sparkle flies to the card or hero it heals, and the mend lands there in Holy
// light with a ring and a lingering glow. The heal's own number still plays (`keep`); #2793's `mend`
// glows on the healed card alone, so the pages' flight is drawn from the motif primitives.

import { targetFor } from "../../../game/animations.ts";
import { tid } from "../../build.ts";
import { flies, motifBurst, motifRing, timesOf } from "../../motifs/build.ts";
import type { FxPalette } from "../../flavors.ts";
import type { FxCue } from "../../types.ts";
import { heroOf, projectile, type CardRecipe } from "../recipe.ts";
import { flourishSpec, type FlourishFixture } from "./flourish.ts";

/** The look the mend lands in. */
const PALETTE: FxPalette = "Holy";

/** Every particle count the mend throws, at scale 1 (rule 9). */
const TUNING = {
  pages: { count: 12, power: 0.7 },
  mend: { count: 24, power: 1 },
  glow: { count: 10, power: 0.5 },
} as const;

/** The heal the gallery's Book of Heal resolves with: the gallery's own fixture amount. */
const FIXTURE_HEAL = 3;

/** #3 Book of Heal mends what its play heals. */
export const pageMend: CardRecipe = (event, p, play) => {
  if (event.type !== "healed") return null;
  const tgt = targetFor(event, p.view);
  const spec = flourishSpec(PALETTE, play, p.view, p.env);
  if (tgt === null || !(p.D > 0) || !(spec.scale > 0)) return null;
  const t = timesOf(p.D);
  const at = tid(tgt);
  const hero = heroOf(p.view, play);
  const cues: FxCue[] = [motifBurst(spec, hero, "point", t.G, TUNING.pages, "arcane")];
  if (flies(hero, at)) cues.push(projectile(spec.scale, "sparkle", hero, at, t.T, t.L - t.T));
  cues.push(motifRing(spec, p.D, at, t.L), motifBurst(spec, at, "area", t.L, TUNING.mend), motifBurst(spec, at, "ring", t.Li, TUNING.glow, "sparkle"));
  return { cues, row: "keep" };
};

/** The gallery's Book of Heal: it heals its player's first unit. */
export const pageMendFixture: FlourishFixture = (view, _instanceId, player) => {
  const own = player === view.viewer ? view.you : view.opponent;
  const unit = own.units.find((candidate) => candidate !== null);
  return unit === undefined || unit === null ? [] : [{ type: "healed", targetId: unit.instanceId, amount: FIXTURE_HEAL }];
};
