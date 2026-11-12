// R3350000 (#3350): a Mythic's diorama, a small model of its picture cut in depth slices. Each slice holds the pieces of the
// painter's still picture its `pick` names (pieces.ts `matches`), stands at a depth (0 the farthest, 1 the nearest) in one of
// the depth planes (R3344000), may bend about one axis to read as a curved surface, and may run an idle loop of its own,
// written for the card. Presentation only (CLAUDE.md rule 7).

import type { ArtBeat } from "../motion.ts";
import type { ArtPlane } from "../pictures.ts";

/** R3350000: a slice bent about one axis by `deg` degrees (DIORAMA_BEND_MAX_DEG either way), so it reads as curved. */
export type DioramaBend = { readonly axis: "x" | "y"; readonly deg: number };

/**
 * R3350000: a slice's idle loop: a move of motion.ts's (`ART_MOVES`) on its own beat; a move measured in the picture's box
 * turns about `origin` (the picture's units, its middle by default); `stagger` starts each of the slice's pieces a step
 * further along its loop, so they move one after another.
 */
export type DioramaLoop = ArtBeat & { readonly origin?: readonly [number, number]; readonly stagger?: true };

/**
 * R3350000: one depth slice: its name, the plane it parallaxes with, its depth (0 to 1), the pieces it holds (`pick`, pieces.ts
 * keys and patterns), whether it holds every piece no other slice picks (`rest`, the farthest slice), and its bend and loop.
 */
export type DioramaSlice = {
  readonly name: string;
  readonly plane: ArtPlane;
  readonly depth: number;
  readonly pick: readonly string[];
  readonly rest?: true;
  readonly bend?: DioramaBend;
  readonly loop?: DioramaLoop;
};

/** R3350000: a Mythic's diorama: its card's id and its slices, back to front. */
export type Diorama = { readonly id: string; readonly slices: readonly DioramaSlice[] };
