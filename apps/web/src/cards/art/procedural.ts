// The procedural card art as data (docs/polish/6-cards.md, Surface A, behaviours B1 and B2).
//
// `artSpec` is pure: the only randomness is `seededRandom(hashId(defId))`, so equal arguments give
// deep-equal specs and `Math.random` is never read. The geometry is drawn first, from that one
// generator, in an order that never looks at `radiant`. The radiant variant therefore reuses the
// base geometry exactly (composition, every ridge path, the emblem's glyph and place) and changes
// only colour, plus its rays, which come from a second generator salted off the same seed.
//
// Per-card variety. A theme sets the palette and the kind of picture; within a theme, each card
// draws its own large-scale features from a stream salted off its seed, so two Humans or two plain
// Spells never share a picture: the composition's LAYOUT (a figure left, right, centred or close
// up; a spell as a starburst, a spiral, an orbit or a shatter; …), a BACKDROP motif behind it (a
// moon, rings, pillars, beams, clouds, a constellation, arches, waves or a floor grid), the SKY's
// lighting (dusk, night, dawn or a split second hue), the emblem from a wider pool, a hue turn, and
// often a second, smaller ACCENT glyph in a free corner.
//
// Everything lives in a 0..100 box and every number is rounded to 2 dp, so the SVG that
// `svg.ts` prints is short and identical on every machine.
//
// v0.2.0 (R503). Three things join the picture, each from its own salted stream so a Core card
// without them draws exactly what it drew before:
// - the card's MOTIF (motifs.ts), the picture its name asks for: a plain card (a type theme or a
//   Token) wears it as its emblem, in the motif's own colours; a tribe or family keeps its emblem
//   and carries the motif beside it (a corner glyph, a scatter, a row along the ground or a fall
//   from the sky), and a figure may wear it on its head;
// - the Book, Pancake and AI families' TWIST: pages in the air, steam and syrup, circuit traces with
//   scanlines and a glitch;
// - a per-set VARIETY SALT: the Classic and Classic+ cards draw their layout, backdrop, sky and
//   emblem from their own salted streams, chosen (like Core's) so that no two catalog cards of one
//   theme share all of them, while Core's stays the salt it always was.

import type { EmblemGlyph } from "./emblems.ts";
import { fmt, hashId, round2, seededRandom } from "./hash.ts";
import { MOTIFS, type Motif, type MotifHeadgear, type MotifId } from "./motifs.ts";
import {
  driftPalette,
  EMBLEM_POOLS,
  HUE_DRIFT,
  mixHex,
  turnHue,
  RADIANT_MIX,
  RADIANT_PALETTE,
  THEME_MOTES,
  THEME_PALETTES,
  type ArtThemeId,
  type Composition,
  type MoteShape,
  type ThemePalette,
} from "./themes.ts";

/** The motif drawn behind a composition. */
export type Backdrop = "none" | "moon" | "rings" | "pillars" | "beams" | "clouds" | "stars" | "arches" | "waves" | "grid";
/** How the sky is lit. */
export type SkyScheme = "dusk" | "night" | "dawn" | "split";

/** Each composition's arrangements, one per card. */
export const LAYOUTS: Readonly<Record<Composition, readonly string[]>> = {
  figure: ["centre", "left", "right", "close"],
  burst: ["star", "spiral", "orbit", "shatter"],
  landscape: ["range", "sea", "towers", "dunes"],
  sigil: ["ring", "diamond", "eye", "wheel"],
};

export const BACKDROPS: readonly Backdrop[] = [
  "none",
  "moon",
  "rings",
  "pillars",
  "beams",
  "clouds",
  "stars",
  "arches",
  "waves",
  "grid",
];

export const SKY_SCHEMES: readonly SkyScheme[] = ["dusk", "night", "dawn", "split"];

type Glyph = { glyph: EmblemGlyph; x: number; y: number; size: number; rotate: number; fill: string; stroke: string };
/** A motif's glyph: a placed glyph with its own opacity. */
type MotifGlyphSpec = Glyph & { opacity: number };

/** Everything the SVG is drawn from. Coordinates are in a 0..100 box, numbers rounded to 2 dp. */
export type ArtSpec = {
  theme: ArtThemeId;
  composition: Composition;
  variant: "base" | "radiant";
  /** The composition's arrangement for this card, one of LAYOUTS[composition]. */
  layout: string;
  /** The motif behind the composition; its paths are the first `ridges`. */
  backdrop: Backdrop;
  skyScheme: SkyScheme;
  sky: { from: string; to: string; angle: number };
  glow: { cx: number; cy: number; r: number; color: string; opacity: number };
  ridges: readonly { d: string; fill: string; opacity: number }[];
  emblem: Glyph;
  /** A second, smaller glyph in the corner furthest from the emblem, or null. */
  accent: Glyph | null;
  motes: readonly { x: number; y: number; r: number; fill: string; opacity: number }[];
  /** How the motes are drawn: round dust, ink drops or square pixels (THEME_MOTES). */
  moteShape: MoteShape;
  /** Empty on the base variant; 7 to 11 rays on the radiant one. */
  rays: readonly { angle: number; width: number; opacity: number }[];
  /** R503: the motif the card's name calls up (motifs.ts), or null. */
  motif: MotifId | null;
  /** The motif's glyphs, drawn over the composition and under the emblem; empty without one. */
  motifGlyphs: readonly MotifGlyphSpec[];
};

/** The drawing box is 0..ART_BOX on both axes. */
export const ART_BOX = 100;
const RAY_COUNT_MIN = 7;
const RAY_COUNT_MAX = 11;
const RADIANT_SALT = 0x9e3779b9;
/** Salts the generator that picks a card's emblem and hue drift, so the geometry stream is untouched. */
const VARIETY_SALT = 0x2545f491;
/**
 * R503: the variety salt of each set's cards, by the set's id prefix (B2.2: `classic-043`,
 * `classicplus-012-1`). Each was chosen, as Core's was, so that no two catalog cards of one theme
 * share their layout, backdrop, sky, emblem and motif, and the families spread over every layout;
 * an id of any other set (a transient `t-<n>`) uses Core's.
 */
const SET_VARIETY_SALTS: Readonly<Record<string, number>> = {
  classic: 0x79bdaf98,
  classicplus: 0x5921ab01,
};
/** Salt the motif's and the family twist's own streams, so neither moves anything else. */
const MOTIF_SALT = 0x165667b1;
const TWIST_SALT = 0x3c6ef372;
/** A radiant face mixes a motif's own colours this far toward the gold. */
const RADIANT_MOTIF_MIX = 0.5;
/** A plain card's emblem is this much larger when it is the card's motif, so the picture reads. */
export const MOTIF_EMBLEM_SCALE = 1.2;
const RADIANT_GLOW_BOOST = 0.1;
const FULL_TURN = 360;
const DEGREES = Math.PI / 180;
/** Sky gradients run roughly top to bottom: 180° ± 30°. */
const SKY_ANGLE_MIN = 150;
const SKY_ANGLE_MAX = 210;
const MOTES_MIN = 5;
const MOTES_MAX = 10;
/** Share of cards that carry an accent glyph. */
const ACCENT_CHANCE = 0.7;
/** How far toward black a night sky goes, and how far a split sky's second stop turns. */
const NIGHT_MIX = 0.45;
const SPLIT_TURN_MIN = 60;
const SPLIT_TURN_MAX = 120;
/** A dawn sky's horizon, warmed this far toward this orange. */
const DAWN_WARMTH = "#ff9a5a";
const DAWN_MIX = 0.38;
/** Every glyph an accent may be. */
const ACCENT_POOL: readonly EmblemGlyph[] = [
  "star",
  "moon",
  "flame",
  "crystal",
  "eye",
  "rune",
  "hourglass",
  "crown",
  "bolt",
  "coin",
  "shield",
  "sword",
  "tower",
];

type Rng = () => number;
/** A layer's colour: a palette slot, the motif's own colour, or the family twist's. */
type LayerFill = "far" | "mid" | "near" | "glow" | "accent" | "motif" | "twist";
type Layer = { d: string; fill: LayerFill; opacity: number };
type Geometry = {
  layers: Layer[];
  glow: { cx: number; cy: number; r: number; opacity: number };
  emblem: { x: number; y: number; size: number; rotate: number };
  /** The band of the box the motes drift in. */
  motes: { top: number; bottom: number };
};

function between(rng: Rng, low: number, high: number): number {
  return low + (high - low) * rng();
}

/** An integer in [low, high], inclusive. */
function whole(rng: Rng, low: number, high: number): number {
  return low + Math.floor(rng() * (high - low + 1));
}

function pt(x: number, y: number): string {
  return `${fmt(x)} ${fmt(y)}`;
}

function circlePath(cx: number, cy: number, r: number): string {
  return `M${pt(cx - r, cy)}A${fmt(r)} ${fmt(r)} 0 1 0 ${pt(cx + r, cy)}A${fmt(r)} ${fmt(r)} 0 1 0 ${pt(cx - r, cy)}Z`;
}

function ellipsePath(cx: number, cy: number, rx: number, ry: number): string {
  return `M${pt(cx - rx, cy)}A${fmt(rx)} ${fmt(ry)} 0 1 0 ${pt(cx + rx, cy)}A${fmt(rx)} ${fmt(ry)} 0 1 0 ${pt(cx - rx, cy)}Z`;
}

/** Rolling hills from edge to edge, closed along the bottom of the box. */
function hills(rng: Rng, baseY: number, amplitude: number, count: number): string {
  const ys: number[] = [];
  for (let i = 0; i <= count; i += 1) ys.push(baseY + (rng() * 2 - 1) * amplitude);
  const x = (i: number): number => (ART_BOX * i) / count;
  const y = (i: number): number => ys[i] ?? baseY;
  let d = `M0 ${ART_BOX}L${pt(0, y(0))}`;
  for (let i = 1; i < count; i += 1) {
    d += `Q${pt(x(i), y(i))} ${pt((x(i) + x(i + 1)) / 2, (y(i) + y(i + 1)) / 2)}`;
  }
  return `${d}L${pt(ART_BOX, y(count))}L${ART_BOX} ${ART_BOX}Z`;
}

/** A jagged range: peaks on the odd points, valleys on the even ones. */
function mountains(rng: Rng, baseY: number, height: number, count: number): string {
  let d = `M0 ${ART_BOX}`;
  for (let i = 0; i <= count; i += 1) {
    const edge = i === 0 || i === count;
    const x = edge ? (i === 0 ? 0 : ART_BOX) : (ART_BOX * i) / count + between(rng, -3, 3);
    const lift = i % 2 === 1 ? between(rng, 0.6, 1) : between(rng, 0, 0.35);
    d += `L${pt(x, baseY - height * lift)}`;
  }
  return `${d}L${ART_BOX} ${ART_BOX}Z`;
}

function starPath(
  rng: Rng,
  cx: number,
  cy: number,
  points: number,
  outer: number,
  inner: number,
  offset: number,
  jitter: number,
): string {
  let d = "";
  for (let i = 0; i < points * 2; i += 1) {
    const angle = (offset + (i * FULL_TURN) / (points * 2)) * DEGREES;
    const r = i % 2 === 0 ? outer + between(rng, -jitter, jitter) : inner;
    d += `${i === 0 ? "M" : "L"}${pt(cx + r * Math.cos(angle), cy + r * Math.sin(angle))}`;
  }
  return `${d}Z`;
}

function polygonPath(cx: number, cy: number, sides: number, radius: number, offset: number): string {
  let d = "";
  for (let i = 0; i < sides; i += 1) {
    const angle = (offset + (i * FULL_TURN) / sides) * DEGREES;
    d += `${i === 0 ? "M" : "L"}${pt(cx + radius * Math.cos(angle), cy + radius * Math.sin(angle))}`;
  }
  return `${d}Z`;
}

/** A kite-shaped shard pointing away from (cx, cy). */
function shardPath(cx: number, cy: number, degrees: number, distance: number, length: number, width: number): string {
  const ux = Math.cos(degrees * DEGREES);
  const uy = Math.sin(degrees * DEGREES);
  const at = (along: number, side: number): string => pt(cx + ux * along - uy * side, cy + uy * along + ux * side);
  const shoulder = distance + length * 0.35;
  return `M${at(distance, 0)}L${at(shoulder, width)}L${at(distance + length, 0)}L${at(shoulder, -width)}Z`;
}

/** A small triangle on a ring, pointing outward. */
function tickPath(cx: number, cy: number, degrees: number, from: number, to: number, halfWidth: number): string {
  const ux = Math.cos(degrees * DEGREES);
  const uy = Math.sin(degrees * DEGREES);
  const at = (along: number, side: number): string => pt(cx + ux * along - uy * side, cy + uy * along + ux * side);
  return `M${at(from, -halfWidth)}L${at(to, 0)}L${at(from, halfWidth)}Z`;
}

/** What a figure wears on its head. Drawn as its own layer, so it can overlap the head freely. */
type Headgear = "bare" | "hood" | "crown" | "helm" | "ears" | "hat" | "antennae" | "sprout" | "horns" | "brim" | "blob" | "toque";
/** What its shoulders look like. */
type Build = "plain" | "pauldrons" | "collar";

/**
 * Per theme, the silhouettes a Unit may take; the geometry stream picks one. A tribe's own shape
 * is listed more than once so most of its units wear it: Felinor ears, KY's pointed hat, CN's
 * antennae, a Fruit's sprout, Call to Chaos horns, a Quickdraw's wide brim, a Token's round
 * critter, a Book scholar's hood, a Pancake cook's toque, an AI's helm and antennae. Only Units are
 * figures, so the Spell and Trap themes never reach this table.
 */
const HEADGEAR: Readonly<Record<ArtThemeId, readonly Headgear[]>> = {
  human: ["bare", "hood", "crown", "helm", "helm"],
  felinor: ["ears", "ears", "ears", "hood"],
  ky: ["hat", "hat", "hood", "bare"],
  cn: ["antennae", "antennae", "helm"],
  fruit: ["sprout", "sprout", "blob"],
  chaos: ["horns", "horns", "hood"],
  quickdraw: ["brim", "brim", "bare"],
  book: ["hood", "hat", "bare"],
  pancake: ["toque", "toque", "blob"],
  ai: ["helm", "antennae", "helm"],
  token: ["blob", "blob", "bare"],
  unit: ["bare", "hood", "crown", "helm", "horns"],
  spell: ["bare", "hood", "crown"],
  "field-spell": ["bare", "hood", "crown"],
  trap: ["bare", "hood", "crown"],
  "field-trap": ["bare", "hood", "crown"],
};

const BUILDS: Readonly<Record<ArtThemeId, readonly Build[]>> = {
  human: ["plain", "pauldrons", "collar"],
  felinor: ["plain", "plain", "collar"],
  ky: ["plain", "collar"],
  cn: ["plain", "pauldrons"],
  fruit: ["plain"],
  chaos: ["collar", "pauldrons", "plain"],
  quickdraw: ["plain", "plain", "pauldrons"],
  book: ["plain", "collar"],
  pancake: ["plain"],
  ai: ["plain", "pauldrons"],
  token: ["plain"],
  unit: ["plain", "pauldrons", "collar"],
  spell: ["plain"],
  "field-spell": ["plain"],
  trap: ["plain"],
  "field-trap": ["plain"],
};

function pick<T>(rng: Rng, pool: readonly T[], fallback: T): T {
  return pool[Math.floor(rng() * pool.length)] ?? fallback;
}

/** A point on a circle, `degrees` clockwise from the +x axis (the y axis points down). */
function onCircle(cx: number, cy: number, r: number, degrees: number): string {
  return pt(cx + r * Math.cos(degrees * DEGREES), cy + r * Math.sin(degrees * DEGREES));
}

/** One horn or ear on the left; `side` -1 draws it there, +1 mirrors it to the right. */
function mirrored(draw: (side: number) => string): string {
  return draw(-1) + draw(1);
}

type Head = { cx: number; cy: number; r: number; top: number; lean: number };

/**
 * The headgear layer and the glowing detail layer (eyes, a visor, inner ears, antenna bulbs).
 * Every shape here is one closed subpath that never overlaps another in the same layer, because
 * the ridges are filled even-odd and an overlap would punch a hole.
 */
function headgear(kind: Headgear, head: Head, rng: Rng): { gear: string; detail: string } {
  const { cx, cy, r, top, lean } = head;
  const eyes =
    ellipsePath(cx - r * 0.36 + lean * 0.3, cy + r * 0.08, r * 0.15, r * 0.1) +
    ellipsePath(cx + r * 0.36 + lean * 0.3, cy + r * 0.08, r * 0.15, r * 0.1);
  switch (kind) {
    case "hood":
      // The face opening is a hole in the body path; the glow behind shows through it.
      return { gear: "", detail: "" };
    case "crown": {
      const t = top - 1.2;
      const gear =
        `M${pt(cx - 6, t)}L${pt(cx - 6.5, t - 5.5)}L${pt(cx - 3.2, t - 2.8)}L${pt(cx, t - 7)}` +
        `L${pt(cx + 3.2, t - 2.8)}L${pt(cx + 6.5, t - 5.5)}L${pt(cx + 6, t)}Z`;
      return { gear, detail: eyes };
    }
    case "helm": {
      const w = r * 1.14;
      const gear =
        `M${pt(cx - w, cy + r * 0.62)}L${pt(cx - w, cy - r * 0.05)}` +
        `A${fmt(w)} ${fmt(r * 1.18)} 0 0 1 ${pt(cx + w, cy - r * 0.05)}` +
        `L${pt(cx + w, cy + r * 0.62)}Q${pt(cx, cy + r * 1.02)} ${pt(cx - w, cy + r * 0.62)}Z`;
      const plumeTop = top - r * 0.18;
      const sweep = between(rng, 0.7, 1.1) * (rng() < 0.5 ? -1 : 1);
      const plume =
        `M${pt(cx - 1.2, plumeTop)}Q${pt(cx + sweep * 3, plumeTop - 9)} ${pt(cx + sweep * 12, plumeTop - 5)}` +
        `Q${pt(cx + sweep * 5, plumeTop - 3)} ${pt(cx + 1.2, plumeTop)}Z`;
      const visor = `M${pt(cx - r * 0.72, cy - 0.9)}L${pt(cx + r * 0.72, cy - 0.9)}L${pt(cx + r * 0.6, cy + 1)}L${pt(cx - r * 0.6, cy + 1)}Z`;
      return { gear: gear + plume, detail: visor };
    }
    case "ears": {
      const tilt = between(rng, 0, 8);
      const gear = mirrored((side) => {
        const inner = -90 + side * (18 + tilt * 0.5);
        const outer = -90 + side * (62 + tilt);
        const tip = pt(cx + side * r * 0.95, top - r * between(rng, 0.55, 0.75));
        return `M${onCircle(cx, cy, r * 0.96, inner)}L${tip}L${onCircle(cx, cy, r * 0.96, outer)}Z`;
      });
      const inner = mirrored((side) => {
        const base = onCircle(cx, cy, r * 1.02, -90 + side * 38);
        const tip = pt(cx + side * r * 0.82, top - r * 0.38);
        return `M${onCircle(cx, cy, r * 1.02, -90 + side * 26)}L${tip}L${base}Z`;
      });
      return { gear, detail: eyes + inner };
    }
    case "hat": {
      const brimY = top + r * 0.42;
      const brimW = r * 1.45;
      const tipX = cx + between(rng, -9, 9);
      const tipY = top - r * between(rng, 1.7, 2.2);
      const gear =
        `M${pt(cx - brimW, brimY)}A${fmt(brimW)} ${fmt(r * 0.3)} 0 0 0 ${pt(cx + brimW, brimY)}` +
        `L${pt(cx + r * 0.8, brimY - r * 0.28)}Q${pt(cx + r * 0.45, top - r * 0.6)} ${pt(tipX, tipY)}` +
        `Q${pt(cx - r * 0.35, top - r * 0.5)} ${pt(cx - r * 0.8, brimY - r * 0.28)}Z`;
      return { gear, detail: eyes };
    }
    case "antennae": {
      const spread = between(rng, 0.55, 0.85);
      const reach = between(rng, 9, 12);
      const stalks = mirrored((side) => {
        const base = cx + side * r * 0.3;
        const tipX = cx + side * r * (0.3 + spread);
        const tipY = top - reach;
        return (
          `M${pt(base - 0.9, top + 1.5)}Q${pt(base + side * 1.5, top - reach * 0.5)} ${pt(tipX - 0.7, tipY)}` +
          `L${pt(tipX + 0.7, tipY)}Q${pt(base + side * 2.8, top - reach * 0.45)} ${pt(base + 0.9, top + 1.5)}Z`
        );
      });
      const bulbs = mirrored((side) => circlePath(cx + side * r * (0.3 + spread), top - reach - 2, 2));
      return { gear: stalks, detail: eyes + bulbs };
    }
    case "sprout": {
      const lean2 = between(rng, -1.5, 1.5);
      const stem =
        `M${pt(cx - 0.9, top + 1)}Q${pt(cx - 1.1 + lean2, top - 3)} ${pt(cx + 0.4 + lean2, top - 6)}` +
        `L${pt(cx + 1.8 + lean2, top - 5.6)}Q${pt(cx + 0.5 + lean2, top - 3)} ${pt(cx + 0.9, top + 1)}Z`;
      const side = rng() < 0.5 ? -1 : 1;
      const leafX = cx + 1.1 + lean2;
      const leaf =
        `M${pt(leafX, top - 5)}Q${pt(leafX + side * 5, top - 12)} ${pt(leafX + side * 11, top - 9.5)}` +
        `Q${pt(leafX + side * 6, top - 3.2)} ${pt(leafX + side * 0.4, top - 4.6)}Z`;
      return { gear: stem + leaf, detail: eyes };
    }
    case "horns": {
      const curl = between(rng, 0.9, 1.3);
      const gear = mirrored((side) => {
        const rootOut = onCircle(cx, cy, r * 0.94, -90 + side * 62);
        const rootIn = onCircle(cx, cy, r * 0.94, -90 + side * 34);
        const tip = pt(cx + side * r * (1.25 + 0.2 * curl), top - r * 0.95 * curl);
        return (
          `M${rootOut}Q${pt(cx + side * r * 1.75, cy - r * 0.9)} ${tip}` +
          `Q${pt(cx + side * r * 1.05, cy - r * 1.05)} ${rootIn}Z`
        );
      });
      return { gear, detail: eyes };
    }
    case "brim": {
      const by = top + r * 0.5;
      const w = r * between(rng, 1.8, 2.1);
      const gear =
        `M${pt(cx - w, by - r * 0.3)}Q${pt(cx - w * 0.7, by + r * 0.28)} ${pt(cx, by + r * 0.18)}` +
        `Q${pt(cx + w * 0.7, by + r * 0.28)} ${pt(cx + w, by - r * 0.3)}` +
        `Q${pt(cx + w * 0.6, by - r * 0.05)} ${pt(cx + r * 0.85, by - r * 0.08)}` +
        `L${pt(cx + r * 0.72, by - r * 0.95)}Q${pt(cx, by - r * 1.25)} ${pt(cx - r * 0.72, by - r * 0.95)}` +
        `L${pt(cx - r * 0.85, by - r * 0.08)}Q${pt(cx - w * 0.6, by - r * 0.05)} ${pt(cx - w, by - r * 0.3)}Z`;
      return { gear, detail: eyes };
    }
    case "toque": {
      // A cook's hat: a band round the head and a puffed crown of three lobes on it.
      const w = r * 0.92;
      const bandTop = top - r * 0.12;
      const band = `M${pt(cx - w, bandTop)}L${pt(cx + w, bandTop)}L${pt(cx + w, top + r * 0.32)}L${pt(cx - w, top + r * 0.32)}Z`;
      const lobe = r * 0.62;
      const puff =
        `M${pt(cx - w, bandTop)}A${fmt(lobe)} ${fmt(lobe)} 0 0 1 ${pt(cx - r * 0.3, bandTop - r * 1.05)}` +
        `A${fmt(lobe)} ${fmt(lobe)} 0 0 1 ${pt(cx + r * 0.3, bandTop - r * 1.05)}` +
        `A${fmt(lobe)} ${fmt(lobe)} 0 0 1 ${pt(cx + w, bandTop)}Z`;
      return { gear: band + puff, detail: eyes };
    }
    case "blob":
    case "bare":
      return { gear: "", detail: eyes };
  }
}

/** Shoulder plates or a high collar, as a layer over the body. */
function buildLayer(build: Build, cx: number, shoulderY: number, half: number, neck: number, neckTop: number): string {
  if (build === "pauldrons") {
    return mirrored((side) => {
      const x = cx + side * half * 0.66;
      return (
        `M${pt(x - half * 0.36, shoulderY + 5)}Q${pt(x - half * 0.34, shoulderY - 5.5)} ${pt(x, shoulderY - 6)}` +
        `Q${pt(x + half * 0.34, shoulderY - 5.5)} ${pt(x + half * 0.36, shoulderY + 5)}` +
        `Q${pt(x, shoulderY + 2)} ${pt(x - half * 0.36, shoulderY + 5)}Z`
      );
    });
  }
  if (build === "collar") {
    return mirrored((side) => {
      const inner = cx + side * (neck + 1.5);
      return (
        `M${pt(inner, shoulderY - 1)}L${pt(cx + side * (neck + 6.5), neckTop - 9)}` +
        `L${pt(cx + side * (neck + 11), shoulderY - 3)}Z`
      );
    });
  }
  return "";
}

/**
 * Units: a silhouetted figure before two rows of hills. The theme picks what it wears and how it
 * stands (HEADGEAR, BUILDS), so a Felinor, a KY scholar and a Call to Chaos fiend read apart
 * before the emblem does. Everything comes from the geometry stream, never from `radiant`, so the
 * base and radiant faces share every path (B2).
 */
/** Where a figure stands for each layout: its centre line, and how close the viewer is. */
const FIGURE_STANCE: Readonly<Record<string, { from: number; to: number; scale: number; drop: number }>> = {
  centre: { from: 44, to: 56, scale: 1, drop: 0 },
  left: { from: 29, to: 37, scale: 0.92, drop: 0 },
  right: { from: 63, to: 71, scale: 0.92, drop: 0 },
  close: { from: 46, to: 54, scale: 1.3, drop: 7 },
};

function figure(rng: Rng, theme: ArtThemeId, layout: string, wears?: MotifHeadgear): Geometry {
  const stance = FIGURE_STANCE[layout] ?? { from: 44, to: 56, scale: 1, drop: 0 };
  const far = hills(rng, between(rng, 56, 64), 7, 5);
  const near = hills(rng, between(rng, 80, 86), 4, 4);
  const cx = between(rng, stance.from, stance.to);
  const shoulderY = between(rng, 66, 72) + stance.drop;
  const neck = between(rng, 4.2, 5.6) * stance.scale;
  const neckTop = shoulderY - between(rng, 5, 7.5) * stance.scale;
  const lean = between(rng, -3, 3);
  // A motif may put something on the figure's head (motifs.ts); the pick still draws, so the rest of
  // the stream reads as it did.
  const picked = pick(rng, HEADGEAR[theme], "bare");
  const kind: Headgear = wears ?? picked;
  const build = pick(rng, BUILDS[theme], "plain");
  const half = (kind === "blob" ? between(rng, 30, 36) : between(rng, 28, 36)) * stance.scale;
  const headR = (kind === "blob" ? between(rng, 12.5, 14.5) : between(rng, 10.5, 13.5)) * stance.scale;
  const headCy = neckTop - Math.sqrt(headR * headR - neck * neck);
  const headTop = headCy - headR;

  let body: string;
  let face = "";
  if (kind === "blob") {
    // A round critter: one mound from the ground up, no neck.
    const crown = headTop + 2;
    body =
      `M${pt(cx - half, ART_BOX)}C${pt(cx - half, shoulderY - 6)} ${pt(cx - half * 0.55, crown)} ${pt(cx, crown)}` +
      `C${pt(cx + half * 0.55, crown)} ${pt(cx + half, shoulderY - 6)} ${pt(cx + half, ART_BOX)}Z`;
  } else {
    let head: string;
    if (kind === "hood") {
      // A peaked hood, with the face left open so the glow shows through.
      head =
        `C${pt(cx - headR - 2.5, neckTop - headR * 0.9)} ${pt(cx - headR * 0.7 + lean, headTop - 1)} ${pt(cx + lean * 1.6, headTop - 5)}` +
        `C${pt(cx + headR * 0.7 + lean, headTop - 1)} ${pt(cx + headR + 2.5, neckTop - headR * 0.9)} ${pt(cx + neck, neckTop)}`;
      face = ellipsePath(cx + lean * 0.5, headCy + 1.5, headR * 0.55, headR * 0.7);
    } else {
      head = `A${fmt(headR)} ${fmt(headR)} 0 1 1 ${pt(cx + neck, neckTop)}`;
    }
    body =
      `M${pt(cx - half, ART_BOX)}C${pt(cx - half, shoulderY + 5)} ${pt(cx - half * 0.72, shoulderY - 1)} ${pt(cx - neck - 3.5, shoulderY - 2.5)}` +
      `L${pt(cx - neck, neckTop)}${head}L${pt(cx + neck + 3.5, shoulderY - 2.5)}` +
      `C${pt(cx + half * 0.72, shoulderY - 1)} ${pt(cx + half, shoulderY + 5)} ${pt(cx + half, ART_BOX)}Z${face}`;
  }

  const head: Head = { cx, cy: kind === "blob" ? headTop + headR * 1.1 : headCy, r: headR, top: headTop, lean };
  const { gear, detail } = headgear(kind, head, rng);
  const shoulders = kind === "blob" ? "" : buildLayer(build, cx, shoulderY, half, neck, neckTop);

  const glowR = between(rng, 24, 32);
  const glowOpacity = between(rng, 0.7, 0.9);
  const emblemY = shoulderY + between(rng, 8, 11);
  const emblemSize = between(rng, 12.5, 15.5);
  const emblemTurn = between(rng, -8, 8);
  // A figure off to one side leaves the other half of the sky to its emblem.
  const aside = layout === "left" || layout === "right";
  const emblem = aside
    ? { x: cx < ART_BOX / 2 ? between(rng, 70, 78) : between(rng, 22, 30), y: between(rng, 22, 32), size: emblemSize + 3, rotate: emblemTurn }
    : { x: cx, y: Math.min(emblemY, ART_BOX - emblemSize / 2 - 1), size: emblemSize, rotate: emblemTurn };
  const layers: Layer[] = [
    { d: far, fill: "far", opacity: 0.6 },
    { d: near, fill: "mid", opacity: 0.85 },
    { d: body, fill: "near", opacity: 1 },
  ];
  if (shoulders !== "") layers.push({ d: shoulders, fill: "near", opacity: 1 });
  if (gear !== "") layers.push({ d: gear, fill: "near", opacity: 1 });
  if (detail !== "") layers.push({ d: detail, fill: "glow", opacity: 0.9 });
  return {
    layers,
    glow: { cx, cy: headCy, r: glowR, opacity: glowOpacity },
    emblem,
    motes: { top: 4, bottom: 58 },
  };
}

/** Spells: a halo, two starbursts and flying shards around the emblem. */
function starburst(rng: Rng): Geometry {
  const cx = between(rng, 44, 56);
  const cy = between(rng, 44, 54);
  const points = whole(rng, 7, 12);
  const offset = rng() * FULL_TURN;
  const haloOuter = between(rng, 42, 47);
  const haloInner = between(rng, 38, 41);
  const halo = circlePath(cx, cy, haloOuter) + circlePath(cx, cy, haloInner);
  const outerTip = between(rng, 34, 42);
  const outerWaist = between(rng, 13, 17);
  const outer = starPath(rng, cx, cy, points, outerTip, outerWaist, offset, 4);
  const innerTip = between(rng, 20, 27);
  const innerWaist = between(rng, 8, 11);
  const inner = starPath(rng, cx, cy, points, innerTip, innerWaist, offset + FULL_TURN / (points * 2), 2);
  const shardCount = whole(rng, 3, 5);
  const shardOffset = rng() * FULL_TURN;
  let shards = "";
  for (let k = 0; k < shardCount; k += 1) {
    const angle = shardOffset + (FULL_TURN / shardCount) * k + between(rng, -12, 12);
    const distance = between(rng, 30, 40);
    const length = between(rng, 5, 9);
    const width = between(rng, 1.4, 2.4);
    shards += shardPath(cx, cy, angle, distance, length, width);
  }
  const glowR = between(rng, 28, 36);
  const glowOpacity = between(rng, 0.8, 1);
  const emblemSize = between(rng, 20, 26);
  const emblemTurn = between(rng, -15, 15);
  return {
    layers: [
      { d: halo, fill: "far", opacity: 0.5 },
      { d: outer, fill: "glow", opacity: 0.3 },
      { d: inner, fill: "glow", opacity: 0.5 },
      { d: shards, fill: "accent", opacity: 0.8 },
    ],
    glow: { cx, cy, r: glowR, opacity: glowOpacity },
    emblem: { x: cx, y: cy, size: emblemSize, rotate: emblemTurn },
    motes: { top: 4, bottom: 96 },
  };
}

/** A spiral arm from radius `r0` to `r1`, turning `sweep` degrees, tapering from `w0` wide to a point. */
function armPath(cx: number, cy: number, start: number, sweep: number, r0: number, r1: number, w0: number): string {
  const STEPS = 8;
  const outer: string[] = [];
  const inner: string[] = [];
  for (let i = 0; i <= STEPS; i += 1) {
    const t = i / STEPS;
    const r = r0 + (r1 - r0) * t;
    const half = (w0 * (1 - t)) / 2 + 0.2;
    const angle = start + sweep * t;
    outer.push(onCircle(cx, cy, r + half, angle));
    inner.push(onCircle(cx, cy, Math.max(0.5, r - half), angle));
  }
  return `M${outer.join("L")}L${inner.reverse().join("L")}Z`;
}

/** An ellipse turned `degrees` about its centre, as one closed path. */
function turnedEllipsePath(cx: number, cy: number, rx: number, ry: number, degrees: number): string {
  const ux = Math.cos(degrees * DEGREES);
  const uy = Math.sin(degrees * DEGREES);
  const a = pt(cx - rx * ux, cy - rx * uy);
  const b = pt(cx + rx * ux, cy + rx * uy);
  const arc = `${fmt(rx)} ${fmt(ry)} ${fmt(degrees)}`;
  return `M${a}A${arc} 1 0 ${b}A${arc} 1 0 ${a}Z`;
}

/** Spells, as a spiral: tapering arms wheeling out from the emblem inside a thin halo. */
function spiral(rng: Rng): Geometry {
  const cx = between(rng, 42, 58);
  const cy = between(rng, 42, 56);
  const arms = whole(rng, 3, 6);
  const start = rng() * FULL_TURN;
  const sweep = between(rng, 70, 120) * (rng() < 0.5 ? -1 : 1);
  const reach = between(rng, 38, 46);
  let blades = "";
  for (let k = 0; k < arms; k += 1) {
    blades += armPath(cx, cy, start + (FULL_TURN / arms) * k, sweep, 7, reach, between(rng, 5, 7));
  }
  const halo = circlePath(cx, cy, reach + 3) + circlePath(cx, cy, reach + 1.6);
  return {
    layers: [
      { d: halo, fill: "far", opacity: 0.45 },
      { d: blades, fill: "glow", opacity: 0.45 },
    ],
    glow: { cx, cy, r: between(rng, 26, 34), opacity: between(rng, 0.8, 1) },
    emblem: { x: cx, y: cy, size: between(rng, 17, 22), rotate: between(rng, -15, 15) },
    motes: { top: 4, bottom: 96 },
  };
}

/** Spells, as an orbit: a planet under the emblem, tilted rings around it, and orbs riding them. */
function orbit(rng: Rng): Geometry {
  const cx = between(rng, 44, 56);
  const cy = between(rng, 44, 56);
  const planet = between(rng, 12, 15);
  const count = whole(rng, 2, 3);
  const tilt = rng() * FULL_TURN;
  const layers: Layer[] = [{ d: circlePath(cx, cy, planet), fill: "far", opacity: 0.7 }];
  let orbs = "";
  for (let k = 0; k < count; k += 1) {
    const rx = between(rng, 34, 44) - k * 3;
    const ry = between(rng, 9, 15);
    const turn = tilt + (FULL_TURN / 2 / count) * k;
    layers.push({
      d: turnedEllipsePath(cx, cy, rx, ry, turn) + turnedEllipsePath(cx, cy, rx - 1.5, ry - 1.5, turn),
      fill: "accent",
      opacity: 0.55,
    });
    // An orb on the ring's far end, and a smaller one partway round.
    const along = rng() * FULL_TURN * DEGREES;
    const ox = rx * Math.cos(along);
    const oy = ry * Math.sin(along);
    const tx = cx + ox * Math.cos(turn * DEGREES) - oy * Math.sin(turn * DEGREES);
    const ty = cy + ox * Math.sin(turn * DEGREES) + oy * Math.cos(turn * DEGREES);
    orbs += circlePath(tx, ty, between(rng, 2.2, 3.6));
  }
  layers.push({ d: orbs, fill: "glow", opacity: 0.85 });
  return {
    layers,
    glow: { cx, cy, r: between(rng, 24, 30), opacity: between(rng, 0.75, 0.95) },
    emblem: { x: cx, y: cy, size: between(rng, 15, 19), rotate: between(rng, -12, 12) },
    motes: { top: 4, bottom: 96 },
  };
}

/** Spells, as a shatter: long shards flying out from a point off the centre. */
function shatter(rng: Rng): Geometry {
  const cx = between(rng, 32, 68);
  const cy = between(rng, 36, 60);
  const count = whole(rng, 7, 11);
  const offset = rng() * FULL_TURN;
  let long = "";
  let short = "";
  for (let k = 0; k < count; k += 1) {
    const angle = offset + (FULL_TURN / count) * k + between(rng, -8, 8);
    long += shardPath(cx, cy, angle, between(rng, 8, 13), between(rng, 20, 38), between(rng, 2.4, 4.6));
    const between2 = angle + FULL_TURN / count / 2;
    short += shardPath(cx, cy, between2, between(rng, 20, 28), between(rng, 5, 9), between(rng, 1, 1.8));
  }
  return {
    layers: [
      { d: long, fill: "glow", opacity: 0.4 },
      { d: short, fill: "accent", opacity: 0.8 },
    ],
    glow: { cx, cy, r: between(rng, 22, 30), opacity: between(rng, 0.85, 1) },
    emblem: { x: cx, y: cy, size: between(rng, 16, 21), rotate: between(rng, -20, 20) },
    motes: { top: 4, bottom: 96 },
  };
}

function burst(rng: Rng, _theme: ArtThemeId, layout: string): Geometry {
  if (layout === "spiral") return spiral(rng);
  if (layout === "orbit") return orbit(rng);
  if (layout === "shatter") return shatter(rng);
  return starburst(rng);
}

/** A closed skyline from x 0 to 100: towers of varied height, some with a spire. */
function skyline(rng: Rng, base: number, low: number, high: number, spires: boolean): string {
  let d = `M0 ${ART_BOX}L0 ${fmt(base)}`;
  let x = 0;
  while (x < ART_BOX) {
    const width = between(rng, 7, 14);
    const top = between(rng, low, high);
    const end = Math.min(ART_BOX, x + width);
    d += `L${pt(x, top)}`;
    if (spires && rng() < 0.4) {
      const mid = (x + end) / 2;
      d += `L${pt(mid - 1.2, top)}L${pt(mid, top - between(rng, 5, 9))}L${pt(mid + 1.2, top)}`;
    }
    d += `L${pt(end, top)}`;
    x = end;
  }
  return `${d}L${ART_BOX} ${ART_BOX}Z`;
}

/** Field Spells, as a sea: a horizon, a sun low over it, its reflection, and rows of waves. */
function sea(rng: Rng): Geometry {
  const horizon = between(rng, 54, 62);
  const sunX = between(rng, 24, 76);
  const sunY = horizon - between(rng, 10, 20);
  const water = `M0 ${fmt(horizon)}L${ART_BOX} ${fmt(horizon)}L${ART_BOX} ${ART_BOX}L0 ${ART_BOX}Z`;
  const islandFrom = rng() < 0.5 ? between(rng, 0, 20) : between(rng, 55, 75);
  const island =
    `M${pt(islandFrom, horizon)}Q${pt(islandFrom + 6, horizon - between(rng, 5, 9))} ${pt(islandFrom + 12, horizon - 2)}` +
    `Q${pt(islandFrom + 18, horizon - between(rng, 3, 6))} ${pt(islandFrom + 26, horizon)}Z`;
  let waves = "";
  for (let k = 0; k < 4; k += 1) {
    const y = horizon + 8 + k * between(rng, 7, 9);
    const lift = between(rng, 1.2, 2.4);
    let d = `M${pt(0, y)}`;
    for (let x = 0; x < ART_BOX; x += 10) d += `Q${pt(x + 5, y - lift)} ${pt(x + 10, y)}`;
    d += `L${pt(ART_BOX, y + 1.1)}`;
    for (let x = ART_BOX; x > 0; x -= 10) d += `Q${pt(x - 5, y - lift + 1.6)} ${pt(x - 10, y + 1.1)}`;
    waves += `${d}Z`;
  }
  let glint = "";
  for (let k = 0; k < 5; k += 1) {
    const y = horizon + 3 + k * 6;
    const w = between(rng, 6, 12) * (1 - k * 0.12);
    glint += `M${pt(sunX - w / 2, y)}L${pt(sunX + w / 2, y)}L${pt(sunX + w / 2, y + 1.2)}L${pt(sunX - w / 2, y + 1.2)}Z`;
  }
  return {
    layers: [
      { d: water, fill: "mid", opacity: 0.85 },
      { d: island, fill: "near", opacity: 0.85 },
      { d: waves, fill: "far", opacity: 0.55 },
      { d: glint, fill: "glow", opacity: 0.5 },
    ],
    glow: { cx: sunX, cy: sunY, r: between(rng, 18, 26), opacity: between(rng, 0.8, 0.95) },
    emblem: { x: sunX, y: sunY, size: between(rng, 13, 17), rotate: between(rng, -6, 6) },
    motes: { top: 4, bottom: horizon - 4 },
  };
}

/** Field Spells, as towers: a far skyline with spires, a near one, and lit windows. */
function towers(rng: Rng): Geometry {
  const far = skyline(rng, 62, 38, 56, true);
  const near = skyline(rng, 80, 64, 78, false);
  let windows = "";
  for (let k = 0; k < 14; k += 1) {
    const x = between(rng, 3, 95);
    const y = between(rng, 82, 95);
    windows += `M${pt(x, y)}L${pt(x + 1.6, y)}L${pt(x + 1.6, y + 2)}L${pt(x, y + 2)}Z`;
  }
  const moonX = between(rng, 18, 82);
  const moonY = between(rng, 16, 28);
  return {
    layers: [
      { d: far, fill: "far", opacity: 0.8 },
      { d: near, fill: "near", opacity: 1 },
      { d: windows, fill: "glow", opacity: 0.75 },
    ],
    glow: { cx: moonX, cy: moonY, r: between(rng, 16, 22), opacity: between(rng, 0.8, 0.95) },
    emblem: { x: moonX, y: moonY, size: between(rng, 13, 17), rotate: between(rng, -6, 6) },
    motes: { top: 4, bottom: 40 },
  };
}

/** Field Spells, as dunes: long smooth ridges under a large, low sun. */
function dunes(rng: Rng): Geometry {
  const sunX = between(rng, 26, 74);
  const sunY = between(rng, 40, 50);
  return {
    layers: [
      { d: hills(rng, between(rng, 58, 64), 5, 2), fill: "far", opacity: 0.8 },
      { d: hills(rng, between(rng, 70, 76), 6, 3), fill: "mid", opacity: 0.9 },
      { d: hills(rng, between(rng, 84, 90), 4, 2), fill: "near", opacity: 1 },
    ],
    glow: { cx: sunX, cy: sunY, r: between(rng, 26, 34), opacity: between(rng, 0.85, 1) },
    emblem: { x: sunX, y: sunY - 4, size: between(rng, 15, 19), rotate: between(rng, -6, 6) },
    motes: { top: 4, bottom: 50 },
  };
}

/** Field Spells: a sun in the sky over a mountain range and two rows of hills. */
function range(rng: Rng): Geometry {
  const sunX = between(rng, 22, 78);
  const sunY = between(rng, 22, 34);
  const rangeBase = between(rng, 54, 60);
  const rangeHeight = between(rng, 12, 18);
  const peaks = whole(rng, 6, 8);
  const far = mountains(rng, rangeBase, rangeHeight, peaks);
  const mid = hills(rng, between(rng, 67, 73), 6, 4);
  const near = hills(rng, between(rng, 83, 88), 3.5, 3);
  const glowR = between(rng, 18, 26);
  const glowOpacity = between(rng, 0.8, 0.95);
  const emblemSize = between(rng, 14, 18);
  const emblemTurn = between(rng, -6, 6);
  return {
    layers: [
      { d: far, fill: "far", opacity: 0.75 },
      { d: mid, fill: "mid", opacity: 0.9 },
      { d: near, fill: "near", opacity: 1 },
    ],
    glow: { cx: sunX, cy: sunY, r: glowR, opacity: glowOpacity },
    emblem: { x: sunX, y: sunY, size: emblemSize, rotate: emblemTurn },
    motes: { top: 4, bottom: 46 },
  };
}

function landscape(rng: Rng, _theme: ArtThemeId, layout: string): Geometry {
  if (layout === "sea") return sea(rng);
  if (layout === "towers") return towers(rng);
  if (layout === "dunes") return dunes(rng);
  return range(rng);
}

/** Traps, as a diamond: two nested square frames on their points, and four ticks outside. */
function diamond(rng: Rng): Geometry {
  const cx = ART_BOX / 2 + between(rng, -2, 2);
  const cy = ART_BOX / 2 + between(rng, -2, 2);
  const outer = between(rng, 34, 38);
  const inner = between(rng, 20, 24);
  const frames =
    polygonPath(cx, cy, 4, outer, 90) + polygonPath(cx, cy, 4, outer - 3, 90);
  const core = polygonPath(cx, cy, 4, inner, 90) + polygonPath(cx, cy, 4, inner - 2.2, 90);
  let ticks = "";
  for (let k = 0; k < 4; k += 1) ticks += tickPath(cx, cy, 45 + 90 * k, outer * 0.62, outer * 0.62 + 7, 2.2);
  return {
    layers: [
      { d: frames, fill: "far", opacity: 0.9 },
      { d: core, fill: "accent", opacity: 0.6 },
      { d: ticks, fill: "glow", opacity: 0.65 },
    ],
    glow: { cx, cy, r: between(rng, 22, 28), opacity: between(rng, 0.6, 0.8) },
    emblem: { x: cx, y: cy, size: between(rng, 15, 19), rotate: between(rng, -10, 10) },
    motes: { top: 4, bottom: 96 },
  };
}

/** Traps, as an eye: an almond frame, an iris ring around the emblem, and lashes above. */
function eyeSigil(rng: Rng): Geometry {
  const cx = ART_BOX / 2 + between(rng, -2, 2);
  const cy = ART_BOX / 2 + between(rng, -1, 3);
  const w = between(rng, 40, 46);
  const h = between(rng, 20, 26);
  const almond = (half: number, lift: number): string =>
    `M${pt(cx - half, cy)}Q${pt(cx, cy - lift * 2)} ${pt(cx + half, cy)}Q${pt(cx, cy + lift * 2)} ${pt(cx - half, cy)}Z`;
  const frame = almond(w, h) + almond(w - 4, h - 3.2);
  const iris = between(rng, 13, 16);
  const ring = circlePath(cx, cy, iris) + circlePath(cx, cy, iris - 2);
  let lashes = "";
  const count = whole(rng, 5, 7);
  for (let k = 0; k < count; k += 1) {
    const angle = -150 + (120 / (count - 1)) * k;
    lashes += tickPath(cx, cy + h * 0.9, angle, h * 1.55, h * 1.55 + between(rng, 5, 8), 1.1);
  }
  return {
    layers: [
      { d: frame, fill: "far", opacity: 0.9 },
      { d: ring, fill: "glow", opacity: 0.6 },
      { d: lashes, fill: "accent", opacity: 0.6 },
    ],
    glow: { cx, cy, r: between(rng, 22, 28), opacity: between(rng, 0.6, 0.8) },
    emblem: { x: cx, y: cy, size: between(rng, 14, 17), rotate: between(rng, -8, 8) },
    motes: { top: 4, bottom: 96 },
  };
}

/** Traps, as a wheel: a rim, spokes and a hub around the emblem. */
function wheel(rng: Rng): Geometry {
  const cx = ART_BOX / 2 + between(rng, -2, 2);
  const cy = ART_BOX / 2 + between(rng, -2, 2);
  const rim = between(rng, 35, 39);
  const hub = between(rng, 12, 14);
  const spokes = whole(rng, 6, 10);
  const turn = rng() * FULL_TURN;
  let d = "";
  for (let k = 0; k < spokes; k += 1) {
    const angle = turn + (FULL_TURN / spokes) * k;
    const ux = Math.cos(angle * DEGREES);
    const uy = Math.sin(angle * DEGREES);
    const w = 0.9;
    const at = (along: number, side: number): string => pt(cx + ux * along - uy * side, cy + uy * along + ux * side);
    d += `M${at(hub + 0.6, -w)}L${at(rim - 3.6, -w * 1.6)}L${at(rim - 3.6, w * 1.6)}L${at(hub + 0.6, w)}Z`;
  }
  return {
    layers: [
      { d: circlePath(cx, cy, rim) + circlePath(cx, cy, rim - 3), fill: "far", opacity: 0.9 },
      { d, fill: "accent", opacity: 0.55 },
      { d: circlePath(cx, cy, hub) + circlePath(cx, cy, hub - 1.8), fill: "glow", opacity: 0.65 },
    ],
    glow: { cx, cy, r: between(rng, 24, 30), opacity: between(rng, 0.6, 0.8) },
    emblem: { x: cx, y: cy, size: between(rng, 15, 19), rotate: between(rng, -10, 10) },
    motes: { top: 4, bottom: 96 },
  };
}

/** Traps and Field Traps: a ticked rim, a polygon frame and an inner ring around the emblem. */
function ringSigil(rng: Rng): Geometry {
  const cx = ART_BOX / 2 + between(rng, -2, 2);
  const cy = ART_BOX / 2 + between(rng, -2, 2);
  const outer = between(rng, 34, 38);
  const band = between(rng, 2.6, 3.6);
  const ticks = 2 * whole(rng, 4, 8);
  const tickOffset = rng() * FULL_TURN;
  let rim = circlePath(cx, cy, outer) + circlePath(cx, cy, outer - band);
  for (let k = 0; k < ticks; k += 1) {
    rim += tickPath(cx, cy, tickOffset + (FULL_TURN / ticks) * k, outer + 1.5, outer + 5.5, 1.3);
  }
  const sides = whole(rng, 3, 6);
  const frameR = between(rng, 24, 29);
  const frameTurn = rng() * FULL_TURN;
  const frame = polygonPath(cx, cy, sides, frameR, frameTurn) + polygonPath(cx, cy, sides, frameR - 2.6, frameTurn);
  const innerR = between(rng, 15, 18);
  const inner = circlePath(cx, cy, innerR) + circlePath(cx, cy, innerR - 1.4);
  const glowR = between(rng, 24, 30);
  const glowOpacity = between(rng, 0.6, 0.8);
  const emblemSize = between(rng, 17, 21);
  const emblemTurn = between(rng, -10, 10);
  return {
    layers: [
      { d: rim, fill: "far", opacity: 0.9 },
      { d: frame, fill: "accent", opacity: 0.55 },
      { d: inner, fill: "glow", opacity: 0.6 },
    ],
    glow: { cx, cy, r: glowR, opacity: glowOpacity },
    emblem: { x: cx, y: cy, size: emblemSize, rotate: emblemTurn },
    motes: { top: 4, bottom: 96 },
  };
}

function sigil(rng: Rng, _theme: ArtThemeId, layout: string): Geometry {
  if (layout === "diamond") return diamond(rng);
  if (layout === "eye") return eyeSigil(rng);
  if (layout === "wheel") return wheel(rng);
  return ringSigil(rng);
}

/** An axis-aligned rectangle. */
function rectPath(x: number, y: number, w: number, h: number): string {
  return `M${pt(x, y)}L${pt(x + w, y)}L${pt(x + w, y + h)}L${pt(x, y + h)}Z`;
}

/** A crescent from (cx, cy - r) to (cx, cy + r), bulging left (side -1) or right (side 1). */
function crescentPath(cx: number, cy: number, r: number, side: number): string {
  const outer = side < 0 ? 0 : 1;
  const inner = side < 0 ? 1 : 0;
  return (
    `M${pt(cx, cy - r)}A${fmt(r)} ${fmt(r)} 0 0 ${outer} ${pt(cx, cy + r)}` +
    `A${fmt(r * 0.55)} ${fmt(r)} 0 0 ${inner} ${pt(cx, cy - r)}Z`
  );
}

/** A tall arch: a rectangle with a round top, as a frame (the inside cut out). */
function archFrame(x: number, top: number, w: number, thick: number): string {
  const r = w / 2;
  const shape = (inset: number): string =>
    `M${pt(x + inset, ART_BOX)}L${pt(x + inset, top + r)}A${fmt(r - inset)} ${fmt(r - inset)} 0 0 1 ${pt(x + w - inset, top + r)}` +
    `L${pt(x + w - inset, ART_BOX)}Z`;
  return shape(0) + shape(thick);
}

/**
 * The motif behind a composition, drawn first and faint. Each path is one layer whose subpaths never
 * overlap, because the layers are filled even-odd.
 */
function backdropLayers(rng: Rng, backdrop: Backdrop): Layer[] {
  switch (backdrop) {
    case "none":
      return [];
    case "moon": {
      const cx = between(rng, 16, 84);
      const cy = between(rng, 12, 26);
      const r = between(rng, 7, 11);
      const d = rng() < 0.6 ? crescentPath(cx, cy, r, rng() < 0.5 ? -1 : 1) : circlePath(cx, cy, r * 0.8);
      return [{ d, fill: "glow", opacity: 0.6 }];
    }
    case "rings": {
      const cx = between(rng, 20, 80);
      const cy = between(rng, 18, 42);
      const first = between(rng, 16, 22);
      const gap = between(rng, 9, 13);
      let d = "";
      for (let k = 2; k >= 0; k -= 1) {
        const r = first + gap * k;
        d += circlePath(cx, cy, r) + circlePath(cx, cy, r - between(rng, 1.1, 1.8));
      }
      return [{ d, fill: "far", opacity: 0.4 }];
    }
    case "pillars": {
      const count = whole(rng, 3, 5);
      const slot = ART_BOX / count;
      let d = "";
      for (let k = 0; k < count; k += 1) {
        const w = between(rng, 4.5, 7);
        const x = slot * k + (slot - w) / 2 + between(rng, -2, 2);
        const top = between(rng, 16, 38);
        d += rectPath(x - 1.4, top - 2.6, w + 2.8, 2.6) + rectPath(x, top, w, ART_BOX - top);
      }
      return [{ d, fill: "far", opacity: 0.42 }];
    }
    case "beams": {
      const fromLeft = rng() < 0.5;
      const ox = fromLeft ? -6 : ART_BOX + 6;
      const oy = -6;
      const count = whole(rng, 3, 5);
      const start = fromLeft ? between(rng, 25, 40) : between(rng, 140, 155);
      const step = (fromLeft ? 1 : -1) * between(rng, 11, 15);
      let d = "";
      for (let k = 0; k < count; k += 1) {
        const angle = start + step * k;
        const width = between(rng, 2.5, 5);
        const far = 170;
        d += `M${pt(ox, oy)}L${onCircle(ox, oy, far, angle - width / 2)}L${onCircle(ox, oy, far, angle + width / 2)}Z`;
      }
      return [{ d, fill: "glow", opacity: 0.17 }];
    }
    case "clouds": {
      let d = "";
      for (const [from, to] of [
        [between(rng, -6, 6), between(rng, 30, 42)],
        [between(rng, 56, 64), between(rng, 92, 106)],
      ] as const) {
        const y = between(rng, 16, 34);
        const bumps = whole(rng, 3, 4);
        const step = (to - from) / bumps;
        d += `M${pt(from, y)}`;
        for (let k = 1; k <= bumps; k += 1) {
          const r = (step / 2) * between(rng, 1, 1.25);
          d += `A${fmt(r)} ${fmt(r)} 0 0 1 ${pt(from + step * k, y)}`;
        }
        d += `L${pt(to, y + 3)}L${pt(from, y + 3)}Z`;
      }
      return [{ d, fill: "far", opacity: 0.5 }];
    }
    case "stars": {
      const count = whole(rng, 5, 7);
      const points: [number, number][] = [];
      for (let k = 0; k < count; k += 1) {
        points.push([ART_BOX * ((k + 0.5) / count) + between(rng, -5, 5), between(rng, 7, 40)]);
      }
      let stars = "";
      for (const [x, y] of points) stars += starPath(rng, x, y, 4, between(rng, 1.8, 3), 0.6, 45, 0.3);
      let lines = "";
      for (let k = 1; k < points.length; k += 1) {
        const [ax, ay] = points[k - 1] ?? [0, 0];
        const [bx, by] = points[k] ?? [0, 0];
        const length = Math.hypot(bx - ax, by - ay);
        const angle = (Math.atan2(by - ay, bx - ax) * 180) / Math.PI;
        lines += shardPath(ax, ay, angle, 3.2, Math.max(0.5, length - 6.4), 0.25);
      }
      return [
        { d: lines, fill: "glow", opacity: 0.3 },
        { d: stars, fill: "glow", opacity: 0.85 },
      ];
    }
    case "arches": {
      const count = whole(rng, 2, 3);
      const slot = ART_BOX / count;
      let d = "";
      for (let k = 0; k < count; k += 1) {
        const w = Math.min(slot - 4, between(rng, 16, 24));
        const x = slot * k + (slot - w) / 2;
        d += archFrame(x, between(rng, 12, 30), w, between(rng, 1.4, 2.2));
      }
      return [{ d, fill: "far", opacity: 0.4 }];
    }
    case "waves": {
      let d = "";
      const first = between(rng, 22, 30);
      for (let k = 0; k < 3; k += 1) {
        const y = first + k * between(rng, 11, 15);
        const lift = between(rng, 2, 4);
        const thick = between(rng, 1.4, 2.4);
        let top = `M${pt(0, y)}`;
        for (let x = 0; x < ART_BOX; x += 20) top += `Q${pt(x + 5, y - lift)} ${pt(x + 10, y)}Q${pt(x + 15, y + lift)} ${pt(x + 20, y)}`;
        let bottom = `L${pt(ART_BOX, y + thick)}`;
        for (let x = ART_BOX; x > 0; x -= 20) bottom += `Q${pt(x - 5, y + thick + lift)} ${pt(x - 10, y + thick)}Q${pt(x - 15, y + thick - lift)} ${pt(x - 20, y + thick)}`;
        d += `${top}${bottom}Z`;
      }
      return [{ d, fill: "mid", opacity: 0.38 }];
    }
    case "grid": {
      const vx = between(rng, 38, 62);
      const vy = between(rng, 48, 56);
      let rays = "";
      for (let k = -5; k <= 5; k += 1) {
        const x = vx + k * between(rng, 17, 20);
        rays += `M${pt(vx - 0.15, vy)}L${pt(x - 0.7, ART_BOX)}L${pt(x + 0.7, ART_BOX)}L${pt(vx + 0.15, vy)}Z`;
      }
      let rows = "";
      let y = vy + 3;
      let gap = 2.2;
      while (y < ART_BOX) {
        rows += rectPath(0, y, ART_BOX, 0.5 + gap * 0.08);
        y += gap;
        gap *= 1.55;
      }
      return [
        { d: rays, fill: "glow", opacity: 0.2 },
        { d: rows, fill: "glow", opacity: 0.14 },
      ];
    }
  }
}

const COMPOSERS: Readonly<Record<Composition, (rng: Rng, theme: ArtThemeId, layout: string, wears?: MotifHeadgear) => Geometry>> = {
  figure,
  burst,
  landscape,
  sigil,
};

/* ------------------------------------------------------------------ R503: twists and motifs --- */

/** A thin band along a polyline, as one closed path. */
function bandPath(points: readonly (readonly [number, number])[], width: number): string {
  const left: string[] = [];
  const right: string[] = [];
  for (let i = 0; i < points.length; i += 1) {
    const [px, py] = points[Math.max(0, i - 1)] ?? [0, 0];
    const [nx, ny] = points[Math.min(points.length - 1, i + 1)] ?? [0, 0];
    const [x, y] = points[i] ?? [0, 0];
    const length = Math.hypot(nx - px, ny - py) || 1;
    const ox = (-(ny - py) / length) * (width / 2);
    const oy = ((nx - px) / length) * (width / 2);
    left.push(pt(x + ox, y + oy));
    right.push(pt(x - ox, y - oy));
  }
  return `M${left.join("L")}L${right.reverse().join("L")}Z`;
}

/**
 * Circuit traces: runs that leave an edge of the box, travel straight, take one 45° bend and end in
 * a ringed pad. The AI family's backdrop, and the `circuits` motif's pattern.
 */
function circuitTraces(rng: Rng): string {
  const count = whole(rng, 6, 8);
  let d = "";
  for (let k = 0; k < count; k += 1) {
    const side = k % 4;
    const along = between(rng, 10, 90);
    const reach = between(rng, 14, 30);
    const bend = between(rng, 6, 14) * (rng() < 0.5 ? -1 : 1);
    // Out from the edge, then a diagonal of `bend` each way.
    const inward: readonly (readonly [number, number])[] =
      side === 0 ? [[along, -1], [along, reach], [along + bend, reach + Math.abs(bend)]]
        : side === 1 ? [[ART_BOX + 1, along], [ART_BOX - reach, along], [ART_BOX - reach - Math.abs(bend), along + bend]]
          : side === 2 ? [[along, ART_BOX + 1], [along, ART_BOX - reach], [along + bend, ART_BOX - reach - Math.abs(bend)]]
            : [[-1, along], [reach, along], [reach + Math.abs(bend), along + bend]];
    const [ex, ey] = inward[2] ?? [0, 0];
    const [bx, by] = inward[1] ?? [0, 0];
    // Stop the run at the pad's rim, so the two never overlap (layers fill even-odd).
    const toPad = Math.hypot(ex - bx, ey - by) || 1;
    const stop: readonly [number, number] = [ex - ((ex - bx) / toPad) * 2.2, ey - ((ey - by) / toPad) * 2.2];
    d += bandPath([inward[0] ?? [0, 0], inward[1] ?? [0, 0], stop], 1.1);
    d += circlePath(ex, ey, 2.2) + circlePath(ex, ey, 1);
  }
  return d;
}

/** Waves across the lower part of the band a face shows, the `waves` motif's pattern. */
function waveBands(rng: Rng): string {
  let d = "";
  const first = between(rng, 60, 64);
  for (let k = 0; k < 3; k += 1) {
    const y = first + k * between(rng, 6, 8);
    const lift = between(rng, 1.6, 2.8);
    const thick = between(rng, 1.8, 2.8);
    let top = `M${pt(0, y)}`;
    for (let x = 0; x < ART_BOX; x += 16) top += `Q${pt(x + 4, y - lift)} ${pt(x + 8, y)}Q${pt(x + 12, y + lift)} ${pt(x + 16, y)}`;
    let bottom = `L${pt(ART_BOX, y + thick)}`;
    for (let x = ART_BOX; x > 0; x -= 16) bottom += `Q${pt(x - 4, y + thick + lift)} ${pt(x - 8, y + thick)}Q${pt(x - 12, y + thick - lift)} ${pt(x - 16, y + thick)}`;
    d += `${top}${bottom}Z`;
  }
  return d;
}

/** Book: loose pages drifting in the sky, each ruled with three lines. */
function pages(rng: Rng): string {
  let d = "";
  const count = whole(rng, 3, 4);
  for (let k = 0; k < count; k += 1) {
    const cx = ART_BOX * ((k + 0.5) / count) + between(rng, -6, 6);
    const cy = between(rng, 24, 42);
    const w = between(rng, 6.5, 8.5);
    const h = w * 1.3;
    const tilt = between(rng, -28, 28) * DEGREES;
    const at = (x: number, y: number): string => pt(cx + x * Math.cos(tilt) - y * Math.sin(tilt), cy + x * Math.sin(tilt) + y * Math.cos(tilt));
    d += `M${at(-w / 2, -h / 2)}L${at(w / 2, -h / 2)}L${at(w / 2, h / 2)}L${at(-w / 2, h / 2)}Z`;
    for (const row of [-0.25, 0, 0.25]) {
      const y = row * h;
      d += `M${at(-w * 0.32, y - 0.3)}L${at(w * 0.32, y - 0.3)}L${at(w * 0.32, y + 0.3)}L${at(-w * 0.32, y + 0.3)}Z`;
    }
  }
  return d;
}

/** Pancake: steam curling up from the middle of the picture. */
function steam(rng: Rng): string {
  let d = "";
  const count = whole(rng, 2, 3);
  for (let k = 0; k < count; k += 1) {
    const x0 = 34 + (32 * (k + 0.5)) / count + between(rng, -4, 4);
    const sway = between(rng, 3, 5) * (rng() < 0.5 ? -1 : 1);
    const points: [number, number][] = [];
    for (let i = 0; i <= 10; i += 1) {
      const t = i / 10;
      points.push([x0 + sway * Math.sin(t * 2 * Math.PI), 54 - t * 32]);
    }
    d += bandPath(points, between(rng, 1.6, 2.4));
  }
  return d;
}

/**
 * Pancake: syrup across the top, running down in drips. It reaches just past the top of the band a
 * full face shows, so the drips hang from the window's top edge there, and a squarer window shows
 * the pool they run from.
 */
function syrup(rng: Rng): string {
  const depth = between(rng, 17.5, 19.5);
  let d = `M0 0L${ART_BOX} 0L${pt(ART_BOX, depth)}L${pt(0, depth)}Z`;
  const count = whole(rng, 3, 5);
  for (let k = 0; k < count; k += 1) {
    const x = ART_BOX * ((k + 0.5) / count) + between(rng, -6, 6);
    const r = between(rng, 1.1, 1.8);
    const length = between(rng, 4, 11);
    d += `M${pt(x - r, depth)}L${pt(x + r, depth)}L${pt(x + r, depth + length)}A${fmt(r)} ${fmt(r)} 0 0 1 ${pt(x - r, depth + length)}Z`;
  }
  return d;
}

/** AI: faint scanlines over the whole picture. */
function scanlines(): string {
  let d = "";
  for (let y = 1; y < ART_BOX; y += 3) d += rectPath(0, y, ART_BOX, 0.7);
  return d;
}

/** AI: one or two glitch bars, knocked sideways. */
function glitch(rng: Rng): string {
  let d = "";
  const count = whole(rng, 1, 2);
  for (let k = 0; k < count; k += 1) {
    const y = between(rng, 18, 82);
    const x = between(rng, -10, 30);
    d += rectPath(x, y, between(rng, 40, 70), between(rng, 1.4, 3.2));
  }
  return d;
}

type Twist = { under: Layer[]; over: Layer[] };

/** A family's own touch (R503), from its own stream: pages, steam and syrup, or circuitry. */
function twistLayers(rng: Rng, theme: ArtThemeId): Twist {
  switch (theme) {
    case "book":
      return { under: [{ d: pages(rng), fill: "twist", opacity: 0.5 }], over: [] };
    case "pancake":
      return {
        under: [{ d: steam(rng), fill: "glow", opacity: 0.28 }],
        over: [{ d: syrup(rng), fill: "twist", opacity: 0.9 }],
      };
    case "ai":
      return {
        under: [{ d: circuitTraces(rng), fill: "twist", opacity: 0.4 }],
        over: [
          { d: glitch(rng), fill: "twist", opacity: 0.22 },
          { d: scanlines(), fill: "glow", opacity: 0.07 },
        ],
      };
    default:
      return { under: [], over: [] };
  }
}

/** Each family twist's own colour: parchment pages, amber syrup, neon traces. */
const TWIST_COLORS: Readonly<Partial<Record<ArtThemeId, string>>> = {
  book: "#f4e4bf",
  pancake: "#a8561c",
  ai: "#39f3ff",
};

/** The themes with no emblem of their own to keep: a plain card, or a Token. */
const PLAIN_THEMES: ReadonlySet<ArtThemeId> = new Set<ArtThemeId>(["unit", "spell", "field-spell", "trap", "field-trap", "token"]);

type Placed = { x: number; y: number; size: number };

/**
 * The band of the square every face shows. A full face's art window is about 1.6 times as wide as
 * it is tall and `background-size: cover` fills it, so it shows roughly y 18 to 82 of the box (a
 * Unit's portrait a little more, a compact face nearly all). A motif stays inside this band.
 */
const SHOWN_TOP = 22;
const SHOWN_BOTTOM = 78;

function farEnough(x: number, y: number, size: number, taken: readonly Placed[]): boolean {
  return taken.every((other) => Math.hypot(other.x - x, other.y - y) >= (other.size + size) / 2 + 1.5);
}

/**
 * Where a motif's glyphs go (R503), from the motif stream, inside the band every face shows and
 * inside the rounded windows (a portrait's oval, a Field Spell's arch). `hero` puts one larger glyph
 * in the upper corner furthest from the emblem and the glow (a plain card has already made it its
 * emblem, so it adds nothing; a figure standing to one side has its emblem in that corner, so the
 * motif goes just under it); `scatter` strews a handful through the composition's sky; `rise`
 * stands a row along the ground; `fall` drops a few from above. Nothing lands on the emblem.
 */
function motifPlacements(
  rng: Rng,
  motif: Motif,
  plain: boolean,
  composition: Composition,
  layout: string,
  geometry: Geometry,
): (Placed & { rotate: number; opacity: number })[] {
  const emblem: Placed = { x: geometry.emblem.x, y: geometry.emblem.y, size: geometry.emblem.size };
  const taken: Placed[] = [emblem];
  const out: (Placed & { rotate: number; opacity: number })[] = [];
  const place = (x: number, y: number, size: number, rotate: number, opacity: number): void => {
    if (!farEnough(x, y, size, taken)) return;
    taken.push({ x, y, size });
    out.push({ x, y, size, rotate, opacity });
  };
  const figure = composition === "figure";
  switch (motif.arrangement) {
    case "hero": {
      if (plain) return out;
      const flip = rng() < 0.5;
      const size = figure ? between(rng, 14, 16) : between(rng, 15, 18);
      const rotate = between(rng, -10, 10);
      if (figure && (layout === "left" || layout === "right")) {
        out.push({ x: emblem.x, y: emblem.y + emblem.size / 2 + size / 2 + 3, size, rotate, opacity: 0.95 });
        return out;
      }
      const inset = figure ? 28 : 20;
      const corners = [
        { x: inset, y: 30 },
        { x: ART_BOX - inset, y: 30 },
      ];
      const score = (corner: { x: number; y: number }): number =>
        Math.min(Math.hypot(corner.x - emblem.x, corner.y - emblem.y), Math.hypot(corner.x - geometry.glow.cx, corner.y - geometry.glow.cy));
      const [a, b] = flip ? [corners[1], corners[0]] : [corners[0], corners[1]];
      const corner = a !== undefined && b !== undefined && score(b) > score(a) ? b : (a ?? { x: inset, y: 30 });
      out.push({ x: corner.x, y: corner.y, size, rotate, opacity: 0.95 });
      return out;
    }
    case "scatter": {
      const count = whole(rng, 4, 6);
      const top = Math.max(geometry.motes.top, SHOWN_TOP) + 3;
      const bottom = Math.max(top + 6, Math.min(geometry.motes.bottom, SHOWN_BOTTOM) - 3);
      for (let k = 0; k < count * 3 && out.length < count; k += 1) {
        place(between(rng, 12, 88), between(rng, top, bottom), between(rng, 7, 10), between(rng, -25, 25), between(rng, 0.8, 0.95));
      }
      return out;
    }
    case "rise": {
      const count = whole(rng, 3, 5);
      // A figure's portrait shows a little lower, and its oval narrows at the foot.
      const [left, right, low, high] = figure ? [26, 74, 76, 82] : [15, 85, 68, 74];
      for (let k = 0; k < count; k += 1) {
        const size = between(rng, 9, 12);
        place(left + ((right - left) * k) / (count - 1) + between(rng, -3, 3), between(rng, low, high), size, between(rng, -8, 8), 0.95);
      }
      return out;
    }
    case "fall": {
      const count = whole(rng, 3, 5);
      for (let k = 0; k < count; k += 1) {
        const size = between(rng, 6.5, 9.5);
        place(14 + (72 * (k + 0.5)) / count + between(rng, -5, 5), between(rng, 24, 40), size, between(rng, -18, 18), 0.9);
      }
      return out;
    }
  }
}

/* ------------------------------------------------------------------------------ colour --- */

type Colors = {
  sky: readonly [string, string];
  fills: Readonly<Record<LayerFill, string>>;
  glow: string;
  emblemFill: string;
  emblemStroke: string;
  mote: string;
};

/** The sky's two stops for a scheme, before any gilding. */
function skyFor(sky: readonly [string, string], scheme: SkyScheme, turn: number): readonly [string, string] {
  switch (scheme) {
    case "dusk":
      return sky;
    case "night":
      return [mixHex(sky[0], "#05060a", NIGHT_MIX), mixHex(sky[1], "#020204", NIGHT_MIX)];
    case "dawn":
      // Lit from below: a warm horizon under a dark zenith.
      return [sky[1], mixHex(sky[0], DAWN_WARMTH, DAWN_MIX)];
    case "split":
      return [turnHue(sky[0], turn), sky[1]];
  }
}

/** The colours outside the palette: the motif's own, and the family twist's. */
type ExtraColors = { motif: string; twist: string };

function colorsFor(palette: ThemePalette, radiant: boolean, extra: ExtraColors): Colors {
  if (!radiant) {
    return {
      sky: palette.sky,
      fills: {
        far: palette.ridges[0],
        mid: palette.ridges[1],
        near: palette.ridges[2],
        glow: palette.glow,
        accent: palette.emblem.fill,
        motif: extra.motif,
        twist: extra.twist,
      },
      glow: palette.glow,
      emblemFill: palette.emblem.fill,
      emblemStroke: palette.emblem.stroke,
      mote: palette.mote,
    };
  }
  const gold = RADIANT_PALETTE;
  const toward = (from: string, to: string): string => mixHex(from, to, RADIANT_MIX);
  return {
    sky: [toward(palette.sky[0], gold.sky[0]), toward(palette.sky[1], gold.sky[1])],
    fills: {
      far: toward(palette.ridges[0], gold.ridges[0]),
      mid: toward(palette.ridges[1], gold.ridges[1]),
      near: toward(palette.ridges[2], gold.ridges[2]),
      glow: gold.glow,
      accent: gold.emblem.fill,
      motif: mixHex(extra.motif, gold.emblem.fill, RADIANT_MOTIF_MIX),
      twist: mixHex(extra.twist, gold.glow, RADIANT_MOTIF_MIX),
    },
    glow: gold.glow,
    emblemFill: gold.emblem.fill,
    emblemStroke: gold.emblem.stroke,
    mote: gold.mote,
  };
}

function scatterMotes(rng: Rng, band: Geometry["motes"], fill: string): ArtSpec["motes"] {
  const count = whole(rng, MOTES_MIN, MOTES_MAX);
  const motes: ArtSpec["motes"][number][] = [];
  for (let i = 0; i < count; i += 1) {
    const x = between(rng, 4, 96);
    const y = between(rng, band.top, band.bottom);
    const r = between(rng, 0.4, 1.3);
    const opacity = between(rng, 0.35, 0.9);
    motes.push({ x: round2(x), y: round2(y), r: round2(r), fill, opacity: round2(opacity) });
  }
  return motes;
}

function radiantRays(seed: number): ArtSpec["rays"] {
  const rng = seededRandom((seed ^ RADIANT_SALT) >>> 0);
  const count = whole(rng, RAY_COUNT_MIN, RAY_COUNT_MAX);
  const offset = rng() * FULL_TURN;
  const rays: ArtSpec["rays"][number][] = [];
  for (let i = 0; i < count; i += 1) {
    const raw = offset + (FULL_TURN / count) * i + between(rng, -8, 8);
    const angle = ((raw % FULL_TURN) + FULL_TURN) % FULL_TURN;
    const width = between(rng, 3, 7);
    const opacity = between(rng, 0.18, 0.4);
    rays.push({ angle: round2(angle), width: round2(width), opacity: round2(opacity) });
  }
  return rays;
}

/** The accent glyph's corner: the one furthest from the emblem, just inside the box. */
function accentCorner(emblem: { x: number; y: number }): { x: number; y: number } {
  const corners = [
    { x: 16, y: 16 },
    { x: 84, y: 16 },
    { x: 16, y: 84 },
    { x: 84, y: 84 },
  ] as const;
  let best: { x: number; y: number } = corners[0];
  let bestDistance = -1;
  for (const corner of corners) {
    const distance = Math.hypot(corner.x - emblem.x, corner.y - emblem.y);
    if (distance > bestDistance) {
      best = corner;
      bestDistance = distance;
    }
  }
  return best;
}

/** R503: the variety salt for a card's set, read off its id's prefix (B2.2); Core's otherwise. */
export function varietySalt(defId: string): number {
  const dash = defId.indexOf("-");
  const prefix = dash < 0 ? "" : defId.slice(0, dash);
  return Object.hasOwn(SET_VARIETY_SALTS, prefix) ? (SET_VARIETY_SALTS[prefix] ?? VARIETY_SALT) : VARIETY_SALT;
}

/**
 * Pure. The seed is hashId(defId); the radiant variant reuses the base geometry. `motif` is the
 * motif the card's name calls up (`motifFor`), or null for none.
 */
export function artSpec(
  defId: string,
  theme: ArtThemeId,
  composition: Composition,
  radiant: boolean,
  motif: MotifId | null = null,
): ArtSpec {
  const seed = hashId(defId);
  const rng = seededRandom(seed);
  // Per card, never per variant: the emblem from the theme's pool, how far the palette turns, the
  // layout, the backdrop, the sky's lighting and the accent. All come from their own salted stream,
  // so the base and radiant faces agree on them (B2).
  const variety = seededRandom((seed ^ varietySalt(defId)) >>> 0);
  const pool = EMBLEM_POOLS[theme];
  const pooled = pool[Math.floor(variety() * pool.length)] ?? THEME_PALETTES[theme].emblem.glyph;
  const drift = (variety() * 2 - 1) * HUE_DRIFT[theme];
  const layouts = LAYOUTS[composition];
  const layout = layouts[Math.floor(variety() * layouts.length)] ?? layouts[0] ?? "";
  const backdrop = BACKDROPS[Math.floor(variety() * BACKDROPS.length)] ?? "none";
  const skyScheme = SKY_SCHEMES[Math.floor(variety() * SKY_SCHEMES.length)] ?? "dusk";
  const splitTurn = between(variety, SPLIT_TURN_MIN, SPLIT_TURN_MAX) * (variety() < 0.5 ? -1 : 1);
  const accentPool = ACCENT_POOL.filter((candidate) => candidate !== pooled);
  const accentPick = variety() < ACCENT_CHANCE ? (accentPool[Math.floor(variety() * accentPool.length)] ?? null) : null;
  const accentSize = between(variety, 7, 10);
  const accentTurn = between(variety, -20, 20);

  // R503: a plain card wears its motif as its emblem, in the motif's own colours; a tribe or family
  // keeps its emblem and carries the motif beside it, in place of the accent when that is a corner.
  const drawn: Motif | null = motif === null ? null : MOTIFS[motif];
  const plain = PLAIN_THEMES.has(theme);
  const glyph: EmblemGlyph = drawn !== null && plain ? drawn.glyph : pooled;
  const heroInCorner = drawn !== null && !plain && drawn.arrangement === "hero";
  const accentGlyph = heroInCorner || accentPick === glyph ? null : accentPick;

  const themed = driftPalette(THEME_PALETTES[theme], drift, glyph);
  const worn = drawn !== null && plain ? { ...themed, emblem: { glyph, fill: drawn.fill, stroke: drawn.stroke } } : themed;
  const palette: ThemePalette = { ...worn, sky: skyFor(worn.sky, skyScheme, splitTurn) };
  const colors = colorsFor(palette, radiant, { motif: drawn?.fill ?? palette.glow, twist: TWIST_COLORS[theme] ?? palette.glow });
  // Geometry first, in a fixed order that never depends on `radiant` (B2).
  const angle = between(rng, SKY_ANGLE_MIN, SKY_ANGLE_MAX);
  const behind = backdropLayers(rng, backdrop);
  const composed = COMPOSERS[composition](rng, theme, layout, drawn?.headgear);
  // The family twist and the motif draw from streams of their own, after everything else.
  const twist = twistLayers(seededRandom((seed ^ TWIST_SALT) >>> 0), theme);
  const motifStream = seededRandom((seed ^ MOTIF_SALT) >>> 0);
  const pattern: Layer[] =
    drawn?.pattern === "circuits"
      ? [{ d: circuitTraces(motifStream), fill: "motif", opacity: 0.4 }]
      : drawn?.pattern === "waves"
        ? [{ d: waveBands(motifStream), fill: "motif", opacity: 0.45 }]
        : [];
  const circuitsUnder = drawn?.pattern === "circuits" ? pattern : [];
  const wavesOver = drawn?.pattern === "waves" ? pattern : [];
  const geometry: Geometry = {
    ...composed,
    layers: [...behind, ...twist.under, ...circuitsUnder, ...composed.layers, ...wavesOver, ...twist.over],
  };
  const motes = scatterMotes(rng, geometry.motes, colors.mote);
  const corner = accentCorner(geometry.emblem);
  const glowOpacity = radiant ? Math.min(1, geometry.glow.opacity + RADIANT_GLOW_BOOST) : geometry.glow.opacity;
  const motifGlyphs: MotifGlyphSpec[] =
    drawn === null
      ? []
      : motifPlacements(motifStream, drawn, plain, composition, layout, geometry).map((placed) => ({
          glyph: drawn.glyph,
          x: round2(placed.x),
          y: round2(placed.y),
          size: round2(placed.size),
          rotate: round2(placed.rotate + (drawn.turn ?? 0)),
          fill: colors.fills.motif,
          stroke: radiant ? mixHex(drawn.stroke, RADIANT_PALETTE.emblem.stroke, RADIANT_MOTIF_MIX) : drawn.stroke,
          opacity: round2(placed.opacity),
        }));
  return {
    theme,
    composition,
    variant: radiant ? "radiant" : "base",
    layout,
    backdrop,
    skyScheme,
    sky: { from: colors.sky[0], to: colors.sky[1], angle: round2(angle) },
    glow: {
      cx: round2(geometry.glow.cx),
      cy: round2(geometry.glow.cy),
      r: round2(geometry.glow.r),
      color: colors.glow,
      opacity: round2(glowOpacity),
    },
    ridges: geometry.layers.map((layer) => ({ d: layer.d, fill: colors.fills[layer.fill], opacity: round2(layer.opacity) })),
    emblem: {
      glyph: palette.emblem.glyph,
      x: round2(geometry.emblem.x),
      y: round2(geometry.emblem.y),
      size: round2(geometry.emblem.size * (drawn !== null && plain ? MOTIF_EMBLEM_SCALE : 1)),
      rotate: round2(geometry.emblem.rotate + (drawn !== null && plain ? (drawn.turn ?? 0) : 0)),
      fill: colors.emblemFill,
      stroke: colors.emblemStroke,
    },
    accent:
      accentGlyph === null
        ? null
        : {
            glyph: accentGlyph,
            x: corner.x,
            y: corner.y,
            size: round2(accentSize),
            rotate: round2(accentTurn),
            fill: colors.fills.glow,
            stroke: colors.emblemStroke,
          },
    motes,
    moteShape: THEME_MOTES[theme],
    rays: radiant ? radiantRays(seed) : [],
    motif,
    motifGlyphs,
  };
}
