// R2787000: the standing looks a card's `aura` may wear (docs/polish/9-card-animations.md, Auras), and
// the scopes it may cover. Each look is a tint, a slow drift of motes and an edge glow, drawn in the
// card's palette (FLAVOR_LOOKS); these numbers are the look's motion and weight (CLAUDE.md rule 9).
// R3389000: a signature's own looks beside the shared nine, each worn by one signature alone (the
// loader fails one on any other entry, tables.ts) and each within AURA_SIGNATURE_MAX_INTENSITY.

/** The nine looks, as a card's table entry or an Anomaly's backdrop (`fx/anomaly.ts`, R3337003) names them. */
export const AURA_LOOK_NAMES = ["ember", "frost", "bloom", "void", "storm", "holy", "toxic", "arcane", "drift"] as const;

/** What an aura covers, nearest first: its card's own zone, its lane, its side or the whole board. The index is its stacking rank. */
export const AURA_SCOPES = ["zone", "lane", "side", "board"] as const;

/** The card types that stand in a zone while they hold, so the only ones whose aura may cover their own zone or lane. */
export const AURA_FIELD_TYPES = ["Field Spell", "Field Trap"] as const;

/**
 * R3389000: the looks of Classic's signatures (#3389), each its card's own: Palantir's watching iris,
 * InfiniScepter's loop, Second Wind's breeze over the graveyard, Joro's web, Nature Titan's moss, Spell
 * Tyrant's crown-sigil, BOOM! Big Max's crater smoke, King Wagtoggle's cloak and In Too Deep's whirlpool.
 */
export const AURA_SIGNATURE_LOOK_NAMES = ["seeingIris", "sceptreLoop", "graveBreeze", "silkWeb", "mossCreep", "crownSigil", "craterSmoke", "cloakShimmer", "abyssWhirl"] as const;

/** A shared look, any entry's to wear. */
export type AuraSharedLook = (typeof AURA_LOOK_NAMES)[number];
/** R3389000: a signature's own look. */
export type AuraSignatureLook = (typeof AURA_SIGNATURE_LOOK_NAMES)[number];
export type AuraLook = AuraSharedLook | AuraSignatureLook;
export type AuraScope = (typeof AURA_SCOPES)[number];

export function isAuraLook(name: string): name is AuraLook {
  return (AURA_LOOK_NAMES as readonly string[]).includes(name) || isSignatureAuraLook(name);
}

/** R3389000: whether `name` is a signature's own look. */
export function isSignatureAuraLook(name: string): name is AuraSignatureLook {
  return (AURA_SIGNATURE_LOOK_NAMES as readonly string[]).includes(name);
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

export const AURA_LOOKS: Readonly<Record<AuraSharedLook, AuraLookSpec>> = {
  ember: { vx: [-6, 6], vy: [-28, -12], size: [2, 4], life: [1800, 3200], alpha: 0.8, tint: 0.1, glow: 14, edge: 0.55, blend: L },
  frost: { vx: [-5, 5], vy: [6, 16], size: [2, 3.5], life: [2400, 4000], alpha: 0.7, tint: 0.12, glow: 10, edge: 0.6, blend: L },
  bloom: { vx: [-8, 8], vy: [-10, -2], size: [3, 5], life: [2600, 4200], alpha: 0.6, tint: 0.1, glow: 12, edge: 0.45, blend: S },
  void: { vx: [-10, 10], vy: [-10, 10], size: [3, 6], life: [2000, 3600], alpha: 0.55, tint: 0.16, glow: 18, edge: 0.5, blend: S },
  storm: { vx: [24, 48], vy: [-4, 4], size: [1.5, 3], life: [900, 1600], alpha: 0.75, tint: 0.08, glow: 12, edge: 0.65, blend: L },
  holy: { vx: [-4, 4], vy: [-18, -6], size: [2, 4], life: [2200, 3800], alpha: 0.75, tint: 0.1, glow: 16, edge: 0.6, blend: L },
  toxic: { vx: [-4, 4], vy: [-14, -4], size: [3, 6], life: [2000, 3400], alpha: 0.6, tint: 0.13, glow: 12, edge: 0.5, blend: S },
  arcane: { vx: [-12, 12], vy: [-12, 12], size: [2, 3.5], life: [1600, 3000], alpha: 0.8, tint: 0.09, glow: 14, edge: 0.6, blend: L },
  // R3337003: an asteroid field's slow sideways drift.
  drift: { vx: [4, 10], vy: [-3, 3], size: [3, 7], life: [3600, 6000], alpha: 0.5, tint: 0.06, glow: 10, edge: 0.35, blend: S },
};

/**
 * R3389000: each signature look's motion and weight, every opacity within AURA_SIGNATURE_MAX_INTENSITY
 * (signatureAuras.test.ts), its motion after its card's concept.
 */
export const AURA_SIGNATURE_LOOKS: Readonly<Record<AuraSignatureLook, AuraLookSpec>> = {
  // Palantir: a slow cyan iris drifting to watch the other side, blinking now and then.
  seeingIris: { vx: [-4, 4], vy: [-4, 4], size: [2, 3], life: [3000, 4600], alpha: 0.6, tint: 0.08, glow: 12, edge: 0.5, blend: L },
  // InfiniScepter: the held Spell turning in its loop, a figure-eight trail circling it.
  sceptreLoop: { vx: [-6, 6], vy: [-6, 6], size: [2, 3.5], life: [2200, 3600], alpha: 0.7, tint: 0.1, glow: 14, edge: 0.55, blend: L },
  // Second Wind: a soft breeze ruffling the graveyard, leaf-motes lifting sideways off it.
  graveBreeze: { vx: [10, 22], vy: [-6, -1], size: [2.5, 4.5], life: [1600, 2800], alpha: 0.55, tint: 0.06, glow: 10, edge: 0.4, blend: S },
  // Joro: a faint web in the zone's corners, a thread swaying above him.
  silkWeb: { vx: [-2, 2], vy: [2, 8], size: [1, 2], life: [2800, 4400], alpha: 0.5, tint: 0.05, glow: 8, edge: 0.35, blend: L },
  // Nature Titan: moss creeping slowly over its frame, a petal drifting now and then.
  mossCreep: { vx: [-3, 3], vy: [-8, -2], size: [3, 5.5], life: [3200, 5000], alpha: 0.55, tint: 0.12, glow: 10, edge: 0.45, blend: S },
  // Spell Tyrant: a small violet crown-sigil turning slowly above him.
  crownSigil: { vx: [-6, 6], vy: [-9, -3], size: [2, 3], life: [2000, 3200], alpha: 0.75, tint: 0.09, glow: 16, edge: 0.6, blend: L },
  // BOOM! Big Max: smoke curling up off the crater's rim, embers ticking round his fists.
  craterSmoke: { vx: [-5, 5], vy: [-22, -8], size: [3, 6], life: [2000, 3400], alpha: 0.5, tint: 0.1, glow: 14, edge: 0.5, blend: S },
  // King Wagtoggle: a slow crimson shimmer running along his cloak's hem.
  cloakShimmer: { vx: [8, 16], vy: [-2, 2], size: [1.5, 3], life: [1400, 2400], alpha: 0.65, tint: 0.07, glow: 12, edge: 0.5, blend: L },
  // In Too Deep: a slow whirlpool of black-blue water, a faint light at its bottom.
  abyssWhirl: { vx: [-14, 14], vy: [-14, 14], size: [3, 6], life: [2400, 4000], alpha: 0.5, tint: 0.16, glow: 18, edge: 0.55, blend: S },
};

/** R3389000: any look's spec, shared or a signature's own. */
export function auraLookSpec(look: AuraLook): AuraLookSpec {
  return isSignatureAuraLook(look) ? AURA_SIGNATURE_LOOKS[look] : AURA_LOOKS[look];
}
