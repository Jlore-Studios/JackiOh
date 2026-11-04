// Prints an `ArtSpec` as a `data:image/svg+xml,…` URI (docs/polish/6-cards.md, Surface A, B3): no
// <text>, <title>, `href` or external reference (its `url(…)`s are its own gradients), percent-encoded
// in full so it sits inside a CSS `url("…")` in any browser and in jsdom's style parser.
//
// R2715001: given a full-art card's motion (motion.ts), the same picture moves: its layers are tagged
// by role and a `<style>` after the `<defs>` animates them, with particles over it. Given none, it is
// the still picture, byte for byte. R2949000: given a seed other than the card's id (pictures.ts), a
// painter draws the card's picture of that look instead, cached under that seed. R2940001: given `sketch`,
// the picture is drawn as the Sketch version's pencil sketch of it (pictures.ts `sketchSvg`), cached apart.

import type { CardType, Tag } from "@jackioh/shared";

import { EMBLEM_BOX, EMBLEMS } from "./emblems.ts";
import { fmt } from "./hash.ts";
import { CENTER_ORIGIN, FIGURE_SUBJECT_ORIGIN, motionStyle, moving, particleMarkup, type ArtMotion, type ArtRole } from "./motion.ts";
import type { MotifId } from "./motifs.ts";
import { sketchSvg } from "./pictures.ts";
import { artSpec, ART_BOX, type ArtSpec } from "./procedural.ts";
import { starshipSvg, stinkyscapeSvg } from "./stinkyscape.ts";
import { legacySvg, trainingGroundsSvg } from "./traininggrounds.ts";
import { RADIANT_PALETTE, type ArtThemeId, type Composition } from "./themes.ts";

const DATA_PREFIX = "data:image/svg+xml,";
/** Rays reach past every corner of the box from anywhere inside it. */
export const RAY_LENGTH = 150;
export const EMBLEM_STROKE_WIDTH = 1.1;
/** The glow's gradient keeps this share of its opacity at its midpoint. */
export const GLOW_MID_STOP = 0.45;
export const VIGNETTE_RADIUS = 72;
export const VIGNETTE_START = 0.6;
export const VIGNETTE_OPACITY = 0.5;
const DEGREES = Math.PI / 180;

const ART_CACHE_MAX = 512;

export const ACCENT_OPACITY = 0.8;
/** An ink-drop mote (Book): its tail rises this many radii above its center. */
export const DROP_TAIL = 2.2;

function glyphMarkup(placed: NonNullable<ArtSpec["accent"]>, opacity: number): string {
  const glyph = EMBLEMS[placed.glyph];
  const scale = placed.size / EMBLEM_BOX;
  const center = EMBLEM_BOX / 2;
  const fade = opacity === 1 ? "" : ` opacity="${fmt(opacity)}"`;
  return (
    `<g transform="translate(${fmt(placed.x)} ${fmt(placed.y)}) rotate(${fmt(placed.rotate)}) ` +
    `scale(${fmt(scale)}) translate(-${center} -${center})"${fade}>` +
    `<path d="${glyph.d}" fill="${placed.fill}" stroke="${placed.stroke}" stroke-width="${EMBLEM_STROKE_WIDTH}" ` +
    `stroke-linejoin="round" fill-rule="${glyph.rule}"/></g>`
  );
}

function moteMarkup(mote: ArtSpec["motes"][number], shape: ArtSpec["moteShape"]): string {
  const paint = `fill="${mote.fill}" opacity="${fmt(mote.opacity)}"`;
  switch (shape) {
    case "dot":
      return `<circle cx="${fmt(mote.x)}" cy="${fmt(mote.y)}" r="${fmt(mote.r)}" ${paint}/>`;
    case "pixel":
      return `<rect x="${fmt(mote.x - mote.r)}" y="${fmt(mote.y - mote.r)}" width="${fmt(mote.r * 2)}" height="${fmt(mote.r * 2)}" ${paint}/>`;
    case "drop":
      return (
        `<path d="M${fmt(mote.x)} ${fmt(mote.y - mote.r * DROP_TAIL)}L${fmt(mote.x + mote.r)} ${fmt(mote.y)}` +
        `A${fmt(mote.r)} ${fmt(mote.r)} 0 1 1 ${fmt(mote.x - mote.r)} ${fmt(mote.y)}Z" ${paint}/>`
      );
  }
}

function rayPath(cx: number, cy: number, angle: number, width: number): string {
  const a = (angle - width / 2) * DEGREES;
  const b = (angle + width / 2) * DEGREES;
  return (
    `M${fmt(cx)} ${fmt(cy)}` +
    `L${fmt(cx + RAY_LENGTH * Math.cos(a))} ${fmt(cy + RAY_LENGTH * Math.sin(a))}` +
    `L${fmt(cx + RAY_LENGTH * Math.cos(b))} ${fmt(cy + RAY_LENGTH * Math.sin(b))}Z`
  );
}

/** The SVG document, unencoded; R2715001: moving as `motion` says, its particles placed by `defId`. */
function artSvg(spec: ArtSpec, motion: ArtMotion | null, defId: string): string {
  const { sky, glow, emblem } = spec;
  const half = ART_BOX / 2;
  const angle = sky.angle * DEGREES;
  const dx = Math.sin(angle) * half;
  const dy = -Math.cos(angle) * half;
  const parts: string[] = [];

  parts.push(
    `<svg viewBox="0 0 ${ART_BOX} ${ART_BOX}" width="${ART_BOX}" height="${ART_BOX}" ` +
      `xmlns="http://www.w3.org/2000/svg" preserveAspectRatio="xMidYMid slice">`,
  );
  parts.push("<defs>");
  parts.push(
    `<linearGradient id="s" gradientUnits="userSpaceOnUse" x1="${fmt(half - dx)}" y1="${fmt(half - dy)}" ` +
      `x2="${fmt(half + dx)}" y2="${fmt(half + dy)}">` +
      `<stop offset="0" stop-color="${sky.from}"/><stop offset="1" stop-color="${sky.to}"/></linearGradient>`,
  );
  parts.push(
    `<radialGradient id="g">` +
      `<stop offset="0" stop-color="${glow.color}" stop-opacity="${fmt(glow.opacity)}"/>` +
      `<stop offset="${GLOW_MID_STOP}" stop-color="${glow.color}" stop-opacity="${fmt(glow.opacity * GLOW_MID_STOP)}"/>` +
      `<stop offset="1" stop-color="${glow.color}" stop-opacity="0"/></radialGradient>`,
  );
  parts.push(
    `<radialGradient id="v" gradientUnits="userSpaceOnUse" cx="${half}" cy="${half}" r="${VIGNETTE_RADIUS}">` +
      `<stop offset="${VIGNETTE_START}" stop-color="#000" stop-opacity="0"/>` +
      `<stop offset="1" stop-color="#000" stop-opacity="${VIGNETTE_OPACITY}"/></radialGradient>`,
  );
  parts.push("</defs>");
  parts.push(motionStyle(motion, spec.composition === "figure" ? FIGURE_SUBJECT_ORIGIN : CENTER_ORIGIN));

  parts.push(`<rect width="${ART_BOX}" height="${ART_BOX}" fill="url(#s)"/>`);
  parts.push(moving("light", `<circle cx="${fmt(glow.cx)}" cy="${fmt(glow.cy)}" r="${fmt(glow.r)}" fill="url(#g)"/>`, motion));

  if (spec.rays.length > 0) {
    parts.push(`<g fill="${RADIANT_PALETTE.ray}">`);
    for (const ray of spec.rays) {
      parts.push(`<path d="${rayPath(glow.cx, glow.cy, ray.angle, ray.width)}" opacity="${fmt(ray.opacity)}"/>`);
    }
    parts.push("</g>");
  }

  // R2715001: each run of ridges of one role moves as one layer.
  let run: string[] = [];
  let runRole: ArtRole | null = null;
  for (const ridge of spec.ridges) {
    if (ridge.role !== runRole && runRole !== null) {
      parts.push(moving(runRole, run.join(""), motion));
      run = [];
    }
    runRole = ridge.role;
    run.push(`<path d="${ridge.d}" fill="${ridge.fill}" opacity="${fmt(ridge.opacity)}" fill-rule="evenodd"/>`);
  }
  if (runRole !== null) parts.push(moving(runRole, run.join(""), motion));

  // R503: the motif's glyphs sit over the composition and under the emblem.
  spec.motifGlyphs.forEach((placed, index) => parts.push(moving("props", glyphMarkup(placed, placed.opacity), motion, index)));

  parts.push(moving("subject", glyphMarkup(emblem, 1), motion));
  if (spec.accent !== null) parts.push(moving("props", glyphMarkup(spec.accent, ACCENT_OPACITY), motion, spec.motifGlyphs.length));

  spec.motes.forEach((mote, index) => parts.push(moving("motes", moteMarkup(mote, spec.moteShape), motion, index)));

  parts.push(particleMarkup(motion, defId));
  parts.push(`<rect width="${ART_BOX}" height="${ART_BOX}" fill="url(#v)"/>`);
  parts.push("</svg>");
  return parts.join("");
}

/** encodeURIComponent, plus the five characters it leaves alone that CSS or jsdom may choke on. */
function encodeSvg(svg: string): string {
  return encodeURIComponent(svg).replace(/[!'()*]/g, (char) => `%${char.charCodeAt(0).toString(16).toUpperCase()}`);
}

/** R2715001: moving as `motion` says, its particles placed by `defId`; the still picture with none. */
export function artDataUri(spec: ArtSpec, motion: ArtMotion | null = null, defId = ""): string {
  return `${DATA_PREFIX}${encodeSvg(artSvg(spec, motion, defId))}`;
}

// The memo behind `CardArt`, keyed as the Surface fixes plus the motif (R503). Transient `t-<n>` defs
// can be minted without bound, so the map is cleared rather than grown. R2715001: a moving picture adds
// `|moving` (a card's motion is fixed by its id, motion.ts), so the still one keeps its key. R2949000: a
// picture of its own is keyed by its seed (`artSeedOf`) in the card's id's place, so the Signature look's
// picture and motion are never served for the card's normal or Animated one.
const proceduralCache = new Map<string, string>();

/** R2715001: what a moving picture's memo key ends with. */
function movingKey(motion: ArtMotion | null): string {
  return motion === null ? "" : "|moving";
}

/** R2940001: what a sketched picture's memo key ends with, so the plain one keeps its key. */
function sketchKey(sketch: boolean): string {
  return sketch ? "|sketch" : "";
}

/** R2940001: the Starships' one seed for a Sketch's strokes: their picture is drawn from no card's id. */
const STARSHIP_SKETCH_SEED = "starship";

/** R2940001: `svg` as the face prints it: the Sketch version's pencil sketch of it when `sketch`, else itself. */
function sketched(svg: string, sketch: boolean, radiant: boolean, seedId: string): string {
  return sketch ? sketchSvg(svg, radiant, seedId) : svg;
}

export function proceduralArtUri(
  defId: string,
  theme: ArtThemeId,
  composition: Composition,
  radiant: boolean,
  motif: MotifId | null = null,
  motion: ArtMotion | null = null,
  sketch = false,
): string {
  const key = `${defId}|${radiant}|${composition}|${motif ?? ""}${movingKey(motion)}${sketchKey(sketch)}`;
  const hit = proceduralCache.get(key);
  if (hit !== undefined) return hit;
  if (proceduralCache.size >= ART_CACHE_MAX) proceduralCache.clear();
  const svg = artSvg(artSpec(defId, theme, composition, radiant, motif), motion, defId);
  const uri = `${DATA_PREFIX}${encodeSvg(sketched(svg, sketch, radiant, defId))}`;
  proceduralCache.set(key, uri);
  return uri;
}

/** R2249000: a Starship's art as a data URI, cached on what it draws from: its pieces, launch and face. */
export function starshipArtUri(pieces: number, launched: boolean, radiant: boolean, sketch = false): string {
  const key = `starship|${pieces}|${launched}|${radiant}${sketchKey(sketch)}`;
  const hit = proceduralCache.get(key);
  if (hit !== undefined) return hit;
  if (proceduralCache.size >= ART_CACHE_MAX) proceduralCache.clear();
  const uri = `${DATA_PREFIX}${encodeSvg(sketched(starshipSvg(pieces, launched, radiant), sketch, radiant, STARSHIP_SKETCH_SEED))}`;
  proceduralCache.set(key, uri);
  return uri;
}

/**
 * R2170000: a Stinkyscape face's picture (stinkyscape.ts); its key adds the name, which picks its detail. R2949000:
 * grown from `seedId` (the card's id unless a look has a picture of its own), and keyed by it.
 */
export function stinkyscapeArtUri(
  defId: string,
  name: string | undefined,
  tags: readonly Tag[],
  type: CardType,
  radiant: boolean,
  motion: ArtMotion | null = null,
  seedId = defId,
  sketch = false,
): string {
  const key = `${seedId}|${radiant}|stinkyscape|${name ?? ""}|${tags.join(",")}|${type}${movingKey(motion)}${sketchKey(sketch)}`;
  const hit = proceduralCache.get(key);
  if (hit !== undefined) return hit;
  if (proceduralCache.size >= ART_CACHE_MAX) proceduralCache.clear();
  const svg = stinkyscapeSvg(defId, name, tags, type, radiant, motion, seedId);
  const uri = `${DATA_PREFIX}${encodeSvg(sketched(svg, sketch, radiant, seedId))}`;
  proceduralCache.set(key, uri);
  return uri;
}

/** R2202000: a Training Grounds face's picture (traininggrounds.ts), memoised and seeded as Stinkyscape's is. */
export function trainingGroundsArtUri(
  defId: string,
  name: string | undefined,
  tags: readonly Tag[],
  type: CardType,
  radiant: boolean,
  motion: ArtMotion | null = null,
  seedId = defId,
  sketch = false,
): string {
  const key = `${seedId}|${radiant}|traininggrounds|${name ?? ""}|${tags.join(",")}|${type}${movingKey(motion)}${sketchKey(sketch)}`;
  const hit = proceduralCache.get(key);
  if (hit !== undefined) return hit;
  if (proceduralCache.size >= ART_CACHE_MAX) proceduralCache.clear();
  const svg = trainingGroundsSvg(defId, name, tags, type, radiant, motion, seedId);
  const uri = `${DATA_PREFIX}${encodeSvg(sketched(svg, sketch, radiant, seedId))}`;
  proceduralCache.set(key, uri);
  return uri;
}

/** R2724000: a Legacy face's picture (traininggrounds.ts), memoised as the set's own are. */
export function legacyArtUri(defId: string, coreId: string, name: string | undefined, tags: readonly Tag[], type: CardType, radiant: boolean, sketch = false): string {
  const key = `${defId}|${radiant}|legacy|${coreId}|${name ?? ""}|${tags.join(",")}|${type}${sketchKey(sketch)}`;
  const hit = proceduralCache.get(key);
  if (hit !== undefined) return hit;
  if (proceduralCache.size >= ART_CACHE_MAX) proceduralCache.clear();
  const uri = `${DATA_PREFIX}${encodeSvg(sketched(legacySvg(defId, coreId, name, tags, type, radiant), sketch, radiant, defId))}`;
  proceduralCache.set(key, uri);
  return uri;
}
