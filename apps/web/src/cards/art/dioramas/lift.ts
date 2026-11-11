// #3674 (R3351000, R3350000): a board Mythic's diorama slices, lifted. A slice's loop and its role layer move inside its picture
// (index.ts `dioramaSliceSvg`), so a phone restyles and repaints every moving slice of every moving face each frame. Where a
// slice's motion is one chain of loops around all it draws (its `<g class="dio">`, a role's `<g class="art-m-…">` inside it, or
// either alone) and each loop moves it only by a move whose origin the page can name (`LIFTABLE_MOVES`), the window draws the
// slice's still picture (the slice less its motion style, byte for byte) on HTML layers, one a loop, outermost first, that
// artLive.css moves in transform and opacity alone on the compositor (CardArt `liftChain`). The picture is 100 units square and
// drawn as `cover` draws it, so a picture unit is a percent of a layer. A staggered piece, a second animated group, a particle,
// a Stinkyscape subject in its placing transform and a move about a piece's own box (a spin, a flare) stay in the picture.
// Presentation only (CLAUDE.md rule 7): a slice is drawn from its card's id and face alone (R202).

import type { ArtMotion, ArtMove } from "../motion.ts";
import { ART_BOX } from "../procedural.ts";
import { svgDataUri } from "../svg.ts";
import { dioramaSliceSvg, dioramaSliceUri } from "./index.ts";
import { DIORAMA_CACHE_MAX } from "./pieces.ts";

/**
 * #3674: the moves a lift runs: the picture-box moves, which turn about the point their rule names, and the moves about a
 * piece's own box whose keyframes ignore their origin (an opacity, a translation).
 */
export const LIFTABLE_MOVES: ReadonlySet<ArtMove> = new Set<ArtMove>(["bob", "breathe", "sway", "hop", "drift", "swell", "pulse", "float", "rise", "fall"]);

/** #3674: one loop of a lift: its move, its seconds and the point of the picture it turns about (null: a move that ignores it). */
export type LiftMove = { readonly move: ArtMove; readonly seconds: number; readonly origin: readonly [number, number] | null };

/** #3674: a lifted slice: its still picture's data URI and its loops, outermost first. */
export type SliceLift = { readonly uri: string; readonly moves: readonly LiftMove[] };

/** A moving picture's style: a rule and keyframes inside the reduced-motion gate (motion.ts `motionStyle`, index.ts's loop). */
export const MOTION_STYLE = /<style>@media \(prefers-reduced-motion: no-preference\)\{[\s\S]*?<\/style>/g;

/** What opens a picture a lift can draw: 100 units square, so its units are a layer's percent. */
const SQUARE_PICTURE = `<svg viewBox="0 0 ${String(ART_BOX)} ${String(ART_BOX)}"`;
/** What ends a picture's definitions, and the picture. */
const DEFS_END = "</defs>";
const SVG_END = "</svg>";
/** A loop's group, exactly: its class and nothing else, so a staggered piece's (`style=` delay) is none. */
const LOOP_OPEN = /^<g class="(dio|art-m-[a-z]+)">/;
/** A group's opening (or one closing itself) and its closing. */
const GROUP_TAG = /<g\b[^>]*>|<\/g>/g;
/** A loop's group anywhere in what a chain holds: a second animated group. */
const ANY_LOOP = /<g class="(dio|art-m-)/;
/** A picture-box origin as the style writes it. */
const VIEW_ORIGIN = /^(-?[0-9.]+)px (-?[0-9.]+)px$/;

/** `s` when it is one loop's group and nothing else: its class and what it holds; null otherwise. */
function wrapper(s: string): { cls: string; inner: string } | null {
  const open = LOOP_OPEN.exec(s);
  if (open === null) return null;
  let depth = 0;
  for (const match of s.matchAll(GROUP_TAG)) {
    const tag = match[0];
    if (tag === "</g>") depth -= 1;
    else if (!tag.endsWith("/>")) depth += 1;
    if (depth === 0) return match.index + tag.length === s.length ? { cls: open[1] ?? "", inner: s.slice(open[0].length, match.index) } : null;
  }
  return null;
}

/** The loop `styles` gives the class `cls`, as `roleRule` writes it; null when it has none or it is no lift's. */
function loopOf(cls: string, styles: string): LiftMove | null {
  const rule = new RegExp(String.raw`\.${cls}\{transform-box:(view-box|fill-box);transform-origin:([^;]+);animation:art-([a-z]+) ([0-9.]+)s `).exec(styles);
  if (rule === null) return null;
  const [, box, origin = "", name = "", seconds = ""] = rule;
  const move = name as ArtMove;
  if (!LIFTABLE_MOVES.has(move)) return null;
  if (box === "fill-box") return { move, seconds: Number(seconds), origin: null };
  const point = VIEW_ORIGIN.exec(origin);
  if (point === null) return null;
  return { move, seconds: Number(seconds), origin: [Number(point[1]), Number(point[2])] };
}

/**
 * #3674: `svg`, a slice's picture, lifted: its still picture (`svg` less its motion style) and the loops of the chain around all
 * it draws, outermost first; null when its motion is not one such chain of `LIFTABLE_MOVES`, or it has none.
 */
export function liftSvg(svg: string): { svg: string; moves: LiftMove[] } | null {
  const styles = svg.match(MOTION_STYLE)?.join("") ?? "";
  if (styles === "" || !svg.startsWith(SQUARE_PICTURE)) return null;
  const still = svg.replace(MOTION_STYLE, "");
  const cut = still.indexOf(DEFS_END);
  if (cut < 0 || !still.endsWith(SVG_END)) return null;
  const body = still.slice(cut + DEFS_END.length, still.length - SVG_END.length);
  const outer = wrapper(body);
  if (outer === null) return null;
  const inner = wrapper(outer.inner);
  const chain = inner === null ? [outer] : [outer, inner];
  if (ANY_LOOP.test(chain[chain.length - 1]?.inner ?? "")) return null;
  const moves: LiftMove[] = [];
  for (const link of chain) {
    const move = loopOf(link.cls, styles);
    if (move === null) return null;
    moves.push(move);
  }
  return { svg: still, moves };
}

const liftCache = new Map<string, SliceLift | null>();

/**
 * #3674: slice `index` of `defId`'s diorama (index.ts `dioramaSliceSvg`'s arguments), lifted when it can be (`liftSvg`): its
 * still picture as a data URI and its loops; null when it cannot, or the card has no such slice. Memoised as
 * `dioramaSliceUri` is.
 */
export function liftedSlice(defId: string, radiant: boolean, index: number, moving: boolean, motion: ArtMotion | null = null): SliceLift | null {
  const key = `${defId}|${radiant}|${index}|${moving}|${motion === null ? "" : JSON.stringify(motion)}`;
  if (liftCache.has(key)) return liftCache.get(key) ?? null;
  if (liftCache.size >= DIORAMA_CACHE_MAX) liftCache.clear();
  const svg = dioramaSliceSvg(defId, radiant, index, moving, motion);
  const lifted = svg === "" ? null : liftSvg(svg);
  const out = lifted === null ? null : { uri: svgDataUri(lifted.svg), moves: lifted.moves };
  liftCache.set(key, out);
  return out;
}

/** #3674: the picture a board face's window draws for slice `index`: its lift's still picture, else the slice as before. */
export function sliceWindowUri(defId: string, radiant: boolean, index: number, moving: boolean, motion: ArtMotion | null = null): string {
  return liftedSlice(defId, radiant, index, moving, motion)?.uri ?? dioramaSliceUri(defId, radiant, index, moving, motion);
}
