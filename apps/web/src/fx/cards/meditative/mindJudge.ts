// R2803000: Meditative #22.1 Fortify Mind (`meditative-022-1`, bespoke "mindJudge"): a judged
// guess keeps its sheen and flare on the hero's badge. `glimpse` is folded, and no motif reads a
// judged guess, so the judgment is a recipe of its own over its play's `predicted` and
// `secretRevealed`, in the card's look at the card's scale, keeping the event rows.

import { FLAVOR_FALLBACK } from "../../../cards/flavor.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { shakeCues, tid } from "../../build.ts";
import { FX_MIND_JUDGE_TRAUMA } from "../../constants.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import { flavorLookOf } from "../../flavorSource.ts";
import { MOTIF_TUNING, motifBurst, motifScale, timesOf, type CueSpec } from "../../motifs/build.ts";
import type { CardRecipe } from "../recipe.ts";

/** Fortify Mind's judgment: the guessed player's hero badge, where `glimpse` drew it. */
function badgeOf(p: Parameters<CardRecipe>[1], player: "p1" | "p2"): ReturnType<typeof tid> {
  return tid(testid.hero(sideOf(p.view, player)));
}

/** Fortify Mind plays its judgment at its `predicted` and its `secretRevealed`, over their rows. */
export const mindJudge: CardRecipe = (event, p, play) => {
  if (event.type !== "predicted" && event.type !== "secretRevealed") return null;
  if (!(p.env.intensity > 0) || !(p.D > 0)) return null;
  const t = timesOf(p.D);
  const look = flavorLookOf(play.defId, play.instanceId, p.view) ?? FLAVOR_LOOKS[FLAVOR_FALLBACK];
  const spec: CueSpec = { look, scale: motifScale(play.defId, p.view, p.env.intensity), row: "keep" };
  const at = badgeOf(p, event.player);
  return {
    cues: [
      { kind: "sheen", at, delayMs: t.T, durationMs: t.L - t.T },
      motifBurst(spec, at, "point", t.L, MOTIF_TUNING.small, "arcane"),
      ...shakeCues(spec.scale, FX_MIND_JUDGE_TRAUMA, t.L),
    ],
    row: "keep",
  };
};
