// R2170000 (#2170): the Stinkyscape set's own art, drawn instead of `artSpec`: a space backdrop under
// one subject its family fixes and its name details. Presentation only (CLAUDE.md rule 7); geometry
// comes from streams the Radiant face shares, so the faces differ only in paint and gold rays. No
// other set reaches this file, so no shipped card's picture moves (R503). R3333001: a Starship Piece's
// own card draws its module (starshipModules.ts) in its bay; R3333000: a Starship in play is drawn from
// its pieces and its design (starshipHulls.ts, `starshipSvg`); R3333002: a piece's module icon.

import type { CardType, Tag, UnitView } from "@jackioh/shared";

import type { AnomalyCode } from "../anomalies.ts";
import type { EmblemGlyph } from "./emblems.ts";
import { fmt, hashId, seededRandom } from "./hash.ts";
import { CENTER_ORIGIN, motionStyle, moving, particleMarkup, type ArtMotion } from "./motion.ts";
import { MOTIFS, motifFor } from "./motifs.ts";
import { partDraws, partHolds, type ArtPart } from "./pictures.ts";
import {
  escortMarkup, hardpointPlacements, hullMarkup, ledgerMarkup, liveryPaint, shipyardMarkup, STARSHIP_DESIGNS, STARSHIP_MODULES_MAX,
  starshipClassOf, type HullPaint, type StarshipArt,
} from "./starshipHulls.ts";
import type { StarshipTrail } from "./starshipLooks.ts";
import {
  MODULE_BOX, moduleCenter, moduleMarkup, modulePaint, STARSHIP_MODULE_IDS, starshipModuleOf, type Hangar, type StarshipModule,
} from "./starshipModules.ts";
import {
  at, between, circle, DEGREES, ellipse, FULL_TURN, glyph, group, inked, line, mirror, OUTLINE, path, points, poly, rect, STROKE, twinkle,
  whole,
  type Paint, type Rng,
} from "./stinkyDraw.ts";
import { mixHex, RADIANT_PALETTE, turnHue } from "./themes.ts";

export const STINKYSCAPE_PREFIX = "stinkyscape-";

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
 * list is kept here, and `stinkyscape.test.ts` holds it to the catalog's. The Stinkyscape Golem left it
 * with the keyword (#3142, R3142002) and is drawn from its name (`FLEET_NAMES`).
 */
export const STARSHIP_PIECES: ReadonlySet<string> = new Set(
  [
    "038", "040", "043", "048", "049", "050", "051", "052", "053", "054", "055", "056", "057", "058", "059",
    "060", "061", "062", "063", "064", "065", "066", "067", "079", "090", "093", "098", "100", "101", "104",
    "105", "106", "107", "108", "122", "131", "132", "133", "134", "136", "140", "141", "142", "143", "175",
    "177", "t-ai-10",
  ].map((suffix) => `${STINKYSCAPE_PREFIX}${suffix}`),
);

/**
 * The ships #2521 took the Starship Piece keyword from: still ships, so still drawn as the fleet they
 * shipped as (R503).
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

/**
 * The fleet's cards that print no Starship Piece: the hull waiting to launch, the base and, since it lost the
 * keyword (#3142), the Stinkyscape Golem, a golem and ship hybrid (R3142002). The Training Grounds Golem is
 * `traininggrounds.ts`'s, so it never reaches this file.
 */
const FLEET_NAMES: readonly string[] = ["dormant starship", "megabase", "golem"];
const FIGHTER_NAMES: readonly string[] = ["falcon", "stratos", "yuma", "honey badger"];
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

/**
 * R3338001: the Terminus cards' pictures of their own, by id (Dice of Doom prints no Terminus tag, so its id is what
 * finds it): Olivian Expanse and its six Terminus cards, and Dice of Doom. Any other Terminus card keeps the shared
 * black hole (`terminus`, the `hole` kind).
 */
export const TERMINUS_KINDS: Readonly<Record<string, string>> = {
  "stinkyscape-004": "expanse",
  "stinkyscape-004-1": "lamp",
  "stinkyscape-004-2": "exam",
  "stinkyscape-004-3": "edge",
  "stinkyscape-004-4": "crown",
  "stinkyscape-004-5": "destination",
  "stinkyscape-004-6": "crunch",
  "stinkyscape-154": "d100",
};

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

/** R3142002: the word that makes a fleet card the golem and ship hybrid. */
const GOLEM_NAME = "golem";

function fleetKind(lower: string): string {
  const piece = wordIn(lower, PIECE_WORDS);
  if (piece !== null) return piece;
  if (lower.includes(GOLEM_NAME)) return "golem";
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
  const terminusKind = Object.hasOwn(TERMINUS_KINDS, defId) ? TERMINUS_KINDS[defId] : undefined;
  if (terminusKind !== undefined) return { family: "terminus", kind: terminusKind, tier };
  if (has("AI")) return { family: "ai", kind: type === "Unit" ? "unit" : "chip", tier };
  if (has("Terminus") || lower.includes("olivian")) return { family: "terminus", kind: "hole", tier };
  if (has("War Doctrine") || lower.includes("firststrike")) {
    const doctrine = DOCTRINE_WORDS.find(([word]) => wordsHave(lower, word))?.[1] ?? "strike";
    return { family: "doctrine", kind: doctrine, tier };
  }
  // R3142002: a Human Unit is a person whatever its name holds, so the Drone Commander is an officer with a
  // drone hovering behind her and not a drone ship; a Drone Interceptor or Corvette (no Human) stays a drone.
  if (type === "Unit" && has("Human") && wordsHave(lower, "drone")) return { family: "crew", kind: "commander", tier };
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

/**
 * R3338001: each Terminus picture's own tones over the family's: a violet expanse, a lamp's warm wood, chalk on a
 * slate sky, a blue-white light at the horizon, a hand and its gold crown, a cold lane over a violet abyss, a
 * collapse burning white, a pale die on black velvet. The shared black hole (`hole`) has none, so it keeps its own.
 */
const TERMINUS_TONES: Readonly<Record<string, Partial<Paint>>> = {
  expanse: { space: ["#1c0b33", "#030008"], nebula: ["#8a3ae0", "#3a1a9a"], light: "#eedcff", hull: "#6a3ac8", dark: "#12062a", accent: "#ffe7b0", glow: "#d2a8ff" },
  lamp: { nebula: ["#5a2a10", "#1a1030"], light: "#ffe9b8", hull: "#6a4026", dark: "#140a04", accent: "#ffcf6a", glow: "#ffd28a" },
  exam: { space: ["#0e1a16", "#010403"], nebula: ["#2a5a4a", "#2a1a4a"], light: "#f4f1e6", hull: "#7a5a3a", dark: "#0a1210", accent: "#ffcf6a", glow: "#e8f4ff" },
  edge: { nebula: ["#1a3a8a", "#3a1a6a"], light: "#ffffff", hull: "#3a6ad8", dark: "#000000", accent: "#ffffff", glow: "#bfe8ff" },
  crown: { nebula: ["#6a2a12", "#2a1a5a"], light: "#f0cfae", hull: "#c08a62", dark: "#2a1410", accent: "#ffd23a", glow: "#ffe9a0" },
  destination: { nebula: ["#5a1aa8", "#1a3a8a"], light: "#f0f4ff", hull: "#4a5ad8", dark: "#0a0c1c", accent: "#ff6a5a", glow: "#a8f0ff" },
  crunch: { nebula: ["#d43fd0", "#3a1a6a"], light: "#ffffff", hull: "#ff9a2a", dark: "#000000", accent: "#ff8af5", glow: "#fff0d8" },
  d100: { space: ["#0c0a10", "#000000"], nebula: ["#3a0a1a", "#16141e"], light: "#f6f4fa", hull: "#b9b6c8", dark: "#1c1a26", accent: "#ffcf6a", glow: "#ff6a5a" },
};

function kindPaint(base: Paint, subject: StinkySubject, lower: string): Paint {
  const kind = subject.kind;
  if (subject.family === "jlockeed") return base;
  if (subject.family === "terminus") return { ...base, ...(Object.hasOwn(TERMINUS_TONES, kind) ? TERMINUS_TONES[kind] : {}) };
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

const BOX = 100;
const HALF = BOX / 2;
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
  const bridgeX = cls === "fighter" ? L * 0.2 : -L * 0.42;
  out.push(rect(bridgeX, -W * 0.45, L * (cls === "fighter" ? 0.3 : 0.26), W * 0.9, p.light, { ...inked(p), rx: W * 0.2 }));
  const windows = cls === "fighter" ? 2 : 3;
  for (let i = 0; i < windows; i += 1) {
    out.push(rect(bridgeX + L * 0.04 + i * L * 0.07, -W * 0.18, L * 0.04, W * 0.36, dormant ? p.dark : p.glow));
  }
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

/**
 * R3142002: the Stinkyscape Golem, a golem and ship hybrid: a stone-plated humanoid, bow to the right, with
 * a cockpit visor for a face, wing fins on its back and engine nozzles burning at its heels.
 */
function golem(rng: Rng, p: Paint): string {
  const out: string[] = [];
  // Wing fins swept back behind the shoulders.
  for (const side of [-1, 1]) {
    out.push(poly([[-6, side * 10], [-30, side * 30], [-38, side * 30], [-28, side * 11]], "url(#hl)", p.line));
    out.push(rect(-37, side > 0 ? 28.6 : -30, 8, 1.4, p.accent));
  }
  // Engine nozzles under the legs, and their burn.
  for (const y of [-9, 9]) {
    out.push(poly([[-27, y - 2.6], [-27 - between(rng, 10, 16), y], [-27, y + 2.6]], p.glow, "none", { opacity: 0.85 }));
    out.push(poly([[-27, y - 1.2], [-33, y], [-27, y + 1.2]], "#ffffff", "none", { opacity: 0.8 }));
    out.push(rect(-30, y - 3.4, 6, 6.8, p.dark, inked(p)));
  }
  // Legs and stone-plated torso, a block built of slabs.
  out.push(rect(-26, -13, 17, 9, "url(#rk)", { ...inked(p), rx: 2 }));
  out.push(rect(-26, 4, 17, 9, "url(#rk)", { ...inked(p), rx: 2 }));
  out.push(poly([[-12, -17], [12, -20], [22, -8], [22, 8], [12, 20], [-12, 17], [-18, 9], [-18, -9]], "url(#rk)", p.line));
  out.push(poly([[-18, 2], [22, 2], [22, 8], [12, 20], [-12, 17], [-18, 9]], p.dark, "none", { opacity: 0.35 }));
  for (const [x1, y1, x2, y2] of [[-10, -17, -6, 17], [2, -19, 6, 19], [-18, -2, 22, -2]] as const) {
    out.push(line(x1, y1, x2, y2, p.dark, 0.5, { opacity: 0.75 }));
  }
  // Shoulder slabs and fists.
  for (const side of [-1, 1]) {
    out.push(poly([[0, side * 17], [14, side * 20], [22, side * 25], [10, side * 29], [-2, side * 24]], "url(#rk)", p.line));
    out.push(circle(24, side * 27, 4.2, "url(#dr)", inked(p)));
    out.push(circle(22.5, side * 25.8, 1.2, p.light, { opacity: 0.5 }));
  }
  // The head with its cockpit visor, a slit of canopy glass at the bow.
  out.push(poly([[14, -9], [30, -6], [34, 0], [30, 6], [14, 9]], "url(#rk)", p.line));
  out.push(path("M20 -5.5L30 -3.2L32.4 0L30 3.2L20 5.5Z", "url(#vz)", inked(p)));
  out.push(path("M22 -3.4L29 -2", "none", { stroke: "#ffffff", "stroke-width": 0.8, opacity: 0.7, "stroke-linecap": "round" }));
  out.push(rect(14, -10.4, 7, 1.6, p.accent));
  out.push(rect(14, 8.8, 7, 1.6, p.accent));
  // The reactor at the chest.
  out.push(circle(2, 0, 4.6, p.dark, inked(p)));
  out.push(circle(2, 0, 3.2, p.glow));
  out.push(circle(1, -1, 1, "#ffffff", { opacity: 0.8 }));
  return out.join("");
}

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

// R3336000: the Drone Battleship's three layers, bottom to top, and the drones its Propagation layer launches.

/** The Battleship's layers, the bottom of its pile first (R1842000). */
export const SHIP_LAYERS = ["meltdown", "propagation", "shield"] as const;
export type ShipLayer = (typeof SHIP_LAYERS)[number];
/** The layers' token ids, by layer. */
export const SHIP_LAYER_IDS: Readonly<Record<ShipLayer, string>> = {
  meltdown: "stinkyscape-t-meltdown-layer",
  propagation: "stinkyscape-t-propagation-layer",
  shield: "stinkyscape-t-shield-layer",
};
/** The drones its bays launch: Activate 2 shoots an Interceptor out, Activate rolls a Corvette. */
export const SHIP_DRONE_IDS = { interceptor: "stinkyscape-t-drone-interceptor", corvette: "stinkyscape-t-drone-corvette" } as const;

/** R3336000: the Battleship layer a card is, or null for any other card. */
export function shipLayerOf(defId: string): ShipLayer | null {
  return SHIP_LAYERS.find((layer) => SHIP_LAYER_IDS[layer] === defId) ?? null;
}

/**
 * R3336000: what a layer on the board knows of its ship: `buried` cards lie beneath it (its decks below), `heat` is how
 * far the reactor has melted in `SHIP_HEAT_STEPS` steps, `brittle` its Brittle count, `bays` how many of its bays still
 * hold drones. A printed card (no ship) draws its deck large with the others faint.
 */
export type ShipArt = { buried: number; heat?: number; brittle?: number; bays?: number };

/**
 * Each deck's distance from the next in a stack on the board; a printed card's own deck is large in the middle, and the
 * decks behind it faint, the nearest `SHIP_BEHIND_NEAR` above or below it and each next one `SHIP_BEHIND_STEP` beyond.
 */
const SHIP_DECK_PITCH = 20;
const SHIP_BEHIND_NEAR = 22;
const SHIP_BEHIND_STEP = 17;
const SHIP_FAINT_OPACITY = 0.3;
const SHIP_BEHIND_SCALE = 0.85;
const SHIP_OWN_SCALE = 1.3;
/** The reactor's heat in steps, the cracks its keel can show, and the Brittle count at or under which one more shows. */
const SHIP_HEAT_STEPS = 6;
const SHIP_CRACKS_MAX = 6;
const SHIP_BRITTLE_CRACK_AT = 2;
/** The hangar's bays; a printed card shows both lit, and the keel its reactor a third melted. */
export const SHIP_BAYS = 2;
const SHIP_PRINTED_HEAT = 2;

/** The decks drawn for `layer` on a board, bottom first: its own and what lies beneath it; the count is `n`. */
function shipDecks(layer: ShipLayer, buried: number): readonly ShipLayer[] {
  const own = SHIP_LAYERS.indexOf(layer);
  return SHIP_LAYERS.slice(Math.max(0, own - Math.max(0, buried)), own + 1);
}

/** R3336000: how many decks `defId`'s picture draws given its ship (a printed card draws all three), or null for any other card. */
export function shipDecksOf(defId: string, ship: ShipArt | null): number | null {
  const layer = shipLayerOf(defId);
  if (layer === null) return null;
  return ship === null ? SHIP_LAYERS.length : shipDecks(layer, ship.buried).length;
}

/** R3336000: what a layer unit on the board draws of its ship, quantised so the art caches; null for any other card. */
export function shipArtOf(defId: string, unit: Pick<UnitView, "buried" | "health" | "maxHealth" | "brittle" | "activations">): ShipArt | null {
  if (shipLayerOf(defId) === null) return null;
  const melted = unit.maxHealth > 0 ? Math.round(SHIP_HEAT_STEPS * (1 - unit.health / unit.maxHealth)) : 0;
  return {
    buried: unit.buried,
    heat: Math.min(SHIP_HEAT_STEPS, Math.max(0, melted)),
    brittle: unit.brittle,
    bays: unit.activations === undefined ? SHIP_BAYS : unit.activations.filter((activation) => activation.usesLeft !== 0).length,
  };
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

// R3336000: the Drone Battleship's three decks, each drawn in a box 74 wide and 18 high around its own center: the
// reactor keel, the hangar decks and the command superstructure.

/** The keel's cracks, the first `cracks` of them drawn: they spread from its edges in toward the core. */
const KEEL_CRACKS: readonly string[] = [
  "M-32 -3L-26 -1L-29 4",
  "M26 -6L30 -2L35 -3",
  "M-20 8L-15 5L-18 2",
  "M15 8L19 4L25 6",
  "M-9 -9L-6 -6L-10 -4",
  "M7 -9L11 -6L8 -3",
];

/** How many of the keel's cracks show at `heat` steps of melting and a Brittle count. */
function keelCracks(heat: number, brittle: number | undefined): number {
  const worn = brittle !== undefined && brittle <= SHIP_BRITTLE_CRACK_AT ? 1 : 0;
  return Math.min(SHIP_CRACKS_MAX, Math.ceil((SHIP_CRACKS_MAX * heat) / SHIP_HEAT_STEPS) + worn);
}

/** The reactor keel: a hull plate with a row of vents, the core glowing through them hotter with `heat`. */
function keelDeck(d: Paint, heat: number, cracks: number): string {
  const warm = heat / SHIP_HEAT_STEPS;
  const out: string[] = [poly([[-37, -4], [-31, -9], [31, -9], [37, -4], [29, 9], [-29, 9]], "url(#hl)", d.line)];
  out.push(circle(0, 1, 7 + 6 * warm, d.glow, { opacity: 0.1 + 0.25 * warm }));
  out.push(circle(0, 1, 4 + 3.5 * warm, d.glow, { opacity: 0.45 + 0.55 * warm, "data-core": heat }));
  out.push(circle(0, 1, 1.6 + 1.6 * warm, "#ffffff", { opacity: 0.35 + 0.65 * warm }));
  for (const y of [-4, -1.5, 1, 3.5]) out.push(rect(-14, y, 28, 1.1, d.dark, { opacity: 0.9 }));
  out.push(rect(-15, -5.5, 30, 11, "none", inked(d)));
  for (let i = 0; i < 9; i += 1) out.push(rect(-32 + i * 8, 6.4, 2.4, 1.6, d.glow));
  const drawn = KEEL_CRACKS.slice(0, Math.max(0, cracks));
  if (drawn.length > 0) out.push(path(drawn.join(""), "none", { stroke: d.glow, "stroke-width": 1, "stroke-linejoin": "round", "stroke-linecap": "round", "data-cracks": drawn.length }));
  return out.join("");
}

/** The hangar decks: two bay doors with drones racked inside, and a light above each, lit while it holds drones. */
function hangarDeck(d: Paint, bays: number): string {
  const out: string[] = [poly([[-37, -6], [-33, -9], [33, -9], [37, -6], [37, 6], [33, 9], [-33, 9], [-37, 6]], "url(#hl)", d.line)];
  [-17, 17].forEach((x, bay) => {
    out.push(rect(x - 12.5, -5.5, 25, 11, d.dark, inked(d)));
    for (const rack of [-8, 0, 8]) out.push(group(at(x + rack, 0.5), droneBody(d, 3.4, 0)));
    out.push(rect(x - 12.5, -5.5, 25, 2, d.hull, inked(d)));
    const lit = bay < bays;
    if (lit) out.push(circle(x, -7.4, 3.4, d.glow, { opacity: 0.25 }));
    out.push(rect(x - 3, -8.2, 6, 1.6, lit ? d.glow : d.dark, lit ? { "data-bay": "lit" } : { opacity: 0.6, "data-bay": "dark" }));
  });
  out.push(rect(-1.5, -9, 3, 18, d.hull, inked(d)));
  return out.join("");
}

/** The command superstructure: a tower on a wide deck, two emitter spires with glowing tips and the shield's dome. */
function bridgeDeck(d: Paint): string {
  const out: string[] = [poly([[-37, 9], [-33, 4], [-14, 4], [-11, -3], [11, -3], [14, 4], [33, 4], [37, 9]], "url(#hl)", d.line)];
  for (let i = 0; i < 5; i += 1) out.push(rect(-8 + i * 4, 0, 2.4, 1.4, d.glow));
  for (const x of [-27, 27]) {
    out.push(poly([[x - 2, 4], [x, -9], [x + 2, 4]], "url(#hl)", d.line));
    out.push(circle(x, -9, 4, d.glow, { opacity: 0.25 }));
    out.push(circle(x, -9, 1.9, d.glow, inked(d)));
  }
  out.push(path("M-13 -3A13 12 0 0 1 13 -3Z", d.glow, { opacity: 0.3, stroke: d.glow, "stroke-width": 0.9 }));
  out.push(path("M-8 -6A9 8 0 0 1 -2 -12", "none", { stroke: "#ffffff", "stroke-width": 0.8, opacity: 0.6, "stroke-linecap": "round" }));
  return out.join("");
}

/**
 * R3336000: the Battleship, its decks stacked as the pile stands. Each layer draws in its own color (its glow, gilded when
 * Radiant: R2725004). A printed card draws its own deck large with the other two faint behind it; a layer on the board
 * (`ship`) draws its own deck and every deck beneath it, centered as a stack, so the top card shows the ship as far as
 * it stands. The keel's core glows with `ship.heat` and its cracks spread with it and with a low Brittle; the hangar's
 * lights stay lit for the bays that still hold drones.
 */
function battleship(layer: ShipLayer, radiant: boolean, ship: ShipArt | null): string {
  const paint = (of: ShipLayer): Paint => {
    const own = { ...FAMILY_PAINTS.drone, glow: KIND_COLORS[of] ?? FAMILY_PAINTS.drone.glow };
    return radiant ? gild(own) : own;
  };
  const deck = (of: ShipLayer): string => {
    const d = paint(of);
    switch (of) {
      case "meltdown": {
        const heat = ship === null ? SHIP_PRINTED_HEAT : (ship.heat ?? 0);
        return keelDeck(d, heat, ship === null ? 0 : keelCracks(heat, ship.brittle));
      }
      case "propagation":
        return hangarDeck(d, ship === null ? SHIP_BAYS : (ship.bays ?? SHIP_BAYS));
      case "shield":
        return bridgeDeck(d);
    }
  };
  if (ship === null) {
    // The others, faint above or below it in the stack's order; then its own deck, large, in the middle.
    const own = SHIP_LAYERS.indexOf(layer);
    const behind = SHIP_LAYERS.map((other, index) => ({ other, away: index - own }))
      .filter(({ away }) => away !== 0)
      .map(({ other, away }) =>
        group(at(0, -Math.sign(away) * (SHIP_BEHIND_NEAR + SHIP_BEHIND_STEP * (Math.abs(away) - 1)), 0, SHIP_BEHIND_SCALE), deck(other), {
          "data-deck": other,
          opacity: SHIP_FAINT_OPACITY,
        }),
      );
    return behind.join("") + group(at(0, 0, 0, SHIP_OWN_SCALE), deck(layer), { "data-deck": layer, "data-own": "true" });
  }
  const decks = shipDecks(layer, ship.buried);
  return decks
    .map((of, i) => {
      const y = ((decks.length - 1) / 2 - i) * SHIP_DECK_PITCH;
      return group(at(0, y), deck(of), of === layer ? { "data-deck": of, "data-own": "true" } : { "data-deck": of });
    })
    .join("");
}

function drone(p: Paint, kind: string, tier: number, radiant: boolean, ship: ShipArt | null): string {
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
    default:
      // R3336000: the battleship's layers, the decks of one ship (a kind that is no layer draws as the meltdown's).
      out.push(battleship(SHIP_LAYERS.find((layer) => layer === kind) ?? "meltdown", radiant, ship));
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

/** The shared black hole's accretion disk, its back half; R3338001: no Terminus card's own picture draws it. */
export const TERMINUS_DISK_BACK = "M-40 0A40 11 0 0 1 40 0L26 0A26 6 0 0 0 -26 0Z";
const TERMINUS_DISK_FRONT = "M-40 0A40 11 0 0 0 40 0L26 0A26 6 0 0 1 -26 0Z";

/** The shared black hole, which any Terminus card `TERMINUS_KINDS` does not list still draws. */
function terminus(rng: Rng, p: Paint): string {
  const tilt = between(rng, -16, 16);
  const disk = (half: "back" | "front"): string =>
    group(at(0, 0, tilt), path(half === "back" ? TERMINUS_DISK_BACK : TERMINUS_DISK_FRONT, "url(#ad)", { opacity: 0.95 }));
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

// R3338001: the Terminus cards' pictures of their own (`TERMINUS_KINDS`), each drawn about (0, 0) as every subject
// is, from the subject's stream alone, so Alt art lays each out anew and the Radiant face keeps the geometry.

/** A path's `d` from its commands and numbers, each number written as SVG takes it. */
function pathOf(parts: readonly (string | number)[]): string {
  return parts.map((part) => (typeof part === "number" ? fmt(part) : part)).join(" ");
}

/** The Terminus cards falling across Olivian Expanse's sky. */
const EXPANSE_STARS = 6;

/** Olivian Expanse: an endless violet expanse, the six Terminus cards falling across its sky as distant stars. */
function expanseScene(rng: Rng, p: Paint): string {
  const out: string[] = [];
  const horizon = between(rng, 4, 12);
  const vanish = between(rng, -14, 14);
  // The plain, lit at its horizon and running off into the dark.
  out.push(rect(-60, horizon, 120, 60, "url(#hl)", { opacity: 0.62 }));
  out.push(ellipse(vanish, horizon, 58, 5, p.glow, { opacity: 0.35 }));
  out.push(ellipse(vanish, horizon, 30, 1.6, p.light, { opacity: 0.6 }));
  // Its rows, closer as they recede, and the lines that meet at the vanishing point: it goes on for ever.
  let gap = 1.6;
  for (let y = horizon + 2; y < 50; y += gap) {
    out.push(line(-60, y, 60, y, p.glow, 0.25 + gap * 0.03, { opacity: 0.3 }));
    gap *= 1.5;
  }
  for (let k = -6; k <= 6; k += 1) out.push(line(vanish, horizon, vanish + k * between(rng, 13, 17), 50, p.glow, 0.3, { opacity: 0.25 }));
  // The six cards, falling the same way, each in its own slot of the sky, a trail behind each.
  const fall = (rng() < 0.5 ? -1 : 1) * between(rng, 28, 48);
  const behind = (fall - 90) * DEGREES;
  const ux = Math.cos(behind);
  const uy = Math.sin(behind);
  for (let k = 0; k < EXPANSE_STARS; k += 1) {
    const x = -40 + (80 * (k + 0.5)) / EXPANSE_STARS + between(rng, -4, 4);
    const y = between(rng, -36, horizon - 12);
    const length = between(rng, 10, 18);
    out.push(poly([[x - uy * 0.8, y + ux * 0.8], [x + ux * length, y + uy * length], [x + uy * 0.8, y - ux * 0.8]], p.glow, "none", { opacity: 0.55 }));
    out.push(line(x, y, x + ux * length * 0.5, y + uy * length * 0.5, "#ffffff", 0.35, { opacity: 0.8 }));
    out.push(circle(x, y, 2.8, p.glow, { opacity: 0.35 }));
    out.push(group(at(x, y, fall), rect(-1.4, -2, 2.8, 4, p.light, { rx: 0.5, stroke: p.accent, "stroke-width": 0.4 })));
  }
  return out.join("");
}

/** Farewell: an empty table under a single lamp, every card gone but the marks where they lay. */
function lampScene(rng: Rng, p: Paint): string {
  const out: string[] = [];
  const lx = between(rng, -6, 6);
  const shade = between(rng, -32, -26);
  const tx = lx + between(rng, -4, 4);
  const ty = between(rng, 10, 15);
  const rx = between(rng, 24, 30);
  const ry = rx * 0.26;
  // The shadow on the floor, the legs, and the table's edge under its top.
  out.push(ellipse(tx, 40, rx * 0.95, 4, "#000000", { opacity: 0.5 }));
  for (const [dx, from, foot] of [[-0.4, -0.4, 34], [0.4, -0.4, 34], [-0.72, 0.4, 40], [0.72, 0.4, 40]] as const) {
    out.push(rect(tx + rx * dx - 1.1, ty + ry * from, 2.2, foot - ty - ry * from, p.dark, inked(p)));
  }
  out.push(ellipse(tx, ty + 2.2, rx, ry, p.dark, inked(p)));
  out.push(ellipse(tx, ty, rx, ry, p.hull, inked(p)));
  // The lamp's cone of light down to the table, and the pool it leaves there.
  const rim = shade + 9;
  out.push(poly([[lx - 7, rim], [lx + 7, rim], [tx + rx * 0.9, ty], [tx - rx * 0.9, ty]], p.glow, "none", { opacity: 0.14 }));
  out.push(poly([[lx - 4, rim], [lx + 4, rim], [tx + rx * 0.55, ty], [tx - rx * 0.55, ty]], p.glow, "none", { opacity: 0.12 }));
  out.push(ellipse(tx, ty, rx * 0.62, ry * 0.62, p.glow, { opacity: 0.45 }));
  // Where the cards lay: their outlines, and nothing in them.
  const marks = whole(rng, 3, 5);
  for (let k = 0; k < marks; k += 1) {
    const mx = tx + between(rng, -rx * 0.55, rx * 0.55);
    const my = ty + between(rng, -ry * 0.45, ry * 0.45);
    const turn = between(rng, -40, 40);
    out.push(
      group(
        `translate(${fmt(mx)} ${fmt(my)}) scale(1 0.3) rotate(${fmt(turn)})`,
        rect(-2.6, -3.6, 5.2, 7.2, "none", { stroke: p.light, "stroke-width": 0.7, "stroke-dasharray": "1 0.8", opacity: 0.7, rx: 0.6 }),
      ),
    );
  }
  // The lamp: its cord, its shade and its bulb.
  out.push(line(lx, -60, lx, shade, p.hull, 0.7));
  out.push(circle(lx, rim + 0.5, 7, p.glow, { opacity: 0.3 }));
  out.push(poly([[lx - 2.2, shade], [lx + 2.2, shade], [lx + 7.5, rim], [lx - 7.5, rim]], "url(#hl)", p.line));
  out.push(ellipse(lx, rim, 7.5, 1.4, p.dark, inked(p)));
  out.push(circle(lx, rim + 0.6, 2, "#ffffff", { opacity: 0.95 }));
  return out.join("");
}

type ChalkMark = (x: number, y: number, u: number) => string;

/** The marks KY's Last Exam chalks, each about two `u` tall about (x, y); no text, so no letter is drawn as one. */
const CHALK_MARKS: Readonly<Record<string, ChalkMark>> = {
  integral: (x, y, u) => pathOf(["M", x + u * 0.9, y - u * 1.6, "Q", x + u * 0.2, y - u * 2.2, x, y - u * 0.8, "L", x, y + u * 0.8, "Q", x - u * 0.2, y + u * 2.2, x - u * 0.9, y + u * 1.6]),
  sigma: (x, y, u) => pathOf(["M", x + u, y - u * 1.4, "L", x - u, y - u * 1.4, "L", x + u * 0.2, y, "L", x - u, y + u * 1.4, "L", x + u, y + u * 1.4]),
  pi: (x, y, u) => pathOf(["M", x - u * 1.1, y - u * 0.9, "L", x + u * 1.1, y - u * 1.1, "M", x - u * 0.5, y - u, "Q", x - u * 0.4, y + u * 0.6, x - u * 0.9, y + u * 1.2, "M", x + u * 0.5, y - u, "L", x + u * 0.6, y + u * 1.2]),
  root: (x, y, u) => pathOf(["M", x - u * 1.4, y + u * 0.2, "L", x - u * 0.9, y - u * 0.1, "L", x - u * 0.3, y + u * 1.3, "L", x + u * 0.3, y - u * 1.5, "L", x + u * 1.8, y - u * 1.5]),
  infinity: (x, y, u) => pathOf(["M", x, y, "Q", x - u * 0.6, y - u, x - u * 1.2, y, "Q", x - u * 0.6, y + u, x, y, "Q", x + u * 0.6, y - u, x + u * 1.2, y, "Q", x + u * 0.6, y + u, x, y]),
  square: (x, y, u) => pathOf(["M", x - u, y - u, "L", x + u * 0.6, y + u, "M", x + u * 0.6, y - u, "L", x - u, y + u, "M", x + u * 0.9, y - u * 1.6, "Q", x + u * 1.5, y - u * 2.2, x + u * 1.7, y - u * 1.4, "L", x + u * 0.9, y - u * 0.6, "L", x + u * 1.8, y - u * 0.6]),
  plus: (x, y, u) => pathOf(["M", x - u, y, "L", x + u, y, "M", x, y - u, "L", x, y + u]),
};
const CHALK_EQUALS: ChalkMark = (x, y, u) => pathOf(["M", x - u, y - u * 0.4, "L", x + u, y - u * 0.4, "M", x - u, y + u * 0.4, "L", x + u, y + u * 0.4]);
const CHALK_ASK: ChalkMark = (x, y, u) =>
  pathOf(["M", x - u * 0.8, y - u * 0.9, "Q", x - u * 0.6, y - u * 1.8, x, y - u * 1.8, "Q", x + u * 0.9, y - u * 1.7, x + u * 0.8, y - u * 0.8, "Q", x + u * 0.6, y - u * 0.1, x, y + u * 0.2, "L", x, y + u * 0.7, "M", x, y + u * 1.3, "L", x, y + u * 1.4]);
/** How many marks the problem writes before its "= ?". */
const CHALK_TERMS_MIN = 4;
const CHALK_TERMS_MAX = 5;
const CHALK_DUST = 10;
const CLOCK_HOURS = 12;
/** The one desk, drawn larger than the hall around it so it reads on a small face. */
const EXAM_DESK_SCALE = 1.4;

/** KY's Last Exam: one desk in an exam hall at the end of time, its clock at midnight, an impossible problem chalked across the sky. */
function examScene(rng: Rng, p: Paint): string {
  const out: string[] = [];
  // The problem: a shuffled run of marks, then "= ?", written along a slope across the sky.
  const terms = whole(rng, CHALK_TERMS_MIN, CHALK_TERMS_MAX);
  const shuffled = Object.values(CHALK_MARKS)
    .map((mark) => ({ mark, key: rng() }))
    .sort((a, b) => a.key - b.key)
    .slice(0, terms)
    .map((entry) => entry.mark);
  const marks = [...shuffled, CHALK_EQUALS, CHALK_ASK];
  const u = between(rng, 2.6, 3.2);
  const base = between(rng, -30, -22);
  const slope = between(rng, -0.08, 0.08);
  const step = 80 / marks.length;
  let problem = "";
  marks.forEach((mark, k) => {
    const x = -40 + step * (k + 0.5);
    problem += mark(x, base + slope * x + between(rng, -1.5, 1.5), u);
  });
  const chalk = { stroke: p.light, "stroke-linecap": "round", "stroke-linejoin": "round" } as const;
  out.push(path(problem, "none", { ...chalk, "stroke-width": 2.4, opacity: 0.16 }));
  out.push(path(problem, "none", { ...chalk, "stroke-width": 0.9, opacity: 0.9 }));
  out.push(path(pathOf(["M", -42, base + 7, "Q", 0, base + 10 + between(rng, -2, 2), 42, base + 6]), "none", { ...chalk, "stroke-width": 0.6, opacity: 0.6 }));
  for (let k = 0; k < CHALK_DUST; k += 1) {
    out.push(circle(between(rng, -42, 42), base + between(rng, -6, 8), between(rng, 0.15, 0.4), p.light, { opacity: between(rng, 0.3, 0.7) }));
  }
  // The hall's floor, its boards running off toward a vanishing point.
  const floor = between(rng, 20, 24);
  const vx = between(rng, -10, 10);
  out.push(rect(-60, floor, 120, 40, p.dark, { opacity: 0.6 }));
  out.push(line(-60, floor, 60, floor, p.light, 0.3, { opacity: 0.35 }));
  for (let k = -5; k <= 5; k += 1) out.push(line(vx + k * 5, floor, vx + k * 18, 50, p.light, 0.25, { opacity: 0.18 }));
  // The clock on one side, stopped at midnight.
  const side = rng() < 0.5 ? -1 : 1;
  const clockX = side * between(rng, 28, 34);
  const clockY = between(rng, -2, 6);
  out.push(circle(clockX, clockY, 6, p.dark, { stroke: p.light, "stroke-width": 0.7 }));
  for (let k = 0; k < CLOCK_HOURS; k += 1) {
    const a = (k * FULL_TURN * DEGREES) / CLOCK_HOURS;
    out.push(line(clockX + 4.6 * Math.cos(a), clockY + 4.6 * Math.sin(a), clockX + 5.4 * Math.cos(a), clockY + 5.4 * Math.sin(a), p.light, 0.4));
  }
  out.push(line(clockX, clockY, clockX, clockY - 4.2, p.light, 0.6));
  out.push(line(clockX, clockY, clockX + 0.4, clockY - 3, p.light, 0.9));
  out.push(circle(clockX, clockY, 0.6, p.accent));
  // The one desk, away from the clock, its feet on the floor: its back legs, its top, its front legs, and the paper
  // and pencil on it.
  const desk: string[] = [];
  for (const x of [-11, 9]) desk.push(rect(x, -2, 1.4, 9, p.hull, inked(p)));
  desk.push(poly([[-12, -3], [11, -3], [14, 1], [-9, 1]], p.hull, p.line));
  desk.push(rect(-9, 1, 23, 1.8, mixHex(p.hull, "#000000", 0.35), inked(p)));
  for (const x of [-8.5, 12]) desk.push(rect(x, 2.8, 1.6, 9.2, p.hull, inked(p)));
  desk.push(poly([[-3, -2.4], [4, -2.4], [5.4, 0.4], [-1.6, 0.4]], p.light, p.line));
  for (const y of [-1.6, -0.8]) desk.push(line(-1.4, y, 3.6, y, p.dark, 0.25, { opacity: 0.6 }));
  desk.push(line(6, -1, 10, -2.2, p.accent, 0.7));
  out.push(group(at(between(rng, -8, 8) - side * 4, floor - 8 * EXAM_DESK_SCALE, 0, EXAM_DESK_SCALE), desk.join("")));
  return out.join("");
}

/** Event Horizon: a light caught at the edge of a black hole, the hours round it stopped, starlight bent about it. */
function edgeScene(rng: Rng, p: Paint): string {
  const out: string[] = [];
  const r = between(rng, 13, 16);
  out.push(circle(0, 0, r * 2, p.glow, { opacity: 0.08 }));
  out.push(circle(0, 0, r * 1.45, p.hull, { opacity: 0.14 }));
  // Starlight lensed round it: arcs, never a disk.
  const arcs = whole(rng, 2, 3);
  for (let k = 0; k < arcs; k += 1) {
    const turn = between(rng, 0, FULL_TURN);
    const a = r * between(rng, 1.5, 2.1);
    out.push(group(at(0, 0, turn), path(pathOf(["M", -a, 0, "A", a, a * 0.62, 0, 0, 1, a, 0]), "none", { stroke: p.glow, "stroke-width": 0.5, opacity: 0.45 })));
  }
  // The stopped clock: its hours round the horizon, every third one longer.
  for (let k = 0; k < CLOCK_HOURS; k += 1) {
    const a = (k * FULL_TURN * DEGREES) / CLOCK_HOURS;
    const quarter = k % 3 === 0;
    const reach = r + (quarter ? 8.5 : 7);
    out.push(line((r + 5) * Math.cos(a), (r + 5) * Math.sin(a), reach * Math.cos(a), reach * Math.sin(a), p.light, quarter ? 0.8 : 0.4, { opacity: 0.55 }));
  }
  // The photon ring and the hole.
  out.push(circle(0, 0, r + 1.4, "none", { stroke: p.glow, "stroke-width": 2.8, opacity: 0.55 }));
  out.push(circle(0, 0, r, "#000000"));
  out.push(circle(0, 0, r + 0.4, "none", { stroke: "#ffffff", "stroke-width": 0.6, opacity: 0.9 }));
  // The light: come from far off, bent toward the hole, and held at its edge.
  const caught = between(rng, -150, -30) * DEGREES;
  const hx = (r + 0.6) * Math.cos(caught);
  const hy = (r + 0.6) * Math.sin(caught);
  const sx = (rng() < 0.5 ? -1 : 1) * 52;
  const sy = between(rng, -30, 10);
  const ray = pathOf(["M", sx, sy, "Q", (sx + hx) / 2, Math.min(sy, hy) - r * 0.6, hx, hy]);
  out.push(path(ray, "none", { stroke: p.glow, "stroke-width": 2.2, opacity: 0.4, "stroke-linecap": "round" }));
  out.push(path(ray, "none", { stroke: "#ffffff", "stroke-width": 0.6, opacity: 0.9, "stroke-linecap": "round" }));
  out.push(circle(hx, hy, 3.4, p.glow, { opacity: 0.5 }));
  out.push(twinkle(hx, hy, 4.4, "#ffffff", 1));
  return out.join("");
}

/** The crown's three stones. */
const CROWN_JEWELS: readonly string[] = [VAMP_RED, "#3a7ad0", "#2fbf7a"];

/** Last Word: a hand reaching up to catch a falling crown. */
function crownScene(rng: Rng, p: Paint): string {
  const out: string[] = [];
  // The hand, leaning as it reaches, its sleeve running out of the window's foot: fingers, palm, thumb.
  const hand: string[] = [rect(-8, 18, 16, 40, p.dark, inked(p)), rect(-8.6, 16, 17.2, 4, p.hull, { ...inked(p), rx: 1 })];
  const fingers: readonly (readonly [number, number, number])[] = [[-5.4, 10, -8], [-1.8, 12.5, -2], [1.8, 11.5, 3], [5.2, 8.5, 9]];
  for (const [x, length, turn] of fingers) hand.push(group(at(x, 4, turn), rect(-1.6, -length, 3.2, length + 2, p.light, { ...inked(p), rx: 1.6 })));
  hand.push(path("M-7 17L-7.6 5Q-7.4 2 -4 2L4.4 2Q7.4 2 7.4 5L7 17Z", p.light, inked(p)));
  hand.push(path("M2 3L4.4 2Q7.4 2 7.4 5L7 17L3 17Z", p.hull, { opacity: 0.45 }));
  hand.push(path("M-7.2 12Q-11 9 -12.4 3Q-12.6 1 -11 1.4Q-9.6 2 -8.6 5L-7 8Z", p.light, inked(p)));
  hand.push(path("M-4 11Q0 13 4 11", "none", { stroke: p.hull, "stroke-width": 0.6, opacity: 0.8 }));
  out.push(group(at(between(rng, -6, 6), 4, between(rng, -14, 14)), hand.join("")));
  // The crown above the fingertips, turning as it falls, its fall streaked above it.
  const cx = between(rng, -8, 8);
  const cy = between(rng, -26, -20);
  for (const dx of [-6, 0, 6]) out.push(line(cx + dx, cy - 12 - between(rng, 0, 4), cx + dx, cy - 20 - between(rng, 0, 6), p.glow, 0.5, { opacity: 0.45 }));
  const body = [
    poly([[-10, 0], [-11, -9], [-5.5, -3.5], [0, -11], [5.5, -3.5], [11, -9], [10, 0]], p.accent, p.line),
    rect(-10.4, -0.6, 20.8, 4.6, mixHex(p.accent, "#000000", 0.22), { ...inked(p), rx: 0.8 }),
    ...([[-11, -9], [0, -11], [11, -9]] as const).map(([x, y]) => circle(x, y, 1.3, p.light, inked(p))),
    ...[-6, 0, 6].map((x, index) => circle(x, 1.7, 1.3, CROWN_JEWELS[index] ?? VAMP_RED, inked(p))),
    path("M-8 -1.2L-7.6 -6", "none", { stroke: "#ffffff", "stroke-width": 0.7, opacity: 0.6, "stroke-linecap": "round" }),
  ].join("");
  out.push(circle(cx, cy - 3, 14, p.glow, { opacity: 0.08 }));
  out.push(circle(cx, cy - 3, 9, p.glow, { opacity: 0.12 }));
  out.push(group(at(cx, cy, between(rng, -35, 35)), body));
  out.push(twinkle(cx + 9, cy - 8, 2.6, "#ffffff", 0.9));
  return out.join("");
}

/** The lanes 1v1 Final Destination stands over: one open, the rest locked and broken. */
const DESTINATION_LANES = 5;
const DESTINATION_SHARDS = 3;
const DESTINATION_HERO_SCALE = 1.5;

/** 1v1 Final Destination: one lane of light over an abyss, a hero at each end, the other lanes broken and dark. */
function destinationScene(rng: Rng, p: Paint): string {
  const out: string[] = [];
  // The abyss, going down and down.
  out.push(ellipse(0, 26, 56, 22, "#000000", { opacity: 0.7 }));
  for (let k = 1; k <= 4; k += 1) out.push(ellipse(0, 26 + k * 2, 56 - k * 11, 22 - k * 4.6, "none", { stroke: p.hull, "stroke-width": 0.5, opacity: 0.35 - k * 0.06 }));
  // The lanes: every one but the open one broken into dark pieces.
  const open = whole(rng, 0, DESTINATION_LANES - 1);
  const rows = Array.from({ length: DESTINATION_LANES }, (_, row) => -22 + row * 11 + between(rng, -1, 1));
  rows.forEach((y, row) => {
    if (row === open) return;
    let x = -40;
    while (x < 40) {
      const w = Math.min(between(rng, 6, 14), 40 - x);
      out.push(rect(x, y - 1, w, 2, p.hull, { ...inked(p), opacity: 0.45 }));
      x += w + between(rng, 3, 7);
    }
  });
  // Pieces of them falling into the abyss.
  for (let k = 0; k < DESTINATION_SHARDS; k += 1) {
    out.push(group(at(between(rng, -30, 30), between(rng, 24, 38), between(rng, -60, 60)), rect(-2, -0.8, 4, 1.6, p.hull, { ...inked(p), opacity: 0.4 })));
  }
  // The open lane, lit, and its two heroes, one at each end, facing each other.
  const y = rows[open] ?? 0;
  out.push(rect(-42, y - 3, 84, 6, p.glow, { opacity: 0.22, rx: 3 }));
  out.push(rect(-40, y - 1.3, 80, 2.6, p.glow, { opacity: 0.95, rx: 1.3 }));
  out.push(line(-40, y, 40, y, "#ffffff", 0.6, { opacity: 0.9 }));
  for (const [x, paint] of [[-35, p.light], [35, p.accent]] as const) {
    const hero = [
      circle(0, -4, 4.2, paint, { opacity: 0.22 }),
      poly([[-2.4, -1.4], [-1.3, -7.6], [1.3, -7.6], [2.4, -1.4]], paint, p.line),
      circle(0, -9.4, 1.8, paint, inked(p)),
    ].join("");
    out.push(group(at(x, y - 1.3, 0, DESTINATION_HERO_SCALE), hero));
  }
  return out.join("");
}

/** The Big Crunch: the universe collapsing to a point, its light streaming in and its galaxies drawn after it. */
function crunchScene(rng: Rng, p: Paint): string {
  const out: string[] = [];
  const twist = between(rng, 18, 34) * (rng() < 0.5 ? -1 : 1);
  // The rings it is shrinking through.
  for (const r of [40, 27, 16]) out.push(circle(0, 0, r, "none", { stroke: p.glow, "stroke-width": 0.5, opacity: 0.25, "stroke-dasharray": "2 1.6" }));
  // Galaxies stretched as they fall in.
  const galaxies = whole(rng, 3, 4);
  const start = between(rng, 0, FULL_TURN);
  for (let k = 0; k < galaxies; k += 1) {
    const a = start + (FULL_TURN * k) / galaxies + between(rng, -20, 20);
    const reach = between(rng, 22, 34);
    out.push(group(at(reach * Math.cos(a * DEGREES), reach * Math.sin(a * DEGREES), a + twist), ellipse(0, 0, 7, 2.2, p.accent, { opacity: 0.35 }) + ellipse(0, 0, 3.4, 1.1, p.light, { opacity: 0.7 })));
  }
  // The light streaming in, curving as it goes.
  const streaks = whole(rng, 22, 30);
  const colors = [p.glow, p.accent, p.light];
  for (let k = 0; k < streaks; k += 1) {
    const a = (FULL_TURN * k) / streaks + between(rng, -5, 5);
    const outer = between(rng, 26, 50);
    const inner = between(rng, 5, 10);
    const w = between(rng, 0.5, 1.3);
    const from = a * DEGREES;
    const to = (a + twist) * DEGREES;
    const bend = (a + twist * 0.4) * DEGREES;
    const mid = (outer + inner) / 2;
    const ox = outer * Math.cos(from);
    const oy = outer * Math.sin(from);
    const nx = -Math.sin(from) * w;
    const ny = Math.cos(from) * w;
    const qx = mid * Math.cos(bend);
    const qy = mid * Math.sin(bend);
    const streak = pathOf(["M", ox + nx, oy + ny, "Q", qx, qy, inner * Math.cos(to), inner * Math.sin(to), "Q", qx, qy, ox - nx, oy - ny, "Z"]);
    out.push(path(streak, colors[k % colors.length] ?? p.glow, { opacity: between(rng, 0.45, 0.85) }));
  }
  // The point it all falls into.
  out.push(circle(0, 0, 9, p.glow, { opacity: 0.35 }));
  out.push(circle(0, 0, 4.2, p.light, { opacity: 0.9 }));
  out.push(twinkle(0, 0, 11, "#ffffff", 0.95));
  out.push(circle(0, 0, 1.6, "#ffffff"));
  return out.join("");
}

const VELVET = "#2a0612";
const VELVET_SHEEN = "#7a2034";

/** The die's 67, a "6" and a "7" stroked about (x, y), `u` half their height: no text, so each digit is a path. */
function sixtySeven(x: number, y: number, u: number): string {
  const six = x - u * 0.62;
  const seven = x + u * 0.62;
  const r = u * 0.48;
  return pathOf([
    "M", six + u * 0.4, y - u, "Q", six - u * 0.5, y - u * 0.75, six - r, y + u * 0.45,
    "A", r, r, 0, 1, 0, six + r, y + u * 0.45, "A", r, r, 0, 1, 0, six - r, y + u * 0.45,
    "M", seven - u * 0.5, y - u, "L", seven + u * 0.5, y - u, "L", seven - u * 0.15, y + u,
  ]);
}

/** Dice of Doom: a hundred-sided die on black velvet, 67 glowing faintly on the face it shows. */
function d100Scene(rng: Rng, p: Paint): string {
  const out: string[] = [];
  // The velvet, draped in folds, and the sheen along them.
  const lift = between(rng, 10, 16);
  const folds = whole(rng, 3, 5);
  let drape = pathOf(["M", -60, lift + 4]);
  for (let k = 0; k < folds; k += 1) {
    const x0 = -60 + (120 * k) / folds;
    const x1 = -60 + (120 * (k + 1)) / folds;
    drape += pathOf(["Q", (x0 + x1) / 2, lift + between(rng, -5, 3), x1, lift + between(rng, 1, 5)]);
  }
  out.push(path(`${drape}L60 60L-60 60Z`, VELVET));
  for (let k = 0; k < folds; k += 1) {
    const x = -60 + (120 * (k + 0.5)) / folds + between(rng, -6, 6);
    out.push(path(pathOf(["M", x, lift + 2, "Q", x + between(rng, -8, 8), lift + 20, x + between(rng, -6, 6), 50]), "none", {
      stroke: VELVET_SHEEN,
      "stroke-width": between(rng, 1.4, 3),
      opacity: 0.35,
      "stroke-linecap": "round",
    }));
  }
  // The die resting on it, its shadow under it, and the facets over it turned as it landed.
  const r = between(rng, 17, 20);
  const dx = between(rng, -6, 6);
  const dy = lift + 2 - r;
  out.push(ellipse(dx, lift + 3, r * 0.9, 3.2, "#000000", { opacity: 0.6 }));
  out.push(circle(dx, dy, r, "url(#dr)", inked(p)));
  const seam = { stroke: p.dark, "stroke-width": 0.4, opacity: 0.45 };
  const facets: string[] = [];
  for (let k = -2; k <= 2; k += 1) {
    const ly = k * r * 0.34;
    const lx = r * Math.sqrt(Math.max(0, 1 - (ly / r) ** 2));
    facets.push(path(pathOf(["M", -lx, ly, "A", lx, lx * 0.3, 0, 0, 0, lx, ly]), "none", seam));
  }
  for (const share of [0.38, 0.72]) {
    for (const sweep of [0, 1]) facets.push(path(pathOf(["M", 0, -r, "A", r * share, r, 0, 0, sweep, 0, r]), "none", seam));
  }
  facets.push(line(0, -r, 0, r, p.dark, 0.4, { opacity: 0.45 }));
  // The face it shows, and the 67 on it, glowing faintly.
  const fx = between(rng, -3, 3);
  const fy = between(rng, -3, 3);
  facets.push(poly([[fx - 5, fy - 4.4], [fx + 5, fy - 4.4], [fx + 6, fy + 0.4], [fx, fy + 5.6], [fx - 6, fy + 0.4]], p.light, p.dark, { opacity: 0.92 }));
  facets.push(circle(fx, fy, 6, p.glow, { opacity: 0.2 }));
  const digits = sixtySeven(fx, fy, 2.6);
  facets.push(path(digits, "none", { stroke: p.glow, "stroke-width": 2.2, opacity: 0.35, "stroke-linecap": "round", "stroke-linejoin": "round" }));
  facets.push(path(digits, "none", { stroke: p.glow, "stroke-width": 0.9, opacity: 0.95, "stroke-linecap": "round", "stroke-linejoin": "round" }));
  out.push(group(at(dx, dy, between(rng, -25, 25)), facets.join("")));
  out.push(circle(dx - r * 0.4, dy - r * 0.45, r * 0.18, "#ffffff", { opacity: 0.45 }));
  out.push(twinkle(dx - r * 0.42, dy - r * 0.5, 2.4, "#ffffff", 0.8));
  return out.join("");
}

/** R3338001: each Terminus picture's painter, by its kind (`TERMINUS_KINDS`). */
const TERMINUS_SCENES: Readonly<Record<string, (rng: Rng, p: Paint) => string>> = {
  expanse: expanseScene,
  lamp: lampScene,
  exam: examScene,
  edge: edgeScene,
  crown: crownScene,
  destination: destinationScene,
  crunch: crunchScene,
  d100: d100Scene,
};

/** A Terminus card's picture: its own, else the shared black hole. */
function terminusScene(rng: Rng, p: Paint, kind: string): string {
  const scene = Object.hasOwn(TERMINUS_SCENES, kind) ? TERMINUS_SCENES[kind] : undefined;
  return scene === undefined ? terminus(rng, p) : scene(rng, p);
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

/**
 * R3142002: the Drone Commander, an officer: the crew figure with epaulettes and a medal, and a drone hovering
 * behind her shoulder, a lens on its glowing eye.
 */
function commander(rng: Rng, p: Paint): string {
  const out: string[] = [];
  out.push(group(at(24, -18, 12, 0.6), droneBody(p, 11, 2)));
  out.push(line(24, -9, 20, 8, p.glow, 0.5, { opacity: 0.45, "stroke-dasharray": "1.5 1.2" }));
  out.push(pilot(rng, p, "commander", false));
  for (const side of [-1, 1]) {
    out.push(rect(side * 25 - 5, 14, 10, 4, p.accent, { ...inked(p), rx: 1, transform: `rotate(${fmt(side * 14)} ${fmt(side * 25)} 16)` }));
    out.push(line(side * 25 - 3, 18, side * 25 - 3, 21, p.accent, 0.5));
    out.push(line(side * 25, 18, side * 25, 21, p.accent, 0.5));
    out.push(line(side * 25 + 3, 18, side * 25 + 3, 21, p.accent, 0.5));
  }
  out.push(circle(-10, 31, 2.2, p.glow, inked(p)));
  out.push(path("M-10 28.8L-10 26.5", "none", { stroke: p.accent, "stroke-width": 0.8 }));
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

function subjectMarkup(rng: Rng, p: Paint, subject: StinkySubject, lower: string, name: string | undefined, radiant: boolean, ship: ShipArt | null): string {
  const { family, kind, tier } = subject;
  switch (family) {
    case "fleet":
      if (kind === "golem") return golem(rng, p);
      return ["fighter", "cruiser", "battleship", "dormant"].includes(kind) ? hull(rng, p, kind) : piece(rng, p, kind);
    case "weapon":
      return weapon(rng, p, kind, tier, lower);
    case "jlockeed":
      return kind === "blueprint" ? hull(rng, p, "battleship") : weapon(rng, p, kind, tier, lower);
    case "drone":
      return drone(p, kind, tier, radiant, ship);
    case "miner":
      return miner(rng, p, kind, tier);
    case "resource":
      return resource(rng, p, kind, lower);
    case "anomaly":
      return anomaly(rng, p, kind);
    case "doctrine":
      return doctrine(p, kind);
    case "terminus":
      return terminusScene(rng, p, kind);
    case "swarm":
      return swarm(rng, p, kind);
    case "acclaimed":
      return pilot(rng, p, kind, true);
    case "crew":
      return kind === "commander" ? commander(rng, p) : pilot(rng, p, kind, false);
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

/** The document every Stinkyscape face prints: its sky, its rays when Radiant, its subject, its particles and vignette. */
function stinkyDocument(defs: Defs, sky: string, subject: string, radiant: boolean, seed: number, motion: ArtMotion | null, seedId: string, part: ArtPart): string {
  // R2943001: a part other than "whole" draws the backdrop with no subject, or the subject on nothing. R3344000: a depth
  // plane draws its own: the sky and rays far, the subject near, the particles nearest, the vignette far, the rest none.
  const behindSubject = partHolds(part, "far");
  const figure = partDraws(part, "subject");
  return (
    `<svg viewBox="0 0 ${BOX} ${BOX}" width="${BOX}" height="${BOX}" xmlns="http://www.w3.org/2000/svg" preserveAspectRatio="xMidYMid slice">` +
    `<defs>${defs.join("")}</defs>` +
    motionStyle(motion, CENTER_ORIGIN) +
    (behindSubject ? moving("back", sky, motion) + (radiant ? rays(seed) : "") : "") +
    (figure ? moving("subject", subject, motion) : "") +
    (partHolds(part, "nearest") ? particleMarkup(motion, seedId) : "") +
    (behindSubject ? rect(0, 0, BOX, BOX, "url(#v)") : "") +
    "</svg>"
  );
}

/**
 * The SVG document a Stinkyscape face draws, unencoded; R2715001: moving as `motion` says. R2949000: its subject is
 * `defId`'s, and the rest of the picture grows from `seedId` (pictures.ts `artSeedOf`), the card's id by default.
 * R2943001: `part` "backdrop" leaves its subject out, and "subject" draws the subject alone (pictures.ts `ArtPart`).
 * R3333001: a Starship Piece's card draws its module instead, in its bay (`pieceCardSvg`).
 */
export function stinkyscapeSvg(
  defId: string,
  name: string | undefined,
  tags: readonly Tag[],
  type: CardType,
  radiant: boolean,
  motion: ArtMotion | null = null,
  seedId: string = defId,
  part: ArtPart = "whole",
  ship: ShipArt | null = null,
): string {
  if (STARSHIP_MODULE_IDS[defId] !== undefined) return pieceCardSvg(starshipModuleOf(defId), radiant, motion, seedId, part);
  const subject = stinkySubject(defId, name, tags, type);
  const lower = (name ?? "").toLowerCase();
  const seed = hashId(seedId);
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
  const drawn = subjectMarkup(ahead, p, subject, lower, name, radiant, ship);
  const placed = group(at(SUBJECT_X + (SUBJECT_SHIFTS[subject.family] ?? 0), SUBJECT_Y, tilt, scale), drawn);
  return stinkyDocument(defs, sky, placed, radiant, seed, motion, seedId, part);
}

// R3333001: a Starship Piece's own card, its module drawn large on a cutaway of a hull, in its group's bay

/** Each bay's paint: its wall, floor and props, its light strips and the stripe it is marked with. None is gold. */
const HANGAR_PAINTS: Readonly<Record<Hangar, Paint>> = {
  range: { space: ["#2a1a1e", "#0c0608"], nebula: ["#d0482a", "#7a2a8f"], light: "#c9ced6", hull: "#4a4f58", dark: "#16181d", accent: "#d8443a", glow: "#ff8a3a", line: OUTLINE },
  orebay: { space: ["#2a221c", "#0a0806"], nebula: ["#c8743a", "#4d6b2a"], light: "#b9ab98", hull: "#5a4f44", dark: "#1a1612", accent: "#ff7a1a", glow: "#4fa8ff", line: OUTLINE },
  shieldlab: { space: ["#0c2a3a", "#03090e"], nebula: ["#1fb5a8", "#2a4fb8"], light: "#c8e8f4", hull: "#3a5a6a", dark: "#0e1c24", accent: "#3ab0ff", glow: "#5ad8ff", line: OUTLINE },
  teststand: { space: ["#2a1410", "#0a0504"], nebula: ["#d0482a", "#3a1a6a"], light: "#c9ced6", hull: "#4a4440", dark: "#14100e", accent: "#ff4a3a", glow: "#ff7a2a", line: OUTLINE },
  loading: { space: ["#1a2430", "#06090c"], nebula: ["#3a6ad8", "#a03ad8"], light: "#d9e1ea", hull: "#4a5a6a", dark: "#121820", accent: "#d8443a", glow: "#ffa04a", line: OUTLINE },
  medbay: { space: ["#24403e", "#081212"], nebula: ["#3ad0a0", "#3a6ad8"], light: "#e8f4f2", hull: "#4a6a68", dark: "#0e1e1c", accent: "#ff6a9a", glow: "#7af0d0", line: OUTLINE },
  hangar: { space: ["#22222e", "#08080c"], nebula: ["#5a3ad8", "#d83a8a"], light: "#d9e1ea", hull: "#4a4a5a", dark: "#121218", accent: "#b48aff", glow: "#bfe4ff", line: OUTLINE },
  blueprint: FAMILY_PAINTS.jlockeed,
};

/** How large a piece's card draws its module, and how far its bay's floor and window reach. */
const MODULE_CARD_SCALE = 2.8;
/** How far a card's subject sits from its module's middle toward its stand, so the two share the window's middle. */
const CARD_STAND_SHIFT: Readonly<Record<StarshipModule["mount"], readonly [number, number]>> = {
  bow: [0, -2],
  spine: [0, -2],
  keel: [0, 2],
  stern: [-2, 0],
  flank: [0, 0],
};
const BAY_FLOOR_Y = 72;
const BAY_STRIPS_MIN = 2;
const BAY_STRIPS_MAX = 4;
const BAY_STARS_MIN = 10;
const BAY_STARS_MAX = 18;
const BAY_TARGET_WHITE = "#e8e8f0";
const BAY_SPACE = "#04060f";

/** A bay's prop, by its kind: a range's target, an ore bay's piles, a lab's hex grid, a test stand's baffles and more. */
function bayProps(rng: Rng, bay: Paint, hangar: Hangar): string {
  const out: string[] = [];
  switch (hangar) {
    case "range": {
      const y = between(rng, 44, 56);
      for (const [r, fill] of [[7, bay.accent], [5, BAY_TARGET_WHITE], [3, bay.accent], [1.2, BAY_TARGET_WHITE]] as const) out.push(circle(88, y, r, fill, { opacity: 0.85 }));
      out.push(line(88, y + 7, 88, BAY_FLOOR_Y, bay.hull, 1));
      break;
    }
    case "orebay":
      for (const [x, w] of [[2, 22], [76, 22]] as const) {
        out.push(poly([[x, 90], [x + w * 0.3, 78], [x + w * 0.7, 76], [x + w, 90]], bay.hull, bay.line));
        for (let i = 0; i < 4; i += 1) out.push(circle(x + between(rng, 4, w - 4), between(rng, 80, 88), between(rng, 0.8, 1.6), bay.glow, { opacity: 0.85 }));
      }
      break;
    case "shieldlab": {
      let d = "";
      for (let row = 0; row < 3; row += 1) {
        for (let col = 0; col < 7; col += 1) {
          const cx = 8 + col * 14 + (row % 2) * 7;
          const cy = 40 + row * 11;
          d += `M${fmt(cx - 6)} ${fmt(cy)}L${fmt(cx - 3)} ${fmt(cy - 5.2)}L${fmt(cx + 3)} ${fmt(cy - 5.2)}L${fmt(cx + 6)} ${fmt(cy)}L${fmt(cx + 3)} ${fmt(cy + 5.2)}L${fmt(cx - 3)} ${fmt(cy + 5.2)}Z`;
        }
      }
      out.push(path(d, "none", { stroke: bay.glow, "stroke-width": 0.4, opacity: 0.3 }));
      break;
    }
    case "teststand":
      for (const x of [2, 12]) out.push(poly([[x, 40], [x + 8, 36], [x + 8, BAY_FLOOR_Y], [x, BAY_FLOOR_Y]], bay.hull, bay.line));
      out.push(ellipse(16, 84, 18, 4, bay.dark, { opacity: 0.8 }));
      break;
    case "loading":
      for (const [x, y, s] of [[3, 58, 14], [17, 64, 8], [82, 60, 12]] as const) {
        out.push(rect(x, y, s, BAY_FLOOR_Y - y, bay.hull, inked(bay)), line(x, y, x + s, BAY_FLOOR_Y, bay.dark, 0.6), line(x + s, y, x, BAY_FLOOR_Y, bay.dark, 0.6));
      }
      break;
    case "medbay":
      out.push(rect(80, 38, 14, 14, BAY_TARGET_WHITE, { ...inked(bay), rx: 1.5 }), rect(85.6, 40.4, 2.8, 9.2, bay.accent), rect(82.4, 43.6, 9.2, 2.8, bay.accent));
      for (const y of [40, 50, 60]) out.push(rect(4, y, 12, 7, bay.hull, inked(bay)), circle(13, y + 2, 0.8, bay.glow));
      break;
    case "hangar": {
      const x = between(rng, 62, 80);
      out.push(rect(0, 10, BOX, 2.4, bay.hull, inked(bay)), rect(x - 3, 12.4, 6, 3, bay.dark, inked(bay)), line(x, 15.4, x, 28, bay.light, 0.4));
      out.push(path(`M${fmt(x)} 28q2 1 0.8 3`, "none", { stroke: bay.light, "stroke-width": 0.8 }));
      break;
    }
    case "blueprint":
      break;
  }
  return out.join("");
}

/** A bay a piece's card stands in: its wall and a window on space, light strips above, a floor, and its props. */
function hangarBackdrop(rng: Rng, bay: Paint, hangar: Hangar, defs: Defs): string {
  defs.push(
    `<linearGradient id="sp" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${bay.space[0]}"/>` +
      `<stop offset="1" stop-color="${bay.space[1]}"/></linearGradient>`,
  );
  const out: string[] = [rect(0, 0, BOX, BOX, "url(#sp)")];
  const wx = between(rng, 12, 30);
  const ww = between(rng, 40, 56);
  out.push(rect(wx, 8, ww, 22, BAY_SPACE, { stroke: bay.light, "stroke-width": 1.2, rx: 2 }));
  const stars = whole(rng, BAY_STARS_MIN, BAY_STARS_MAX);
  for (let i = 0; i < stars; i += 1) out.push(circle(between(rng, wx + 1, wx + ww - 1), between(rng, 9, 29), between(rng, 0.15, 0.5), "#ffffff", { opacity: between(rng, 0.4, 1) }));
  for (const share of [1 / 3, 2 / 3]) out.push(line(wx + ww * share, 8, wx + ww * share, 30, bay.light, 0.6));
  const strips = whole(rng, BAY_STRIPS_MIN, BAY_STRIPS_MAX);
  for (let i = 0; i < strips; i += 1) {
    const x = ((i + 0.5) * BOX) / strips;
    out.push(rect(x - 7, 1, 14, 3, bay.glow, { opacity: 0.18 }), rect(x - 5, 1.6, 10, 1.4, bay.glow, { opacity: 0.85 }));
  }
  out.push(rect(0, BAY_FLOOR_Y, BOX, BOX - BAY_FLOOR_Y, bay.dark));
  for (let i = -4; i <= 4; i += 1) out.push(line(HALF + i * 9, BAY_FLOOR_Y, HALF + i * 24, BOX, bay.hull, 0.4, { opacity: 0.6 }));
  for (const y of [78, 87]) out.push(line(0, y, BOX, y, bay.hull, 0.3, { opacity: 0.5 }));
  out.push(line(0, BAY_FLOOR_Y, BOX, BAY_FLOOR_Y, bay.light, 0.5, { opacity: 0.6 }));
  out.push(bayProps(rng, bay, hangar));
  return out.join("");
}

/** A cut section of a hull's plating under a deck (y from 0 down), its ribs bare: the bay's stand for a module. */
function deckSection(p: Paint, bay: Paint): string {
  const out = [
    poly([[-12, 1.6], [12, 1.6], [11.2, 3], [12.4, 4.4], [11, 5.8], [-11.2, 5.8], [-12.4, 4.6], [-11, 3.2]], p.dark, p.line),
    poly([[-13, 0], [13, 0], [12, 1.6], [-12, 1.6]], p.light, p.line),
    rect(-13, 0.3, 26, 0.5, bay.accent),
  ];
  for (const x of [-8, -4, 0, 4, 8]) out.push(line(x, 1.6, x, 5.6, p.hull, 0.8));
  out.push(line(-11.4, 3.7, 11.4, 3.7, p.hull, 0.5));
  return out.join("");
}

/** A cut section of a hull's side behind a flank module, its ribs bare at the cut ends. */
function flankSection(p: Paint, bay: Paint): string {
  const out = [
    path("M-13 -6Q0 -8 13 -6L12 -3L13.4 0L12 3L13 6Q0 8 -13 6L-12 3L-13.4 0L-12 -3Z", p.dark, inked(p)),
    path("M-13 -6Q0 -8 13 -6L12.6 -4.6Q0 -6.6 -12.6 -4.6Z", p.light),
    rect(-12.4, 4, 24.8, 0.6, bay.accent),
  ];
  for (const x of [-8, 8]) out.push(line(x, -6.4, x, 6.4, p.hull, 0.7));
  return out.join("");
}

/** A Starship Piece's own card's subject: its module large on a cutaway of the hull where it mounts. */
function moduleCardMarkup(m: StarshipModule, p: Paint, bay: Paint): string {
  const [cx, cy] = moduleCenter(m);
  const section = deckSection(p, bay);
  const stand =
    m.mount === "keel"
      ? group("scale(1 -1)", section)
      : m.mount === "stern"
        ? group("rotate(-90)", section)
        : m.mount === "flank"
          ? flankSection(p, bay)
          : section;
  // The module's middle and its stand's share the window's middle.
  const [sx, sy] = CARD_STAND_SHIFT[m.mount];
  return group(at((sx - cx) * MODULE_CARD_SCALE, (sy - cy) * MODULE_CARD_SCALE, 0, MODULE_CARD_SCALE), stand + moduleMarkup(m, p));
}

/** R3333001: a Starship Piece's own card: its module on a cutaway of a hull in its group's bay, tilted a little. */
function pieceCardSvg(m: StarshipModule, radiant: boolean, motion: ArtMotion | null, seedId: string, part: ArtPart): string {
  const seed = hashId(seedId);
  const base = modulePaint(m);
  const p = radiant ? gild(base) : base;
  const bay = radiant ? gild(HANGAR_PAINTS[m.hangar]) : HANGAR_PAINTS[m.hangar];
  const behind = seededRandom((seed ^ BACKDROP_SALT) >>> 0);
  const ahead = seededRandom((seed ^ SUBJECT_SALT) >>> 0);
  const tilt = between(ahead, -SUBJECT_TILT_JITTER, SUBJECT_TILT_JITTER);
  const defs: Defs = [];
  gradients(p, defs);
  const sky = m.hangar === "blueprint" ? backdrop(behind, bay, "jlockeed", false, defs) : hangarBackdrop(behind, bay, m.hangar, defs);
  const placed = group(at(SUBJECT_X, SUBJECT_Y, tilt), moduleCardMarkup(m, p, bay));
  return stinkyDocument(defs, sky, placed, radiant, seed, motion, seedId, part);
}

// R2249000, R3333000: a Starship's own art, drawn from what it holds

/** The seed a Starship's sky grows from: its design's (its `sky`, else its id), so a ship keeps one sky whatever fuses into it. */
const STARSHIP_SEED_PREFIX = "starship-";
const STARSHIP_ESCORTS_MAX = 3;
/** A launched Starship is escorted by one frigate for every this many pieces it holds, up to the most. */
const STARSHIP_PIECES_PER_ESCORT = 3;
const STARSHIP_ESCORT_SCALE = 0.25;
/** Where its escorts fly, as shares of its half length and gaps above or below its hull. */
const STARSHIP_ESCORTS: readonly (readonly [number, number])[] = [[0.35, -22], [-0.35, 20], [0.8, 17]];
const STARSHIP_STREAKS = 14;
/** A launched Starship climbs; a Dormant one sits level in its yard. */
const STARSHIP_TILT_LAUNCHED = -8;
/** A barbette under a raised module is this share of its module's box wide. */
const BARBETTE_WIDTH = 0.6;
/** The most a Starship's picture may weigh, unencoded, however many pieces it holds: two may share a board. */
export const STARSHIP_ART_MAX_BYTES = 49_152;
const ICON_PLATE = "#3f4964";
const ICON_RIVET = "#9aa1b8";
/** How large a module's icon draws it on its plate. */
const MODULE_ICON_SCALE = 0.75;

function streak(rng: Rng, color: string): string {
  const y = between(rng, 6, 94);
  const x = between(rng, 40, 100);
  const length = between(rng, 18, 46);
  return line(x, y, x - length, y + between(rng, -2, 2), color, between(rng, 0.25, 0.7), {
    opacity: between(rng, 0.25, 0.6),
  });
}

function gildHull(p: HullPaint): HullPaint {
  return { ...gild(p), trim: mixHex(p.trim, RADIANT_PALETTE.mote, GILD_MIX), window: mixHex(p.window, RADIANT_PALETTE.glow, GILD_MIX) };
}

/** The white-hot core of a Starship's burn, and how far its warp streaks pale its glow, with no trail of its own. */
const BURN_CORE = "#ffffff";
const STREAK_PALE = 0.5;

/**
 * R3335000: a launched Starship's engine colours, its burn, the burn's core and its warp streaks: its trail's, gilded
 * as its glow is on a Radiant face, or, with none, its livery's glow, a white core and streaks paler than its glow.
 */
function burnPaint(trail: StarshipTrail | null, p: HullPaint, radiant: boolean): StarshipTrail {
  if (trail === null) return { burn: p.glow, core: BURN_CORE, streak: mixHex(p.glow, BURN_CORE, STREAK_PALE) };
  if (!radiant) return trail;
  return { burn: mixHex(trail.burn, RADIANT_PALETTE.glow, GILD_MIX), core: trail.core, streak: mixHex(trail.streak, RADIANT_PALETTE.glow, GILD_MIX) };
}

/**
 * R3333000: the art a Starship wears, Dormant or launched, drawn from its inputs (`StarshipArt`): its class from the
 * count of its pieces, each piece's module (gilded when the piece or the face is Radiant) on its hardpoint oldest
 * first, its design's hull in its livery, and the shipyard about a Dormant one or, launched, its engines' burn, warp
 * streaks and escorts. Never its definition's id or name, which a fusion changes (R2249000). R3335000: a launched one's
 * burn and streaks in its trail's colours and its modules in its piece skin's tones, gilded after, when it has them.
 */
export function starshipSvg(ship: StarshipArt): string {
  const design = STARSHIP_DESIGNS[ship.design];
  const seed = hashId(`${STARSHIP_SEED_PREFIX}${design.sky ?? design.id}`);
  const base = liveryPaint(ship.livery ?? design.livery);
  const p = ship.radiant ? gildHull(base) : base;
  const behind = seededRandom((seed ^ BACKDROP_SALT) >>> 0);
  const defs: Defs = [];
  gradients(p, defs);
  const sky = backdrop(behind, p, "fleet", true, defs);
  const cls = starshipClassOf(ship.pieces.length);
  const L = cls.length;
  const H = cls.height;
  const shown = ship.pieces.slice(0, STARSHIP_MODULES_MAX);
  const modules = shown.map((piece) => starshipModuleOf(piece.defId));
  const places = hardpointPlacements(cls, modules.map((m) => m.mount));
  const skin = ship.skin ?? null;
  const skinned = (m: StarshipModule): Paint => (skin === null ? modulePaint(m) : { ...modulePaint(m), ...skin });
  const paints = modules.map((m, i) => (shown[i]?.radiant === true || ship.radiant ? gild(skinned(m)) : skinned(m)));
  const engines = burnPaint(ship.trail ?? null, p, ship.radiant);
  const mounted = (keep: (m: StarshipModule) => boolean): string =>
    modules
      .map((m, i) => {
        const place = places[i];
        const paint = paints[i];
        if (place === undefined || paint === undefined || !keep(m)) return "";
        const wide = BARBETTE_WIDTH * MODULE_BOX * place.scale;
        const barbette = place.lift > 0 ? rect(place.x - wide / 2, place.y, wide, place.lift, p.hull, inked(p)) : "";
        return barbette + group(at(place.x, place.y, 0, place.scale), moduleMarkup(m, paint));
      })
      .join("");
  const body: string[] = [];
  if (ship.launched) {
    body.push(poly([[-L, -0.45 * H], [-L - 10 - 0.3 * L, 0], [-L, 0.45 * H]], engines.burn, "none", { opacity: 0.7 }));
    body.push(poly([[-L, -0.2 * H], [-L - 6 - 0.15 * L, 0], [-L, 0.2 * H]], engines.core, "none", { opacity: 0.8 }));
  }
  body.push(mounted((m) => m.mount === "stern"));
  body.push(hullMarkup(design, cls, p, ship.launched));
  body.push(ledgerMarkup(cls, paints.map((paint) => [paint.accent, paint.glow] as const), ship.launched, p));
  body.push(mounted((m) => m.mount !== "stern"));
  if (ship.launched) {
    const escorts = Math.min(STARSHIP_ESCORTS_MAX, Math.ceil(ship.pieces.length / STARSHIP_PIECES_PER_ESCORT));
    STARSHIP_ESCORTS.slice(0, escorts).forEach(([x, gap]) => {
      body.push(group(at(x * L, gap < 0 ? -H + gap : H + gap, 0, STARSHIP_ESCORT_SCALE), escortMarkup(design, p)));
    });
  } else {
    const last = places[places.length - 1];
    body.push(shipyardMarkup(cls, last === undefined ? null : { x: last.x, y: last.y }, ship.pieces.length, p));
  }
  const streaks = ship.launched ? Array.from({ length: STARSHIP_STREAKS }, () => streak(behind, engines.streak)).join("") : "";
  return (
    `<svg viewBox="0 0 ${BOX} ${BOX}" width="${BOX}" height="${BOX}" xmlns="http://www.w3.org/2000/svg" preserveAspectRatio="xMidYMid slice">` +
    `<defs>${defs.join("")}</defs>` +
    sky +
    streaks +
    (ship.radiant ? rays(seed) : "") +
    group(at(SUBJECT_X, SUBJECT_Y, ship.launched ? STARSHIP_TILT_LAUNCHED : 0, cls.zoom), body.join("")) +
    rect(0, 0, BOX, BOX, "url(#v)") +
    "</svg>"
  );
}

// R3337001: an Anomaly Found!'s own art, the board its code names (anomalies.ts)

/** R3337001: what a board's art grows from, before its code, so a board keeps one look on every card that holds it. */
export const ANOMALY_SEED_PREFIX = "anomaly-";

/**
 * R3337001: the family whose paint a board wears, the kind (`KIND_COLORS`) that colors its ore or its plumes, and
 * whether a planet may hang in its sky.
 */
type AnomalyLook = { family: StinkyFamily; kind: string | null; planet: boolean };

/** R3337001: each board's look: the wormhole's own violet, a battle's red, the asteroid fields' rock, a drone's teal. */
const ANOMALY_LOOKS: { readonly [C in AnomalyCode]: AnomalyLook } = {
  WH: { family: "anomaly", kind: null, planet: false },
  BF: { family: "weapon", kind: null, planet: true },
  AL: { family: "resource", kind: null, planet: true },
  AT: { family: "resource", kind: null, planet: false },
  SP: { family: "resource", kind: "spice", planet: true },
  MT: { family: "terminus", kind: null, planet: false },
  DH: { family: "drone", kind: null, planet: true },
  FO: { family: "fleet", kind: null, planet: true },
  AN: { family: "resource", kind: "red-narcor", planet: true },
  AA: { family: "resource", kind: "axnit", planet: true },
};

/** R3337001: a board's faintest layer, a haze of light. */
const ANOMALY_HAZE = 0.16;
/** R3337001: a board's shaded layer. */
const ANOMALY_SHADE = 0.35;
/** R3337001: a board's wash of light. */
const ANOMALY_WASH = 0.6;
/** R3337001: a board's part lit in full. */
const ANOMALY_LIT = 0.92;

/** WH: how many rings the Wormhole's tunnel folds through, mouth to throat. */
const WORMHOLE_RINGS = 9;
/** WH: its mouth's half-width and half-height. */
const WORMHOLE_MOUTH: readonly [number, number] = [40, 26];
/** WH: the share of the mouth's size its throat loses. */
const WORMHOLE_NARROWING = 0.9;
/** WH: where its throat opens, off the mouth's middle. */
const WORMHOLE_THROAT: readonly [number, number] = [10, -6];
/** WH: the streaks of light running down its wall. */
const WORMHOLE_STREAKS = 16;
/** WH: the stroke its outermost ring takes beyond the set's ink line. */
const WORMHOLE_STROKE = 1.6;
/** WH: the radius of the light at its end. */
const WORMHOLE_LIGHT = 4.5;

/** WH: rings folding into a tunnel of light, each narrower and deeper than the last. */
function wormholeBoard(p: Paint): string {
  const [rx, ry] = WORMHOLE_MOUTH;
  const [tx, ty] = WORMHOLE_THROAT;
  const out: string[] = [ellipse(0, 0, rx, ry, p.glow, { opacity: ANOMALY_HAZE })];
  for (let i = 0; i < WORMHOLE_STREAKS; i += 1) {
    const a = (i * FULL_TURN * DEGREES) / WORMHOLE_STREAKS;
    out.push(line(rx * Math.cos(a), ry * Math.sin(a), tx, ty, p.light, STROKE / 2, { opacity: ANOMALY_SHADE }));
  }
  for (let i = 0; i < WORMHOLE_RINGS; i += 1) {
    const t = i / (WORMHOLE_RINGS - 1);
    const left = 1 - t * WORMHOLE_NARROWING;
    out.push(
      ellipse(tx * t, ty * t, rx * left, ry * left, p.dark, {
        "fill-opacity": ANOMALY_HAZE,
        stroke: i % 2 === 0 ? p.glow : p.accent,
        "stroke-width": STROKE + WORMHOLE_STROKE * left,
        "stroke-opacity": ANOMALY_LIT - t * ANOMALY_SHADE,
      }),
    );
  }
  out.push(circle(tx, ty, WORMHOLE_LIGHT * 2, p.glow, { opacity: ANOMALY_WASH }));
  out.push(twinkle(tx, ty, WORMHOLE_LIGHT * 2, "#ffffff", ANOMALY_LIT));
  out.push(circle(tx, ty, WORMHOLE_LIGHT, "#ffffff", { opacity: ANOMALY_LIT }));
  return out.join("");
}

/** R3337001: how far a broken thing's clip reaches past it, every way, so its halves cover all of it. */
const BREAK_REACH = 70;
/** R3337001: the clip paths a broken thing's halves draw through: the part that stays, the part that drifts off. */
const BREAK_KEEP_ID = "wk";
const BREAK_CHUNK_ID = "wc";

/**
 * R3337001: `drawn` broken along `edge` (a run of points, top to bottom, in its own frame), the part right of the edge
 * drifted off by `drift`: across, down and turned.
 */
function broken(drawn: string, edge: readonly (readonly [number, number])[], drift: readonly [number, number, number], defs: Defs): string {
  const [topX] = edge[0] ?? [0, 0];
  const [bottomX] = edge[edge.length - 1] ?? [0, 0];
  const keep: (readonly [number, number])[] = [[-BREAK_REACH, -BREAK_REACH], [topX, -BREAK_REACH], ...edge, [bottomX, BREAK_REACH], [-BREAK_REACH, BREAK_REACH]];
  const chunk: (readonly [number, number])[] = [[topX, -BREAK_REACH], ...edge, [bottomX, BREAK_REACH], [BREAK_REACH, BREAK_REACH], [BREAK_REACH, -BREAK_REACH]];
  defs.push(
    `<clipPath id="${BREAK_KEEP_ID}"><polygon points="${points(keep)}"/></clipPath>`,
    `<clipPath id="${BREAK_CHUNK_ID}"><polygon points="${points(chunk)}"/></clipPath>`,
  );
  const [dx, dy, turn] = drift;
  return (
    `<g clip-path="url(#${BREAK_KEEP_ID})">${drawn}</g>` +
    group(at(dx, dy, turn), `<g clip-path="url(#${BREAK_CHUNK_ID})">${drawn}</g>`)
  );
}

/** BF: the wrecked battleship: where it drifts, its tilt and its size. */
const BATTLEFIELD_WRECK = { x: -4, y: 0, turn: 24, scale: 0.92 } as const;
/** BF: the break that splits its hull, top to bottom, in the hull's own frame. */
const BATTLEFIELD_BREAK: readonly (readonly [number, number])[] = [[4, -18], [8, -8], [2, -2], [9, 5], [3, 18]];
/** BF: how far its bow drifts from its stern: across, down and turned. */
const BATTLEFIELD_DRIFT: readonly [number, number, number] = [7, 5, 14];
/** BF: the shards strewn about it: where each drifts, its tilt and its size. */
const BATTLEFIELD_SHARDS: readonly (readonly [number, number, number, number])[] = [
  [-34, -22, 20, 5],
  [30, -28, -35, 6],
  [36, 22, 60, 4.5],
  [-32, 24, -15, 6.5],
  [8, 32, 80, 3.5],
  [-12, -34, 45, 3.2],
  [40, -6, 10, 3],
];
/** BF: a shard's outline, as shares of its size. */
const SHARD_SHAPE: readonly (readonly [number, number])[] = [[-1, -0.4], [0.2, -0.7], [1, -0.1], [0.3, 0.6], [-0.6, 0.4]];
/** BF: the fires still burning on the wreck and at its break: where each is and its radius. */
const BATTLEFIELD_FIRES: readonly (readonly [number, number, number])[] = [[-14, -6, 3], [4, 2, 3.4], [16, 10, 2.4], [26, -4, 1.8]];

/** BF: wreck-strewn space: a battleship broken in two, its shards drifting and its fires still burning. */
function battlefieldBoard(rng: Rng, p: Paint, defs: Defs): string {
  const out: string[] = [];
  for (const [x, y, turn, size] of BATTLEFIELD_SHARDS) {
    out.push(group(at(x, y, turn, size), poly(SHARD_SHAPE, "url(#hl)", p.line, { "stroke-width": STROKE / size })));
  }
  const { x, y, turn, scale } = BATTLEFIELD_WRECK;
  out.push(group(at(x, y, turn, scale), broken(hull(rng, p, "battleship"), BATTLEFIELD_BREAK, BATTLEFIELD_DRIFT, defs)));
  for (const [fx, fy, r] of BATTLEFIELD_FIRES) {
    out.push(circle(fx, fy, r * 2, p.glow, { opacity: ANOMALY_HAZE }));
    out.push(twinkle(fx, fy, r * 1.5, p.glow, ANOMALY_LIT));
    out.push(circle(fx, fy, r / 2, "#ffffff", { opacity: ANOMALY_LIT }));
  }
  return out.join("");
}

/** The dust drifting between a field's rocks: how many grains. */
const FIELD_DUST = 12;
/** How far from a field's middle its dust and its scattered rocks reach. */
const FIELD_REACH = 40;
/** A grain of a field's dust: its least and its greatest radius. */
const FIELD_DUST_RADII: readonly [number, number] = [0.4, 1.2];

function dust(rng: Rng, p: Paint): string {
  const out: string[] = [];
  for (let i = 0; i < FIELD_DUST; i += 1) {
    const [low, high] = FIELD_DUST_RADII;
    out.push(circle(between(rng, -FIELD_REACH, FIELD_REACH), between(rng, -FIELD_REACH, FIELD_REACH), between(rng, low, high), p.light, { opacity: ANOMALY_SHADE }));
  }
  return out.join("");
}

/** R3337001: asteroids, each where it drifts and its radius. */
function rocks(rng: Rng, p: Paint, list: readonly (readonly [number, number, number])[]): string {
  return list.map(([x, y, r]) => group(at(x, y), asteroid(rng, p, r))).join("");
}

/** AL: the great rocks of the large field: where each drifts and its radius. */
const LARGE_FIELD_ROCKS: readonly (readonly [number, number, number])[] = [[22, -24, 11], [20, 15, 16], [-12, -4, 22]];

/** AL: two or three great asteroids. */
function largeFieldBoard(rng: Rng, p: Paint): string {
  return dust(rng, p) + rocks(rng, p, LARGE_FIELD_ROCKS);
}

/** AT: how many pebbles the small field scatters. */
const SMALL_FIELD_ROCKS = 16;
/** AT: a pebble's least and greatest radius. */
const SMALL_FIELD_RADII: readonly [number, number] = [2.6, 6.5];

/** AT: many small asteroids, the smaller behind. */
function smallFieldBoard(rng: Rng, p: Paint): string {
  const [low, high] = SMALL_FIELD_RADII;
  const list: (readonly [number, number, number])[] = [];
  for (let i = 0; i < SMALL_FIELD_ROCKS; i += 1) {
    list.push([between(rng, -FIELD_REACH, FIELD_REACH), between(rng, -FIELD_REACH, FIELD_REACH), between(rng, low, high)]);
  }
  list.sort((a, b) => a[2] - b[2]);
  return dust(rng, p) + rocks(rng, p, list);
}

/** AN: the cluster of red narcour crystals in the field's middle: where its rock sits and its size. */
const NARCOUR_ORE = { x: 0, y: 4, scale: 0.9 } as const;
/** AN: the glow its crystals cast, its radius. */
const NARCOUR_GLOW = 28;
/** AN: the rocks ringing it: where each drifts and its radius. */
const NARCOUR_ROCKS: readonly (readonly [number, number, number])[] = [[-30, -22, 9], [30, -20, 11], [-30, 22, 7], [32, 24, 8], [4, -34, 5]];

/** AN: asteroids ringing a cluster of red narcour crystals. */
function narcourBoard(rng: Rng, p: Paint): string {
  const { x, y, scale } = NARCOUR_ORE;
  return (
    dust(rng, p) +
    rocks(rng, p, NARCOUR_ROCKS) +
    circle(x, y, NARCOUR_GLOW, p.accent, { opacity: ANOMALY_HAZE }) +
    group(at(x, y, 0, scale), resource(rng, p, "red-narcor", ""))
  );
}

/** AA: the axnit-veined rock: where it drifts and its radius. */
const AXNIT_ROCK: readonly [number, number, number] = [-4, 8, 25];
/** AA: the veins of axnit through it, each a run of points in the rock's frame. */
const AXNIT_VEINS: readonly (readonly (readonly [number, number])[])[] = [
  [[-18, -6], [-8, -2], [-4, 6], [6, 10], [14, 17]],
  [[-6, -18], [-2, -8], [8, -4], [18, 2]],
  [[-15, 12], [-6, 8], [-4, 6]],
];
/** AA: a vein's stroke. */
const AXNIT_VEIN = 1.2;
/** AA: the stroke of a vein's glow. */
const AXNIT_VEIN_GLOW = 3.4;
/** AA: the ore breaking out of the rock: where it sits and its size. */
const AXNIT_ORE = { x: 12, y: -12, scale: 0.5 } as const;
/** AA: the rocks that drift with it: where each drifts and its radius. */
const AXNIT_ROCKS: readonly (readonly [number, number, number])[] = [[32, 26, 7], [-33, -26, 6], [33, -28, 5]];

function trail(list: readonly (readonly [number, number])[]): string {
  return list.map(([x, y], index) => `${index === 0 ? "M" : "L"}${fmt(x)} ${fmt(y)}`).join("");
}

/** AA: a great asteroid veined with axnit, its ore breaking out of it. */
function axnitBoard(rng: Rng, p: Paint): string {
  const [x, y, r] = AXNIT_ROCK;
  const veins = AXNIT_VEINS.map(
    (vein) =>
      path(trail(vein), "none", { stroke: p.accent, "stroke-width": AXNIT_VEIN_GLOW, "stroke-linejoin": "round", "stroke-linecap": "round", opacity: ANOMALY_SHADE }) +
      path(trail(vein), "none", { stroke: p.accent, "stroke-width": AXNIT_VEIN, "stroke-linejoin": "round", "stroke-linecap": "round", opacity: ANOMALY_LIT }),
  ).join("");
  const ore = AXNIT_ORE;
  return (
    dust(rng, p) +
    rocks(rng, p, AXNIT_ROCKS) +
    group(at(x, y), asteroid(rng, p, r) + veins) +
    group(at(ore.x, ore.y, 0, ore.scale), resource(rng, p, "axnit", ""))
  );
}

/** SP: the platform's break, top to bottom, in its own frame. */
const SPICE_PLATFORM_BREAK: readonly (readonly [number, number])[] = [[16, -24], [20, -12], [14, -4], [22, 3], [17, 10], [21, 18]];
/** SP: how far its broken end drifts: across, down and turned. */
const SPICE_PLATFORM_DRIFT: readonly [number, number, number] = [9, 7, 16];
/** SP: the plumes of spice it vents: where each leaves it, its heading and its length. */
const SPICE_PLUMES: readonly (readonly [number, number, number, number])[] = [[19, -6, -20, 24], [19, 8, 30, 20], [-32, 2, 190, 14], [0, -22, -95, 16]];
/** SP: a plume's half-width where it leaves the platform. */
const SPICE_PLUME_WIDTH = 1.6;
/** SP: how wide a plume fans out, as a share of its length. */
const SPICE_PLUME_FAN = 0.32;
/** SP: how far a plume's rounded tip reaches past its length, as a share of it. */
const SPICE_PLUME_TIP = 1.12;
/** SP: the share of a plume its bright core fills. */
const SPICE_PLUME_CORE = 0.55;
/** SP: the grains of spice drifting off the plumes. */
const SPICE_GRAINS = 20;
/** SP: how far along a plume a grain drifts, as shares of its length. */
const SPICE_GRAIN_REACH: readonly [number, number] = [0.4, 1.3];
/** SP: a grain's least and greatest radius. */
const SPICE_GRAIN_RADII: readonly [number, number] = [0.4, 1.2];

function plume(length: number, fan: number, width: number): (readonly [number, number])[] {
  return [[0, -width], [length, -length * fan], [length * SPICE_PLUME_TIP, 0], [length, length * fan], [0, width]];
}

/** SP: a broken station venting plumes of spice from its wound. */
function spicePlatformBoard(rng: Rng, p: Paint, defs: Defs): string {
  const out: string[] = [broken(station(p), SPICE_PLATFORM_BREAK, SPICE_PLATFORM_DRIFT, defs)];
  for (const [x, y, heading, length] of SPICE_PLUMES) {
    out.push(
      group(
        at(x, y, heading),
        poly(plume(length, SPICE_PLUME_FAN, SPICE_PLUME_WIDTH), p.accent, "none", { opacity: ANOMALY_WASH }) +
          poly(plume(length * SPICE_PLUME_CORE, SPICE_PLUME_FAN * SPICE_PLUME_CORE, SPICE_PLUME_WIDTH / 2), "#ffffff", "none", { opacity: ANOMALY_SHADE }),
      ),
    );
  }
  for (let i = 0; i < SPICE_GRAINS; i += 1) {
    const [x, y, heading, length] = SPICE_PLUMES[whole(rng, 0, SPICE_PLUMES.length - 1)] ?? [0, 0, 0, 0];
    const along = length * between(rng, SPICE_GRAIN_REACH[0], SPICE_GRAIN_REACH[1]);
    const across = along * SPICE_PLUME_FAN * between(rng, -1, 1);
    const a = heading * DEGREES;
    out.push(
      circle(x + along * Math.cos(a) - across * Math.sin(a), y + along * Math.sin(a) + across * Math.cos(a), between(rng, SPICE_GRAIN_RADII[0], SPICE_GRAIN_RADII[1]), p.accent, {
        opacity: between(rng, ANOMALY_WASH, 1),
      }),
    );
  }
  return out.join("");
}

/** MT: the Monument's slab, a tall stone narrowing from its foot to its crown. */
const MONUMENT_SLAB: readonly (readonly [number, number])[] = [[-14, 36], [-10, -26], [-5, -38], [5, -38], [10, -26], [14, 36]];
/** MT: the slab's lit flank. */
const MONUMENT_FLANK: readonly (readonly [number, number])[] = [[-14, 36], [-10, -26], [-5, -38], [-1, -38], [-2, 36]];
/** MT: the runes glowing down its face: each one's corner, width and height. */
const MONUMENT_RUNES: readonly (readonly [number, number, number, number])[] = [
  [-5, -28, 10, 1.6],
  [-6, -20, 3, 7],
  [3, -20, 3, 7],
  [-1.5, -9, 3, 3],
  [-6, -1, 12, 1.6],
  [-4, 6, 2, 10],
  [2, 6, 2, 10],
  [-5, 22, 10, 1.6],
];
/** MT: how far a rune's glow spreads past its edge. */
const MONUMENT_RUNE_BLOOM = 0.8;
/** MT: the halo of its light: its half-width and half-height. */
const MONUMENT_HALO: readonly [number, number] = [30, 50];
/** MT: where its crown is, and the radius of the light on it. */
const MONUMENT_CROWN: readonly [number, number, number] = [0, -38, 4];
/** MT: the beam its crown throws up into the void: its half-width at the crown and at its end, and its reach. */
const MONUMENT_BEAM: readonly [number, number, number] = [4, 12, 30];
/** MT: the rubble that floats about it: where each drifts and its radius. */
const MONUMENT_RUBBLE: readonly (readonly [number, number, number])[] = [[-26, 20, 4], [24, 28, 3], [22, -14, 2.4], [-24, -24, 2]];

/** MT: a colossal monolith, its runes glowing, in the void. */
function monumentBoard(rng: Rng, p: Paint): string {
  const [hx, hy] = MONUMENT_HALO;
  const [cx, cy, cr] = MONUMENT_CROWN;
  const [beamNear, beamFar, beamReach] = MONUMENT_BEAM;
  const out: string[] = [
    ellipse(0, 0, hx, hy, p.glow, { opacity: ANOMALY_HAZE }),
    ellipse(0, 0, hx / 2, hy, p.glow, { opacity: ANOMALY_HAZE }),
    poly([[cx - beamNear, cy], [cx - beamFar, cy - beamReach], [cx + beamFar, cy - beamReach], [cx + beamNear, cy]], p.glow, "none", { opacity: ANOMALY_HAZE }),
    rocks(rng, p, MONUMENT_RUBBLE),
    poly(MONUMENT_SLAB, p.dark, p.glow),
    poly(MONUMENT_FLANK, p.hull, "none", { opacity: ANOMALY_SHADE }),
  ];
  for (const [x, y, w, h] of MONUMENT_RUNES) {
    out.push(rect(x - MONUMENT_RUNE_BLOOM, y - MONUMENT_RUNE_BLOOM, w + MONUMENT_RUNE_BLOOM * 2, h + MONUMENT_RUNE_BLOOM * 2, p.glow, { opacity: ANOMALY_SHADE }));
    out.push(rect(x, y, w, h, p.accent, { opacity: ANOMALY_LIT }));
  }
  out.push(circle(cx, cy, cr * 2, p.glow, { opacity: ANOMALY_WASH }));
  out.push(twinkle(cx, cy, cr * 2, "#ffffff", ANOMALY_LIT));
  out.push(circle(cx, cy, cr / 2, "#ffffff"));
  return out.join("");
}

/** DH: the rock the drones hide in, its radius. */
const HIDEOUT_ROCK = 32;
/** DH: the step between the holes honeycombing it. */
const HIDEOUT_STEP = 9;
/** DH: how far from its middle the holes reach. */
const HIDEOUT_REACH = 24;
/** DH: a hole's radius. */
const HIDEOUT_HOLE = 3.4;
/** DH: a drone's eye in its hole, its radius. */
const HIDEOUT_EYE = 2;
/** DH: an eye's pupil, as a share of it. */
const HIDEOUT_PUPIL = 0.45;
/** DH: the chance a hole's drone is awake, its eye lit. */
const HIDEOUT_AWAKE = 0.7;
/** DH: a honeycomb's rows lie this share of its step apart. */
const HEX_ROW = Math.sqrt(3) / 2;
/** DH: the drones out on patrol: where each flies and its size. */
const HIDEOUT_PATROLS: readonly (readonly [number, number, number])[] = [[33, -28, 0.42], [-34, 27, 0.34]];
/** DH: a patrolling drone's radius, before its size. */
const HIDEOUT_DRONE = 11;

/** DH: a rock honeycombed with holes, a drone's glowing eye in each. */
function hideoutBoard(rng: Rng, p: Paint): string {
  const out: string[] = [circle(0, 0, HIDEOUT_ROCK, p.glow, { opacity: ANOMALY_HAZE }), asteroid(rng, p, HIDEOUT_ROCK)];
  const rowStep = HIDEOUT_STEP * HEX_ROW;
  const rows = Math.floor(HIDEOUT_REACH / rowStep);
  const cols = Math.ceil(HIDEOUT_REACH / HIDEOUT_STEP);
  for (let row = -rows; row <= rows; row += 1) {
    for (let col = -cols; col <= cols; col += 1) {
      const x = col * HIDEOUT_STEP + (Math.abs(row) % 2 === 1 ? HIDEOUT_STEP / 2 : 0);
      const y = row * rowStep;
      if (Math.hypot(x, y) > HIDEOUT_REACH) continue;
      out.push(circle(x, y, HIDEOUT_HOLE, p.dark, inked(p)));
      if (rng() < HIDEOUT_AWAKE) {
        out.push(circle(x, y, HIDEOUT_EYE * 2, p.glow, { opacity: ANOMALY_HAZE }));
        out.push(circle(x, y, HIDEOUT_EYE, p.glow));
        out.push(circle(x, y, HIDEOUT_EYE * HIDEOUT_PUPIL, p.dark));
      }
    }
  }
  for (const [x, y, size] of HIDEOUT_PATROLS) out.push(group(at(x, y, 0, size), droneBody(p, HIDEOUT_DRONE, 2)));
  return out.join("");
}

/** FO: where the outpost's station stands, and its size. */
const OUTPOST_STATION = { x: -6, y: 0, scale: 0.8 } as const;
/** FO: the warm light around it, its radius. */
const OUTPOST_GLOW = 30;
/** FO: how many lights ring it. */
const OUTPOST_LIGHTS = 20;
/** FO: the ring of its lights: its half-width and half-height. */
const OUTPOST_RING: readonly [number, number] = [40, 22];
/** FO: one light's radius. */
const OUTPOST_LIGHT = 1.1;
/**
 * FO: the map's edge it stands at, a curve right of it: where it runs at its ends, where it bulges, how far it reaches
 * up and down, and how far the unmapped dark beyond it runs.
 */
const OUTPOST_EDGE = { x: 36, bulge: 52, reach: 46, beyond: 70 } as const;
/** FO: the frontier's dash. */
const OUTPOST_EDGE_DASH = "3 2";
/** FO: the beacons along the frontier. */
const OUTPOST_BEACONS = 5;

/** FO: a lit station ringed with lights, at the edge of the map. */
function outpostBoard(p: Paint): string {
  const { x, y, scale } = OUTPOST_STATION;
  const { x: edgeX, bulge, reach, beyond } = OUTPOST_EDGE;
  const curve = `M${fmt(edgeX)} ${fmt(-reach)}Q${fmt(bulge)} 0 ${fmt(edgeX)} ${fmt(reach)}`;
  const out: string[] = [
    path(`${curve}L${fmt(beyond)} ${fmt(reach)}L${fmt(beyond)} ${fmt(-reach)}Z`, p.dark, { opacity: ANOMALY_WASH }),
    path(curve, "none", { stroke: p.accent, "stroke-width": STROKE * 2, "stroke-dasharray": OUTPOST_EDGE_DASH, opacity: ANOMALY_LIT }),
  ];
  for (let i = 0; i < OUTPOST_BEACONS; i += 1) {
    const t = (i + 1) / (OUTPOST_BEACONS + 1);
    const bx = (1 - t) * (1 - t) * edgeX + 2 * (1 - t) * t * bulge + t * t * edgeX;
    const by = (2 * t - 1) * reach;
    out.push(twinkle(bx, by, OUTPOST_LIGHT * 3, p.glow, ANOMALY_LIT));
  }
  out.push(circle(x, y, OUTPOST_GLOW, p.glow, { opacity: ANOMALY_HAZE }));
  const [rx, ry] = OUTPOST_RING;
  for (let i = 0; i < OUTPOST_LIGHTS; i += 1) {
    const a = (i * FULL_TURN * DEGREES) / OUTPOST_LIGHTS;
    const lx = x + rx * Math.cos(a);
    const ly = y + ry * Math.sin(a);
    out.push(circle(lx, ly, OUTPOST_LIGHT * 2, p.glow, { opacity: ANOMALY_SHADE }));
    out.push(circle(lx, ly, OUTPOST_LIGHT, p.glow));
    out.push(circle(lx, ly, OUTPOST_LIGHT / 2, "#ffffff"));
  }
  out.push(group(at(x, y, 0, scale), station(p)));
  return out.join("");
}

function anomalyBoard(rng: Rng, p: Paint, code: AnomalyCode, defs: Defs): string {
  switch (code) {
    case "WH":
      return wormholeBoard(p);
    case "BF":
      return battlefieldBoard(rng, p, defs);
    case "AL":
      return largeFieldBoard(rng, p);
    case "AT":
      return smallFieldBoard(rng, p);
    case "SP":
      return spicePlatformBoard(rng, p, defs);
    case "MT":
      return monumentBoard(rng, p);
    case "DH":
      return hideoutBoard(rng, p);
    case "FO":
      return outpostBoard(p);
    case "AN":
      return narcourBoard(rng, p);
    case "AA":
      return axnitBoard(rng, p);
  }
}

/**
 * R3337001: the art an Anomaly Found! in play wears for the board its code names (anomalies.ts), drawn as the
 * Starship's is: its own seed, the set's space behind it, gilded with the rays on the Radiant face.
 */
export function anomalySvg(code: AnomalyCode, radiant: boolean): string {
  const look = ANOMALY_LOOKS[code];
  const seed = hashId(`${ANOMALY_SEED_PREFIX}${code}`);
  const family = FAMILY_PAINTS[look.family];
  const base: Paint = look.kind === null ? family : { ...family, accent: KIND_COLORS[look.kind] ?? family.accent };
  const p = radiant ? gild(base) : base;
  const behind = seededRandom((seed ^ BACKDROP_SALT) >>> 0);
  const ahead = seededRandom((seed ^ SUBJECT_SALT) >>> 0);
  const tilt = between(ahead, -SUBJECT_TILT_JITTER, SUBJECT_TILT_JITTER);
  const defs: Defs = [];
  gradients(p, defs);
  const sky = backdrop(behind, p, look.family, look.planet, defs);
  const body = anomalyBoard(ahead, p, code, defs);
  return (
    `<svg viewBox="0 0 ${BOX} ${BOX}" width="${BOX}" height="${BOX}" xmlns="http://www.w3.org/2000/svg" preserveAspectRatio="xMidYMid slice">` +
    `<defs>${defs.join("")}</defs>` +
    sky +
    (radiant ? rays(seed) : "") +
    group(at(SUBJECT_X, SUBJECT_Y, tilt), body) +
    rect(0, 0, BOX, BOX, "url(#v)") +
    "</svg>"
  );
}

/**
 * R3333002: a piece's module as a small icon on a riveted slate plate, for a Dormant hull's plates and the pieces
 * list: an id that is no piece draws the spare plate.
 */
export function starshipModuleIconSvg(defId: string, radiant: boolean): string {
  const m = starshipModuleOf(defId);
  const base = modulePaint(m);
  const p = radiant ? gild(base) : base;
  const [cx, cy] = moduleCenter(m);
  const rivets = [[-13, -8], [13, -8], [-13, 8], [13, 8]].map(([x, y]) => circle(x ?? 0, y ?? 0, 0.7, ICON_RIVET)).join("");
  return (
    `<svg viewBox="-15 -10 30 20" width="30" height="20" xmlns="http://www.w3.org/2000/svg">` +
    rect(-15, -10, 30, 20, ICON_PLATE, { rx: 2 }) +
    rivets +
    group(at(-cx * MODULE_ICON_SCALE, -cy * MODULE_ICON_SCALE, 0, MODULE_ICON_SCALE), moduleMarkup(m, p)) +
    "</svg>"
  );
}
