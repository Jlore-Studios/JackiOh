// R3333000 (#3333): a Starship's hull, drawn from what it holds. Its class grows with the count of its pieces, from a
// frigate to a dreadnought; each piece's module (starshipModules.ts) mounts on the hardpoint its kind takes, oldest
// first; a launched Starship wears the hull design of the name it took (`STARSHIP_NAMES`, R1852004), and a Dormant
// one, still unnamed, sits in a shipyard. A design and a livery are the painter's inputs (`StarshipArt`), so a
// cosmetic may hand it another (#3330). Presentation only (CLAUDE.md rule 7): the pieces are the view's, public on
// both seats (R1852005). R3335000 (#3335): a launched Starship wears its seat's Starship cosmetics
// (starshipLooks.ts), a hull design of their own among them, the Stinkyscape flagship's the most elaborate of all.

import type { StarshipPieceRef } from "../../wire/generated/StarshipPieceRef.ts";
import type { EmblemGlyph } from "./emblems.ts";
import { fmt, hashId } from "./hash.ts";
import { HULL_DESIGNS, LIVERIES, lookPick, PIECE_SKINS, TRAILS, type ModuleSkin, type StarshipLook, type StarshipTrail } from "./starshipLooks.ts";
import { MODULE_BOX, type Hardpoint } from "./starshipModules.ts";
import { circle, ellipse, glyph, inked, line, OUTLINE, path, poly, rect, twinkle, type Paint } from "./stinkyDraw.ts";
import { mixHex } from "./themes.ts";

export type StarshipDesignId =
  | "ninth-life" | "frontiers-promise" | "celestial-archive" | "lantern-ferry" | "jlockeed-skylark" | "ember-of-departure" | "silent-choir"
  | "shipyard"
  | "scout" | "courier" | "freighter" | "corsair" | "cathedral" | "leviathan" | "ark" | "stinkyscape" | "jlorious";

/** A design's bow. */
export type Prow = "whiskered" | "wedge" | "dish" | "junk" | "canard" | "ram" | "pipes" | "ribs" | "flagship";
/** A design's line of windows. */
export type WindowLine = "portholes" | "ribbon" | "grid" | "lanterns" | "slits" | "cracks" | "sparse" | "dark" | "galleries";

/** A hull's colors: its plating, its trim, its stripe, its windows lit and its engines' glow. */
export type Livery = { hull: string; trim: string; stripe: string; window: string; glow: string };

/** A hull design; `sky` is the name its picture's sky grows from, its id when it has none. */
export type HullDesign = { id: StarshipDesignId; prow: Prow; windows: WindowLine; livery: Livery; emblem: EmblemGlyph | null; sky?: string };

/**
 * Each of the seven names' hull, as its name suits, the shipyard's bare one and, R3335000, each hull design cosmetic's
 * (`HULL_DESIGNS`): the names' prows and windows in new pairs, each in a livery of its own under its emblem, but for the
 * Stinkyscape flagship's, a prow and galleries no other design has. No livery is in Radiant's gold.
 */
export const STARSHIP_DESIGNS: Readonly<Record<StarshipDesignId, HullDesign>> = {
  "ninth-life": {
    id: "ninth-life",
    prow: "whiskered",
    windows: "portholes",
    livery: { hull: "#a08ac8", trim: "#ece4f6", stripe: "#6a4aa8", window: "#ffd0f0", glow: "#c8a0ff" },
    emblem: "cat",
  },
  "frontiers-promise": {
    id: "frontiers-promise",
    prow: "wedge",
    windows: "ribbon",
    livery: { hull: "#e4e8ee", trim: "#1f2f5a", stripe: "#c8323a", window: "#9ad8ff", glow: "#7fd8ff" },
    emblem: "flag",
  },
  "celestial-archive": {
    id: "celestial-archive",
    prow: "dish",
    windows: "grid",
    livery: { hull: "#2a3f7a", trim: "#c8d0dc", stripe: "#8aa0d8", window: "#bfe4ff", glow: "#8ab8ff" },
    emblem: "book",
  },
  "lantern-ferry": {
    id: "lantern-ferry",
    prow: "junk",
    windows: "lanterns",
    livery: { hull: "#a8222e", trim: "#2fbf7a", stripe: "#1a7a52", window: "#ff9a5a", glow: "#ff7a4a" },
    emblem: "flame",
  },
  "jlockeed-skylark": {
    id: "jlockeed-skylark",
    prow: "canard",
    windows: "slits",
    livery: { hull: "#6ab0e8", trim: "#ffffff", stripe: "#1a5aa8", window: "#e6f3ff", glow: "#bfe4ff" },
    emblem: "jet",
  },
  "ember-of-departure": {
    id: "ember-of-departure",
    prow: "ram",
    windows: "cracks",
    livery: { hull: "#3a3a40", trim: "#6a6a74", stripe: "#ff6a1a", window: "#ff8a3a", glow: "#ff5a1a" },
    emblem: "star",
  },
  "silent-choir": {
    id: "silent-choir",
    prow: "pipes",
    windows: "sparse",
    livery: { hull: "#5a6470", trim: "#9aa8b8", stripe: "#3a4450", window: "#c8e0ff", glow: "#a8c8ff" },
    emblem: "note",
  },
  shipyard: {
    id: "shipyard",
    prow: "ribs",
    windows: "dark",
    livery: { hull: "#8a8f96", trim: "#5a5f66", stripe: "#ff6a14", window: "#2a2f37", glow: "#ff8a3a" },
    emblem: null,
  },
  // A picket's canards and a lookout's portholes, in field green.
  scout: {
    id: "scout",
    prow: "canard",
    windows: "portholes",
    livery: { hull: "#5a7a5a", trim: "#d8e0c8", stripe: "#ff6a2a", window: "#c8ffd8", glow: "#8affc8" },
    emblem: "eye",
  },
  // A mail runner's wedge and slits, teal and white, under a sky of its own: its id's hangs a planet in Radiant's gold.
  courier: {
    id: "courier",
    prow: "wedge",
    windows: "slits",
    livery: { hull: "#2a9a9a", trim: "#f0f8f8", stripe: "#ff5a6a", window: "#e0ffff", glow: "#6affea" },
    emblem: "bolt",
    sky: "courier-post",
  },
  // A hauler's blunt ram and few windows, in cargo orange.
  freighter: {
    id: "freighter",
    prow: "ram",
    windows: "sparse",
    livery: { hull: "#c8642a", trim: "#3a3f48", stripe: "#e8e8ec", window: "#ffc8a0", glow: "#ff8a4a" },
    emblem: "crate",
  },
  // A raider's wedge, scarred, black and crimson.
  corsair: {
    id: "corsair",
    prow: "wedge",
    windows: "cracks",
    livery: { hull: "#1f1a22", trim: "#c8142a", stripe: "#e8e0e0", window: "#ff4a5a", glow: "#ff2a4a" },
    emblem: "bones",
  },
  // A spire over lancet windows, pale stone and stained glass.
  cathedral: {
    id: "cathedral",
    prow: "dish",
    windows: "sparse",
    livery: { hull: "#c8c4d8", trim: "#4a3a6a", stripe: "#a82a5a", window: "#8ad8ff", glow: "#e0b0ff" },
    emblem: "halo",
  },
  // A deep thing's barbels and its lures, in abyssal teal.
  leviathan: {
    id: "leviathan",
    prow: "whiskered",
    windows: "lanterns",
    livery: { hull: "#1a4a5a", trim: "#6ad0c8", stripe: "#0a2a3a", window: "#7affd0", glow: "#3affc8" },
    emblem: "jaws",
  },
  // A refuge's high junk bow and portholes, green with life.
  ark: {
    id: "ark",
    prow: "junk",
    windows: "portholes",
    livery: { hull: "#4a7a4a", trim: "#eef0e6", stripe: "#3a8ad8", window: "#d8ffb0", glow: "#9aff8a" },
    emblem: "leaf",
  },
  // The Mythic capstone: the flagship's own bow and galleries, in the Stinkyscape's indigo, cyan and magenta.
  stinkyscape: {
    id: "stinkyscape",
    prow: "flagship",
    windows: "galleries",
    livery: { hull: "#3a2f7a", trim: "#43f5d2", stripe: "#d43fd0", window: "#e6d8ff", glow: "#7affea" },
    emblem: "vortex",
  },
  // The Jlorious rank's: violet and ivory under a star.
  jlorious: {
    id: "jlorious",
    prow: "pipes",
    windows: "ribbon",
    livery: { hull: "#5a2a9a", trim: "#f6f2ea", stripe: "#9a5ae8", window: "#fff8f0", glow: "#c89aff" },
    emblem: "star",
  },
};

/** R1852004's seven names, each its design; starship.test.tsx holds the names to the engine's `STARSHIP_NAMES`. */
export const STARSHIP_DESIGN_NAMES: Readonly<Record<string, StarshipDesignId>> = {
  "The Ninth Life": "ninth-life",
  "Frontier's Promise": "frontiers-promise",
  "The Celestial Archive": "celestial-archive",
  "The Lantern Ferry": "lantern-ferry",
  "The Jlockeed Skylark": "jlockeed-skylark",
  "Ember of Departure": "ember-of-departure",
  "The Silent Choir": "silent-choir",
};

export const LAUNCHED_DESIGNS: readonly StarshipDesignId[] = Object.values(STARSHIP_DESIGN_NAMES);

/** The design a Starship wears: the shipyard's while Dormant, else its name's, a name no design has one by its hash. */
export function starshipDesignOf(launched: boolean, name: string | undefined): StarshipDesignId {
  if (!launched) return "shipyard";
  const named = name === undefined ? undefined : STARSHIP_DESIGN_NAMES[name];
  return named ?? LAUNCHED_DESIGNS[hashId(name ?? "") % LAUNCHED_DESIGNS.length] ?? "ember-of-departure";
}

// The classes

export type StarshipClassId = "frigate" | "destroyer" | "cruiser" | "dreadnought";
/**
 * A class's half length and half height, the count of pieces it starts at and how close the picture draws it: a
 * small ship nearer, so a frigate still fills its window, though each class stands longer in it than the last.
 */
export type StarshipClass = { id: StarshipClassId; from: number; length: number; height: number; zoom: number };

const FRIGATE: StarshipClass = { id: "frigate", from: 0, length: 20, height: 6, zoom: 1.15 };

export const STARSHIP_CLASSES: readonly StarshipClass[] = [
  FRIGATE,
  { id: "destroyer", from: 3, length: 26, height: 7, zoom: 1 },
  { id: "cruiser", from: 5, length: 32, height: 8.5, zoom: 0.9 },
  { id: "dreadnought", from: 8, length: 38, height: 10, zoom: 0.8 },
];

/** The most modules one Starship's picture mounts, its oldest pieces', however many it holds. */
export const STARSHIP_MODULES_MAX = 16;

export function starshipClassOf(count: number): StarshipClass {
  let found = STARSHIP_CLASSES[0] ?? FRIGATE;
  for (const cls of STARSHIP_CLASSES) if (cls.from <= count) found = cls;
  return found;
}

// The hardpoints

export const MODULE_SHIP_SCALE_MIN = 0.5;
export const MODULE_SHIP_SCALE_MAX = 0.9;
/** A module's drawing fills this share of its box, so neighbours sit this much closer than their boxes. */
const MODULE_PACKING = 0.6;
/** A deck holding more than this many modules raises every other one on a barbette, a second row. */
const DECK_ROW_MAX = 3;
/** How high a second row's barbette lifts its module, in module boxes at its scale. */
const BARBETTE_LIFT = 0.42;

/** Where a module mounts on a hull and its scale there; `lift` is the barbette under a second row's. */
export type Placement = { x: number; y: number; scale: number; lift: number };

/** Each hardpoint's span along the hull, as shares of the half length, front to back. */
const SPANS: Readonly<Record<Exclude<Hardpoint, "stern">, readonly [number, number]>> = {
  bow: [0.7, 0.15],
  spine: [0.1, -0.6],
  flank: [0.45, -0.55],
  keel: [0.5, -0.5],
};
const FLANK_ROW = 0.35;
const STERN_SPREAD = 0.6;

function clampScale(span: number, n: number): number {
  return Math.max(MODULE_SHIP_SCALE_MIN, Math.min(MODULE_SHIP_SCALE_MAX, span / (n * MODULE_BOX * MODULE_PACKING)));
}

/**
 * Where each module of a hull of `cls` mounts, in list order: each hardpoint spreads its own evenly, the oldest
 * foremost (or highest, at the stern); a deck of more than three raises every other one; the flanks take port and
 * starboard in turn.
 */
export function hardpointPlacements(cls: StarshipClass, mounts: readonly Hardpoint[]): Placement[] {
  const L = cls.length;
  const H = cls.height;
  const counts = new Map<Hardpoint, number>();
  for (const mount of mounts) counts.set(mount, (counts.get(mount) ?? 0) + 1);
  const seen = new Map<Hardpoint, number>();
  return mounts.map((mount) => {
    const n = counts.get(mount) ?? 1;
    const i = seen.get(mount) ?? 0;
    seen.set(mount, i + 1);
    const share = (i + 0.5) / n;
    if (mount === "stern") {
      return { x: -L, y: -STERN_SPREAD * H + 2 * STERN_SPREAD * H * share, scale: clampScale(2 * STERN_SPREAD * H, n), lift: 0 };
    }
    const [front, back] = SPANS[mount];
    const x = L * (front + (back - front) * share);
    const span = L * Math.abs(front - back);
    if (mount === "flank") return { x, y: (i % 2 === 0 ? -FLANK_ROW : FLANK_ROW) * H, scale: clampScale(span, Math.ceil(n / 2)), lift: 0 };
    if (mount === "keel") return { x, y: H, scale: clampScale(span, n), lift: 0 };
    const rows = n > DECK_ROW_MAX;
    const scale = clampScale(span, rows ? Math.ceil(n / 2) : n);
    const lift = rows && i % 2 === 1 ? BARBETTE_LIFT * MODULE_BOX * scale : 0;
    return { x, y: -H - lift, scale, lift };
  });
}

// The paint

/** A hull's paint: the fleet's sky, its livery's plating in three tones, and its trim and windows besides. */
export type HullPaint = Paint & { trim: string; window: string };

const FLEET_SPACE: readonly [string, string] = ["#141d3d", "#04060f"];
const FLEET_NEBULA: readonly [string, string] = ["#4f5dd8", "#a13fa8"];
const LIGHT_MIX = 0.4;
const DARK_MIX = 0.6;

export function liveryPaint(livery: Livery): HullPaint {
  return {
    space: FLEET_SPACE,
    nebula: FLEET_NEBULA,
    light: mixHex(livery.hull, "#ffffff", LIGHT_MIX),
    hull: livery.hull,
    dark: mixHex(livery.hull, "#000000", DARK_MIX),
    accent: livery.stripe,
    glow: livery.glow,
    line: OUTLINE,
    trim: livery.trim,
    window: livery.window,
  };
}

// The hull

type Pt = readonly [number, number];

/** R3335000: the flagship's bow, a raised forecastle running out to a ram, as shares of the half length and height. */
const FLAGSHIP_OUTLINE: readonly Pt[] = [[0.86, -1.2], [0.97, -0.95], [1.06, -0.5], [1.2, -0.1], [1.06, 0.25], [1, 0.62], [0.84, 0.95]];
/** The armour bands along it, each the first moved this far aft. */
const FLAGSHIP_BAND: readonly Pt[] = [[0.84, -1], [0.95, -0.5], [1.06, -0.1], [0.96, 0.3], [0.88, 0.75]];
const FLAGSHIP_BANDS: readonly number[] = [0, -0.07, -0.14];
/** Its lance out past the ram, the lamp at its point, its canards under the bow and the running lamps down its edge. */
const FLAGSHIP_LANCE_TIP: Pt = [1.36, -0.1];
const FLAGSHIP_LANCE: readonly Pt[] = [[1.16, -0.22], FLAGSHIP_LANCE_TIP, [1.16, 0.02]];
const FLAGSHIP_TIP = 0.7;
const FLAGSHIP_CANARD: readonly Pt[] = [[0.78, 0.7], [1.02, 1.05], [0.74, 1.3]];
const FLAGSHIP_LAMPS: readonly Pt[] = [[0.99, -0.72], [1.1, -0.28], [1.04, 0.36], [0.94, 0.72]];
const FLAGSHIP_LAMP = 0.45;
/** The bridge on its forecastle (its aft edge, width, top and foot) and its lit band (inset, from its top, height). */
const FLAGSHIP_BRIDGE = { x: 0.76, width: 0.11, top: -1.8, foot: -1, inset: 0.015, band: 0.15, bandHeight: 0.15 } as const;
/** The mast over the bridge, its yard and the pennant it flies, how far forward it streams and how deep it hangs. */
const FLAGSHIP_MAST = { x: 0.82, top: -2.8, yard: -2.45, yardHalf: 0.04, pennant: 0.13, pennantDrop: 0.3 } as const;

function scaled(list: readonly Pt[], L: number, H: number): Pt[] {
  return list.map(([x, y]) => [x * L, y * H] as const);
}

/**
 * R3335000: what the flagship's bow adds over its hull: armour bands along it, a lance past its ram with a lamp at its
 * point, a bridge on its forecastle under a mast flying the flagship's pennant, canards under it and running lamps down
 * its edge.
 */
function flagshipBow(L: number, H: number, p: HullPaint, lit: boolean): string {
  const lamp = lit ? p.window : p.dark;
  const beacon = lit ? p.glow : p.dark;
  const bands = FLAGSHIP_BANDS.map((aft) => `M${FLAGSHIP_BAND.map(([x, y]) => `${fmt((x + aft) * L)} ${fmt(y * H)}`).join("L")}`).join("");
  const b = FLAGSHIP_BRIDGE;
  const m = FLAGSHIP_MAST;
  const [tipX, tipY] = FLAGSHIP_LANCE_TIP;
  return [
    path(bands, "none", { stroke: p.trim, "stroke-width": 0.5, "stroke-linejoin": "round" }),
    poly(scaled(FLAGSHIP_CANARD, L, H), p.trim, p.line),
    poly(scaled(FLAGSHIP_LANCE, L, H), p.light, p.line),
    circle(tipX * L, tipY * H, FLAGSHIP_TIP, beacon),
    rect(b.x * L, b.top * H, b.width * L, (b.foot - b.top) * H, p.hull, inked(p)),
    rect((b.x + b.inset) * L, (b.top + b.band) * H, (b.width - 2 * b.inset) * L, b.bandHeight * H, lamp),
    line(m.x * L, b.top * H, m.x * L, m.top * H, p.trim, 0.4),
    line((m.x - m.yardHalf) * L, m.yard * H, (m.x + m.yardHalf) * L, m.yard * H, p.trim, 0.3),
    poly([[m.x * L, m.top * H], [(m.x + m.pennant) * L, (m.top + m.pennantDrop / 2) * H], [m.x * L, (m.top + m.pennantDrop) * H]], p.accent, p.line),
    circle(m.x * L, m.top * H, FLAGSHIP_LAMP, beacon),
    ...FLAGSHIP_LAMPS.map(([x, y]) => circle(x * L, y * H, FLAGSHIP_LAMP, lamp)),
  ].join("");
}

/** A prow's outline from the deck's front corner to the keel's, and what it adds on top of the hull. */
function prow(kind: Prow, L: number, H: number, p: HullPaint, lit: boolean): { outline: Pt[]; over: string } {
  const lamp = lit ? p.window : p.dark;
  switch (kind) {
    case "whiskered": {
      const ears = [0.6, 0.7].map((x) => poly([[x * L - 1.2, -H], [x * L, -H * 1.5], [x * L + 1.4, -H]], p.hull, p.line)).join("");
      const whiskers = [-0.15, 0.15, 0.45].map((y) => line(L * 0.95, H * 0.1, L * 1.14, H * y, p.trim, 0.35)).join("");
      return { outline: [[0.82 * L, -0.95 * H], [0.93 * L, -0.72 * H], [0.99 * L, -0.35 * H], [L, 0.05 * H], [0.95 * L, 0.45 * H], [0.84 * L, 0.78 * H]], over: ears + whiskers };
    }
    case "wedge":
      return { outline: [[1.06 * L, 0.2 * H]], over: line(0.74 * L, -0.92 * H, 1.04 * L, 0.18 * H, p.trim, 0.5) };
    case "dish": {
      const spire = rect(0.47 * L, -2.4 * H, 1.2, 1.4 * H, p.hull, inked(p)) + path(`M${fmt(0.4 * L)} ${fmt(-2.4 * H)}Q${fmt(0.5 * L)} ${fmt(-1.9 * H)} ${fmt(0.6 * L)} ${fmt(-2.4 * H)}Z`, p.light, inked(p));
      return { outline: [[0.86 * L, -0.85 * H], [0.9 * L, 0.5 * H], [0.8 * L, 0.85 * H]], over: spire + circle(0.5 * L, -2.5 * H, 0.7, p.glow) };
    }
    case "junk": {
      const cord = line(1.01 * L, -1.45 * H, 1.06 * L, -0.95 * H, p.trim, 0.3);
      return {
        outline: [[0.95 * L, -1.55 * H], [1.03 * L, -1.4 * H], [0.88 * L, 0.2 * H], [0.78 * L, 0.8 * H]],
        over: cord + rect(1.06 * L - 0.9, -0.95 * H - 0.4, 1.8, 0.5, p.trim) + circle(1.06 * L, -0.95 * H + 0.9, 0.9, lamp, inked(p)),
      };
    }
    case "canard":
      return {
        outline: [[0.9 * L, -0.6 * H], [1.14 * L, -0.05 * H], [0.92 * L, 0.3 * H], [0.78 * L, 0.75 * H]],
        over: poly([[0.56 * L, 0.05 * H], [0.86 * L, 0.1 * H], [0.5 * L, 0.6 * H]], p.trim, p.line) + poly([[0.56 * L, -0.25 * H], [0.84 * L, -0.25 * H], [0.6 * L, -0.85 * H]], p.light, p.line),
      };
    case "ram":
      return {
        outline: [[0.84 * L, -0.5 * H], [1.1 * L, 0.2 * H], [1.12 * L, 0.55 * H], [0.78 * L, 0.8 * H]],
        over: poly([[1.0 * L, 0.0], [1.1 * L, 0.2 * H], [1.12 * L, 0.55 * H], [1.0 * L, 0.6 * H]], lit ? p.glow : p.dark, "none", { opacity: 0.8 }),
      };
    case "pipes": {
      const pipes = [0, 1, 2, 3].map((k) => rect((0.74 + k * 0.04) * L, -H - H * (0.5 + k * 0.3), 0.04 * L, H * (0.5 + k * 0.3), p.trim, inked(p))).join("");
      return { outline: [[0.9 * L, -H], [0.92 * L, 0.7 * H], [0.75 * L, 0.95 * H]], over: pipes };
    }
    case "ribs": {
      const ribs = [0.78, 0.86].map((x) => line(x * L, -0.9 * H, x * L, 0.8 * H, p.trim, 0.5)).join("");
      return { outline: [[0.9 * L, -0.6 * H], [0.96 * L, 0.2 * H], [0.8 * L, 0.8 * H]], over: ribs };
    }
    case "flagship":
      return { outline: scaled(FLAGSHIP_OUTLINE, L, H), over: flagshipBow(L, H, p, lit) };
  }
}

const PORTHOLE = 0.75;
/**
 * R3335000: the flagship's galleries: a deck of arched windows, `step` apart, `half` wide on either side of their
 * middle, their straight sides `rise` tall above their sill at `foot`, under a cornice and over a balcony's rail that
 * run `overhang` past the first, and a deck of square ports below; every height from the window line's middle.
 */
const GALLERY = {
  step: 2.6,
  half: 0.6,
  rise: 1.1,
  foot: 0.7,
  cornice: -1.4,
  rail: 1.2,
  railWidth: 0.4,
  overhang: 0.6,
  portInset: 0.1,
  portY: 2,
  portHeight: 0.8,
} as const;

/** Many boxes of one fill as one path, so a long window line weighs little. */
function boxes(list: readonly (readonly [number, number, number, number])[], fill: string): string {
  return path(list.map(([x, y, w, h]) => `M${fmt(x)} ${fmt(y)}h${fmt(w)}v${fmt(h)}h${fmt(-w)}Z`).join(""), fill);
}

/** The window line a design draws along its hull, lit or dark. */
function windows(kind: WindowLine, L: number, H: number, p: HullPaint, lit: boolean): string {
  const lamp = lit ? p.window : p.dark;
  const out: string[] = [];
  const panes: [number, number, number, number][] = [];
  const y = -0.38 * H;
  const from = -0.66 * L;
  const to = 0.6 * L;
  const along = (step: number, draw: (x: number) => void): void => {
    for (let x = from; x <= to; x += step) draw(x);
  };
  switch (kind) {
    case "portholes": {
      let d = "";
      along(3.6, (x) => {
        d += `M${fmt(x - PORTHOLE)} ${fmt(y)}a${PORTHOLE} ${PORTHOLE} 0 1 0 ${2 * PORTHOLE} 0a${PORTHOLE} ${PORTHOLE} 0 1 0 ${-2 * PORTHOLE} 0Z`;
      });
      out.push(path(d, lamp, inked(p)));
      break;
    }
    case "ribbon":
      out.push(rect(from, y - 0.6, to - from, 1.2, lamp, { rx: 0.6 }));
      break;
    case "grid":
      along(2.4, (x) => panes.push([x, y - 1.1, 1.2, 0.8], [x, y + 0.3, 1.2, 0.8]));
      break;
    case "lanterns": {
      const caps: [number, number, number, number][] = [];
      along(4.8, (x) => {
        caps.push([x - 0.8, y - 1.5, 1.6, 0.5]);
        out.push(circle(x, y, 0.85, lamp));
      });
      out.push(boxes(caps, p.trim));
      break;
    }
    case "slits": {
      let d = "";
      along(3, (x) => {
        d += `M${fmt(x)} ${fmt(y + 0.5)}l1.6 -1h0.6l-1.6 1Z`;
      });
      out.push(path(d, lamp));
      break;
    }
    case "cracks":
      out.push(path(`M${fmt(from)} ${fmt(y)}L${fmt(from + 4)} ${fmt(y - 1)}L${fmt(from + 7)} ${fmt(y + 0.6)}L${fmt(from + 11)} ${fmt(y - 0.4)}`, "none", { stroke: lamp, "stroke-width": 0.6 }));
      out.push(path(`M${fmt(0.1 * L)} ${fmt(0.1 * H)}L${fmt(0.25 * L)} ${fmt(-0.3 * H)}L${fmt(0.32 * L)} ${fmt(0.2 * H)}L${fmt(0.5 * L)} ${fmt(-0.1 * H)}`, "none", { stroke: lamp, "stroke-width": 0.5 }));
      along(6, (x) => panes.push([x, y - 0.4, 1.4, 0.8]));
      break;
    case "sparse":
      along(7, (x) => out.push(rect(x, y - 1.6, 0.9, 2.6, lamp, { rx: 0.45 })));
      break;
    case "dark":
      along(3, (x) => panes.push([x, y - 0.5, 1, 1]));
      break;
    case "galleries": {
      const g = GALLERY;
      let arches = "";
      along(g.step, (x) => {
        arches += `M${fmt(x)} ${fmt(y + g.foot)}v${-g.rise}a${g.half} ${g.half} 0 0 1 ${2 * g.half} 0v${g.rise}Z`;
        panes.push([x + g.portInset, y + g.portY, 2 * (g.half - g.portInset), g.portHeight]);
      });
      out.push(line(from - g.overhang, y + g.cornice, to + g.step, y + g.cornice, p.trim, g.railWidth));
      out.push(path(arches, lamp, inked(p)));
      out.push(line(from - g.overhang, y + g.rail, to + g.step, y + g.rail, p.trim, g.railWidth));
      break;
    }
  }
  if (panes.length > 0) out.push(boxes(panes, lamp));
  return out.join("");
}

/**
 * A hull of `design` in profile, bow to the right, centered on (0, 0), its plating the paint's `url(#hl)`: its prow,
 * windows (lit or dark), stripe and emblem; a destroyer adds a bridge, a cruiser a superstructure and a fin, a
 * dreadnought a dorsal stack and nacelles.
 */
export function hullMarkup(design: HullDesign, cls: StarshipClass, p: HullPaint, lit: boolean): string {
  const L = cls.length;
  const H = cls.height;
  const out: string[] = [];
  const big = STARSHIP_CLASSES.findIndex((each) => each.id === cls.id);
  const front = prow(design.prow, L, H, p, lit);
  if (big >= 2) out.push(poly([[-L, -0.7 * H], [-1.08 * L, -1.9 * H], [-0.95 * L, -1.9 * H], [-0.84 * L, -H]], p.hull, p.line));
  if (big >= 3) {
    for (const y of [-0.55, 0.75]) out.push(ellipse(-0.78 * L, y * H + 0.3 * H, 0.2 * L, 0.26 * H, p.dark, inked(p)), ellipse(-0.98 * L, y * H + 0.3 * H, 0.6, 0.18 * H, p.glow, { opacity: lit ? 0.9 : 0.2 }));
  }
  const outline: Pt[] = [[-L, -0.7 * H], [-0.88 * L, -H], [0.72 * L, -H], ...front.outline, [0.6 * L, H], [-0.7 * L, H], [-L, 0.7 * H]];
  out.push(poly(outline, "url(#hl)", p.line));
  out.push(poly([[-L, 0.15 * H], [0.9 * L, 0.15 * H], [0.6 * L, H], [-0.7 * L, H], [-L, 0.7 * H]], p.dark, "none", { opacity: 0.3 }));
  out.push(rect(-0.95 * L, 0.3 * H, 1.55 * L, 0.2 * H, p.accent));
  out.push(line(-0.88 * L, -H, 0.72 * L, -H, p.trim, 0.6));
  for (const seam of [-0.45, 0, 0.45]) out.push(line(seam * L, -0.9 * H, seam * L, 0.9 * H, p.dark, 0.3, { opacity: 0.5 }));
  out.push(windows(design.windows, L, H, p, lit));
  if (design.emblem !== null) out.push(glyph(design.emblem, -0.8 * L, 0.05 * H, H, p.trim, p.dark));
  if (big >= 2) out.push(poly([[-0.9 * L, -H], [-0.88 * L, -1.7 * H], [-0.64 * L, -1.7 * H], [-0.58 * L, -H]], p.hull, p.line));
  if (big >= 1) {
    const top = big >= 2 ? -1.7 * H : -H;
    out.push(rect(-0.84 * L, top - 0.5 * H, 0.22 * L, 0.5 * H, p.light, { ...inked(p), rx: 0.5 }));
    out.push(rect(-0.8 * L, top - 0.36 * H, 0.14 * L, 0.16 * H, lit ? p.window : p.dark));
  }
  if (big >= 3) out.push(rect(-0.76 * L, -2.9 * H, 0.06 * L, 0.7 * H, p.hull, inked(p)), circle(-0.73 * L, -2.9 * H, 0.7, lit ? p.glow : p.dark));
  out.push(front.over);
  return out.join("");
}

/**
 * An escort: a frigate of `design` in silhouette, its prow's outline, stripe and engine burn but none of its windows,
 * emblem or towers, which at an escort's size no one could see.
 */
export function escortMarkup(design: HullDesign, p: HullPaint): string {
  const L = FRIGATE.length;
  const H = FRIGATE.height;
  const front = prow(design.prow, L, H, p, true);
  const outline: Pt[] = [[-L, -0.7 * H], [-0.88 * L, -H], [0.72 * L, -H], ...front.outline, [0.6 * L, H], [-0.7 * L, H], [-L, 0.7 * H]];
  return (
    poly([[-L, -0.45 * H], [-1.5 * L, 0], [-L, 0.45 * H]], p.glow, "none", { opacity: 0.8 }) +
    poly(outline, p.hull, p.line) +
    rect(-0.95 * L, 0.3 * H, 1.55 * L, 0.2 * H, p.accent)
  );
}

// The ledger

/** How far apart a ledger's lamps sit at most, and their size. */
const LEDGER_STEP_MAX = 4;
const LEDGER_LAMP = 0.6;
const LEDGER_Y = 0.8;
const LEDGER_FROM = 0.5;
const LEDGER_TO = -0.7;
/** A Dormant ledger's lamps are unlit, its colors this faint. */
const LEDGER_DIM = 0.45;

/**
 * A hull's ledger: a running light along its keel for each module it mounts, the oldest nearest the bow, each in its
 * module's two colors (`ring`, its group's; `core`, its kind's), so the order its pieces went in shows wherever on the
 * hull they mount. Lit once it launches.
 */
export function ledgerMarkup(cls: StarshipClass, lamps: readonly (readonly [string, string])[], lit: boolean, p: HullPaint): string {
  if (lamps.length === 0) return "";
  const L = cls.length;
  const y = LEDGER_Y * cls.height;
  const step = Math.min(LEDGER_STEP_MAX, (L * (LEDGER_FROM - LEDGER_TO)) / Math.max(1, lamps.length - 1));
  const out: string[] = [rect(LEDGER_TO * L - 1, y - 1, LEDGER_FROM * L - LEDGER_TO * L + 2, 2, p.dark, { rx: 1, opacity: 0.8 })];
  lamps.forEach(([ring, core], k) => {
    const x = LEDGER_FROM * L - k * step;
    out.push(circle(x, y, LEDGER_LAMP, ring, lit ? {} : { opacity: LEDGER_DIM }), circle(x, y, LEDGER_LAMP * 0.5, core, lit ? {} : { opacity: LEDGER_DIM }));
  });
  return out.join("");
}

// The shipyard

export const SHIPYARD_LIGHT_MIN = 0.25;
export const SHIPYARD_LIGHT_MAX = 0.9;
/** The count of pieces at which the dock light is at its brightest. */
export const SHIPYARD_LIGHT_FULL = 8;
const SHIPYARD_LAMP = "#e8f4ff";
const SPARK = "#ffffff";
const GANTRY_REACH = 7;
const GANTRY_TOP = 15;
const GANTRY_FOOT = 13;
const GANTRY_STEP = 5;

function gantry(x: number, top: number, foot: number, p: HullPaint): string {
  const out = [line(x - 1.5, top, x - 1.5, foot, p.dark, 0.6), line(x + 1.5, top, x + 1.5, foot, p.dark, 0.6)];
  let d = "";
  for (let y = top; y < foot; y += GANTRY_STEP) {
    const next = Math.min(foot, y + GANTRY_STEP);
    d += `M${fmt(x - 1.5)} ${fmt(y)}L${fmt(x + 1.5)} ${fmt(next)}M${fmt(x + 1.5)} ${fmt(y)}L${fmt(x - 1.5)} ${fmt(next)}`;
  }
  out.push(path(d, "none", { stroke: p.dark, "stroke-width": 0.3, opacity: 0.85 }));
  return out.join("");
}

/**
 * A Dormant Starship's shipyard about its hull: X-braced gantries fore and aft under a crane, scaffolds along it,
 * welding sparks where its newest piece went in, and a dock lamp that brightens as it fills.
 */
export function shipyardMarkup(cls: StarshipClass, newest: { x: number; y: number } | null, count: number, p: HullPaint): string {
  const L = cls.length;
  const H = cls.height;
  const top = -H - GANTRY_TOP;
  const foot = H + GANTRY_FOOT;
  const aft = -L - GANTRY_REACH;
  const fore = L * 1.12 + GANTRY_REACH / 2;
  const out: string[] = [rect(aft - 3, foot, fore - aft + 6, 1.6, p.dark, inked(p))];
  out.push(gantry(aft, top, foot, p), gantry(fore, top, foot, p));
  out.push(rect(aft - 2, top - 1.4, fore - aft + 4, 1.4, p.dark, inked(p)));
  for (const x of [-0.5, 0, 0.5]) {
    out.push(line(x * L, -H - 4, x * L, H + 4, p.trim, 0.4, { opacity: 0.7 }));
    out.push(line(x * L - 3, -H - 3, x * L + 3, -H - 3, p.trim, 0.5, { opacity: 0.7 }), line(x * L - 3, H + 3, x * L + 3, H + 3, p.trim, 0.5, { opacity: 0.7 }));
  }
  const hookX = newest?.x ?? 0;
  const hookY = Math.min(newest?.y ?? -H, -H) - 3;
  out.push(rect(hookX - 1.4, top, 2.8, 1.6, p.accent, inked(p)), line(hookX, top + 1.6, hookX, hookY, p.dark, 0.3));
  out.push(path(`M${fmt(hookX)} ${fmt(hookY)}q1 0.4 0.4 1.4`, "none", { stroke: p.dark, "stroke-width": 0.4 }));
  if (newest !== null) {
    out.push(circle(newest.x, newest.y, 2.2, p.accent, { opacity: 0.3 }), twinkle(newest.x, newest.y, 2.4, SPARK, 0.95));
    for (const [dx, dy] of [[2.4, -1.6], [-2, -2.2], [1.4, 2], [3.2, 0.6], [-2.8, 1]] as const) out.push(circle(newest.x + dx, newest.y + dy, 0.3, p.accent));
  }
  const light = SHIPYARD_LIGHT_MIN + ((SHIPYARD_LIGHT_MAX - SHIPYARD_LIGHT_MIN) * Math.max(0, Math.min(count, SHIPYARD_LIGHT_FULL))) / SHIPYARD_LIGHT_FULL;
  out.push(poly([[aft, top + 1], [aft + L * 0.9, foot], [aft + L * 0.2, foot]], SHIPYARD_LAMP, "none", { opacity: light * 0.25 }));
  out.push(circle(aft, top + 1, 1.4, SHIPYARD_LAMP, { ...inked(p), opacity: light }));
  return out.join("");
}

// What the painter draws

/**
 * A Starship's picture's inputs: its pieces oldest first, its launch, its design and its livery (`null`: the design's
 * own), and R3335000's engine trail and piece skin (`null` or none: the livery's glow, the modules' gunmetal).
 */
export type StarshipArt = {
  pieces: readonly StarshipPieceRef[];
  launched: boolean;
  design: StarshipDesignId;
  livery: Livery | null;
  radiant: boolean;
  trail?: StarshipTrail | null;
  skin?: ModuleSkin | null;
};

/**
 * R3335000: a Starship's picture's inputs, a launched one's in its seat's `look`: its hull pick's design in place of
 * its name's, its livery, trail and piece skin, each kind's standard (or an id no cosmetic has) keeping what it draws
 * without one. A Dormant one keeps its shipyard whatever the look.
 */
export function starshipArtOf(
  pieces: readonly StarshipPieceRef[],
  launched: boolean,
  name: string | undefined,
  radiant: boolean,
  look: StarshipLook | null = null,
): StarshipArt {
  const art: StarshipArt = { pieces, launched, design: starshipDesignOf(launched, name), livery: null, radiant };
  if (look === null || !launched) return art;
  return {
    ...art,
    design: lookPick(HULL_DESIGNS, look.starshipHull) ?? art.design,
    livery: lookPick(LIVERIES, look.starshipLivery),
    trail: lookPick(TRAILS, look.starshipTrail),
    skin: lookPick(PIECE_SKINS, look.starshipPieceSkin),
  };
}

function keyOf(value: object | null | undefined): string {
  return value === null || value === undefined ? "" : JSON.stringify(value);
}

/** A Starship's picture's memo key: everything it draws from, its pieces by id and face, oldest first. */
export function starshipArtKey(ship: StarshipArt): string {
  const pieces = ship.pieces.map((piece) => `${piece.defId}${piece.radiant ? "*" : ""}`).join(",");
  return `starship|${ship.design}|${keyOf(ship.livery)}|${keyOf(ship.trail)}|${keyOf(ship.skin)}|${ship.launched}|${ship.radiant}|${pieces}`;
}
