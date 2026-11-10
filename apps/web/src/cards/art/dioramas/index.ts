// R3350000 (#3350): the Mythics' dioramas. Each Mythic card, and each token printed Mythic, has one file of its own in its set's
// directory (`<set>/<name>.ts`), its `DIORAMA` cutting its card's still picture in DIORAMA_SLICES_MIN to DIORAMA_SLICES_MAX
// depth slices (types.ts), and each slice is a picture of its own (`dioramaSliceSvg`): the picture's head (its definitions,
// so every paint it names is there), the pieces the slice holds in the order the picture draws them, a Stinkyscape subject's
// in its placing transform, and, when it moves, its idle loop: a rule in transform and opacity alone inside the
// reduced-motion gate, on a `<g class="dio">` around its pieces (each piece its own, a step along, when it staggers). Given the
// card's moving art (a face at an animated level, a Mythic moving on the board: R2715001, R3340000), a slice also moves its
// pieces as the whole moving picture does: the picture's own style, each run of one role in that role's layer, and the
// particles in the nearest slice. A face stacks the slices back to front where it drew depth planes (CardFace, R3344000),
// the full-screen view and the takeover too. Presentation only (CLAUDE.md rule 7): a slice is drawn from its card's id and
// face alone (R202).

import { fmt } from "../hash.ts";
import { ART_MOVES, CENTER_ORIGIN, moving as roleLayer, particleMarkup, roleRule, type ArtMotion } from "../motion.ts";
import { group } from "../stinkyDraw.ts";
import { svgDataUri } from "../svg.ts";
import { CLASSIC_DIORAMAS } from "./classic/index.ts";
import { CLASSICPLUS_DIORAMAS } from "./classicplus/index.ts";
import { CORE_DIORAMAS } from "./core/index.ts";
import { MEDITATIVE_DIORAMAS } from "./meditative/index.ts";
import { MEDITATIVEPLUS_DIORAMAS } from "./meditativeplus/index.ts";
import { DIORAMA_CACHE_MAX, matches, motionStyleOf, picturePieces, type PicturePiece } from "./pieces.ts";
import { STINKYSCAPE_DIORAMAS } from "./stinkyscape/index.ts";
import { TRAININGGROUNDS_DIORAMAS } from "./traininggrounds/index.ts";
import type { Diorama, DioramaSlice } from "./types.ts";

export type { Diorama, DioramaBend, DioramaLoop, DioramaSlice } from "./types.ts";

/** R3350000: every diorama, by its card's id. */
export const DIORAMAS: ReadonlyMap<string, Diorama> = new Map(
  [
    ...CORE_DIORAMAS,
    ...CLASSIC_DIORAMAS,
    ...CLASSICPLUS_DIORAMAS,
    ...MEDITATIVE_DIORAMAS,
    ...MEDITATIVEPLUS_DIORAMAS,
    ...STINKYSCAPE_DIORAMAS,
    ...TRAININGGROUNDS_DIORAMAS,
  ].map((diorama) => [diorama.id, diorama]),
);

/** R3350000: `defId`'s diorama, or null for a card with none. */
export function dioramaOf(defId: string): Diorama | null {
  return DIORAMAS.get(defId) ?? null;
}

/** R3350000: the slice of `slices` that holds `piece`: the one that picks it, else the one that holds the rest. */
function sliceOf(slices: readonly DioramaSlice[], piece: PicturePiece): number {
  const picked = slices.findIndex((slice) => slice.rest !== true && slice.pick.some((pattern) => matches(pattern, piece.key)));
  return picked >= 0 ? picked : slices.findIndex((slice) => slice.rest === true);
}

/**
 * R3350000: the pieces of `defId`'s picture on the face `radiant` says, slice by slice, each slice's in the order the picture
 * draws them; null for a card with no diorama, or whose picture has no pieces.
 */
export function slicePieces(defId: string, radiant: boolean): PicturePiece[][] | null {
  const key = `${defId}|${radiant}`;
  if (piecesBySlice.has(key)) return piecesBySlice.get(key) ?? null;
  if (piecesBySlice.size >= DIORAMA_CACHE_MAX) piecesBySlice.clear();
  const diorama = dioramaOf(defId);
  const picture = picturePieces(defId, radiant);
  let out: PicturePiece[][] | null = null;
  if (diorama !== null && picture !== null) {
    const sliced: PicturePiece[][] = diorama.slices.map(() => []);
    for (const piece of picture.pieces) sliced[sliceOf(diorama.slices, piece)]?.push(piece);
    out = sliced;
  }
  piecesBySlice.set(key, out);
  return out;
}

const piecesBySlice = new Map<string, PicturePiece[][] | null>();

/** Consecutive runs of `pieces` that agree on `same`. */
function runs(pieces: readonly PicturePiece[], same: (a: PicturePiece, b: PicturePiece) => boolean): PicturePiece[][] {
  const out: PicturePiece[][] = [];
  for (const piece of pieces) {
    const last = out.at(-1);
    const first = last?.[0];
    if (last !== undefined && first !== undefined && same(first, piece)) last.push(piece);
    else out.push([piece]);
  }
  return out;
}

/** R2715001: `pieces` with each run of one role in that role's layer, as the moving picture draws it; unchanged with no motion. */
function inRoles(pieces: readonly PicturePiece[], motion: ArtMotion | null, markup: (run: readonly PicturePiece[]) => string): string {
  if (motion === null) return markup(pieces);
  return runs(pieces, (a, b) => a.role === b.role)
    .map((run) => {
      const role = run[0]?.role ?? null;
      return role === null ? markup(run) : roleLayer(role, markup(run), motion);
    })
    .join("");
}

/** The class a moving slice's loop runs on. */
const LOOP_CLASS = "dio";

/**
 * R3350000: slice `index` of `defId`'s diorama on the face `radiant` says, as an SVG document, unencoded: the picture's head,
 * its loop's style when `moving` (inside the reduced-motion gate), and its pieces, a Stinkyscape subject's in its placing
 * transform. Given the card's moving art (`motion`), it also moves its pieces by their roles under the picture's own style,
 * and the nearest slice draws the particles. Empty for a card with no diorama, or a slice it does not have.
 */
export function dioramaSliceSvg(defId: string, radiant: boolean, index: number, moving: boolean, motion: ArtMotion | null = null): string {
  const diorama = dioramaOf(defId);
  const slice = diorama?.slices[index];
  const picture = picturePieces(defId, radiant);
  const pieces = slicePieces(defId, radiant)?.[index];
  if (diorama === null || slice === undefined || picture === null || pieces === undefined) return "";
  const loop = moving ? (slice.loop ?? null) : null;
  const parts: string[] = [picture.head];
  if (motion !== null) parts.push(motionStyleOf(defId, radiant, motion));
  if (loop !== null) {
    const origin = loop.origin === undefined ? CENTER_ORIGIN : `${fmt(loop.origin[0])}px ${fmt(loop.origin[1])}px`;
    parts.push(
      `<style>@media (prefers-reduced-motion: no-preference){${roleRule(`.${LOOP_CLASS}`, loop, origin)}` +
        `@keyframes art-${loop.move}{${ART_MOVES[loop.move].keyframes}}}</style>`,
    );
  }
  // A staggered loop starts each piece a step further along it, a step a piece.
  const step = (k: number): string => fmt((k * (loop?.seconds ?? 0)) / pieces.length);
  const plain = (run: readonly PicturePiece[]): string => run.map((piece) => piece.markup).join("");
  const staggered = (run: readonly PicturePiece[]): string =>
    run.map((piece) => `<g class="${LOOP_CLASS}" style="animation-delay:-${step(pieces.indexOf(piece))}s">${piece.markup}</g>`).join("");
  for (const run of runs(pieces, (a, b) => a.inSubject === b.inSubject)) {
    if (run[0]?.inSubject === true && picture.transform !== null) {
      // A Stinkyscape subject is placed by its transform, its loop inside it and its role's layer around it, as the moving
      // picture has it.
      const inner = loop === null ? plain(run) : loop.stagger === true ? staggered(run) : `<g class="${LOOP_CLASS}">${plain(run)}</g>`;
      const placed = group(picture.transform, inner);
      parts.push(motion === null ? placed : roleLayer("subject", placed, motion));
      continue;
    }
    // A loop wraps the run, its role layers inside it; a staggered one each piece, inside its role's layer.
    parts.push(
      loop === null
        ? inRoles(run, motion, plain)
        : loop.stagger === true
          ? inRoles(run, motion, staggered)
          : `<g class="${LOOP_CLASS}">${inRoles(run, motion, plain)}</g>`,
    );
  }
  if (motion !== null && index === diorama.slices.length - 1) parts.push(particleMarkup(motion, defId));
  parts.push("</svg>");
  return parts.join("");
}

const sliceCache = new Map<string, string>();

/**
 * R3350000: `dioramaSliceSvg` as the data URI a window draws, memoised on the card, its face, the slice, whether its loop runs
 * and the moving art it carries.
 */
export function dioramaSliceUri(defId: string, radiant: boolean, index: number, moving: boolean, motion: ArtMotion | null = null): string {
  const key = `${defId}|${radiant}|${index}|${moving}|${motion === null ? "" : JSON.stringify(motion)}`;
  const hit = sliceCache.get(key);
  if (hit !== undefined) return hit;
  if (sliceCache.size >= DIORAMA_CACHE_MAX) sliceCache.clear();
  const svg = dioramaSliceSvg(defId, radiant, index, moving, motion);
  const uri = svg === "" ? "" : svgDataUri(svg);
  sliceCache.set(key, uri);
  return uri;
}

/** R3350000: `dioramaSliceUri` as a CSS `background-image` value (the effects layer holds no `url(` of its own: B27). */
export function dioramaSliceBackground(defId: string, radiant: boolean, index: number, moving = false): string {
  return `url("${dioramaSliceUri(defId, radiant, index, moving)}")`;
}
