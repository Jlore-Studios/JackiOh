// R3333001 (#3333): each Starship Piece's module, the part it is drawn as where it mounts on a Starship's hull
// (starshipHulls.ts, stinkyscape.ts `starshipSvg`), large on its own card (stinkyscape.ts) and small as its icon on
// the hull's plates and in the pieces list. A module is a function of the piece's definition alone: its kind, the
// tier its name marks, the hardpoint it mounts on and the bay its card stands in. It is drawn in flat fills of the
// paint it is given, never a gradient, so any picture can mount it without its defs. Presentation only (CLAUDE.md
// rule 7).

import { fmt } from "./hash.ts";
import { at, circle, ellipse, group, inked, line, OUTLINE, path, poly, rect, twinkle, type Paint } from "./stinkyDraw.ts";

/** Where on a hull a module mounts: the bow's deck, the spine behind it, the flanks, the keel or the stern. */
export type Hardpoint = "bow" | "flank" | "stern" | "keel" | "spine";
export type ModuleGroup = "weapon" | "miner" | "defense" | "engine" | "launcher" | "pod" | "odd";
/** The bay a piece's own card stands in. */
export type Hangar = "range" | "orebay" | "shieldlab" | "teststand" | "loading" | "medbay" | "hangar" | "blueprint";

export type ModuleKind =
  | "beam" | "repeater" | "cannon" | "flak" | "ap-rounds" | "vamp-rounds" | "drill"
  | "shield" | "plating" | "crystal" | "capacitor" | "booster"
  | "afterburner" | "overdrive" | "warpfield" | "stimulus"
  | "rush-launcher" | "fruit-launcher" | "jade-launcher" | "book-launcher"
  | "healing" | "poison" | "frost" | "magic" | "cloaking" | "felinor" | "nanobots" | "tractor"
  | "disintegrex" | "rail" | "trila" | "orca" | "spacemine" | "chassis" | "salvage"
  | "yamato" | "siege-tank" | "missile-pod" | "spare";

export type StarshipModule = {
  defId: string;
  kind: ModuleKind;
  tier: number;
  group: ModuleGroup;
  mount: Hardpoint;
  hangar: Hangar;
};

const PREFIX = "stinkyscape-";

/**
 * Every piece definition's module, by its id: the 47 cards that print Starship Piece (stinkyscape.ts
 * `STARSHIP_PIECES`) and the three Jlockeed weapons that are pieces only when Embiggened (#127 to #129). The
 * tier is the one the card's name marks (`tierOf`), 1 where it marks none.
 */
export const STARSHIP_MODULE_IDS: Readonly<Record<string, readonly [ModuleKind, number]>> = Object.fromEntries(
  (
    [
      ["038", "shield", 1], ["040", "poison", 1], ["043", "healing", 1], ["048", "chassis", 1], ["049", "spacemine", 1],
      ["050", "beam", 3], ["051", "repeater", 3], ["052", "cannon", 3], ["053", "flak", 3],
      ["054", "beam", 4], ["055", "repeater", 4], ["056", "cannon", 4], ["057", "flak", 4],
      ["058", "beam", 5], ["059", "repeater", 5], ["060", "cannon", 5], ["061", "flak", 5],
      ["062", "drill", 1], ["063", "drill", 2], ["064", "drill", 3], ["065", "drill", 4], ["066", "drill", 5],
      ["067", "tractor", 1], ["079", "orca", 1], ["090", "afterburner", 1], ["093", "nanobots", 1], ["098", "booster", 1],
      ["100", "ap-rounds", 1], ["101", "overdrive", 1], ["104", "plating", 1], ["105", "disintegrex", 1],
      ["106", "warpfield", 1], ["107", "rail", 1], ["108", "trila", 1], ["122", "vamp-rounds", 1],
      ["131", "stimulus", 1], ["132", "crystal", 1], ["133", "capacitor", 1], ["134", "felinor", 1], ["136", "magic", 1],
      ["140", "rush-launcher", 1], ["141", "fruit-launcher", 1], ["142", "jade-launcher", 1], ["143", "book-launcher", 1],
      ["175", "frost", 1], ["177", "cloaking", 1], ["t-ai-10", "salvage", 1],
      ["127", "siege-tank", 1], ["128", "yamato", 1], ["129", "missile-pod", 1],
    ] as const
  ).map(([suffix, kind, tier]) => [`${PREFIX}${suffix}`, [kind, tier] as const]),
);

type KindSpec = { group: ModuleGroup; mount: Hardpoint; hangar: Hangar; energy: string };

function kinds(group: ModuleGroup, mount: Hardpoint, hangar: Hangar, energies: Readonly<Partial<Record<ModuleKind, string>>>): [ModuleKind, KindSpec][] {
  return Object.entries(energies).map(([kind, energy]) => [kind as ModuleKind, { group, mount, hangar, energy: energy ?? OUTLINE }]);
}

/** Each kind's group, hardpoint, bay and energy (the color its glow, shot or fluid takes). */
export const MODULE_KINDS: Readonly<Record<ModuleKind, KindSpec>> = Object.fromEntries([
  ...kinds("weapon", "bow", "range", {
    beam: "#4ff0ff", repeater: "#ffa04a", cannon: "#ff5a2a", flak: "#ff8a3a", "ap-rounds": "#c0c8d0", "vamp-rounds": "#ff2a3a",
  }),
  ...kinds("miner", "keel", "orebay", { drill: "#4fa8ff" }),
  ...kinds("defense", "spine", "shieldlab", { shield: "#5ad8ff", plating: "#dfe8ff", crystal: "#7af0d0", capacitor: "#8ab8ff", booster: "#7fe8ff" }),
  ...kinds("engine", "stern", "teststand", { afterburner: "#ff7a2a", overdrive: "#ff4a3a", warpfield: "#8a6aff", stimulus: "#ff9a3a" }),
  ...kinds("launcher", "flank", "loading", { "rush-launcher": "#ff6a3a", "fruit-launcher": "#ff6a9a", "jade-launcher": "#2fbf7a", "book-launcher": "#c86aff" }),
  ...kinds("pod", "flank", "medbay", {
    healing: "#ff6a9a", poison: "#8aff3a", frost: "#9ae8ff", magic: "#c86aff", cloaking: "#9aa8c8", felinor: "#c08aff", nanobots: "#6aff9a", tractor: "#6ab8ff",
  }),
  ...kinds("odd", "bow", "hangar", { disintegrex: "#7aff5a", rail: "#b48aff", trila: "#ff8a5a" }),
  ...kinds("odd", "keel", "hangar", { orca: "#6ab8ff" }),
  ...kinds("odd", "stern", "hangar", { spacemine: "#ff3a2a" }),
  ...kinds("odd", "spine", "hangar", { chassis: "#bfe4ff", salvage: "#5affb0" }),
  ...kinds("weapon", "bow", "blueprint", { yamato: "#ff9ad8" }),
  ...kinds("odd", "keel", "blueprint", { "siege-tank": "#ff6a3a" }),
  ...kinds("launcher", "flank", "blueprint", { "missile-pod": "#ff6a3a" }),
  ...kinds("pod", "flank", "hangar", { spare: "#9aa8b8" }),
]) as Record<ModuleKind, KindSpec>;

/** The kinds whose tiers climb a ladder (v1 to v5), drawn bigger and busier each step, their tier in chevrons. */
export const LADDERED_KINDS: ReadonlySet<ModuleKind> = new Set<ModuleKind>(["beam", "repeater", "cannon", "flak", "drill"]);

export const MODULE_TIER_MAX = 5;
/** A module at its biggest fits this box about its center (`MOUNT_CENTERS`). */
export const MODULE_BOX = 24;
export const MODULE_TIER_SCALE_BASE = 0.6;
export const MODULE_TIER_SCALE_STEP = 0.1;
/** A module on no ladder draws at this tier's size. */
export const MODULE_PLAIN_TIER = 3;

/** Where a module's middle sits from its mount, before its scale: above a deck, below the keel, behind the stern. */
export const MOUNT_CENTERS: Readonly<Record<Hardpoint, readonly [number, number]>> = {
  bow: [0, -6],
  spine: [0, -6],
  keel: [0, 6],
  stern: [-6, 0],
  flank: [0, 0],
};

/** The modules' gunmetal, its accent and glow set by the module (`modulePaint`). */
export const MODULE_PAINT: Paint = {
  space: ["#141d3d", "#04060f"],
  nebula: ["#4f5dd8", "#a13fa8"],
  light: "#c9ced6",
  hull: "#6b737e",
  dark: "#2a2f37",
  accent: "#e8642a",
  glow: "#ffffff",
  line: OUTLINE,
};

export const GROUP_ACCENTS: Readonly<Record<ModuleGroup, string>> = {
  weapon: "#e8642a",
  miner: "#ff7a1a",
  defense: "#3ab0ff",
  engine: "#ff4a3a",
  launcher: "#d8443a",
  pod: "#3ad0a0",
  odd: "#b48aff",
};

/** The colors a few kinds draw that no paint holds: the rounds' copper and tips, a fruit, a book, an orca. */
const COPPER = "#c8743a";
const AP_STEEL = "#3a3f48";
const VAMP_RED = "#d0142a";
const MINE_RED = "#ff3a2a";
const FRUIT_RED = "#e2513f";
const LEAF_GREEN = "#6ad04a";
const JADE = "#2fbf7a";
const JADE_CORE = "#9affc8";
const BOOK_COVER = "#8a3a2a";
const BOOK_SPINE = "#5a2010";
const BOOK_PAGES = "#f4f1ea";
const ORCA_BLACK = "#1a1d24";
const ORCA_WHITE = "#eef2f6";
const CAT_FUR = "#e8e0f0";
const CAT_EYES = "#3aa84a";
const CHIP_GREEN = "#1f5a3d";
const WHITE = "#ffffff";
const CLOAK_OPACITY = 0.5;

export function moduleTierScale(tier: number): number {
  return MODULE_TIER_SCALE_BASE + MODULE_TIER_SCALE_STEP * Math.max(1, Math.min(MODULE_TIER_MAX, tier));
}

/** The scale a module draws at: its tier's on a ladder, else `MODULE_PLAIN_TIER`'s. */
export function moduleScale(m: StarshipModule): number {
  return moduleTierScale(LADDERED_KINDS.has(m.kind) ? m.tier : MODULE_PLAIN_TIER);
}

/** The middle of a module as drawn about its mount, where an icon or a card centers it. */
export function moduleCenter(m: StarshipModule): readonly [number, number] {
  const [x, y] = MOUNT_CENTERS[m.mount];
  const scale = moduleScale(m);
  return [x * scale, y * scale];
}

/** A piece's module; an id that is no piece draws the spare plate, tier 1. */
export function starshipModuleOf(defId: string): StarshipModule {
  const [kind, tier] = STARSHIP_MODULE_IDS[defId] ?? (["spare", 1] as const);
  const { group, mount, hangar } = MODULE_KINDS[kind];
  return { defId, kind, tier, group, mount, hangar };
}

/** The gunmetal a module is drawn in, its glow its kind's energy and its accent its group's. */
export function modulePaint(m: StarshipModule): Paint {
  return { ...MODULE_PAINT, glow: MODULE_KINDS[m.kind].energy, accent: GROUP_ACCENTS[m.group] };
}

// The painters: each draws about its mount (0, 0) facing +x, a deck module rising to -y, a keel module hanging to
// +y, a stern module trailing to -x and a flank module centered.

type Painter = (tier: number, p: Paint) => string;

function deck(p: Paint): string {
  return poly([[-8, 2.5], [8, 2.5], [6.5, -0.8], [-6.5, -0.8]], p.dark, p.line) + rect(-6.5, -1.6, 13, 1, p.hull, inked(p));
}

function underDeck(p: Paint): string {
  return poly([[-7, -2.5], [7, -2.5], [5.5, 0.8], [-5.5, 0.8]], p.dark, p.line) + rect(-1.2, 0.5, 2.4, 2.5, p.hull, inked(p));
}

function flange(p: Paint): string {
  return rect(-1.5, -4.5, 3, 9, p.dark, inked(p));
}

function chevrons(p: Paint, tier: number, x: number, y: number): string {
  let d = "";
  for (let i = 0; i < tier; i += 1) d += `M${fmt(x + i * 1.5)} ${fmt(y)}l0.7 0.8l-0.7 0.8`;
  return path(d, "none", { stroke: p.accent, "stroke-width": 0.45, "stroke-linejoin": "round" });
}

function tip(p: Paint, x: number, y: number, r: number): string {
  return circle(x, y, r, p.glow, inked(p)) + circle(x, y, r * 0.45, WHITE);
}

function flame(p: Paint, x: number, half: number, reach: number): string {
  return (
    poly([[x, -half], [x - reach * 0.8, -half * 0.45], [x - reach, 0], [x - reach * 0.8, half * 0.45], [x, half]], p.glow, "none", { opacity: 0.85 }) +
    poly([[x, -half * 0.5], [x - reach * 0.6, 0], [x, half * 0.5]], WHITE, "none", { opacity: 0.85 })
  );
}

const beam: Painter = (t, p) => {
  const out = [deck(p), rect(-8, -7.5, 9, 6.5, p.hull, { ...inked(p), rx: 1.2 }), rect(-7.2, -7, 7.4, 1.2, p.light, { opacity: 0.7 })];
  for (let i = 0; i <= t; i += 1) out.push(rect(-7.4 + (i * 7) / Math.max(1, t), -9.8, 0.8, 2.4, p.dark, inked(p)));
  const length = 3 + t;
  out.push(rect(1, -5, length, 2, p.light, inked(p)));
  for (let i = 0; i < t; i += 1) out.push(ellipse(1.8 + ((length - 1.6) * (i + 0.5)) / t, -4, 0.7, 1.9, p.accent, inked(p)));
  out.push(tip(p, 1 + length, -4, 1.4));
  return out.join("");
};

const repeater: Painter = (t, p) => {
  const out = [deck(p), rect(-8, -8.5, 8, 7.5, p.hull, { ...inked(p), rx: 1 })];
  const barrels = t + 1;
  const height = 0.9;
  const gap = 0.25;
  const total = barrels * height + (barrels - 1) * gap;
  const top = -4.75 - total / 2;
  for (let i = 0; i < barrels; i += 1) {
    const y = top + i * (height + gap);
    out.push(rect(0, y, 9.5, height, p.light, inked(p)));
    out.push(circle(9.9, y + height / 2, 0.35, p.glow));
  }
  for (const x of [3, 7]) out.push(rect(x, top - 0.5, 1.2, total + 1, p.dark, inked(p)));
  if (t >= 3) out.push(circle(-4, -4.75, 2.6, p.dark, inked(p)), circle(-4, -4.75, 1.2, p.accent));
  return out.join("");
};

const cannon: Painter = (t, p) => {
  const out = [deck(p), poly([[-8, -1], [-8, -7], [-5, -9.5], [2, -9.5], [3, -6], [3, -1]], p.hull, p.line), rect(-5, -9, 6.5, 1, p.light, { opacity: 0.6 })];
  const barrels = t <= 2 ? 1 : t <= 4 ? 2 : 3;
  const thick = 1.3 + 0.12 * t;
  const length = 3.5 + 0.9 * t;
  for (let i = 0; i < barrels; i += 1) {
    const y = -5.2 + (i - (barrels - 1) / 2) * (thick + 0.6);
    out.push(rect(3, y - thick / 2, length, thick, p.light, inked(p)));
    if (t >= 2) out.push(rect(2.2 + length, y - thick / 2 - 0.4, 1.4, thick + 0.8, p.dark, inked(p)));
  }
  if (t >= 4) out.push(poly([[-9, -1.2], [-9, -6.8], [-6.5, -8.6], [-6.5, -1.2]], p.dark, p.line), rect(-4, -10.6, 5, 1.1, p.dark, inked(p)));
  return out.join("");
};

const flak: Painter = (t, p) => {
  const out = [deck(p), rect(-6, -6, 8, 5, p.hull, { ...inked(p), rx: 1.2 })];
  out.push(group(at(1, -4.5, -35), rect(0, -1.6, 7.5, 1.3, p.light, inked(p)) + rect(0, 0.3, 7.5, 1.3, p.light, inked(p))));
  out.push(twinkle(8.4, -9.6, 1.6, p.glow, 0.9));
  for (let i = 0; i < t; i += 1) {
    const x = -5.4 + i * 1.3;
    out.push(rect(x, -8.8, 0.9, 2.8, p.accent, inked(p)), ellipse(x + 0.45, -8.9, 0.45, 0.6, p.light));
  }
  if (t >= 3) out.push(line(-6.5, -6, -8, -10.5, p.dark, 0.6), path("M-10.2 -11Q-8 -8.6 -5.8 -11Z", p.light, inked(p)));
  return out.join("");
};

function rounds(p: Paint, tipColor: string, vamp: boolean): string {
  const out = [deck(p), circle(-3.5, -5, 4, p.hull, inked(p)), circle(-3.5, -5, 2.6, p.dark), circle(-3.5, -5, 1, p.light), rect(0.4, -6, 3.6, 2, p.dark, inked(p))];
  for (const [x, tilt] of [[5, -8], [7.4, 0], [9.8, 8]] as const) {
    out.push(
      group(
        at(x, -1, tilt),
        rect(-0.9, -5, 1.8, 5, COPPER, inked(p)) +
          path("M-0.9 -5L-0.9 -6Q0 -8.4 0.9 -6L0.9 -5Z", tipColor, inked(p)) +
          (vamp ? "" : rect(-0.9, -5.7, 1.8, 0.5, "#111111")),
      ),
    );
  }
  out.push(vamp ? path("M9 -12Q10.4 -9.8 9 -9Q7.6 -9.8 9 -12Z", VAMP_RED, inked(p)) : line(4.2, -9.4, 6.4, -11.2, p.glow, 0.6));
  return out.join("");
}

const drill: Painter = (t, p) => {
  const out = [underDeck(p), rect(-7, 3, 9, 6, p.hull, { ...inked(p), rx: 1 }), rect(-7, 7.2, 9, 1.2, p.accent)];
  const length = 3 + 1.2 * t;
  out.push(poly([[2, 3.2], [2 + length, 6], [2, 8.8]], p.light, p.line));
  const bands = t + 2;
  for (let i = 1; i <= bands; i += 1) {
    const x = 2 + (length * i) / (bands + 1);
    const half = 2.8 * (1 - i / (bands + 1));
    out.push(line(x, 6 - half, x + 0.6, 6 + half, p.dark, 0.5));
  }
  for (let i = 0; i < Math.ceil(t / 2); i += 1) {
    const x = -6.5 + i * 3.2;
    out.push(poly([[x, 9], [x + 2.6, 9], [x + 2.1, 11.5], [x + 0.5, 11.5]], p.dark, p.line), circle(x + 1.3, 9, 0.7, p.glow));
  }
  if (t >= 3) out.push(line(-5, 0.6, -6.4, 3, p.dark, 0.8), line(5, 0.6, 1.6, 3, p.dark, 0.8));
  if (t >= 5) {
    out.push(rect(-7, 12.4, 14, 1.4, p.dark, { ...inked(p), rx: 0.7 }));
    for (const x of [-5.5, -2, 1.5, 5]) out.push(circle(x, 13.1, 0.45, p.light));
  }
  return out.join("");
};

const shield: Painter = (_, p) =>
  [
    path("M-10 -1A10 10 0 0 1 10 -1Z", p.glow, { opacity: 0.18 }),
    path("M-10 -1A10 10 0 0 1 10 -1", "none", { stroke: p.glow, "stroke-width": 1, opacity: 0.85 }),
    path("M-7.5 -1A7.5 7.5 0 0 1 7.5 -1", "none", { stroke: p.glow, "stroke-width": 0.4, opacity: 0.6 }),
    deck(p),
    rect(-1, -6, 2, 5, p.hull, inked(p)),
    path("M-5 -6Q0 -9.5 5 -6Z", p.light, inked(p)),
    circle(0, -8, 1.2, p.glow, inked(p)),
  ].join("");

const plating: Painter = (_, p) => {
  const out = [rect(-9, -0.8, 18, 1.6, p.dark, inked(p))];
  for (let i = 0; i < 3; i += 1) {
    const x = -8.5 + i * 5.4;
    out.push(poly([[x, 0], [x + 2, -7 + i * 0.6], [x + 6.5, -7 + i * 0.6], [x + 5.5, 0]], p.light, p.line));
    out.push(line(x + 2.6, -5.8 + i * 0.6, x + 4.4, -1, WHITE, 0.7, { opacity: 0.8 }), circle(x + 1.6, -1, 0.4, p.dark));
  }
  return out.join("");
};

const crystal: Painter = (_, p) => {
  const out = [poly([[-6, 0.5], [6, 0.5], [4.2, -2.5], [-4.2, -2.5]], p.hull, p.line)];
  for (const [x, tilt, h] of [[-3, -18, 7], [3.2, 16, 6.5], [0, 0, 10]] as const) {
    const shard = poly([[-1.6, 0], [-1.8, -h * 0.7], [0, -h], [1.8, -h * 0.7], [1.6, 0]], p.glow, p.line, { opacity: 0.92 });
    out.push(group(at(x, -2.5, tilt), shard + poly([[-0.6, -0.4], [-0.7, -h * 0.68], [0, -h * 0.92]], WHITE, "none", { opacity: 0.5 })));
  }
  return out.join("");
};

const capacitor: Painter = (_, p) => {
  const out = [deck(p)];
  for (const x of [-6, -1.7, 2.6]) {
    out.push(rect(x, -8, 3.4, 7.2, p.hull, { ...inked(p), rx: 1.2 }), rect(x + 0.4, -7.6, 0.8, 6.4, p.light, { opacity: 0.6 }), rect(x + 0.6, -9, 2.2, 1.1, p.dark, inked(p)));
  }
  out.push(rect(-6.2, -9.6, 12.4, 0.8, p.accent));
  out.push(path("M-4.3 -10.6L-2.4 -12L-1 -10.6L0.6 -12.2L2.6 -10.6L4.6 -12", "none", { stroke: p.glow, "stroke-width": 0.7, "stroke-linejoin": "round" }));
  return out.join("");
};

const booster: Painter = (_, p) => {
  const out = [poly([[-9, 0], [-7.5, -6.5], [7.5, -6.5], [9, 0]], p.hull, p.line), rect(-7, -7.6, 14, 1.1, p.glow, { opacity: 0.8 })];
  for (const x of [-5, 5]) out.push(rect(x - 0.6, -6.5, 1.2, 6.5, p.dark, { opacity: 0.6 }));
  out.push(rect(-0.9, -5.6, 1.8, 4.4, p.accent), rect(-2.2, -4.3, 4.4, 1.8, p.accent));
  for (const [x, y] of [[-7, -1], [7, -1], [-6.2, -5.4], [6.2, -5.4]] as const) out.push(circle(x, y, 0.45, p.light));
  return out.join("");
};

const afterburner: Painter = (_, p) =>
  [
    flame(p, -7.6, 4, 8.6),
    flange(p),
    poly([[-1.5, -3.6], [-7, -5], [-7, 5], [-1.5, 3.6]], p.hull, p.line),
    rect(-7.6, -5.2, 1, 10.4, p.light, inked(p)),
    ...[-10, -12.2].map((x) => path(`M${x} -1L${x - 0.8} 0L${x} 1L${x + 0.8} 0Z`, WHITE, { opacity: 0.9 })),
  ].join("");

const overdrive: Painter = (_, p) => {
  const out = [group(at(0, -2.6), flame(p, -11, 1.2, 4)), group(at(0, 2.6), flame(p, -11, 1.2, 4)), flange(p)];
  out.push(rect(-9, -4.5, 7.5, 9, p.hull, { ...inked(p), rx: 1.5 }));
  for (let i = 0; i < 3; i += 1) out.push(poly([[-8.4 + i * 2.4, -4.5], [-7.4 + i * 2.4, -4.5], [-8.6 + i * 2.4, -2.8], [-9.6 + i * 2.4, -2.8]], p.accent, "none"));
  out.push(circle(-5.2, 0.8, 2.6, p.dark, inked(p)), circle(-5.2, 0.8, 1.6, p.glow));
  for (const turn of [0, 60, 120]) out.push(group(at(-5.2, 0.8, turn), line(-1.5, 0, 1.5, 0, p.dark, 0.4)));
  out.push(rect(-11, -3.8, 2, 2.4, p.dark, inked(p)), rect(-11, 1.4, 2, 2.4, p.dark, inked(p)));
  return out.join("");
};

const warpfield: Painter = (_, p) =>
  [
    path("M-9 -6Q-14 0 -9 6", "none", { stroke: p.glow, "stroke-width": 0.8, opacity: 0.8 }),
    path("M-12 -7.5Q-18 0 -12 7.5", "none", { stroke: p.glow, "stroke-width": 0.6, opacity: 0.5, "stroke-dasharray": "1.2 0.8" }),
    flange(p),
    rect(-6, -0.8, 5, 1.6, p.dark, inked(p)),
    ellipse(-6, 0, 2.4, 5.5, "none", { stroke: p.hull, "stroke-width": 1.6 }),
    ellipse(-6, 0, 2.4, 5.5, "none", { stroke: p.glow, "stroke-width": 0.5 }),
    circle(-6, 0, 1.2, WHITE),
  ].join("");

const stimulus: Painter = (_, p) =>
  [
    flame(p, -9, 1.6, 6.5),
    rect(-1.5, -1, 3, 2, p.dark, inked(p)),
    poly([[-9, -2.4], [-10.5, -4.6], [-7, -2.4]], p.hull, p.line),
    poly([[-9, 2.4], [-10.5, 4.6], [-7, 2.4]], p.hull, p.line),
    rect(-9, -2.4, 9.5, 4.8, p.light, { ...inked(p), rx: 1 }),
    poly([[0.5, -2.4], [3.5, 0], [0.5, 2.4]], p.accent, p.line),
    rect(-6, -1.2, 3, 2.4, p.glow, { ...inked(p), rx: 1.2 }),
    line(-3.4, -0.6, -1.4, -0.6, WHITE, 0.4, { opacity: 0.8 }),
  ].join("");

function tube(p: Paint): string {
  return [
    rect(-10, -2.2, 1.4, 4.4, p.dark, inked(p)),
    rect(-9, -3, 14, 6, p.hull, { ...inked(p), rx: 1.5 }),
    rect(-6, -3.2, 1.2, 6.4, p.dark, inked(p)),
    rect(-1.5, -3.2, 1.2, 6.4, p.dark, inked(p)),
    rect(-8.4, -2.4, 12, 0.9, p.light, { opacity: 0.5 }),
    rect(5, -3.6, 1.6, 7.2, p.light, inked(p)),
  ].join("");
}

function launcher(payload: (p: Paint) => string): Painter {
  return (_, p) => tube(p) + line(6.6, -1.4, 7.6, -1.4, p.light, 0.4, { opacity: 0.6 }) + payload(p);
}

const rushLauncher = launcher((p) => circle(9, 0, 2.6, p.glow, inked(p)) + path("M7.8 -1.2L9 0L7.8 1.2M9.2 -1.2L10.4 0L9.2 1.2", "none", { stroke: p.dark, "stroke-width": 0.5 }));
const fruitLauncher = launcher(
  (p) => circle(9, 0.4, 2.6, FRUIT_RED, inked(p)) + path("M9 -2.2Q11.2 -4.4 12 -2.6Q10.4 -1.8 9 -2.2Z", LEAF_GREEN, inked(p)) + circle(8.1, -0.5, 0.7, WHITE, { opacity: 0.5 }),
);
const jadeLauncher = launcher((p) => circle(9, 0, 2.8, JADE, inked(p)) + circle(9, 0, 1.4, JADE_CORE, { opacity: 0.8 }) + circle(8, -1, 0.6, WHITE, { opacity: 0.7 }));
const bookLauncher = launcher((p) =>
  group(at(9, 0, -14), rect(-2.4, -3.2, 4.8, 6.4, BOOK_COVER, { ...inked(p), rx: 0.4 }) + rect(-2.4, -3.2, 1, 6.4, BOOK_SPINE) + rect(2, -2.8, 0.6, 5.6, BOOK_PAGES)),
);

function capsule(p: Paint, fluid: string, body: string): string {
  return [
    rect(-9, -4, 18, 8, p.hull, { ...inked(p), rx: 4 }),
    rect(-7.6, -3.4, 15.2, 1.2, p.light, { opacity: 0.45, rx: 0.6 }),
    rect(-5.5, -2.4, 11, 4.8, fluid, { ...inked(p), rx: 2.4, opacity: 0.9 }),
    rect(-9.6, -1.6, 1, 3.2, p.dark, inked(p)),
    rect(8.6, -1.6, 1, 3.2, p.dark, inked(p)),
    body,
  ].join("");
}

const healing: Painter = (_, p) => capsule(p, p.glow, rect(-0.6, -1.8, 1.2, 3.6, WHITE) + rect(-1.8, -0.6, 3.6, 1.2, WHITE));

const poison: Painter = (_, p) =>
  capsule(
    p,
    p.glow,
    [[-3, 0.6, 0.8], [-0.6, -0.8, 0.5], [2, 0.4, 0.7], [3.8, -0.9, 0.4]].map(([x, y, r]) => circle(x ?? 0, y ?? 0, r ?? 0, WHITE, { opacity: 0.55 })).join("") +
      path("M2 4Q3 5.6 2 6.4Q1 5.6 2 4Z", p.glow, inked(p)),
  );

const frost: Painter = (_, p) =>
  capsule(
    p,
    p.glow,
    [0, 60, 120].map((turn) => group(at(0, 0, turn), line(-1.9, 0, 1.9, 0, WHITE, 0.45))).join("") +
      [-6, -2, 2, 6].map((x) => poly([[x - 0.8, -4], [x, -5.6], [x + 0.8, -4]], WHITE, "none", { opacity: 0.8 })).join(""),
  );

const magic: Painter = (_, p) =>
  capsule(p, p.glow, twinkle(0, 0, 2, WHITE, 0.95) + twinkle(-6.4, -5.6, 1.2, p.glow, 0.9) + twinkle(6.8, -5.2, 0.9, p.glow, 0.8) + twinkle(3, 5.6, 0.8, p.glow, 0.8));

const cloaking: Painter = (_, p) =>
  group(
    at(0, 0),
    [
      rect(-9, -4, 18, 8, p.hull, { stroke: p.light, "stroke-width": 0.6, "stroke-dasharray": "1.2 0.8", rx: 4 }),
      rect(-5.5, -2.4, 11, 4.8, p.glow, { stroke: p.light, "stroke-width": 0.4, "stroke-dasharray": "0.8 0.6", rx: 2.4 }),
      path("M-7 -1.2Q-3.5 -2.6 0 -1.2T7 -1.2M-7 1.4Q-3.5 0 0 1.4T7 1.4", "none", { stroke: WHITE, "stroke-width": 0.35, opacity: 0.7 }),
    ].join(""),
    { opacity: CLOAK_OPACITY },
  );

const felinor: Painter = (_, p) =>
  [
    rect(-7, -6, 14, 12, p.hull, { ...inked(p), rx: 2 }),
    rect(-7.6, -6.6, 15.2, 1.6, p.dark, inked(p)),
    rect(-7.6, 5, 15.2, 1.6, p.dark, inked(p)),
    circle(0, 0, 4, p.light, inked(p)),
    circle(0, 0, 3.2, p.glow, { opacity: 0.85 }),
    poly([[-2, -0.4], [-1.7, -2.9], [-0.4, -1.5]], CAT_FUR, p.line),
    poly([[2, -0.4], [1.7, -2.9], [0.4, -1.5]], CAT_FUR, p.line),
    ellipse(0, 0.5, 2.1, 1.8, CAT_FUR, inked(p)),
    circle(-0.8, 0.2, 0.36, CAT_EYES),
    circle(0.8, 0.2, 0.36, CAT_EYES),
    path("M-0.3 0.9L0.3 0.9L0 1.3Z", p.accent),
    path("M-1 1.2L-3 0.8M-1 1.5L-3 1.8M1 1.2L3 0.8M1 1.5L3 1.8", "none", { stroke: p.dark, "stroke-width": 0.2 }),
    rect(4.6, -4.4, 1.4, 1.4, p.accent),
  ].join("");

const nanobots: Painter = (_, p) => {
  const out = [rect(-8, -4.5, 9, 9, p.hull, { ...inked(p), rx: 1 })];
  for (const [x, y] of [[-5.6, -1.6], [-3.6, -1.6], [-4.6, 0.1], [-2.6, 0.1], [-5.6, 1.8], [-3.6, 1.8]] as const) {
    out.push(poly([[x - 0.9, y], [x - 0.45, y - 0.8], [x + 0.45, y - 0.8], [x + 0.9, y], [x + 0.45, y + 0.8], [x - 0.45, y + 0.8]], p.glow, "none", { opacity: 0.8 }));
  }
  for (const [x, y] of [[3, -2.4], [5.6, -0.4], [4.4, 2.6], [7.8, 1.2], [8.6, -2.8]] as const) {
    out.push(circle(x, y, 0.8, p.light, inked(p)), circle(x, y, 0.3, p.glow));
  }
  out.push(line(1, 0, 2.4, -1.8, p.glow, 0.25, { opacity: 0.6, "stroke-dasharray": "0.4 0.4" }));
  return out.join("");
};

const tractor: Painter = (_, p) => {
  const out = [poly([[2, -0.6], [11, -5], [11, 5], [2, 0.6]], p.glow, "none", { opacity: 0.25 })];
  for (const x of [5, 7.6, 10]) out.push(ellipse(x, 0, 0.6, (x - 2) * 0.5, "none", { stroke: p.glow, "stroke-width": 0.35, opacity: 0.7 }));
  out.push(group(at(9.4, 0.4, 20), poly([[-1.6, -1], [-0.5, -1.8], [1.3, -1.3], [1.8, 0.3], [0.8, 1.5], [-1, 1.3], [-1.8, 0.3]], "#8a7a68", p.line)));
  out.push(rect(-7, -3, 7, 6, p.hull, { ...inked(p), rx: 1 }), path("M0 -4Q3.4 0 0 4Z", p.light, inked(p)), line(0, 0, 2.2, 0, p.dark, 0.5), circle(2.2, 0, 0.7, p.glow));
  return out.join("");
};

const disintegrex: Painter = (_, p) => {
  const out = [deck(p), ellipse(-3, -5, 5, 2.6, p.hull, inked(p))];
  for (const x of [-5.4, -3.4, -1.4]) out.push(ellipse(x, -5, 0.6, 3.2, p.accent, inked(p)));
  out.push(path("M2 -7Q4 -5 2 -3Z", p.light, inked(p)));
  out.push(path("M3.6 -5L5.2 -6.2L6.6 -4L8 -6L9.4 -4.2L11 -5.4", "none", { stroke: p.glow, "stroke-width": 0.9, "stroke-linejoin": "round" }));
  out.push(circle(3.4, -5, 1.1, p.glow, { opacity: 0.8 }));
  return out.join("");
};

const rail: Painter = (_, p) => {
  const out = [deck(p), rect(-8, -7, 6, 6, p.hull, { ...inked(p), rx: 1 }), rect(-2, -8, 13, 1.1, p.light, inked(p)), rect(-2, -3.6, 13, 1.1, p.light, inked(p))];
  for (const x of [1, 5, 9]) out.push(rect(x, -8.2, 0.8, 5.8, p.dark, inked(p)));
  out.push(path("M-1 -5.4L1 -6.4L3 -4.6L5 -6.4L7 -4.6L9 -6.4L11 -5.4", "none", { stroke: p.glow, "stroke-width": 0.5 }));
  out.push(rect(10.4, -5.8, 1.4, 0.8, p.glow));
  return out.join("");
};

const trila: Painter = (_, p) => {
  const out = [deck(p)];
  for (const turn of [-14, 0, 14]) out.push(group(at(0, -5.5, turn), rect(0, -0.75, 9.5, 1.5, p.light, inked(p)) + circle(10, 0, 0.6, p.glow)));
  out.push(circle(0, -5.5, 3.2, p.hull, inked(p)), circle(0, -5.5, 1.4, p.dark), circle(0, -5.5, 0.6, p.accent));
  return out.join("");
};

const orca: Painter = (_, p) =>
  [
    underDeck(p),
    poly([[-0.6, 4], [1, 1.6], [2.4, 4]], ORCA_BLACK, p.line),
    poly([[-7, 6.6], [-10, 4.4], [-9.4, 7], [-10, 9.4]], ORCA_BLACK, p.line),
    ellipse(1, 7, 8, 3.2, ORCA_BLACK, inked(p)),
    path("M-5 8.2Q1 11.6 8.6 7.6Q4 8.6 -5 8.2Z", ORCA_WHITE),
    ellipse(5.2, 5.8, 1.3, 0.6, ORCA_WHITE),
    poly([[9, 6.2], [11, 7], [9, 7.8]], p.light, p.line),
    circle(-2, 7, 0.6, p.glow),
  ].join("");

const spacemine: Painter = (_, p) => {
  const out = [flange(p), rect(-6, -3, 5, 6, p.hull, { ...inked(p), rx: 0.8 }), line(-6, 0, -9.4, 1.6, p.light, 0.3, { "stroke-dasharray": "0.6 0.5" })];
  for (let i = 0; i < 8; i += 1) out.push(group(at(-11.5, 2.4, i * 45), poly([[2.2, -0.5], [3.6, 0], [2.2, 0.5]], p.hull, p.line)));
  out.push(circle(-11.5, 2.4, 2.6, p.dark, inked(p)), circle(-11.5, 2.4, 1, MINE_RED), circle(-12.4, 1.5, 0.6, WHITE, { opacity: 0.3 }));
  return out.join("");
};

const chassis: Painter = (_, p) =>
  [
    path("M-9 0L-9 -6L9 -6L9 0ZM-9 0L-4.5 -6L0 0L4.5 -6L9 0M-4.5 -6L-4.5 0M0 -6L0 0M4.5 -6L4.5 0", "none", {
      stroke: p.light,
      "stroke-width": 0.6,
      "stroke-linejoin": "round",
    }),
    ...[-9, -4.5, 0, 4.5, 9].flatMap((x) => [circle(x, 0, 0.55, p.accent), circle(x, -6, 0.55, p.accent)]),
  ].join("");

const salvage: Painter = (_, p) =>
  [
    deck(p),
    rect(-8, -5, 4, 4, CHIP_GREEN, inked(p)),
    rect(-7, -4, 2, 2, p.glow),
    circle(-2, -1.6, 1.6, p.hull, inked(p)),
    group(at(-2, -1.6, -55), rect(0, -0.7, 8, 1.4, p.light, inked(p))),
    group(at(2.6, -8.1, 20), rect(0, -0.6, 6, 1.2, p.light, inked(p))),
    path("M8 -6.4Q10 -6 9.6 -3.6M8 -6.4Q9.6 -8 11 -6.4", "none", { stroke: p.dark, "stroke-width": 0.8, "stroke-linecap": "round" }),
    circle(2.6, -8.1, 0.8, p.accent),
  ].join("");

const yamato: Painter = (_, p) =>
  [
    deck(p),
    poly([[-9, -1], [-9, -8], [-6, -10.5], [3, -10.5], [4, -7], [4, -1]], p.hull, p.line),
    rect(-6, -10, 8, 1, p.light, { opacity: 0.6 }),
    rect(4, -7, 6.5, 3.4, p.light, inked(p)),
    rect(9.6, -7.6, 1.4, 4.6, p.dark, inked(p)),
    circle(12, -5.3, 2.4, p.glow, { opacity: 0.4 }),
    circle(12, -5.3, 1.4, p.glow, { opacity: 0.85 }),
    circle(12, -5.3, 0.6, WHITE),
  ].join("");

const siegeTank: Painter = (_, p) => {
  const out = [underDeck(p), rect(-7, 9, 13, 3.4, p.dark, { ...inked(p), rx: 1.7 })];
  for (const x of [-5.4, -2.2, 1, 4.2]) out.push(circle(x, 10.7, 1.1, p.hull, inked(p)));
  out.push(poly([[-7, 9], [-5.6, 5.4], [4.4, 5.4], [6, 9]], p.hull, p.line), path("M-3.4 5.4Q-3.4 2.6 0 2.6Q3 2.6 3 5.4Z", p.light, inked(p)));
  out.push(rect(2.6, 3.4, 8.4, 1.2, p.light, inked(p)), rect(-2, 3.4, 2.4, 1, p.glow));
  return out.join("");
};

const missilePod: Painter = (_, p) => {
  const out = [rect(-7, -4.5, 11, 9, p.hull, { ...inked(p), rx: 1 })];
  for (let row = 0; row < 2; row += 1) {
    for (let col = 0; col < 3; col += 1) {
      const x = -4.6 + col * 3.2;
      const y = -2 + row * 4;
      out.push(circle(x, y, 1.3, p.dark, inked(p)));
      if (!(row === 0 && col === 2)) out.push(circle(x, y, 0.75, p.accent));
    }
  }
  out.push(group(at(7.6, -4, -24), path("M-5 0Q-3 -0.5 -1.5 0Q-3 0.5 -5 0Z", p.light, { opacity: 0.5 }) + rect(-1.5, -0.6, 4, 1.2, p.light, inked(p)) + poly([[2.5, -0.6], [3.8, 0], [2.5, 0.6]], p.accent, p.line)));
  return out.join("");
};

const spare: Painter = (_, p) =>
  [
    rect(-7, -4, 14, 8, p.hull, { ...inked(p), rx: 1 }),
    line(-6, -3, 6, 3, p.dark, 0.5, { opacity: 0.6 }),
    ...[[-5.8, -2.8], [5.8, -2.8], [-5.8, 2.8], [5.8, 2.8]].map(([x, y]) => circle(x ?? 0, y ?? 0, 0.55, p.light)),
  ].join("");

const PAINTERS: Readonly<Record<ModuleKind, Painter>> = {
  beam,
  repeater,
  cannon,
  flak,
  "ap-rounds": (_, p) => rounds(p, AP_STEEL, false),
  "vamp-rounds": (_, p) => rounds(p, VAMP_RED, true),
  drill,
  shield,
  plating,
  crystal,
  capacitor,
  booster,
  afterburner,
  overdrive,
  warpfield,
  stimulus,
  "rush-launcher": rushLauncher,
  "fruit-launcher": fruitLauncher,
  "jade-launcher": jadeLauncher,
  "book-launcher": bookLauncher,
  healing,
  poison,
  frost,
  magic,
  cloaking,
  felinor,
  nanobots,
  tractor,
  disintegrex,
  rail,
  trila,
  orca,
  spacemine,
  chassis,
  salvage,
  yamato,
  "siege-tank": siegeTank,
  "missile-pod": missilePod,
  spare,
};

/** Where a laddered module's chevrons sit: on its housing, above the deck or below the keel. */
const CHEVRON_AT: Readonly<Record<Hardpoint, readonly [number, number]>> = {
  bow: [-6.6, -3.6],
  spine: [-6.6, -3.6],
  keel: [-6.4, 4],
  stern: [-8, -1],
  flank: [-6, -1],
};

/** A module drawn about its mount in `p`, flat, at its scale; a laddered one marks its tier in chevrons. */
export function moduleMarkup(m: StarshipModule, p: Paint): string {
  const [x, y] = CHEVRON_AT[m.mount];
  const marks = LADDERED_KINDS.has(m.kind) ? chevrons(p, m.tier, x, y) : "";
  return group(at(0, 0, 0, moduleScale(m)), PAINTERS[m.kind](m.tier, p) + marks);
}
