// R2797000: Stinkyscape #4 Olivian Expanse (`stinkyscape-004`, bespoke "terminusRift"), the Mythic that
// Discovers a Terminus card: a void rift tears open over the board's centre, and the card it gives up
// flies out of it into its caster's hand. It draws at the play's first card added, once a play; the
// added card's own row still plays (`keep`), and a play that adds nothing draws nothing.

import { animTestid } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { ringCue, shakeCues, tid } from "../../build.ts";
import { FX_TERMINUS_TRAUMA } from "../../constants.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import { flavorLookOf } from "../../flavorSource.ts";
import { motifBurst, motifScale, timesOf, type CueSpec } from "../../motifs/build.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the rift throws at scale 1 (rule 9). */
const TUNING = {
  rift: { count: 30, power: 1.2 },
  arrive: { count: 12, power: 0.9 },
} as const;

/** R2797000: Olivian Expanse's rift, at the play's first `addedToHand`. */
export const terminusRift: CardRecipe = (event, p, play) => {
  if (event.type !== "addedToHand") return null;
  const own = p.entry.events.filter((candidate) => candidate.type === "addedToHand");
  if (own[0] !== event || play.seen.addedToHand !== own.length) return null;
  const look = flavorLookOf(play.defId, play.instanceId, p.view) ?? FLAVOR_LOOKS.Nerd;
  const spec: CueSpec = { look, scale: motifScale(play.defId, p.view, p.env.intensity), row: "keep" };
  const t = timesOf(p.D);
  const centre = tid(testid.board);
  const hand = tid(animTestid.hand(sideOf(p.view, event.player)));
  return {
    cues: [
      ringCue(t.D, "void", centre, t.G),
      motifBurst(spec, centre, "area", t.G, TUNING.rift, "void"),
      { kind: "ghost", from: centre, to: hand, delayMs: t.T, durationMs: t.L - t.T },
      motifBurst(spec, hand, "point", t.L, TUNING.arrive, "sparkle"),
      ringCue(t.D, "void", centre, t.Li),
      ...shakeCues(spec.scale, FX_TERMINUS_TRAUMA, t.L),
    ],
    row: "keep",
  };
};
