// R2787000: the standing looks a card's `aura` may wear (docs/polish/9-card-animations.md, Auras), and
// the scopes it may cover. Each look is a tint, a slow drift of motes and an edge glow, drawn in the
// card's palette (FLAVOR_LOOKS); these numbers are the look's motion and weight (CLAUDE.md rule 9).
// R3400000: a Unit's aura may stand on its own `tile`, its unit zone, the nearest scope; and three looks
// join for the Units whose signatures wear one there: Bob's snow `flurry`, Tutorial Master's `chalk`
// dust and the Training-Grounds Golem's `timber` dust.

/** The eleven looks, as a card's table entry names them. */
export const AURA_LOOK_NAMES = ["ember", "frost", "bloom", "void", "storm", "holy", "toxic", "arcane", "flurry", "chalk", "timber"] as const;

/** What an aura covers, nearest first: a Unit's own tile, its card's own zone, its lane, its side or the whole board. The index is its stacking rank. */
export const AURA_SCOPES = ["tile", "zone", "lane", "side", "board"] as const;

/** The card types that stand in a zone while they hold, so the only ones whose aura may cover their own zone or lane. */
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
  flurry: { vx: [-8, 8], vy: [4, 12], size: [1.5, 3], life: [2600, 4200], alpha: 0.55, tint: 0.06, glow: 8, edge: 0.35, blend: L },
  chalk: { vx: [-6, 6], vy: [-8, -2], size: [1.5, 3], life: [2400, 4000], alpha: 0.45, tint: 0.05, glow: 6, edge: 0.3, blend: S },
  timber: { vx: [-3, 3], vy: [6, 14], size: [1.5, 3], life: [2000, 3400], alpha: 0.4, tint: 0.07, glow: 8, edge: 0.4, blend: S },
};
