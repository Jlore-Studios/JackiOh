// Prints an `ArtSpec` as a `data:image/svg+xml,…` URI (docs/polish/6-cards.md, Surface A, B3).
//
// The document is one <svg viewBox="0 0 100 100"> with no <text>, no <title> and no external
// reference: the only `url(…)`s point at gradients defined inside it, and nothing uses `href`.
// It is percent-encoded in full, parentheses and quotes included, so it can sit inside a CSS
// `url("…")` in any browser and in jsdom's style parser.
//
// R2715001: given a full-art card's motion (motion.ts), the same picture moves: its layers are tagged
// by role and a `<style>` after the `<defs>` animates them, with particles over it. Given none, it is
// the still picture, byte for byte.

import type { CardType, Tag } from "@jackioh/shared";

import { EMBLEM_BOX, EMBLEMS } from "./emblems.ts";
import { fmt } from "./hash.ts";
import { CENTER_ORIGIN, FIGURE_SUBJECT_ORIGIN, motionStyle, moving, particleMarkup, type ArtMotion, type ArtRole } from "./motion.ts";
import type { MotifId } from "./motifs.ts";
import { artSpec, ART_BOX, type ArtSpec } from "./procedural.ts";
import { starshipSvg, stinkyscapeSvg } from "./stinkyscape.ts";
import { trainingGroundsSvg } from "./traininggrounds.ts";
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

/** The procedural-art memo is cleared once it would pass this many entries. */
const ART_CACHE_MAX = 512;

/** The accent glyph is quieter than the emblem. */
export const ACCENT_OPACITY = 0.8;
/** An ink-drop mote (Book): its tail rises this many radii above its center. */
export const DROP_TAIL = 2.2;

/** One glyph from EMBLEMS, placed, turned and scaled into the box. */
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

/** One mote: a round dot, an ink drop (Book) or a square pixel (AI). */
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

/**
 * `data:image/svg+xml,…`: one <svg viewBox="0 0 100 100">, no <text>, no <title>, no external refs.
 * R2715001: moving as `motion` says, its particles placed by `defId`; the still picture with none.
 */
export function artDataUri(spec: ArtSpec, motion: ArtMotion | null = null, defId = ""): string {
  return `${DATA_PREFIX}${encodeSvg(artSvg(spec, motion, defId))}`;
}

// The memo behind `CardArt`: keyed `${defId}|${radiant}|${composition}`, as the Surface fixes, plus
// the motif (R503), which a card's name fixes as its id fixes the rest. Transient `t-<n>` defs can
// be minted without bound, so the map is cleared rather than grown. R2715001: a moving picture adds
// `|moving` (a card's motion is fixed by its id, motion.ts), so the still one keeps its key.
const proceduralCache = new Map<string, string>();

/** R2715001: what a moving picture's memo key ends with. */
function movingKey(motion: ArtMotion | null): string {
  return motion === null ? "" : "|moving";
}

export function proceduralArtUri(
  defId: string,
  theme: ArtThemeId,
  composition: Composition,
  radiant: boolean,
  motif: MotifId | null = null,
  motion: ArtMotion | null = null,
): string {
  const key = `${defId}|${radiant}|${composition}|${motif ?? ""}${movingKey(motion)}`;
  const hit = proceduralCache.get(key);
  if (hit !== undefined) return hit;
  if (proceduralCache.size >= ART_CACHE_MAX) proceduralCache.clear();
  const uri = artDataUri(artSpec(defId, theme, composition, radiant, motif), motion, defId);
  proceduralCache.set(key, uri);
  return uri;
}

/**
 * R2170000: a Stinkyscape face's picture (stinkyscape.ts), memoised beside the rest. Its key adds
 * the name, which picks the detail inside the card's family, as the motif does above.
 */
/** R2249000: a Starship's art as a data URI, cached on what it draws from: its pieces, launch and face. */
export function starshipArtUri(pieces: number, launched: boolean, radiant: boolean): string {
  const key = `starship|${pieces}|${launched}|${radiant}`;
  const hit = proceduralCache.get(key);
  if (hit !== undefined) return hit;
  if (proceduralCache.size >= ART_CACHE_MAX) proceduralCache.clear();
  const uri = `${DATA_PREFIX}${encodeSvg(starshipSvg(pieces, launched, radiant))}`;
  proceduralCache.set(key, uri);
  return uri;
}

export function stinkyscapeArtUri(
  defId: string,
  name: string | undefined,
  tags: readonly Tag[],
  type: CardType,
  radiant: boolean,
  motion: ArtMotion | null = null,
): string {
  const key = `${defId}|${radiant}|stinkyscape|${name ?? ""}|${tags.join(",")}|${type}${movingKey(motion)}`;
  const hit = proceduralCache.get(key);
  if (hit !== undefined) return hit;
  if (proceduralCache.size >= ART_CACHE_MAX) proceduralCache.clear();
  const uri = `${DATA_PREFIX}${encodeSvg(stinkyscapeSvg(defId, name, tags, type, radiant, motion))}`;
  proceduralCache.set(key, uri);
  return uri;
}

/** R2202000: a Training Grounds face's picture (traininggrounds.ts), memoised as Stinkyscape's is. */
export function trainingGroundsArtUri(
  defId: string,
  name: string | undefined,
  tags: readonly Tag[],
  type: CardType,
  radiant: boolean,
  motion: ArtMotion | null = null,
): string {
  const key = `${defId}|${radiant}|traininggrounds|${name ?? ""}|${tags.join(",")}|${type}${movingKey(motion)}`;
  const hit = proceduralCache.get(key);
  if (hit !== undefined) return hit;
  if (proceduralCache.size >= ART_CACHE_MAX) proceduralCache.clear();
  const uri = `${DATA_PREFIX}${encodeSvg(trainingGroundsSvg(defId, name, tags, type, radiant, motion))}`;
  proceduralCache.set(key, uri);
  return uri;
}
