// R3406000: the alt scene recipes of Stinkyscape's later cards (#3406), cards 116 to 146 (stinkyscape91AltRecipes1.ts
// says what each is held to). Presentation only (CLAUDE.md rule 7): a recipe is read by the card's id alone.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";
import { normalLookOf } from "./stinkyscape91AltRecipes1.ts";

/** Megabase (Stinkyscape #116, Legendary), Alt art: inside Megabase: its moon-sized lobby atrium, crowds riding escalators past a glass-fronted gift shop, a tiny shuttle docking far above; soft coral light, worm's-eye view. */
const MEGABASE_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-300", "slate-100"],
  light: { x: 50, y: 18, r: 40, paint: "red-300", opacity: 0.4 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "glass-dome", x: 50, y: 40, scale: 2 }],
    far: [
      { part: "glass-spire", x: 10, y: 100, scale: 1.2 },
      { part: "glass-spire", x: 90, y: 100, scale: 1.2 },
      { part: "shuttle", x: 58, y: 16, scale: 0.4 },
      { part: "escalator", x: 60, y: 70, scale: 0.6, flip: -1 },
      { part: "ceiling-lights", x: 50, y: 4, scale: 1 },
    ],
    mid: [
      { part: "glass-wall", x: 74, y: 88, scale: 1, paint: "sky-300" },
      { part: "awning", x: 74, y: 72, scale: 1, paint: "red-300" },
      { part: "crew-crowd", x: 74, y: 90, scale: 0.9 },
    ],
    subject: [
      { part: "escalator", x: 12, y: 92, scale: 1.3 },
      { part: "escalator", x: 34, y: 66, scale: 0.9 },
      { part: "crew-crowd", x: 30, y: 80, scale: 0.6 },
    ],
    near: [{ part: "crew-crowd", x: 52, y: 104, scale: 2, count: 2, spread: [20, 2] }],
    light: [
      { part: "god-rays", x: 50, y: 0, scale: 1.2, paint: "red-300" },
      { part: "halo", x: 50, y: 18, scale: 1.4, paint: "red-300" },
    ],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.2, paint: "white" }],
  },
};

/** Core System (Stinkyscape #117, Epic), Alt art: the morning after the reset: an empty, dust-sheeted tactical floor lit rose by sunrise through tall windows, one technician sipping tea beside a rebooting console. The kit has no dust sheet, so the floor's dust hangs in the sunlight. */
const CORE_SYSTEM_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-300", "violet-300"],
  light: { x: 76, y: 40, r: 40, paint: "red-300", opacity: 0.4 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  figure: {
    x: 34,
    y: 92,
    scale: 0.95,
    pose: "pose-hold",
    look: { ...figureLookFor(["Human"]), hair: "glasses", outfit: "outfit-overalls", item: null, skin: "wood-600", cloth: "slate-400" },
  },
  layers: {
    sky: [{ part: "sun", x: 76, y: 46, scale: 0.6 }],
    far: [
      { part: "room-wall", x: 50, y: 74, paint: "slate-400" },
      { part: "window", x: 62, y: 66, scale: 2 },
      { part: "window", x: 88, y: 66, scale: 2 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 74, paint: "slate-400" },
      { part: "holo-table", x: 80, y: 86, scale: 0.9, fade: 0.35 },
    ],
    subject: [{ part: "console", x: 56, y: 88, scale: 1.2 }],
    near: [
      { part: "cup", x: 42, y: 66, scale: 0.6 },
      { part: "chair", x: 90, y: 100, scale: 1.2, paint: "slate-100" },
    ],
    light: [
      { part: "light-shaft", x: 62, y: 44, scale: 1.2, paint: "red-300" },
      { part: "light-shaft", x: 88, y: 44, scale: 1.2, paint: "red-300" },
      { part: "halo", x: 56, y: 78, scale: 0.8, paint: "sky-300" },
    ],
    particles: [
      { part: "dust", x: 50, y: 60, scale: 1.2, paint: "red-300" },
      { part: "steam", x: 42, y: 58, scale: 0.4, paint: "white" },
    ],
  },
};

/** Vamp Rounds (Stinkyscape #122, Common), Alt art: a belt of crimson-tipped rounds drains a red glow from a struck pirate hulk and streams it home to a battered gunship; wide battle view over a violet nebula. */
const VAMP_ROUNDS_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["violet-700", "violet-500"],
  light: { x: 76, y: 40, r: 34, paint: "red-500", opacity: 0.3 },
  emblem: { x: 84, y: 76, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "nebula", x: 50, y: 40, scale: 1.6, paint: "purple-400" },
      { part: "stars", x: 50, y: 30, scale: 1.4 },
    ],
    far: [{ part: "wreckage", x: 78, y: 40, scale: 1.1 }],
    mid: [{ part: "starship", x: 22, y: 66, scale: 0.7 }],
    subject: [{ part: "cartridge", x: 50, y: 52, scale: 1.2, count: 5, spread: [14, 6] }],
    light: [
      { part: "energy-beam", x: 78, y: 18, scale: 0.6, paint: "red-500" },
      { part: "halo", x: 22, y: 66, scale: 0.9, paint: "red-400" },
    ],
    particles: [{ part: "sparks", x: 78, y: 40, scale: 0.8, paint: "red-400" }],
  },
};

/** Gokugains (Stinkyscape #123, Rare), Alt art: Gokugains on a sunny station gym deck lifts a crate of stolen glowing sparks overhead, grinning at his reflection in a mirror wall; bright aqua daylight. */
const GOKUGAINS_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["cyan-400", "sky-300"],
  light: { x: 50, y: 24, r: 40, paint: "white", opacity: 0.35 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  figure: { x: 34, y: 96, scale: 1.1, pose: "pose-lift", look: normalLookOf("stinkyscape-123") },
  extras: [{ layer: "far", x: 76, y: 82, scale: 0.7, flip: -1, pose: "pose-lift", look: normalLookOf("stinkyscape-123"), fade: 0.5 }],
  layers: {
    far: [{ part: "mirror", x: 76, y: 84, scale: 1.8 }],
    mid: [
      { part: "tile-floor", x: 50, y: 82, paint: "slate-100" },
      { part: "barbell", x: 14, y: 90, scale: 0.6 },
    ],
    subject: [{ part: "crate", x: 34, y: 36, scale: 1.2, paint: "cyan-400" }],
    near: [{ part: "weights", x: 86, y: 98, scale: 1 }],
    light: [{ part: "halo", x: 34, y: 30, scale: 1, paint: "orange-400" }],
    particles: [
      { part: "sparks", x: 34, y: 28, scale: 1, paint: "orange-400" },
      { part: "motes", x: 50, y: 50, scale: 1, paint: "white" },
    ],
  },
};

/** Jlockeed's Shipbuilding Grant (Stinkyscape #124, Common), Alt art: on a windy launch field, a beaming Jlockeed executive hands a sealed crystal capsule to a nervous engineer before a skeletal rocket; pale lavender overcast. */
const SHIPBUILDING_GRANT_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["violet-300", "slate-100"],
  light: { x: 30, y: 26, r: 36, paint: "white", opacity: 0.3 },
  emblem: { x: 16, y: 72, size: 5, rotate: 0 },
  figure: {
    x: 38,
    y: 94,
    scale: 0.9,
    pose: "pose-hold",
    look: { ...figureLookFor(["Human"]), hair: "hair-short", outfit: "outfit-suit", item: null, skin: "red-300", cloth: "slate-700" },
  },
  extras: [
    {
      layer: "subject",
      x: 60,
      y: 94,
      scale: 0.85,
      flip: -1,
      pose: "pose-hold",
      look: { ...figureLookFor(["Human"]), hair: "goggles", outfit: "outfit-overalls", item: null, skin: "wood-700", cloth: "orange-400" },
    },
  ],
  layers: {
    sky: [
      { part: "cloud", x: 30, y: 20, scale: 1.4, paint: "slate-100" },
      { part: "cloud", x: 76, y: 28, scale: 1.2, paint: "white" },
    ],
    far: [
      { part: "field", x: 50, y: 70, paint: "slate-400" },
      { part: "gantry", x: 88, y: 78, scale: 1.4 },
    ],
    subject: [
      { part: "rocket", x: 76, y: 82, scale: 1.4, fade: 0.55 },
      { part: "capsule", x: 49, y: 66, scale: 0.6 },
    ],
    particles: [{ part: "gust", x: 50, y: 40, scale: 1, paint: "white" }],
  },
};

/** Jlockeed's Siege Tank (Stinkyscape #127, Rare), Alt art: the siege tank rolls down a rain-soaked frontier town's main street, treads churning mud, turret swivelling toward a distant ridge; grey-green storm light. */
const SIEGE_TANK_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["slate-400", "green-200"],
  light: { x: 76, y: 24, r: 34, paint: "green-200", opacity: 0.3 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "cloud", x: 40, y: 20, scale: 1.6, paint: "slate-700" },
      { part: "lightning", x: 80, y: 16, scale: 0.6, paint: "white" },
    ],
    far: [
      { part: "hills", x: 50, y: 56, paint: "green-700" },
      { part: "townhouse", x: 14, y: 72, scale: 0.9 },
      { part: "townhouse", x: 86, y: 72, scale: 0.9, flip: -1 },
    ],
    mid: [{ part: "street", x: 50, y: 74, paint: "wood-700" }],
    subject: [{ part: "tank", x: 50, y: 86, scale: 1.8 }],
    near: [{ part: "fence-post", x: 8, y: 100, scale: 1.4 }],
    light: [{ part: "rim-light", x: 50, y: 76, scale: 1, paint: "green-200" }],
    particles: [
      { part: "rain", x: 50, y: 40, scale: 1.4, paint: "slate-100" },
      { part: "dust", x: 40, y: 90, scale: 1, paint: "wood-700" },
    ],
  },
};

/** Jlockeed's Yamato Cannon (Stinkyscape #128, Rare), Alt art: the great cannon warms up on a battleship's bow above an ocean world's violet sunset, crew scrambling clear of its glowing muzzle; wide side view. */
const YAMATO_CANNON_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["violet-500", "orange-400"],
  light: { x: 26, y: 62, r: 36, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 72, y: 54.5, scale: 0.14, pose: "pose-run", look: { ...figureLookFor(["Human"]), item: null, cloth: "slate-100" } },
    { layer: "subject", x: 66, y: 55, scale: 0.13, flip: -1, pose: "pose-run", look: { ...figureLookFor(["Human"]), hair: "steel-helmet", item: null, cloth: "slate-100" } },
  ],
  layers: {
    sky: [
      { part: "sun-disc", x: 26, y: 62, scale: 0.9 },
      { part: "cloud", x: 70, y: 28, scale: 1.2, paint: "violet-300" },
    ],
    far: [{ part: "sea", x: 50, y: 70, paint: "violet-700" }],
    subject: [
      { part: "starship", x: 44, y: 62, scale: 1.4 },
      { part: "great-cannon", x: 50, y: 55, scale: 0.75 },
    ],
    light: [{ part: "halo", x: 88, y: 50, scale: 0.8, paint: "cyan-400" }],
    particles: [{ part: "sparks", x: 88, y: 50, scale: 0.6, paint: "white" }],
  },
};

/** A farmer of #129's moon, looking up from the field. */
const FARMER: FigureLook = { ...figureLookFor(["Human"]), hair: "hat-brim", outfit: "outfit-overalls", item: null, skin: "wood-700", cloth: "blue-800" };

/** Jlockeed's Missile Pod (Stinkyscape #129, Epic), Alt art: the tiny pod bolted onto a sleepy cargo hauler fires a fan of rockets over a quiet farming moon at dusk, farmers looking up from the fields below. */
const MISSILE_POD_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["blue-800", "red-300"],
  light: { x: 76, y: 66, r: 36, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  extras: [
    { layer: "mid", x: 30, y: 84, scale: 0.3, pose: "pose-point", look: FARMER },
    { layer: "mid", x: 60, y: 86, scale: 0.32, flip: -1, pose: "pose-stand", look: { ...FARMER, hair: "kerchief", cloth: "red-600" } },
  ],
  layers: {
    sky: [{ part: "stars", x: 50, y: 18, scale: 1 }],
    far: [
      { part: "hills", x: 50, y: 66, paint: "green-900" },
      { part: "barn", x: 82, y: 72, scale: 0.8 },
    ],
    mid: [{ part: "furrows", x: 50, y: 76 }],
    subject: [
      { part: "freighter", x: 36, y: 36, scale: 0.9 },
      { part: "missile-pod", x: 36, y: 27, scale: 0.6 },
      { part: "missile", x: 56, y: 16, scale: 0.6 },
      { part: "missile", x: 64, y: 24, scale: 0.6 },
      { part: "missile", x: 68, y: 34, scale: 0.6 },
    ],
    near: [
      { part: "wheat", x: 10, y: 98, scale: 1.4 },
      { part: "wheat", x: 92, y: 98, scale: 1.2 },
    ],
    light: [{ part: "halo", x: 36, y: 27, scale: 0.6, paint: "orange-400" }],
    particles: [{ part: "smoke", x: 46, y: 24, scale: 0.6, paint: "slate-400" }],
  },
};

/** Defensive Crystal (Stinkyscape #132, Common), Alt art: the crystal grows a new facet on a ship's prow as it rams through debris in a pink ice storm; tight close-up on the fresh facet forming. */
const DEFENSIVE_CRYSTAL_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-300", "violet-300"],
  light: { x: 62, y: 40, r: 30, paint: "white", opacity: 0.4 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  layers: {
    mid: [{ part: "starship", x: 10, y: 62, scale: 2.4 }],
    subject: [
      { part: "crystal", x: 72, y: 66, scale: 1.6, paint: "sky-300" },
      { part: "crystal", x: 76, y: 58, scale: 0.8, paint: "white" },
    ],
    near: [
      { part: "ice-shard", x: 88, y: 30, scale: 1.2, flip: -1 },
      { part: "ice-shard", x: 86, y: 84, scale: 1, flip: -1 },
    ],
    particles: [
      { part: "snow", x: 50, y: 40, scale: 1.4, paint: "white" },
      { part: "sparks", x: 76, y: 56, scale: 0.8, paint: "white" },
    ],
  },
};

/** Ultra Capacitor (Stinkyscape #133, Common), Alt art: a miner uses the capacitor as a lunchbox-sized lamp to light a dark cave camp, lightning arcing between stalactites; electric violet, close framing. */
const ULTRA_CAPACITOR_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["violet-700", "purple-400"],
  light: { x: 58, y: 70, r: 36, paint: "violet-300", opacity: 0.45 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  figure: {
    x: 34,
    y: 98,
    scale: 1.05,
    pose: "pose-kneel",
    look: { ...figureLookFor(["Human"]), hair: "space-helmet", outfit: "outfit-overalls", item: null, skin: "brass-600", cloth: "orange-400" },
  },
  layers: {
    far: [{ part: "cave", x: 50, y: 50 }],
    mid: [{ part: "camp-tent", x: 82, y: 86, scale: 1.2 }],
    subject: [{ part: "capacitor-cell", x: 58, y: 88, scale: 1.2 }],
    light: [
      { part: "lightning", x: 30, y: 12, scale: 0.6, paint: "violet-300" },
      { part: "lightning", x: 70, y: 10, scale: 0.5, paint: "white" },
      { part: "halo", x: 58, y: 76, scale: 1.4, paint: "violet-300" },
    ],
    particles: [{ part: "sparks", x: 58, y: 70, scale: 0.8, paint: "white" }],
  },
};

/** Sacked (Stinkyscape #135, Common), Alt art: later: the tied sack slung over a smuggler's shoulder through a crowded desert caravan stop, something kicking inside; dusty terracotta daylight. */
const SACKED_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-600", "orange-400"],
  light: { x: 76, y: 26, r: 34, paint: "white", opacity: 0.3 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 76, y: 24, scale: 0.6 }],
    far: [
      { part: "sand", x: 50, y: 66, paint: "orange-400" },
      { part: "tent", x: 82, y: 72, scale: 1 },
      { part: "awning", x: 18, y: 74, scale: 1, paint: "red-600" },
    ],
    mid: [{ part: "crew-crowd", x: 62, y: 82, scale: 1.4 }],
    subject: [
      { part: "alien", x: 40, y: 96, scale: 1.6 },
      { part: "kicking-sack", x: 47, y: 68, scale: 1.1 },
    ],
    particles: [{ part: "dust", x: 50, y: 90, scale: 1.2, paint: "red-300" }],
  },
};

/** An engineer of #136's engine room, baffled. */
const ENGINEER: FigureLook = { ...figureLookFor(["Human"]), hair: "goggles", outfit: "outfit-overalls", item: null, skin: "red-300", cloth: "orange-400" };

/** Magic Module (Stinkyscape #136, Common), Alt art: a bearded wizard in starry robes installs the module himself in a cramped engine room, motes flying while baffled engineers watch; deep teal and plum. His robe is plain, the stars the motes about him. */
const MAGIC_MODULE_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["green-700", "violet-700"],
  light: { x: 50, y: 56, r: 34, paint: "violet-300", opacity: 0.4 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  figure: {
    x: 32,
    y: 94,
    scale: 1.05,
    pose: "pose-cast",
    look: { ...figureLookFor(["Human"]), hair: "beard", outfit: "outfit-robe", item: "item-wand", skin: "red-300", cloth: "violet-700" },
  },
  extras: [
    { layer: "mid", x: 74, y: 88, scale: 0.7, flip: -1, pose: "pose-stand", look: ENGINEER },
    { layer: "mid", x: 88, y: 90, scale: 0.66, flip: -1, pose: "pose-hold", look: { ...ENGINEER, hair: "beanie", cloth: "blue-600" } },
  ],
  layers: {
    far: [
      { part: "bulkhead", x: 50, y: 72, paint: "slate-700" },
      { part: "pipes", x: 14, y: 44, scale: 0.8 },
    ],
    mid: [{ part: "reactor", x: 60, y: 76, scale: 0.8 }],
    subject: [{ part: "pod-capsule", x: 54, y: 58, scale: 1.2, paint: "violet-500" }],
    light: [{ part: "halo", x: 54, y: 58, scale: 1, paint: "violet-300" }],
    particles: [
      { part: "motes", x: 46, y: 50, scale: 1.2, paint: "violet-300" },
      { part: "sparks", x: 50, y: 56, scale: 0.8, paint: "white" },
    ],
  },
};

/** Starfleet Commander (Stinkyscape #137, Legendary), Alt art: a quiet close-up of the Commander on her bridge, chin on fist, studying a holographic map of four war theatres by rose dawn light through the viewport. The figure kit has no chin-on-fist pose, so she stands with her hands before her. */
const STARFLEET_COMMANDER_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-300", "violet-500"],
  light: { x: 70, y: 36, r: 40, paint: "red-300", opacity: 0.45 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  figure: { x: 34, y: 112, scale: 1.6, pose: "pose-hold", look: normalLookOf("stinkyscape-137") },
  layers: {
    sky: [
      { part: "sun", x: 70, y: 42, scale: 0.6 },
      { part: "stars", x: 50, y: 14, scale: 0.8 },
    ],
    far: [{ part: "viewport", x: 64, y: 40, scale: 1.6 }],
    mid: [{ part: "console", x: 88, y: 84, scale: 1 }],
    near: [{ part: "holo-table", x: 70, y: 108, scale: 1.4 }],
    light: [
      { part: "light-shaft", x: 64, y: 24, scale: 1.2, paint: "red-300" },
      { part: "rim-light", x: 34, y: 60, scale: 1.2, paint: "red-300" },
      { part: "halo", x: 70, y: 76, scale: 1, paint: "cyan-400" },
      { part: "planet", x: 70, y: 76, scale: 0.18, count: 4, spread: [10, 2], paint: "cyan-400" },
    ],
    particles: [{ part: "motes", x: 60, y: 60, scale: 1, paint: "cyan-400" }],
  },
};

/** Pocket Peptides (Stinkyscape #138, Common), Alt art: before the fight: the jade pocket vial rests on a lacquered table in a CN teahouse module, steam curling, a gloved hand reaching for it; soft jade morning. */
const POCKET_PEPTIDES_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["jade-400", "green-200"],
  light: { x: 74, y: 30, r: 36, paint: "white", opacity: 0.35 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "paper-screen", x: 22, y: 78, scale: 1.4 },
      { part: "round-window", x: 70, y: 42, scale: 1.4 },
    ],
    mid: [{ part: "wide-table", x: 50, y: 98, scale: 1.8, paint: "red-600" }],
    subject: [{ part: "vial", x: 50, y: 78, scale: 1.6, paint: "jade-400" }],
    near: [
      { part: "gloved-hand", x: 92, y: 72, scale: 1.1, flip: -1 },
      { part: "teapot", x: 26, y: 78, scale: 1 },
    ],
    particles: [{ part: "steam", x: 28, y: 70, scale: 0.8, paint: "white" }],
  },
};

/** Wormhole (Stinkyscape #139, Rare), Alt art: the far side of the wormhole: a stolen spell bursts out above a friendly harbour station, seen from inside the tunnel's rippling orange throat. */
const WORMHOLE_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["orange-400", "red-600"],
  light: { x: 50, y: 50, r: 40, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 50, scale: 1 }],
    subject: [{ part: "wormhole", x: 50, y: 50, scale: 2.6, paint: "orange-400" }],
    near: [{ part: "space-station", x: 50, y: 62, scale: 0.35 }],
    light: [
      { part: "magic-bolt", x: 46, y: 44, scale: 0.7, paint: "violet-300" },
      { part: "halo", x: 50, y: 46, scale: 1.2, paint: "white" },
    ],
    particles: [{ part: "sparks", x: 50, y: 42, scale: 1, paint: "white" }],
  },
};

/** Fruit Launcher (Stinkyscape #141, Rare), Alt art: the launcher's first orbital food delivery gone wrong: melons and pineapples splatter across a startled colony's greenhouse dome; wide shot, juicy pink and green. */
const FRUIT_LAUNCHER_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-300", "green-200"],
  light: { x: 30, y: 30, r: 34, paint: "white", opacity: 0.35 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "planet", x: 80, y: 20, scale: 0.6, paint: "green-400" }],
    far: [{ part: "regolith", x: 50, y: 74, paint: "stone-500" }],
    mid: [{ part: "colony-dome", x: 56, y: 82, scale: 1.4 }],
    subject: [
      { part: "launcher-tube", x: 12, y: 94, scale: 0.9 },
      { part: "melon", x: 48, y: 58, scale: 0.6 },
      { part: "pineapple", x: 66, y: 62, scale: 0.8 },
      { part: "melon", x: 76, y: 66, scale: 0.5 },
    ],
    particles: [
      { part: "drops", x: 54, y: 58, scale: 1.2, paint: "red-400" },
      { part: "drops", x: 70, y: 60, scale: 1, paint: "green-400" },
    ],
  },
};

/** Jade Launcher (Stinkyscape #142, Epic), Alt art: the jade launcher mounted on a junk-rigged sail ship riding the solar wind, firing glowing jade stones into a crimson nebula; wide profile shot. */
const JADE_LAUNCHER_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-900", "red-600"],
  light: { x: 76, y: 36, r: 36, paint: "red-400", opacity: 0.35 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "nebula", x: 70, y: 40, scale: 1.8, paint: "red-400" },
      { part: "stars", x: 50, y: 30, scale: 1.2 },
    ],
    far: [{ part: "asteroid", x: 82, y: 70, scale: 0.6, count: 3, spread: [10, 8] }],
    mid: [{ part: "gust", x: 26, y: 42, scale: 1.4, paint: "white" }],
    subject: [
      { part: "boat", x: 40, y: 70, scale: 1.6 },
      { part: "launcher-tube", x: 52, y: 70, scale: 0.6, paint: "jade-400" },
      { part: "jade-boulder", x: 72, y: 38, scale: 0.4 },
      { part: "jade-boulder", x: 84, y: 26, scale: 0.3 },
    ],
    near: [{ part: "speed-streaks", x: 22, y: 84, scale: 1, paint: "red-300" }],
    light: [{ part: "halo", x: 72, y: 34, scale: 0.8, paint: "jade-400" }],
    particles: [{ part: "sparks", x: 66, y: 42, scale: 0.8, paint: "green-200" }],
  },
};

/** Flash Freeze (Stinkyscape #146, Rare), Alt art: the thaw: a frozen desert-planet market begins to melt in the hot sun, ice dripping off merchants and fruit stalls; dusty pink and sand tones, wide street view. */
const FLASH_FREEZE_ALT: SceneRecipe = {
  family: "stinkyscape",
  sky: ["red-300", "orange-400"],
  light: { x: 76, y: 24, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  figure: { x: 44, y: 86, scale: 0.55, pose: "pose-stand", look: normalLookOf("stinkyscape-146") },
  layers: {
    sky: [{ part: "sun", x: 76, y: 24, scale: 0.8 }],
    far: [
      { part: "sand", x: 50, y: 66, paint: "orange-400" },
      { part: "whitewashed-house", x: 14, y: 70, scale: 0.8 },
    ],
    mid: [
      { part: "awning", x: 24, y: 80, scale: 1, paint: "red-400" },
      { part: "awning", x: 78, y: 80, scale: 1, paint: "jade-400" },
    ],
    subject: [{ part: "ice-block", x: 78, y: 86, scale: 1.2 }],
    near: [{ part: "ice-block", x: 44, y: 88, scale: 1.1, fade: 0.7 }],
    particles: [
      { part: "drops", x: 44, y: 72, scale: 1, paint: "sky-300" },
      { part: "drops", x: 78, y: 74, scale: 0.8, paint: "sky-300" },
      { part: "steam", x: 60, y: 62, scale: 0.6, paint: "white" },
    ],
  },
};

/** R3406000: the alt recipes of #3406's cards 116 to 146, by id. */
export const STINKYSCAPE_91_ALT_RECIPES_2: Readonly<Record<string, SceneRecipe>> = {
  "stinkyscape-116": MEGABASE_ALT,
  "stinkyscape-117": CORE_SYSTEM_ALT,
  "stinkyscape-122": VAMP_ROUNDS_ALT,
  "stinkyscape-123": GOKUGAINS_ALT,
  "stinkyscape-124": SHIPBUILDING_GRANT_ALT,
  "stinkyscape-127": SIEGE_TANK_ALT,
  "stinkyscape-128": YAMATO_CANNON_ALT,
  "stinkyscape-129": MISSILE_POD_ALT,
  "stinkyscape-132": DEFENSIVE_CRYSTAL_ALT,
  "stinkyscape-133": ULTRA_CAPACITOR_ALT,
  "stinkyscape-135": SACKED_ALT,
  "stinkyscape-136": MAGIC_MODULE_ALT,
  "stinkyscape-137": STARFLEET_COMMANDER_ALT,
  "stinkyscape-138": POCKET_PEPTIDES_ALT,
  "stinkyscape-139": WORMHOLE_ALT,
  "stinkyscape-141": FRUIT_LAUNCHER_ALT,
  "stinkyscape-142": JADE_LAUNCHER_ALT,
  "stinkyscape-146": FLASH_FREEZE_ALT,
};
