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
      { part: "cracker", x: 56, y: 50, scale: 1.2 },
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
    y: 85,
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
      { part: "crate", x: 84, y: 80, scale: 1.1 },
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
      { part: "embers", x: 50, y: 70, scale: 1.6, count: 2, spread: [20, 8] },
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
    y: 85,
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
    mid: [{ part: "deck-floor", x: 50, y: 80 }],
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

/** Shadow Word Pain (Stinkyscape #12.2, Token): in a cramped ventilation crawlspace a tiny maintenance drone withers under a wisp of black smoke curling from a whispering mechanic's fingertip, under one flickering lamp. */
const SHADOW_WORD_PAIN: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 64, y: 20, r: 26, paint: "orange-400", opacity: 0.3 },
  emblem: { x: 30, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 32,
    y: 84,
    scale: 0.95,
    pose: "pose-kneel",
    look: { head: "head-human", hair: "beanie", outfit: "outfit-overalls", item: null, skin: "wood-700", cloth: "blue-800" },
  },
  layers: {
    far: [{ part: "vent-duct", x: 60, y: 44, scale: 1.2 }],
    mid: [
      { part: "lantern", x: 64, y: 6, scale: 1.2 },
      { part: "pipes", x: 84, y: 80, scale: 0.8, flip: -1 },
    ],
    subject: [{ part: "drone", x: 62, y: 60, scale: 1.4, paint: "slate-400" }],
    near: [
      { part: "shadow-tendril", x: 48, y: 44, scale: 0.55 },
      { part: "smoke", x: 62, y: 56, scale: 0.8 },
    ],
    light: [
      { part: "halo", x: 64, y: 14, scale: 1.2, paint: "orange-400" },
      { part: "halo", x: 62, y: 60, scale: 0.9, paint: "violet-500" },
      { part: "rim-light", x: 32, y: 62, scale: 1, paint: "orange-400" },
    ],
    particles: [
      { part: "sparks", x: 62, y: 62, scale: 1, count: 2, spread: [5, 4], paint: "orange-400" },
      { part: "motes", x: 50, y: 50, scale: 1.2, paint: "slate-400" },
    ],
  },
};

/** The bridge crew Psychic Scream sweeps up. */
const BRIDGE_CREW = { head: "head-human", hair: "hair-short", outfit: "outfit-jacket", item: null, skin: "brass-600", cloth: "blue-800" } as const;

/** Psychic Scream (Stinkyscape #12.3, Token): on a crowded station bridge a shrieking psychic's violet shockwave sweeps crew and robots up in a spiral of light toward a distant enemy cruiser. */
const PSYCHIC_SCREAM: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 50, y: 54, r: 40, paint: "violet-500", opacity: 0.45 },
  emblem: { x: 50, y: 54, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 85,
    scale: 0.9,
    pose: "pose-spread",
    look: { head: "head-human", hair: "hair-long", outfit: "outfit-robe", item: null, skin: "red-300", cloth: "violet-500" },
  },
  extras: [
    { layer: "mid", x: 24, y: 60, scale: 0.42, flip: -1, pose: "pose-spread", look: BRIDGE_CREW, fade: 0.85 },
    { layer: "mid", x: 77, y: 48, scale: 0.36, pose: "pose-run", look: { ...BRIDGE_CREW, hair: "visor", skin: "wood-700", cloth: "red-600" }, fade: 0.8 },
  ],
  layers: {
    sky: [
      { part: "stars", x: 50, y: 28, scale: 1.6 },
      { part: "starship", x: 68, y: 30, scale: 0.3, flip: -1, paint: "red-900" },
    ],
    far: [{ part: "viewport", x: 50, y: 34, scale: 2.1 }],
    mid: [{ part: "tile-floor", x: 50, y: 72, paint: "slate-800" }],
    subject: [
      { part: "robot", x: 32, y: 44, scale: 0.8 },
      { part: "robot", x: 68, y: 64, scale: 0.7, flip: -1 },
    ],
    near: [{ part: "crew-crowd", x: 50, y: 100, scale: 1.4 }],
    light: [
      { part: "shockwave", x: 50, y: 62, scale: 1.4, paint: "violet-300" },
      { part: "light-ribbon", x: 56, y: 42, scale: 0.9, paint: "purple-400" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 46, scale: 1.2, count: 3, spread: [22, 12], paint: "violet-300" },
      { part: "motes", x: 56, y: 40, scale: 1.4, paint: "purple-400" },
    ],
  },
};

/** MELCHIOR (Stinkyscape #18.1, Token): a sterile research deck, a cyan AI core column ringed by holographic tactical tables, tiny unit figures hovering over them to be counted, in a clinical white light; the units are the kit's small soldiers, faded to holograms. */
const MELCHIOR: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-400", "slate-100"],
  light: { x: 50, y: 44, r: 38, paint: "cyan-400", opacity: 0.35 },
  emblem: { x: 50, y: 80, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "tile-wall", x: 50, y: 72 },
      { part: "ceiling-lights", x: 50, y: 4, scale: 1.1 },
    ],
    mid: [
      { part: "holo-table", x: 22, y: 84, scale: 1.2 },
      { part: "holo-table", x: 78, y: 84, scale: 1.2, flip: -1 },
    ],
    subject: [{ part: "ai-core", x: 50, y: 86, scale: 1.6, paint: "cyan-400" }],
    near: [
      { part: "soldier", x: 22, y: 70, scale: 0.4, count: 3, spread: [5, 1], paint: "cyan-400", fade: 0.7 },
      { part: "soldier", x: 78, y: 70, scale: 0.4, flip: -1, count: 3, spread: [5, 1], paint: "sky-300", fade: 0.7 },
    ],
    light: [
      { part: "halo", x: 50, y: 56, scale: 2, paint: "cyan-400" },
      { part: "light-shaft", x: 50, y: 4, scale: 1.2, paint: "white" },
    ],
    particles: [
      { part: "motes", x: 50, y: 56, scale: 1.4, paint: "cyan-400" },
      { part: "sparks", x: 50, y: 60, scale: 1, count: 2, spread: [16, 8], paint: "white" },
    ],
  },
};

/** BALTHASAR (Stinkyscape #18.2, Token): a soft-lit station nursery, a rose and teal AI core cradled in cables, heart-monitor holograms pulsing over empty cots; the knitted blanket is the cots' own, draped over the nearer one. */
const BALTHASAR: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 54, r: 34, paint: "red-300", opacity: 0.35 },
  emblem: { x: 50, y: 80, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "bulkhead", x: 50, y: 74, paint: "slate-700" },
      { part: "porthole", x: 22, y: 36, scale: 1.2 },
      { part: "porthole", x: 78, y: 36, scale: 1.2 },
    ],
    mid: [
      { part: "cot", x: 18, y: 88, scale: 1.4, paint: "red-300" },
      { part: "cot", x: 82, y: 88, scale: 1.4, flip: -1, paint: "jade-400" },
      { part: "hologram", x: 18, y: 64, scale: 0.45, paint: "jade-400", fade: 0.8 },
      { part: "hologram", x: 82, y: 64, scale: 0.45, paint: "jade-400", fade: 0.8 },
    ],
    subject: [{ part: "ai-core", x: 50, y: 86, scale: 1.4, paint: "red-300" }],
    near: [
      { part: "cable", x: 30, y: 94, scale: 1.1 },
      { part: "cable", x: 70, y: 94, scale: 1.1, flip: -1 },
    ],
    light: [
      { part: "halo", x: 50, y: 58, scale: 2, paint: "red-300" },
      { part: "heal-glow", x: 18, y: 62, scale: 0.8, paint: "jade-400" },
      { part: "heal-glow", x: 82, y: 62, scale: 0.8, paint: "jade-400" },
    ],
    particles: [
      { part: "motes", x: 50, y: 56, scale: 1.4, paint: "red-300" },
      { part: "sparks", x: 50, y: 60, scale: 1, count: 2, spread: [14, 8], paint: "cyan-400" },
    ],
  },
};

/** CASPER (Stinkyscape #18.3, Token): a mirrored boudoir cabin, a violet AI core behind a vanity glass, an unspent mana crystal on a silver tray, a charmed enemy drone hovering; the tray is a small silver stand, the charm the drone's drifting hearts. */
const CASPER: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 50, y: 46, r: 36, paint: "violet-500", opacity: 0.4 },
  emblem: { x: 34, y: 80, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "drapes", x: 50, y: 6, scale: 1.3, paint: "violet-700" },
      { part: "ai-core", x: 50, y: 76, scale: 1.3, paint: "violet-500" },
    ],
    mid: [
      { part: "glass-wall", x: 50, y: 80, scale: 1, paint: "violet-300" },
      { part: "mirror", x: 12, y: 88, scale: 1.4, paint: "slate-100" },
      { part: "mirror", x: 88, y: 88, scale: 1.4, paint: "slate-100" },
    ],
    subject: [
      { part: "drone", x: 64, y: 46, scale: 1.3, paint: "red-600" },
      { part: "ink-hearts", x: 64, y: 38, scale: 0.9 },
    ],
    near: [
      { part: "wide-table", x: 40, y: 102, scale: 1.4, paint: "violet-700" },
      { part: "pedestal", x: 34, y: 90, scale: 0.7, paint: "slate-100" },
      { part: "crystal", x: 34, y: 83, scale: 1.3, paint: "sky-300" },
    ],
    light: [
      { part: "halo", x: 50, y: 52, scale: 1.8, paint: "violet-300" },
      { part: "halo", x: 34, y: 80, scale: 0.8, paint: "sky-300" },
      { part: "chandelier", x: 50, y: 4, scale: 0.9 },
    ],
    particles: [
      { part: "sparks", x: 50, y: 50, scale: 1, count: 2, spread: [14, 10], paint: "violet-300" },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "purple-400" },
    ],
  },
};

/** Metal Scraps (Stinkyscape #84.1, Token): a salvager's gloved hand pulls a glowing ore chunk from tumbling hull wreckage in a ring of orbital debris, loose bolts drifting, under a harsh white sun. */
const METAL_SCRAPS: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 20, y: 18, r: 32, paint: "white", opacity: 0.32 },
  emblem: { x: 46, y: 46, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 56, y: 26, scale: 2 },
      { part: "sun-disc", x: 20, y: 18, scale: 1.1 },
    ],
    far: [
      { part: "planet", x: 78, y: 74, scale: 2.6, paint: "blue-800" },
      { part: "ring-arc", x: 56, y: 70, scale: 1.1, paint: "slate-400" },
      { part: "asteroid", x: 50, y: 78, scale: 0.5, count: 5, spread: [40, 4] },
    ],
    subject: [
      { part: "wreckage", x: 54, y: 54, scale: 1.8 },
      { part: "ore-chunk", x: 58, y: 60, scale: 1.6, paint: "cyan-400" },
    ],
    near: [
      { part: "gloved-hand", x: 30, y: 66, scale: 1.4 },
      { part: "asteroid", x: 86, y: 30, scale: 1.2 },
    ],
    light: [
      { part: "lens-flare", x: 20, y: 18, scale: 0.7, paint: "white" },
      { part: "halo", x: 58, y: 58, scale: 1.2, paint: "cyan-400" },
      { part: "rim-light", x: 54, y: 54, scale: 1.4, flip: -1, paint: "white" },
    ],
    particles: [
      { part: "shards", x: 60, y: 44, scale: 1, count: 2, spread: [16, 10] },
      { part: "sparks", x: 58, y: 58, scale: 1, paint: "cyan-400" },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "slate-100" },
    ],
  },
};

/** The Propagation's Wasp (Stinkyscape #91, Common): a chitin-plated space wasp with a jet-thruster abdomen splits into a twin mid-sting above a hive asteroid's honeycomb craters, in an acid-green venom glow under a dark sky. */
const THE_PROPAGATIONS_WASP: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "green-900"],
  light: { x: 52, y: 48, r: 34, paint: "green-400", opacity: 0.4 },
  emblem: { x: 22, y: 80, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 24, scale: 2 },
      { part: "planet", x: 82, y: 20, scale: 0.9, paint: "jade-400" },
    ],
    far: [{ part: "honeycomb", x: 50, y: 70 }],
    subject: [
      { part: "space-wasp", x: 60, y: 54, scale: 1.5, paint: "lime-400", fade: 0.55 },
      { part: "space-wasp", x: 44, y: 44, scale: 1.7 },
    ],
    light: [
      { part: "halo", x: 52, y: 50, scale: 1.6, paint: "green-400" },
      { part: "rim-light", x: 44, y: 44, scale: 1.4, paint: "lime-400" },
    ],
    particles: [
      { part: "sparks", x: 54, y: 52, scale: 1.1, count: 2, spread: [6, 6], paint: "lime-400" },
      { part: "motes", x: 50, y: 56, scale: 1.4, paint: "green-400" },
    ],
  },
};

/** The Propagation's Hornet (Stinkyscape #92, Rare): a bigger armoured hornet with ion-blue wing membranes dives through a gas giant's ring beside its freshly spawned copy, rush streaks behind it, in a harsh rim light. */
const THE_PROPAGATIONS_HORNET: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 30, y: 30, r: 34, paint: "white", opacity: 0.22 },
  emblem: { x: 20, y: 76, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 22, scale: 2 },
      { part: "stars", x: 30, y: 70, scale: 1.4 },
      { part: "planet", x: 74, y: 30, scale: 3.2, paint: "brass-600" },
    ],
    far: [
      { part: "ring-arc", x: 60, y: 44, scale: 1.2, paint: "violet-300" },
      { part: "asteroid", x: 60, y: 52, scale: 0.4, count: 5, spread: [32, 3] },
    ],
    subject: [{ part: "space-wasp", x: 50, y: 52, scale: 2.1, paint: "sky-400", count: 2, spread: [60, 30] }],
    near: [
      { part: "asteroid", x: 14, y: 88, scale: 2.2 },
      { part: "asteroid", x: 88, y: 84, scale: 1.4 },
      { part: "probe", x: 84, y: 64, scale: 0.9 },
    ],
    light: [
      { part: "rim-light", x: 50, y: 52, scale: 2, flip: -1, paint: "white" },
      { part: "speed-streaks", x: 40, y: 52, scale: 1.2, paint: "sky-300" },
      { part: "halo", x: 50, y: 52, scale: 1.4, paint: "sky-400" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 56, scale: 1.2, count: 2, spread: [14, 10], paint: "white" },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "sky-300" },
    ],
  },
};

/** The enemy pirate who turns too late in Tmack Trav's scene. */
const BOARDER = { head: "head-robot", hair: null, outfit: "outfit-armor", item: "item-blaster", skin: "slate-400", cloth: "slate-700" } as const;

/** Tmack Trav (Stinkyscape #94, Epic): in front, Tmack Trav, a broad-shouldered brawler in a scuffed red boarding suit, wrenches his shock-axe free of a toppled robot; in the middle a second enemy turns too late; behind, a burning boarding tube between two ships under emergency strobes; the tube is seen from inside, the far ship's airlock burning at its end. */
const TMACK_TRAV: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "red-900"],
  light: { x: 54, y: 36, r: 34, paint: "ember-500", opacity: 0.36 },
  emblem: { x: 42, y: 50, size: 5, rotate: 0 },
  figure: {
    x: 42,
    y: 85,
    scale: 1.1,
    pose: "pose-guard",
    look: { head: "head-human", hair: "hair-short", outfit: "outfit-flightsuit", item: "item-shock-axe", skin: "brass-600", cloth: "red-600" },
  },
  extras: [{ layer: "mid", x: 70, y: 70, scale: 0.6, pose: "pose-run", look: BOARDER }],
  layers: {
    far: [{ part: "vent-duct", x: 54, y: 38, scale: 2.2 }],
    mid: [
      { part: "airlock", x: 54, y: 38, scale: 1 },
      { part: "flames", x: 46, y: 46, scale: 1.2 },
      { part: "flames", x: 62, y: 47, scale: 1 },
      { part: "smoke", x: 54, y: 30, scale: 1.1 },
    ],
    near: [{ part: "robot-wreck", x: 62, y: 92, scale: 1.6 }],
    light: [
      { part: "alarm-light", x: 12, y: 24, scale: 1.2 },
      { part: "alarm-light", x: 88, y: 24, scale: 1.2, flip: -1 },
      { part: "halo", x: 54, y: 40, scale: 1.8, paint: "ember-500" },
      { part: "rim-light", x: 42, y: 60, scale: 1.4, paint: "orange-400" },
    ],
    particles: [
      { part: "embers", x: 54, y: 44, scale: 1.4, count: 3, spread: [18, 10] },
      { part: "sparks", x: 52, y: 52, scale: 1, count: 2, spread: [6, 6], paint: "cyan-400" },
      { part: "sparks", x: 62, y: 86, scale: 0.9, paint: "orange-400" },
    ],
  },
};

/** The first of Vanguard Coalition's six mismatched heroes, and each of the other five, no two of a kind. */
const VANGUARD_CAPTAIN = { head: "head-human", hair: "visor", outfit: "outfit-armor", item: "item-blaster", skin: "wood-700", cloth: "blue-800" } as const;

/** Faction: Vanguard Coalition (Stinkyscape #95, Rare): six mismatched heroes argue beneath one torn star-emblem banner as their dropship's ramp lowers onto a war-torn moon, in dust clouds and red flare light; the robot among them is the kit's boxy robot, the banner's flag the card's emblem. */
const FACTION_VANGUARD_COALITION: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "red-900"],
  light: { x: 74, y: 24, r: 36, paint: "red-500", opacity: 0.34 },
  emblem: { x: 51, y: 55, size: 5, rotate: 0 },
  figure: { x: 38, y: 85, scale: 0.8, pose: "pose-point", look: VANGUARD_CAPTAIN },
  extras: [
    { layer: "mid", x: 20, y: 77, scale: 0.58, pose: "pose-hold", look: { head: "head-lizard", hair: null, outfit: "outfit-tunic", item: "item-spear", skin: "green-700", cloth: "brass-600" } },
    { layer: "mid", x: 30, y: 79, scale: 0.56, flip: -1, pose: "pose-guard", look: { head: "head-felinor", hair: null, outfit: "outfit-jacket", item: null, skin: "orange-400", cloth: "violet-700" } },
    { layer: "near", x: 12, y: 100, scale: 0.86, pose: "pose-hips", look: { head: "head-ogre", hair: null, outfit: "outfit-fur", item: null, skin: "jade-400", cloth: "wood-700" } },
    { layer: "near", x: 88, y: 100, scale: 0.84, flip: -1, pose: "pose-point", look: { head: "head-human", hair: "hair-long", outfit: "outfit-robe", item: null, skin: "red-300", cloth: "jade-400" } },
  ],
  layers: {
    sky: [{ part: "planet", x: 18, y: 22, scale: 1.2, paint: "red-600" }],
    far: [
      { part: "regolith", x: 50, y: 70, paint: "slate-700" },
      { part: "smoke-billow", x: 86, y: 56, scale: 1 },
    ],
    mid: [{ part: "dropship", x: 76, y: 70, scale: 1.5, flip: -1 }],
    subject: [
      { part: "banner", x: 47, y: 77, scale: 1.5, paint: "red-600" },
      { part: "robot", x: 62, y: 85, scale: 1.6, flip: -1 },
    ],
    light: [
      { part: "light-shaft", x: 74, y: 24, scale: 1.2, paint: "red-500" },
      { part: "rim-light", x: 50, y: 62, scale: 1.6, flip: -1, paint: "red-500" },
    ],
    particles: [
      { part: "dust-cloud", x: 30, y: 84, scale: 1.4 },
      { part: "embers", x: 60, y: 50, scale: 1.2 },
    ],
  },
};

/** Ocean Tsunami (Stinkyscape #96, Rare): Ocean Tsunami, a towering surfer-pilot in a sea-green flight suit, sweeps a crashing wave across a cantina table, leaving only glittering ore crystals behind. */
const OCEAN_TSUNAMI: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "blue-800"],
  light: { x: 62, y: 66, r: 34, paint: "cyan-400", opacity: 0.32 },
  emblem: { x: 34, y: 52, size: 5, rotate: 0 },
  figure: {
    x: 34,
    y: 85,
    scale: 1.05,
    pose: "pose-raise",
    look: { head: "head-human", hair: "hair-long", outfit: "outfit-flightsuit", item: null, skin: "wood-700", cloth: "jade-400" },
  },
  layers: {
    far: [
      { part: "bulkhead", x: 50, y: 74, paint: "slate-800" },
      { part: "porthole", x: 70, y: 30, scale: 1.3 },
      { part: "lantern", x: 52, y: 4, scale: 1 },
      { part: "lantern", x: 86, y: 4, scale: 1 },
    ],
    mid: [
      { part: "counter", x: 86, y: 82, scale: 0.7, paint: "wood-700" },
      { part: "vial", x: 82, y: 75, scale: 1, count: 3, spread: [6, 0], paint: "jade-400" },
      { part: "chair", x: 14, y: 90, scale: 1 },
    ],
    subject: [{ part: "wide-table", x: 62, y: 96, scale: 1.3, paint: "wood-700" }],
    near: [
      { part: "surf-wave", x: 68, y: 82, scale: 1.3 },
      { part: "crystal", x: 50, y: 82, scale: 1, count: 3, spread: [5, 1], paint: "sky-300" },
      { part: "gems", x: 52, y: 84, scale: 0.6 },
    ],
    light: [
      { part: "halo", x: 50, y: 80, scale: 1, paint: "cyan-400" },
      { part: "rim-light", x: 34, y: 60, scale: 1.2, paint: "sky-300" },
    ],
    particles: [
      { part: "spray", x: 74, y: 66, scale: 1.4, count: 2, spread: [8, 4] },
      { part: "sparks", x: 50, y: 78, scale: 1, paint: "white" },
      { part: "motes", x: 56, y: 60, scale: 1.2, paint: "sky-300" },
    ],
  },
};

/** Asteroid Field (Stinkyscape #97, Common): a scout ship rattles through a dense asteroid field, rocks cracking its shields and hull, a glinting ore shard caught in its tractor net; the scout is the kit's fighter. */
const ASTEROID_FIELD: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 46, y: 48, r: 30, paint: "sky-300", opacity: 0.25 },
  emblem: { x: 84, y: 76, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 30, scale: 2.2 },
      { part: "nebula", x: 30, y: 64, scale: 1.4, paint: "blue-600" },
    ],
    far: [{ part: "asteroid", x: 50, y: 50, scale: 0.7, count: 6, spread: [40, 30] }],
    mid: [
      { part: "asteroid", x: 14, y: 26, scale: 2.4 },
      { part: "asteroid", x: 86, y: 36, scale: 1.8 },
    ],
    subject: [
      { part: "fighter", x: 46, y: 44, scale: 2.6 },
      { part: "shield-bubble", x: 46, y: 58, scale: 1.1, paint: "sky-300", fade: 0.6 },
      { part: "tractor-beam", x: 46, y: 50, scale: 1, paint: "cyan-400" },
      { part: "ore-chunk", x: 46, y: 74, scale: 1.4, paint: "cyan-400" },
    ],
    near: [
      { part: "asteroid", x: 82, y: 86, scale: 2.8 },
      { part: "asteroid", x: 12, y: 80, scale: 1.6 },
    ],
    light: [
      { part: "halo", x: 46, y: 74, scale: 0.9, paint: "cyan-400" },
      { part: "zap", x: 60, y: 34, scale: 0.4, paint: "sky-300" },
    ],
    particles: [
      { part: "shards", x: 60, y: 38, scale: 1, count: 2, spread: [6, 6] },
      { part: "sparks", x: 58, y: 40, scale: 1, count: 2, spread: [6, 8], paint: "white" },
    ],
  },
};

/** A gasping juror of No-No Words' gallery. */
const JUROR = { head: "head-human", hair: "hair-short", outfit: "outfit-blazer", item: null, skin: "red-300", cloth: "slate-700" } as const;

/** No-No Words (Stinkyscape #99, Common): in a hushed station courtroom a scowling pilot spits a curse as a crackling purple shockwave crumples a small robot while the gallery gasps; the courtroom's tiered gallery is the kit's auditorium, the bench its counter. */
const NO_NO_WORDS: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 56, y: 66, r: 34, paint: "purple-400", opacity: 0.38 },
  emblem: { x: 50, y: 52, size: 5, rotate: 0 },
  figure: {
    x: 38,
    y: 85,
    scale: 0.86,
    pose: "pose-point",
    look: { head: "head-human", hair: "visor", outfit: "outfit-jacket", item: null, skin: "orange-400", cloth: "red-900" },
  },
  extras: [
    { layer: "mid", x: 16, y: 80, scale: 0.46, pose: "pose-raise", look: JUROR },
    { layer: "mid", x: 84, y: 80, scale: 0.46, flip: -1, pose: "pose-spread", look: { ...JUROR, hair: "hair-long", skin: "wood-700", cloth: "violet-700" } },
  ],
  layers: {
    sky: [{ part: "nebula", x: 50, y: 26, scale: 1.4 }],
    far: [{ part: "viewport", x: 50, y: 28, scale: 1.8 }],
    mid: [
      { part: "auditorium", x: 50, y: 56 },
      { part: "counter", x: 50, y: 58, scale: 0.6, paint: "wood-700" },
    ],
    subject: [{ part: "robot-wreck", x: 62, y: 84, scale: 0.9 }],
    light: [
      { part: "shockwave", x: 60, y: 76, scale: 0.8, paint: "purple-400" },
      { part: "zap", x: 44, y: 48, scale: 0.55, paint: "violet-300" },
    ],
    particles: [{ part: "sparks", x: 60, y: 74, scale: 1.1, count: 2, spread: [6, 4], paint: "violet-300" }],
  },
};

/** The pupils of Book of Nono-Words' classroom, each a look of their own. */
const PUPIL = { head: "head-human", hair: "hair-spiky", outfit: "outfit-jacket", item: null, skin: "orange-400", cloth: "blue-600" } as const;

/** Book of Nono-Words (Stinkyscape #102, Epic): in front, a small padlocked pocket book snaps open on a teacher's desk in a colony-ship classroom, four purple sparks of rising size escaping its blank pages; in the middle its pages riffle; behind, wide-eyed pupils and a violet nebula window. */
const BOOK_OF_NONO_WORDS: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 50, y: 72, r: 34, paint: "purple-400", opacity: 0.4 },
  emblem: { x: 40, y: 84, size: 5, rotate: 0 },
  figure: { x: 68, y: 80, scale: 0.6, pose: "pose-spread", look: { ...PUPIL, hair: "hair-bun", skin: "brass-600", cloth: "jade-400" } },
  extras: [
    { layer: "far", x: 18, y: 72, scale: 0.5, pose: "pose-raise", look: PUPIL },
    { layer: "far", x: 84, y: 72, scale: 0.5, flip: -1, pose: "pose-stand", look: { ...PUPIL, hair: "beanie", skin: "wood-700", cloth: "violet-500" } },
  ],
  layers: {
    sky: [
      { part: "nebula", x: 50, y: 32, scale: 1.8 },
      { part: "stars", x: 50, y: 30, scale: 1.6 },
      { part: "planet", x: 80, y: 22, scale: 0.8, paint: "violet-300" },
    ],
    far: [
      { part: "viewport", x: 50, y: 36, scale: 2.3 },
      { part: "tile-floor", x: 50, y: 72, paint: "slate-800" },
    ],
    mid: [
      { part: "desk", x: 18, y: 78, scale: 1.2 },
      { part: "desk", x: 84, y: 78, scale: 1.2 },
      { part: "pages", x: 50, y: 66, scale: 1.2 },
    ],
    near: [
      { part: "wide-table", x: 50, y: 102, scale: 1.8, paint: "wood-700" },
      { part: "tome", x: 50, y: 84, scale: 1, paint: "violet-700" },
      { part: "padlock", x: 62, y: 86, scale: 0.8 },
    ],
    light: [
      { part: "burst", x: 46, y: 74, scale: 0.3, paint: "purple-400" },
      { part: "burst", x: 40, y: 64, scale: 0.45, paint: "purple-400" },
      { part: "burst", x: 46, y: 52, scale: 0.6, paint: "purple-400" },
      { part: "burst", x: 56, y: 40, scale: 0.8, paint: "purple-400" },
      { part: "halo", x: 50, y: 78, scale: 1.4, paint: "violet-300" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 60, scale: 1, count: 2, spread: [10, 12], paint: "violet-300" },
      { part: "motes", x: 50, y: 60, scale: 1.4, paint: "purple-400" },
    ],
  },
};

/** Vxyl (Stinkyscape #103, Rare): Vxyl, a slender assassin in a shimmering half-invisible stealth cloak, a venom-green dart between two fingers, slips out of a station air duct, in a dim green light. */
const VXYL: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "green-900"],
  light: { x: 54, y: 46, r: 30, paint: "green-400", opacity: 0.26 },
  emblem: { x: 46, y: 50, size: 5, rotate: 0 },
  figure: {
    x: 48,
    y: 85,
    scale: 1,
    pose: "pose-guard",
    look: { head: "head-human", hair: "hood", outfit: "outfit-cloak", item: "item-dart", skin: "red-300", cloth: "slate-700" },
  },
  layers: {
    far: [{ part: "vent-duct", x: 50, y: 40, scale: 1.6 }],
    mid: [
      { part: "pipes", x: 16, y: 34, scale: 0.7 },
      { part: "alarm-light", x: 86, y: 22, scale: 1, flip: -1, paint: "green-400" },
    ],
    near: [{ part: "mist", x: 50, y: 90, scale: 1.2, paint: "green-200" }],
    light: [
      { part: "halo", x: 57, y: 52, scale: 0.8, paint: "green-400" },
      { part: "rim-light", x: 48, y: 60, scale: 1.2, paint: "green-400" },
    ],
    particles: [
      { part: "drops", x: 58, y: 58, scale: 0.6, paint: "green-400" },
      { part: "sparks", x: 44, y: 74, scale: 1, count: 2, spread: [6, 6], paint: "cyan-400" },
      { part: "motes", x: 50, y: 50, scale: 1.2, paint: "green-400" },
    ],
  },
};

/** xxFreshJack (Stinkyscape #109, Mythic): xxFreshJack, a young ace in a crisp white-and-teal flight jacket and mirrored visor, rockets out of a cracked, glitching warp gate over a crowded spaceport, one arm hurling an overcharged comet of raw power; streams of glowing pixels spill from the tear behind him, the crowd's ships tilting in its glare, in a teal and magenta flare with lens bloom and sparks raining onto the landing strips below. */
const XXFRESHJACK: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 40, r: 44, paint: "cyan-400", opacity: 0.4 },
  emblem: { x: 46, y: 58, size: 5, rotate: 0 },
  figure: {
    x: 46,
    y: 76,
    scale: 0.95,
    pose: "pose-raise",
    look: { head: "head-human", hair: "visor", outfit: "outfit-jacket", item: "item-orb", skin: "orange-400", cloth: "slate-100" },
  },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 18, scale: 2.2 },
      { part: "planet", x: 86, y: 14, scale: 0.8, paint: "purple-400" },
    ],
    far: [
      { part: "launch-pad", x: 50, y: 90, scale: 1.5 },
      { part: "shuttle", x: 16, y: 56, scale: 1, flip: -1 },
      { part: "fighter", x: 84, y: 62, scale: 1.1 },
    ],
    mid: [
      { part: "warp-gate", x: 50, y: 40, scale: 1.4 },
      { part: "space-rift", x: 50, y: 40, scale: 0.8, paint: "purple-400" },
    ],
    subject: [{ part: "comet", x: 70, y: 18, scale: 1, paint: "cyan-400" }],
    near: [{ part: "crew-crowd", x: 50, y: 100, scale: 2 }],
    light: [
      { part: "lens-flare", x: 57, y: 20, scale: 0.8, paint: "purple-400" },
      { part: "halo", x: 50, y: 40, scale: 2.2, paint: "cyan-400" },
      { part: "halo", x: 57, y: 20, scale: 1.2, paint: "purple-400" },
      { part: "god-rays", x: 50, y: 40, scale: 1, paint: "cyan-400" },
      { part: "rim-light", x: 46, y: 56, scale: 1.2, paint: "cyan-400" },
      { part: "light-shaft", x: 50, y: 40, scale: 1.2, paint: "purple-400" },
    ],
    particles: [
      { part: "pixels", x: 50, y: 44, scale: 1.2 },
      { part: "sparks", x: 50, y: 80, scale: 1.2, count: 4, spread: [30, 6], paint: "cyan-400" },
      { part: "speed-streaks", x: 46, y: 56, scale: 1, flip: -1, paint: "white" },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "purple-400" },
    ],
  },
};

/** iKwzki (Stinkyscape #110, Mythic): iKwzki, a poised navigator in a pearl-white long coat with a starlight-lined hood, stands on a glass observation spire above a ringed planet, one hand linked by a ribbon of light to a dim starship beside her, which begins to glow from within; a halo of shimmering motes spirals around them, and behind, the planet's rings catch rose and cyan dawn; the spire's deck is a glass parapet before her, its sister spires beside. */
const IKWZKI: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 70, y: 64, r: 42, paint: "red-300", opacity: 0.32 },
  emblem: { x: 40, y: 58, size: 5, rotate: 0 },
  figure: {
    x: 40,
    y: 84,
    scale: 1.02,
    pose: "pose-stand",
    look: { head: "head-human", hair: "hood", outfit: "outfit-coat", item: null, skin: "red-300", cloth: "white" },
  },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 18, scale: 2.2 },
      { part: "stars", x: 24, y: 46, scale: 1.4 },
      { part: "nebula", x: 22, y: 26, scale: 1.2, paint: "blue-600" },
      { part: "planet", x: 64, y: 84, scale: 4, paint: "blue-800" },
    ],
    far: [
      { part: "ring-arc", x: 64, y: 72, scale: 1.1, paint: "red-300" },
      { part: "sun-disc", x: 92, y: 66, scale: 0.9 },
      { part: "asteroid", x: 50, y: 70, scale: 0.4, count: 3, spread: [36, 3] },
    ],
    mid: [
      { part: "glass-spire", x: 14, y: 100, scale: 1.3 },
      { part: "glass-spire", x: 90, y: 104, scale: 1 },
      { part: "starship", x: 72, y: 40, scale: 0.7, paint: "slate-700", fade: 0.75 },
    ],
    near: [{ part: "glass-wall", x: 40, y: 104, scale: 1.4, paint: "sky-300", fade: 0.6 }],
    light: [
      { part: "light-ribbon", x: 62, y: 50, scale: 0.6, paint: "cyan-400" },
      { part: "halo", x: 72, y: 40, scale: 1.8, paint: "cyan-400" },
      { part: "halo", x: 40, y: 52, scale: 1.8, paint: "red-300" },
      { part: "halo", x: 92, y: 66, scale: 1.4, paint: "red-300" },
      { part: "god-rays", x: 92, y: 66, scale: 1, flip: -1, paint: "red-300" },
      { part: "rim-light", x: 40, y: 60, scale: 1.4, paint: "sky-300" },
    ],
    particles: [
      { part: "motes", x: 52, y: 48, scale: 1.4, count: 3, spread: [14, 10], paint: "sky-300" },
      { part: "sparks", x: 54, y: 46, scale: 1.2, count: 3, spread: [18, 14], paint: "white" },
      { part: "embers", x: 72, y: 44, scale: 1, paint: "cyan-400" },
    ],
  },
};

/** A listener in Shockwave447's front rows, blown back by his explanation. */
const LISTENER = { head: "head-human", hair: "hair-short", outfit: "outfit-jacket", item: null, skin: "red-300", cloth: "slate-700" } as const;

/** Shockwave447 (Stinkyscape #111, Mythic): Shockwave447, a wiry engineer in an electric-blue lab harness and cracked goggles, lectures at speed in a vast orbital auditorium, sprawling diagrams of light lines and gears spiralling from his hands across the whole hall; arcs crackle from his fingertips, and the shockwave of his explanation blows the front rows' hands of cards into storms of tangled machinery shapes, electric blue against deep navy; the hall's tiers are the kit's amphitheatre, as the auditorium's rows would take the Mythic past its byte ceiling. */
const SHOCKWAVE447: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 50, y: 50, r: 44, paint: "blue-600", opacity: 0.45 },
  emblem: { x: 50, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 82,
    scale: 0.86,
    pose: "pose-raise",
    look: { head: "head-human", hair: "goggles", outfit: "outfit-harness", item: null, skin: "wood-700", cloth: "blue-600" },
  },
  extras: [
    { layer: "near", x: 14, y: 100, scale: 0.7, pose: "pose-spread", look: LISTENER },
    { layer: "near", x: 86, y: 100, scale: 0.7, flip: -1, pose: "pose-raise", look: { ...LISTENER, hair: "hair-long", skin: "brass-600", cloth: "blue-800" } },
  ],
  layers: {
    sky: [
      { part: "stars", x: 50, y: 18, scale: 2.2 },
      { part: "planet", x: 78, y: 22, scale: 1, paint: "sky-400" },
    ],
    far: [
      { part: "viewport", x: 50, y: 30, scale: 2.4 },
      { part: "amphitheatre", x: 50, y: 80, scale: 1.2, paint: "slate-800" },
    ],
    mid: [{ part: "crew-crowd", x: 50, y: 68, scale: 1.8 }],
    subject: [{ part: "light-diagram", x: 50, y: 44, scale: 1.5, paint: "sky-400" }],
    near: [
      { part: "card", x: 30, y: 84, scale: 1, count: 3, spread: [8, 8] },
      { part: "card", x: 70, y: 84, scale: 1, count: 2, spread: [8, 8] },
      { part: "cog", x: 24, y: 70, scale: 1, paint: "sky-400" },
      { part: "gear", x: 76, y: 68, scale: 0.8, paint: "sky-300" },
    ],
    light: [
      { part: "zap", x: 40, y: 34, scale: 0.6, flip: -1, paint: "sky-300" },
      { part: "zap", x: 60, y: 34, scale: 0.6, paint: "sky-300" },
      { part: "shockwave", x: 50, y: 76, scale: 1.4, paint: "sky-400" },
      { part: "halo", x: 50, y: 44, scale: 2.2, paint: "blue-600" },
      { part: "rim-light", x: 50, y: 60, scale: 1.2, paint: "sky-300" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 40, scale: 1.2, count: 3, spread: [24, 12], paint: "sky-300" },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "blue-200" },
    ],
  },
};

/** A hopeful queueing on Job Application's concourse, a portfolio held to the chest. */
const HOPEFUL = { head: "head-human", hair: "hair-long", outfit: "outfit-suit", item: "item-clipboard", skin: "red-300", cloth: "slate-400" } as const;

/** Job Application (Stinkyscape #112, Legendary): a corporate recruitment concourse in an orbital office tower, hopefuls with portfolios queueing past glowing interview pods as a blank resume sheet is sucked into a pneumatic intake tube; a hiring drone hovers at the desk, and through the window rises the next stage's tower, in harsh fluorescent light with one warm spotlight on a closed offer door; the next stage's tower is the kit's glass spire, and the queue behind the three hopefuls the kit's crew. */
const JOB_APPLICATION: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-800"],
  light: { x: 84, y: 56, r: 30, paint: "orange-400", opacity: 0.34 },
  emblem: { x: 14, y: 30, size: 5, rotate: 0 },
  figure: { x: 34, y: 88, scale: 0.78, pose: "pose-hold", look: HOPEFUL },
  extras: [
    { layer: "mid", x: 22, y: 80, scale: 0.6, pose: "pose-hold", look: { ...HOPEFUL, hair: "hair-short", item: "item-book", skin: "wood-700", cloth: "slate-700" } },
    { layer: "mid", x: 11, y: 79, scale: 0.56, pose: "pose-stand", look: { ...HOPEFUL, hair: "hair-bun", item: null, skin: "brass-600", cloth: "blue-800" } },
  ],
  layers: {
    sky: [{ part: "stars", x: 50, y: 22, scale: 2 }],
    far: [
      { part: "glass-spire", x: 48, y: 62, scale: 0.9, paint: "sky-400", fade: 0.8 },
      { part: "viewport", x: 50, y: 34, scale: 1.8 },
      { part: "ceiling-lights", x: 50, y: 4, scale: 1.2 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 70, paint: "slate-700" },
      { part: "pod", x: 14, y: 70, scale: 0.6, paint: "sky-300" },
      { part: "framed-door", x: 84, y: 72, scale: 1.7, paint: "wood-700" },
      { part: "desk", x: 66, y: 78, scale: 1.4, paint: "slate-400" },
    ],
    subject: [
      { part: "intake-tube", x: 50, y: 80, scale: 1.2 },
      { part: "pages", x: 54, y: 44, scale: 0.6 },
      { part: "drone", x: 66, y: 56, scale: 0.9 },
    ],
    near: [{ part: "crew-crowd", x: 18, y: 104, scale: 1.3 }],
    light: [
      { part: "spotlight", x: 84, y: 20, scale: 0.9, paint: "orange-400" },
      { part: "halo", x: 84, y: 60, scale: 1.4, paint: "orange-400" },
      { part: "halo", x: 50, y: 50, scale: 1.2, paint: "sky-300" },
      { part: "rim-light", x: 34, y: 66, scale: 1, paint: "slate-100" },
    ],
    particles: [{ part: "motes", x: 50, y: 40, scale: 1.2, paint: "slate-100" }],
  },
};

/** Paycheck (Stinkyscape #112.1, Token): a crisp green credit chip lands on a station casino's roulette table as the ball rattles toward red pockets on a blank wheel, in green lamplight. */
const PAYCHECK: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "green-900"],
  light: { x: 50, y: 30, r: 34, paint: "green-400", opacity: 0.32 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "bulkhead", x: 50, y: 62, paint: "slate-800" },
      { part: "porthole", x: 82, y: 34, scale: 0.9 },
    ],
    mid: [{ part: "deck-floor", x: 50, y: 64, paint: "green-900" }],
    subject: [
      { part: "roulette", x: 50, y: 84, scale: 2 },
      { part: "credit-chip", x: 58, y: 52, scale: 2 },
    ],
    near: [{ part: "coin-stack", x: 20, y: 92, scale: 0.9 }],
    light: [
      { part: "spotlight", x: 50, y: 2, scale: 1, paint: "green-400" },
      { part: "halo", x: 58, y: 56, scale: 1, paint: "green-400" },
    ],
    particles: [{ part: "sparks", x: 58, y: 58, scale: 1, count: 2, spread: [6, 4], paint: "green-200" }],
  },
};

/** A Very Dandy Lad (Stinkyscape #113, Epic): in front, a dandy in a plum velvet frock coat and top hat flicks a lace cuff; in the middle, cheap little robots and drones blink out in violet sparks around him; behind, a velvet-draped first-class lounge on a luxury liner, its window on the stars and a chandelier overhead; he stands hand on hip, as the figure kit has no monocle or lace cuff to flick. */
const A_VERY_DANDY_LAD: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 50, y: 34, r: 38, paint: "violet-500", opacity: 0.34 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 88,
    scale: 1,
    pose: "pose-hips",
    look: { head: "head-human", hair: "top-hat", outfit: "outfit-frock", item: null, skin: "red-300", cloth: "violet-700" },
  },
  layers: {
    sky: [{ part: "stars", x: 50, y: 30, scale: 1.6 }],
    far: [
      { part: "viewport", x: 50, y: 36, scale: 1.3 },
      { part: "drapes", x: 50, y: 4, scale: 1.7, paint: "violet-700" },
      { part: "chandelier", x: 50, y: 4, scale: 1.1 },
    ],
    mid: [
      { part: "checker-floor", x: 50, y: 70, paint: "violet-700" },
      { part: "chair", x: 14, y: 82, scale: 1.3, paint: "red-900" },
      { part: "robot", x: 30, y: 82, scale: 0.6, fade: 0.55 },
      { part: "robot", x: 72, y: 84, scale: 0.55, flip: -1, fade: 0.45 },
      { part: "drone", x: 70, y: 52, scale: 0.8, fade: 0.5 },
    ],
    near: [
      { part: "wide-table", x: 88, y: 102, scale: 0.7, paint: "wood-800" },
      { part: "cup", x: 86, y: 92, scale: 1 },
    ],
    light: [
      { part: "warp-flash", x: 30, y: 72, scale: 0.5, paint: "violet-500" },
      { part: "halo", x: 50, y: 10, scale: 1.2, paint: "violet-300" },
      { part: "rim-light", x: 50, y: 62, scale: 1, paint: "violet-300" },
    ],
    particles: [
      { part: "sparks", x: 30, y: 74, scale: 1, count: 3, spread: [6, 8], paint: "violet-300" },
      { part: "motes", x: 50, y: 40, scale: 1.2, paint: "violet-300" },
    ],
  },
};

/** Arctic Citadel (Stinkyscape #114, Epic): in front, a giant in a frost-white parka, his armour bolted from scavenged wreck plates and a turret on one shoulder; in the middle, drifting scraps fly in and weld onto him; behind, an ice moon's graveyard of frozen ships; the turret hovers at his shoulder, as the figure kit mounts nothing there. */
const ARCTIC_CITADEL: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-800"],
  light: { x: 70, y: 30, r: 36, paint: "sky-300", opacity: 0.3 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 90,
    scale: 1.12,
    pose: "pose-guard",
    look: { head: "head-human", hair: "space-helmet", outfit: "outfit-plated-parka", item: "item-blaster", skin: "brass-600", cloth: "slate-100" },
  },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 20, scale: 2 },
      { part: "planet", x: 80, y: 18, scale: 0.8, paint: "sky-400" },
    ],
    far: [
      { part: "regolith", x: 50, y: 66, paint: "slate-100" },
      { part: "starship", x: 20, y: 58, scale: 0.6, paint: "slate-400", fade: 0.7 },
      { part: "freighter", x: 84, y: 62, scale: 0.7, flip: -1, paint: "slate-400", fade: 0.7 },
    ],
    mid: [
      { part: "robot-wreck", x: 18, y: 84, scale: 1 },
      { part: "ice-block", x: 84, y: 88, scale: 0.7 },
      { part: "wreckage", x: 24, y: 44, scale: 0.45 },
    ],
    subject: [{ part: "turret", x: 43, y: 38, scale: 0.8, flip: -1 }],
    near: [{ part: "frost", x: 50, y: 96, scale: 1.2 }],
    light: [
      { part: "halo", x: 50, y: 50, scale: 1.8, paint: "sky-300" },
      { part: "rim-light", x: 50, y: 60, scale: 1.2, paint: "sky-300" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 50, scale: 0.9, count: 3, spread: [12, 6], paint: "orange-400" },
    ],
  },
};

/** A crewman Scornboi lets stay. */
const KEPT_CREW = { head: "head-human", hair: "hair-long", outfit: "outfit-jacket", item: null, skin: "wood-700", cloth: "slate-400" } as const;

/** Scornboi (Stinkyscape #115, Legendary): a sneering admiral in a black and crimson high-collared greatcoat stands at a docking gate holding up a very short blank list and pointing; behind him aliens, beasts and robots are dragged by a crimson exile beam into a void hatch while a few humans and starships stay, in harsh crimson floodlight with long shadows; his short blank list is the kit's slate, and the humans who stay one crewman and the crew behind him. */
const SCORNBOI: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "red-900"],
  light: { x: 74, y: 40, r: 40, paint: "red-500", opacity: 0.4 },
  emblem: { x: 14, y: 30, size: 5, rotate: 0 },
  figure: {
    x: 38,
    y: 88,
    scale: 0.96,
    pose: "pose-point",
    look: { head: "head-human", hair: "hair-short", outfit: "outfit-greatcoat", item: "item-slate", skin: "red-300", cloth: "red-900" },
  },
  extras: [{ layer: "mid", x: 14, y: 80, scale: 0.56, pose: "pose-stand", look: KEPT_CREW }],
  layers: {
    sky: [{ part: "stars", x: 50, y: 18, scale: 2 }],
    far: [
      { part: "starship", x: 24, y: 30, scale: 0.6 },
      { part: "crew-crowd", x: 22, y: 72, scale: 0.8 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 72, paint: "slate-700" },
      { part: "airlock", x: 76, y: 42, scale: 1.4 },
      { part: "alien", x: 70, y: 68, scale: 0.6, fade: 0.8 },
      { part: "dog", x: 82, y: 70, scale: 0.8, flip: -1, fade: 0.8 },
      { part: "robot", x: 62, y: 74, scale: 0.55, fade: 0.85 },
    ],
    near: [{ part: "crate", x: 90, y: 100, scale: 1.4, paint: "slate-700" }],
    light: [
      { part: "tractor-beam", x: 76, y: 46, scale: 1.3, paint: "red-500" },
      { part: "spotlight", x: 40, y: 0, scale: 1, paint: "red-400" },
      { part: "halo", x: 76, y: 42, scale: 1.6, paint: "red-500" },
      { part: "rim-light", x: 38, y: 64, scale: 1.1, paint: "red-400" },
    ],
    particles: [{ part: "sparks", x: 76, y: 58, scale: 1, count: 2, spread: [10, 10], paint: "red-400" }],
  },
};

/** Megabase (Stinkyscape #116, Legendary): a colossal station the size of a moon, tiered decks and plated armour rings round it, its lobby dome glowing; shuttles stream into its docks as enemy fire sparks harmlessly off its hull, a gift shop's windows twinkling on one ring, a blue-green planet below and dawn glare raking across the steel. */
const MEGABASE: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 88, y: 74, r: 40, paint: "orange-400", opacity: 0.34 },
  emblem: { x: 14, y: 28, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 20, scale: 2.2 },
      { part: "stars", x: 24, y: 56, scale: 1.2 },
      { part: "nebula", x: 24, y: 24, scale: 1, paint: "blue-600" },
    ],
    far: [
      { part: "planet-limb", x: 40, y: 82, paint: "jade-400" },
      { part: "sun-disc", x: 92, y: 78, scale: 0.8 },
    ],
    mid: [
      { part: "shuttle", x: 18, y: 50, scale: 0.7 },
      { part: "shuttle", x: 26, y: 62, scale: 0.55 },
      { part: "fighter", x: 86, y: 28, scale: 0.6, flip: -1, paint: "red-600" },
      { part: "fighter", x: 90, y: 42, scale: 0.5, flip: -1, paint: "red-600" },
    ],
    subject: [{ part: "space-station", x: 50, y: 50, scale: 1.05 }],
    near: [
      { part: "laser", x: 84, y: 30, scale: 0.5, flip: -1, paint: "red-500" },
      { part: "satellite", x: 12, y: 84, scale: 0.8 },
    ],
    light: [
      { part: "lens-flare", x: 92, y: 78, scale: 0.8 },
      { part: "god-rays", x: 92, y: 78, scale: 1, flip: -1, paint: "orange-400" },
      { part: "halo", x: 50, y: 30, scale: 1.2, paint: "sky-300" },
      { part: "rim-light", x: 56, y: 52, scale: 1.3, paint: "orange-400" },
    ],
    particles: [
      { part: "sparks", x: 66, y: 40, scale: 1, count: 3, spread: [4, 8], paint: "red-400" },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "sky-300" },
    ],
  },
};

/** Core System (Stinkyscape #117, Epic): in front, a technician's gloved hand slams a big reset lever on a station command floor; in the middle, every ship and turret on the tactical table folds back into pale blue light streaming home; behind, a reactor powering down to cold blue; the folding ships and turret are faded copies in the table's light. */
const CORE_SYSTEM: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "blue-900"],
  light: { x: 50, y: 40, r: 36, paint: "sky-300", opacity: 0.3 },
  emblem: { x: 86, y: 30, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "bulkhead", x: 50, y: 62, paint: "slate-800" },
      { part: "reactor", x: 50, y: 60, scale: 1.1, paint: "blue-600" },
      { part: "console", x: 14, y: 64, scale: 0.7 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 64, paint: "slate-700" },
      { part: "holo-table", x: 46, y: 80, scale: 1.4 },
    ],
    subject: [
      { part: "fighter", x: 36, y: 50, scale: 0.45, fade: 0.6 },
      { part: "turret", x: 56, y: 56, scale: 0.55, fade: 0.6 },
      { part: "fighter", x: 60, y: 44, scale: 0.4, flip: -1, fade: 0.5 },
      { part: "energy-beam", x: 46, y: 20, scale: 0.8, paint: "sky-300" },
    ],
    near: [
      { part: "reset-lever", x: 76, y: 100, scale: 1.4, flip: -1 },
      { part: "gloved-hand", x: 90, y: 84, scale: 1, flip: -1 },
    ],
    light: [
      { part: "halo", x: 46, y: 54, scale: 1.6, paint: "sky-300" },
      { part: "halo", x: 50, y: 40, scale: 1, paint: "blue-600" },
      { part: "rim-light", x: 76, y: 86, scale: 0.8, paint: "sky-300" },
    ],
    particles: [
      { part: "motes", x: 46, y: 46, scale: 1.2, paint: "sky-300" },
      { part: "sparks", x: 72, y: 80, scale: 0.8, count: 2, spread: [4, 4], paint: "blue-200" },
    ],
  },
};

/** Secure System (Stinkyscape #118, Epic): in front, armoured blast shutters slam down over a fortress-station gun deck; in the middle, every cannon barrel sealed by glowing teal locking clamps; behind, rows of silent turrets along the hull and annoyed crew glaring, in cold teal light; the locking clamps are the kit's padlocks, lit teal. */
const SECURE_SYSTEM: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "slate-900"],
  light: { x: 50, y: 34, r: 36, paint: "cyan-400", opacity: 0.28 },
  emblem: { x: 14, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 20, scale: 2 }],
    far: [
      { part: "catwalk", x: 50, y: 52 },
      { part: "turret", x: 50, y: 50, scale: 0.6, count: 4, spread: [20, 0] },
      { part: "crew-crowd", x: 50, y: 62, scale: 1 },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 66, paint: "slate-700" }],
    subject: [
      { part: "turret", x: 32, y: 80, scale: 1.4 },
      { part: "turret", x: 66, y: 80, scale: 1.4, flip: -1 },
      { part: "padlock", x: 39, y: 70, scale: 0.9, paint: "cyan-400" },
      { part: "padlock", x: 59, y: 70, scale: 0.9, paint: "cyan-400" },
    ],
    near: [
      { part: "blast-door", x: 12, y: 102, scale: 1.3 },
      { part: "blast-door", x: 88, y: 102, scale: 1.3, flip: -1 },
    ],
    light: [
      { part: "halo", x: 39, y: 68, scale: 0.8, paint: "cyan-400" },
      { part: "halo", x: 59, y: 68, scale: 0.8, paint: "cyan-400" },
      { part: "rim-light", x: 50, y: 70, scale: 1.2, paint: "cyan-400" },
    ],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.2, paint: "cyan-400" }],
  },
};

/** Unsecure System (Stinkyscape #119, Epic): in front, a giggling hacker yanks out a glowing firewall cable; in the middle, a shimmering energy wall crumbles into glitching pixels; behind, every turret on a station ring flares to twice its size, in magenta glitch light; the palette's nearest to magenta is its purple. */
const UNSECURE_SYSTEM: SceneRecipe = {
  family: "stinkyscape",
  sky: ["black", "violet-700"],
  light: { x: 62, y: 50, r: 36, paint: "purple-400", opacity: 0.34 },
  emblem: { x: 86, y: 76, size: 5, rotate: 0 },
  figure: {
    x: 34,
    y: 88,
    scale: 0.9,
    pose: "pose-lunge",
    flip: -1,
    look: { head: "head-human", hair: "hood", outfit: "outfit-hoodie", item: null, skin: "orange-400", cloth: "slate-700" },
  },
  layers: {
    sky: [{ part: "stars", x: 50, y: 18, scale: 2 }],
    far: [
      { part: "ring-arc", x: 50, y: 30, scale: 1, paint: "slate-400" },
      { part: "turret", x: 22, y: 30, scale: 1.3 },
      { part: "turret", x: 78, y: 30, scale: 1.3, flip: -1 },
    ],
    mid: [
      { part: "deck-floor", x: 50, y: 70, paint: "slate-800" },
      { part: "energy-wall", x: 66, y: 80, scale: 1.1, paint: "purple-400" },
      { part: "glitch", x: 66, y: 60, scale: 1 },
    ],
    subject: [{ part: "cable", x: 58, y: 66, scale: 0.9, flip: -1 }],
    near: [{ part: "console", x: 88, y: 100, scale: 1, flip: -1 }],
    light: [
      { part: "halo", x: 22, y: 28, scale: 1.2, paint: "purple-400" },
      { part: "halo", x: 78, y: 28, scale: 1.2, paint: "purple-400" },
      { part: "rim-light", x: 34, y: 62, scale: 1, paint: "purple-400" },
    ],
    particles: [
      { part: "pixels", x: 66, y: 56, scale: 1 },
      { part: "sparks", x: 46, y: 64, scale: 0.9, count: 2, spread: [6, 6], paint: "purple-400" },
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
  "stinkyscape-012-2": SHADOW_WORD_PAIN,
  "stinkyscape-012-3": PSYCHIC_SCREAM,
  "stinkyscape-018-1": MELCHIOR,
  "stinkyscape-018-2": BALTHASAR,
  "stinkyscape-018-3": CASPER,
  "stinkyscape-084-1": METAL_SCRAPS,
  "stinkyscape-091": THE_PROPAGATIONS_WASP,
  "stinkyscape-092": THE_PROPAGATIONS_HORNET,
  "stinkyscape-094": TMACK_TRAV,
  "stinkyscape-095": FACTION_VANGUARD_COALITION,
  "stinkyscape-096": OCEAN_TSUNAMI,
  "stinkyscape-097": ASTEROID_FIELD,
  "stinkyscape-099": NO_NO_WORDS,
  "stinkyscape-102": BOOK_OF_NONO_WORDS,
  "stinkyscape-103": VXYL,
  "stinkyscape-109": XXFRESHJACK,
  "stinkyscape-110": IKWZKI,
  "stinkyscape-111": SHOCKWAVE447,
  "stinkyscape-112": JOB_APPLICATION,
  "stinkyscape-112-1": PAYCHECK,
  "stinkyscape-113": A_VERY_DANDY_LAD,
  "stinkyscape-114": ARCTIC_CITADEL,
  "stinkyscape-115": SCORNBOI,
  "stinkyscape-116": MEGABASE,
  "stinkyscape-117": CORE_SYSTEM,
  "stinkyscape-118": SECURE_SYSTEM,
  "stinkyscape-119": UNSECURE_SYSTEM,
};
