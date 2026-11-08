// R3342000: numbers that govern the tile aura (issue #3342, CLAUDE.md rule 9).
// A soft edge glow and motes for Legendary; a richer pattern for Mythic.

export const TILE_MOTES_LEGENDARY = 3;
export const TILE_MOTES_MYTHIC = 5;
export const TILE_PARTICLE_SHARE = 0.2;

export const TILE_HUE_SPREAD_DEG = 18;
export const TILE_VARIANTS = 12;

export const TILE_GLOW_PX = { Legendary: 8, Mythic: 12 } as const;
export const TILE_EDGE = { Legendary: 0.45, Mythic: 0.6 } as const;

export const TILE_PATTERN_OPACITY = 0.35;
export const TILE_DIM = 0.3;
export const TILE_LIT_BOOST = 0.4;

export const TILE_PULSE_MS = 6000;
export const TILE_RIPPLE_MS = 700;
export const TILE_FLICKER_MS = 450;
export const TILE_FADE_OUT_MS = 350;

export const TILE_TOUCH_GAP_MS = 250;
export const TILE_MOTE_TRAVEL_PX = 24;
