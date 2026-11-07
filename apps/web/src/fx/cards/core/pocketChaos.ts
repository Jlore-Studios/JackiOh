// R3402001: #87 Pocket Chaos (`core-087`, bespoke "pocketSwap"), a Legendary Spell's signature (R3332000).
// At its cast a velvet pocket pops open over the board's centre in a prismatic ring and a tumbling cloud
// of dice-bright confetti, both heroes' light tugged into it. On its `swapped` the chosen pair trades
// places through the pocket: the Deck piles' and the boards' as card backs crossing (the boards' through
// the middle lanes), the heroes' health as rings and a sheen on both; and the card it hands the other
// player drops into their hand pile as a card back. A Spell's budget leaves no room for particles past
// its cast, so the swap and the drop draw without them, the drop in place of the hand's own glint. It
// anchors on the heroes, the piles, the zones and the board, the same on both seats (R202).

import { animTestid } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_CENTER, FX_POCKET_TRAUMA } from "../../constants.ts";
import { MOTIF_TUNING, motifBurst, motifRing, motifShot, timesOf } from "../../motifs/build.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import type { CardRecipe } from "../recipe.ts";
import { flourishSpec, isFirstBeat } from "./flourish.ts";

/** Every particle count the pocket throws, at intensity "normal" (rule 9). */
const TUNING = {
  dice: { count: 30, power: 1.2 },
} as const;

/** The lane whose zones the boards trade through: the middle one. */
const MIDDLE_LANE = 3;

/** The two ends of each swap, the caster's first. */
function swapEnds(what: "health" | "board" | "library"): [FxAnchor, FxAnchor] {
  if (what === "health") return [tid(testid.hero("you")), tid(testid.hero("opponent"))];
  if (what === "board") return [tid(testid.zone("you", "units", MIDDLE_LANE)), tid(testid.zone("opponent", "units", MIDDLE_LANE))];
  return [tid(animTestid.library("you")), tid(animTestid.library("opponent"))];
}

export const pocketSwap: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  const spec = flourishSpec(p, play);
  const t = timesOf(p.D);
  const centre = tid(testid.board, FX_CENTER);
  if (isFirstBeat(event, play)) {
    const heroes = [tid(testid.hero("you")), tid(testid.hero("opponent"))];
    return {
      cues: [
        motifRing(spec, p.D, centre, t.G),
        ...heroes.map((hero) => motifShot(spec, "bolt", hero, centre, t.G, t.L - t.G)),
        { ...tunedBurst(p.env.intensity, "confetti", centre, "area", t.L, TUNING.dice), colors: spec.look.colors },
        motifBurst(spec, centre, "ring", t.L, MOTIF_TUNING.small, "prismatic"),
        ...shakeCues(p.env.intensity, FX_POCKET_TRAUMA, t.L),
      ],
      row: "keep",
    };
  }
  if (event.type === "swapped") {
    const [a, b] = swapEnds(event.what);
    const cues: FxCue[] = [ringCue(p.D, "prismatic", a, t.G), ringCue(p.D, "prismatic", b, t.G), ...shakeCues(p.env.intensity, FX_POCKET_TRAUMA, t.T)];
    if (event.what === "health") {
      cues.push({ kind: "sheen", at: a, delayMs: t.G, durationMs: t.L }, { kind: "sheen", at: b, delayMs: t.G, durationMs: t.L });
    } else {
      cues.push({ kind: "ghost", from: a, to: b, delayMs: t.G, durationMs: p.D }, { kind: "ghost", from: b, to: a, delayMs: t.G, durationMs: p.D });
    }
    return { cues, row: "keep" };
  }
  if (event.type === "addedToHand" && event.player !== play.player) {
    const hand = tid(animTestid.hand(sideOf(p.view, event.player)));
    return { cues: [{ kind: "ghost", from: centre, to: hand, delayMs: t.G, durationMs: p.D }], row: "replace" };
  }
  return null;
};
