// R3350000 (#3350): the pieces a Mythic's still picture is drawn from, each keyed so a diorama's slice can pick it. A diorama
// draws no new drawing: it recomposes its card's picture, so a slice holds pieces of the picture its painter draws, and
// every piece is in one slice (dioramas.test.ts). A scene card's picture (scene.ts, R3331000) is its sky, its glow, its
// Radiant rays, a ridge for each of its scene's shapes (and the painter's own subject where the recipe keeps it), its emblem
// and its vignette; a Stinkyscape painter's (stinkyscape.ts `stinkyParts`) is its sky's elements, its rays', its subject's
// (drawn in the subject's placing transform) and its vignette.
//
// The keys: `sky`, `glow`, `rays`, `emblem` and `vignette`; a scene shape's `scene:<layer>:<part>:<n>` (the nth shape of that
// part in that layer) and the painter's own subject's `own:<n>`; a Stinkyscape painter's `sky:<n>`, `rays:<n>` and
// `subject:<n>`. A slice picks a key by itself, every key under a prefix ending in ":", or a run `<prefix>:<a>-<b>`.
// Each piece knows the role its painter moves it in (motion.ts, R2715001), so a slice of a face at an animated level moves
// its pieces as the whole moving picture does (index.ts).
// Presentation only (CLAUDE.md rule 7): a piece is drawn from the card's id and face alone (R202).

import { CATALOG } from "../../../wire/cards.ts";
import type { ArtMotion, ArtRole } from "../motion.ts";
import { artPlaneUri } from "../pictures.ts";
import { composeScene, SCENE_LAYER_ROLES, SCENE_LAYERS, sceneRecipeOf, type SceneShape } from "../scene.ts";
import { isStinkyscapeArt, stinkyParts } from "../stinkyscape.ts";

/** R3350000: how many pictures' pieces, slices and slice URIs are memoised before each memo is cleared. */
export const DIORAMA_CACHE_MAX = 512;

/** One tag of the markup: whether it closes, its name, and whether it closes itself. */
const TAG = /<(\/?)([a-zA-Z][\w:-]*)[^>]*?(\/?)>/g;

/** What a data URI of an SVG picture starts with. */
const DATA_PREFIX = "data:image/svg+xml,";

/** What ends a picture's definitions, the last of its head. */
const DEFS_END = "</defs>";

/** What ends a picture. */
const SVG_END = "</svg>";

/** The nodes of a scene card's still picture around its ridges: its sky and glow before them, its emblem and vignette after. */
const SCENE_HEAD_NODES = 2;
const SCENE_TAIL_NODES = 2;

/**
 * R3350000: `markup`'s top-level elements, each as it is written. It holds elements alone: text outside them throws, as
 * does a tag closed out of turn.
 */
export function splitNodes(markup: string): string[] {
  const out: string[] = [];
  let depth = 0;
  let start = 0;
  let last = 0;
  for (const match of markup.matchAll(TAG)) {
    const at = match.index;
    if (depth === 0) {
      if (markup.slice(last, at).trim() !== "") throw new Error(`text outside an element: ${markup.slice(last, at)}`);
      start = at;
    }
    if (match[1] === "/") depth -= 1;
    else if (match[3] !== "/") depth += 1;
    if (depth < 0) throw new Error(`a tag closed out of turn: ${match[0]}`);
    last = at + match[0].length;
    if (depth === 0) out.push(markup.slice(start, last));
  }
  if (depth !== 0) throw new Error("an element left open");
  if (markup.slice(last).trim() !== "") throw new Error(`text outside an element: ${markup.slice(last)}`);
  return out;
}

/**
 * R3350000: one piece of a picture: its key, its markup, whether it is drawn in the subject's placing transform, and the role
 * its painter moves it in (R2715001), or null for what no motion moves (a scene's sky, the rays, the vignette).
 */
export type PicturePiece = { readonly key: string; readonly markup: string; readonly inSubject: boolean; readonly role: ArtRole | null };

/**
 * R3350000: a picture in pieces: its head (the document's opening and its definitions, up to `</defs>`), the transform its
 * subject's pieces are placed in (a Stinkyscape painter's; null for a scene's), and its pieces in the order it draws them.
 */
export type PicturePieces = { readonly head: string; readonly transform: string | null; readonly pieces: readonly PicturePiece[] };

const piecesCache = new Map<string, PicturePieces | null>();

/** The whole picture of `defId`'s face, as its face draws it, unencoded: still, or moving as `motion` says. */
function wholeSvg(defId: string, radiant: boolean, motion: ArtMotion | null = null): string {
  const uri = artPlaneUri(defId, "whole", radiant, motion);
  if (!uri.startsWith(DATA_PREFIX)) throw new Error(`${defId} has no painted picture`);
  return decodeURIComponent(uri.slice(DATA_PREFIX.length));
}

/** What opens a moving picture's style, right after its definitions (motion.ts `motionStyle`). */
const STYLE_OPEN = "<style>";
const STYLE_CLOSE = "</style>";

/**
 * R3350000: the `<style>` `defId`'s whole picture moves by as `motion` says (its roles' rules, its particles' and their
 * keyframes, inside the reduced-motion gate), which a slice of a face at an animated level carries; empty when it has none.
 */
export function motionStyleOf(defId: string, radiant: boolean, motion: ArtMotion): string {
  const memo = `${defId}|${radiant}|${JSON.stringify(motion)}`;
  const hit = stylesCache.get(memo);
  if (hit !== undefined) return hit;
  if (stylesCache.size >= DIORAMA_CACHE_MAX) stylesCache.clear();
  const { body } = headAndBody(wholeSvg(defId, radiant, motion));
  const style = body.startsWith(STYLE_OPEN) ? body.slice(0, body.indexOf(STYLE_CLOSE) + STYLE_CLOSE.length) : "";
  stylesCache.set(memo, style);
  return style;
}

const stylesCache = new Map<string, string>();

/** A picture's head and its body: what lies between `</defs>` and its closing `</svg>`. */
function headAndBody(svg: string): { head: string; body: string } {
  const cut = svg.indexOf(DEFS_END);
  if (cut < 0 || !svg.endsWith(SVG_END)) throw new Error("a picture with no definitions");
  const end = cut + DEFS_END.length;
  return { head: svg.slice(0, end), body: svg.slice(end, svg.length - SVG_END.length) };
}

/** Keys counted per name: `name:0`, `name:1`… in the order asked. */
function counter(): (name: string) => string {
  const seen = new Map<string, number>();
  return (name) => {
    const n = seen.get(name) ?? 0;
    seen.set(name, n + 1);
    return `${name}:${n}`;
  };
}

/** A scene card's pieces (scene.ts `withScene`'s ridges, between its sky and glow and its emblem and vignette). */
function scenePieces(defId: string, radiant: boolean): PicturePieces | null {
  const recipe = sceneRecipeOf(defId);
  if (recipe === null) return null;
  const { head, body } = headAndBody(wholeSvg(defId, radiant));
  const nodes = splitNodes(body);
  const shapes = composeScene(recipe, defId);
  const rays = radiant ? 1 : 0;
  const ridges = nodes.length - SCENE_HEAD_NODES - rays - SCENE_TAIL_NODES;
  const own = ridges - shapes.length;
  if (own < 0 || (recipe.ownSubject !== true && own !== 0)) throw new Error(`${defId}: ${ridges} ridges for ${shapes.length} shapes`);
  const subjectAt = SCENE_LAYERS.indexOf("subject");
  const subjectEnd = shapes.filter((shape) => SCENE_LAYERS.indexOf(shape.layer) <= subjectAt).length;
  const key = counter();
  const shapeKeys = shapes.map((shape) => key(`scene:${shape.layer}:${shape.part}`));
  const ridgeKey = (i: number): string =>
    i < subjectEnd ? (shapeKeys[i] ?? "") : i < subjectEnd + own ? `own:${i - subjectEnd}` : (shapeKeys[i - own] ?? "");
  // The role each ridge moves in, as `withScene` gives it: a `props` placement's the props', else its layer's; the painter's own
  // subject's the subject's.
  const roleOf = (shape: SceneShape | undefined): ArtRole | null => (shape === undefined ? null : shape.props === true ? "props" : SCENE_LAYER_ROLES[shape.layer]);
  const ridgeRole = (i: number): ArtRole | null =>
    i < subjectEnd ? roleOf(shapes[i]) : i < subjectEnd + own ? "subject" : roleOf(shapes[i - own]);
  const keyed: [string, ArtRole | null][] = [
    ["sky", null],
    ["glow", "light"],
    ...(radiant ? [["rays", null] as [string, null]] : []),
    ...Array.from({ length: ridges }, (_, i): [string, ArtRole | null] => [ridgeKey(i), ridgeRole(i)]),
    ["emblem", "subject"],
    ["vignette", null],
  ];
  if (keyed.length !== nodes.length) throw new Error(`${defId}: ${nodes.length} nodes for ${keyed.length} keys`);
  return {
    head,
    transform: null,
    pieces: nodes.map((markup, i) => ({ key: keyed[i]?.[0] ?? "", markup, inSubject: false, role: keyed[i]?.[1] ?? null })),
  };
}

/** A Stinkyscape painter's pieces (`stinkyParts`): its sky's, its rays', its subject's and its vignette. */
function stinkyPieces(defId: string, radiant: boolean): PicturePieces | null {
  const def = CATALOG[defId];
  if (def === undefined || !isStinkyscapeArt(defId)) return null;
  const parts = stinkyParts(defId, def.name, def.tags, def.type, radiant);
  if (parts === null) return null;
  const { head } = headAndBody(wholeSvg(defId, radiant));
  const keyed = (name: string, markup: string, role: ArtRole | null, inSubject = false): PicturePiece[] =>
    splitNodes(markup).map((node, n) => ({ key: `${name}:${n}`, markup: node, inSubject, role }));
  // The painter moves its sky as the back and its subject as the subject (`stinkyDocument`).
  return {
    head,
    transform: parts.transform,
    pieces: [
      ...keyed("sky", parts.sky, "back"),
      ...keyed("rays", parts.rays, null),
      ...keyed("subject", parts.subject, "subject", true),
      { key: "vignette", markup: parts.vignette, inSubject: false, role: null },
    ],
  };
}

/**
 * R3350000: `defId`'s still picture on the face `radiant` says, in its pieces: a scene card's, else a Stinkyscape painter's;
 * null for a card drawn by neither. Memoised.
 */
export function picturePieces(defId: string, radiant: boolean): PicturePieces | null {
  const memo = `${defId}|${radiant}`;
  if (piecesCache.has(memo)) return piecesCache.get(memo) ?? null;
  if (piecesCache.size >= DIORAMA_CACHE_MAX) piecesCache.clear();
  const pieces = scenePieces(defId, radiant) ?? stinkyPieces(defId, radiant);
  piecesCache.set(memo, pieces);
  return pieces;
}

/** A run's pattern: `<prefix>:<a>-<b>`. */
const RUN = /^(.*):(\d+)-(\d+)$/;

/**
 * R3350000: whether a slice's `pattern` picks the piece `key`: the key itself, any key under a prefix that ends in ":", or a
 * key `<prefix>:<n>` with n in a run `<prefix>:<a>-<b>`.
 */
export function matches(pattern: string, key: string): boolean {
  if (pattern === key) return true;
  if (pattern.endsWith(":")) return key.startsWith(pattern);
  const run = RUN.exec(pattern);
  if (run === null) return false;
  const [, prefix, low, high] = run;
  const cut = key.lastIndexOf(":");
  if (cut < 0 || key.slice(0, cut) !== prefix) return false;
  const tail = key.slice(cut + 1);
  if (!/^\d+$/.test(tail)) return false;
  const n = Number(tail);
  return n >= Number(low) && n <= Number(high);
}
