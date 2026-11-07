// R2787000: the standing looks a card's `aura` may wear (docs/polish/9-card-animations.md, Auras), and
// the scopes it may cover. Each look is a tint, a slow drift of motes and an edge glow, drawn in the
// card's palette (FLAVOR_LOOKS); these numbers are the look's motion and weight (CLAUDE.md rule 9).
// R3407000: the signature looks are one card's each (Stinkyscape part two's "on its tile"), within
// AURA_SIGNATURE_MAX_INTENSITY in their opacities; the shared eight are any card's.

/** The eight shared looks, as a card's table entry names them. */
export const AURA_SHARED_LOOK_NAMES = ["ember", "frost", "bloom", "void", "storm", "holy", "toxic", "arcane"] as const;

/** R3407000: the signature looks, each one card's own standing look. */
export const AURA_SIGNATURE_LOOK_NAMES = [
  "scanline",
  "heartbeat",
  "glint",
  "pixelcrawl",
  "duohalo",
  "arcflick",
  "steplights",
  "searchlight",
  "stationrings",
  "aiwisp",
  "crownpulse",
  "drumbeat",
  "starpath",
  "stinkvapour",
  "pages",
] as const;

/** Every look a table entry may name. */
export const AURA_LOOK_NAMES = [...AURA_SHARED_LOOK_NAMES, ...AURA_SIGNATURE_LOOK_NAMES] as const;

/** What an aura covers, nearest first: its card's own zone, its lane, its side or the whole board. The index is its stacking rank. */
export const AURA_SCOPES = ["zone", "lane", "side", "board"] as const;

/** The card types that stand in a zone while they hold, so the only ones whose aura may cover their own lane; R3407000: a Unit's may cover its own zone too. */
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
  // R3407000: MELCHIOR's scan line sweeping its zone, counting.
  scanline: { vx: [30, 50], vy: [-1, 1], size: [1.5, 2.5], life: [800, 1400], alpha: 0.7, tint: 0.08, glow: 10, edge: 0.55, blend: L },
  // BALTHASAR's gentle heartbeat pulsing along its zone's edge.
  heartbeat: { vx: [-3, 3], vy: [-3, 3], size: [3, 5], life: [900, 1300], alpha: 0.65, tint: 0.1, glow: 14, edge: 0.6, blend: L },
  // CASPER's rose glint drifting across a mirror sheen.
  glint: { vx: [14, 24], vy: [-6, -2], size: [2, 3], life: [1400, 2400], alpha: 0.75, tint: 0.07, glow: 12, edge: 0.5, blend: L },
  // xxFreshJack's pixel shimmer crawling up his edge, like an unstable patch.
  pixelcrawl: { vx: [-2, 2], vy: [-30, -16], size: [2, 2.5], life: [700, 1300], alpha: 0.7, tint: 0.06, glow: 8, edge: 0.5, blend: L },
  // iKwzki's slow halo of motes.
  duohalo: { vx: [-6, 6], vy: [-6, 6], size: [2.5, 4], life: [3000, 4600], alpha: 0.65, tint: 0.08, glow: 16, edge: 0.55, blend: L },
  // Shockwave447's thin arcs flicking between his keywords.
  arcflick: { vx: [-40, 40], vy: [-30, 30], size: [1, 2], life: [250, 600], alpha: 0.8, tint: 0.06, glow: 12, edge: 0.65, blend: L },
  // Job Application's step lights over its door, blinking softly where they stand.
  steplights: { vx: [0, 0], vy: [0, 0], size: [3, 4], life: [1200, 1800], alpha: 0.7, tint: 0.08, glow: 10, edge: 0.5, blend: L },
  // Scornboi's slow crimson searchlight sweeping out from his zone.
  searchlight: { vx: [18, 30], vy: [-2, 2], size: [4, 7], life: [2000, 3000], alpha: 0.5, tint: 0.12, glow: 18, edge: 0.6, blend: S },
  // Megabase's slow-turning rings, its shuttles circling and its windows twinkling.
  stationrings: { vx: [-8, 8], vy: [-3, 3], size: [1.5, 3], life: [2400, 4000], alpha: 0.7, tint: 0.1, glow: 14, edge: 0.55, blend: L },
  // IN33DH3's AI wisp orbiting his head, blinking as it thinks.
  aiwisp: { vx: [-14, 14], vy: [-14, 4], size: [2.5, 4], life: [1000, 1800], alpha: 0.75, tint: 0.07, glow: 12, edge: 0.5, blend: L },
  // IN33DH3 Prime's slow pulse circling his neural crown.
  crownpulse: { vx: [-5, 5], vy: [-12, -4], size: [2, 3.5], life: [1600, 2600], alpha: 0.7, tint: 0.08, glow: 16, edge: 0.6, blend: L },
  // Wardrum Blob's soft wobble in time to a slow drumbeat.
  drumbeat: { vx: [-4, 4], vy: [-8, 8], size: [4, 7], life: [800, 1000], alpha: 0.6, tint: 0.12, glow: 12, edge: 0.5, blend: S },
  // Alteran Reach's faint star path across its zone.
  starpath: { vx: [6, 12], vy: [-2, 2], size: [1.5, 3], life: [2600, 4200], alpha: 0.75, tint: 0.1, glow: 12, edge: 0.55, blend: L },
  // Stinkyscape Golem's lazy green stink vapour curling off its stones.
  stinkvapour: { vx: [-6, 6], vy: [-16, -6], size: [5, 9], life: [2400, 3800], alpha: 0.5, tint: 0.12, glow: 12, edge: 0.45, blend: S },
  // Sanctum's silver motes drifting up like turning pages.
  pages: { vx: [-6, 6], vy: [-20, -8], size: [2, 3.5], life: [2200, 3600], alpha: 0.7, tint: 0.08, glow: 14, edge: 0.55, blend: L },
};
