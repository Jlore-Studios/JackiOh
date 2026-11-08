// R3403000: Stinkyscape's later scene recipes, part 1 of three (#91 to #119 with #8's, #10's, #11's, #12's, #18's, #84's and #112's tokens), one per entry of
// #3403's table, each drawn from its brief in Stinkyscape's deep-space look (ART.md, "Stinkyscape's later cards drawn
// as scenes"): its subject within x 30 to 70 and y 25 to 85, its foreground near, its middle in the mid and subject
// layers and its background far, richer the rarer. Presentation only (CLAUDE.md rule 7): a recipe is read by the
// card's id alone.

import type { SceneRecipe } from "./scene.ts";

/** Rosemary Triscuit (Stinkyscape #8.1, Token): a weary pilot on a station observation bench bites a rosemary-topped wheat cracker, blue mana sparks rising as her second wind returns, in soft starlight. */
const ROSEMARY_TRISCUIT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 50, y: 34, r: 36, paint: "sky-300", opacity: 0.32 },
  emblem: { x: 70, y: 75.6, size: 5, rotate: 0 },
  figure: {
    x: 48,
    y: 84,
    scale: 0.95,
    pose: "pose-sit",
    look: { head: "head-human", hair: "hair-long", outfit: "outfit-flightsuit", item: null, skin: "wood-700", cloth: "orange-400" },
  },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 26, scale: 2.2 },
      { part: "stars", x: 34, y: 46, scale: 1.3 },
      { part: "nebula", x: 30, y: 32, scale: 1.2, paint: "blue-600" },
      { part: "planet", x: 74, y: 32, scale: 0.9, paint: "blue-200" },
    ],
    far: [{ part: "viewport", x: 50, y: 40, scale: 2.1 }],
    mid: [
      { part: "deck-floor", x: 50, y: 70 },
      { part: "obs-bench", x: 50, y: 84, scale: 3 },
    ],
    near: [
      { part: "cracker", x: 55.5, y: 48.5, scale: 0.9 },
      { part: "cup", x: 32, y: 73.8, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 50, y: 52, scale: 2.2, paint: "sky-300" },
      { part: "rim-light", x: 48, y: 62, scale: 1, paint: "blue-200" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 50, scale: 1.4, count: 3, spread: [10, 10], paint: "sky-300" },
      { part: "motes", x: 50, y: 40, scale: 1.2, paint: "blue-200" },
    ],
  },
};

/** Inner Fire (Stinkyscape #10.1, Token): a battered brawler kneels in a freighter's makeshift boxing ring, bruises glowing white-hot as inner fire flares from her chest into her fists, under a swinging lamp. */
const INNER_FIRE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "wood-900"],
  light: { x: 50, y: 18, r: 34, paint: "orange-400", opacity: 0.42 },
  emblem: { x: 80, y: 44, size: 5, rotate: 0 },
  figure: {
    x: 46,
    y: 82,
    scale: 1,
    pose: "pose-kneel",
    look: { head: "head-human", hair: "headband", outfit: "outfit-sleeveless", item: null, skin: "brass-600", cloth: "red-600" },
  },
  layers: {
    far: [
      { part: "bulkhead", x: 50, y: 74 },
      { part: "crate", x: 12, y: 74, scale: 1.6, count: 2, spread: [5, 0] },
      { part: "crate", x: 88, y: 74, scale: 1.4 },
    ],
    mid: [{ part: "boxing-ring", x: 50, y: 92, scale: 1.5 }],
    near: [
      { part: "flame", x: 47, y: 58, scale: 1.4 },
      { part: "towels", x: 82, y: 86, scale: 1.1 },
    ],
    light: [
      { part: "spotlight", x: 50, y: 4, scale: 1.2, paint: "orange-400" },
      { part: "halo", x: 47, y: 55, scale: 1.4, paint: "orange-400" },
      { part: "halo", x: 63, y: 40, scale: 0.8, paint: "white" },
    ],
    particles: [
      { part: "embers", x: 50, y: 50, scale: 1.4, count: 3, spread: [12, 10] },
      { part: "sparks", x: 63, y: 40, scale: 1, paint: "orange-400" },
    ],
  },
};

/** Divine Spirit (Stinkyscape #10.2, Token): in a cloister orbiting a pale blue planet, a monk-pilot's translucent spirit twin steps into him, his outline swelling twice as solid, in soft halo light. */
const DIVINE_SPIRIT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 50, y: 44, r: 36, paint: "blue-200", opacity: 0.4 },
  emblem: { x: 22, y: 64, size: 5, rotate: 0 },
  figure: {
    x: 48,
    y: 84,
    scale: 1,
    pose: "pose-stand",
    look: { head: "head-human", hair: "tonsure", outfit: "outfit-flightsuit", item: null, skin: "red-300", cloth: "wood-700" },
  },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 22, scale: 2 },
      { part: "planet", x: 72, y: 32, scale: 2.6, paint: "blue-200" },
    ],
    far: [
      { part: "arch", x: 22, y: 72, scale: 1.3, paint: "slate-400" },
      { part: "arch", x: 78, y: 72, scale: 1.3, paint: "slate-400" },
    ],
    mid: [{ part: "deck-floor", x: 50, y: 72, paint: "slate-800" }],
    subject: [{ part: "spirit", x: 54, y: 84, scale: 1.08 }],
    near: [
      { part: "candle", x: 24, y: 90, scale: 1.4 },
      { part: "candle", x: 76, y: 90, scale: 1.4 },
    ],
    light: [
      { part: "halo", x: 48, y: 36, scale: 1.6, paint: "blue-200" },
      { part: "rim-light", x: 48, y: 60, scale: 1.2, paint: "sky-300" },
      { part: "light-shaft", x: 50, y: 10, scale: 1.2, paint: "blue-200" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "sky-300" },
      { part: "sparks", x: 50, y: 46, scale: 1.2, paint: "white" },
    ],
  },
};

/** Power Word Shield (Stinkyscape #10.3, Token): on an exposed hull walkway a chaplain raises his palm and a bubble of pale light snaps around a crewman as micrometeors ricochet away. */
const POWER_WORD_SHIELD: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 62, y: 56, r: 30, paint: "blue-200", opacity: 0.38 },
  emblem: { x: 32, y: 60, size: 5, rotate: 0 },
  figure: {
    x: 32,
    y: 82,
    scale: 0.95,
    pose: "pose-raise",
    look: { head: "head-human", hair: "space-helmet", outfit: "outfit-robe", item: null, skin: "brass-600", cloth: "white" },
  },
  extras: [
    {
      layer: "subject",
      x: 64,
      y: 82,
      scale: 0.8,
      flip: -1,
      pose: "pose-block",
      look: { head: "head-human", hair: "space-helmet", outfit: "outfit-flightsuit", item: null, skin: "red-300", cloth: "orange-400" },
    },
  ],
  layers: {
    sky: [
      { part: "stars", x: 50, y: 24, scale: 2.2 },
      { part: "nebula", x: 76, y: 26, scale: 1.2 },
    ],
    far: [
      { part: "planet", x: 18, y: 30, scale: 1 },
      { part: "asteroid", x: 80, y: 36, scale: 0.6, count: 4, spread: [14, 10] },
    ],
    mid: [{ part: "catwalk", x: 50, y: 82 }],
    near: [{ part: "shield-bubble", x: 64, y: 84, scale: 1.7, paint: "blue-200" }],
    light: [
      { part: "halo", x: 42.5, y: 29, scale: 0.8, paint: "white" },
      { part: "rim-light", x: 64, y: 60, scale: 1.4, paint: "blue-200" },
    ],
    particles: [
      { part: "asteroid", x: 86, y: 44, scale: 0.35, count: 4, spread: [6, 16] },
      { part: "speed-streaks", x: 86, y: 46, scale: 0.6, flip: -1 },
      { part: "sparks", x: 84, y: 52, scale: 1, count: 3, spread: [4, 14], paint: "white" },
    ],
  },
};

/** Frostbolt (Stinkyscape #11.1, Token): a jagged ice shard hurled across a docking bay strikes a charging pirate, who freezes mid-stride as frost spreads over the deck, in icy blue light. */
const FROSTBOLT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "blue-900"],
  light: { x: 54, y: 54, r: 32, paint: "sky-300", opacity: 0.42 },
  emblem: { x: 20, y: 58, size: 5, rotate: 0 },
  figure: {
    x: 58,
    y: 84,
    scale: 1,
    flip: -1,
    pose: "pose-run",
    look: { head: "head-human", hair: "kerchief", outfit: "outfit-vest", item: "item-knife", skin: "wood-700", cloth: "red-900" },
  },
  layers: {
    far: [
      { part: "bulkhead", x: 50, y: 70 },
      { part: "shuttle", x: 20, y: 58, scale: 1.5 },
      { part: "crate", x: 86, y: 70, scale: 1.4, count: 2, spread: [5, 0] },
    ],
    mid: [{ part: "deck-floor", x: 50, y: 70 }],
    near: [
      { part: "frost", x: 58, y: 86, scale: 1.6 },
      { part: "ice-block", x: 58, y: 88, scale: 1.2, fade: 0.75 },
      { part: "ice-shard", x: 46, y: 52, scale: 1.4 },
    ],
    light: [
      { part: "halo", x: 56, y: 50, scale: 1.4, paint: "sky-300" },
      { part: "rim-light", x: 58, y: 60, scale: 1.1, paint: "blue-200" },
    ],
    particles: [
      { part: "shards", x: 56, y: 50, scale: 1.2, count: 2, spread: [6, 6] },
      { part: "snow", x: 50, y: 50, scale: 1.2 },
      { part: "motes", x: 50, y: 70, scale: 1.2, paint: "blue-200" },
    ],
  },
};

/** Arcane Missiles (Stinkyscape #11.2, Token): a young mage at an open airlock looses three wobbly violet bolts that curve off every which way among drifting cargo crates, in a violet glow. */
const ARCANE_MISSILES: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 46, y: 50, r: 34, paint: "violet-500", opacity: 0.42 },
  emblem: { x: 86, y: 80, size: 5, rotate: 0 },
  figure: {
    x: 38,
    y: 86,
    scale: 0.9,
    pose: "pose-cast",
    look: { head: "head-human", hair: "hair-spiky", outfit: "outfit-robe", item: "item-wand", skin: "orange-400", cloth: "violet-700" },
  },
  layers: {
    sky: [{ part: "stars", x: 60, y: 30, scale: 1.6 }],
    far: [{ part: "bulkhead", x: 50, y: 76, paint: "slate-800" }],
    mid: [
      { part: "deck-floor", x: 50, y: 76 },
      { part: "airlock", x: 64, y: 52, scale: 2.2 },
      { part: "crate", x: 76, y: 52, scale: 1.1, count: 3, spread: [12, 14] },
    ],
    near: [
      { part: "magic-bolt", x: 51, y: 55, scale: 1.3, paint: "violet-300" },
      { part: "crate", x: 86, y: 90, scale: 1.6 },
    ],
    light: [
      { part: "halo", x: 51, y: 55, scale: 1, paint: "purple-400" },
      { part: "rim-light", x: 38, y: 64, scale: 1, paint: "violet-300" },
    ],
    particles: [
      { part: "sparks", x: 66, y: 46, scale: 1.2, count: 3, spread: [16, 12], paint: "violet-300" },
      { part: "motes", x: 60, y: 50, scale: 1.4, paint: "purple-400" },
    ],
  },
};

/** The enemy troops Flame Strike scatters. */
const TROOPER = { head: "head-human", hair: "steel-helmet", outfit: "outfit-armor", item: "item-blaster", skin: "red-300", cloth: "slate-700" } as const;

/** Flame Strike (Stinkyscape #11.3, Token): a pillar of orange fire lances down from an orbital mirror onto an enemy camp on a red desert moon, troops scattering, in a scorching glare; the mirror is a relay satellite's dish. */
const FLAME_STRIKE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "red-900"],
  light: { x: 50, y: 72, r: 40, paint: "orange-400", opacity: 0.5 },
  emblem: { x: 30, y: 72, size: 5, rotate: 0 },
  extras: [
    { layer: "mid", x: 20, y: 82, scale: 0.42, flip: -1, pose: "pose-run", look: TROOPER },
    { layer: "mid", x: 76, y: 84, scale: 0.45, pose: "pose-run", look: { ...TROOPER, cloth: "slate-800" } },
    { layer: "near", x: 86, y: 94, scale: 0.55, pose: "pose-run", look: { ...TROOPER, item: null } },
  ],
  layers: {
    sky: [
      { part: "stars", x: 30, y: 22, scale: 1.8 },
      { part: "satellite", x: 50, y: 15, scale: 0.9 },
      { part: "planet", x: 84, y: 26, scale: 1, paint: "orange-400" },
    ],
    far: [
      { part: "regolith", x: 50, y: 68, paint: "red-600" },
      { part: "mesa", x: 18, y: 68, scale: 0.8, paint: "red-900" },
      { part: "mesa", x: 86, y: 68, scale: 0.6, paint: "red-900" },
    ],
    mid: [
      { part: "lamp-tent", x: 30, y: 78, scale: 1.2 },
      { part: "lamp-tent", x: 70, y: 79, scale: 1.3, flip: -1 },
      { part: "crate", x: 84, y: 80, scale: 1.1, count: 2, spread: [4, 0] },
    ],
    subject: [
      { part: "energy-beam", x: 50, y: 25, scale: 0.85, paint: "orange-400" },
      { part: "flames", x: 50, y: 79, scale: 1.6 },
    ],
    near: [{ part: "smoke-billow", x: 36, y: 66, scale: 1 }],
    light: [
      { part: "halo", x: 50, y: 76, scale: 2.4, paint: "orange-400" },
      { part: "light-shaft", x: 50, y: 25, scale: 1.4, paint: "orange-400" },
    ],
    particles: [
      { part: "embers", x: 50, y: 70, scale: 1.6, count: 3, spread: [20, 8] },
      { part: "dust-cloud", x: 62, y: 82, scale: 1.2 },
    ],
  },
};

/** Shadow Word Death (Stinkyscape #12.1, Token): on a foundry station's repair gantry a towering mech buckles as one thread of black shadow pierces its core, a hooded figure below, in a violet gloom. */
const SHADOW_WORD_DEATH: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 54, y: 50, r: 30, paint: "violet-500", opacity: 0.4 },
  emblem: { x: 16, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 32,
    y: 86,
    scale: 0.62,
    pose: "pose-cast",
    look: { head: "head-human", hair: "hood", outfit: "outfit-robe", item: null, skin: "wood-700", cloth: "slate-900" },
  },
  layers: {
    far: [
      { part: "bulkhead", x: 50, y: 80 },
      { part: "gantry", x: 16, y: 80, scale: 1.6 },
      { part: "gantry", x: 84, y: 80, scale: 1.6, flip: -1 },
    ],
    mid: [
      { part: "deck-floor", x: 50, y: 80 },
      { part: "catwalk", x: 50, y: 46 },
    ],
    subject: [{ part: "mech", x: 56, y: 82, scale: 1.3 }],
    near: [{ part: "shadow-tendril", x: 40, y: 64, scale: 0.8 }],
    light: [
      { part: "halo", x: 56, y: 52, scale: 1, paint: "violet-300" },
      { part: "rim-light", x: 56, y: 56, scale: 1.4, paint: "violet-300" },
    ],
    particles: [
      { part: "sparks", x: 56, y: 52, scale: 1.2, count: 2, spread: [4, 4], paint: "violet-300" },
      { part: "embers", x: 30, y: 50, scale: 1.2, count: 2, spread: [20, 10] },
      { part: "motes", x: 50, y: 60, scale: 1.4, paint: "violet-300" },
    ],
  },
};

export const STINKYSCAPE_91_RECIPES_1: Readonly<Record<string, SceneRecipe>> = {
  "stinkyscape-008-1": ROSEMARY_TRISCUIT,
  "stinkyscape-010-1": INNER_FIRE,
  "stinkyscape-010-2": DIVINE_SPIRIT,
  "stinkyscape-010-3": POWER_WORD_SHIELD,
  "stinkyscape-011-1": FROSTBOLT,
  "stinkyscape-011-2": ARCANE_MISSILES,
  "stinkyscape-011-3": FLAME_STRIKE,
  "stinkyscape-012-1": SHADOW_WORD_DEATH,
};
