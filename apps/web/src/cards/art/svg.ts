// Prints an `ArtSpec` as a `data:image/svg+xml,…` URI (docs/polish/6-cards.md, Surface A, B3): no
// <text>, <title>, `href` or external reference (its `url(…)`s are its own gradients), percent-encoded
// in full so it sits inside a CSS `url("…")` in any browser and in jsdom's style parser.
//
// R2715001: given a full-art card's motion (motion.ts), the same picture moves: its layers are tagged
// by role and a `<style>` after the `<defs>` animates them, with particles over it. Given none, it is
// the still picture, byte for byte. R2949000: given a seed other than the card's id (pictures.ts), a
// painter draws the card's picture of that look instead, cached under that seed. R2940001: given `restyle`
// "sketch", the picture is drawn as the Sketch version's pencil sketch of it (pictures.ts `sketchSvg`), cached
// apart; R2942001: given "pixel" or "comic", as Pixel's or Comic's (`restyledSvg`), cached apart too; R2943001: given
// a Set showcase restyle, in its set's treatment. R2943001: given a `part` other than "whole" (Out of frame's), the
// procedural, Stinkyscape and Training Grounds painters draw the picture's backdrop with no subject, or its subject on
// nothing, cached apart; "whole" is the picture, byte for byte.

import type { CardType, Tag } from "@jackioh/shared";

import type { AnomalyCode } from "../anomalies.ts";
import { EMBLEM_BOX, EMBLEMS } from "./emblems.ts";
import { fmt } from "./hash.ts";
import { CENTER_ORIGIN, FIGURE_SUBJECT_ORIGIN, motionStyle, moving, particleMarkup, type ArtMotion, type ArtRole } from "./motion.ts";
import type { MotifId } from "./motifs.ts";
import { restyledSvg, type ArtPart, type ArtRestyle } from "./pictures.ts";
import { artSpec, ART_BOX, type ArtSpec } from "./procedural.ts";
import { starshipArtKey, type StarshipArt } from "./starshipHulls.ts";
import { ANOMALY_SEED_PREFIX, anomalySvg, starshipModuleIconSvg, starshipSvg, stinkyscapeSvg, type ShipArt } from "./stinkyscape.ts";
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

/**
 * The SVG document, unencoded; R2715001: moving as `motion` says, its particles placed by `defId`. R2943001: `part`
 * "backdrop" leaves out the subject (its subject ridges and emblem), and "subject" draws those alone.
 */
function artSvg(spec: ArtSpec, motion: ArtMotion | null, defId: string, part: ArtPart = "whole"): string {
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
  const subject = part !== "backdrop";
  const backdrop = part !== "subject";

  if (backdrop) parts.push(`<rect width="${ART_BOX}" height="${ART_BOX}" fill="url(#s)"/>`);
  if (backdrop) parts.push(moving("light", `<circle cx="${fmt(glow.cx)}" cy="${fmt(glow.cy)}" r="${fmt(glow.r)}" fill="url(#g)"/>`, motion));

  if (backdrop && spec.rays.length > 0) {
    parts.push(`<g fill="${RADIANT_PALETTE.ray}">`);
    for (const ray of spec.rays) {
      parts.push(`<path d="${rayPath(glow.cx, glow.cy, ray.angle, ray.width)}" opacity="${fmt(ray.opacity)}"/>`);
    }
    parts.push("</g>");
  }

  // R2715001: each run of ridges of one role moves as one layer. R2943001: a part draws the runs that are its own.
  const drawn = (role: ArtRole): boolean => (role === "subject" ? subject : backdrop);
  let run: string[] = [];
  let runRole: ArtRole | null = null;
  for (const ridge of spec.ridges) {
    if (ridge.role !== runRole && runRole !== null) {
      if (drawn(runRole)) parts.push(moving(runRole, run.join(""), motion));
      run = [];
    }
    runRole = ridge.role;
    run.push(`<path d="${ridge.d}" fill="${ridge.fill}" opacity="${fmt(ridge.opacity)}" fill-rule="evenodd"/>`);
  }
  if (runRole !== null && drawn(runRole)) parts.push(moving(runRole, run.join(""), motion));

  // R503: the motif's glyphs sit over the composition and under the emblem.
  if (backdrop) spec.motifGlyphs.forEach((placed, index) => parts.push(moving("props", glyphMarkup(placed, placed.opacity), motion, index)));

  if (subject) parts.push(moving("subject", glyphMarkup(emblem, 1), motion));
  if (backdrop && spec.accent !== null) parts.push(moving("props", glyphMarkup(spec.accent, ACCENT_OPACITY), motion, spec.motifGlyphs.length));

  if (backdrop) spec.motes.forEach((mote, index) => parts.push(moving("motes", moteMarkup(mote, spec.moteShape), motion, index)));

  if (backdrop) parts.push(particleMarkup(motion, defId));
  if (backdrop) parts.push(`<rect width="${ART_BOX}" height="${ART_BOX}" fill="url(#v)"/>`);
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
// `|moving` (a card's motion is fixed by its id, motion.ts), so the still one keeps its key. R2940001: a
// picture of its own is keyed by its seed (`artSeedOf`) in the card's id's place, so Alt art's picture is
// never served for the card's normal one.
const proceduralCache = new Map<string, string>();

/** R2715001: what a moving picture's memo key ends with. */
function movingKey(motion: ArtMotion | null): string {
  return motion === null ? "" : "|moving";
}

/** R2940001, R2942001: what a restyled picture's memo key ends with, so the plain one and the Sketch keep theirs. */
function restyleKey(restyle: ArtRestyle | null): string {
  return restyle === null ? "" : `|${restyle}`;
}

/** R2943001: what a part's memo key ends with: nothing for the whole picture, so it keeps its key. */
function partKey(part: ArtPart): string {
  return part === "whole" ? "" : `|${part}`;
}

/** R3336000: what a Battleship layer's memo key ends with: nothing for a printed card, so it keeps its key. */
function shipKey(ship: ShipArt | null): string {
  return ship === null ? "" : `|ship${ship.buried}.${ship.heat ?? 0}.${ship.brittle ?? ""}.${ship.bays ?? ""}`;
}

/** R2940001: the Starships' one seed for a Sketch's strokes: their picture is drawn from no card's id. */
const STARSHIP_SKETCH_SEED = "starship";

/** R2940001, R2942001: `svg` as the face prints it: restyled by `restyle` (the Sketch, Pixel or Comic), else itself. */
function restyled(svg: string, restyle: ArtRestyle | null, radiant: boolean, seedId: string): string {
  return restyle === null ? svg : restyledSvg(svg, restyle, radiant, seedId);
}

export function proceduralArtUri(
  defId: string,
  theme: ArtThemeId,
  composition: Composition,
  radiant: boolean,
  motif: MotifId | null = null,
  motion: ArtMotion | null = null,
  restyle: ArtRestyle | null = null,
  part: ArtPart = "whole",
): string {
  const key = `${defId}|${radiant}|${composition}|${motif ?? ""}${movingKey(motion)}${restyleKey(restyle)}${partKey(part)}`;
  const hit = proceduralCache.get(key);
  if (hit !== undefined) return hit;
  if (proceduralCache.size >= ART_CACHE_MAX) proceduralCache.clear();
  const svg = artSvg(artSpec(defId, theme, composition, radiant, motif), motion, defId, part);
  const uri = `${DATA_PREFIX}${encodeSvg(restyled(svg, restyle, radiant, defId))}`;
  proceduralCache.set(key, uri);
  return uri;
}

/**
 * R2249000, R3333000: a Starship's art as a data URI, cached on what it draws from (`starshipArtKey`): its pieces by
 * id and face oldest first, its launch, its design, its livery and its face.
 */
export function starshipArtUri(ship: StarshipArt, restyle: ArtRestyle | null = null): string {
  const key = `${starshipArtKey(ship)}${restyleKey(restyle)}`;
  const hit = proceduralCache.get(key);
  if (hit !== undefined) return hit;
  if (proceduralCache.size >= ART_CACHE_MAX) proceduralCache.clear();
  const uri = `${DATA_PREFIX}${encodeSvg(restyled(starshipSvg(ship), restyle, ship.radiant, STARSHIP_SKETCH_SEED))}`;
  proceduralCache.set(key, uri);
  return uri;
}

/** R3333002: a piece's module icon (stinkyscape.ts `starshipModuleIconSvg`) as a data URI, cached by the piece and its face. */
export function starshipModuleIconUri(defId: string, radiant: boolean): string {
  const key = `starship-module|${defId}|${radiant}`;
  const hit = proceduralCache.get(key);
  if (hit !== undefined) return hit;
  if (proceduralCache.size >= ART_CACHE_MAX) proceduralCache.clear();
  const uri = `${DATA_PREFIX}${encodeSvg(starshipModuleIconSvg(defId, radiant))}`;
  proceduralCache.set(key, uri);
  return uri;
}

/**
 * R3337001: an Anomaly Found!'s art for the board its code names (stinkyscape.ts `anomalySvg`), cached on its code and
 * face; a restyle's strokes grow from the board's own seed.
 */
export function anomalyArtUri(code: AnomalyCode, radiant: boolean, restyle: ArtRestyle | null = null): string {
  const key = `anomaly|${code}|${radiant}${restyleKey(restyle)}`;
  const hit = proceduralCache.get(key);
  if (hit !== undefined) return hit;
  if (proceduralCache.size >= ART_CACHE_MAX) proceduralCache.clear();
  const uri = `${DATA_PREFIX}${encodeSvg(restyled(anomalySvg(code, radiant), restyle, radiant, `${ANOMALY_SEED_PREFIX}${code}`))}`;
  proceduralCache.set(key, uri);
  return uri;
}

/** R3337003: where an inlined picture names an id: its `id` attributes and the `url(#…)` paints that point at them. */
const INLINE_ID = /(?<=\s)id="|url\(#/g;

/**
 * R3337003: the same picture as SVG markup the effects layer draws in place (the title card's, fx/dom.ts), no image
 * (B27); its ids are scoped to its board and face so they meet no other drawing's in the page. Cached apart.
 */
export function anomalyMarkup(code: AnomalyCode, radiant: boolean): string {
  const key = `anomaly-markup|${code}|${radiant}`;
  const hit = proceduralCache.get(key);
  if (hit !== undefined) return hit;
  if (proceduralCache.size >= ART_CACHE_MAX) proceduralCache.clear();
  const scope = `${ANOMALY_SEED_PREFIX}${code}-${radiant ? "radiant" : "base"}-`;
  const markup = anomalySvg(code, radiant).replace(INLINE_ID, (head) => `${head}${scope}`);
  proceduralCache.set(key, markup);
  return markup;
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
  restyle: ArtRestyle | null = null,
  part: ArtPart = "whole",
  ship: ShipArt | null = null,
): string {
  const key = `${seedId}|${radiant}|stinkyscape|${name ?? ""}|${tags.join(",")}|${type}${movingKey(motion)}${restyleKey(restyle)}${partKey(part)}${shipKey(ship)}`;
  const hit = proceduralCache.get(key);
  if (hit !== undefined) return hit;
  if (proceduralCache.size >= ART_CACHE_MAX) proceduralCache.clear();
  const svg = stinkyscapeSvg(defId, name, tags, type, radiant, motion, seedId, part, ship);
  const uri = `${DATA_PREFIX}${encodeSvg(restyled(svg, restyle, radiant, seedId))}`;
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
  restyle: ArtRestyle | null = null,
  part: ArtPart = "whole",
): string {
  const key = `${seedId}|${radiant}|traininggrounds|${name ?? ""}|${tags.join(",")}|${type}${movingKey(motion)}${restyleKey(restyle)}${partKey(part)}`;
  const hit = proceduralCache.get(key);
  if (hit !== undefined) return hit;
  if (proceduralCache.size >= ART_CACHE_MAX) proceduralCache.clear();
  const svg = trainingGroundsSvg(defId, name, tags, type, radiant, motion, seedId, part);
  const uri = `${DATA_PREFIX}${encodeSvg(restyled(svg, restyle, radiant, seedId))}`;
  proceduralCache.set(key, uri);
  return uri;
}

/** R2724000: a Legacy face's picture (traininggrounds.ts), memoised as the set's own are. */
export function legacyArtUri(defId: string, coreId: string, name: string | undefined, tags: readonly Tag[], type: CardType, radiant: boolean, restyle: ArtRestyle | null = null): string {
  const key = `${defId}|${radiant}|legacy|${coreId}|${name ?? ""}|${tags.join(",")}|${type}${restyleKey(restyle)}`;
  const hit = proceduralCache.get(key);
  if (hit !== undefined) return hit;
  if (proceduralCache.size >= ART_CACHE_MAX) proceduralCache.clear();
  const uri = `${DATA_PREFIX}${encodeSvg(restyled(legacySvg(defId, coreId, name, tags, type, radiant), restyle, radiant, defId))}`;
  proceduralCache.set(key, uri);
  return uri;
}
