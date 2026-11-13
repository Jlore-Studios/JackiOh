// R3572001 (#3572): the held Stinkyscape+ set's own art. The set's cards that no scene recipe draws
// (stinkyscapePlusRecipes.ts, the Legendaries and Mythics) draw Stinkyscape's deep-space painter
// (`stinkyscapeSvg`) under the Mindustrial overlay, the girders, gears and conveyor the set's Mindustry
// machines share; the ten Terminus Starships and the five Automa-ζ fusions draw a subject of their own
// on the same space backdrop, each its hull, livery and look. Presentation only (CLAUDE.md rule 7): a pure
// function of the card's id, name, tags and type, geometry from streams the Radiant face shares, so the two
// faces differ only in paint and the gold rays. No base face paints in Radiant's gold window here. No other
// set reaches this file, so no shipped card's picture moves (R503).

import type { CardDef, CardType, Tag } from "@jackioh/shared";

import { fmt, hashId, seededRandom } from "./hash.ts";
import { CENTER_ORIGIN, motionStyle, moving, particleMarkup, type ArtMotion } from "./motion.ts";
import { partDraws, partHolds, type ArtPart } from "./pictures.ts";
import { hullMarkup, liveryPaint, STARSHIP_DESIGNS, starshipClassOf, type HullPaint, type Livery, type StarshipDesignId } from "./starshipHulls.ts";
import { FAMILY_PAINTS, stinkyscapeSvg, stinkySubject } from "./stinkyscape.ts";
import {
  at, between, circle, DEGREES, ellipse, FULL_TURN, group, line, path, poly, rect, twinkle, whole, type Paint, type Rng,
} from "./stinkyDraw.ts";
import { mixHex, RADIANT_PALETTE } from "./themes.ts";

export const STINKYSCAPE_PLUS_PREFIX = "stinkyscapeplus-";

/** Whether an id draws the set's own art: every entry of the held set, its tokens included. */
export function isStinkyscapePlusArt(defId: string): boolean {
  return defId.startsWith(STINKYSCAPE_PLUS_PREFIX);
}

/** What a face of the set carries as its `data-art-family`: all of them draw one family of art. */
export const STINKYSCAPE_PLUS_FAMILY = "stinkyscapeplus";

// The Terminus Starships (SP6): each a named hull of the starship painter in a dying star's livery

/** A far light the ship's sky holds: a dying sun, a cold moon, a black hole with its disk, a ringed world. */
export type Beacon = "sun" | "moon" | "hole" | "rings";

/** One Terminus Starship's picture: its hull design, the livery it wears, the pieces it holds (its class) and its far light. */
export type ShipLook = { hull: StarshipDesignId; livery: Livery; pieces: number; beacon: Beacon };

/** The ten Terminus Starships of #80 and #155, by id; their pieces are SP3's. No livery is in Radiant's gold window. */
export const TERMINUS_SHIPS: Readonly<Record<string, ShipLook>> = {
  "stinkyscapeplus-t-terminus-starship-ark-of-ashes": {
    hull: "ark", pieces: 3, beacon: "sun",
    livery: { hull: "#6a6670", trim: "#d8d2d0", stripe: "#e0561a", window: "#ffb08a", glow: "#ff7a3a" },
  },
  "stinkyscapeplus-t-terminus-starship-dying-ember": {
    hull: "scout", pieces: 4, beacon: "sun",
    livery: { hull: "#4a2a2a", trim: "#a86a5a", stripe: "#ff4a1a", window: "#ff9a6a", glow: "#ff4a2a" },
  },
  "stinkyscapeplus-t-terminus-starship-heat-death-pilgrim": {
    hull: "cathedral", pieces: 4, beacon: "moon",
    livery: { hull: "#b8b0c8", trim: "#5a4a7a", stripe: "#7a5ab8", window: "#d8f0ff", glow: "#c8a8ff" },
  },
  "stinkyscapeplus-t-terminus-starship-entropys-wake": {
    hull: "leviathan", pieces: 4, beacon: "hole",
    livery: { hull: "#3a2a5a", trim: "#9a7ad8", stripe: "#1a0a2a", window: "#b8a0ff", glow: "#8a5aff" },
  },
  "stinkyscapeplus-t-terminus-starship-red-giant-requiem": {
    hull: "freighter", pieces: 3, beacon: "sun",
    livery: { hull: "#a8322a", trim: "#2a2226", stripe: "#f0e0d8", window: "#ffc0a0", glow: "#ff5a3a" },
  },
  "stinkyscapeplus-t-terminus-starship-cold-exodus": {
    hull: "lantern-ferry", pieces: 4, beacon: "moon",
    livery: { hull: "#8ab8d8", trim: "#e8f4ff", stripe: "#3a6aa8", window: "#e0f8ff", glow: "#9ae0ff" },
  },
  "stinkyscapeplus-t-terminus-starship-omega-drift": {
    hull: "stinkyscape", pieces: 4, beacon: "hole",
    livery: { hull: "#2a1a5a", trim: "#e8e0ff", stripe: "#d43fd0", window: "#f0e8ff", glow: "#b08aff" },
  },
  "stinkyscapeplus-t-terminus-starship-farthest-shore": {
    hull: "frontiers-promise", pieces: 5, beacon: "rings",
    livery: { hull: "#dce6ee", trim: "#2a4a6a", stripe: "#3aa8c8", window: "#a8e8ff", glow: "#7ad8ff" },
  },
  "stinkyscapeplus-t-terminus-starship-nightfall-covenant": {
    hull: "silent-choir", pieces: 5, beacon: "moon",
    livery: { hull: "#242636", trim: "#6a6a9a", stripe: "#8a2ab8", window: "#c8b0ff", glow: "#a06aff" },
  },
  "stinkyscapeplus-t-terminus-starship-vacuum-psalm": {
    hull: "celestial-archive", pieces: 3, beacon: "rings",
    livery: { hull: "#1a2a6a", trim: "#b8c8e8", stripe: "#5a7ad8", window: "#c8e4ff", glow: "#6a9aff" },
  },
};

// The Automa-ζ (SP3): a ground machine fused with its air machine, each rung bigger and better armed

/** One Automa-ζ's look: its size, legs, wings, guns, missile pods, crown and paint. */
export type ZetaLook = { scale: number; legs: 2 | 4; wings: number; guns: number; pods: number; crest: boolean; hull: string; accent: string; glow: string };

/** The five fusions, by id: Dagger and Flare, Mace and Horizon, Fortress and Zenith, Scepter and Antumbra, Reign and Eclipse. */
export const AUTOMA_ZETA: Readonly<Record<string, ZetaLook>> = {
  "stinkyscapeplus-133-1": { scale: 1.0, legs: 2, wings: 1, guns: 1, pods: 0, crest: false, hull: "#8fa3b8", accent: "#e8742a", glow: "#7ff0ff" },
  "stinkyscapeplus-133-2": { scale: 1.12, legs: 2, wings: 2, guns: 1, pods: 1, crest: false, hull: "#8a909c", accent: "#e8502a", glow: "#ff8a4a" },
  "stinkyscapeplus-133-3": { scale: 1.24, legs: 4, wings: 2, guns: 1, pods: 2, crest: false, hull: "#7a8c6a", accent: "#d85a3a", glow: "#b8ff7a" },
  "stinkyscapeplus-133-4": { scale: 1.34, legs: 4, wings: 3, guns: 2, pods: 2, crest: false, hull: "#6a7ab0", accent: "#e83a8a", glow: "#ff7ad8" },
  "stinkyscapeplus-133-5": { scale: 1.44, legs: 4, wings: 3, guns: 3, pods: 3, crest: true, hull: "#4a4f68", accent: "#e83a3a", glow: "#ff6a5a" },
};

// Paint

const GILD_MIX = 0.5;
const GOLD_DARK = "#4a2c04";
const GOLD_SPACE: readonly [string, string] = ["#3a2408", "#0c0602"];
const SHIP_SPACE: readonly [string, string] = ["#10122a", "#030409"];
const SHIP_NEBULA: readonly [string, string] = ["#4a3ad8", "#a13fa8"];

function toward(from: string, to: string): string {
  return mixHex(from, to, GILD_MIX);
}

/** A hull paint as a Radiant face wears it: every tone mixed toward the foil's gold. */
function gild(p: HullPaint): HullPaint {
  return {
    space: [toward(p.space[0], GOLD_SPACE[0]), toward(p.space[1], GOLD_SPACE[1])],
    nebula: [toward(p.nebula[0], RADIANT_PALETTE.glow), toward(p.nebula[1], RADIANT_PALETTE.ridges[0])],
    light: toward(p.light, RADIANT_PALETTE.mote),
    hull: toward(p.hull, RADIANT_PALETTE.ridges[0]),
    dark: toward(p.dark, GOLD_DARK),
    accent: toward(p.accent, RADIANT_PALETTE.emblem.fill),
    glow: toward(p.glow, RADIANT_PALETTE.glow),
    line: p.line,
    trim: toward(p.trim, RADIANT_PALETTE.mote),
    window: toward(p.window, RADIANT_PALETTE.glow),
  };
}

// The document

const BOX = 100;
const HALF = BOX / 2;
const SUBJECT_X = 50;
const SUBJECT_Y = 53;
const SUBJECT_TILT_JITTER = 4;
const BACKDROP_SALT = 0x35720a01;
const SUBJECT_SALT = 0x35720b02;
const RAY_SALT = 0x35720c03;
const OVERLAY_SALT = 0x35720d04;
const STARS_MIN = 28;
const STARS_MAX = 44;
const NEBULAE_MIN = 2;
const NEBULAE_MAX = 3;
const HUE_DRIFT = 20;
const RAYS_MIN = 8;
const RAYS_MAX = 12;
const RAY_LENGTH = 150;
const RAY_OPACITY = 0.22;
const SPARKLES = 7;
const VIGNETTE_RADIUS = 74;
const VIGNETTE_START = 0.62;
const VIGNETTE_OPACITY = 0.55;
/** The vignette rect, as `stinkyscapeSvg` ends its pictures with: the overlay lies under it. */
const VIGNETTE = rect(0, 0, BOX, BOX, "url(#v)");

type Defs = string[];

/** The plating's gradient and the vignette, which every subject of the set draws with. */
function gradients(p: Paint, defs: Defs): void {
  defs.push(
    `<linearGradient id="hl" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${p.light}"/>` +
      `<stop offset="0.45" stop-color="${p.hull}"/><stop offset="1" stop-color="${p.dark}"/></linearGradient>`,
    `<radialGradient id="v" gradientUnits="userSpaceOnUse" cx="${HALF}" cy="${HALF}" r="${VIGNETTE_RADIUS}">` +
      `<stop offset="${VIGNETTE_START}" stop-color="#000" stop-opacity="0"/>` +
      `<stop offset="1" stop-color="#000" stop-opacity="${VIGNETTE_OPACITY}"/></radialGradient>`,
  );
}

/** A far light the sky holds, drawn at (x, y). */
function beaconMarkup(kind: Beacon, x: number, y: number, p: HullPaint, defs: Defs): string {
  switch (kind) {
    case "sun":
      defs.push(`<radialGradient id="bc"><stop offset="0" stop-color="${p.glow}" stop-opacity="0.9"/><stop offset="0.5" stop-color="${p.glow}" stop-opacity="0.3"/><stop offset="1" stop-color="${p.glow}" stop-opacity="0"/></radialGradient>`);
      return circle(x, y, 22, "url(#bc)") + circle(x, y, 7.5, p.glow, { opacity: 0.85 }) + circle(x, y, 4.5, p.light);
    case "moon":
      return (
        circle(x, y, 9, p.light, { opacity: 0.85 }) + circle(x + 3, y + 2, 7.2, p.dark, { opacity: 0.35 }) +
        circle(x - 3, y - 2, 1.6, p.dark, { opacity: 0.4 }) + circle(x + 2, y + 3, 1.1, p.dark, { opacity: 0.4 })
      );
    case "hole":
      return (
        ellipse(x, y, 17, 4.5, p.accent, { opacity: 0.55 }) + ellipse(x, y, 12, 2.8, p.glow, { opacity: 0.75 }) +
        circle(x, y, 6.5, "#000000") + ellipse(x, y, 17, 4.5, "none", { stroke: p.glow, "stroke-width": 0.5, opacity: 0.8 })
      );
    case "rings":
      return (
        circle(x, y, 7, p.hull, { opacity: 0.9 }) + circle(x - 2, y - 2, 4.5, p.light, { opacity: 0.45 }) +
        ellipse(x, y, 15, 3.6, "none", { stroke: p.trim, "stroke-width": 0.9, opacity: 0.8 })
      );
  }
}

/** The space every subject of the set sits in: a gradient, nebulae, stars and, for a ship, its far light. */
function backdrop(rng: Rng, space: readonly [string, string], nebula: readonly [string, string], defs: Defs): string {
  const out: string[] = [];
  const drift = between(rng, -HUE_DRIFT, HUE_DRIFT);
  defs.push(`<linearGradient id="sp" x1="0" y1="0" x2="0.3" y2="1"><stop offset="0" stop-color="${space[0]}"/><stop offset="1" stop-color="${space[1]}"/></linearGradient>`);
  out.push(rect(0, 0, BOX, BOX, "url(#sp)"));
  const nebulae = whole(rng, NEBULAE_MIN, NEBULAE_MAX);
  for (let i = 0; i < nebulae; i += 1) {
    const color = nebula[i % 2] ?? nebula[0];
    defs.push(
      `<radialGradient id="n${i}"><stop offset="0" stop-color="${color}" stop-opacity="0.75"/>` +
        `<stop offset="0.5" stop-color="${color}" stop-opacity="0.28"/><stop offset="1" stop-color="${color}" stop-opacity="0"/></radialGradient>`,
    );
    out.push(
      group(at(between(rng, 10, 90), between(rng, 10, 90), between(rng, 0, FULL_TURN) + drift), ellipse(0, 0, between(rng, 26, 46), between(rng, 12, 24), `url(#n${i})`), {
        opacity: between(rng, 0.45, 0.8),
      }),
    );
  }
  const stars = whole(rng, STARS_MIN, STARS_MAX);
  for (let i = 0; i < stars; i += 1) {
    out.push(circle(between(rng, 1, 99), between(rng, 1, 99), between(rng, 0.15, 0.55), "#ffffff", { opacity: between(rng, 0.35, 1) }));
  }
  for (let i = 0; i < 3; i += 1) out.push(twinkle(between(rng, 4, 96), between(rng, 4, 96), between(rng, 1.4, 2.6), "#ffffff", between(rng, 0.6, 0.95)));
  return out.join("");
}

/** A Radiant face's gold rays and sparkles about the subject. */
function rays(seed: number): string {
  const rng = seededRandom((seed ^ RAY_SALT) >>> 0);
  const count = whole(rng, RAYS_MIN, RAYS_MAX);
  const offset = rng() * FULL_TURN;
  const out: string[] = [];
  for (let i = 0; i < count; i += 1) {
    const angle = offset + (FULL_TURN / count) * i + between(rng, -8, 8);
    const width = between(rng, 3, 7);
    const a = (angle - width / 2) * DEGREES;
    const b = (angle + width / 2) * DEGREES;
    out.push(
      path(
        `M${SUBJECT_X} ${SUBJECT_Y}L${fmt(SUBJECT_X + RAY_LENGTH * Math.cos(a))} ${fmt(SUBJECT_Y + RAY_LENGTH * Math.sin(a))}` +
          `L${fmt(SUBJECT_X + RAY_LENGTH * Math.cos(b))} ${fmt(SUBJECT_Y + RAY_LENGTH * Math.sin(b))}Z`,
        RADIANT_PALETTE.ray,
        { opacity: between(rng, RAY_OPACITY * 0.6, RAY_OPACITY * 1.4) },
      ),
    );
  }
  for (let i = 0; i < SPARKLES; i += 1) {
    out.push(twinkle(between(rng, 6, 94), between(rng, 6, 94), between(rng, 1.2, 2.6), RADIANT_PALETTE.mote, between(rng, 0.6, 1)));
  }
  return out.join("");
}

/** The document a ship's or a fusion's face prints: sky, rays when Radiant, subject, particles, vignette, by `part`. */
function ownDocument(defs: Defs, sky: string, subject: string, radiant: boolean, seed: number, motion: ArtMotion | null, seedId: string, part: ArtPart): string {
  const behindSubject = partHolds(part, "far");
  return (
    `<svg viewBox="0 0 ${BOX} ${BOX}" width="${BOX}" height="${BOX}" xmlns="http://www.w3.org/2000/svg" preserveAspectRatio="xMidYMid slice">` +
    `<defs>${defs.join("")}</defs>` +
    motionStyle(motion, CENTER_ORIGIN) +
    (behindSubject ? moving("back", sky, motion) + (radiant ? rays(seed) : "") : "") +
    (partDraws(part, "subject") ? moving("subject", subject, motion) : "") +
    (partHolds(part, "nearest") ? particleMarkup(motion, seedId) : "") +
    (behindSubject ? VIGNETTE : "") +
    "</svg>"
  );
}

// A Terminus Starship

/** How much larger than its hull space a ship is drawn, so the starship's length fills its window. */
const SHIP_SCALE = 1.3;
/** A ship's tilt, bow up, as a launched Starship's is. */
const SHIP_TILT = -6;

function shipSvg(look: ShipLook, radiant: boolean, motion: ArtMotion | null, seedId: string, part: ArtPart): string {
  const seed = hashId(seedId);
  const design = { ...STARSHIP_DESIGNS[look.hull], livery: look.livery };
  const base: HullPaint = { ...liveryPaint(look.livery), space: SHIP_SPACE, nebula: SHIP_NEBULA };
  const p = radiant ? gild(base) : base;
  const behind = seededRandom((seed ^ BACKDROP_SALT) >>> 0);
  const ahead = seededRandom((seed ^ SUBJECT_SALT) >>> 0);
  const cls = starshipClassOf(look.pieces);
  const L = cls.length;
  const H = cls.height;
  const defs: Defs = [];
  gradients(p, defs);
  const sky = backdrop(behind, p.space, p.nebula, defs);
  const beaconAt = beaconMarkup(look.beacon, between(behind, 14, 28) + (behind() < 0.5 ? 0 : 56), between(behind, 16, 26), p, defs);
  const burn =
    poly([[-L, -0.45 * H], [-L - 10 - 0.3 * L, 0], [-L, 0.45 * H]], p.glow, "none", { opacity: 0.7 }) +
    poly([[-L, -0.2 * H], [-L - 6 - 0.15 * L, 0], [-L, 0.2 * H]], "#ffffff", "none", { opacity: 0.8 });
  const streaks = Array.from({ length: 8 }, () => {
    const x = between(ahead, 40, 100);
    const y = between(ahead, 14, 90);
    return line(x, y, x - between(ahead, 14, 34), y, mixHex(p.glow, "#ffffff", 0.5), between(ahead, 0.25, 0.6), { opacity: between(ahead, 0.25, 0.55) });
  }).join("");
  const hull = group(at(0, 0, 0, cls.zoom * SHIP_SCALE), burn + hullMarkup(design, cls, p, true));
  const tilt = SHIP_TILT + between(ahead, -SUBJECT_TILT_JITTER, SUBJECT_TILT_JITTER);
  return ownDocument(defs, sky + beaconAt + streaks, group(at(SUBJECT_X, SUBJECT_Y, tilt), hull), radiant, seed, motion, seedId, part);
}

// An Automa-ζ

/** An Automa-ζ drawn about (0, 0), facing right: a ground machine's legs and gun under an air machine's wings and jets. */
/** The rivets along a fusion's seam and its panel lines. */
const ZETA_RIVETS = 7;
const ZETA_PANELS = 5;

function zetaMachine(look: ZetaLook, p: HullPaint): string {
  const out: string[] = [];
  const inkedStroke = { stroke: p.line, "stroke-width": 0.6 };
  for (let w = 0; w < look.wings; w += 1) {
    const lift = w * 4.5;
    out.push(poly([[-3, -9 - lift * 0.4], [-31 - w * 4, -25 - lift], [-24 - w * 4, -10 - lift * 0.5]], w % 2 === 0 ? p.dark : p.hull, p.line));
    out.push(line(-6, -10 - lift * 0.4, -27 - w * 4, -22 - lift, p.accent, 0.6));
  }
  const legX = look.legs === 4 ? [-11, -4.5, 3, 10] : [-5.5, 5.5];
  legX.forEach((x, i) => {
    const reach = look.legs === 4 ? 1 : 1.25;
    out.push(poly([[x - 2.2, 1], [x + 2.2, 1], [x + 3.4 * reach, 9], [x - 0.6, 11]], "url(#hl)", p.line));
    out.push(poly([[x - 0.6, 10], [x + 3 * reach, 8.4], [x + 2.2, 18], [x - 2.6, 18]], p.hull, p.line));
    out.push(rect(x - 3.6, 17.4, 7, 2.2, p.dark, inkedStroke));
    out.push(poly([[x - 2.4, 19.6], [x, 19.6 + 5 + (i % 2) * 2], [x + 2.4, 19.6]], p.glow, "none", { opacity: 0.85 }));
  });
  out.push(poly([[-10, -13], [-6, -16], [7, -16], [11, -11], [9, 2], [-9, 2]], "url(#hl)", p.line));
  out.push(rect(-8.5, -4.5, 17, 2, p.accent));
  out.push(poly([[2, -15], [8, -13.5], [9, -9.5], [2, -9.5]], p.dark, p.line));
  out.push(circle(5.4, -12, 1.6, p.glow));
  for (let g = 0; g < look.guns; g += 1) {
    const y = -8.5 + g * 3.6;
    out.push(rect(7, y, 15 + g * 2.5, 2, p.dark, inkedStroke));
    out.push(circle(22 + g * 2.5, y + 1, 1.5, p.glow));
  }
  for (let m = 0; m < look.pods; m += 1) {
    const x = -8 + m * 4.2;
    out.push(rect(x, -20.6, 3.4, 4.6, p.hull, inkedStroke));
    out.push(circle(x + 1.7, -21.2, 1.1, p.accent));
  }
  // Rivets along the hull's seam and four panel lines, so even the barest fusion holds a picture's detail.
  for (let r = 0; r < ZETA_RIVETS; r += 1) out.push(circle(-7.5 + r * (15 / (ZETA_RIVETS - 1)), -1.2, 0.5, p.line));
  for (let l = 0; l < ZETA_PANELS; l += 1) out.push(line(-8 + l * 4.6, -12.5 + (l % 2), -8 + l * 4.6, 1, p.line, 0.3));
  if (look.crest) {
    out.push(poly([[-4, -16], [-3, -23], [-1, -17], [1, -24], [3, -17], [5, -23], [6, -16]], p.accent, p.line));
  }
  return out.join("");
}

function zetaSvg(look: ZetaLook, radiant: boolean, motion: ArtMotion | null, seedId: string, part: ArtPart): string {
  const seed = hashId(seedId);
  const livery: Livery = { hull: look.hull, trim: mixHex(look.hull, "#ffffff", 0.6), stripe: look.accent, window: look.glow, glow: look.glow };
  const base: HullPaint = { ...liveryPaint(livery), space: SHIP_SPACE, nebula: [look.accent, look.glow] };
  const p = radiant ? gild(base) : base;
  const behind = seededRandom((seed ^ BACKDROP_SALT) >>> 0);
  const ahead = seededRandom((seed ^ SUBJECT_SALT) >>> 0);
  const defs: Defs = [];
  gradients(p, defs);
  const sky = backdrop(behind, p.space, p.nebula, defs);
  const tilt = between(ahead, -SUBJECT_TILT_JITTER, SUBJECT_TILT_JITTER);
  const body = group(at(SUBJECT_X, SUBJECT_Y + 1, tilt, look.scale), zetaMachine(look, p));
  return ownDocument(defs, sky, body, radiant, seed, motion, seedId, part);
}

// The Mindustrial overlay

/** How many shapes the overlay draws at the least: girders, gears and a conveyor, so a picture holds at least twice that. */
export const OVERLAY_SHAPES_MIN = 30;

const STEEL = "#6b7480";
const STEEL_DARK = "#2a2f38";
const STEEL_LIGHT = "#9aa3b0";
const HAZARD = "#e8742a";
const BRACES_MIN = 5;
const BRACES_MAX = 8;
const GEARS_MIN = 2;
const GEARS_MAX = 3;
const CHEVRONS_MIN = 5;
const CHEVRONS_MAX = 7;

/** A gear's outline about (0, 0): `teeth` squared teeth round a body of radius `r`. */
function gearPath(r: number, teeth: number): string {
  const tooth = r * 0.28;
  const step = FULL_TURN / teeth;
  const pts: string[] = [];
  for (let i = 0; i < teeth; i += 1) {
    for (const [turn, radius] of [[-0.3, r], [-0.2, r + tooth], [0.2, r + tooth], [0.3, r]] as const) {
      const a = (i * step + turn * step) * DEGREES;
      pts.push(`${fmt(Math.cos(a) * radius)} ${fmt(Math.sin(a) * radius)}`);
    }
  }
  return `M${pts.join("L")}Z`;
}

/** One girder column at x: two rails, a zigzag of braces and a rivet at each joint. */
function girder(x: number, braces: number, tone: readonly [string, string, string]): string {
  const [rail, brace, rivet] = tone;
  const width = 3;
  const span = BOX / braces;
  const out: string[] = [rect(x - width, 0, 1, BOX, rail), rect(x + width - 1, 0, 1, BOX, rail)];
  for (let i = 0; i < braces; i += 1) {
    const flip = i % 2 === 0;
    out.push(line(flip ? x - width + 1 : x + width - 1, i * span, flip ? x + width - 1 : x - width + 1, (i + 1) * span, brace, 0.6));
    out.push(circle(flip ? x + width - 1 : x - width + 1, (i + 1) * span, 0.55, rivet));
  }
  return out.join("");
}

/** A conveyor along the foot of the picture: a belt, its rollers, its chevrons and two crates riding it. */
function conveyor(rng: Rng, chevrons: number, tone: readonly [string, string, string, string]): string {
  const [belt, roller, mark, crate] = tone;
  const top = 90;
  const out: string[] = [rect(0, top, BOX, 5, belt), rect(0, top, BOX, 0.7, roller)];
  const gap = BOX / (chevrons + 1);
  for (let i = 1; i <= chevrons; i += 1) {
    const x = i * gap;
    out.push(circle(x, top + 5.6, 1.5, roller));
    out.push(path(`M${fmt(x - 1.4)} ${top + 1}L${fmt(x + 0.6)} ${top + 2.5}L${fmt(x - 1.4)} ${top + 4}`, "none", { stroke: mark, "stroke-width": 0.7, "stroke-linecap": "round" }));
  }
  for (let i = 0; i < 2; i += 1) out.push(rect(between(rng, 14, 76), top - 5, 8 + between(rng, 0, 4), 5, crate, { stroke: STEEL_DARK, "stroke-width": 0.4 }));
  return out.join("");
}

/**
 * The Mindustrial overlay's shapes for a seed: a girder up each edge, gears at its sides turning in the box's margins
 * and a conveyor along the foot, at least `OVERLAY_SHAPES_MIN` of them. A Radiant face's tones are mixed toward gold.
 */
export function mindustrialOverlay(seedId: string, radiant: boolean): string {
  const rng = seededRandom((hashId(seedId) ^ OVERLAY_SALT) >>> 0);
  const tone = (hex: string): string => (radiant ? mixHex(hex, RADIANT_PALETTE.ridges[0], GILD_MIX) : hex);
  const braces = whole(rng, BRACES_MIN, BRACES_MAX);
  const out: string[] = [];
  for (const x of [3.5, BOX - 3.5]) out.push(girder(x, braces, [tone(STEEL), tone(STEEL_LIGHT), tone(HAZARD)]));
  const gears = whole(rng, GEARS_MIN, GEARS_MAX);
  for (let i = 0; i < gears; i += 1) {
    const left = i % 2 === 0;
    const r = between(rng, 5, 8.5);
    const x = left ? between(rng, 10, 15) : between(rng, BOX - 15, BOX - 10);
    const y = between(rng, 22 + i * 16, 36 + i * 16);
    const turn = between(rng, 0, FULL_TURN);
    out.push(path(gearPath(r, whole(rng, 8, 12)), tone(STEEL), { transform: at(x, y, turn), stroke: STEEL_DARK, "stroke-width": 0.5, opacity: 0.85 }));
    out.push(circle(x, y, r * 0.38, tone(STEEL_DARK), { opacity: 0.9 }));
    out.push(circle(x, y, r * 0.14, tone(HAZARD)));
  }
  out.push(conveyor(rng, whole(rng, CHEVRONS_MIN, CHEVRONS_MAX), [tone(STEEL_DARK), tone(STEEL_LIGHT), tone(HAZARD), tone(STEEL)]));
  return out.join("");
}

// The picture

/** Whether `defId` draws Stinkyscape's painter under the Mindustrial overlay: a card of the set no ship's or fusion's subject draws. */
export function paintsUnderOverlay(defId: string): boolean {
  return isStinkyscapePlusArt(defId) && !Object.hasOwn(TERMINUS_SHIPS, defId) && !Object.hasOwn(AUTOMA_ZETA, defId);
}

/**
 * The SVG document a Stinkyscape+ face draws, unencoded; moving as `motion` says, its backdrop and the rest grown from
 * `seedId` (the card's id by default) and `part` leaving its subject out or drawing it alone. A Terminus Starship
 * and an Automa-ζ draw their own subject; every other card draws Stinkyscape's painter under the Mindustrial overlay,
 * which belongs to no subject, so the subject alone has none.
 */
export function stinkyscapePlusSvg(
  defId: string,
  name: string | undefined,
  tags: readonly Tag[],
  type: CardType,
  radiant: boolean,
  motion: ArtMotion | null = null,
  seedId: string = defId,
  part: ArtPart = "whole",
): string {
  if (Object.hasOwn(TERMINUS_SHIPS, defId)) return shipSvg(TERMINUS_SHIPS[defId] as ShipLook, radiant, motion, seedId, part);
  if (Object.hasOwn(AUTOMA_ZETA, defId)) return zetaSvg(AUTOMA_ZETA[defId] as ZetaLook, radiant, motion, seedId, part);
  const svg = stinkyscapeSvg(defId, name, tags, type, radiant, motion, seedId, part);
  if (!partDraws(part, "props")) return svg;
  const overlay = moving("props", mindustrialOverlay(seedId, radiant), motion);
  const end = svg.endsWith(`${VIGNETTE}</svg>`) ? svg.length - VIGNETTE.length - "</svg>".length : svg.length - "</svg>".length;
  return svg.slice(0, end) + overlay + svg.slice(end);
}

/** The light, middle and glow tones a Stinkyscape+ card's picture is painted in, for its tile's aura (fx/aura/tile.ts). */
export function stinkyscapePlusColors(def: Pick<CardDef, "id" | "name" | "tags" | "type">): readonly [string, string, string] {
  const ship = Object.hasOwn(TERMINUS_SHIPS, def.id) ? (TERMINUS_SHIPS[def.id] as ShipLook).livery : null;
  if (ship !== null) return [ship.trim, ship.hull, ship.glow];
  const zeta = Object.hasOwn(AUTOMA_ZETA, def.id) ? (AUTOMA_ZETA[def.id] as ZetaLook) : null;
  if (zeta !== null) return [mixHex(zeta.hull, "#ffffff", 0.6), zeta.hull, zeta.glow];
  const paint = FAMILY_PAINTS[stinkySubject(def.id, def.name, def.tags, def.type).family];
  return [paint.light, paint.nebula[0], paint.glow];
}
