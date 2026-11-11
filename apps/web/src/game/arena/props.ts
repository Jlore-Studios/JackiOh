// The corner props of the playmats (#3348, R3348001): two each, one by each corner, drawn in the
// mat's look (`PropArt.tsx`); R3335001's Fleet track playmat, Stinkyscape, among them. A prop's sound is one of the field taps' recipes (audio RAPID_SFX) with a
// variant and a pitch, so it never spends the game's voices; the playmats' tap families stay the
// tiles' (`SandSurface.tsx`). Public ids only: a mat is a cosmetic (R1834004), never hidden state.

import type { SfxId } from "../../audio/types.ts";
import type { TableId } from "../SandSurface.tsx";

/** A prop's small motion on a plain tap. */
export type PropMove = "wiggle" | "pop" | "hop" | "tilt";
/** A prop's bigger payoff once its taps escalate. */
export type PropPayoff = "flare" | "shake" | "spin" | "leap" | "surface" | "glow";

export type CornerPropDef = {
  /** `<mat>-<name>`, e.g. `tavern-candle`. */
  id: string;
  /** The corner of its half the prop sits by: the playmat's left or right. */
  corner: "left" | "right";
  sfx: SfxId;
  /** The recipe's texture variant, 0 to 3. */
  variant: 0 | 1 | 2 | 3;
  /** The recipe's variation (its pitch), 0 to 1. */
  pitch: number;
  move: PropMove;
  payoff: PropPayoff;
};

const prop = (
  id: string,
  corner: "left" | "right",
  sfx: SfxId,
  variant: 0 | 1 | 2 | 3,
  pitch: number,
  move: PropMove,
  payoff: PropPayoff,
): CornerPropDef => ({ id, corner, sfx, variant, pitch, move, payoff });

/** Each playmat's two props, left then right. */
export const CORNER_PROPS: Readonly<Record<TableId, readonly [CornerPropDef, CornerPropDef]>> = {
  "table-tavern": [
    prop("tavern-candle", "left", "tapParchment", 1, 0.35, "wiggle", "flare"),
    prop("tavern-mug", "right", "tapWood", 2, 0.2, "tilt", "shake"),
  ],
  "table-wood": [
    prop("wood-dice", "left", "tapWood", 0, 0.6, "hop", "spin"),
    prop("wood-coin", "right", "tapStone", 3, 0.8, "pop", "leap"),
  ],
  "table-felt": [
    prop("felt-chips", "left", "tapFelt", 0, 0.3, "pop", "leap"),
    prop("felt-ace", "right", "tapParchment", 2, 0.7, "tilt", "spin"),
  ],
  "table-neon-grid": [
    prop("neon-ghost", "left", "tapNeon", 1, 0.5, "wiggle", "surface"),
    prop("neon-bolt", "right", "tapNeon", 3, 0.9, "pop", "flare"),
  ],
  "table-marble": [
    prop("marble-urn", "left", "tapStone", 0, 0.2, "tilt", "shake"),
    prop("marble-laurel", "right", "tapFelt", 2, 0.6, "wiggle", "glow"),
  ],
  "table-chessboard": [
    prop("chess-knight", "left", "tapWood", 1, 0.5, "hop", "leap"),
    prop("chess-clock", "right", "tapNeon", 0, 0.7, "tilt", "shake"),
  ],
  "table-desert-sand": [
    prop("desert-cactus", "left", "sand", 0, 0.4, "wiggle", "glow"),
    prop("desert-skull", "right", "sand", 2, 0.15, "hop", "surface"),
  ],
  "table-obsidian-slab": [
    prop("obsidian-crystal", "left", "tapStone", 1, 0.95, "pop", "glow"),
    prop("obsidian-rune", "right", "tapAstral", 3, 0.5, "wiggle", "spin"),
  ],
  "table-ancient-map": [
    prop("map-compass", "left", "tapParchment", 0, 0.5, "wiggle", "spin"),
    prop("map-serpent", "right", "tapFelt", 3, 0.25, "hop", "surface"),
  ],
  "table-astral": [
    prop("astral-comet", "left", "tapAstral", 0, 0.8, "hop", "leap"),
    prop("astral-satellite", "right", "tapNeon", 2, 0.4, "tilt", "spin"),
  ],
  "table-stinkyscape": [
    prop("stinkyscape-rocket", "left", "tapNeon", 1, 0.65, "hop", "flare"),
    prop("stinkyscape-planet", "right", "tapAstral", 3, 0.3, "tilt", "spin"),
  ],
};

/** The props of `mat`; a mat with none (an unknown id) has none. */
export function propsOf(mat: string): readonly CornerPropDef[] {
  return Object.hasOwn(CORNER_PROPS, mat) ? CORNER_PROPS[mat as TableId] : [];
}

/**
 * Four playmats hide a tap sequence: its prop ids, in order. Tap the props in this order, each within
 * CORNER_SEQUENCE_GAP_MS of the last, for a special moment (right, left, right).
 */
export const CORNER_SEQUENCES: Readonly<Partial<Record<TableId, readonly string[]>>> = {
  "table-tavern": ["tavern-mug", "tavern-candle", "tavern-mug"],
  "table-chessboard": ["chess-clock", "chess-knight", "chess-clock"],
  "table-ancient-map": ["map-serpent", "map-compass", "map-serpent"],
  "table-astral": ["astral-satellite", "astral-comet", "astral-satellite"],
};

/** The hidden sequence of `mat`, or none. */
export function sequenceOf(mat: string): readonly string[] | undefined {
  return Object.hasOwn(CORNER_SEQUENCES, mat) ? CORNER_SEQUENCES[mat as TableId] : undefined;
}
