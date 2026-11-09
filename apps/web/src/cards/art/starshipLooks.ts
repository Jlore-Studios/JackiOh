// R3335000 (#3335): the Starship cosmetics a seat wears, account-wide, one pick of each of four kinds: a hull design,
// a livery, an engine trail and a piece skin (a recolour of every module it mounts). Each kind's standard pick
// changes nothing, so a Starship whose seat picked none draws as it always has. They are the painter's inputs
// (`starshipArtOf`, starshipHulls.ts) and only a launched Starship wears them: a Dormant one keeps its shipyard.
// Presentation only (CLAUDE.md rule 7). No colour here is in Radiant's gold (R2725004); a Radiant face gilds them.

import type { Livery, StarshipDesignId } from "./starshipHulls.ts";

/** A seat's Starship cosmetics, by id, one of each kind. */
export type StarshipLook = { starshipHull: string; starshipLivery: string; starshipTrail: string; starshipPieceSkin: string };

/** An engine trail's colours: its burn, the burn's hot core and the warp streaks it leaves. */
export type StarshipTrail = { burn: string; core: string; streak: string };

/** A piece skin: the tones it lays over every module's plating (`modulePaint`), before a Radiant face gilds them. */
export type ModuleSkin = { hull: string; dark: string; light: string };

export const STARSHIP_HULL_STANDARD = "starship-hull-standard";
export const STARSHIP_LIVERY_STANDARD = "starship-livery-standard";
export const STARSHIP_TRAIL_STANDARD = "starship-trail-standard";
export const STARSHIP_PIECE_SKIN_STANDARD = "starship-piece-skin-standard";

/** The look a seat that picked nothing wears: each kind's standard, which keeps what the Starship draws. */
export const STARSHIP_LOOK_STANDARD: StarshipLook = {
  starshipHull: STARSHIP_HULL_STANDARD,
  starshipLivery: STARSHIP_LIVERY_STANDARD,
  starshipTrail: STARSHIP_TRAIL_STANDARD,
  starshipPieceSkin: STARSHIP_PIECE_SKIN_STANDARD,
};

/** Each hull design cosmetic's design (`STARSHIP_DESIGNS`); the standard keeps the one its name gave it. */
export const HULL_DESIGNS: Readonly<Record<string, StarshipDesignId | null>> = {
  [STARSHIP_HULL_STANDARD]: null,
  "starship-hull-scout": "scout",
  "starship-hull-courier": "courier",
  "starship-hull-freighter": "freighter",
  "starship-hull-corsair": "corsair",
  "starship-hull-cathedral": "cathedral",
  "starship-hull-leviathan": "leviathan",
  "starship-hull-ark": "ark",
  "starship-hull-stinkyscape": "stinkyscape",
  "starship-hull-jlorious": "jlorious",
};

/** Each livery cosmetic's colours, in the Stinkyscape set's space; the standard keeps its design's own. */
export const LIVERIES: Readonly<Record<string, Livery | null>> = {
  [STARSHIP_LIVERY_STANDARD]: null,
  // Gunmetal under safety lime and warning orange.
  "starship-livery-hazard": { hull: "#34373d", trim: "#dfee2a", stripe: "#ff5a1a", window: "#f0ff9a", glow: "#ff7a2a" },
  // A violet cloud lit magenta.
  "starship-livery-nebula": { hull: "#3a1f5c", trim: "#e48aff", stripe: "#ff4fb8", window: "#c8b0ff", glow: "#b46aff" },
  // Oxidised plating, its stripe a weathered slate.
  "starship-livery-rust": { hull: "#8a3f22", trim: "#c8865a", stripe: "#3a4a4f", window: "#ff9a6a", glow: "#ff6a3a" },
  // Frost white under ice blue.
  "starship-livery-arctic": { hull: "#dbe8f2", trim: "#5a8ab0", stripe: "#2ac0e8", window: "#e6fbff", glow: "#9ae8ff" },
  // Navy all but black, running dark but for its blue.
  "starship-livery-midnight": { hull: "#141a33", trim: "#3a4a7a", stripe: "#5a6aff", window: "#8ab0ff", glow: "#4a6aff" },
  // A reef's coral and its lagoon's teal.
  "starship-livery-coral": { hull: "#ff7f6a", trim: "#fff0e8", stripe: "#1fa8a0", window: "#bffff2", glow: "#ff9ab0" },
  // A star's last blaze: red hot, white trim, a violet shock front.
  "starship-livery-supernova": { hull: "#e8463a", trim: "#fff2f0", stripe: "#7a2aff", window: "#fff0ff", glow: "#ff6ad0" },
  // An admiral's royal blue, silver and crimson.
  "starship-livery-flagship": { hull: "#1f3a8a", trim: "#e6ecf5", stripe: "#c8143a", window: "#bfe0ff", glow: "#7ab8ff" },
};

/** Each engine trail cosmetic's colours; the standard keeps the livery's glow and its white-hot core. */
export const TRAILS: Readonly<Record<string, StarshipTrail | null>> = {
  [STARSHIP_TRAIL_STANDARD]: null,
  "starship-trail-ion": { burn: "#4aa8ff", core: "#e6f6ff", streak: "#9ad0ff" },
  "starship-trail-plasma": { burn: "#c84aff", core: "#ffe6ff", streak: "#e6a0ff" },
  "starship-trail-comet": { burn: "#bfefff", core: "#ffffff", streak: "#7ad8ff" },
  "starship-trail-stardust": { burn: "#ff9ad8", core: "#fff4fb", streak: "#f0c8ff" },
  "starship-trail-aurora": { burn: "#2aff8a", core: "#e0fff0", streak: "#b07aff" },
  "starship-trail-prism": { burn: "#5affff", core: "#ffffff", streak: "#ff6ad8" },
  // A burn that swallows its light: a black core in a violet flare.
  "starship-trail-singularity": { burn: "#6a2aff", core: "#0a0418", streak: "#8a6aff" },
};

/** What the cosmetic `id` of one kind's `record` draws: `null`, as the standard does, for an id the record lacks. */
export function lookPick<T>(record: Readonly<Record<string, T | null>>, id: string): T | null {
  return Object.hasOwn(record, id) ? (record[id] ?? null) : null;
}

/** Each piece skin cosmetic's tones; the standard keeps the modules' gunmetal (`MODULE_PAINT`). */
export const PIECE_SKINS: Readonly<Record<string, ModuleSkin | null>> = {
  [STARSHIP_PIECE_SKIN_STANDARD]: null,
  "starship-piece-skin-gunmetal": { hull: "#4a525e", dark: "#1a1e24", light: "#8a949e" },
  "starship-piece-skin-verdigris": { hull: "#3f8a78", dark: "#1a3a32", light: "#8fd0bc" },
  "starship-piece-skin-neon": { hull: "#2a1a4a", dark: "#120a24", light: "#4affe8" },
  "starship-piece-skin-chrome": { hull: "#b8c2cc", dark: "#5a6470", light: "#f4f8fc" },
};
