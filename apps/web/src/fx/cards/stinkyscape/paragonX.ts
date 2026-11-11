// R3395000: Stinkyscape #74 Paragon-X (`stinkyscape-074`, entrance "paragonDive", takeover "whiteDwarf",
// a pale halo's motes on its tile), a Mythic Unit's signature: it has no text but its Tribute and keywords,
// so its signature is its takeover and its entrance. Its takeover is a white dwarf flaring across the dark,
// its black-and-white blade silhouette cutting across it in one stark pass (fx/takeoverArt.ts); then it
// dives from the top of the board into its zone, its halo shield ringing, landing lit for Charge in a burst
// of starlight. It keys on its own landing, never on a card's definition (R202). Moved: its two tributes'
// deaths are paid before its play opens, so no frame keys them to its card, and the Spell sweep its prism
// hull turns aside keys nothing of its own; the halo ring and engine glow on its tile are pale motes.

import { testid } from "../../../game/contract.ts";
import { ringCue, tid, tunedBurst } from "../../build.ts";
import { motifShot } from "../../motifs/build.ts";
import { type Entrance } from "../recipe.ts";
import { entranceFigure, landingSide, leadOf, lookOf, slamAt, slamShake } from "./signatureKit.ts";

/** Its white light. */
const WHITE = "Spirit";

/** Where it dives from: the top of the board above the middle lane (the bottom, for the other seat's). */
const FROM = { you: { x: 0.5, y: -0.05 }, opponent: { x: 0.5, y: 1.05 } } as const;

/** Every particle count it throws, at intensity "normal" (rule 9). */
const TUNING = {
  starlight: { count: 26, power: 1.2 },
} as const;

/** As a share of the entry, when the dive starts. */
const TIMING = { dive: 0.2 } as const;

export const paragonDive: Entrance = (plan) => {
  const look = lookOf(WHITE);
  const at = tid(plan.tgt);
  const slam = slamAt(plan);
  const dive = leadOf(plan) + Math.round(TIMING.dive * (plan.D - leadOf(plan)));
  return [
    motifShot({ look, scale: plan.intensity, row: "replace" }, "bolt", tid(testid.board, FROM[landingSide(plan) ?? "you"]), at, dive, slam - dive),
    entranceFigure("blade", WHITE, plan, true),
    { ...tunedBurst(plan.intensity, "star", at, "area", slam, TUNING.starlight), colors: look.colors },
    ringCue(plan.D, "holy", at, slam),
    ...slamShake(plan, "paragonDive"),
  ];
};
