// #2793: #97 Zephrys (`core-097`, bespoke "perfectWish"), bespoke because its "perfect" card blows in
// from the far side of the board, which no hand motif says. R3402001: its signature (R3332000), a Mythic
// Spell's: its takeover is a cyclone of sky-blue silk from which a towering wind spirit uncoils, chin on
// fist, card backs orbiting him (takeover look "genie", takeoverArt.ts); through its hold the silk
// gathers at the board's centre in sparkles and frost rings, the spirit's eyes snapping open as the card
// lands. Its Discover's options stay in the caster's prompt; on its `addedToHand` one card back streaks
// on a gust from the other hero into the caster's hand pile, which rings and takes a sheen as it lands,
// in place of the hand's own glint, inside a Spell's budget. It anchors on the heroes, the board and the
// hand pile alone, the same on both seats (R202).

import { animTestid, leadMs } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { ringCue, shakeCues, tid } from "../../build.ts";
import { FX_CENTER, FX_WISH_TRAUMA } from "../../constants.ts";
import { MOTIF_TUNING, motifBurst, motifRing, timesOf } from "../../motifs/build.ts";
import type { CardRecipe } from "../recipe.ts";
import { flourishSpec, isFirstBeat } from "./flourish.ts";

export const perfectWish: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  const spec = flourishSpec(p, play);
  const t = timesOf(p.D);
  if (isFirstBeat(event, play)) {
    const centre = tid(testid.board, FX_CENTER);
    // Through the takeover's hold, before its card lands, so the silk has settled by then; with Big
    // moments off there is no hold, and it settles as the entry lands.
    const hold = Math.max(leadMs(p.entry), t.L);
    return {
      cues: [
        motifBurst(spec, centre, "area", t.G, MOTIF_TUNING.gather, "sparkle"),
        ringCue(p.D, "frost", centre, Math.round(hold / 2)),
        motifRing(spec, p.D, centre, hold),
        motifBurst(spec, centre, "ring", hold, MOTIF_TUNING.small, "frost"),
      ],
      row: "keep",
    };
  }
  if (event.type !== "addedToHand" || event.player !== play.player) return null;
  const side = sideOf(p.view, play.player);
  const far = tid(testid.hero(side === "you" ? "opponent" : "you"));
  const hand = tid(animTestid.hand(side));
  return {
    cues: [
      { kind: "ghost", from: far, to: hand, delayMs: t.G, durationMs: p.D },
      ringCue(p.D, "frost", hand, t.G),
      { kind: "sheen", at: hand, delayMs: t.G, durationMs: p.D },
      ...shakeCues(p.env.intensity, FX_WISH_TRAUMA, t.L),
    ],
    row: "replace",
  };
};
