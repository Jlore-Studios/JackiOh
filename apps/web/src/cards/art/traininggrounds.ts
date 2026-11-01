// R2202000: the Training Grounds set's own procedural art (issue #2202). Every `traininggrounds-`
// entry but the snapshots draws here instead of `artSpec`: a training yard (a sky, a palisade, a
// sand floor) under one subject that its family fixes and its name's word details, so the set's
// dummies, ogres, knives, totems, Felinors, runes, posters and its Golem each read as what the card
// is. The snapshots #41–#60 keep their Core card's picture (`SNAPSHOT_ART`). Presentation only
// (CLAUDE.md rule 7): a pure function of the card's id, name, tags and type, with geometry drawn from
// streams the Radiant face shares, so the two faces differ only in their paint and the gold rays.
// No other set reaches this file, so no shipped card's picture moves (R503).

import type { CardType, Tag } from "@jackioh/shared";

import { EMBLEM_BOX, EMBLEMS, type EmblemGlyph } from "./emblems.ts";
import { fmt, hashId, seededRandom } from "./hash.ts";
import { MOTIFS, motifFor } from "./motifs.ts";
import { mixHex, RADIANT_PALETTE, turnHue } from "./themes.ts";

export const TRAINING_GROUNDS_PREFIX = "traininggrounds-";

/**
 * The Legacy cards, the snapshots of docs/training-grounds-set.md TG2: each draws the picture of the
 * Core card it freezes, so a Legacy card and its Core card look the same (R503); its `Legacy:` name and
 * tag set it apart (R2511000).
 */
export const SNAPSHOT_ART: Readonly<Record<string, string>> = {
  "traininggrounds-041": "core-002",
  "traininggrounds-042": "core-005",
  "traininggrounds-043": "core-008",
  "traininggrounds-044": "core-012",
  "traininggrounds-045": "core-014",
  "traininggrounds-046": "core-016",
  "traininggrounds-047": "core-019",
  "traininggrounds-048": "core-013",
  "traininggrounds-049": "core-023",
  "traininggrounds-050": "core-026",
  "traininggrounds-051": "core-043",
  "traininggrounds-052": "core-049",
  "traininggrounds-053": "core-061",
  "traininggrounds-054": "core-066",
  "traininggrounds-055": "core-072",
  "traininggrounds-056": "core-089",
  "traininggrounds-057": "core-039",
  "traininggrounds-058": "core-044",
  "traininggrounds-059": "core-047",
  "traininggrounds-060": "core-058",
};

/** R2511000: what a Legacy card's name holds before its Core card's. */
export const LEGACY_PREFIX = /^Legacy: /;

/** The Core card whose picture a Legacy card wears, or null for any other id. */
export function snapshotArtOf(defId: string): string | null {
  return Object.hasOwn(SNAPSHOT_ART, defId) ? (SNAPSHOT_ART[defId] ?? null) : null;
}

/** Whether an id draws the set's own art: the held set's entries and its token, not its snapshots. */
export function isTrainingGroundsArt(defId: string): boolean {
  return defId.startsWith(TRAINING_GROUNDS_PREFIX) && snapshotArtOf(defId) === null;
}

// The families

export type YardFamily =
  | "dummy" | "ogre" | "knife" | "totem" | "felinor" | "rune" | "poster" | "golem" | "chaos" | "sky"
  | "seance" | "bean" | "gate" | "recruit" | "target";

export type YardSubject = { family: YardFamily; kind: string };

/** First match wins: a name's word and the family and kind it draws. */
const NAME_WORDS: readonly (readonly [string, YardFamily, string])[] = [
  ["golem", "golem", "golem"],
  ["dummy", "dummy", "dummy"],
  ["boulder", "ogre", "boulder"],
  ["clobber", "ogre", "club"],
  ["ravager", "ogre", "ravager"],
  ["goliath", "ogre", "goliath"],
  ["fan of knives", "knife", "fan"],
  ["knife", "knife", "juggler"],
  ["totem", "totem", "flame"],
  ["celestial", "sky", "both"],
  ["sun", "sky", "sun"],
  ["moon", "sky", "moon"],
  ["seance", "seance", "seance"],
  ["jelly bean", "bean", "bean"],
  ["floodgate", "gate", "gate"],
];

/** A Felinor's detail, by the word its name holds. */
const FELINOR_WORDS: readonly (readonly [string, string])[] = [
  ["flaming", "flaming"],
  ["right-hand", "paw"],
  ["quadrupling", "four"],
  ["guerilla", "guerilla"],
];

/** A Field Spell's poster, by the word its name holds. */
const POSTER_WORDS: readonly (readonly [string, string])[] = [
  ["ration", "rations"],
  ["level", "level"],
  ["recruit", "recruit"],
  ["corpse", "recycle"],
];

/** A recruit's detail, by the word its name holds. */
const RECRUIT_WORDS: readonly (readonly [string, string])[] = [
  ["spikey", "spiky"],
  ["trainer", "trainer"],
  ["leader", "leader"],
  ["protector", "protector"],
  ["tutorial", "master"],
  ["trigor", "three"],
  ["occidental", "west"],
  ["oriental", "east"],
];

function wordIn(lower: string, table: readonly (readonly [string, string])[], fallback: string): string {
  return table.find(([word]) => lower.includes(word))?.[1] ?? fallback;
}

export function yardSubject(_defId: string, name: string | undefined, tags: readonly Tag[], type: CardType): YardSubject {
  const lower = (name ?? "").toLowerCase();
  if (tags.includes("Call to Chaos")) return { family: "chaos", kind: "die" };
  if (tags.includes("Felinor")) return { family: "felinor", kind: wordIn(lower, FELINOR_WORDS, "plain") };
  if (tags.includes("Jelly Bean")) return { family: "bean", kind: "bean" };
  const named = NAME_WORDS.find(([word]) => lower.includes(word));
  if (named !== undefined) return { family: named[1], kind: named[2] };
  if (type === "Trap" || type === "Field Trap") return { family: "rune", kind: lower.includes("rune") ? "runes" : "plate" };
  if (type === "Field Spell") return { family: "poster", kind: wordIn(lower, POSTER_WORDS, "notice") };
  if (type === "Unit") return { family: "recruit", kind: wordIn(lower, RECRUIT_WORDS, "recruit") };
  return { family: "target", kind: "target" };
}

// The paint

type Paint = {
  sky: readonly [string, string];
  ground: readonly [string, string];
  wood: string;
  /** The subject's light, middle and shadow tones. */
  light: string;
  body: string;
  dark: string;
  accent: string;
  glow: string;
  line: string;
};

const OUTLINE = "#1f1408";
const DAY_SKY: readonly [string, string] = ["#7fb8e6", "#e8f1d8"];
const DUSK_SKY: readonly [string, string] = ["#3a3a7a", "#e89a6a"];
const NIGHT_SKY: readonly [string, string] = ["#151a3a", "#4a3a6a"];
const SAND: readonly [string, string] = ["#e3c48a", "#a87e45"];
const WOOD = "#a8743e";

export const FAMILY_PAINTS: Readonly<Record<YardFamily, Paint>> = {
  dummy: { sky: DAY_SKY, ground: SAND, wood: WOOD, light: "#f2dc9a", body: "#d1ad62", dark: "#7a5a26", accent: "#d2452f", glow: "#fff2c0", line: OUTLINE },
  ogre: { sky: DUSK_SKY, ground: SAND, wood: WOOD, light: "#b8d88a", body: "#6f9a3e", dark: "#2f4a18", accent: "#8a6a4a", glow: "#ffe0a0", line: OUTLINE },
  knife: { sky: DAY_SKY, ground: SAND, wood: WOOD, light: "#eef2f6", body: "#a9b4c0", dark: "#4a525c", accent: "#d2452f", glow: "#ffffff", line: OUTLINE },
  totem: { sky: DUSK_SKY, ground: SAND, wood: "#8a5a2e", light: "#d89a5a", body: "#a8643a", dark: "#4a2a14", accent: "#2f8a8a", glow: "#ffb02a", line: OUTLINE },
  felinor: { sky: DAY_SKY, ground: SAND, wood: WOOD, light: "#f6d6a8", body: "#d8924a", dark: "#6a3a14", accent: "#5ac85a", glow: "#ff8a2a", line: OUTLINE },
  rune: { sky: NIGHT_SKY, ground: ["#8a7a6a", "#3a3028"], wood: WOOD, light: "#b8b4ac", body: "#7a766e", dark: "#34322e", accent: "#ff6a2a", glow: "#5ae8ff", line: OUTLINE },
  poster: { sky: DAY_SKY, ground: SAND, wood: WOOD, light: "#f8f0dc", body: "#e8d8b0", dark: "#8a7450", accent: "#2a5ab8", glow: "#d2452f", line: OUTLINE },
  golem: { sky: NIGHT_SKY, ground: ["#8a7a6a", "#3a3028"], wood: WOOD, light: "#c8c0b0", body: "#8a8272", dark: "#3a362e", accent: "#5a4a3a", glow: "#7affc8", line: OUTLINE },
  chaos: { sky: ["#2a0a3a", "#a83a8a"], ground: ["#6a4a7a", "#2a1a3a"], wood: WOOD, light: "#fff4f8", body: "#e8d8f0", dark: "#4a2a5a", accent: "#2a0a3a", glow: "#ff6ad8", line: OUTLINE },
  sky: { sky: ["#1a2a6a", "#f0b86a"], ground: SAND, wood: WOOD, light: "#fff6c0", body: "#ffc83a", dark: "#b86a1a", accent: "#d8e4ff", glow: "#fff0a0", line: OUTLINE },
  seance: { sky: NIGHT_SKY, ground: ["#5a4a5a", "#1a1420"], wood: WOOD, light: "#f0f4ff", body: "#b8c8e8", dark: "#3a4a6a", accent: "#f2e6c8", glow: "#ffd25a", line: OUTLINE },
  bean: { sky: DAY_SKY, ground: SAND, wood: WOOD, light: "#ffd8f0", body: "#ff6ab8", dark: "#8a1a5a", accent: "#2a1a2a", glow: "#ffffff", line: OUTLINE },
  gate: { sky: DAY_SKY, ground: SAND, wood: "#8a5a2e", light: "#bfe8ff", body: "#4a9ad8", dark: "#1a4a7a", accent: "#ffffff", glow: "#e8f8ff", line: OUTLINE },
  recruit: { sky: DAY_SKY, ground: SAND, wood: WOOD, light: "#e8ecf0", body: "#3a6ab8", dark: "#1a2a4a", accent: "#c0c8d0", glow: "#f2c8a0", line: OUTLINE },
  target: { sky: DAY_SKY, ground: SAND, wood: WOOD, light: "#f6ecc8", body: "#d2452f", dark: "#5a1a10", accent: "#2a5ab8", glow: "#fff6d8", line: OUTLINE },
};

/** A color a kind gives its family's body or glow: a Felinor's fur, a sky's body. */
const KIND_BODIES: Readonly<Record<string, string>> = {
  flaming: "#e8582a",
  guerilla: "#7a7a3a",
  moon: "#d8e0f0",
  ravager: "#8a9a4a",
  goliath: "#5a8a3a",
};
/** A recruit's skin, drawn per card from the set's own list. */
const SKINS: readonly string[] = ["#f2c8a0", "#d8a070", "#a8703e", "#7a4a26", "#f6dcc0"];
/** How far a recruit's tunic turns from the family's own. */
const TUNIC_DRIFT = 120;

/** Radiant faces mix every family tone this far toward the foil's gold. */
const GILD_MIX = 0.5;
const GOLD_DARK = "#4a2c04";
const GOLD_SKY: readonly [string, string] = ["#6a4810", "#f8dc8a"];

function gild(paint: Paint): Paint {
  const toward = (from: string, to: string): string => mixHex(from, to, GILD_MIX);
  return {
    sky: [toward(paint.sky[0], GOLD_SKY[0]), toward(paint.sky[1], GOLD_SKY[1])],
    ground: [toward(paint.ground[0], RADIANT_PALETTE.glow), toward(paint.ground[1], GOLD_DARK)],
    wood: toward(paint.wood, RADIANT_PALETTE.ridges[0]),
    light: toward(paint.light, RADIANT_PALETTE.mote),
    body: toward(paint.body, RADIANT_PALETTE.ridges[0]),
    dark: toward(paint.dark, GOLD_DARK),
    accent: toward(paint.accent, RADIANT_PALETTE.emblem.fill),
    glow: toward(paint.glow, RADIANT_PALETTE.glow),
    line: paint.line,
  };
}

const PAINT_SALT = 0x2202a17d;

/** The variety a family allows inside its look, drawn before any gilding so both faces share it. */
function drift(seed: number, subject: YardSubject, base: Paint): Paint {
  const body = KIND_BODIES[subject.kind];
  const paint = body === undefined ? base : { ...base, body };
  if (subject.family !== "recruit") return paint;
  const rng = seededRandom((seed ^ PAINT_SALT) >>> 0);
  const skin = SKINS[Math.floor(rng() * SKINS.length)] ?? SKINS[0] ?? paint.glow;
  return { ...paint, body: turnHue(paint.body, between(rng, -TUNIC_DRIFT, TUNIC_DRIFT)), glow: skin };
}

// Drawing

type Rng = () => number;
type Pt = readonly [number, number];

const BOX = 100;
const HALF = BOX / 2;
const DEGREES = Math.PI / 180;
const FULL_TURN = 360;
const STROKE = 0.8;
/** Where the subject sits: a little below the middle, as every window crops it (ART.md). */
const SUBJECT_X = 50;
const SUBJECT_Y = 54;
const SUBJECT_TILT_JITTER = 5;
const SUBJECT_SCALE_JITTER = 0.05;

const BACKDROP_SALT = 0x7a1d5e11;
const SUBJECT_SALT = 0x2202c0de;
const RAY_SALT = 0x5eed2202;
const HORIZON = 66;
const CLOUDS_MIN = 1;
const CLOUDS_MAX = 3;
const STAKE_MIN = 5;
const STAKE_MAX = 7.5;
const BALE_CHANCE = 0.5;
const PENNANT_CHANCE = 0.55;
const STARS = 18;
const RAYS_MIN = 8;
const RAYS_MAX = 12;
const RAY_LENGTH = 150;
const RAY_OPACITY = 0.22;
const SPARKLES = 7;
const VIGNETTE_RADIUS = 74;
const VIGNETTE_START = 0.62;
const VIGNETTE_OPACITY = 0.45;

function between(rng: Rng, low: number, high: number): number {
  return low + rng() * (high - low);
}

function whole(rng: Rng, low: number, high: number): number {
  return Math.floor(between(rng, low, high + 1));
}

function attrs(values: Readonly<Record<string, string | number>>): string {
  return Object.entries(values)
    .map(([key, value]) => ` ${key}="${typeof value === "number" ? fmt(value) : value}"`)
    .join("");
}

function shape(tag: string, values: Readonly<Record<string, string | number>>): string {
  return `<${tag}${attrs(values)}/>`;
}

function poly(list: readonly Pt[], fill: string, extra: Record<string, string | number> = {}): string {
  return shape("polygon", { points: list.map(([x, y]) => `${fmt(x)},${fmt(y)}`).join(" "), fill, ...extra });
}

function circle(cx: number, cy: number, r: number, fill: string, extra: Record<string, string | number> = {}): string {
  return shape("circle", { cx, cy, r, fill, ...extra });
}

function ellipse(cx: number, cy: number, rx: number, ry: number, fill: string, extra: Record<string, string | number> = {}): string {
  return shape("ellipse", { cx, cy, rx, ry, fill, ...extra });
}

function rect(x: number, y: number, w: number, h: number, fill: string, extra: Record<string, string | number> = {}): string {
  return shape("rect", { x, y, width: w, height: h, fill, ...extra });
}

function path(d: string, fill: string, extra: Record<string, string | number> = {}): string {
  return shape("path", { d, fill, ...extra });
}

function line(x1: number, y1: number, x2: number, y2: number, stroke: string, width: number, extra: Record<string, string | number> = {}): string {
  return shape("line", { x1, y1, x2, y2, stroke, "stroke-width": width, "stroke-linecap": "round", ...extra });
}

function inked(p: Paint): Record<string, string | number> {
  return { stroke: p.line, "stroke-width": STROKE, "stroke-linejoin": "round" };
}

function group(transform: string, body: string, extra: Record<string, string | number> = {}): string {
  return `<g${attrs({ transform, ...extra })}>${body}</g>`;
}

function at(x: number, y: number, rotate = 0, scale = 1): string {
  return `translate(${fmt(x)} ${fmt(y)}) rotate(${fmt(rotate)}) scale(${fmt(scale)})`;
}

function twinkle(x: number, y: number, r: number, fill: string, opacity: number): string {
  const w = r * 0.22;
  return path(
    `M${fmt(x)} ${fmt(y - r)}L${fmt(x + w)} ${fmt(y - w)}L${fmt(x + r)} ${fmt(y)}L${fmt(x + w)} ${fmt(y + w)}` +
      `L${fmt(x)} ${fmt(y + r)}L${fmt(x - w)} ${fmt(y + w)}L${fmt(x - r)} ${fmt(y)}L${fmt(x - w)} ${fmt(y - w)}Z`,
    fill,
    { opacity },
  );
}

/** One of EMBLEMS' glyphs, centered at (x, y), `size` across. */
function glyph(name: EmblemGlyph, x: number, y: number, size: number, fill: string, stroke: string): string {
  const drawn = EMBLEMS[name];
  const scale = size / EMBLEM_BOX;
  const center = EMBLEM_BOX / 2;
  return group(
    `translate(${fmt(x)} ${fmt(y)}) scale(${fmt(scale)}) translate(-${center} -${center})`,
    path(drawn.d, fill, { stroke, "stroke-width": 1.1, "stroke-linejoin": "round", "fill-rule": drawn.rule }),
  );
}

/** A flame tongue rising from (x, y), `h` tall. */
function flame(x: number, y: number, h: number, outer: string, inner: string): string {
  const w = h * 0.45;
  return (
    path(
      `M${fmt(x - w)} ${fmt(y)}Q${fmt(x - w * 1.2)} ${fmt(y - h * 0.5)} ${fmt(x)} ${fmt(y - h)}` +
        `Q${fmt(x + w * 0.2)} ${fmt(y - h * 0.55)} ${fmt(x + w * 0.6)} ${fmt(y - h * 0.6)}Q${fmt(x + w * 1.2)} ${fmt(y - h * 0.3)} ${fmt(x + w)} ${fmt(y)}Z`,
      outer,
    ) +
    path(`M${fmt(x - w * 0.5)} ${fmt(y)}Q${fmt(x - w * 0.5)} ${fmt(y - h * 0.4)} ${fmt(x)} ${fmt(y - h * 0.62)}Q${fmt(x + w * 0.6)} ${fmt(y - h * 0.3)} ${fmt(x + w * 0.5)} ${fmt(y)}Z`, inner)
  );
}

/** A wooden practice sword, hilt at the origin, pointing up, `length` long. */
function practiceSword(p: Paint, length: number): string {
  return (
    rect(-1.6, -length, 3.2, length - 4, p.wood, inked(p)) +
    rect(-5, -5, 10, 2.4, p.dark, inked(p)) +
    rect(-1.2, -2.6, 2.4, 5, p.dark, inked(p))
  );
}

// The backdrop: a training yard

type Defs = string[];

function backdrop(rng: Rng, p: Paint, family: YardFamily, defs: Defs): string {
  const out: string[] = [];
  defs.push(
    `<linearGradient id="sk" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${p.sky[0]}"/>` +
      `<stop offset="1" stop-color="${p.sky[1]}"/></linearGradient>`,
    `<linearGradient id="gr" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${p.ground[0]}"/>` +
      `<stop offset="1" stop-color="${p.ground[1]}"/></linearGradient>`,
  );
  out.push(rect(0, 0, BOX, BOX, "url(#sk)"));
  const night = FAMILY_PAINTS[family].sky === NIGHT_SKY;
  if (night) {
    for (let i = 0; i < STARS; i += 1) {
      out.push(circle(between(rng, 1, 99), between(rng, 1, HORIZON - 14), between(rng, 0.2, 0.6), "#ffffff", { opacity: between(rng, 0.4, 1) }));
    }
  }
  const clouds = whole(rng, CLOUDS_MIN, CLOUDS_MAX);
  for (let i = 0; i < clouds; i += 1) {
    const cx = between(rng, 6, 94);
    const cy = between(rng, 8, 34);
    const r = between(rng, 4, 7);
    out.push(
      group(
        at(cx, cy),
        ellipse(0, 0, r * 2.2, r * 0.8, "#ffffff") + circle(-r * 0.7, -r * 0.4, r * 0.8, "#ffffff") + circle(r * 0.5, -r * 0.6, r, "#ffffff"),
        { opacity: night ? 0.18 : 0.75 },
      ),
    );
  }
  // The hills behind the yard.
  const hillA = between(rng, 48, 56);
  const hillB = between(rng, 50, 58);
  out.push(
    path(
      `M0 ${fmt(HORIZON)}L0 ${fmt(hillA + 4)}Q${fmt(between(rng, 18, 30))} ${fmt(hillA - 6)} ${fmt(between(rng, 44, 56))} ${fmt(hillB)}` +
        `Q${fmt(between(rng, 70, 84))} ${fmt(hillB - 8)} 100 ${fmt(hillB + 2)}L100 ${fmt(HORIZON)}Z`,
      mixHex(p.sky[1], p.ground[1], 0.55),
      { opacity: 0.85 },
    ),
  );
  // The palisade: pointed stakes across the yard's far side, and the rail that binds them.
  const stakes: string[] = [];
  let x = -between(rng, 0, 3);
  while (x < BOX) {
    const w = between(rng, STAKE_MIN, STAKE_MAX);
    const top = HORIZON - between(rng, 10, 15);
    stakes.push(poly([[x, HORIZON + 1], [x, top + 2.5], [x + w / 2, top], [x + w, top + 2.5], [x + w, HORIZON + 1]], p.wood, inked(p)));
    x += w;
  }
  out.push(...stakes);
  out.push(rect(0, HORIZON - 7, BOX, 1.8, p.dark, { opacity: 0.8 }));
  // The sand floor.
  out.push(rect(0, HORIZON, BOX, BOX - HORIZON, "url(#gr)"));
  for (let i = 0; i < 5; i += 1) {
    const tx = between(rng, 4, 96);
    const ty = between(rng, HORIZON + 4, 96);
    out.push(path(`M${fmt(tx - 1.6)} ${fmt(ty)}L${fmt(tx)} ${fmt(ty - 2.2)}L${fmt(tx + 1.6)} ${fmt(ty)}`, "none", { stroke: p.ground[1], "stroke-width": 0.5, opacity: 0.7 }));
  }
  const baleRoll = rng();
  const baleLeft = rng() < 0.5;
  if (baleRoll < BALE_CHANCE) {
    const bx = baleLeft ? between(rng, 2, 10) : between(rng, 78, 86);
    out.push(rect(bx, HORIZON + 2, 14, 8, "#e0c060", inked(p)));
    out.push(line(bx + 4, HORIZON + 2, bx + 4, HORIZON + 10, "#9a7a2a", 0.6));
    out.push(line(bx + 10, HORIZON + 2, bx + 10, HORIZON + 10, "#9a7a2a", 0.6));
  }
  const pennantRoll = rng();
  const px = between(rng, 6, 94);
  const pennantHue = between(rng, 0, FULL_TURN);
  if (pennantRoll < PENNANT_CHANCE) {
    const top = HORIZON - 30;
    out.push(line(px, HORIZON - 6, px, top, p.dark, 0.9));
    out.push(poly([[px, top], [px + 9, top + 2.5], [px, top + 5]], turnHue("#d2452f", pennantHue), inked(p)));
  }
  return out.join("");
}

// The subjects: each drawn about (0, 0), within about 40 of it.

function dummy(p: Paint): string {
  const out: string[] = [];
  out.push(rect(-2.6, 14, 5.2, 30, p.wood, inked(p)));
  out.push(rect(-28, -6, 56, 5, p.wood, inked(p)));
  for (const side of [-1, 1]) {
    const sx = side * 28;
    out.push(path(`M${sx} -6L${sx + side * 6} -9M${sx} -3.5L${sx + side * 7} -3.5M${sx} -1L${sx + side * 6} 2`, "none", { stroke: p.body, "stroke-width": 1.4, "stroke-linecap": "round" }));
  }
  out.push(ellipse(0, 6, 14, 18, p.body, inked(p)));
  out.push(path("M-12 -4Q0 -1 12 -4M-13 16Q0 19 13 16", "none", { stroke: p.dark, "stroke-width": 1.2 }));
  out.push(circle(0, 6, 7.4, p.light, inked(p)));
  out.push(circle(0, 6, 5, p.accent));
  out.push(circle(0, 6, 2.6, p.light));
  out.push(circle(0, 6, 1.1, p.accent));
  out.push(circle(0, -21, 10, p.body, inked(p)));
  out.push(path("M-11 -26Q-6 -34 0 -31Q6 -35 11 -26", p.light, inked(p)));
  out.push(path("M-6.2 -24.2L-2.8 -20.8M-2.8 -24.2L-6.2 -20.8M2.8 -24.2L6.2 -20.8M6.2 -24.2L2.8 -20.8", "none", { stroke: p.dark, "stroke-width": 1.3, "stroke-linecap": "round" }));
  out.push(path("M-5 -15.5L-2.5 -16.8L0 -15.5L2.5 -16.8L5 -15.5", "none", { stroke: p.dark, "stroke-width": 1 }));
  return out.join("");
}

function ogre(p: Paint, kind: string): string {
  const out: string[] = [];
  if (kind === "club") {
    out.push(group(at(22, 8, 30), rect(-2, -30, 4, 30, p.wood, inked(p)) + ellipse(0, -32, 6.5, 9, p.wood, inked(p)) + circle(-3, -35, 1.2, p.dark) + circle(3, -30, 1.2, p.dark)));
  }
  out.push(ellipse(0, 18, 25, 22, p.body, inked(p)));
  out.push(path("M-18 26Q0 34 18 26", "none", { stroke: p.dark, "stroke-width": 1.2 }));
  out.push(rect(-25, 30, 50, 6, p.accent, inked(p)));
  out.push(circle(0, -12, 15, p.body, inked(p)));
  out.push(path("M-10 -18L-3 -15.5M10 -18L3 -15.5", "none", { stroke: p.dark, "stroke-width": 2.2, "stroke-linecap": "round" }));
  out.push(circle(-5.5, -12.5, 1.8, p.dark));
  out.push(circle(5.5, -12.5, 1.8, p.dark));
  out.push(path("M-7 -3Q0 2 7 -3", "none", { stroke: p.dark, "stroke-width": 1.4 }));
  out.push(poly([[-5.5, -3.6], [-4, -8.5], [-2.5, -3]], p.light, inked(p)));
  out.push(poly([[5.5, -3.6], [4, -8.5], [2.5, -3]], p.light, inked(p)));
  if (kind === "ravager") {
    out.push(path("M-12 -21Q-20 -28 -18 -38Q-13 -30 -8 -25Z", p.light, inked(p)));
    out.push(path("M12 -21Q20 -28 18 -38Q13 -30 8 -25Z", p.light, inked(p)));
  }
  if (kind === "goliath") {
    out.push(poly([[-11, -24], [-11, -32], [-5.5, -27], [0, -34], [5.5, -27], [11, -32], [11, -24]], "#f2c94a", inked(p)));
  }
  if (kind === "boulder") {
    out.push(path("M18 4L30 -2L38 6L36 18L24 20L17 12Z", "#9a9488", inked(p)));
    out.push(path("M24 4L28 10M31 6L33 13", "none", { stroke: "#5a564e", "stroke-width": 0.9 }));
  }
  return out.join("");
}

function knife(rng: Rng, p: Paint, angle: number, x: number, y: number, size: number): string {
  return group(
    at(x, y, angle, size),
    path("M-2 0L-2 -16Q0 -22 2.2 -16L2 0Z", p.light, inked(p)) + line(-0.2, -1, -0.2, -15, p.body, 0.6) + rect(-3.6, 0, 7.2, 1.8, p.dark, inked(p)) + rect(-1.5, 1.8, 3, 7, p.wood, inked(p)),
    { opacity: 0.85 + rng() * 0.15 },
  );
}

function knives(rng: Rng, p: Paint, kind: string): string {
  const out: string[] = [];
  if (kind === "fan") {
    for (let i = 0; i < 7; i += 1) out.push(knife(rng, p, -63 + i * 21, 0, 18, 1.5));
    out.push(circle(0, 18, 3, p.dark, inked(p)));
    return out.join("");
  }
  // The juggler's knives circle the apple on the volunteer's head.
  out.push(circle(0, 6, 9, p.accent, inked(p)));
  out.push(path("M0 -3Q-1 -6 1 -8", "none", { stroke: p.dark, "stroke-width": 1.2 }));
  out.push(path("M1 -6Q5 -9 7 -6Q4 -4 1 -6Z", "#5ab84a", inked(p)));
  out.push(ellipse(-3, 3, 2, 3, "#ffffff", { opacity: 0.45 }));
  out.push(path("M-30 12A30 30 0 0 1 30 12", "none", { stroke: p.light, "stroke-width": 0.6, "stroke-dasharray": "2 2", opacity: 0.8 }));
  for (const [turn, rx, ry] of [[-60, -24, -10], [0, 0, -28], [60, 24, -10]] as const) out.push(knife(rng, p, turn + 90, rx, ry, 1.1));
  return out.join("");
}

function totem(p: Paint): string {
  const out: string[] = [];
  out.push(flame(0, -27, 18, "#ff7a1a", "#ffe04a"));
  out.push(path("M-5 -27Q0 -36 5 -27Z", p.accent));
  const faces: readonly [number, string][] = [[-26, p.body], [-8, p.light], [10, p.body]];
  for (const [y, fill] of faces) {
    out.push(rect(-11, y, 22, 17, fill, inked(p)));
    out.push(rect(-7, y + 4, 4.4, 3, p.dark));
    out.push(rect(2.6, y + 4, 4.4, 3, p.dark));
    out.push(rect(-5, y + 10.5, 10, 2.4, p.accent, inked(p)));
  }
  out.push(path("M-11 -6L-24 -12L-22 -2L-11 0Z", p.accent, inked(p)));
  out.push(path("M11 -6L24 -12L22 -2L11 0Z", p.accent, inked(p)));
  out.push(rect(-13, 27, 26, 5, p.dark, inked(p)));
  return out.join("");
}

function felinorHead(p: Paint, x: number, y: number, s: number, kind: string): string {
  const out: string[] = [];
  out.push(poly([[-13, -6], [-11, -20], [-3, -11]], p.body, inked(p)));
  out.push(poly([[13, -6], [11, -20], [3, -11]], p.body, inked(p)));
  out.push(poly([[-10.5, -8], [-10, -16], [-5, -10.5]], p.light));
  out.push(poly([[10.5, -8], [10, -16], [5, -10.5]], p.light));
  out.push(ellipse(0, 0, 14, 12.5, p.body, inked(p)));
  out.push(ellipse(0, 5, 7, 5, p.light));
  out.push(ellipse(-5, -2, 2.6, 3, p.accent, inked(p)));
  out.push(ellipse(5, -2, 2.6, 3, p.accent, inked(p)));
  out.push(ellipse(-5, -2, 0.8, 2.4, p.line));
  out.push(ellipse(5, -2, 0.8, 2.4, p.line));
  out.push(poly([[-1.6, 2.6], [1.6, 2.6], [0, 4.4]], p.dark));
  out.push(path("M-3 7Q0 9 0 4.4Q0 9 3 7", "none", { stroke: p.dark, "stroke-width": 0.7 }));
  out.push(path("M-6 4L-16 2M-6 5.5L-16 6.5M6 4L16 2M6 5.5L16 6.5", "none", { stroke: p.dark, "stroke-width": 0.5 }));
  if (kind === "guerilla") {
    out.push(path("M-13.5 -6Q0 -12 13.5 -6L13.5 -3Q0 -9 -13.5 -3Z", "#4a5a2a", inked(p)));
    out.push(path("M13 -5L19 -1L17 -7Z", "#4a5a2a", inked(p)));
  }
  return group(at(x, y, 0, s), out.join(""));
}

function felinor(p: Paint, kind: string): string {
  const out: string[] = [];
  if (kind === "four") {
    for (const [x, y] of [[-15, -12], [15, -12], [-15, 16], [15, 16]] as const) out.push(felinorHead(p, x, y, 0.85, kind));
    return out.join("");
  }
  out.push(path("M-20 40Q-22 18 -10 12L10 12Q22 18 20 40Z", p.body, inked(p)));
  out.push(path("M-6 14L0 24L6 14", "none", { stroke: p.light, "stroke-width": 1.4 }));
  if (kind === "flaming") {
    out.push(flame(-14, -10, 20, "#ff7a1a", "#ffe04a"));
    out.push(flame(14, -10, 20, "#ff7a1a", "#ffe04a"));
    out.push(flame(0, -16, 16, "#ff7a1a", "#ffe04a"));
  }
  if (kind === "paw") {
    out.push(path("M16 16Q26 10 26 -6", "none", { stroke: p.line, "stroke-width": 7.2, "stroke-linecap": "round" }));
    out.push(path("M16 16Q26 10 26 -6", "none", { stroke: p.body, "stroke-width": 5.6, "stroke-linecap": "round" }));
    out.push(circle(26, -10, 5, p.body, inked(p)));
    for (const [tx, ty] of [[22.6, -14], [26, -15.6], [29.4, -14]] as const) out.push(circle(tx, ty, 1.3, p.light));
  }
  out.push(felinorHead(p, 0, -4, 1.1, kind));
  return out.join("");
}

/** A rune's sigil, a zigzag that a glow traces. */
const SIGIL = "M-8 -10L4 -10L-4 0L8 0L-6 12M-2 -16L-2 -12M2 14L2 18";

function rune(p: Paint, kind: string): string {
  const out: string[] = [];
  out.push(circle(0, 2, 22, p.glow, { opacity: 0.18 }));
  if (kind === "runes") {
    out.push(path("M-15 26L-17 -16Q-16 -26 0 -27Q16 -26 17 -16L15 26Z", p.body, inked(p)));
    out.push(path("M-12 -14L-6 -20M10 22L14 18", "none", { stroke: p.dark, "stroke-width": 0.8 }));
    out.push(path(SIGIL, "none", { stroke: p.glow, "stroke-width": 4, opacity: 0.35, "stroke-linecap": "round" }));
    out.push(path(SIGIL, "none", { stroke: p.glow, "stroke-width": 1.6, "stroke-linecap": "round" }));
  } else {
    out.push(ellipse(0, 18, 26, 8, p.dark, inked(p)));
    out.push(ellipse(0, 16, 22, 6, p.body, inked(p)));
    out.push(group(at(0, 16, 0, 1), group("scale(1 0.32)", path(SIGIL, "none", { stroke: p.glow, "stroke-width": 3.4, "stroke-linecap": "round" }))));
    out.push(path("M-8 4Q-12 -10 -2 -16", "none", { stroke: p.dark, "stroke-width": 1.4 }));
    out.push(circle(-2, -16, 4.5, p.dark, inked(p)));
    out.push(twinkle(1, -21, 5, p.accent, 1));
  }
  // The burst it is waiting to let off.
  out.push(twinkle(-18, -22, 4, p.accent, 0.9));
  out.push(twinkle(19, -12, 3, p.accent, 0.8));
  return out.join("");
}

function poster(p: Paint, kind: string): string {
  const out: string[] = [];
  out.push(rect(-26, -10, 4, 50, p.wood, inked(p)));
  out.push(rect(22, -10, 4, 50, p.wood, inked(p)));
  out.push(rect(-30, -30, 60, 40, p.wood, inked(p)));
  out.push(path("M-30 -20H30M-30 -8H30M-30 2H30", "none", { stroke: p.dark, "stroke-width": 0.5, opacity: 0.6 }));
  out.push(group(at(0, -10, -4), rect(-17, -18, 34, 36, p.light, inked(p)) + circle(-14, -15, 1.4, p.glow) + circle(14, -15, 1.4, p.glow) + pictogram(p, kind)));
  return out.join("");
}

function pictogram(p: Paint, kind: string): string {
  switch (kind) {
    case "rations":
      return (
        ellipse(0, 6, 12, 4, p.dark, inked(p)) +
        path("M-8 4Q-10 -6 -2 -8Q6 -10 6 -2Z", "#b8642a", inked(p)) +
        rect(4, -6, 7, 2.4, p.light, inked(p)) +
        circle(11.5, -6.6, 1.6, p.light, inked(p)) +
        circle(11.5, -3.6, 1.6, p.light, inked(p))
      );
    case "level":
      return [0, 1, 2].map((i) => path(`M-10 ${10 - i * 8}L0 ${2 - i * 8}L10 ${10 - i * 8}`, "none", { stroke: p.accent, "stroke-width": 3.4, "stroke-linecap": "round", "stroke-linejoin": "round" })).join("");
    case "recruit":
      return (
        circle(0, -6, 4.5, p.dark) +
        path("M-8 12L-6 2Q0 -1 6 2L8 12Z", p.dark) +
        path("M6 3L13 -2", "none", { stroke: p.dark, "stroke-width": 2.4, "stroke-linecap": "round" }) +
        twinkle(-10, -10, 3.6, p.glow, 1)
      );
    case "recycle":
      return [0, 120, 240]
        .map((turn) => group(at(0, 0, turn), path("M-3 -10A10 10 0 0 1 8.6 -5L6 -3.4L12 -1.6L12.6 -8L10.4 -6.4A12 12 0 0 0 -3.6 -12Z", "#3a8a3a", inked(p))))
        .join("");
    default:
      return path("M-11 -10H11M-11 -4H11M-11 2H7M-11 8H9", "none", { stroke: p.dark, "stroke-width": 1.4 });
  }
}

function golem(p: Paint): string {
  const out: string[] = [];
  out.push(rect(-30, -8, 12, 28, p.body, inked(p)));
  out.push(rect(18, -8, 12, 28, p.body, inked(p)));
  out.push(rect(-31, 18, 14, 9, p.dark, inked(p)));
  out.push(rect(17, 18, 14, 9, p.dark, inked(p)));
  out.push(rect(-16, 22, 12, 18, p.dark, inked(p)));
  out.push(rect(4, 22, 12, 18, p.dark, inked(p)));
  out.push(path("M-20 -12L20 -12L17 26L-17 26Z", p.body, inked(p)));
  out.push(path("M-20 -12L-4 -2M20 -12L6 0M-17 18L-6 12M17 22L8 14", "none", { stroke: p.dark, "stroke-width": 0.8 }));
  out.push(circle(0, 6, 5.5, p.glow, { opacity: 0.35 }));
  out.push(path("M-3 2L3 2L0 10Z", p.glow));
  out.push(rect(-10, -30, 20, 17, p.light, inked(p)));
  out.push(rect(-6.5, -25, 4.4, 2.6, p.glow));
  out.push(rect(2.1, -25, 4.4, 2.6, p.glow));
  out.push(rect(-4.5, -18.5, 9, 1.8, p.dark));
  return out.join("");
}

function chaos(rng: Rng, p: Paint): string {
  const out: string[] = [];
  for (let i = 0; i < 3; i += 1) {
    const r = 26 + i * 6;
    const start = between(rng, 0, FULL_TURN) * DEGREES;
    const end = start + between(rng, 120, 200) * DEGREES;
    out.push(
      path(`M${fmt(r * Math.cos(start))} ${fmt(r * Math.sin(start))}A${r} ${r} 0 0 1 ${fmt(r * Math.cos(end))} ${fmt(r * Math.sin(end))}`, "none", {
        stroke: p.glow,
        "stroke-width": 1.6 - i * 0.4,
        "stroke-linecap": "round",
        opacity: 0.8,
      }),
    );
  }
  const top: readonly Pt[] = [[0, -22], [19, -11], [0, 0], [-19, -11]];
  const left: readonly Pt[] = [[-19, -11], [0, 0], [0, 22], [-19, 11]];
  const right: readonly Pt[] = [[0, 0], [19, -11], [19, 11], [0, 22]];
  out.push(poly(top, p.light, inked(p)));
  out.push(poly(left, p.body, inked(p)));
  out.push(poly(right, mixHex(p.body, p.dark, 0.35), inked(p)));
  out.push(ellipse(0, -11, 2.4, 1.4, p.accent));
  for (const [x, y] of [[-14, -2], [-9.5, 5.5], [-5, 13]] as const) out.push(ellipse(x, y, 1.4, 2, p.accent));
  for (const [x, y] of [[5, 2], [14, -3], [5, 14], [14, 9]] as const) out.push(ellipse(x, y, 1.4, 2, p.accent));
  out.push(twinkle(-26, -24, 4, p.glow, 1));
  out.push(twinkle(27, 18, 3, p.glow, 0.9));
  return out.join("");
}

function sun(p: Paint, x: number, y: number, r: number): string {
  const out: string[] = [];
  for (let i = 0; i < 12; i += 1) {
    const a = (i * 30) * DEGREES;
    const b = a + 6 * DEGREES;
    const c = a - 6 * DEGREES;
    out.push(poly([[x + r * 1.15 * Math.cos(b), y + r * 1.15 * Math.sin(b)], [x + r * 1.7 * Math.cos(a), y + r * 1.7 * Math.sin(a)], [x + r * 1.15 * Math.cos(c), y + r * 1.15 * Math.sin(c)]], p.body, inked(p)));
  }
  out.push(circle(x, y, r * 1.3, p.glow, { opacity: 0.4 }));
  out.push(circle(x, y, r, p.body, inked(p)));
  out.push(circle(x - r * 0.3, y - r * 0.3, r * 0.45, p.light, { opacity: 0.7 }));
  return out.join("");
}

function moon(p: Paint, x: number, y: number, r: number): string {
  return (
    circle(x, y, r * 1.35, p.accent, { opacity: 0.25 }) +
    path(`M${fmt(x + r * 0.2)} ${fmt(y - r)}A${fmt(r)} ${fmt(r)} 0 1 0 ${fmt(x + r * 0.2)} ${fmt(y + r)}A${fmt(r * 0.75)} ${fmt(r * 0.9)} 0 1 1 ${fmt(x + r * 0.2)} ${fmt(y - r)}Z`, p.accent, inked(p)) +
    circle(x - r * 0.45, y - r * 0.1, r * 0.12, p.dark, { opacity: 0.35 }) +
    circle(x - r * 0.25, y + r * 0.45, r * 0.09, p.dark, { opacity: 0.35 })
  );
}

function sky(p: Paint, kind: string): string {
  if (kind === "sun") return sun(p, 0, -6, 16);
  if (kind === "moon") return moon({ ...p, accent: p.body }, 0, -6, 20);
  // The caller: a raised staff between the two lights it calls.
  return (
    sun(p, -16, -14, 10) +
    moon(p, 18, -16, 12) +
    line(0, 36, 0, -6, p.wood, 2.6) +
    circle(0, -8, 4, p.glow, inked(p)) +
    twinkle(0, -8, 9, p.light, 0.9)
  );
}

function seance(p: Paint): string {
  const out: string[] = [];
  out.push(path("M-12 -2Q-16 -24 0 -32Q16 -24 12 -2Q8 4 6 -2Q3 4 0 -2Q-3 4 -6 -2Q-8 4 -12 -2Z", p.light, { ...inked(p), opacity: 0.85 }));
  out.push(ellipse(-4.5, -18, 2, 3, p.dark));
  out.push(ellipse(4.5, -18, 2, 3, p.dark));
  out.push(ellipse(0, -10, 2.4, 3, p.dark));
  out.push(ellipse(0, 32, 30, 6, p.dark, inked(p)));
  for (const [x, h] of [[-18, 18], [0, 24], [18, 16]] as const) {
    out.push(rect(x - 3, 32 - h, 6, h, p.accent, inked(p)));
    out.push(circle(x, 30 - h, 6, p.glow, { opacity: 0.3 }));
    out.push(flame(x, 32 - h, 8, "#ffb02a", "#fff2a0"));
  }
  return out.join("");
}

function bean(p: Paint): string {
  return (
    group(
      at(0, 2, -18),
      path("M-20 -6Q-22 -20 -6 -18Q4 -16 10 -20Q24 -24 24 -6Q24 14 4 16Q-18 18 -20 -6Z", p.body, { ...inked(p), "stroke-width": 1.2 }) +
        ellipse(-6, -10, 8, 3.4, p.light, { opacity: 0.7 }),
    ) +
    circle(-6, 0, 2, p.accent) +
    circle(6, -2, 2, p.accent) +
    circle(-5.4, -0.6, 0.6, p.glow) +
    circle(6.6, -2.6, 0.6, p.glow) +
    path("M-4 6Q1 10 6 5", "none", { stroke: p.accent, "stroke-width": 1.2, "stroke-linecap": "round" }) +
    twinkle(20, -22, 4, "#fff6a0", 1) +
    twinkle(-24, -18, 2.6, "#fff6a0", 0.8)
  );
}

function gate(p: Paint): string {
  const out: string[] = [];
  for (let i = 0; i < 3; i += 1) {
    const y = 14 + i * 6;
    out.push(path(`M-34 ${y}Q-26 ${y - 3} -17 ${y}T0 ${y}T17 ${y}T34 ${y}L34 ${y + 6}L-34 ${y + 6}Z`, i === 0 ? p.light : p.body, { opacity: 0.9 }));
  }
  out.push(rect(-26, -30, 6, 46, p.wood, inked(p)));
  out.push(rect(20, -30, 6, 46, p.wood, inked(p)));
  out.push(rect(-30, -34, 60, 6, p.wood, inked(p)));
  out.push(rect(-20, -24, 40, 26, mixHex(p.wood, p.dark, 0.25), inked(p)));
  for (const y of [-17, -10, -3]) out.push(line(-20, y, 20, y, p.line, 0.6));
  out.push(path("M-16 2Q-18 10 -14 16M0 2Q-2 10 2 16M16 2Q18 10 14 16", "none", { stroke: p.accent, "stroke-width": 1.4, "stroke-linecap": "round" }));
  out.push(circle(0, -40, 4, p.dark, inked(p)));
  out.push(line(0, -36, 0, -34, p.dark, 1.4));
  return out.join("");
}

function recruit(rng: Rng, p: Paint, kind: string): string {
  const out: string[] = [];
  const skin = p.glow;
  const head = (x: number, y: number, s: number): string =>
    group(
      at(x, y, 0, s),
      circle(0, 0, 10, skin, inked(p)) +
        circle(-3.4, 0.4, 1.2, p.line) +
        circle(3.4, 0.4, 1.2, p.line) +
        path("M-3 5Q0 7 3 5", "none", { stroke: p.line, "stroke-width": 0.9, "stroke-linecap": "round" }) +
        (kind === "trainer"
          ? rect(-10.4, -6.4, 20.8, 3.4, p.accent, inked(p))
          : kind === "master"
            ? poly([[-14, -8], [0, -14], [14, -8], [0, -3]], p.dark, inked(p)) + rect(-7, -8.4, 14, 3.4, p.dark) + line(12, -8, 13, 0, "#f2c94a", 0.9)
            : kind === "west"
              ? ellipse(0, -6.4, 17, 3, p.wood, inked(p)) + path("M-8 -6Q-8 -16 0 -15Q8 -16 8 -6Z", p.wood, inked(p))
              : kind === "east"
                ? poly([[-17, -4.4], [0, -16], [17, -4.4]], "#d8b45a", inked(p))
                : path("M-10.6 -1.6A10.6 10.6 0 0 1 10.6 -1.6Z", p.accent, inked(p)) + rect(-1.4, -14, 2.8, 4, p.body, inked(p))),
    );
  // The tunic, belt and shoulders.
  out.push(path("M-17 40L-19 10Q-17 2 -8 0L8 0Q17 2 19 10L17 40Z", p.body, inked(p)));
  out.push(rect(-17, 22, 34, 3.4, p.dark, inked(p)));
  out.push(rect(-2, 22, 4, 3.4, "#f2c94a", inked(p)));
  if (kind === "leader") {
    out.push(line(-26, 40, -26, -34, p.wood, 1.6));
    out.push(path("M-26 -34L-6 -30L-26 -22Z", p.accent, inked(p)));
  }
  if (kind === "spiky") {
    for (const side of [-1, 1]) {
      out.push(poly([[side * 10, 2], [side * 20, -6], [side * 15, 6]], p.accent, inked(p)));
      out.push(poly([[side * 15, 6], [side * 24, 2], [side * 18, 12]], p.accent, inked(p)));
    }
  }
  if (kind === "three") {
    out.push(head(-14, -12, 0.75));
    out.push(head(14, -12, 0.75));
    out.push(head(0, -18, 0.9));
  } else {
    out.push(head(0, -12, 1.05));
  }
  if (kind === "spiky") {
    for (const tx of [-7, 0, 7]) out.push(poly([[tx - 2.4, -22], [tx, -30], [tx + 2.4, -22]], p.accent, inked(p)));
  }
  if (kind === "trainer") {
    out.push(path("M0 2Q6 10 10 12", "none", { stroke: p.dark, "stroke-width": 0.7 }));
    out.push(rect(9, 11, 6, 3.4, "#c0c8d0", inked(p)));
  }
  if (kind === "master") {
    out.push(group(at(18, 18, -20), rect(-3, -10, 6, 20, p.light, inked(p)) + ellipse(0, -10, 4, 1.6, p.light, inked(p)) + ellipse(0, 10, 4, 1.6, p.light, inked(p))));
  }
  if (kind === "protector") {
    out.push(path("M-14 -2L14 -2L14 22Q14 34 0 40Q-14 34 -14 22Z", p.accent, { ...inked(p), "stroke-width": 1.2 }));
    out.push(path("M0 2L0 34M-10 14L10 14", "none", { stroke: p.dark, "stroke-width": 2 }));
  } else {
    out.push(circle(-20, 18, 9, p.wood, inked(p)));
    out.push(circle(-20, 18, 3, p.dark, inked(p)));
    out.push(group(at(20, 22, 12 + between(rng, -6, 6)), practiceSword(p, 30)));
  }
  return out.join("");
}

function target(rng: Rng, p: Paint, name: string | undefined): string {
  const motif = motifFor(name);
  const emblem: EmblemGlyph = motif === null ? "star" : MOTIFS[motif].glyph;
  const out: string[] = [];
  out.push(line(-14, 40, -2, 6, p.wood, 2.4));
  out.push(line(14, 40, 2, 6, p.wood, 2.4));
  out.push(line(0, 40, 0, 4, p.wood, 2.2));
  out.push(circle(0, -4, 27, p.light, { ...inked(p), "stroke-width": 1.2 }));
  out.push(circle(0, -4, 22, p.body));
  out.push(circle(0, -4, 17, p.light));
  out.push(circle(0, -4, 12.5, p.body, inked(p)));
  out.push(glyph(emblem, 0, -4, 18, p.light, p.dark));
  const arrows = whole(rng, 1, 3);
  for (let i = 0; i < arrows; i += 1) {
    const ax = between(rng, -16, 16);
    const ay = between(rng, -20, 10);
    const turn = between(rng, -30, 30);
    out.push(group(at(ax, ay, turn), line(0, 0, 13, -4, p.wood, 1.4) + poly([[13, -4], [17, -7.6], [18, -3], [16, -1]], p.accent)));
  }
  return out.join("");
}

function subjectMarkup(rng: Rng, p: Paint, subject: YardSubject, name: string | undefined): string {
  const { family, kind } = subject;
  switch (family) {
    case "dummy":
      return dummy(p);
    case "ogre":
      return ogre(p, kind);
    case "knife":
      return knives(rng, p, kind);
    case "totem":
      return totem(p);
    case "felinor":
      return felinor(p, kind);
    case "rune":
      return rune(p, kind);
    case "poster":
      return poster(p, kind);
    case "golem":
      return golem(p);
    case "chaos":
      return chaos(rng, p);
    case "sky":
      return sky(p, kind);
    case "seance":
      return seance(p);
    case "bean":
      return bean(p);
    case "gate":
      return gate(p);
    case "recruit":
      return recruit(rng, p, kind);
    case "target":
      return target(rng, p, name);
  }
}

// The document

const SUBJECT_SCALES: Readonly<Partial<Record<YardFamily, number>>> = { ogre: 1.06, golem: 1.08, poster: 1.05 };

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

/** The SVG document a Training Grounds face draws, unencoded. */
export function trainingGroundsSvg(defId: string, name: string | undefined, tags: readonly Tag[], type: CardType, radiant: boolean): string {
  const subject = yardSubject(defId, name, tags, type);
  const seed = hashId(defId);
  const based = drift(seed, subject, FAMILY_PAINTS[subject.family]);
  const p = radiant ? gild(based) : based;
  const behind = seededRandom((seed ^ BACKDROP_SALT) >>> 0);
  const ahead = seededRandom((seed ^ SUBJECT_SALT) >>> 0);
  const tilt = between(ahead, -SUBJECT_TILT_JITTER, SUBJECT_TILT_JITTER);
  const scale = (SUBJECT_SCALES[subject.family] ?? 1) * (1 + between(ahead, -SUBJECT_SCALE_JITTER, SUBJECT_SCALE_JITTER));
  const defs: Defs = [];
  defs.push(
    `<radialGradient id="v" gradientUnits="userSpaceOnUse" cx="${HALF}" cy="${HALF}" r="${VIGNETTE_RADIUS}">` +
      `<stop offset="${VIGNETTE_START}" stop-color="#000" stop-opacity="0"/>` +
      `<stop offset="1" stop-color="#000" stop-opacity="${VIGNETTE_OPACITY}"/></radialGradient>`,
  );
  const yard = backdrop(behind, p, subject.family, defs);
  const drawn = subjectMarkup(ahead, p, subject, name);
  return (
    `<svg viewBox="0 0 ${BOX} ${BOX}" width="${BOX}" height="${BOX}" xmlns="http://www.w3.org/2000/svg" preserveAspectRatio="xMidYMid slice">` +
    `<defs>${defs.join("")}</defs>` +
    yard +
    (radiant ? rays(seed) : "") +
    ellipse(SUBJECT_X, SUBJECT_Y + 40, 30, 4.5, "#000000", { opacity: 0.2 }) +
    group(at(SUBJECT_X, SUBJECT_Y, tilt, scale), drawn) +
    rect(0, 0, BOX, BOX, "url(#v)") +
    "</svg>"
  );
}
