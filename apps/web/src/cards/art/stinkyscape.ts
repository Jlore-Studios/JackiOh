// R2170000: the Stinkyscape set's own procedural art (issue #2170). Every `stinkyscape-` entry draws
// here instead of `artSpec`: a space backdrop (stars, nebulae, a planet) under one subject that its
// family fixes and its name's word details, so the Drone Cores and their drones share one drone, the
// Starships and their pieces one fleet, and each Weapon is the weapon it names. Presentation only
// (CLAUDE.md rule 7): a pure function of the card's id, name, tags and type, with geometry drawn from
// streams the Radiant face shares, so the two faces differ only in their paint and the gold rays.
// No other set reaches this file, so no shipped card's picture moves (R503).

import type { CardType, Tag } from "@jackioh/shared";

import { EMBLEM_BOX, EMBLEMS, type EmblemGlyph } from "./emblems.ts";
import { fmt, hashId, seededRandom } from "./hash.ts";
import { MOTIFS, motifFor } from "./motifs.ts";
import { mixHex, RADIANT_PALETTE, turnHue } from "./themes.ts";

export const STINKYSCAPE_PREFIX = "stinkyscape-";

/** Whether an id draws the set's own art: the set's entries and tokens, nothing else. */
export function isStinkyscapeArt(defId: string): boolean {
  return defId.startsWith(STINKYSCAPE_PREFIX);
}

// The families

export type StinkyFamily =
  | "drone" | "fleet" | "weapon" | "miner" | "resource" | "anomaly" | "doctrine" | "terminus"
  | "swarm" | "acclaimed" | "jlockeed" | "ai" | "crew" | "space";

export type WeaponKind =
  | "cannon" | "repeater" | "flak" | "beam" | "ray" | "rail" | "triple" | "missiles" | "launcher"
  | "rounds" | "mine" | "tank" | "tractor";

export type StinkySubject = { family: StinkyFamily; kind: string; tier: number };

/**
 * The cards printed with the Starship Piece keyword. A face's art reads no keywords, so the set's
 * list is kept here, and `stinkyscape.test.ts` holds it to the catalog's.
 */
export const STARSHIP_PIECES: ReadonlySet<string> = new Set(
  [
    "038", "040", "043", "048", "049", "050", "051", "052", "053", "054", "055", "056", "057", "058", "059",
    "060", "061", "062", "063", "064", "065", "066", "067", "079", "090", "093", "098", "100", "101", "104",
    "105", "106", "107", "108", "122", "131", "132", "133", "134", "136", "140", "141", "142", "143", "150-1",
    "175", "177", "t-ai-10",
  ].map((suffix) => `${STINKYSCAPE_PREFIX}${suffix}`),
);

/**
 * The ships #2521 took the Starship Piece keyword from (Phalanx, the Prototypes, the Paragons, Halo,
 * Galaxy, Radix, Ozark, Falcon, Yuma, the Badgers and Stratos): still ships, so still drawn as the fleet
 * they shipped as (R503).
 */
const FORMER_STARSHIP_PIECES: ReadonlySet<string> = new Set(
  ["068", "069", "072", "073", "074", "075", "076", "077", "078", "080", "081", "082", "083", "085", "086"].map(
    (suffix) => `${STINKYSCAPE_PREFIX}${suffix}`,
  ),
);

/** First match wins: each weapon the set names, word by word. */
export const WEAPON_WORDS: readonly (readonly [string, WeaponKind])[] = [
  ["missile", "missiles"],
  ["launcher", "launcher"],
  ["rounds", "rounds"],
  ["siege tank", "tank"],
  ["tractor beam", "tractor"],
  ["yamato", "cannon"],
  ["cannon", "cannon"],
  ["repeater", "repeater"],
  ["flak", "flak"],
  ["beam", "beam"],
  ["freeze ray", "ray"],
  ["disintegrex", "ray"],
  ["rr-5000", "rail"],
  ["trila", "triple"],
  ["spacemine", "mine"],
];

/** A piece of a ship that is not a weapon, by the word its name holds. */
const PIECE_WORDS: readonly (readonly [string, string])[] = [
  ["dormant", "dormant"],
  ["megabase", "station"],
  ["shield", "shield"],
  ["plating", "plate"],
  ["hull", "plate"],
  ["crystal", "crystal"],
  ["capsule", "pod"],
  ["chamber", "pod"],
  ["burner", "engine"],
  ["chassis", "engine"],
  ["overdrive", "engine"],
  ["warpfield", "module"],
  ["capacitor", "module"],
  ["module", "module"],
  ["nanobots", "module"],
  ["stimulus", "module"],
];

/** The fleet's cards that print no Starship Piece: the hull waiting to launch and the base. */
const FLEET_NAMES: readonly string[] = ["dormant starship", "megabase"];
const FIGHTER_NAMES: readonly string[] = ["falcon", "stratos", "yuma", "golem", "honey badger"];
const BATTLESHIP_NAMES: readonly string[] = ["paragon-x", "halo", "prototype-x2", "phalanx", "galaxy"];
const MINER_NAMES: readonly string[] = ["miner", "ozark", "orca", "galaxy"];

const RESOURCE_WORDS: readonly (readonly [string, string])[] = [
  ["asteroid field", "field"],
  ["spice tower", "tower"],
  ["asteroid", "asteroid"],
  ["meteor", "comet"],
  ["comet", "comet"],
  ["reknite", "reknite"],
  ["korrelite", "korrelite"],
  ["gellium", "gel"],
  ["axnit", "axnit"],
  ["red narcor", "red-narcor"],
  ["narcor", "narcor"],
  ["spice", "spice"],
  ["metal scraps", "scrap"],
  ["power cell", "cell"],
];

const ANOMALY_WORDS: readonly (readonly [string, string])[] = [
  ["anomaly", "vortex"],
  ["scanning probe", "probe"],
  ["wormhole", "wormhole"],
];

const DOCTRINE_WORDS: readonly (readonly [string, string])[] = [
  ["land", "land"],
  ["sea", "sea"],
  ["air", "air"],
  ["space", "space"],
  ["firststrike", "strike"],
];

const ROMAN: Readonly<Record<string, number>> = { I: 1, V: 5, X: 10 };
const TIER_MAX = 10;

/** A name's mark: v3, Mk2, X1 or a Roman numeral at its end; 1 without one. */
export function tierOf(name: string): number {
  const digits = /(?:\bv|\bmk|-x)(\d+)\b/i.exec(name);
  if (digits !== null) return Math.min(TIER_MAX, Number(digits[1]));
  const roman = /\b([IVX]+)$/.exec(name.trim());
  if (roman === null) return 1;
  const letters = [...(roman[1] ?? "")].map((letter) => ROMAN[letter] ?? 0);
  let total = 0;
  letters.forEach((value, index) => {
    total += value < (letters[index + 1] ?? 0) ? -value : value;
  });
  return Math.max(1, Math.min(TIER_MAX, total));
}

function wordIn(lower: string, table: readonly (readonly [string, string])[]): string | null {
  for (const [word, kind] of table) if (lower.includes(word)) return kind;
  return null;
}

function wordsHave(lower: string, word: string): boolean {
  return new RegExp(`(^|[^a-z])${word}([^a-z]|$)`).test(lower);
}

function fleetKind(lower: string): string {
  const piece = wordIn(lower, PIECE_WORDS);
  if (piece !== null) return piece;
  if (FIGHTER_NAMES.some((ship) => lower.includes(ship))) return "fighter";
  if (BATTLESHIP_NAMES.some((ship) => lower.includes(ship))) return "battleship";
  return "cruiser";
}

/** The family a Stinkyscape card draws from, and the detail its name picks inside it. */
export function stinkySubject(defId: string, name: string | undefined, tags: readonly Tag[], type: CardType): StinkySubject {
  const lower = (name ?? "").toLowerCase();
  const tier = tierOf(name ?? "");
  const weapon = wordIn(lower, WEAPON_WORDS);
  const has = (tag: Tag): boolean => tags.includes(tag);
  if (has("AI")) return { family: "ai", kind: type === "Unit" ? "unit" : "chip", tier };
  if (has("Terminus") || lower.includes("olivian")) return { family: "terminus", kind: "hole", tier };
  if (has("War Doctrine") || lower.includes("firststrike")) {
    const doctrine = DOCTRINE_WORDS.find(([word]) => wordsHave(lower, word))?.[1] ?? "strike";
    return { family: "doctrine", kind: doctrine, tier };
  }
  if (has("Drone Core") || wordsHave(lower, "drone")) {
    const kind = has("Drone Core")
      ? "core"
      : lower.includes("interceptor")
        ? "interceptor"
        : lower.includes("corvette")
          ? "corvette"
          : lower.includes("battleship")
            ? lower.includes("shield")
              ? "shield"
              : lower.includes("propagation")
                ? "propagation"
                : "meltdown"
            : "hive";
    return { family: "drone", kind, tier };
  }
  if (has("Swarm") || lower.includes("swarm")) {
    const kind = lower.includes("hornet") ? "hornet" : lower.includes("wasp") ? "wasp" : "swarm";
    return { family: "swarm", kind, tier };
  }
  if (has("Acclaimed")) return { family: "acclaimed", kind: has("Felinor") ? "felinor" : "pilot", tier };
  if (has("Jlockeed")) return { family: "jlockeed", kind: weapon ?? "blueprint", tier };
  if (weapon !== null || has("Weapon")) return { family: "weapon", kind: weapon ?? "cannon", tier };
  if (MINER_NAMES.some((miner) => lower.includes(miner))) {
    return { family: "miner", kind: lower.includes("miner") ? "drill" : "ship", tier };
  }
  const resource = wordIn(lower, RESOURCE_WORDS);
  if (resource !== null) return { family: "resource", kind: resource, tier };
  const anomaly = wordIn(lower, ANOMALY_WORDS);
  if (anomaly !== null) return { family: "anomaly", kind: anomaly, tier };
  if (STARSHIP_PIECES.has(defId) || FORMER_STARSHIP_PIECES.has(defId) || FLEET_NAMES.some((ship) => lower.includes(ship))) {
    return { family: "fleet", kind: fleetKind(lower), tier };
  }
  if (type === "Unit") return { family: "crew", kind: has("Felinor") ? "felinor" : "pilot", tier };
  return { family: "space", kind: type === "Spell" ? "planet" : type === "Field Spell" ? "horizon" : "reticle", tier };
}

// Paint

type Paint = {
  space: readonly [string, string];
  nebula: readonly [string, string];
  /** The main surface's light, middle and shadow tones. */
  light: string;
  hull: string;
  dark: string;
  accent: string;
  glow: string;
  line: string;
};

const OUTLINE = "#0a0d14";

export const FAMILY_PAINTS: Readonly<Record<StinkyFamily, Paint>> = {
  drone: { space: ["#0b2a33", "#03080c"], nebula: ["#1fb5a8", "#2a4fb8"], light: "#c5d3df", hull: "#6b7a8c", dark: "#2a323e", accent: "#ff6b3d", glow: "#43f5d2", line: OUTLINE },
  fleet: { space: ["#141d3d", "#04060f"], nebula: ["#4f5dd8", "#a13fa8"], light: "#d9e1ea", hull: "#7d8ba0", dark: "#323c4d", accent: "#d8443a", glow: "#ffb45a", line: OUTLINE },
  weapon: { space: ["#2b1214", "#090304"], nebula: ["#d0482a", "#7a2a8f"], light: "#c9ced6", hull: "#5f6670", dark: "#24282e", accent: "#f0c03a", glow: "#ff9f3a", line: OUTLINE },
  miner: { space: ["#2a2310", "#080604"], nebula: ["#c98a1f", "#4d6b2a"], light: "#ffe28a", hull: "#d9a52a", dark: "#3a2f14", accent: "#1d1d1d", glow: "#ffcf5a", line: OUTLINE },
  resource: { space: ["#1e1a2c", "#060509"], nebula: ["#7e4fc0", "#3a8f8a"], light: "#b9ab98", hull: "#7a6b5a", dark: "#2e261e", accent: "#ff5a5a", glow: "#ffd0a0", line: OUTLINE },
  anomaly: { space: ["#24093a", "#05020a"], nebula: ["#d43fd0", "#3fd0d4"], light: "#f0d8ff", hull: "#8a5ab8", dark: "#2a1240", accent: "#3fe8ff", glow: "#ff8af5", line: OUTLINE },
  doctrine: { space: ["#1d2414", "#050704"], nebula: ["#8a9a3a", "#3a6a8a"], light: "#c8cf9a", hull: "#6b7440", dark: "#262b14", accent: "#f2c94a", glow: "#fff0a0", line: OUTLINE },
  terminus: { space: ["#120a06", "#000000"], nebula: ["#a33a12", "#3a1a6a"], light: "#fff2cc", hull: "#ff9a2a", dark: "#000000", accent: "#ffcf6a", glow: "#ffd28a", line: OUTLINE },
  swarm: { space: ["#1a2408", "#050802"], nebula: ["#9ad42a", "#d4a82a"], light: "#f2e27a", hull: "#d4b21a", dark: "#1c1a08", accent: "#2a2a12", glow: "#d9ff5a", line: OUTLINE },
  acclaimed: { space: ["#1a1838", "#05040c"], nebula: ["#5a6ad8", "#d8a03a"], light: "#f2f4f8", hull: "#3d5a9a", dark: "#16213d", accent: "#ffd24a", glow: "#7fd8ff", line: OUTLINE },
  jlockeed: { space: ["#1d4c8f", "#0b2550"], nebula: ["#3a7ad0", "#2a5aa8"], light: "#e6f3ff", hull: "#4a86d0", dark: "#1a3f78", accent: "#ffffff", glow: "#bfe4ff", line: "#e6f3ff" },
  ai: { space: ["#062418", "#010805"], nebula: ["#1fd47a", "#1f7ad4"], light: "#9af5c4", hull: "#1f5a3d", dark: "#0a2418", accent: "#5affb0", glow: "#5affb0", line: OUTLINE },
  crew: { space: ["#16203a", "#04060c"], nebula: ["#3a6ad8", "#a03ad8"], light: "#eef1f5", hull: "#c75a2a", dark: "#3a1a0c", accent: "#9aa8b8", glow: "#6ac8ff", line: OUTLINE },
  space: { space: ["#151a3a", "#03040a"], nebula: ["#5a3ad8", "#d83a8a"], light: "#e8ecff", hull: "#4a6ac8", dark: "#141c3a", accent: "#ffd25a", glow: "#bfd2ff", line: OUTLINE },
};

/** A color a kind gives its family's glow or accent: a weapon's energy, a resource's ore, a module's core. */
const KIND_COLORS: Readonly<Record<string, string>> = {
  beam: "#4ff0ff",
  ray: "#9af0ff",
  rail: "#b48aff",
  flak: "#ffb347",
  tractor: "#6ab8ff",
  reknite: "#ff4f5a",
  korrelite: "#4fa8ff",
  gel: "#7dff6a",
  axnit: "#b87bff",
  narcor: "#ffb02a",
  "red-narcor": "#ff2a3a",
  spice: "#ff8a2a",
  scrap: "#c0c8d0",
  cell: "#ffe84a",
  shield: "#5ad8ff",
  meltdown: "#ff3a2a",
  propagation: "#7aff5a",
};
const DISINTEGREX_GREEN = "#7aff5a";
const VAMP_RED = "#d0142a";
const AP_STEEL = "#3a3f48";
const BRASS = "#d4a24c";
const POD_COLORS: Readonly<Record<string, string>> = { poison: "#8aff3a", healing: "#ff6a9a", felinor: "#c08aff" };
const MODULE_COLORS: Readonly<Record<string, string>> = {
  magic: "#c86aff",
  frost: "#9ae8ff",
  overdrive: "#ff4a3a",
  capacitor: "#ffe84a",
  warpfield: "#8a6aff",
  nanobots: "#6aff9a",
  stimulus: "#ff9a3a",
};
const PLANET_COLORS: readonly (readonly [string, string])[] = [
  ["#7fd06a", "#1a3a12"],
  ["#d08a5a", "#3a1a0a"],
  ["#6aa8e8", "#0a1a3a"],
  ["#d86ab8", "#3a0a2a"],
  ["#e8d06a", "#3a300a"],
  ["#9a9ab0", "#1a1a24"],
];

/** Radiant faces mix every family tone this far toward the foil's gold. */
const GILD_MIX = 0.55;
const GOLD_DARK = "#4a2c04";
const GOLD_SPACE: readonly [string, string] = ["#3a2408", "#0c0602"];

function gild(paint: Paint): Paint {
  const toward = (from: string, to: string): string => mixHex(from, to, GILD_MIX);
  return {
    space: [toward(paint.space[0], GOLD_SPACE[0]), toward(paint.space[1], GOLD_SPACE[1])],
    nebula: [toward(paint.nebula[0], RADIANT_PALETTE.glow), toward(paint.nebula[1], RADIANT_PALETTE.ridges[0])],
    light: toward(paint.light, RADIANT_PALETTE.mote),
    hull: toward(paint.hull, RADIANT_PALETTE.ridges[0]),
    dark: toward(paint.dark, GOLD_DARK),
    accent: toward(paint.accent, RADIANT_PALETTE.emblem.fill),
    glow: toward(paint.glow, RADIANT_PALETTE.glow),
    line: paint.line === OUTLINE ? paint.line : toward(paint.line, RADIANT_PALETTE.mote),
  };
}

function kindPaint(base: Paint, subject: StinkySubject, lower: string): Paint {
  const kind = subject.kind;
  if (subject.family === "jlockeed") return base;
  const color = KIND_COLORS[kind];
  if (subject.family === "resource" && color !== undefined) return { ...base, accent: color };
  if (subject.family === "fleet" && kind === "pod") {
    const fluid = Object.entries(POD_COLORS).find(([word]) => lower.includes(word))?.[1];
    return fluid === undefined ? base : { ...base, glow: fluid };
  }
  if (subject.family === "fleet" && kind === "module") {
    const core = Object.entries(MODULE_COLORS).find(([word]) => lower.includes(word))?.[1];
    return core === undefined ? base : { ...base, glow: core };
  }
  if (subject.family === "weapon" && kind === "ray" && lower.includes("disintegrex")) return { ...base, glow: DISINTEGREX_GREEN };
  if (color !== undefined) return { ...base, glow: color };
  return base;
}

// Drawing

type Rng = () => number;

const BOX = 100;
const HALF = BOX / 2;
const DEGREES = Math.PI / 180;
const FULL_TURN = 360;
const STROKE = 0.7;
/** Where the subject sits: a little below the middle, as every window crops it (ART.md). */
const SUBJECT_X = 50;
const SUBJECT_Y = 52;
const SUBJECT_TILT_JITTER = 6;
const SUBJECT_SCALE_JITTER = 0.05;

const BACKDROP_SALT = 0x51a7c0de;
const SUBJECT_SALT = 0x2170a5e7;
const RAY_SALT = 0x6b43a9b5;
const STARS_MIN = 26;
const STARS_MAX = 44;
const TWINKLES_MIN = 2;
const TWINKLES_MAX = 4;
const NEBULAE_MIN = 2;
const NEBULAE_MAX = 3;
const PLANET_CHANCE = 0.65;
const RING_CHANCE = 0.4;
const HUE_DRIFT = 24;
const RAYS_MIN = 8;
const RAYS_MAX = 12;
const RAY_LENGTH = 150;
const RAY_OPACITY = 0.22;
const SPARKLES = 7;
const VIGNETTE_RADIUS = 74;
const VIGNETTE_START = 0.62;
const VIGNETTE_OPACITY = 0.55;
const GRID_STEP = 10;
const GRID_MAJOR = 5;

function between(rng: Rng, low: number, high: number): number {
  return low + (high - low) * rng();
}

function whole(rng: Rng, low: number, high: number): number {
  return low + Math.floor(rng() * (high - low + 1));
}

function attrs(values: Readonly<Record<string, string | number>>): string {
  return Object.entries(values)
    .map(([key, value]) => `${key}="${typeof value === "number" ? fmt(value) : value}"`)
    .join(" ");
}

function shape(tag: string, values: Readonly<Record<string, string | number>>): string {
  return `<${tag} ${attrs(values)}/>`;
}

function points(list: readonly (readonly [number, number])[]): string {
  return list.map(([x, y]) => `${fmt(x)},${fmt(y)}`).join(" ");
}

/** A filled outline in the set's ink. */
function poly(list: readonly (readonly [number, number])[], fill: string, line: string, extra: Record<string, string | number> = {}): string {
  return shape("polygon", { points: points(list), fill, stroke: line, "stroke-width": STROKE, "stroke-linejoin": "round", ...extra });
}

function mirror(list: readonly (readonly [number, number])[]): (readonly [number, number])[] {
  return [...list, ...[...list].reverse().map(([x, y]) => [x, -y] as const)];
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
  return { stroke: p.line, "stroke-width": STROKE };
}

function group(transform: string, body: string, extra: Record<string, string | number> = {}): string {
  const more = Object.keys(extra).length === 0 ? "" : ` ${attrs(extra)}`;
  return `<g transform="${transform}"${more}>${body}</g>`;
}

function at(x: number, y: number, rotate = 0, scale = 1): string {
  return `translate(${fmt(x)} ${fmt(y)}) rotate(${fmt(rotate)}) scale(${fmt(scale)})`;
}

/** A four-pointed glint. */
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

// The backdrop

type Defs = string[];

function backdrop(rng: Rng, p: Paint, family: StinkyFamily, planet: boolean, defs: Defs): string {
  const out: string[] = [];
  const drift = between(rng, -HUE_DRIFT, HUE_DRIFT);
  const top = turnHue(p.space[0], drift);
  defs.push(
    `<linearGradient id="sp" x1="0" y1="0" x2="0.3" y2="1"><stop offset="0" stop-color="${top}"/>` +
      `<stop offset="1" stop-color="${p.space[1]}"/></linearGradient>`,
  );
  out.push(rect(0, 0, BOX, BOX, "url(#sp)"));
  if (family === "jlockeed") {
    for (let i = 1; i < BOX / GRID_STEP; i += 1) {
      const major = i % GRID_MAJOR === 0;
      const extra = { opacity: major ? 0.45 : 0.2 };
      out.push(line(i * GRID_STEP, 0, i * GRID_STEP, BOX, p.light, major ? 0.5 : 0.25, extra));
      out.push(line(0, i * GRID_STEP, BOX, i * GRID_STEP, p.light, major ? 0.5 : 0.25, extra));
    }
    out.push(rect(66, 84, 28, 10, "none", { stroke: p.light, "stroke-width": 0.5, opacity: 0.7 }));
    out.push(line(66, 89, 94, 89, p.light, 0.3, { opacity: 0.7 }));
    out.push(line(8, 92, 40, 92, p.light, 0.4, { opacity: 0.7 }));
    out.push(line(8, 90, 8, 94, p.light, 0.4, { opacity: 0.7 }));
    out.push(line(40, 90, 40, 94, p.light, 0.4, { opacity: 0.7 }));
    return out.join("");
  }
  const nebulae = whole(rng, NEBULAE_MIN, NEBULAE_MAX);
  for (let i = 0; i < nebulae; i += 1) {
    const color = turnHue(i % 2 === 0 ? p.nebula[0] : p.nebula[1], drift);
    defs.push(
      `<radialGradient id="n${i}"><stop offset="0" stop-color="${color}" stop-opacity="0.75"/>` +
        `<stop offset="0.5" stop-color="${color}" stop-opacity="0.28"/>` +
        `<stop offset="1" stop-color="${color}" stop-opacity="0"/></radialGradient>`,
    );
    const cx = between(rng, 10, 90);
    const cy = between(rng, 10, 90);
    out.push(
      group(at(cx, cy, between(rng, 0, FULL_TURN)), ellipse(0, 0, between(rng, 26, 46), between(rng, 12, 24), `url(#n${i})`), {
        opacity: between(rng, 0.45, 0.8),
      }),
    );
  }
  const stars = whole(rng, STARS_MIN, STARS_MAX);
  for (let i = 0; i < stars; i += 1) {
    out.push(circle(between(rng, 1, 99), between(rng, 1, 99), between(rng, 0.15, 0.55), "#ffffff", { opacity: between(rng, 0.35, 1) }));
  }
  const twinkles = whole(rng, TWINKLES_MIN, TWINKLES_MAX);
  for (let i = 0; i < twinkles; i += 1) {
    out.push(twinkle(between(rng, 4, 96), between(rng, 4, 96), between(rng, 1.4, 2.6), "#ffffff", between(rng, 0.6, 0.95)));
  }
  const planetRoll = rng();
  const corner = whole(rng, 0, 3);
  const r = between(rng, 6, 13);
  const colors = PLANET_COLORS[whole(rng, 0, PLANET_COLORS.length - 1)] ?? PLANET_COLORS[0] ?? ["#888888", "#222222"];
  const ringed = rng() < RING_CHANCE;
  const ringTurn = between(rng, -30, 30);
  if (planet && planetRoll < PLANET_CHANCE) {
    const cx = corner % 2 === 0 ? between(rng, 10, 24) : between(rng, 76, 90);
    const cy = corner < 2 ? between(rng, 10, 24) : between(rng, 76, 90);
    defs.push(
      `<radialGradient id="pl" cx="0.35" cy="0.35" r="0.75"><stop offset="0" stop-color="${colors[0]}"/>` +
        `<stop offset="1" stop-color="${colors[1]}"/></radialGradient>`,
    );
    const ring = (half: "back" | "front"): string =>
      ringed
        ? group(
            at(cx, cy, ringTurn),
            path(
              half === "back"
                ? `M${fmt(-r * 1.7)} 0A${fmt(r * 1.7)} ${fmt(r * 0.45)} 0 0 1 ${fmt(r * 1.7)} 0`
                : `M${fmt(-r * 1.7)} 0A${fmt(r * 1.7)} ${fmt(r * 0.45)} 0 0 0 ${fmt(r * 1.7)} 0`,
              "none",
              { stroke: colors[0], "stroke-width": 1.1, opacity: 0.75 },
            ),
          )
        : "";
    out.push(ring("back"));
    out.push(circle(cx, cy, r, "url(#pl)"));
    out.push(ring("front"));
  }
  return out.join("");
}

// The subjects: each drawn about (0, 0), within about 40 of it, pointing right where it points.

/** The fleet's one hull: a warship seen from above, bow to the right. */
function hull(rng: Rng, p: Paint, cls: string): string {
  const out: string[] = [];
  const size = cls === "fighter" ? 0.72 : cls === "battleship" || cls === "dormant" ? 1.12 : 0.95;
  const L = 36 * size * between(rng, 0.92, 1.06);
  const W = 9.5 * size * between(rng, 0.88, 1.12);
  const dormant = cls === "dormant";
  const flame = dormant ? p.dark : p.glow;
  const engines = cls === "fighter" ? 1 : whole(rng, 2, 3);
  // Engines and their flames, behind the stern.
  for (let i = 0; i < engines; i += 1) {
    const y = engines === 1 ? 0 : -W * 0.55 + (W * 1.1 * i) / (engines - 1);
    if (!dormant) {
      out.push(poly([[-L - 1, y - W * 0.2], [-L - 1 - between(rng, 10, 16), y], [-L - 1, y + W * 0.2]], flame, "none", { opacity: 0.85 }));
      out.push(poly([[-L - 1, y - W * 0.1], [-L - 7, y], [-L - 1, y + W * 0.1]], "#ffffff", "none", { opacity: 0.8 }));
    }
    out.push(rect(-L - 3, y - W * 0.22, 5, W * 0.44, p.dark, inked(p)));
  }
  if (cls === "fighter") {
    const wing = mirror([[-L * 0.1, -W * 0.7], [-L * 0.55, -W * 2.7], [-L * 0.85, -W * 2.7], [-L * 0.7, -W * 0.8]]);
    out.push(poly(wing.slice(0, 4), "url(#hl)", p.line));
    out.push(poly(wing.slice(4), "url(#hl)", p.line));
    out.push(rect(-L * 0.75, -W * 2.75, L * 0.3, 1.4, p.accent));
    out.push(rect(-L * 0.75, W * 2.75 - 1.4, L * 0.3, 1.4, p.accent));
  }
  const body = mirror([[-L, -W * 0.55], [-L * 0.82, -W], [L * 0.3, -W], [L * 0.72, -W * 0.48], [L, 0]]).slice(0, -1);
  out.push(poly(body, "url(#hl)", p.line));
  // The lower flank in shade, the plating seams and the bow's stripe.
  out.push(poly([[-L, W * 0.1], [L * 0.95, W * 0.1], [L * 0.72, W * 0.48], [L * 0.3, W], [-L * 0.82, W], [-L, W * 0.55]], p.dark, "none", { opacity: 0.35 }));
  for (const seam of [-0.55, -0.15, 0.25]) out.push(line(L * seam, -W * 0.95, L * seam, W * 0.95, p.dark, 0.4, { opacity: 0.7 }));
  out.push(poly([[L * 0.42, -W * 0.82], [L * 0.52, -W * 0.68], [L * 0.52, W * 0.68], [L * 0.42, W * 0.82]], p.accent, "none"));
  if (cls === "battleship" || cls === "dormant") {
    out.push(poly([[-L * 0.6, -W], [-L * 0.45, -W * 1.45], [L * 0.05, -W * 1.45], [L * 0.2, -W]], "url(#hl)", p.line));
    out.push(poly([[-L * 0.6, W], [-L * 0.45, W * 1.45], [L * 0.05, W * 1.45], [L * 0.2, W]], p.hull, p.line));
  }
  // The bridge, its lit windows dark on a dormant ship.
  const bridgeX = cls === "fighter" ? L * 0.2 : -L * 0.42;
  out.push(rect(bridgeX, -W * 0.45, L * (cls === "fighter" ? 0.3 : 0.26), W * 0.9, p.light, { ...inked(p), rx: W * 0.2 }));
  const windows = cls === "fighter" ? 2 : 3;
  for (let i = 0; i < windows; i += 1) {
    out.push(rect(bridgeX + L * 0.04 + i * L * 0.07, -W * 0.18, L * 0.04, W * 0.36, dormant ? p.dark : p.glow));
  }
  // The turrets along the spine, barrels to the bow.
  const turrets = cls === "fighter" ? 0 : cls === "battleship" || cls === "dormant" ? whole(rng, 3, 4) : whole(rng, 2, 3);
  const slots = [L * 0.08, L * 0.32, -L * 0.62, -L * 0.1].slice(0, turrets);
  for (const x of slots) {
    const r = W * 0.36;
    out.push(rect(x, -r * 0.55, r * 2.6, r * 0.38, p.dark, inked(p)));
    out.push(rect(x, r * 0.17, r * 2.6, r * 0.38, p.dark, inked(p)));
    out.push(circle(x, 0, r, p.hull, inked(p)));
    out.push(circle(x - r * 0.25, -r * 0.25, r * 0.35, p.light, { opacity: 0.6 }));
  }
  if (cls === "fighter") out.push(ellipse(L * 0.62, 0, L * 0.12, W * 0.28, p.glow, inked(p)));
  if (dormant) {
    for (const x of [-0.75, -0.2, 0.35]) {
      out.push(line(L * x, -W * 1.9, L * x, W * 1.9, p.light, 0.5, { opacity: 0.45, "stroke-dasharray": "1.5 1.2" }));
    }
  }
  return out.join("");
}

/** A ring station: Megabase. */
function station(p: Paint): string {
  const out: string[] = [];
  out.push(ellipse(0, 0, 34, 14, "none", { stroke: p.dark, "stroke-width": 7 }));
  out.push(ellipse(0, 0, 34, 14, "none", { stroke: "url(#hl)", "stroke-width": 5.4 }));
  for (const turn of [0, 60, 120]) out.push(group(at(0, 0, turn), rect(-30, -1, 60, 2, p.hull, inked(p))));
  for (let i = 0; i < 12; i += 1) {
    const a = (i * FULL_TURN * DEGREES) / 12;
    out.push(rect(34 * Math.cos(a) - 0.8, 14 * Math.sin(a) - 0.8, 1.6, 1.6, p.glow));
  }
  out.push(circle(0, 0, 9, "url(#hl)", inked(p)));
  out.push(rect(-3, -22, 6, 16, p.hull, inked(p)));
  out.push(circle(0, -22, 2.4, p.glow));
  out.push(circle(0, 0, 3.6, p.glow, inked(p)));
  return out.join("");
}

/** A piece of a ship in front of the fleet's hull, faint behind it. */
function piece(rng: Rng, p: Paint, kind: string): string {
  const out: string[] = [group(at(0, 0, 0, 1.12), hull(rng, p, "cruiser"), { opacity: 0.22 })];
  switch (kind) {
    case "plate":
    case "shield": {
      const hex = (rx: number, ry: number): (readonly [number, number])[] => [
        [-rx, 0], [-rx * 0.5, -ry], [rx * 0.5, -ry], [rx, 0], [rx * 0.5, ry], [-rx * 0.5, ry],
      ];
      out.push(poly(hex(27, 19), "url(#hl)", p.line));
      out.push(poly(hex(20, 13), p.light, "none", { opacity: 0.25 }));
      out.push(poly([[-13, -19], [-4, -19], [-24, 6], [-27, 0]], "#ffffff", "none", { opacity: 0.22 }));
      for (const [x, y] of [[-20, -3], [-11, -14], [11, -14], [20, -3], [11, 14], [-11, 14]] as const) {
        out.push(circle(x, y, 1.3, p.dark, { stroke: p.light, "stroke-width": 0.3 }));
      }
      out.push(poly([[-6, 6], [0, 0], [6, 6], [6, 10], [0, 4], [-6, 10]], p.accent, p.line));
      if (kind === "shield") {
        out.push(path("M-36 14A36 36 0 0 1 36 14", p.glow, { opacity: 0.16 }));
        out.push(path("M-36 14A36 36 0 0 1 36 14", "none", { stroke: p.glow, "stroke-width": 1.6, opacity: 0.9 }));
        out.push(path("M-30 12A30 30 0 0 1 30 12", "none", { stroke: p.glow, "stroke-width": 0.6, opacity: 0.6 }));
      }
      break;
    }
    case "crystal": {
      out.push(poly([[-16, 22], [16, 22], [10, 12], [-10, 12]], "url(#hl)", p.line));
      const shards: readonly (readonly [number, number, number])[] = [[0, -6, 28], [-9, 0, 18], [9, 2, 16]];
      for (const [x, tilt, h] of shards) {
        out.push(group(at(x, 12, tilt * 0.8), poly([[-4, 0], [-4.5, -h * 0.7], [0, -h], [4.5, -h * 0.7], [4, 0]], p.glow, p.line, { opacity: 0.92 })));
        out.push(group(at(x, 12, tilt * 0.8), poly([[-1.5, -1], [-1.8, -h * 0.68], [0, -h * 0.92]], "#ffffff", "none", { opacity: 0.5 })));
      }
      break;
    }
    case "pod": {
      out.push(rect(-26, -12, 52, 24, "url(#hl)", { ...inked(p), rx: 12 }));
      out.push(rect(-15, -6, 30, 12, p.glow, { ...inked(p), rx: 6, opacity: 0.9 }));
      for (const [x, y, r] of [[-8, 1, 2], [-1, -2, 1.4], [6, 2, 1.8], [11, -2, 1]] as const) {
        out.push(circle(x, y, r, "#ffffff", { opacity: 0.55 }));
      }
      for (const x of [-20, 20]) out.push(rect(x - 1.2, -12, 2.4, 24, p.dark, { opacity: 0.6 }));
      out.push(rect(-12, -6, 24, 2.2, "#ffffff", { opacity: 0.35, rx: 1 }));
      out.push(rect(26, -4, 6, 8, p.dark, inked(p)));
      break;
    }
    case "engine": {
      out.push(poly([[-22, -9], [-40, -4], [-46, 0], [-40, 4], [-22, 9]], p.glow, "none", { opacity: 0.85 }));
      out.push(poly([[-22, -5], [-34, 0], [-22, 5]], "#ffffff", "none", { opacity: 0.85 }));
      out.push(poly([[-22, -16], [6, -9], [6, 9], [-22, 16]], "url(#hl)", p.line));
      for (const x of [-16, -9, -2]) out.push(line(x, -14 + (x + 22) * 0.25, x, 14 - (x + 22) * 0.25, p.dark, 0.9));
      out.push(rect(6, -12, 18, 24, p.hull, { ...inked(p), rx: 2 }));
      out.push(rect(10, -8, 10, 4, p.accent));
      out.push(rect(10, 4, 10, 4, p.dark));
      break;
    }
    case "station":
      return station(p);
    default: {
      // A module: a cased core with its cables.
      out.push(path("M-20 -6C-30 -6 -30 -18 -38 -18", "none", { stroke: p.dark, "stroke-width": 2.2 }));
      out.push(path("M20 6C30 6 30 18 38 18", "none", { stroke: p.dark, "stroke-width": 2.2 }));
      out.push(rect(-20, -15, 40, 30, "url(#hl)", { ...inked(p), rx: 3 }));
      out.push(rect(-17, -12, 34, 24, p.dark, { opacity: 0.35, rx: 2 }));
      out.push(circle(0, 0, 13, p.glow, { opacity: 0.25 }));
      out.push(circle(0, 0, 8, p.glow, inked(p)));
      out.push(circle(0, 0, 4, "#ffffff", { opacity: 0.85 }));
      for (const x of [-15, -11, -7]) out.push(circle(x, 11, 1, p.accent));
      for (const y of [-10, 10]) out.push(rect(-20, y - 0.4, 40, 0.8, p.dark, { opacity: 0.5 }));
    }
  }
  return out.join("");
}

// Weapons share one gunmetal turret mount, its tier in chevrons, and draw the barrels their names say.

function mount(p: Paint, tier: number): string {
  const out: string[] = [];
  out.push(poly([[-26, 24], [6, 24], [0, 13], [-20, 13]], p.dark, p.line));
  out.push(rect(-24, 22, 28, 2, p.hull, { opacity: 0.6 }));
  out.push(circle(-10, 7, 10, "url(#hl)", inked(p)));
  const chevrons = Math.min(tier, 5);
  for (let i = 0; i < chevrons; i += 1) {
    const x = -18 + i * 4.4;
    out.push(path(`M${fmt(x)} 16L${fmt(x + 1.8)} 19L${fmt(x)} 22`, "none", { stroke: p.accent, "stroke-width": 1.1, "stroke-linejoin": "round" }));
  }
  return out.join("");
}

function barrel(p: Paint, x: number, y: number, length: number, height: number): string {
  return (
    rect(x, y - height / 2, length, height, "url(#hl)", inked(p)) +
    rect(x, y - height / 2 + height * 0.15, length, height * 0.2, "#ffffff", { opacity: 0.25 })
  );
}

function flash(p: Paint, x: number, y: number, r: number): string {
  return (
    circle(x, y, r * 1.6, p.glow, { opacity: 0.25 }) +
    twinkle(x, y, r * 1.3, p.glow, 0.95) +
    circle(x, y, r * 0.45, "#ffffff", { opacity: 0.95 })
  );
}

function weapon(rng: Rng, p: Paint, kind: string, tier: number, lower: string): string {
  const out: string[] = [];
  switch (kind) {
    case "cannon": {
      const yamato = lower.includes("yamato");
      const h = yamato ? 11 : 7 + tier * 0.4;
      const length = yamato ? 42 : 30 + tier * 2;
      out.push(mount(p, tier));
      out.push(barrel(p, -8, 4, length, h));
      for (const x of [4, 12]) out.push(rect(x, 4 - h / 2 - 0.8, 2, h + 1.6, p.dark, inked(p)));
      out.push(rect(-8 + length, 4 - h / 2 - 2, 6, h + 4, p.hull, inked(p)));
      out.push(rect(-20, -3, 14, 14, p.hull, { ...inked(p), rx: 2 }));
      if (yamato) {
        out.push(circle(-2 + length + 6, 4, 9, p.glow, { opacity: 0.3 }));
        out.push(circle(-2 + length + 6, 4, 5, p.glow, { opacity: 0.85 }));
        out.push(circle(-2 + length + 6, 4, 2.2, "#ffffff"));
      } else {
        out.push(flash(p, -8 + length + 10, 4, 5));
      }
      break;
    }
    case "repeater": {
      const count = Math.min(6, tier + 1);
      out.push(mount(p, tier));
      for (let i = 0; i < count; i += 1) out.push(barrel(p, -6, 4 - (count - 1) * 1.1 + i * 2.2, 34, 1.8));
      for (const x of [6, 22]) out.push(rect(x, 4 - count * 1.3, 3, count * 2.6, p.dark, inked(p)));
      out.push(rect(-18, -4, 14, 16, p.hull, { ...inked(p), rx: 2 }));
      for (let i = 0; i < 3; i += 1) out.push(flash(p, 32 + i * 5, 4 - 4 + i * 4, 2.6));
      break;
    }
    case "flak": {
      out.push(mount(p, tier));
      out.push(barrel(p, -6, 0, 28, 4.2));
      out.push(barrel(p, -6, 8, 28, 4.2));
      out.push(rect(-18, -6, 16, 18, p.hull, { ...inked(p), rx: 2 }));
      const bursts: readonly (readonly [number, number, number])[] = [[30, -20, 7], [38, -6, 5.5], [24, -32, 5]];
      for (const [x, y, r] of bursts) {
        out.push(circle(x, y, r, "#8a8f98", { opacity: 0.55 }));
        out.push(twinkle(x, y, r * 1.25, p.glow, 0.9));
        out.push(circle(x, y, r * 0.35, "#ffffff", { opacity: 0.9 }));
      }
      break;
    }
    case "beam": {
      out.push(mount(p, tier));
      out.push(rect(-18, -5, 30, 18, "url(#hl)", { ...inked(p), rx: 3 }));
      for (const x of [-12, -6, 0]) out.push(rect(x, -5, 2, 18, p.dark, { opacity: 0.5 }));
      out.push(poly([[12, -2], [18, -4], [18, 12], [12, 10]], p.hull, p.line));
      out.push(rect(18, 4 - 1.4 - tier * 0.3, 32, 2.8 + tier * 0.6, p.glow, { opacity: 0.85 }));
      out.push(rect(18, 4 - 0.5, 32, 1, "#ffffff"));
      out.push(circle(19, 4, 4.5, p.glow, { opacity: 0.6 }));
      out.push(circle(19, 4, 2.2, "#ffffff"));
      break;
    }
    case "ray": {
      // A ray gun: grip, finned body, dish and its beam.
      out.push(poly([[-14, 2], [-8, 2], [-12, 18], [-19, 18]], p.dark, p.line));
      out.push(ellipse(-4, -2, 16, 7, "url(#hl)", inked(p)));
      for (const x of [-12, -6, 0]) out.push(ellipse(x, -2, 1.6, 9.5, p.accent, inked(p)));
      out.push(rect(10, -4, 8, 4, p.hull, inked(p)));
      out.push(path("M18 -10Q24 -2 18 6Z", p.light, inked(p)));
      let d = "M22 -2";
      for (let i = 1; i <= 8; i += 1) d += `L${fmt(22 + i * 4.5)} ${fmt(-2 + (i % 2 === 0 ? -1 : 1) * between(rng, 2, 4))}`;
      out.push(path(d, "none", { stroke: p.glow, "stroke-width": 3, "stroke-linejoin": "round", opacity: 0.55 }));
      out.push(path(d, "none", { stroke: "#ffffff", "stroke-width": 0.9, "stroke-linejoin": "round" }));
      out.push(circle(20, -2, 4, p.glow, { opacity: 0.7 }));
      break;
    }
    case "rail": {
      out.push(mount(p, tier));
      out.push(rect(-18, -2, 14, 14, p.hull, { ...inked(p), rx: 2 }));
      out.push(barrel(p, -6, -1, 46, 3));
      out.push(barrel(p, -6, 9, 46, 3));
      for (const x of [2, 12, 22, 32]) out.push(rect(x, -2, 2.4, 12, p.dark, inked(p)));
      let d = "M-2 4";
      for (let i = 1; i <= 10; i += 1) d += `L${fmt(-2 + i * 4)} ${fmt(4 + (i % 2 === 0 ? -2.6 : 2.6))}`;
      out.push(path(d, "none", { stroke: p.glow, "stroke-width": 1, opacity: 0.95 }));
      out.push(rect(44, 2.5, 8, 3, p.light, inked(p)));
      out.push(rect(52, 3.2, 10, 1.6, p.glow, { opacity: 0.7 }));
      break;
    }
    case "triple": {
      out.push(mount(p, tier));
      out.push(rect(-20, -8, 18, 24, p.hull, { ...inked(p), rx: 2 }));
      for (const y of [-4, 4, 12]) {
        out.push(barrel(p, -4, y, 30, 3.6));
        out.push(flash(p, 32, y, 2.6));
      }
      break;
    }
    case "missiles": {
      out.push(mount(p, tier));
      out.push(rect(-24, -14, 34, 26, "url(#hl)", { ...inked(p), rx: 2 }));
      for (let row = 0; row < 2; row += 1) {
        for (let col = 0; col < 3; col += 1) {
          const x = -17 + col * 10;
          const y = -7 + row * 11;
          out.push(circle(x, y, 4, p.dark, inked(p)));
          if (!(row === 0 && col === 2)) out.push(circle(x, y, 2.4, p.accent));
        }
      }
      // The one in flight, its trail behind it.
      out.push(group(at(26, -20, -32), [
        path("M-26 0Q-14 -2 -6 0Q-14 2 -26 0Z", "#c9ced6", { opacity: 0.5 }),
        poly([[-6, -1.6], [-9, -4], [-4, -1.6]], p.dark, p.line),
        poly([[-6, 1.6], [-9, 4], [-4, 1.6]], p.dark, p.line),
        rect(-6, -1.8, 14, 3.6, p.light, inked(p)),
        poly([[8, -1.8], [13, 0], [8, 1.8]], p.accent, p.line),
        circle(-7, 0, 1.8, p.glow),
      ].join("")));
      break;
    }
    case "launcher": {
      out.push(mount(p, tier));
      out.push(rect(-20, -5, 46, 13, "url(#hl)", { ...inked(p), rx: 3 }));
      for (const x of [-12, 4, 18]) out.push(rect(x, -6, 2.6, 15, p.dark, inked(p)));
      out.push(rect(26, -7, 4, 17, p.hull, inked(p)));
      for (const y of [-3, 1.5, 6]) out.push(line(31, y, 37, y, p.light, 0.5, { opacity: 0.6 }));
      out.push(payload(p, lower, 44, 1));
      break;
    }
    case "rounds": {
      const vamp = lower.includes("vamp");
      const tip = vamp ? VAMP_RED : AP_STEEL;
      for (const [x, y, tilt] of [[-14, 4, -8], [0, 0, 0], [14, 4, 8]] as const) {
        out.push(group(at(x, y, tilt), [
          rect(-4.5, -4, 9, 24, BRASS, { ...inked(p), rx: 1 }),
          rect(-5.2, 18, 10.4, 3, mixHex(BRASS, "#000000", 0.3), inked(p)),
          rect(-3, -3, 1.6, 20, "#ffffff", { opacity: 0.3 }),
          path("M-4.5 -4L-4.5 -9Q-4 -19 0 -22Q4 -19 4.5 -9L4.5 -4Z", tip, inked(p)),
          vamp ? "" : rect(-4.5, -6.5, 9, 2, "#111111"),
        ].join("")));
      }
      if (vamp) {
        out.push(path("M18 -18Q22 -11 18 -8Q14 -11 18 -18Z", VAMP_RED, inked(p)));
        out.push(path("M-20 -14Q-17 -9 -20 -7Q-23 -9 -20 -14Z", VAMP_RED, inked(p)));
      } else {
        out.push(path("M18 -16L24 -22M20 -12L28 -14", "none", { stroke: p.glow, "stroke-width": 1.2, "stroke-linecap": "round" }));
      }
      break;
    }
    case "mine": {
      for (let i = 0; i < 8; i += 1) {
        out.push(group(at(0, 0, i * 45), poly([[14, -3], [23, 0], [14, 3]], p.hull, p.line)));
        out.push(group(at(0, 0, i * 45), circle(23, 0, 1.6, p.accent)));
      }
      out.push(circle(0, 0, 16, "url(#hl)", inked(p)));
      out.push(ellipse(0, 0, 16, 4, "none", { stroke: p.dark, "stroke-width": 1.4 }));
      out.push(circle(0, 0, 6, VAMP_RED, { opacity: 0.35 }));
      out.push(circle(0, 0, 3.4, "#ff3a2a", inked(p)));
      out.push(circle(-5, -7, 3, "#ffffff", { opacity: 0.25 }));
      break;
    }
    case "tank": {
      out.push(rect(-30, 8, 52, 12, p.dark, { ...inked(p), rx: 6 }));
      for (let i = 0; i < 6; i += 1) out.push(circle(-24 + i * 8, 14, 3.4, p.hull, inked(p)));
      out.push(poly([[-28, 8], [-22, -2], [16, -2], [24, 8]], "url(#hl)", p.line));
      out.push(barrel(p, 0, -8, 40, 4));
      out.push(rect(38, -10.5, 5, 5, p.hull, inked(p)));
      out.push(path("M-14 -2Q-14 -16 0 -16Q12 -16 12 -2Z", p.hull, inked(p)));
      out.push(rect(-8, -12, 8, 3, p.glow));
      out.push(flash(p, 48, -8, 4));
      break;
    }
    default: {
      // tractor: a dish on the mount and the blue cone that drags an asteroid in.
      out.push(poly([[6, 0], [50, -18], [50, 26], [6, 8]], p.glow, "none", { opacity: 0.22 }));
      for (const x of [18, 28, 38]) {
        const spread = (x - 6) * 0.5;
        out.push(ellipse(x, 4, 2, spread * 0.9, "none", { stroke: p.glow, "stroke-width": 0.8, opacity: 0.7 }));
      }
      out.push(group(at(40, 4, 20), poly([[-6, -4], [-2, -7], [5, -5], [7, 1], [3, 6], [-4, 5], [-7, 1]], "#8a7a68", p.line)));
      out.push(mount(p, tier));
      out.push(path("M-2 -12Q10 4 -2 20", p.light, inked(p)));
      out.push(line(-2, 4, 8, 4, p.dark, 1.4));
      out.push(circle(8, 4, 2.4, p.glow));
    }
  }
  return out.join("");
}

/** What a launcher fires, by the name's first word. */
function payload(p: Paint, lower: string, x: number, y: number): string {
  const out: string[] = [line(x - 14, y - 2, x - 6, y - 1, p.light, 0.6, { opacity: 0.5 }), line(x - 14, y + 3, x - 6, y + 2, p.light, 0.6, { opacity: 0.5 })];
  if (lower.includes("fruit")) {
    out.push(circle(x, y, 6, "#e2513f", inked(p)));
    out.push(path(`M${fmt(x)} ${fmt(y - 6)}Q${fmt(x + 5)} ${fmt(y - 11)} ${fmt(x + 7)} ${fmt(y - 7)}Q${fmt(x + 3)} ${fmt(y - 5)} ${fmt(x)} ${fmt(y - 6)}Z`, "#6ad04a", inked(p)));
    out.push(circle(x - 2, y - 2, 1.6, "#ffffff", { opacity: 0.5 }));
  } else if (lower.includes("jade")) {
    out.push(circle(x, y, 6.5, "#2fbf7a", inked(p)));
    out.push(circle(x, y, 3.2, "#9affc8", { opacity: 0.8 }));
    out.push(circle(x - 2.2, y - 2.2, 1.4, "#ffffff", { opacity: 0.7 }));
  } else if (lower.includes("book")) {
    out.push(group(at(x, y, -14), [rect(-6, -8, 12, 16, "#8a3a2a", { ...inked(p), rx: 1 }), rect(-6, -8, 2.4, 16, "#5a2010"), rect(-2, -4, 6, 1.2, "#f2d48a"), rect(5, -7, 1.4, 14, "#f3e3bf")].join("")));
  } else {
    out.push(circle(x, y, 6.5, p.accent, inked(p)));
    out.push(circle(x, y, 4.6, "none", { stroke: p.dark, "stroke-width": 0.6 }));
    out.push(twinkle(x, y, 3.4, p.dark, 0.9));
  }
  return out.join("");
}

// Drones: one sphere with a glowing eye, from the bare core to the battleship.

function droneBody(p: Paint, r: number, fins: number): string {
  const out: string[] = [];
  if (fins === 2) {
    out.push(poly([[-r * 0.2, -r * 0.6], [-r * 2.1, -r * 1.9], [-r * 1.7, -r * 0.5], [-r * 0.4, r * 0.1]], "url(#hl)", p.line));
    out.push(poly([[r * 0.2, -r * 0.6], [r * 2.1, -r * 1.9], [r * 1.7, -r * 0.5], [r * 0.4, r * 0.1]], "url(#hl)", p.line));
  } else if (fins === 4) {
    for (const turn of [45, 135, 225, 315]) {
      out.push(group(at(0, 0, turn), poly([[r * 0.6, -r * 0.3], [r * 2, -r * 0.5], [r * 2.2, 0], [r * 2, r * 0.5], [r * 0.6, r * 0.3]], "url(#hl)", p.line)));
      out.push(group(at(0, 0, turn), circle(r * 2.05, 0, r * 0.14, p.glow)));
    }
  }
  out.push(circle(0, 0, r, "url(#dr)", inked(p)));
  out.push(path(`M${fmt(-r)} 0A${fmt(r)} ${fmt(r * 0.3)} 0 0 0 ${fmt(r)} 0`, "none", { stroke: p.dark, "stroke-width": r * 0.08 }));
  out.push(circle(0, r * 0.05, r * 0.42, p.dark, inked(p)));
  out.push(circle(0, r * 0.05, r * 0.32, p.glow));
  out.push(circle(0, r * 0.05, r * 0.14, p.dark));
  out.push(circle(-r * 0.1, -r * 0.06, r * 0.06, "#ffffff"));
  out.push(circle(-r * 0.4, -r * 0.45, r * 0.18, "#ffffff", { opacity: 0.35 }));
  return out.join("");
}

function drone(p: Paint, kind: string, tier: number): string {
  const out: string[] = [];
  switch (kind) {
    case "core": {
      const ring = (half: "back" | "front"): string =>
        group(at(0, 0, -14), path(half === "back" ? "M-30 0A30 9 0 0 1 30 0" : "M-30 0A30 9 0 0 0 30 0", "none", { stroke: p.accent, "stroke-width": 1.6, opacity: 0.9 }));
      out.push(ring("back"));
      out.push(circle(0, 0, 24, p.glow, { opacity: 0.12 }));
      out.push(droneBody(p, 16, 0));
      out.push(ring("front"));
      for (let i = 0; i < tier; i += 1) {
        const a = (-90 + (i * FULL_TURN) / tier) * DEGREES;
        out.push(group(at(0, 0, -14), circle(30 * Math.cos(a), 9 * Math.sin(a), 1.8, p.glow, inked(p))));
      }
      break;
    }
    case "interceptor":
      out.push(droneBody(p, 11, 2));
      out.push(rect(-4, 9, 2, 8, p.dark, inked(p)), rect(2, 9, 2, 8, p.dark, inked(p)));
      break;
    case "corvette":
      out.push(droneBody(p, 13, 4));
      out.push(rect(-20, -3, 8, 6, p.hull, inked(p)), rect(12, -3, 8, 6, p.hull, inked(p)));
      break;
    case "hive":
      out.push(group(at(-25, -17, -10, 0.5), droneBody(p, 11, 2)));
      out.push(group(at(25, -15, 10, 0.5), droneBody(p, 11, 2)));
      out.push(droneBody(p, 14, 4));
      break;
    default: {
      // The battleships: a drone's eye in a wide hull, each layer its own color.
      if (kind === "shield") out.push(path("M-40 6A40 30 0 0 1 40 6Z", p.glow, { opacity: 0.18, stroke: p.glow, "stroke-width": 1 }));
      out.push(ellipse(0, 6, 40, 14, "url(#hl)", inked(p)));
      out.push(ellipse(0, 3, 30, 9, p.light, { opacity: 0.25 }));
      for (let i = 0; i < 9; i += 1) out.push(rect(-32 + i * 8, 11, 2.4, 1.6, p.glow));
      for (const x of [-26, 26]) out.push(rect(x - 3, -2, 6, 6, p.dark, inked(p)));
      out.push(droneBody(p, 12, 0));
      if (kind === "propagation") {
        for (const [x, y, r] of [[-30, -10, 2.2], [-20, -20, 1.6], [24, -18, 2], [34, -6, 1.4], [8, -26, 1.8]] as const) {
          out.push(circle(x, y, r, p.glow, { opacity: 0.85 }));
        }
      }
      if (kind === "meltdown") {
        out.push(circle(0, 6, 30, p.glow, { opacity: 0.18 }));
        out.push(path("M-30 4L-22 8L-26 12M18 2L24 7L30 4M-6 14L0 18L6 15", "none", { stroke: p.glow, "stroke-width": 1.2, "stroke-linejoin": "round" }));
      }
    }
  }
  return out.join("");
}

// The other families

function asteroid(rng: Rng, p: Paint, r: number): string {
  const corners = whole(rng, 9, 12);
  const list: (readonly [number, number])[] = [];
  for (let i = 0; i < corners; i += 1) {
    const a = (i * FULL_TURN * DEGREES) / corners;
    const reach = r * between(rng, 0.78, 1.08);
    list.push([reach * Math.cos(a), reach * Math.sin(a)]);
  }
  const out: string[] = [poly(list, "url(#rk)", p.line)];
  const craters = whole(rng, 3, 5);
  for (let i = 0; i < craters; i += 1) {
    const a = between(rng, 0, FULL_TURN) * DEGREES;
    const reach = between(rng, 0, r * 0.55);
    const size = between(rng, r * 0.1, r * 0.22);
    out.push(ellipse(reach * Math.cos(a), reach * Math.sin(a), size, size * 0.8, p.dark, { opacity: 0.55 }));
    out.push(ellipse(reach * Math.cos(a) - size * 0.2, reach * Math.sin(a) - size * 0.2, size * 0.6, size * 0.45, p.light, { opacity: 0.18 }));
  }
  return out.join("");
}

const ASTEROID_SIZES: Readonly<Record<string, number>> = { tiny: 9, medium: 15, large: 21, huge: 28 };

function resource(rng: Rng, p: Paint, kind: string, lower: string): string {
  const out: string[] = [];
  switch (kind) {
    case "asteroid": {
      const size = Object.entries(ASTEROID_SIZES).find(([word]) => lower.includes(word))?.[1] ?? ASTEROID_SIZES.medium ?? 15;
      out.push(asteroid(rng, p, size));
      break;
    }
    case "field":
      for (const [x, y, r] of [[-18, -12, 12], [16, 4, 15], [-12, 18, 8], [26, -20, 6], [-30, 6, 5]] as const) {
        out.push(group(at(x, y), asteroid(rng, p, r)));
      }
      break;
    case "tower":
      out.push(group(at(0, 10), asteroid(rng, p, 22)));
      out.push(rect(-4, -24, 8, 20, p.light, inked(p)));
      out.push(rect(-7, -28, 14, 5, p.hull, inked(p)));
      out.push(line(0, -28, 0, -36, p.light, 0.8));
      out.push(circle(0, -36, 1.4, p.accent));
      out.push(rect(-2, -20, 4, 3, p.accent, { opacity: 0.6 }));
      break;
    case "comet":
      out.push(poly([[0, -8], [-46, -26], [-48, -18], [-40, -12], [0, 8]], p.glow, "none", { opacity: 0.35 }));
      out.push(poly([[0, -4], [-38, -20], [0, 4]], "#ffffff", "none", { opacity: 0.35 }));
      out.push(circle(0, 0, 14, p.glow, { opacity: 0.35 }));
      out.push(asteroid(rng, p, 11));
      break;
    case "gel":
      out.push(path("M-20 14Q-28 -6 -12 -16Q0 -26 14 -16Q28 -6 20 14Q0 22 -20 14Z", p.accent, { ...inked(p), opacity: 0.88 }));
      for (const [x, y, r] of [[-8, -6, 4], [6, 2, 3], [10, -10, 2], [-4, 8, 2.4]] as const) {
        out.push(circle(x, y, r, "#ffffff", { opacity: 0.4 }));
      }
      break;
    case "spice":
      out.push(ellipse(0, 18, 30, 6, p.dark, { opacity: 0.6 }));
      out.push(path("M-26 18Q-18 0 -6 -6Q0 -10 6 -6Q18 0 26 18Z", p.accent, inked(p)));
      out.push(path("M-26 18Q-18 0 -6 -6Q-2 -8 0 -8Q-10 4 -12 18Z", mixHex(p.accent, "#000000", 0.3), { opacity: 0.6 }));
      out.push(path("M-14 6Q-6 -4 2 -5", "none", { stroke: "#ffffff", "stroke-width": 1.2, opacity: 0.4 }));
      for (let i = 0; i < 18; i += 1) {
        const a = between(rng, -150, -30) * DEGREES;
        const reach = between(rng, 14, 32);
        out.push(circle(reach * Math.cos(a), 4 + reach * Math.sin(a), between(rng, 0.5, 1.4), p.accent, { opacity: between(rng, 0.5, 1) }));
      }
      break;
    case "scrap":
      out.push(group(at(-8, 2, -20), poly([[-14, -6], [10, -10], [14, 2], [-4, 8], [-16, 4]], "url(#rk)", p.line)));
      out.push(group(at(12, -6, 30), poly([[-10, -4], [10, -6], [6, 6], [-8, 4]], p.accent, p.line)));
      out.push(group(at(4, 14, 10), rect(-10, -2, 20, 4, p.hull, inked(p))));
      out.push(circle(-14, -12, 3, p.dark, inked(p)), circle(-14, -12, 1.2, p.accent));
      break;
    case "cell":
      out.push(rect(-12, -22, 24, 44, "url(#hl)", { ...inked(p), rx: 4 }));
      out.push(rect(-5, -27, 10, 5, p.dark, inked(p)));
      out.push(rect(-8, -14, 16, 28, p.accent, { opacity: 0.85, rx: 2 }));
      out.push(path("M2 -10L-4 1L1 1L-2 10L5 -2L0 -2Z", "#ffffff", { opacity: 0.9 }));
      break;
    default: {
      // An ore: crystals out of a rock.
      out.push(poly([[-26, 18], [-20, 6], [-6, 2], [12, 4], [24, 10], [28, 20], [-4, 24]], "url(#rk)", p.line));
      const shards: readonly (readonly [number, number, number, number])[] = [[-10, -20, 30, 6], [2, 0, 38, 7], [14, 18, 26, 5.5], [-18, -40, 18, 4.5]];
      for (const [x, tilt, h, w] of shards) {
        out.push(group(at(x, 10, tilt), poly([[-w, 0], [-w, -h * 0.75], [0, -h], [w, -h * 0.75], [w, 0]], p.accent, p.line)));
        out.push(group(at(x, 10, tilt), poly([[-w * 0.6, -1], [-w * 0.6, -h * 0.72], [0, -h * 0.92]], "#ffffff", "none", { opacity: 0.4 })));
      }
    }
  }
  return out.join("");
}

function miner(rng: Rng, p: Paint, kind: string, tier: number): string {
  const out: string[] = [];
  if (kind === "ship") out.push(group(at(-8, 0), hull(rng, p, "cruiser")));
  const x = kind === "ship" ? 30 : 4;
  const scale = kind === "ship" ? 0.7 : 1;
  const bits: string[] = [];
  bits.push(rect(-30, -10, 22, 20, "url(#hl)", { ...inked(p), rx: 2 }));
  for (let i = 0; i < 4; i += 1) bits.push(poly([[-30 + i * 6, -10], [-26 + i * 6, -10], [-30 + i * 6, 10], [-34 + i * 6, 10]], p.accent, "none", { opacity: 0.85 }));
  bits.push(rect(-8, -6, 8, 12, p.dark, inked(p)));
  const cone = 22 + tier * 2;
  bits.push(poly([[0, -9], [cone, 0], [0, 9]], p.light, p.line));
  for (let i = 1; i < 4; i += 1) bits.push(line((cone * i) / 4, -9 + (9 * i) / 4, (cone * (i + 0.6)) / 4, 9 - (9 * (i + 0.6)) / 4, p.dark, 0.9));
  bits.push(circle(cone + 4, 0, 3, p.glow, { opacity: 0.6 }));
  for (const [dx, dy] of [[cone + 8, -6], [cone + 10, 4], [cone + 4, 9]] as const) bits.push(circle(dx, dy, 1.4, "#c08a5a"));
  out.push(group(at(x, kind === "ship" ? 8 : 0, 0, scale), bits.join("")));
  return out.join("");
}

function anomaly(rng: Rng, p: Paint, kind: string): string {
  const out: string[] = [];
  if (kind === "wormhole") {
    for (let i = 0; i < 7; i += 1) {
      const t = i / 6;
      out.push(ellipse(t * 6, -t * 4, 36 - t * 30, 22 - t * 18, "none", { stroke: i % 2 === 0 ? p.glow : p.accent, "stroke-width": 1.6 - t, opacity: 0.9 - t * 0.4 }));
    }
    out.push(circle(6, -4, 4, "#ffffff", { opacity: 0.9 }));
    return out.join("");
  }
  out.push(circle(0, 0, 26, p.glow, { opacity: 0.15 }));
  const arms = whole(rng, 4, 5);
  const offset = between(rng, 0, FULL_TURN);
  for (let arm = 0; arm < arms; arm += 1) {
    let d = "";
    for (let step = 0; step <= 16; step += 1) {
      const a = (offset + (arm * FULL_TURN) / arms + step * 22) * DEGREES;
      const r = 3 + step * 2.2;
      d += `${step === 0 ? "M" : "L"}${fmt(r * Math.cos(a))} ${fmt(r * Math.sin(a) * 0.8)}`;
    }
    out.push(path(d, "none", { stroke: arm % 2 === 0 ? p.glow : p.accent, "stroke-width": 2.4, "stroke-linecap": "round", "stroke-linejoin": "round", opacity: 0.75 }));
  }
  out.push(circle(0, 0, 6, "#ffffff", { opacity: 0.9 }));
  out.push(circle(0, 0, 10, p.glow, { opacity: 0.4 }));
  if (kind === "probe") {
    out.push(group(at(18, 22, -20), [
      line(-6, 0, -16, -10, p.light, 0.8),
      rect(-14, -4, 10, 8, p.accent, { ...inked(p), opacity: 0.85 }),
      rect(4, -4, 10, 8, p.accent, { ...inked(p), opacity: 0.85 }),
      circle(0, 0, 5, "url(#hl)", inked(p)),
      path("M3 -8Q10 -4 3 0", p.light, inked(p)),
      circle(0, 0, 1.6, p.glow),
    ].join("")));
  }
  return out.join("");
}

function doctrine(p: Paint, kind: string): string {
  const out: string[] = [];
  out.push(path("M-24 -26L24 -26L24 4Q24 22 0 32Q-24 22 -24 4Z", "url(#hl)", { stroke: p.accent, "stroke-width": 2.2, "stroke-linejoin": "round" }));
  out.push(path("M-19 -21L19 -21L19 3Q19 18 0 26Q-19 18 -19 3Z", "none", { stroke: p.dark, "stroke-width": 0.8, opacity: 0.7 }));
  for (const y of [-18, -13]) out.push(path(`M-10 ${y}L0 ${y + 4}L10 ${y}`, "none", { stroke: p.accent, "stroke-width": 1.8, "stroke-linejoin": "round" }));
  const ink = { stroke: p.line, "stroke-width": STROKE };
  switch (kind) {
    case "land":
      out.push(poly([[-14, 14], [-4, -4], [2, 6], [7, 0], [15, 14]], p.light, p.line));
      out.push(rect(-12, 14, 24, 5, p.dark, { ...ink, rx: 2.5 }));
      break;
    case "sea":
      for (const y of [0, 6, 12]) out.push(path(`M-14 ${y}Q-10 ${y - 4} -6 ${y}T2 ${y}T10 ${y}T18 ${y}`, "none", { stroke: p.light, "stroke-width": 2 }));
      break;
    case "air":
      out.push(poly([[-2, 2], [-18, -4], [-14, 2], [-18, 8], [-2, 8]], p.light, p.line));
      out.push(poly([[2, 2], [18, -4], [14, 2], [18, 8], [2, 8]], p.light, p.line));
      out.push(circle(0, 5, 3.4, p.accent, ink));
      break;
    case "space":
      out.push(ellipse(0, 6, 15, 5, "none", { stroke: p.light, "stroke-width": 1.2 }));
      out.push(twinkle(0, 5, 10, p.accent, 1));
      out.push(circle(0, 5, 2.4, "#ffffff"));
      break;
    default:
      out.push(path("M3 -6L-8 8L-1 8L-4 20L9 4L2 4L5 -6Z", p.accent, ink));
  }
  return out.join("");
}

function terminus(rng: Rng, p: Paint): string {
  const tilt = between(rng, -16, 16);
  const disk = (half: "back" | "front"): string =>
    group(at(0, 0, tilt), path(half === "back" ? "M-40 0A40 11 0 0 1 40 0L26 0A26 6 0 0 0 -26 0Z" : "M-40 0A40 11 0 0 0 40 0L26 0A26 6 0 0 1 -26 0Z", "url(#ad)", { opacity: 0.95 }));
  return [
    circle(0, 0, 30, p.hull, { opacity: 0.12 }),
    disk("back"),
    circle(0, 0, 15, p.glow, { opacity: 0.5 }),
    path("M-16 0A16 16 0 0 1 16 0", "none", { stroke: p.light, "stroke-width": 1.4, opacity: 0.9 }),
    circle(0, 0, 13, "#000000"),
    circle(0, 0, 13.4, "none", { stroke: p.accent, "stroke-width": 0.8 }),
    disk("front"),
  ].join("");
}

function insect(p: Paint, kind: string): string {
  const hornet = kind === "hornet";
  const length = hornet ? 1.15 : 1;
  const out: string[] = [];
  out.push(ellipse(-2, -12, 14, 6, p.light, { opacity: 0.35, stroke: p.light, "stroke-width": 0.5, transform: "rotate(-20)" }));
  out.push(ellipse(6, -12, 12, 5, p.light, { opacity: 0.35, stroke: p.light, "stroke-width": 0.5, transform: "rotate(-40)" }));
  for (const x of [-2, 4, 10]) out.push(path(`M${x} 4L${x - 4} 12L${x - 6} 16`, "none", { stroke: p.dark, "stroke-width": 1 }));
  out.push(ellipse(-16 * length, 2, 14 * length, hornet ? 8 : 6.5, p.hull, inked(p)));
  const bands = hornet ? 4 : 3;
  for (let i = 0; i < bands; i += 1) out.push(rect(-26 * length + i * 6, -5, 2.8, 14, p.accent, { opacity: 0.9 }));
  out.push(poly([[-30 * length, 0], [-38 * length, 3], [-30 * length, 5]], p.accent, p.line));
  out.push(ellipse(4, 0, 8, 6, p.hull, inked(p)));
  out.push(circle(15, -1, 5, p.hull, inked(p)));
  out.push(ellipse(17, -2, 2.4, 3, p.glow, inked(p)));
  out.push(path("M16 -5Q20 -14 26 -14M18 -5Q24 -10 28 -8", "none", { stroke: p.dark, "stroke-width": 0.8 }));
  out.push(rect(20, 1, 6, 1.4, p.dark));
  return out.join("");
}

function swarm(rng: Rng, p: Paint, kind: string): string {
  if (kind !== "swarm") return insect(p, kind);
  const out: string[] = [];
  for (const [x, y, s] of [[-22, -18, 0.42], [20, -22, 0.38], [-26, 16, 0.36], [24, 14, 0.44], [0, -2, 0.62]] as const) {
    out.push(group(at(x, y, between(rng, -25, 25), s), insect(p, rng() < 0.5 ? "wasp" : "hornet")));
  }
  return out.join("");
}

/** A laurel's leaf lies along its branch. */
const LAUREL_TURN = 90;
/** Its leaves fan off the branch, alternately. */
const LAUREL_LEAN = 28;

function pilot(rng: Rng, p: Paint, kind: string, laurels: boolean): string {
  const out: string[] = [];
  if (laurels) {
    out.push(path("M-12 38A34 30 0 0 1 -30 -14M12 38A34 30 0 0 0 30 -14", "none", { stroke: p.accent, "stroke-width": 0.9, opacity: 0.8 }));
    for (const side of [-1, 1]) {
      for (let i = 0; i < 7; i += 1) {
        const a = (90 + side * (40 + i * 18)) * DEGREES;
        const x = 34 * Math.cos(a);
        const y = 6 + 30 * Math.sin(a);
        const lean = LAUREL_TURN + (i % 2 === 0 ? LAUREL_LEAN : -LAUREL_LEAN);
        out.push(ellipse(x, y, 5, 2.1, p.accent, { ...inked(p), transform: `rotate(${fmt(a / DEGREES + lean)} ${fmt(x)} ${fmt(y)})` }));
      }
    }
  }
  out.push(path("M-30 42Q-30 16 -12 12L12 12Q30 16 30 42Z", "url(#hl)", inked(p)));
  out.push(path("M-30 42Q-30 16 -12 12L-6 12L-14 42Z", p.dark, { opacity: 0.3 }));
  out.push(ellipse(0, 13, 13, 4, p.accent, inked(p)));
  out.push(rect(-16, 22, 8, 5, p.accent, { ...inked(p), rx: 1 }));
  out.push(circle(-12, 24.5, 1, p.glow));
  if (laurels) out.push(twinkle(14, 26, 5, p.accent, 1));
  if (kind === "felinor") {
    out.push(poly([[-13, -14], [-12, -27], [-4, -19]], p.light, p.line));
    out.push(poly([[13, -14], [12, -27], [4, -19]], p.light, p.line));
  }
  out.push(circle(0, -4, 16, p.light, inked(p)));
  out.push(path("M-10 -4A10 10 0 0 0 10 -4", "none", { stroke: p.accent, "stroke-width": 1.2, opacity: 0.6 }));
  out.push(path("M-12 -8Q-12 -15 0 -15Q12 -15 12 -8L12 -2Q12 4 0 4Q-12 4 -12 -2Z", "url(#vz)", inked(p)));
  out.push(path("M-8 -11Q-4 -13 2 -13", "none", { stroke: "#ffffff", "stroke-width": 1.2, opacity: 0.7, "stroke-linecap": "round" }));
  for (let i = 0; i < 3; i += 1) out.push(circle(between(rng, -8, 8), between(rng, -9, 1), 0.5, "#ffffff", { opacity: 0.9 }));
  out.push(line(14, -12, 18, -24, p.accent, 1));
  out.push(circle(18, -24, 1.4, p.glow));
  return out.join("");
}

function ai(p: Paint, kind: string): string {
  const out: string[] = [];
  out.push(circle(0, 0, 36, "none", { stroke: p.glow, "stroke-width": 0.8, opacity: 0.5, "stroke-dasharray": "6 3" }));
  out.push(circle(0, 0, 28, "none", { stroke: p.glow, "stroke-width": 1.2, opacity: 0.7 }));
  for (let i = 0; i < 12; i += 1) {
    const a = (i * 30 * DEGREES);
    out.push(line(28 * Math.cos(a), 28 * Math.sin(a), 32 * Math.cos(a), 32 * Math.sin(a), p.glow, 0.8, { opacity: 0.8 }));
  }
  for (const [x1, y1, x2, y2, x3, y3] of [[-14, -6, -22, -6, -26, -12], [14, 6, 22, 6, 26, 12], [-6, 14, -6, 22, -12, 26], [6, -14, 6, -22, 12, -26]] as const) {
    out.push(path(`M${x1} ${y1}L${x2} ${y2}L${x3} ${y3}`, "none", { stroke: p.glow, "stroke-width": 0.8, opacity: 0.8 }));
    out.push(circle(x3, y3, 1.2, p.glow));
  }
  for (let i = 0; i < 4; i += 1) {
    for (const turn of [0, 90, 180, 270]) out.push(group(at(0, 0, turn), rect(-9 + i * 5, -17, 2, 4, p.light, { opacity: 0.8 })));
  }
  out.push(rect(-14, -14, 28, 28, "url(#hl)", { ...inked(p), rx: 3 }));
  out.push(rect(-10, -10, 20, 20, "none", { stroke: p.glow, "stroke-width": 0.6, opacity: 0.6, rx: 2 }));
  if (kind === "unit") {
    out.push(rect(-7, -3, 14, 5, p.glow, { rx: 2.5 }));
    out.push(rect(-4, -2, 3, 3, "#ffffff"));
  } else {
    out.push(circle(0, 0, 5.5, p.glow));
    out.push(circle(0, 0, 2.4, p.dark));
  }
  return out.join("");
}

function scenic(rng: Rng, p: Paint, kind: string, name: string | undefined): string {
  const motif = motifFor(name);
  const emblem: EmblemGlyph = motif === null ? (kind === "reticle" ? "eye" : "star") : MOTIFS[motif].glyph;
  const out: string[] = [];
  if (kind === "horizon") {
    out.push(circle(0, 112, 84, p.glow, { opacity: 0.25 }));
    out.push(circle(0, 114, 80, "url(#pl2)", inked(p)));
    for (const [x, h] of [[-14, 10], [-6, 16], [4, 8], [12, 13]] as const) out.push(rect(x, 34 - h, 6, h + 2, p.dark, inked(p)));
    out.push(glyph(emblem, 0, -10, 30, p.light, p.dark));
    return out.join("");
  }
  if (kind === "reticle") {
    out.push(circle(0, 0, 30, "none", { stroke: p.accent, "stroke-width": 1.4 }));
    out.push(circle(0, 0, 22, "none", { stroke: p.accent, "stroke-width": 0.6, opacity: 0.6, "stroke-dasharray": "3 2" }));
    for (const turn of [0, 90, 180, 270]) out.push(group(at(0, 0, turn), rect(24, -0.8, 14, 1.6, p.accent)));
    out.push(glyph(emblem, 0, 0, 26, p.light, p.dark));
    return out.join("");
  }
  const ringed = rng() < RING_CHANCE;
  if (ringed) out.push(group(at(0, 0, -18), path("M-36 0A36 9 0 0 1 36 0", "none", { stroke: p.light, "stroke-width": 1.4, opacity: 0.6 })));
  out.push(circle(0, 0, 22, "url(#pl2)", inked(p)));
  if (ringed) out.push(group(at(0, 0, -18), path("M-36 0A36 9 0 0 0 36 0", "none", { stroke: p.light, "stroke-width": 1.4, opacity: 0.8 })));
  out.push(glyph(emblem, 0, 0, 24, p.light, p.dark));
  return out.join("");
}

// The document

const SUBJECT_TILTS: Readonly<Partial<Record<StinkyFamily, number>>> = { fleet: -18, weapon: -14, miner: -16, jlockeed: -12, swarm: -10 };
/** Weapons fire to the right, so their mounts sit left of the middle and the shot stays in the window. */
const SUBJECT_SHIFTS: Readonly<Partial<Record<StinkyFamily, number>>> = { weapon: -9, jlockeed: -7, miner: -6 };
const SUBJECT_SCALES: Readonly<Partial<Record<StinkyFamily, number>>> = { fleet: 1.14, miner: 1.08, weapon: 1.06, jlockeed: 1.06 };
const PAINT_SALT = 0x7f4a7c15;
/** How far a crew member's suit and visor, or an Acclaimed's, turn from the family's own. */
const SUIT_DRIFT: Readonly<Partial<Record<StinkyFamily, number>>> = { crew: 80, acclaimed: 28 };
const VISOR_DRIFT = 50;

/** The variety a family allows inside its look, drawn before any gilding so both faces share it. */
function drift(seed: number, family: StinkyFamily, paint: Paint): Paint {
  const rng = seededRandom((seed ^ PAINT_SALT) >>> 0);
  const suit = SUIT_DRIFT[family];
  if (suit !== undefined) {
    return { ...paint, hull: turnHue(paint.hull, between(rng, -suit, suit)), glow: turnHue(paint.glow, between(rng, -VISOR_DRIFT, VISOR_DRIFT)) };
  }
  if (family === "space") {
    const [lit, shade] = PLANET_COLORS[whole(rng, 0, PLANET_COLORS.length - 1)] ?? PLANET_COLORS[0] ?? [paint.glow, paint.dark];
    return { ...paint, glow: lit, hull: mixHex(lit, shade, 0.5), dark: shade };
  }
  return paint;
}

function subjectMarkup(rng: Rng, p: Paint, subject: StinkySubject, lower: string, name: string | undefined): string {
  const { family, kind, tier } = subject;
  switch (family) {
    case "fleet":
      return ["fighter", "cruiser", "battleship", "dormant"].includes(kind) ? hull(rng, p, kind) : piece(rng, p, kind);
    case "weapon":
      return weapon(rng, p, kind, tier, lower);
    case "jlockeed":
      return kind === "blueprint" ? hull(rng, p, "battleship") : weapon(rng, p, kind, tier, lower);
    case "drone":
      return drone(p, kind, tier);
    case "miner":
      return miner(rng, p, kind, tier);
    case "resource":
      return resource(rng, p, kind, lower);
    case "anomaly":
      return anomaly(rng, p, kind);
    case "doctrine":
      return doctrine(p, kind);
    case "terminus":
      return terminus(rng, p);
    case "swarm":
      return swarm(rng, p, kind);
    case "acclaimed":
      return pilot(rng, p, kind, true);
    case "crew":
      return pilot(rng, p, kind, false);
    case "ai":
      return ai(p, kind);
    case "space":
      return scenic(rng, p, kind, name);
  }
}

function gradients(p: Paint, defs: Defs): void {
  defs.push(
    `<linearGradient id="hl" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${p.light}"/>` +
      `<stop offset="0.45" stop-color="${p.hull}"/><stop offset="1" stop-color="${p.dark}"/></linearGradient>`,
    `<radialGradient id="dr" cx="0.35" cy="0.3" r="0.8"><stop offset="0" stop-color="${p.light}"/>` +
      `<stop offset="0.55" stop-color="${p.hull}"/><stop offset="1" stop-color="${p.dark}"/></radialGradient>`,
    `<radialGradient id="rk" cx="0.35" cy="0.3" r="0.85"><stop offset="0" stop-color="${p.light}"/>` +
      `<stop offset="0.5" stop-color="${p.hull}"/><stop offset="1" stop-color="${p.dark}"/></radialGradient>`,
    `<linearGradient id="vz" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${p.glow}"/>` +
      `<stop offset="1" stop-color="${p.dark}"/></linearGradient>`,
    `<radialGradient id="ad"><stop offset="0.55" stop-color="#ffffff"/><stop offset="0.7" stop-color="${p.accent}"/>` +
      `<stop offset="1" stop-color="${p.hull}" stop-opacity="0.2"/></radialGradient>`,
    `<radialGradient id="pl2" cx="0.35" cy="0.3" r="0.8"><stop offset="0" stop-color="${p.glow}"/>` +
      `<stop offset="0.6" stop-color="${p.hull}"/><stop offset="1" stop-color="${p.dark}"/></radialGradient>`,
    `<radialGradient id="v" gradientUnits="userSpaceOnUse" cx="${HALF}" cy="${HALF}" r="${VIGNETTE_RADIUS}">` +
      `<stop offset="${VIGNETTE_START}" stop-color="#000" stop-opacity="0"/>` +
      `<stop offset="1" stop-color="#000" stop-opacity="${VIGNETTE_OPACITY}"/></radialGradient>`,
  );
}

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

/** The SVG document a Stinkyscape face draws, unencoded. */
export function stinkyscapeSvg(defId: string, name: string | undefined, tags: readonly Tag[], type: CardType, radiant: boolean): string {
  const subject = stinkySubject(defId, name, tags, type);
  const lower = (name ?? "").toLowerCase();
  const seed = hashId(defId);
  const based = drift(seed, subject.family, kindPaint(FAMILY_PAINTS[subject.family], subject, lower));
  const p = radiant ? gild(based) : based;
  const behind = seededRandom((seed ^ BACKDROP_SALT) >>> 0);
  const ahead = seededRandom((seed ^ SUBJECT_SALT) >>> 0);
  const tilt = (SUBJECT_TILTS[subject.family] ?? 0) + between(ahead, -SUBJECT_TILT_JITTER, SUBJECT_TILT_JITTER);
  const scale = (SUBJECT_SCALES[subject.family] ?? 1) * (1 + between(ahead, -SUBJECT_SCALE_JITTER, SUBJECT_SCALE_JITTER));
  const defs: Defs = [];
  gradients(p, defs);
  const planet = subject.family !== "terminus" && !(subject.family === "space" && subject.kind !== "reticle");
  const sky = backdrop(behind, p, subject.family, planet, defs);
  const drawn = subjectMarkup(ahead, p, subject, lower, name);
  return (
    `<svg viewBox="0 0 ${BOX} ${BOX}" width="${BOX}" height="${BOX}" xmlns="http://www.w3.org/2000/svg" preserveAspectRatio="xMidYMid slice">` +
    `<defs>${defs.join("")}</defs>` +
    sky +
    (radiant ? rays(seed) : "") +
    group(at(SUBJECT_X + (SUBJECT_SHIFTS[subject.family] ?? 0), SUBJECT_Y, tilt, scale), drawn) +
    rect(0, 0, BOX, BOX, "url(#v)") +
    "</svg>"
  );
}

// R2249000: the Starship's own art

/** The seed every Starship's art grows from, so a Starship keeps one look whatever fuses into it. */
const STARSHIP_SEED_ID = "stinkyscape-t-dormant-starship";
/** The most modules, escorts and lit pieces one Starship's art draws, however many pieces it holds. */
const STARSHIP_MODULES_MAX = 8;
const STARSHIP_ESCORTS_MAX = 3;
const STARSHIP_STREAKS = 14;
const STARSHIP_SCALE = 1.22;
const STARSHIP_TILT = -14;

/** A warp streak behind a launched Starship, a line of light running off to the left. */
function streak(rng: Rng, p: Paint): string {
  const y = between(rng, 6, 94);
  const x = between(rng, 40, 100);
  const length = between(rng, 18, 46);
  return line(x, y, x - length, y + between(rng, -2, 2), mixHex(p.glow, "#ffffff", 0.5), between(rng, 0.25, 0.7), {
    opacity: between(rng, 0.25, 0.6),
  });
}

/** One module fused onto the hull's flank: a pod on a strut, its core lit once the ship launches. */
function module(p: Paint, x: number, y: number, side: number, lit: boolean): string {
  const strut = line(x, 0, x, y, p.dark, 1.4);
  const pod = ellipse(x, y, 4.6, 2.6, "url(#hl)", inked(p));
  const core = circle(x + 1.2, y, 1.1, lit ? p.glow : p.dark, lit ? { opacity: 0.95 } : {});
  const fin = poly([[x - 4, y], [x - 7, y + side * 3], [x - 2.5, y + side * 1.8]], p.hull, p.line);
  return strut + fin + pod + core;
}

/**
 * R2249000: the art every Starship wears, Dormant or launched and however many pieces are fused into it:
 * a capital ship under its shield, a module on its flanks for each piece (up to `STARSHIP_MODULES_MAX`),
 * its engines cold and its scaffold up while it is Dormant, and once it launches its engines lit, its
 * shield glowing, warp streaks behind it and escorts beside it. Its seed is the hull's, never the fused
 * definition's, which changes with every piece.
 */
export function starshipSvg(pieces: number, launched: boolean, radiant: boolean): string {
  const seed = hashId(STARSHIP_SEED_ID);
  const base: Paint = { ...FAMILY_PAINTS.fleet, glow: launched ? "#7fe8ff" : FAMILY_PAINTS.fleet.glow };
  const p = radiant ? gild(base) : base;
  const behind = seededRandom((seed ^ BACKDROP_SALT) >>> 0);
  const ahead = seededRandom((seed ^ SUBJECT_SALT) >>> 0);
  const defs: Defs = [];
  gradients(p, defs);
  defs.push(
    `<radialGradient id="sh"><stop offset="0.6" stop-color="${p.glow}" stop-opacity="0"/>` +
      `<stop offset="0.9" stop-color="${p.glow}" stop-opacity="${launched ? 0.35 : 0.12}"/>` +
      `<stop offset="1" stop-color="${p.glow}" stop-opacity="0"/></radialGradient>`,
    `<radialGradient id="ha"><stop offset="0" stop-color="${p.nebula[0]}" stop-opacity="0.7"/>` +
      `<stop offset="1" stop-color="${p.nebula[1]}" stop-opacity="0"/></radialGradient>`,
  );
  const sky = backdrop(behind, p, "fleet", true, defs);
  const body: string[] = [];
  // The halo behind it and, launched, the warp streaks it leaves.
  body.push(ellipse(0, 0, 52, 30, "url(#ha)", { opacity: 0.55 }));
  const shown = Math.max(0, Math.min(pieces, STARSHIP_MODULES_MAX));
  const lit = launched ? shown : 0;
  // Its modules, one a piece, alternating flanks from the stern forward.
  const modules: string[] = [];
  for (let i = 0; i < shown; i += 1) {
    const side = i % 2 === 0 ? -1 : 1;
    const x = -26 + Math.floor(i / 2) * 13;
    modules.push(module(p, x, side * between(ahead, 17, 21), side, i < lit));
  }
  body.push(modules.join(""));
  body.push(group(at(0, 0, 0, 1.18), hull(ahead, p, launched ? "battleship" : "dormant")));
  // The shield: a faint bubble while it sleeps, a glowing one once it flies.
  body.push(ellipse(2, 0, 50, 27, "url(#sh)"));
  body.push(ellipse(2, 0, 50, 27, "none", { stroke: p.glow, "stroke-width": 0.5, opacity: launched ? 0.7 : 0.25, "stroke-dasharray": launched ? "none" : "2 2" }));
  if (launched) {
    const escorts = Math.max(1, Math.min(STARSHIP_ESCORTS_MAX, Math.ceil(pieces / 2)));
    for (let i = 0; i < escorts; i += 1) {
      const y = (i % 2 === 0 ? -1 : 1) * (32 + i * 4);
      body.push(group(at(18 - i * 16, y, 0, 0.3), hull(ahead, p, "fighter")));
    }
  }
  const streaks = launched ? Array.from({ length: STARSHIP_STREAKS }, () => streak(behind, p)).join("") : "";
  return (
    `<svg viewBox="0 0 ${BOX} ${BOX}" width="${BOX}" height="${BOX}" xmlns="http://www.w3.org/2000/svg" preserveAspectRatio="xMidYMid slice">` +
    `<defs>${defs.join("")}</defs>` +
    sky +
    streaks +
    (radiant ? rays(seed) : "") +
    group(at(SUBJECT_X, SUBJECT_Y, STARSHIP_TILT, STARSHIP_SCALE), body.join("")) +
    rect(0, 0, BOX, BOX, "url(#v)") +
    "</svg>"
  );
}
