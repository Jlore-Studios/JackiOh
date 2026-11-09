// R3395000: Stinkyscape #44 Great Paperclip Theory (`stinkyscape-044`, bespoke "paperclipLink", takeover
// "wire"), a Mythic Spell's signature. Its takeover is one silver wire filling the screen, bending into a
// staircase of drone cores that grows from desk size to moon size and snaps back to one
// (fx/takeoverArt.ts); as the hold ends silver sparks fly at the board's centre and the board shakes. The
// Drone Core I it adds flies to the caster's hand pile as a card back while a wire link clips onto the
// caster's hero, and its rest-of-game rule settles on the hero in a sheen. It keys on its own play, never
// on the added card's definition, and flies only a card back (R202). Moved: each later tier's link and
// card come in later plays, which key nothing of this one's.

import { testid } from "../../../game/contract.ts";
import { shakeCues, tid } from "../../build.ts";
import { FX_CENTER } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { motifBurst, motifScale } from "../../motifs/build.ts";
import type { CardRecipe } from "../recipe.ts";
import { cardBack, handOf, heroAt, lookOf, replace } from "./signatureKit.ts";

/** Its silver wire. */
const WIRE = "Physical/steel";

/** The sparks it throws as the hold ends, at intensity "normal" and scale 1 (rule 9). */
const TUNING = { sparks: { count: 28, power: 1.2 } } as const;

/** Its shake as the hold ends (a Spell's at most 1). */
const PAPERCLIP_TRAUMA = 0.55;

/** As a share of the entry, when the added card's back leaves the link. */
const TIMING = { backAt: 0.2 } as const;

export const paperclipLink: CardRecipe = (event, p, play) => {
  const look = lookOf(WIRE);
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const hold = p.entry.setPiece?.holdMs ?? 0;
    const spec = { look, scale: motifScale(play.defId, p.view, p.env.intensity), row: "replace" } as const;
    return replace([motifBurst(spec, tid(testid.board, FX_CENTER), "area", hold, TUNING.sparks, "spark"), ...shakeCues(p.env.intensity, PAPERCLIP_TRAUMA, hold)]);
  }
  if (event.type === "addedToHand" && event.player === play.player) {
    const hero = heroAt(p.view, play.player);
    const leave = Math.round(TIMING.backAt * p.D);
    return replace([figureCue("paperclip", hero, figureTint(look), 0, p.D), cardBack(hero, handOf(p.view, event.player), leave, p.D)]);
  }
  if (event.type === "modifierChanged" && event.added && event.player === play.player) {
    return replace([{ kind: "sheen", at: heroAt(p.view, play.player), delayMs: 0, durationMs: p.D }]);
  }
  return null;
};
