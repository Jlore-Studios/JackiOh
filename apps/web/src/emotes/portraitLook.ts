// The six portraits' colours and the places of their drifting motes (R1332). Every portrait is
// drawn the same on the board, in its inspect view and in the deck builder's picker, so the look
// is a table of colours and the motes a pure function of the portrait's id: no clock, no random.
// The CSS that uses them is `portrait.css`; the component that sets them is `PortraitArt.tsx`.

import type { PortraitId } from "@jackioh/shared";
import { PORTRAIT_IDS } from "@jackioh/shared";

import { hashId, round2, seededRandom } from "../cards/art/hash.ts";
import {
  PORTRAIT_MOTE_COUNT,
  PORTRAIT_MOTE_DRIFT_MS,
  PORTRAIT_MOTE_EDGE_PCT,
  PORTRAIT_MOTE_MAX_PCT,
  PORTRAIT_MOTE_MIN_PCT,
} from "./config.ts";

/** One portrait's colours: the ground lit from its upper left, the light, the rim and the motes. */
export type PortraitLook = {
  /** The oval's own gradient background, light at the upper left to dark at the foot. */
  ground: readonly [string, string];
  /** The light that washes over the picture and breathes. */
  light: string;
  /** The rim light along the oval's lit edge. */
  rim: string;
  /** The drifting motes. */
  mote: string;
};

/** A saturated look for each portrait, so the four Human portraits differ as well as the others. */
export const PORTRAIT_LOOKS: Readonly<Record<PortraitId, PortraitLook>> = {
  vanilla: { ground: ["#ffe3a3", "#7a4a12"], light: "#fff4d6", rim: "#ffd98a", mote: "#fffbe8" },
  gary: { ground: ["#3ddc84", "#06331b"], light: "#ffe27a", rim: "#ffd447", mote: "#fff1a8" },
  timmy: { ground: ["#ff8a3d", "#5a0f2a"], light: "#fff36b", rim: "#ffe14d", mote: "#fffbc2" },
  dfender: { ground: ["#5aa8ff", "#0b1f4d"], light: "#d6ecff", rim: "#9fd4ff", mote: "#e8f5ff" },
  felinors: { ground: ["#ff6fcf", "#3a0a3f"], light: "#ffd1f0", rim: "#ff9be0", mote: "#ffe6f7" },
  shredder: { ground: ["#ff4d4d", "#1f0a0a"], light: "#ffb08a", rim: "#ff7a59", mote: "#ffd0bd" },
};

/** One mote: where it starts and how big, in percent of the oval, and how far into its drift. */
export type PortraitMote = { x: number; y: number; size: number; delayMs: number };

/** The oval's width and height are each this many percent of themselves. */
const OVAL_PCT = 100;

/** The motes of one portrait, seeded from its id. */
export function portraitMotes(portrait: PortraitId): readonly PortraitMote[] {
  const next = seededRandom(hashId(`portrait-motes:${portrait}`));
  const span = OVAL_PCT - PORTRAIT_MOTE_EDGE_PCT - PORTRAIT_MOTE_EDGE_PCT;
  return Array.from({ length: PORTRAIT_MOTE_COUNT }, () => ({
    x: round2(PORTRAIT_MOTE_EDGE_PCT + next() * span),
    y: round2(PORTRAIT_MOTE_EDGE_PCT + next() * span),
    size: round2(PORTRAIT_MOTE_MIN_PCT + next() * (PORTRAIT_MOTE_MAX_PCT - PORTRAIT_MOTE_MIN_PCT)),
    delayMs: Math.round(next() * PORTRAIT_MOTE_DRIFT_MS),
  }));
}

/** Every portrait's motes, built once. */
export const PORTRAIT_MOTES: Readonly<Record<PortraitId, readonly PortraitMote[]>> =
  Object.fromEntries(PORTRAIT_IDS.map((id) => [id, portraitMotes(id)])) as Record<
    PortraitId,
    readonly PortraitMote[]
  >;
