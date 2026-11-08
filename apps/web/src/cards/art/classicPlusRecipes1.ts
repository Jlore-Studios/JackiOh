// R3390000: Classic+'s scene recipes, part 1 of three (#1 to #26 with #12's and #19's tokens), one per entry, each
// drawn from its brief in Classic's temple-stone look (ART.md, "Classic+ drawn as scenes"): its subject within x 30
// to 70 and y 25 to 85, its foreground near, its middle in the mid and subject layers and its background far, richer
// the rarer. Presentation only (CLAUDE.md rule 7): a recipe is read by the card's id alone.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** A temple raider of the Classic+ scenes: helmed, in mail, a spear in hand. */
const RAIDER: FigureLook = { ...figureLookFor(["Human"]), hair: "helm", outfit: "outfit-armor", item: "item-spear", cloth: "red-900" };
/** A raider flung off his feet: no helm, no spear. */
const LIFTED: FigureLook = { ...figureLookFor(["Human"]), item: null, cloth: "red-900" };
/** A cat-folk wedding guest, sashed in white. */
const WEDDING_FELINOR: FigureLook = { ...figureLookFor(["Felinor"]), item: null, cloth: "white" };

/** Doom Shroom (Classic+ #1, Epic): a charcoal toadstool the size of a shrine bursts in a sunken temple courtyard, its violet shockwave lifting raiders into dust. */
const DOOM_SHROOM: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 44, r: 36, paint: "violet-500", opacity: 0.45 },
  emblem: { x: 50, y: 72, size: 5, rotate: 0 },
  extras: [
    { layer: "mid", x: 22, y: 58, scale: 0.4, pose: "pose-spread", look: LIFTED },
    { layer: "mid", x: 79, y: 54, scale: 0.36, flip: -1, pose: "pose-spread", look: { ...LIFTED, cloth: "slate-700" } },
    { layer: "subject", x: 80, y: 92, scale: 0.6, pose: "pose-run", look: RAIDER },
  ],
  layers: {
    sky: [{ part: "cloud", x: 72, y: 24, scale: 0.8, count: 2, spread: [10, 3], paint: "slate-700" }],
    far: [
      { part: "ruin-wall", x: 13, y: 60, scale: 1.2, paint: "slate-700" },
      { part: "ruin-wall", x: 88, y: 60, scale: 1.1, flip: -1, paint: "slate-700" },
      { part: "column", x: 30, y: 60, scale: 0.9 },
      { part: "column", x: 70, y: 60, scale: 0.8 },
      { part: "fallen-column", x: 84, y: 63, scale: 0.8 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 62, paint: "slate-700" },
      { part: "shockwave", x: 50, y: 78, scale: 1.3, paint: "violet-500" },
      { part: "dust", x: 34, y: 74, scale: 1.4, paint: "stone-500" },
    ],
    subject: [{ part: "giant-mushroom", x: 50, y: 80, scale: 1.6 }],
    near: [
      { part: "fallen-column", x: 14, y: 97, scale: 1 },
      { part: "pebbles", x: 52, y: 97 },
      { part: "grass-tuft", x: 88, y: 98, count: 2, spread: [5, 1], paint: "slate-700" },
    ],
    light: [
      { part: "halo", x: 50, y: 40, scale: 2.6, paint: "violet-500" },
      { part: "rim-light", x: 50, y: 60, scale: 1.6, paint: "violet-300" },
    ],
    particles: [
      { part: "spores", x: 50, y: 36, scale: 1.4, paint: "violet-300" },
      { part: "sparks", x: 50, y: 34, scale: 1, count: 2, spread: [16, 6], paint: "violet-300" },
      { part: "embers", x: 50, y: 60, scale: 1.4, count: 2, spread: [24, 8], paint: "violet-300" },
    ],
  },
};

/** Groom Shroom (Classic+ #2, Epic): a pink mushroom springs under a raider's boot in a flowered cloister, cat-folk in wedding sashes leaping out. */
const GROOM_SHROOM: SceneRecipe = {
  family: "classic",
  sky: ["violet-300", "red-300"],
  light: { x: 26, y: 34, r: 34, paint: "red-300", opacity: 0.4 },
  emblem: { x: 50, y: 32, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 34, y: 62, scale: 0.38, pose: "pose-spread", look: WEDDING_FELINOR },
    { layer: "subject", x: 66, y: 58, scale: 0.36, flip: -1, pose: "pose-raise", look: { ...WEDDING_FELINOR, skin: "stone-500", cloth: "red-300" } },
    { layer: "subject", x: 50, y: 50, scale: 0.32, pose: "pose-run", look: { ...WEDDING_FELINOR, skin: "wood-600" } },
    { layer: "subject", x: 78, y: 90, scale: 0.58, flip: -1, pose: "pose-run", look: { ...RAIDER, hair: "hood" } },
  ],
  layers: {
    sky: [{ part: "cloud", x: 78, y: 22, scale: 0.7, paint: "white" }],
    far: [
      { part: "column", x: 14, y: 64, scale: 0.9 },
      { part: "column", x: 86, y: 64, scale: 0.9 },
      { part: "arch", x: 50, y: 64, scale: 1.5, paint: "stone-500" },
      { part: "flowers", x: 44, y: 31, scale: 0.9, count: 2, spread: [12, 3] },
    ],
    mid: [
      { part: "field", x: 50, y: 64, paint: "green-700" },
      { part: "bush", x: 18, y: 74, scale: 0.9 },
      { part: "flowers", x: 76, y: 76, count: 2, spread: [10, 3] },
    ],
    subject: [{ part: "giant-mushroom", x: 50, y: 82, scale: 1.25, paint: "red-300" }],
    near: [
      { part: "mushroom", x: 16, y: 97, scale: 2.6, paint: "red-300" },
      { part: "spores", x: 50, y: 90, scale: 1.6, paint: "red-300" },
    ],
    light: [
      { part: "halo", x: 50, y: 58, scale: 2.4, paint: "red-300" },
      { part: "light-shaft", x: 24, y: 0, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 50, scale: 1.2, count: 2, spread: [14, 8], paint: "violet-300" },
      { part: "petals", x: 50, y: 40, scale: 1.2, paint: "red-300" },
    ],
  },
};

/** Second Amendment Snake (Classic+ #3, Rare): a plague-sored rattlesnake coiled on a cracked temple step among notched tally stones, rattle raised. */
const SECOND_AMENDMENT_SNAKE: SceneRecipe = {
  family: "classic",
  sky: ["blue-200", "stone-500"],
  light: { x: 50, y: 22, r: 40, paint: "white", opacity: 0.35 },
  emblem: { x: 50, y: 36, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 80, y: 20, scale: 0.9 }],
    far: [
      { part: "hills", x: 50, y: 58, paint: "stone-500" },
      { part: "temple", x: 50, y: 58, scale: 0.8 },
      { part: "standing-stone", x: 14, y: 62, scale: 0.8 },
      { part: "standing-stone", x: 86, y: 62, scale: 0.7 },
    ],
    mid: [{ part: "stairs", x: 50, y: 58, paint: "stone-500" }],
    subject: [{ part: "snake", x: 50, y: 82, scale: 2.4 }],
    near: [
      { part: "pebbles", x: 26, y: 95, count: 2, spread: [10, 2] },
      { part: "pebbles", x: 78, y: 96 },
      { part: "standing-stone", x: 12, y: 99, scale: 0.5 },
      { part: "standing-stone", x: 90, y: 98, scale: 0.45, flip: -1 },
      { part: "grass-tuft", x: 70, y: 99, count: 2, spread: [6, 1], paint: "wood-600" },
    ],
    light: [
      { part: "halo", x: 80, y: 20, scale: 2, paint: "white" },
      { part: "rim-light", x: 50, y: 68, scale: 1.4, paint: "orange-400" },
    ],
    particles: [{ part: "dust", x: 50, y: 86, scale: 1.4, paint: "stone-500" }],
  },
};

/** Juhan Biggest Bat (Classic+ #4, Common): Juhan, a huge bat in a red silk scholar's jacket, perched smugly on a heap of lookalike bats in a lantern-lit cave shrine. */
const JUHAN_BIGGEST_BAT: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "wood-800"],
  light: { x: 50, y: 42, r: 32, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 81, y: 75, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 72,
    scale: 1,
    pose: "pose-sit",
    look: { ...figureLookFor(["CN"]), head: "head-bat", hair: null, skin: "slate-700", cloth: "red-600" },
  },
  layers: {
    far: [
      { part: "cave", x: 50, y: 30, paint: "slate-800" },
      { part: "lantern", x: 24, y: 22 },
      { part: "lantern", x: 76, y: 20 },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 70, paint: "wood-800" },
      { part: "altar", x: 81, y: 82, scale: 1 },
      { part: "grave-mound", x: 50, y: 84, scale: 3, paint: "stone-500" },
    ],
    subject: [{ part: "bat", x: 50, y: 78, scale: 1.1, count: 8, spread: [14, 4] }],
    near: [{ part: "bat", x: 50, y: 93, scale: 1.3, count: 4, spread: [32, 3] }],
    light: [
      { part: "halo", x: 24, y: 26, scale: 1.2, paint: "orange-400" },
      { part: "halo", x: 76, y: 24, scale: 1.2, paint: "orange-400" },
    ],
    particles: [{ part: "embers", x: 50, y: 50, count: 2, spread: [20, 10] }],
  },
};

/** Guy Att (Classic+ #5, Common): Guy Att, a burly man in a leather apron, smashing his own stone drawers with a mallet in a dusty temple storeroom. */
const GUY_ATT: SceneRecipe = {
  family: "classic",
  sky: ["slate-800", "stone-500"],
  light: { x: 30, y: 34, r: 30, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 16, y: 75, size: 5, rotate: 0 },
  figure: {
    x: 42,
    y: 86,
    scale: 1,
    pose: "pose-strike",
    look: { ...figureLookFor(["Human"]), outfit: "outfit-muscle", item: "item-mallet", cloth: "wood-700" },
  },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 64, paint: "slate-700" },
      { part: "cabinet", x: 86, y: 66, scale: 0.9 },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 64, paint: "wood-800" },
      { part: "crate", x: 16, y: 82, scale: 1.4 },
      { part: "crate", x: 84, y: 80, scale: 1.2, count: 2, spread: [5, 1] },
    ],
    subject: [{ part: "cabinet", x: 64, y: 86, scale: 1.7 }],
    near: [{ part: "pebbles", x: 64, y: 95, count: 2, spread: [14, 2] }],
    light: [{ part: "light-shaft", x: 30, y: 0, paint: "white" }],
    particles: [
      { part: "sparks", x: 64, y: 70, scale: 1.2, count: 3, spread: [8, 6], paint: "wood-600" },
      { part: "dust", x: 60, y: 86, scale: 1.2, paint: "stone-500" },
    ],
  },
};

/** Wrong-House Attacker (Classic+ #6, Common): a scrawny hooded burglar with a poison-dripping dagger kicks in the wrong cottage door, a startled cat inside. */
const WRONG_HOUSE_ATTACKER: SceneRecipe = {
  family: "classic",
  sky: ["blue-900", "orange-400"],
  light: { x: 20, y: 52, r: 30, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 74, y: 54, size: 5, rotate: 0 },
  figure: {
    x: 38,
    y: 86,
    scale: 0.95,
    pose: "pose-run",
    look: { ...figureLookFor(["Human"]), hair: "hood", outfit: "outfit-coat", item: "item-dagger", cloth: "slate-800" },
  },
  layers: {
    sky: [{ part: "moon", x: 20, y: 22, scale: 0.5 }],
    far: [
      { part: "hills", x: 50, y: 64, paint: "slate-800" },
      { part: "tree", x: 12, y: 68, scale: 1.1, paint: "green-900" },
    ],
    mid: [
      { part: "field", x: 50, y: 76, paint: "green-900" },
      { part: "house", x: 66, y: 82, scale: 1.5 },
      { part: "doorway", x: 66, y: 82, scale: 1.4 },
      { part: "cat", x: 66, y: 80, scale: 0.8, flip: -1 },
    ],
    near: [
      { part: "grass-tuft", x: 18, y: 97, count: 3, spread: [14, 2] },
      { part: "bush", x: 90, y: 99, scale: 0.9 },
    ],
    light: [{ part: "halo", x: 66, y: 74, scale: 1.3, paint: "orange-400" }],
    particles: [{ part: "dust", x: 54, y: 86, scale: 0.9, paint: "stone-500" }],
  },
};

/** The House (Classic+ #7, Rare): a crooked stone townhouse at a temple lane's end, two doors side by side, one barred and guarded, one ajar, under plague moss. */
const THE_HOUSE: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "slate-400"],
  light: { x: 76, y: 70, r: 26, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 50, y: 58, size: 5, rotate: 0 },
  extras: [{ layer: "subject", x: 28, y: 88, scale: 0.6, pose: "pose-guard", look: RAIDER }],
  layers: {
    sky: [{ part: "cloud", x: 30, y: 20, scale: 1, count: 3, spread: [30, 4], paint: "slate-400" }],
    far: [
      { part: "house", x: 12, y: 66, scale: 0.9, paint: "slate-700" },
      { part: "house", x: 88, y: 66, scale: 0.9, flip: -1, paint: "slate-700" },
    ],
    mid: [
      { part: "street", x: 50, y: 80, paint: "stone-500" },
      { part: "vine", x: 30, y: 72, count: 2, spread: [3, 4], paint: "green-400" },
    ],
    subject: [
      { part: "house", x: 50, y: 80, scale: 1.8 },
      { part: "door", x: 44, y: 80, scale: 1.3 },
      { part: "doorway", x: 56, y: 80, scale: 1.3 },
      { part: "chain", x: 39, y: 72, scale: 0.5 },
      { part: "padlock", x: 44, y: 75, scale: 0.9 },
      { part: "bush", x: 66, y: 82, scale: 0.7, paint: "green-400" },
    ],
    near: [
      { part: "fence-post", x: 74, y: 92 },
      { part: "lantern", x: 78.8, y: 80.2 },
      { part: "mushroom", x: 20, y: 97, count: 3, spread: [6, 1], paint: "green-400" },
    ],
    light: [
      { part: "halo", x: 78.8, y: 84.4, scale: 1.1, paint: "orange-400" },
      { part: "halo", x: 56, y: 74, scale: 1, paint: "orange-400" },
    ],
    particles: [
      { part: "spores", x: 36, y: 70, scale: 1.2, paint: "green-400" },
      { part: "mist", x: 50, y: 86, scale: 0.8, paint: "green-200" },
    ],
  },
};

/** Withering Storm (Classic+ #8, Rare): a dust-brown storm swirls over standing stones, the cards on the central stone curling and fading as the wind ages them. */
const WITHERING_STORM: SceneRecipe = {
  family: "classic",
  sky: ["wood-800", "stone-500"],
  light: { x: 50, y: 42, r: 36, paint: "stone-500", opacity: 0.35 },
  emblem: { x: 50, y: 77, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 28, y: 20, scale: 1.2, count: 3, spread: [28, 4], paint: "wood-700" }],
    far: [
      { part: "hills", x: 50, y: 62, paint: "wood-800" },
      { part: "standing-stone", x: 16, y: 68, scale: 1.1 },
      { part: "standing-stone", x: 84, y: 68, scale: 1 },
      { part: "standing-stone", x: 31, y: 65, scale: 0.7 },
      { part: "standing-stone", x: 69, y: 65, scale: 0.7, flip: -1 },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 66, paint: "wood-700" },
      { part: "tornado", x: 50, y: 70, scale: 1.6, paint: "wood-600" },
    ],
    subject: [
      { part: "altar", x: 50, y: 84, scale: 1.6 },
      { part: "card", x: 44, y: 71, scale: 1, paint: "stone-500" },
      { part: "card", x: 51, y: 70, scale: 1.05, paint: "slate-400" },
      { part: "card", x: 58, y: 71, scale: 0.9, paint: "wood-600" },
    ],
    near: [
      { part: "standing-stone", x: 9, y: 101, scale: 1.3 },
      { part: "standing-stone", x: 91, y: 100, scale: 1.2, flip: -1 },
      { part: "dust", x: 50, y: 96, scale: 1.6, paint: "wood-600" },
    ],
    light: [{ part: "halo", x: 50, y: 66, scale: 2, paint: "stone-500" }],
    particles: [
      { part: "dust", x: 50, y: 48, scale: 1.2, count: 2, spread: [20, 10], paint: "wood-600" },
      { part: "petals", x: 50, y: 44, scale: 1.3, paint: "wood-600" },
    ],
  },
};

/** Silence (Classic+ #9, Common): a hooded monk presses a finger to a stone warrior's lips in a dim chapel, its runes guttering out like snuffed candles. */
const SILENCE: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "blue-900"],
  light: { x: 54, y: 38, r: 30, paint: "blue-200", opacity: 0.35 },
  emblem: { x: 84, y: 77, size: 5, rotate: 0 },
  figure: {
    x: 59,
    y: 86,
    scale: 1.05,
    flip: -1,
    pose: "pose-stand",
    look: { head: "head-golem", hair: null, outfit: "outfit-stone", item: "item-sword", skin: "stone-500", cloth: "slate-700" },
  },
  extras: [
    {
      layer: "subject",
      x: 42,
      y: 86,
      scale: 0.9,
      pose: "pose-raise",
      look: { ...figureLookFor([]), outfit: "outfit-robe", skin: "red-300", cloth: "slate-700" },
    },
  ],
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 66, paint: "slate-800" }],
    far: [
      { part: "arch", x: 50, y: 66, scale: 1.6, paint: "slate-700" },
      { part: "column", x: 12, y: 68, scale: 1.1 },
      { part: "column", x: 88, y: 68, scale: 1.1 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66, paint: "slate-700" },
      { part: "rune-circle", x: 59, y: 86, scale: 0.7, paint: "slate-700" },
      { part: "altar", x: 84, y: 84, scale: 1 },
      { part: "candle", x: 22, y: 82, scale: 1.2, count: 2, spread: [4, 1] },
    ],
    light: [
      { part: "halo", x: 56, y: 36, scale: 1.6, paint: "blue-200" },
      { part: "light-shaft", x: 50, y: 0, paint: "blue-200" },
    ],
    particles: [
      { part: "smoke", x: 22, y: 74, scale: 1 },
      { part: "motes", x: 56, y: 50, scale: 1.2, paint: "blue-200" },
    ],
  },
};

/** New Wraps (Classic+ #10, Common): fresh linen bandages wind themselves round a weary mummy on a tomb slab, a faint teal glow of renewal in a lamp-lit crypt. */
const NEW_WRAPS: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "slate-800"],
  light: { x: 50, y: 56, r: 30, paint: "cyan-400", opacity: 0.35 },
  emblem: { x: 81, y: 75, size: 5, rotate: 0 },
  figure: { x: 50, y: 82, scale: 1, pose: "pose-sit", look: { head: "head-mummy", hair: null, outfit: "outfit-wraps", item: null, skin: "slate-100", cloth: "slate-100" } },
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 66, paint: "slate-800" }],
    far: [
      { part: "arch", x: 50, y: 66, scale: 1.5, paint: "slate-700" },
      { part: "lantern", x: 20, y: 24 },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 66, paint: "slate-700" },
      { part: "tombstone", x: 81, y: 82, scale: 1.8 },
      { part: "tombstone", x: 16, y: 80, scale: 1.4 },
    ],
    subject: [{ part: "cot", x: 50, y: 86, scale: 2.2, paint: "stone-500" }],
    near: [{ part: "bones", x: 86, y: 97, scale: 1.1 }],
    light: [
      { part: "light-ribbon", x: 50, y: 58, scale: 0.6, paint: "white" },
      { part: "halo", x: 50, y: 56, scale: 2.2, paint: "cyan-400" },
      { part: "halo", x: 20, y: 28, scale: 1.2, paint: "orange-400" },
    ],
    particles: [{ part: "sparks", x: 50, y: 50, scale: 1, count: 2, spread: [10, 8], paint: "cyan-400" }],
  },
};

/** Anime Armor (Classic+ #11, Rare): a young knight in oversized gleaming plate, a sword bouncing off his chest in a ruined colosseum under a heroic sky. */
const ANIME_ARMOR: SceneRecipe = {
  family: "classic",
  sky: ["blue-700", "sky-300"],
  light: { x: 46, y: 50, r: 34, paint: "white", opacity: 0.4 },
  emblem: { x: 16, y: 70, size: 5, rotate: 0 },
  figure: {
    x: 45,
    y: 86,
    scale: 1.05,
    pose: "pose-guard",
    look: { ...figureLookFor([]), hair: "hair-short", outfit: "outfit-plate", item: "item-sword", cloth: "slate-400" },
  },
  extras: [
    {
      layer: "subject",
      x: 72,
      y: 88,
      scale: 0.72,
      flip: -1,
      pose: "pose-strike",
      look: { ...figureLookFor(["Human"]), hair: "hood", item: "item-sword", cloth: "red-900" },
    },
  ],
  layers: {
    sky: [{ part: "cloud", x: 24, y: 22, scale: 0.9, count: 2, spread: [16, 4] }],
    far: [
      { part: "arch", x: 10, y: 60, scale: 0.75, paint: "stone-500" },
      { part: "arch", x: 30, y: 58, scale: 0.7, paint: "stone-500" },
      { part: "arch", x: 70, y: 58, scale: 0.7, paint: "stone-500" },
      { part: "arch", x: 90, y: 60, scale: 0.75, paint: "stone-500" },
      { part: "ruin-wall", x: 16, y: 72, scale: 1 },
    ],
    mid: [
      { part: "field", x: 50, y: 62, paint: "wood-600" },
      { part: "dust", x: 70, y: 80, scale: 1.2, paint: "stone-500" },
    ],
    near: [{ part: "fallen-column", x: 86, y: 97, scale: 0.9, flip: -1 }],
    light: [
      { part: "god-rays", x: 46, y: -4 },
      { part: "rim-light", x: 45, y: 58, scale: 1.4, paint: "white" },
    ],
    particles: [{ part: "sparks", x: 53, y: 55, scale: 1.4, count: 2, spread: [4, 3], paint: "white" }],
  },
};

/** The Mother Pancake (Classic+ #12, Legendary): a towering matronly stack in a flour-dusted apron spreads her arms as a shield before a temple kitchen's stone hearth, flipping a pancake child toward a waiting hand. */
const THE_MOTHER_PANCAKE: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "blue-800"],
  light: { x: 50, y: 62, r: 38, paint: "ember-500", opacity: 0.4 },
  emblem: { x: 16, y: 44, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 88,
    scale: 1.1,
    pose: "pose-spread",
    look: { ...figureLookFor(["Pancake"]), hair: null, outfit: "outfit-apron", skin: "brass-600", cloth: "brass-500" },
  },
  extras: [
    { layer: "subject", x: 76, y: 34, scale: 0.26, pose: "pose-spread", look: { ...figureLookFor(["Pancake"]), hair: null, cloth: "orange-400" } },
    { layer: "mid", x: 86, y: 80, scale: 0.42, flip: -1, pose: "pose-raise", look: { ...figureLookFor(["Pancake"]), cloth: "blue-200" } },
  ],
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 66, paint: "blue-800" }],
    far: [
      { part: "banner", x: 14, y: 62, scale: 1.3 },
      { part: "column", x: 30, y: 70, scale: 1.1 },
      { part: "column", x: 70, y: 70, scale: 1.1 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66, paint: "slate-700" },
      { part: "hearth", x: 50, y: 76, scale: 2.1 },
    ],
    near: [
      { part: "table", x: 14, y: 98, scale: 1.1 },
      { part: "pancakes", x: 14, y: 90, scale: 1.2 },
      { part: "pancakes", x: 88, y: 97, scale: 1.5 },
    ],
    light: [
      { part: "halo", x: 33, y: 72, scale: 1.3, paint: "ember-500" },
      { part: "halo", x: 67, y: 72, scale: 1.3, paint: "ember-500" },
      { part: "rim-light", x: 50, y: 58, scale: 1.6, paint: "blue-200" },
      { part: "light-shaft", x: 76, y: 0, paint: "blue-200" },
    ],
    particles: [
      { part: "smoke", x: 50, y: 30, scale: 1.2, count: 2, spread: [16, 4], paint: "white" },
      { part: "embers", x: 28, y: 64, scale: 1.6, count: 2, spread: [5, 8], paint: "ember-500" },
      { part: "embers", x: 72, y: 64, scale: 1.6, count: 2, spread: [5, 8], paint: "ember-500" },
      { part: "sparks", x: 76, y: 36, scale: 0.9, paint: "orange-400" },
    ],
  },
};

/** Devour (Classic+ #12.1, Token): a pancake-headed glutton swallows a stone soldier whole at a banquet table, clutching a bloated, aching belly. */
const DEVOUR: SceneRecipe = {
  family: "classic",
  sky: ["wood-900", "wood-700"],
  light: { x: 40, y: 70, r: 30, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 86, y: 40, size: 4, rotate: 0 },
  figure: {
    x: 50,
    y: 88,
    scale: 1,
    pose: "pose-raise",
    look: { ...figureLookFor(["Pancake"]), hair: null, outfit: "outfit-drum", skin: "brass-600", cloth: "red-600" },
  },
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 66, paint: "wood-700" }],
    far: [
      { part: "banner", x: 84, y: 62, scale: 1.3 },
      { part: "candle", x: 16, y: 46, scale: 1.2 },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 66, paint: "wood-800" }],
    subject: [{ part: "statue", x: 61, y: 33, scale: 0.62 }],
    near: [
      { part: "table", x: 50, y: 100, scale: 2.4 },
      { part: "bread", x: 30, y: 81, scale: 1.2 },
      { part: "pancakes", x: 70, y: 81, scale: 1.1 },
      { part: "candle", x: 40, y: 81, scale: 1.1 },
    ],
    light: [
      { part: "halo", x: 40, y: 77, scale: 1.4, paint: "orange-400" },
      { part: "halo", x: 16, y: 42, scale: 1.2, paint: "orange-400" },
    ],
    particles: [{ part: "sparks", x: 56, y: 40, scale: 1, paint: "orange-400" }],
  },
};

/** Death Boil (Classic+ #12.2, Token): a cauldron of boiling batter in a hermit's cave kitchen spits green bubbles that heal a friend and scald an enemy. */
const DEATH_BOIL: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "green-900"],
  light: { x: 50, y: 58, r: 34, paint: "green-400", opacity: 0.4 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  extras: [
    { layer: "mid", x: 20, y: 84, scale: 0.55, pose: "pose-raise", look: { ...figureLookFor(["Pancake"]), cloth: "green-700" } },
    { layer: "mid", x: 82, y: 86, scale: 0.55, pose: "pose-run", look: RAIDER },
  ],
  layers: {
    far: [
      { part: "cave", x: 50, y: 34, paint: "slate-800" },
      { part: "lantern", x: 30, y: 24 },
    ],
    mid: [{ part: "rock-floor", x: 50, y: 68, paint: "slate-700" }],
    subject: [
      { part: "flames", x: 50, y: 86, scale: 1.5 },
      { part: "cauldron", x: 50, y: 82, scale: 2.2 },
    ],
    near: [{ part: "bones", x: 14, y: 97, scale: 1 }],
    light: [
      { part: "halo", x: 50, y: 58, scale: 2.4, paint: "green-400" },
      { part: "halo", x: 20, y: 60, scale: 1.2, paint: "green-200" },
    ],
    particles: [
      { part: "bubbles", x: 50, y: 48, scale: 1.4, paint: "green-400" },
      { part: "smoke", x: 50, y: 50, scale: 1.2, paint: "green-200" },
      { part: "sparks", x: 22, y: 56, scale: 1, paint: "green-200" },
      { part: "embers", x: 80, y: 62, scale: 1.4, count: 2, spread: [6, 4], paint: "ember-500" },
    ],
  },
};

/** Fluffy Grip (Classic+ #12.3, Token): a fluffy dough hand on a stretchy arm plucks a startled little soldier from a rival's satchel in a crowded caravanserai. */
const FLUFFY_GRIP: SceneRecipe = {
  family: "classic",
  sky: ["wood-700", "orange-400"],
  light: { x: 54, y: 50, r: 30, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 18, y: 55, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 49, y: 64, scale: 0.3, pose: "pose-spread", look: { ...figureLookFor(["Human"]), hair: "helm", outfit: "outfit-armor", item: null, cloth: "blue-700" } },
    {
      layer: "subject",
      x: 72,
      y: 88,
      scale: 0.72,
      flip: -1,
      pose: "pose-stand",
      look: { ...figureLookFor(["Human"]), hair: "hat-brim", outfit: "outfit-coat", item: "item-sack", cloth: "wood-700" },
    },
    { layer: "far", x: 84, y: 66, scale: 0.42, flip: -1, pose: "pose-stand", look: { ...figureLookFor(["Human"]), hair: "kerchief", item: null, cloth: "green-700" } },
  ],
  layers: {
    far: [
      { part: "arch", x: 50, y: 66, scale: 1.6, paint: "stone-500" },
      { part: "column", x: 30, y: 66, scale: 1 },
      { part: "column", x: 70, y: 66, scale: 1 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66, paint: "wood-700" },
      { part: "awning", x: 18, y: 76, scale: 1.1 },
      { part: "crate", x: 88, y: 84, scale: 1.3 },
    ],
    subject: [
      { part: "pale-hand", x: 47, y: 96, scale: 3.4, paint: "brass-500" },
    ],
    near: [{ part: "cart", x: 14, y: 97, scale: 1.1 }],
    light: [
      { part: "halo", x: 48, y: 52, scale: 1.6, paint: "orange-400" },
      { part: "light-shaft", x: 62, y: 0, paint: "white" },
    ],
    particles: [
      { part: "petals", x: 50, y: 30, scale: 1.4, paint: "green-700" },
      { part: "sparks", x: 52, y: 42, scale: 0.9, paint: "white" },
    ],
  },
};

/** A goblin raider, coughing in the flour. */
const GOBLIN: FigureLook = { head: "head-goblin", hair: null, outfit: "outfit-tunic", item: null, skin: "green-700", cloth: "wood-700" };

/** Powder Spray (Classic+ #12.4, Token): a bellows sprays a burst of white flour across a line of coughing goblin raiders inside a stone mill. */
const POWDER_SPRAY: SceneRecipe = {
  family: "classic",
  sky: ["slate-400", "slate-100"],
  light: { x: 46, y: 54, r: 36, paint: "white", opacity: 0.45 },
  emblem: { x: 80, y: 32, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 54, y: 84, scale: 0.55, flip: -1, pose: "pose-kneel", look: GOBLIN },
    { layer: "subject", x: 68, y: 82, scale: 0.5, flip: -1, pose: "pose-guard", look: { ...GOBLIN, cloth: "slate-700" } },
    { layer: "subject", x: 83, y: 85, scale: 0.55, pose: "pose-run", look: { ...GOBLIN, item: "item-dagger", cloth: "red-900" } },
  ],
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 66, paint: "stone-500" }],
    far: [
      { part: "gear", x: 80, y: 32, scale: 1.3 },
      { part: "gear", x: 68, y: 44, scale: 0.8 },
      { part: "wheat", x: 12, y: 66, count: 3, spread: [4, 1] },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66, paint: "wood-800" },
      { part: "crate", x: 14, y: 84, scale: 1.4 },
    ],
    subject: [
      { part: "bellows", x: 28, y: 58, scale: 1.8 },
      { part: "mist", x: 56, y: 58, scale: 1, count: 2, spread: [10, 4], paint: "white" },
      { part: "dust", x: 62, y: 64, scale: 1.5, paint: "white" },
    ],
    light: [
      { part: "light-shaft", x: 40, y: 0, paint: "white" },
      { part: "halo", x: 50, y: 58, scale: 2, paint: "white" },
    ],
    particles: [{ part: "motes", x: 62, y: 60, scale: 1.4, paint: "white" }],
  },
};

/** A pancake soldier of the camps, spear in hand. */
const PANCAKE_SOLDIER: FigureLook = { ...figureLookFor(["Pancake"]), hair: null, item: "item-spear", cloth: "red-600" };

/** Anti-Waffle Shell (Classic+ #12.5, Token): a pancake camp on a temple terrace under a shimmering waffle-grid dome of pale blue light, armed pancakes resting inside. */
const ANTI_WAFFLE_SHELL: SceneRecipe = {
  family: "classic",
  sky: ["blue-800", "orange-400"],
  light: { x: 50, y: 62, r: 34, paint: "sky-300", opacity: 0.4 },
  emblem: { x: 50, y: 50, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 38, y: 79, scale: 0.38, pose: "pose-sit", look: PANCAKE_SOLDIER },
    { layer: "subject", x: 60, y: 79, scale: 0.42, flip: -1, pose: "pose-guard", look: { ...PANCAKE_SOLDIER, item: "item-shield", cloth: "blue-800" } },
  ],
  layers: {
    sky: [
      { part: "sun", x: 16, y: 56, scale: 0.8 },
      { part: "cloud", x: 74, y: 24, scale: 0.8, paint: "red-300" },
    ],
    far: [
      { part: "hills", x: 50, y: 62, paint: "slate-800" },
      { part: "temple", x: 50, y: 64, scale: 0.8 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66, paint: "stone-500" },
      { part: "banner", x: 80, y: 80, scale: 0.8 },
    ],
    subject: [{ part: "flames", x: 49, y: 80, scale: 0.6 }],
    near: [{ part: "dome", x: 50, y: 81, scale: 1.55, paint: "blue-200" }],
    light: [
      { part: "halo", x: 49, y: 76, scale: 1.2, paint: "orange-400" },
      { part: "rim-light", x: 50, y: 64, scale: 1.8, paint: "sky-300" },
    ],
    particles: [{ part: "sparks", x: 50, y: 56, scale: 1.2, count: 3, spread: [22, 8], paint: "blue-200" }],
  },
};

/** Frozen Wastes (Classic+ #12.6, Token): a blizzard sweeps a long stone banquet table on a frozen mountain pass, frosted pancakes and diners freezing and shattering. */
const FROZEN_WASTES: SceneRecipe = {
  family: "classic",
  sky: ["blue-900", "blue-200"],
  light: { x: 50, y: 50, r: 36, paint: "white", opacity: 0.35 },
  emblem: { x: 50, y: 75, size: 5, rotate: 0 },
  extras: [
    { layer: "mid", x: 34, y: 72, scale: 0.5, pose: "pose-sit", look: { ...figureLookFor(["Pancake"]), skin: "blue-200", cloth: "slate-100" } },
    { layer: "mid", x: 66, y: 72, scale: 0.5, flip: -1, pose: "pose-sit", look: { ...figureLookFor(["Pancake"]), hair: null, skin: "sky-300", cloth: "blue-200" } },
  ],
  layers: {
    far: [{ part: "mountains", x: 50, y: 56, paint: "slate-700" }],
    mid: [{ part: "ice-floor", x: 50, y: 68 }],
    subject: [
      { part: "table", x: 50, y: 84, scale: 2.4, paint: "stone-500" },
      { part: "pancakes", x: 41, y: 65, scale: 1, paint: "slate-100" },
      { part: "pancakes", x: 59, y: 65, scale: 0.9, paint: "slate-100" },
    ],
    near: [
      { part: "crystal", x: 20, y: 96, scale: 2.2, count: 3, spread: [8, 2], paint: "blue-200" },
      { part: "crystal", x: 82, y: 97, scale: 2, count: 2, spread: [6, 2], paint: "blue-200" },
    ],
    light: [{ part: "halo", x: 50, y: 60, scale: 2.2, paint: "white" }],
    particles: [
      { part: "snow", x: 50, y: 44, scale: 1.2 },
      { part: "mist", x: 50, y: 80, scale: 1, paint: "white" },
    ],
  },
};

/** A pancake ghoul of the Legion, a fork raised. */
const PANCAKE_GHOUL: FigureLook = { ...figureLookFor(["Pancake"]), hair: null, outfit: "outfit-patched", item: "item-spear", skin: "stone-500", cloth: "slate-700" };

/** Legion of the Hungry (Classic+ #12.7, Token): a horde of ravenous pancake ghouls claws out of a ruined granary's flour bins, forks raised, pouring toward a feast hall. */
const LEGION_OF_THE_HUNGRY: SceneRecipe = {
  family: "classic",
  sky: ["black", "green-900"],
  light: { x: 40, y: 60, r: 34, paint: "green-400", opacity: 0.4 },
  emblem: { x: 80, y: 47, size: 4, rotate: 0 },
  figure: { x: 46, y: 88, scale: 0.85, pose: "pose-run", look: PANCAKE_GHOUL },
  extras: [
    { layer: "subject", x: 28, y: 80, scale: 0.6, pose: "pose-strike", look: { ...PANCAKE_GHOUL, cloth: "wood-700" } },
    { layer: "subject", x: 64, y: 78, scale: 0.55, pose: "pose-raise", look: { ...PANCAKE_GHOUL, skin: "green-700" } },
  ],
  layers: {
    far: [
      { part: "hills", x: 50, y: 62, paint: "slate-900" },
      { part: "temple", x: 80, y: 64, scale: 0.6 },
      { part: "ruin-wall", x: 18, y: 64, scale: 1.3, paint: "slate-800" },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 66, paint: "slate-800" },
      { part: "crate", x: 14, y: 82, scale: 1.8 },
      { part: "crate", x: 30, y: 84, scale: 1.5 },
      { part: "flames", x: 90, y: 72, scale: 0.7, paint: "green-400" },
    ],
    subject: [
      { part: "bone-hand", x: 14, y: 70, scale: 1 },
      { part: "pale-hand", x: 30, y: 74, scale: 0.9, paint: "stone-500" },
    ],
    near: [{ part: "wheat", x: 86, y: 99, count: 3, spread: [5, 1] }],
    light: [
      { part: "halo", x: 80, y: 58, scale: 1.4, paint: "orange-400" },
      { part: "halo", x: 90, y: 66, scale: 1.2, paint: "green-400" },
    ],
    particles: [
      { part: "embers", x: 50, y: 60, scale: 1.4, count: 2, spread: [20, 10], paint: "green-400" },
      { part: "motes", x: 24, y: 70, scale: 1.2, paint: "white" },
    ],
  },
};

/** Frostspatula (Classic+ #12.8, Token): a frost-rimed iron spatula planted on an ice-cave altar, ghosts of flipped pancakes circling above it. */
const FROSTSPATULA: SceneRecipe = {
  family: "classic",
  sky: ["blue-900", "cyan-400"],
  light: { x: 50, y: 50, r: 34, paint: "cyan-400", opacity: 0.45 },
  emblem: { x: 50, y: 77, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "cave", x: 50, y: 34, paint: "blue-900" }],
    mid: [{ part: "ice-floor", x: 50, y: 68 }],
    subject: [
      { part: "altar", x: 50, y: 86, scale: 1.8 },
      { part: "crystal", x: 50, y: 71, scale: 1.4, count: 3, spread: [6, 1], paint: "blue-200" },
      { part: "spatula", x: 50, y: 71, scale: 1.6, paint: "slate-400" },
    ],
    near: [
      { part: "crystal", x: 14, y: 97, scale: 2.4, count: 2, spread: [6, 2], paint: "sky-300" },
      { part: "crystal", x: 88, y: 96, scale: 2.2, paint: "sky-300" },
    ],
    light: [
      { part: "light-ribbon", x: 50, y: 38, scale: 1, paint: "cyan-400" },
      { part: "pancakes", x: 30, y: 41, scale: 1, paint: "sky-300" },
      { part: "pancakes", x: 70, y: 37, scale: 0.9, paint: "sky-300" },
      { part: "ghost", x: 58, y: 28, scale: 0.8, paint: "cyan-400" },
      { part: "halo", x: 50, y: 52, scale: 2, paint: "cyan-400" },
    ],
    particles: [{ part: "snow", x: 50, y: 44, scale: 1 }],
  },
};

/** Mommy Barker (Classic+ #13, Legendary): a stout baker in a flour-caked apron and kerchief barks an order, spoon raised, as a tray of pancakes leaps from her stone oven, apprentices scrambling behind. */
const MOMMY_BARKER: SceneRecipe = {
  family: "classic",
  sky: ["violet-700", "red-300"],
  light: { x: 20, y: 30, r: 34, paint: "red-300", opacity: 0.45 },
  emblem: { x: 75, y: 65, size: 5, rotate: 0 },
  figure: {
    x: 42,
    y: 88,
    scale: 1.08,
    pose: "pose-raise",
    look: { ...figureLookFor(["Human", "Pancake"]), head: "head-human", hair: "kerchief", outfit: "outfit-apron", item: "item-spoon", skin: "red-300", cloth: "blue-800" },
  },
  extras: [
    { layer: "mid", x: 88, y: 80, scale: 0.48, flip: -1, pose: "pose-run", look: { ...figureLookFor(["Human"]), hair: "toque", item: "item-tray", cloth: "slate-100" } },
    { layer: "mid", x: 16, y: 76, scale: 0.44, pose: "pose-run", look: { ...figureLookFor(["Human"]), hair: "hat-brim", outfit: "outfit-tunic", item: "item-sack", cloth: "green-700" } },
  ],
  layers: {
    sky: [{ part: "cloud", x: 80, y: 20, scale: 0.8, paint: "red-300" }],
    far: [
      { part: "arch", x: 50, y: 66, scale: 1.6, paint: "stone-500" },
      { part: "column", x: 10, y: 66, scale: 1 },
      { part: "column", x: 90, y: 66, scale: 1 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66, paint: "stone-500" },
      { part: "hearth", x: 74, y: 84, scale: 1.3 },
    ],
    subject: [
      { part: "pancakes", x: 70, y: 52, scale: 1.2 },
      { part: "pancakes", x: 82, y: 44, scale: 1 },
      { part: "pancakes", x: 62, y: 42, scale: 0.9 },
    ],
    near: [
      { part: "table", x: 12, y: 99, scale: 1.2 },
      { part: "bread", x: 12, y: 90, scale: 1.1 },
      { part: "crate", x: 90, y: 98, scale: 1.4 },
    ],
    light: [
      { part: "light-shaft", x: 22, y: 0, scale: 1.1, paint: "red-300" },
      { part: "halo", x: 74, y: 76, scale: 1.4, paint: "orange-400" },
      { part: "rim-light", x: 42, y: 58, scale: 1.4, paint: "red-300" },
    ],
    particles: [
      { part: "motes", x: 40, y: 44, scale: 1.6, paint: "white" },
      { part: "smoke", x: 72, y: 40, scale: 1, paint: "white" },
      { part: "sparks", x: 72, y: 48, scale: 1, paint: "orange-400" },
    ],
  },
};

/** Forever& (Classic+ #14, Epic): a glowing spell scroll flung from a mage's hand loops back along a ribbon of violet light, an endless figure-eight round a temple bell tower. */
const FOREVER_AND: SceneRecipe = {
  family: "classic",
  sky: ["blue-900", "violet-500"],
  light: { x: 50, y: 46, r: 36, paint: "violet-500", opacity: 0.45 },
  emblem: { x: 50, y: 62, size: 5, rotate: 0 },
  extras: [
    {
      layer: "near",
      x: 10,
      y: 104,
      scale: 1,
      pose: "pose-spread",
      look: { ...figureLookFor([]), hair: "hat-pointed", outfit: "outfit-robe", skin: "red-300", cloth: "violet-700" },
    },
  ],
  layers: {
    sky: [
      { part: "stars", x: 30, y: 18, scale: 1 },
      { part: "moon", x: 82, y: 20, scale: 0.45 },
    ],
    far: [{ part: "mountains", x: 50, y: 64, paint: "slate-800" }],
    mid: [
      { part: "field", x: 50, y: 78, paint: "slate-700" },
      { part: "tower", x: 50, y: 82, scale: 1.3 },
    ],
    subject: [
      { part: "light-ribbon", x: 50, y: 46, scale: 1.3, paint: "violet-500" },
      { part: "scroll", x: 72, y: 42, scale: 0.9 },
    ],
    near: [{ part: "grass-tuft", x: 80, y: 98, count: 3, spread: [10, 2], paint: "slate-700" }],
    light: [
      { part: "halo", x: 72, y: 40, scale: 1.4, paint: "violet-300" },
      { part: "halo", x: 50, y: 34, scale: 1, paint: "brass-500" },
      { part: "rim-light", x: 50, y: 52, scale: 1.4, paint: "violet-300" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 46, scale: 1.2, count: 3, spread: [20, 6], paint: "violet-300" },
      { part: "motes", x: 50, y: 30, scale: 1.2, paint: "violet-300" },
    ],
  },
};

/** Conjure Rush Token (Classic+ #15, Common): a small stone-grey sprinter springs out of a swirling wind circle on a temple running track, a spark on one fist. */
const CONJURE_RUSH_TOKEN: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "blue-200"],
  light: { x: 50, y: 60, r: 28, paint: "white", opacity: 0.35 },
  emblem: { x: 22, y: 30, size: 5, rotate: 0 },
  figure: { x: 50, y: 82, scale: 0.85, pose: "pose-run", look: { ...figureLookFor([]), hair: null, skin: "stone-500", cloth: "slate-400" } },
  layers: {
    sky: [{ part: "sun", x: 80, y: 22, scale: 0.6 }],
    far: [
      { part: "temple", x: 50, y: 64, scale: 0.6 },
      { part: "column", x: 16, y: 66, scale: 0.9 },
      { part: "column", x: 84, y: 66, scale: 0.9 },
    ],
    mid: [
      { part: "street", x: 50, y: 64, paint: "stone-500" },
      { part: "rune-circle", x: 50, y: 83, scale: 1.1, paint: "slate-100" },
      { part: "mist", x: 50, y: 78, scale: 0.8, paint: "white" },
    ],
    particles: [{ part: "sparks", x: 62, y: 50, scale: 1, paint: "white" }],
  },
};

/** Conjure Rush Token+ (Classic+ #16, Rare): a sprinter in new winged sandals bursts from a whirl of wind at a hillside shrine's starting line, a silver coin spinning above. */
const CONJURE_RUSH_TOKEN_PLUS: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "sky-300"],
  light: { x: 50, y: 50, r: 30, paint: "white", opacity: 0.35 },
  emblem: { x: 23, y: 59, size: 4, rotate: 0 },
  figure: { x: 48, y: 84, scale: 0.95, pose: "pose-run", look: { ...figureLookFor([]), hair: "hair-short", item: "item-coin", skin: "red-300", cloth: "sky-400" } },
  layers: {
    sky: [
      { part: "sun", x: 16, y: 22, scale: 0.7 },
      { part: "cloud", x: 72, y: 24, scale: 0.8, count: 2, spread: [10, 3] },
    ],
    far: [
      { part: "hills", x: 50, y: 60, paint: "green-700" },
      { part: "temple", x: 82, y: 62, scale: 0.45 },
    ],
    mid: [
      { part: "field", x: 50, y: 70, paint: "green-700" },
      { part: "banner", x: 20, y: 80, scale: 1.2 },
      { part: "fence-post", x: 76, y: 82 },
    ],
    subject: [{ part: "tornado", x: 36, y: 88, scale: 0.9, paint: "white" }],
    near: [
      { part: "bird", x: 40, y: 84, scale: 0.4, paint: "white" },
      { part: "bird", x: 58, y: 82, scale: 0.4, paint: "white" },
      { part: "grass-tuft", x: 70, y: 98, count: 2, spread: [12, 2] },
    ],
    light: [{ part: "halo", x: 62, y: 42, scale: 1.2, paint: "white" }],
    particles: [
      { part: "petals", x: 40, y: 60, scale: 1.2, paint: "white" },
      { part: "sparks", x: 62, y: 40, scale: 0.9, paint: "white" },
    ],
  },
};

/** R3390000: Classic+ #1 to #26 with #12's and #19's tokens, by id. */
export const CLASSIC_PLUS_RECIPES_1: Readonly<Record<string, SceneRecipe>> = {
  "classicplus-001": DOOM_SHROOM,
  "classicplus-002": GROOM_SHROOM,
  "classicplus-003": SECOND_AMENDMENT_SNAKE,
  "classicplus-004": JUHAN_BIGGEST_BAT,
  "classicplus-005": GUY_ATT,
  "classicplus-006": WRONG_HOUSE_ATTACKER,
  "classicplus-007": THE_HOUSE,
  "classicplus-008": WITHERING_STORM,
  "classicplus-009": SILENCE,
  "classicplus-010": NEW_WRAPS,
  "classicplus-011": ANIME_ARMOR,
  "classicplus-012": THE_MOTHER_PANCAKE,
  "classicplus-012-1": DEVOUR,
  "classicplus-012-2": DEATH_BOIL,
  "classicplus-012-3": FLUFFY_GRIP,
  "classicplus-012-4": POWDER_SPRAY,
  "classicplus-012-5": ANTI_WAFFLE_SHELL,
  "classicplus-012-6": FROZEN_WASTES,
  "classicplus-012-7": LEGION_OF_THE_HUNGRY,
  "classicplus-012-8": FROSTSPATULA,
  "classicplus-013": MOMMY_BARKER,
  "classicplus-014": FOREVER_AND,
  "classicplus-015": CONJURE_RUSH_TOKEN,
  "classicplus-016": CONJURE_RUSH_TOKEN_PLUS,
};
