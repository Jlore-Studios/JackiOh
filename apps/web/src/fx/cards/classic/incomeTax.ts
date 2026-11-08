// R3389001: Classic #9 Income Tax (`classic-009`), a Trap's signature: bespoke "taxSeizure", a cast of its
// own in place of the shared Legendary rays (R3389000). As it fires (its own `trapFired`), an iron
// strongbox lid flies open where it stood, in gold and sparks, a copper chain shoots from its owner's
// hand round the other player's and a crimson seal stamps a ring over it; each card it takes (`stolen`
// from that hand) is yanked across, a card back from hand to hand in place of the steal's own row, its
// face never shown (R202). It wears no aura: a face-down Trap shows nothing (R202) and leaves as it
// fires. Moved: the chain to the firing, so the steals end inside its budget; and the keep its drawer
// defers to their next turn, which opens no play or hook of the Trap's (R1279000), so those cards cross
// under the shared rows.

import { animTestid } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { shakeCues, tid } from "../../build.ts";
import type { FxPalette } from "../../flavors.ts";
import { motifBurst, motifRing, motifShot, otherSide, timesOf } from "../../motifs/build.ts";
import { ownWindow, SIGNATURE_RUN_MAX, SIGNATURE_TRAUMA, stepMs, type CardRecipe } from "../recipe.ts";
import { flourishSpec, type FlourishFixture } from "./flourish.ts";

/** The look the strongbox, the seal and the chain are drawn in. */
const PALETTE: FxPalette = "Hustle/cash";

/** Every particle count it throws, at scale 1 (rule 9). */
const TUNING = {
  lid: { count: 16, power: 1.1 },
  coins: { count: 12, power: 0.9 },
  seal: { count: 10, power: 0.7 },
} as const;

/** #9 Income Tax opens its strongbox as it fires and chains the other hand's cards across. */
export const taxSeizure: CardRecipe = (event, p, play) => {
  const spec = flourishSpec(PALETTE, play, p.view, p.env);
  if (!(p.D > 0) || !(spec.scale > 0) || !ownWindow(p, play)) return null;
  const t = timesOf(p.D);
  const side = sideOf(p.view, play.player);
  const theirs = tid(animTestid.hand(otherSide(side)));
  if (event.type === "trapFired" && event.instanceId === play.instanceId) {
    const box = tid(testid.zone(sideOf(p.view, event.controller), event.row, event.lane));
    const mine = tid(animTestid.hand(side));
    return {
      cues: [
        motifBurst(spec, box, "area", t.G, TUNING.lid, "gold"),
        motifBurst(spec, box, "ring", t.G, TUNING.coins, "spark"),
        motifShot(spec, "beam", mine, theirs, t.T, 0),
        motifRing(spec, p.D, theirs, t.T),
        motifBurst(spec, theirs, "ring", t.T, TUNING.seal, "blood"),
        ...shakeCues(p.env.intensity, SIGNATURE_TRAUMA.taxSeizure, t.G),
      ],
      row: "keep",
    };
  }
  if (event.type === "stolen" && event.zone === "hand" && event.to === play.player) {
    // The entry's steals in order, each one step behind the last, at most SIGNATURE_RUN_MAX of them drawn.
    const k = p.entry.events.filter((candidate) => candidate.type === "stolen" && candidate.zone === "hand" && candidate.to === play.player).indexOf(event);
    if (k < 0 || k >= SIGNATURE_RUN_MAX) return null;
    const mine = tid(animTestid.hand(side));
    const at = stepMs(p.D, t.G, k);
    return { cues: [{ kind: "ghost", from: theirs, to: mine, delayMs: at, durationMs: Math.max(0, t.L - at) }], row: "replace" };
  }
  return null;
};

/** The gallery's Income Tax: it fires from the first backrow zone and takes a card of the other player's. */
export const taxSeizureFixture: FlourishFixture = (view, instanceId, player) => {
  const other = player === view.viewer ? view.opponent.player : view.you.player;
  return [
    { type: "trapFired", instanceId, defId: "classic-009", controller: player, row: "backrow", lane: 1 },
    { type: "stolen", instanceId: "fx-gallery-taxed", defId: "hidden", from: other, to: player, zone: "hand" },
  ];
};
