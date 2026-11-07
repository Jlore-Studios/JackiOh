// R2787000: the standing looks a card's `aura` may wear (docs/polish/9-card-animations.md, Auras), and
// the scopes it may cover. Each look is a tint, a slow drift of motes and an edge glow, drawn in the
// card's palette (FLAVOR_LOOKS); these numbers are the look's motion and weight (CLAUDE.md rule 9).
// R3399001: Classic+'s signatures (#3399) add one look of its own per permanent, after the eight shared
// ones, each moving as its card's concept does (syrup creeping down, steam rising, a banner in a breeze),
// every mote alpha, tint and edge glow within AURA_SIGNATURE_MAX_INTENSITY (R3332000).

/** The eight shared looks any card's table entry may name. */
export const AURA_SHARED_LOOK_NAMES = ["ember", "frost", "bloom", "void", "storm", "holy", "toxic", "arcane"] as const;

/** R3399001: Classic+'s signature looks, one per permanent, in catalog order. */
export const AURA_SIGNATURE_LOOK_NAMES = [
  "syrup",
  "waffleDome",
  "frostMist",
  "ovenSteam",
  "laneWall",
  "snoreLeaves",
  "coinGlint",
  "staffBreath",
  "bowNotch",
  "warHeartbeat",
  "appleRibbon",
  "bannerBreeze",
  "greatBannerShade",
  "laserDot",
  "stoneFaces",
  "stairCracks",
  "jungleLeaves",
  "serverLights",
] as const;

/** Every look, as a card's table entry names them: the shared ones, then the signature ones. */
export const AURA_LOOK_NAMES = [...AURA_SHARED_LOOK_NAMES, ...AURA_SIGNATURE_LOOK_NAMES] as const;

/** What an aura covers, nearest first: its card's own zone, its lane, its side or the whole board. The index is its stacking rank. */
export const AURA_SCOPES = ["zone", "lane", "side", "board"] as const;

/** The card types that stand in a backrow zone while they hold, so the only ones whose aura may cover their own zone or lane (R3399001: a Unit's may cover its own unit zone). */
export const AURA_FIELD_TYPES = ["Field Spell", "Field Trap"] as const;

export type AuraLook = (typeof AURA_LOOK_NAMES)[number];
export type AuraScope = (typeof AURA_SCOPES)[number];

export function isAuraLook(name: string): name is AuraLook {
  return (AURA_LOOK_NAMES as readonly string[]).includes(name);
}

export function isAuraScope(name: string): name is AuraScope {
  return (AURA_SCOPES as readonly string[]).includes(name);
}

export function isAuraFieldType(type: string): boolean {
  return (AURA_FIELD_TYPES as readonly string[]).includes(type);
}

/**
 * One look. `vx` and `vy` are a mote's velocity range in px/s (a negative `vy` rises), `size` its
 * diameter and `glow` the edge glow's spread in px, `life` a mote's lifetime in ms; `alpha` (a mote's
 * peak), `tint` (the body's fill) and `edge` (the glow) are opacities; `blend` is how motes composite.
 */
export type AuraLookSpec = {
  vx: readonly [number, number];
  vy: readonly [number, number];
  size: readonly [number, number];
  life: readonly [number, number];
  alpha: number;
  tint: number;
  glow: number;
  edge: number;
  blend: "lighter" | "source-over";
};

const L = "lighter";
const S = "source-over";

export const AURA_LOOKS: Readonly<Record<AuraLook, AuraLookSpec>> = {
  ember: { vx: [-6, 6], vy: [-28, -12], size: [2, 4], life: [1800, 3200], alpha: 0.8, tint: 0.1, glow: 14, edge: 0.55, blend: L },
  frost: { vx: [-5, 5], vy: [6, 16], size: [2, 3.5], life: [2400, 4000], alpha: 0.7, tint: 0.12, glow: 10, edge: 0.6, blend: L },
  bloom: { vx: [-8, 8], vy: [-10, -2], size: [3, 5], life: [2600, 4200], alpha: 0.6, tint: 0.1, glow: 12, edge: 0.45, blend: S },
  void: { vx: [-10, 10], vy: [-10, 10], size: [3, 6], life: [2000, 3600], alpha: 0.55, tint: 0.16, glow: 18, edge: 0.5, blend: S },
  storm: { vx: [24, 48], vy: [-4, 4], size: [1.5, 3], life: [900, 1600], alpha: 0.75, tint: 0.08, glow: 12, edge: 0.65, blend: L },
  holy: { vx: [-4, 4], vy: [-18, -6], size: [2, 4], life: [2200, 3800], alpha: 0.75, tint: 0.1, glow: 16, edge: 0.6, blend: L },
  toxic: { vx: [-4, 4], vy: [-14, -4], size: [3, 6], life: [2000, 3400], alpha: 0.6, tint: 0.13, glow: 12, edge: 0.5, blend: S },
  arcane: { vx: [-12, 12], vy: [-12, 12], size: [2, 3.5], life: [1600, 3000], alpha: 0.8, tint: 0.09, glow: 14, edge: 0.6, blend: L },
  // R3399001: Classic+'s signature looks, one per permanent, in catalog order.
  /** The Mother Pancake: syrup creeping slowly down her sides, a thin curl of steam among it. */
  syrup: { vx: [-3, 3], vy: [4, 10], size: [3, 5], life: [3000, 4600], alpha: 0.6, tint: 0.12, glow: 10, edge: 0.45, blend: S },
  /** Anti-Waffle Shell: a slow pale blue waffle-grid shimmer over its side, its cells lighting in turn. */
  waffleDome: { vx: [-1, 1], vy: [-1, 1], size: [3, 4.5], life: [2200, 3400], alpha: 0.6, tint: 0.1, glow: 16, edge: 0.65, blend: L },
  /** Frostspatula: a thin cold mist curling off its blade, frost creeping out and back across its griddle. */
  frostMist: { vx: [-7, 7], vy: [2, 8], size: [3, 6], life: [2800, 4400], alpha: 0.5, tint: 0.14, glow: 14, edge: 0.55, blend: S },
  /** Mommy Barker: a flour mote off her apron and a wisp of oven steam rising behind her. */
  ovenSteam: { vx: [-5, 5], vy: [-16, -8], size: [2.5, 4.5], life: [2400, 3800], alpha: 0.55, tint: 0.08, glow: 12, edge: 0.4, blend: S },
  /** Top Loser: a faint grey lane wall around his zone, his cape stirring in a lonely wind. */
  laneWall: { vx: [-2, 2], vy: [-24, -14], size: [1.5, 2.5], life: [1400, 2400], alpha: 0.45, tint: 0.16, glow: 20, edge: 0.75, blend: S },
  /** Jungle Loser: slow rustling leaves around him and a snore bubble that swells and pops. */
  snoreLeaves: { vx: [-4, 4], vy: [-8, -2], size: [4, 7], life: [3200, 5000], alpha: 0.5, tint: 0.08, glow: 10, edge: 0.4, blend: S },
  /** Mid Loser: his coin rolling over his knuckles, half his aura bright and half dull. */
  coinGlint: { vx: [-14, 14], vy: [-6, 0], size: [1.5, 3], life: [1200, 2200], alpha: 0.8, tint: 0.08, glow: 12, edge: 0.55, blend: L },
  /** Support Loser: his Divine Shield bubble breathing slowly, a faint white glow on his staff. */
  staffBreath: { vx: [-3, 3], vy: [-10, -4], size: [3, 5], life: [3400, 5200], alpha: 0.6, tint: 0.11, glow: 18, edge: 0.6, blend: L },
  /** Bot Loser: notches glowing on his bow and a cocky flex that loops slowly. */
  bowNotch: { vx: [-6, 6], vy: [-14, -6], size: [1.5, 3], life: [1600, 2600], alpha: 0.75, tint: 0.1, glow: 12, edge: 0.6, blend: L },
  /** Wardrum: a slow red-orange heartbeat ring from his chest, motes thrown outward on each beat. */
  warHeartbeat: { vx: [-18, 18], vy: [-18, 18], size: [2, 3.5], life: [900, 1500], alpha: 0.7, tint: 0.14, glow: 18, edge: 0.7, blend: L },
  /** KY's Gift: an apple glinting on its desk and ribbons fluttering on the boxes. */
  appleRibbon: { vx: [-8, 8], vy: [-6, 4], size: [2, 3.5], life: [2000, 3200], alpha: 0.7, tint: 0.09, glow: 10, edge: 0.5, blend: L },
  /** Felinor Flagbearer: his banner rippling in a light breeze, a soft red glow beside him. */
  bannerBreeze: { vx: [10, 22], vy: [-4, 2], size: [2, 3.5], life: [1800, 3000], alpha: 0.65, tint: 0.1, glow: 14, edge: 0.55, blend: L },
  /** Felinor Flagbearer Prime: the great banner swaying over him, its shadow drifting across his side. */
  greatBannerShade: { vx: [6, 14], vy: [-2, 2], size: [4, 7], life: [3200, 5000], alpha: 0.45, tint: 0.15, glow: 22, edge: 0.5, blend: S },
  /** Jlockeed's Lobbyist: a slow red laser dot wandering over his briefcase, in steel blue. */
  laserDot: { vx: [-9, 9], vy: [-6, 6], size: [1.5, 2.5], life: [2600, 4200], alpha: 0.8, tint: 0.07, glow: 8, edge: 0.5, blend: L },
  /** Classic Golem: carved faces sliding across his chest, dust trickling from his joints. */
  stoneFaces: { vx: [-2, 2], vy: [10, 20], size: [1.5, 3], life: [1800, 3000], alpha: 0.55, tint: 0.12, glow: 8, edge: 0.4, blend: S },
  /** Twice Forward One Step Backwards: its stair-stones shuffling two up and one back, cracks pulsing in the void. */
  stairCracks: { vx: [-4, 4], vy: [-12, 6], size: [2.5, 5], life: [2000, 3400], alpha: 0.6, tint: 0.16, glow: 18, edge: 0.6, blend: S },
  /** J-lease J-Jungle EX-plorer: a few leaves drifting down and a vine swaying behind her. */
  jungleLeaves: { vx: [-10, 10], vy: [8, 18], size: [3, 5], life: [2600, 4000], alpha: 0.6, tint: 0.09, glow: 10, edge: 0.45, blend: S },
  /** Claude's Datacenter: blinking status lights and a gentle heat shimmer above its racks. */
  serverLights: { vx: [-1, 1], vy: [-12, -6], size: [1.5, 2.5], life: [700, 1300], alpha: 0.8, tint: 0.1, glow: 12, edge: 0.6, blend: L },
};
