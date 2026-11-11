// R3393000: the second half of the Meditative+ set's scene recipes (#12 to #26 and their tokens), kept apart from
// sceneRecipesMeditativePlus.ts, which spreads them into its table, as the Classic+ pass split its own. Each is
// drawn from its brief in issue #3393's table in the Meditative family's look (ART.md's art bible): ink-wash peaks
// in mist behind, a high and distant camera, and one red seal in the near layer. A Unit is a figure-kit character.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** A living potion: a glass bottle body with a face on it and stubby legs, no hair and nothing held. */
const POTION: FigureLook = { ...figureLookFor(["Potion"]), head: "head-eyes", hair: null, outfit: "outfit-flask", item: null, skin: "red-300" };
/** A traveller of the mountain roads in a hood and a plain tunic. */
const TRAVELLER: FigureLook = { ...figureLookFor(["Human"]), hair: "hood", item: null, cloth: "stone-500" };

/** Living Armor Potion (Meditative+ #12, Rare): a squat steel-plated potion bottle with little arms hugs a tired traveller by the fire in a mountain hut at night; iron-grey and ember, snug. */
const LIVING_ARMOR_POTION: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "wood-900"],
  light: { x: 24, y: 72, r: 36, paint: "ember-500", opacity: 0.4 },
  emblem: { x: 44, y: 70, size: 6, rotate: 0 },
  figure: { x: 44, y: 88, scale: 0.8, pose: "pose-spread", look: { ...POTION, cloth: "slate-400" } },
  extras: [{ layer: "subject", x: 63, y: 88, scale: 0.62, flip: -1, pose: "pose-sit", look: TRAVELLER }],
  layers: {
    sky: [
      { part: "moon", x: 56, y: 32, scale: 0.5 },
      { part: "stars", x: 48, y: 36, scale: 0.5 },
    ],
    far: [{ part: "peaks", x: 50, y: 52, scale: 0.8 }],
    mid: [
      { part: "moon-wall", x: 50, y: 78, scale: 1.3, paint: "slate-700" },
      { part: "hearth", x: 16, y: 86, scale: 1.1 },
    ],
    near: [
      { part: "crate", x: 88, y: 98, scale: 1.2 },
      { part: "seal", x: 90, y: 77, scale: 1 },
    ],
    light: [{ part: "rim-light", x: 44, y: 66, scale: 1, flip: -1, paint: "ember-500" }],
    particles: [{ part: "embers", x: 18, y: 68, scale: 1.2, props: true }],
  },
};

/** Living Health Potion (Meditative+ #13, Rare): a round red potion pot with a ladle arm serves warm soup to weary adventurers at a mountain inn's long table; rosy steam. */
const LIVING_HEALTH_POTION: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 60, r: 38, paint: "red-300", opacity: 0.35 },
  emblem: { x: 36, y: 70, size: 6, rotate: 0 },
  figure: { x: 36, y: 88, scale: 0.78, pose: "pose-stand", look: { ...POTION, cloth: "red-600", item: "item-ladle" } },
  extras: [
    { layer: "mid", x: 64, y: 84, scale: 0.52, flip: -1, pose: "pose-sit", look: { ...TRAVELLER, hair: "hat-brim", cloth: "green-700" } },
    { layer: "mid", x: 80, y: 84, scale: 0.5, flip: -1, pose: "pose-sit", look: { ...TRAVELLER, cloth: "blue-700" } },
  ],
  layers: {
    sky: [{ part: "stars", x: 50, y: 30, scale: 0.4 }],
    far: [{ part: "peaks", x: 50, y: 50, scale: 0.8 }],
    mid: [{ part: "moon-wall", x: 50, y: 78, scale: 1.3, paint: "slate-700" }],
    subject: [
      { part: "table", x: 72, y: 90, scale: 1.3 },
      { part: "cauldron", x: 66, y: 79, scale: 0.4 },
      { part: "steam-curl", x: 66, y: 74, scale: 0.7, paint: "red-300" },
      { part: "bread", x: 80, y: 79, scale: 0.7 },
    ],
    near: [
      { part: "seal", x: 10, y: 77, scale: 1 },
      { part: "steam-curl", x: 52, y: 66, scale: 0.6, paint: "red-300" },
    ],
    light: [{ part: "halo", x: 46, y: 68, scale: 1.2, paint: "red-300" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1, props: true }],
  },
};

/** Potion Pouch (Meditative+ #14, Rare): a leather pouch bursts open on a rocky trail, bottles of every colour tumbling out clinking, one popping in a puff of smoke; bright noon. */
const POTION_POUCH: SceneRecipe = {
  family: "meditative",
  sky: ["sky-400", "blue-200"],
  light: { x: 78, y: 18, r: 32, paint: "white", opacity: 0.45 },
  emblem: { x: 40, y: 62, size: 6, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 80, y: 18, scale: 0.6 },
      { part: "cloud", x: 24, y: 24, scale: 0.7, paint: "white" },
    ],
    far: [{ part: "peaks", x: 50, y: 58, scale: 1 }],
    mid: [
      { part: "rock-floor", x: 50, y: 70, paint: "stone-500" },
      { part: "ink-pine", x: 86, y: 70, scale: 0.8, flip: -1 },
      { part: "ink-pine", x: 14, y: 72, scale: 0.6 },
    ],
    subject: [
      { part: "pouch", x: 40, y: 74, scale: 2 },
      { part: "flask", x: 54, y: 78, scale: 0.8, paint: "red-500" },
      { part: "flask", x: 62, y: 72, scale: 0.7, paint: "violet-500" },
      { part: "flask", x: 48, y: 82, scale: 0.6, paint: "cyan-400" },
      { part: "flask", x: 68, y: 80, scale: 0.6, paint: "orange-400" },
      { part: "smoke", x: 64, y: 62, scale: 0.8, paint: "violet-300" },
    ],
    near: [
      { part: "grass-tuft", x: 18, y: 94, scale: 1.2, count: 2, spread: [8, 2], paint: "green-700" },
      { part: "seal", x: 88, y: 77, scale: 1 },
    ],
    light: [{ part: "god-rays", x: 80, y: 14, scale: 0.8, paint: "white" }],
    particles: [{ part: "sparks", x: 60, y: 66, scale: 1.2, props: true }],
  },
};

/** Book of Potions (Meditative+ #15, Epic): in a hermit alchemist's tower above the clouds, a giant recipe book flips open on a stone table and its glowing page lifts one shining potion into the air; jars on shelves, peaks through the arch, pearly white-teal light. */
const BOOK_OF_POTIONS: SceneRecipe = {
  family: "meditative",
  sky: ["slate-700", "cyan-400"],
  light: { x: 50, y: 48, r: 36, paint: "cyan-400", opacity: 0.35 },
  emblem: { x: 50, y: 70, size: 6, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 50, y: 52, scale: 1.1, count: 2, spread: [14, 2], paint: "white" }],
    far: [
      { part: "peaks", x: 50, y: 60, scale: 0.9 },
      { part: "mist", x: 50, y: 62, scale: 0.8, paint: "white" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 78, paint: "stone-500" },
      { part: "column", x: 28, y: 80, scale: 1.4 },
      { part: "column", x: 72, y: 80, scale: 1.4 },
      { part: "arch", x: 50, y: 80, scale: 2 },
      { part: "bookshelf", x: 14, y: 84, scale: 1.1 },
      { part: "flask", x: 10, y: 59, scale: 0.4, paint: "green-400" },
      { part: "flask", x: 17, y: 59, scale: 0.35, paint: "violet-500" },
      { part: "bookshelf", x: 86, y: 84, scale: 1.1, flip: -1 },
      { part: "flask", x: 86, y: 59, scale: 0.4, paint: "red-500" },
    ],
    subject: [
      { part: "table", x: 50, y: 90, scale: 1.3, paint: "stone-500" },
      { part: "tome", x: 50, y: 80, scale: 1.8 },
      { part: "flask", x: 50, y: 58, scale: 1.3, paint: "cyan-400" },
    ],
    near: [
      { part: "candle", x: 12, y: 96, scale: 1.2 },
      { part: "scroll", x: 80, y: 98, scale: 1 },
      { part: "seal", x: 92, y: 77, scale: 1 },
    ],
    light: [
      { part: "light-shaft", x: 50, y: 56, scale: 0.8, paint: "cyan-400" },
      { part: "halo", x: 50, y: 52, scale: 1.2, paint: "white" },
    ],
    particles: [{ part: "sparks", x: 50, y: 52, scale: 1.4, props: true }],
  },
};

/** Jazakus (Meditative+ #16, Epic): Jazakus, a wild-haired brewer with goggles on his brow and a stained apron, sloshes an unlabelled bottle into a bubbling cauldron in his workshop behind a waterfall; mismatched flasks on the shelves, mist pouring past the entrance, mischievous green firelight. */
const JAZAKUS: SceneRecipe = {
  family: "meditative",
  sky: ["slate-800", "jade-400"],
  light: { x: 60, y: 76, r: 34, paint: "green-400", opacity: 0.4 },
  emblem: { x: 40, y: 66, size: 6, rotate: 0 },
  figure: { x: 40, y: 88, scale: 0.8, pose: "pose-stand", look: { ...figureLookFor(["Human", "Catalyst"]), hair: "goggles", outfit: "outfit-apron", item: "item-flask", cloth: "stone-500" } },
  layers: {
    sky: [{ part: "cloud", x: 50, y: 30, scale: 0.8, paint: "white" }],
    far: [
      { part: "peaks", x: 50, y: 56, scale: 0.9 },
      { part: "cascade", x: 52, y: 16, scale: 1.4 },
    ],
    mid: [
      { part: "cave", x: 52, y: 52, scale: 1.1 },
      { part: "bookshelf", x: 12, y: 84, scale: 1 },
      { part: "flask", x: 8, y: 61, scale: 0.35, paint: "violet-500" },
      { part: "flask", x: 14, y: 61, scale: 0.45, paint: "red-500" },
      { part: "flask", x: 19, y: 61, scale: 0.3, paint: "cyan-400" },
      { part: "mist", x: 54, y: 66, scale: 0.8, paint: "white" },
    ],
    subject: [
      { part: "flames", x: 60, y: 90, scale: 0.9, paint: "green-400" },
      { part: "cauldron", x: 60, y: 88, scale: 0.9 },
      { part: "bubbles", x: 60, y: 70, scale: 0.8 },
    ],
    near: [
      { part: "flask", x: 86, y: 98, scale: 0.9, paint: "lime-400" },
      { part: "fern", x: 22, y: 98, scale: 1.2, paint: "green-900" },
      { part: "seal", x: 92, y: 77, scale: 1 },
    ],
    light: [{ part: "halo", x: 60, y: 78, scale: 1.3, paint: "green-400" }],
    particles: [
      { part: "spores", x: 58, y: 64, scale: 1, paint: "lime-400" },
      { part: "motes", x: 50, y: 50, scale: 1, props: true },
    ],
  },
};

/** Jazakus Prime (Meditative+ #16, token): Jazakus grown huge and grizzled, stirring a cauldron the size of a hut with a boat oar on a steaming crater rim; ominous purple fumes. */
const JAZAKUS_PRIME: SceneRecipe = {
  family: "meditative",
  sky: ["violet-700", "slate-900"],
  light: { x: 64, y: 70, r: 36, paint: "purple-400", opacity: 0.4 },
  emblem: { x: 44, y: 54, size: 7, rotate: 0 },
  figure: { x: 44, y: 84, scale: 1, pose: "pose-guard", look: { ...figureLookFor(["Human", "Prime"]), hair: "beard", outfit: "outfit-apron", item: "item-oar", cloth: "slate-700" } },
  layers: {
    far: [{ part: "peaks", x: 50, y: 58, scale: 1, paint: "slate-900" }],
    mid: [
      { part: "rock-floor", x: 50, y: 74, paint: "slate-700" },
      { part: "smoke", x: 76, y: 50, scale: 1.6, paint: "violet-500" },
    ],
    near: [
      { part: "cauldron", x: 52, y: 100, scale: 1.9 },
      { part: "steam-curl", x: 56, y: 76, scale: 1, paint: "violet-300" },
      { part: "seal", x: 10, y: 77, scale: 1 },
    ],
    particles: [{ part: "embers", x: 70, y: 66, scale: 1.2 }],
  },
};

/** Dogpile (Meditative+ #17, Rare): spell after spell, bolts, fireballs and frost shards, piling onto one cornered foe on a narrow mountain ridge; chaotic orange and blue flashes. */
const DOGPILE: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "blue-800"],
  light: { x: 54, y: 56, r: 34, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 54, y: 72, size: 5, rotate: 0 },
  extras: [{ layer: "subject", x: 54, y: 80, scale: 0.5, pose: "pose-guard", look: { ...figureLookFor([]), item: "item-shield", cloth: "slate-700" } }],
  layers: {
    sky: [{ part: "stars", x: 30, y: 24, scale: 0.8 }],
    far: [{ part: "peaks", x: 50, y: 66, scale: 1.1 }],
    mid: [
      { part: "cliff", x: 56, y: 80, scale: 0.9, paint: "slate-800" },
      { part: "mist", x: 50, y: 86, scale: 0.9, paint: "white" },
    ],
    subject: [
      { part: "lightning", x: 56, y: 22, scale: 0.9, paint: "blue-200" },
      { part: "fireball", x: 32, y: 50, scale: 1.1 },
      { part: "fireball", x: 76, y: 44, scale: 1, flip: -1 },
      { part: "crystal", x: 40, y: 72, scale: 0.8, paint: "blue-200" },
      { part: "crystal", x: 68, y: 70, scale: 0.7, paint: "sky-300" },
      { part: "shockwave", x: 54, y: 66, scale: 0.6, paint: "orange-400" },
    ],
    near: [{ part: "seal", x: 90, y: 77, scale: 1 }],
    light: [{ part: "halo", x: 54, y: 62, scale: 1.2, paint: "orange-400" }],
    particles: [
      { part: "sparks", x: 54, y: 60, scale: 1.6, props: true },
      { part: "embers", x: 44, y: 54, scale: 1 },
    ],
  },
};

/** Cognitive Offloading (Meditative+ #18, Epic): in a lamplit mountain study a scholar dozes over his desk while a lantern-eyed wooden automaton gently takes the brush from his hand and plays his cards; scroll racks behind, moonlit peaks through the window, eerie calm. */
const COGNITIVE_OFFLOADING: SceneRecipe = {
  family: "meditative",
  sky: ["blue-900", "slate-700"],
  light: { x: 30, y: 52, r: 30, paint: "orange-400", opacity: 0.3 },
  emblem: { x: 62, y: 64, size: 6, rotate: 0 },
  figure: { x: 62, y: 88, scale: 0.72, flip: -1, pose: "pose-stand", look: { head: "head-automaton", hair: null, outfit: "outfit-armor", item: "item-brush", skin: "wood-600", cloth: "wood-700" } },
  extras: [{ layer: "subject", x: 34, y: 90, scale: 0.6, pose: "pose-sit", look: { ...figureLookFor(["Human"]), hair: "tonsure", outfit: "outfit-robe", item: null, cloth: "blue-800" } }],
  layers: {
    sky: [{ part: "moon", x: 70, y: 32, scale: 0.6 }],
    far: [{ part: "peaks", x: 60, y: 52, scale: 0.7, paint: "slate-800" }],
    mid: [
      { part: "moon-wall", x: 68, y: 80, scale: 1.2, paint: "wood-800" },
      { part: "bookshelf", x: 14, y: 84, scale: 1.1 },
      { part: "scroll", x: 14, y: 61, scale: 0.6 },
      { part: "lantern", x: 40, y: 30, scale: 1.6 },
    ],
    subject: [
      { part: "table", x: 46, y: 94, scale: 1.2 },
      { part: "card", x: 42, y: 84, scale: 0.55, count: 3, spread: [6, 0] },
      { part: "candle", x: 54, y: 85, scale: 0.6 },
    ],
    near: [
      { part: "scroll", x: 84, y: 98, scale: 1 },
      { part: "seal", x: 92, y: 77, scale: 1 },
    ],
    light: [
      { part: "light-shaft", x: 68, y: 46, scale: 0.6, paint: "blue-200" },
      { part: "rim-light", x: 62, y: 64, scale: 1, paint: "blue-200" },
    ],
    particles: [{ part: "motes", x: 50, y: 56, scale: 0.9, props: true }],
  },
};

/** Grubby Gloves (Meditative+ #19, Common): mud-caked gloves sneak the top card off a rival's deck across a gambling table in a den carved into a cliff; dim green lantern, sly. */
const GRUBBY_GLOVES: SceneRecipe = {
  family: "meditative",
  sky: ["slate-800", "slate-400"],
  light: { x: 50, y: 40, r: 30, paint: "green-400", opacity: 0.3 },
  emblem: { x: 62, y: 62, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "peaks", x: 50, y: 60, scale: 0.8 }],
    mid: [
      { part: "cave", x: 50, y: 48, scale: 1.1, paint: "wood-900" },
      { part: "lantern", x: 50, y: 18, scale: 1.1 },
    ],
    subject: [
      { part: "table", x: 50, y: 100, scale: 2.6, paint: "green-900" },
      { part: "card", x: 62, y: 80, scale: 1.2, count: 3, spread: [0.8, 0.5] },
      { part: "glove", x: 44, y: 74, scale: 2.2 },
      { part: "coins", x: 28, y: 84, scale: 1 },
    ],
    near: [{ part: "seal", x: 88, y: 77, scale: 1 }],
    light: [{ part: "halo", x: 50, y: 32, scale: 1, paint: "green-400" }],
  },
};

/** KY's Competition (Meditative+ #20, Rare): KY, a sharp-eyed math teacher, calls answers aloud from a mountain pavilion stage as students scribble on slates, an hourglass draining; crisp chalk-white light. */
const KYS_COMPETITION: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "slate-100"],
  light: { x: 50, y: 50, r: 34, paint: "white", opacity: 0.45 },
  emblem: { x: 46, y: 60, size: 5, rotate: 0 },
  figure: { x: 46, y: 74, scale: 0.55, pose: "pose-raise", look: { ...figureLookFor(["KY"]), item: null } },
  extras: [
    { layer: "near", x: 24, y: 98, scale: 0.5, pose: "pose-sit", look: { ...figureLookFor(["Human"]), item: "item-book", cloth: "green-700" } },
    { layer: "near", x: 76, y: 98, scale: 0.5, flip: -1, pose: "pose-sit", look: { ...figureLookFor(["Human"]), hair: "hair-long", item: "item-book", cloth: "red-600" } },
  ],
  layers: {
    sky: [{ part: "cloud", x: 24, y: 22, scale: 0.6, paint: "white" }],
    far: [
      { part: "peaks", x: 50, y: 56, scale: 0.9 },
      { part: "mist", x: 50, y: 60, scale: 0.8, paint: "white" },
    ],
    mid: [{ part: "lacquer-pavilion", x: 50, y: 76, scale: 1.7 }],
    subject: [{ part: "hourglass", x: 62, y: 74, scale: 0.7 }],
    near: [{ part: "seal", x: 92, y: 77, scale: 1 }],
    light: [{ part: "god-rays", x: 50, y: 14, scale: 0.8, paint: "white" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1, paint: "white", props: true }],
  },
};

/** Lil' Kenny (Meditative+ #20, token): Lil' Kenny, a small smug boy in an oversized scholar's cap, perches on a mossy boulder by a stream keeping a pebble score; clear morning. */
const LIL_KENNY: SceneRecipe = {
  family: "meditative",
  sky: ["sky-400", "blue-200"],
  light: { x: 76, y: 22, r: 30, paint: "white", opacity: 0.4 },
  emblem: { x: 46, y: 70, size: 5, rotate: 0 },
  figure: { x: 46, y: 84, scale: 0.62, pose: "pose-sit", look: { ...figureLookFor(["KY", "Token"]), hair: "scholar-cap", outfit: "outfit-robe", item: null, cloth: "blue-800" } },
  layers: {
    sky: [{ part: "sun", x: 80, y: 20, scale: 0.5 }],
    far: [{ part: "peaks", x: 50, y: 56, scale: 0.9 }],
    mid: [
      { part: "water", x: 50, y: 76, paint: "blue-600" },
      { part: "ink-pine", x: 14, y: 76, scale: 0.7 },
    ],
    subject: [{ part: "mossy-boulder", x: 46, y: 92, scale: 2.4 }],
    near: [
      { part: "pebbles", x: 66, y: 92, scale: 0.6 },
      { part: "seal", x: 90, y: 77, scale: 1 },
    ],
  },
};

/** Harmony Spirit (Meditative+ #21, Rare): a translucent jade spirit hums one note on a stone chime above a teahouse courtyard, every fighter near it shrinking or swelling alike; misty calm. */
const HARMONY_SPIRIT: SceneRecipe = {
  family: "meditative",
  sky: ["slate-700", "green-200"],
  light: { x: 50, y: 56, r: 34, paint: "jade-400", opacity: 0.35 },
  emblem: { x: 42, y: 56, size: 6, rotate: 0 },
  figure: { x: 42, y: 76, scale: 0.68, pose: "pose-spread", fade: 0.6, look: { ...figureLookFor(["CN"]), hair: "hair-long", outfit: "outfit-robe", item: null, skin: "jade-400", cloth: "jade-400" } },
  extras: [
    { layer: "mid", x: 22, y: 86, scale: 0.42, pose: "pose-guard", look: { ...figureLookFor(["Human"]), cloth: "red-600" } },
    { layer: "mid", x: 80, y: 86, scale: 0.42, flip: -1, pose: "pose-guard", look: { ...figureLookFor(["Human"]), item: "item-spear", cloth: "blue-800" } },
  ],
  layers: {
    far: [
      { part: "peaks", x: 50, y: 54, scale: 0.9 },
      { part: "lacquer-pavilion", x: 50, y: 66, scale: 1.1, paint: "green-900" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 70, paint: "stone-500" },
      { part: "mist", x: 50, y: 72, scale: 0.9, paint: "white" },
    ],
    subject: [{ part: "chime", x: 60, y: 80, scale: 1.2 }],
    near: [
      { part: "plum-branch", x: 2, y: 24, scale: 0.9 },
      { part: "seal", x: 90, y: 77, scale: 1 },
    ],
    light: [{ part: "ripples", x: 50, y: 66, scale: 1, paint: "jade-400" }],
    particles: [{ part: "motes", x: 50, y: 56, scale: 0.9, paint: "green-200", props: true }],
  },
};

/** Mountaintop Garden (Meditative+ #22, Epic): a terraced garden on a summit above a sea of clouds: stone water channels in the foreground, giant pumpkins and towering beanstalks with boulder-sized seeds, a tiny gardener's hut and far peaks behind; thin crisp air, emerald light. */
const MOUNTAINTOP_GARDEN: SceneRecipe = {
  family: "meditative",
  sky: ["blue-800", "green-200"],
  light: { x: 44, y: 40, r: 36, paint: "green-400", opacity: 0.35 },
  emblem: { x: 58, y: 76, size: 6, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 50, y: 58, scale: 1.3, count: 3, spread: [30, 2], paint: "white" }],
    far: [
      { part: "peaks", x: 50, y: 54, scale: 0.7 },
      { part: "mist", x: 50, y: 60, scale: 1, paint: "white" },
    ],
    mid: [
      { part: "terrace", x: 50, y: 66 },
      { part: "house", x: 82, y: 66, scale: 0.36 },
    ],
    subject: [
      { part: "beanstalk", x: 40, y: 88, scale: 1.3 },
      { part: "pumpkin", x: 60, y: 86, scale: 1.5 },
      { part: "pumpkin", x: 28, y: 88, scale: 0.9 },
    ],
    near: [
      { part: "water", x: 50, y: 94, paint: "sky-400" },
      { part: "fern", x: 10, y: 98, scale: 1.3 },
      { part: "seal", x: 90, y: 77, scale: 1 },
    ],
    light: [{ part: "god-rays", x: 40, y: 12, scale: 0.9, paint: "green-200" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1, paint: "green-200", props: true }],
  },
};

/** Enormous Elderberry (Meditative+ #23, Epic): in a mountain orchard pickers on tall ladders wrestle one elderberry the size of a cart down from a twisted tree, while a brave soul waits below with a spoon; dark purple fruit against misty green slopes, festive. */
const ENORMOUS_ELDERBERRY: SceneRecipe = {
  family: "meditative",
  sky: ["sky-400", "green-200"],
  light: { x: 50, y: 46, r: 34, paint: "violet-300", opacity: 0.3 },
  emblem: { x: 50, y: 48, size: 6, rotate: 0 },
  figure: { x: 50, y: 92, scale: 0.42, pose: "pose-raise", look: { ...figureLookFor(["Human"]), item: "item-spoon", cloth: "red-600" } },
  extras: [
    { layer: "subject", x: 30, y: 72, scale: 0.32, pose: "pose-raise", look: { ...figureLookFor(["Human"]), hair: "kerchief", item: null, cloth: "green-700" } },
    { layer: "subject", x: 70, y: 72, scale: 0.32, flip: -1, pose: "pose-raise", look: { ...figureLookFor(["Human"]), hair: "hat-brim", item: null, cloth: "blue-700" } },
    { layer: "subject", x: 36, y: 56, scale: 0.28, pose: "pose-spread", look: { ...figureLookFor(["Human"]), hair: "hair-long", item: null, cloth: "orange-400" } },
    { layer: "subject", x: 64, y: 56, scale: 0.28, flip: -1, pose: "pose-spread", look: { ...figureLookFor(["Human"]), item: null, cloth: "violet-700" } },
  ],
  layers: {
    sky: [{ part: "cloud", x: 78, y: 22, scale: 0.6, paint: "white" }],
    far: [
      { part: "peaks", x: 50, y: 60, scale: 0.9, paint: "green-900" },
      { part: "mist", x: 50, y: 64, scale: 1, paint: "white" },
    ],
    mid: [
      { part: "hills", x: 50, y: 70, paint: "green-700" },
      { part: "dead-tree", x: 50, y: 86, scale: 2.6 },
    ],
    subject: [
      { part: "wood-ladder", x: 26, y: 92, scale: 1.1 },
      { part: "wood-ladder", x: 74, y: 92, scale: 1.1, flip: -1 },
      { part: "berry", x: 50, y: 58, scale: 1.8 },
    ],
    near: [
      { part: "basket", x: 14, y: 98, scale: 1.2 },
      { part: "flowers", x: 80, y: 98, scale: 1.4, count: 2, spread: [6, 1] },
      { part: "seal", x: 92, y: 77, scale: 1 },
    ],
    light: [{ part: "rim-light", x: 50, y: 50, scale: 1.2, paint: "violet-300" }],
    particles: [{ part: "petals", x: 50, y: 40, scale: 1.2, paint: "red-300", props: true }],
  },
};

/** Bob, Wisconsin's Strongest Soldier (Meditative+ #24, Legendary): Bob, broad-shouldered in a fur-lined army parka and flap cap, bench-presses a full-size snowplow over his head on a blizzard-swept mountain pass, boots cracking the ice; enemies frozen like statues behind him; through the snow a lakeside fish-fry shack glows warm. Hard white wind light, heroic. */
const BOB_WISCONSINS_STRONGEST: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "white"],
  light: { x: 50, y: 40, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 68, size: 7, rotate: 0 },
  figure: { x: 50, y: 90, scale: 0.74, pose: "pose-lift", look: { ...figureLookFor([]), hair: "flap-cap", outfit: "outfit-parka", item: null, cloth: "green-700" } },
  layers: {
    sky: [{ part: "cloud", x: 30, y: 20, scale: 1, count: 2, spread: [20, 3], paint: "slate-100" }],
    far: [
      { part: "peaks", x: 50, y: 58, scale: 0.9 },
      { part: "water", x: 50, y: 64, paint: "slate-400" },
      { part: "house", x: 84, y: 66, scale: 0.4 },
      { part: "ink-pine", x: 26, y: 66, scale: 0.5, count: 2, spread: [8, 1], paint: "slate-700" },
      { part: "mist", x: 50, y: 66, scale: 1, paint: "white" },
    ],
    mid: [
      { part: "ice-floor", x: 50, y: 74 },
      { part: "statue", x: 18, y: 84, scale: 1, paint: "blue-200" },
      { part: "statue", x: 82, y: 84, scale: 0.9, flip: -1, paint: "blue-200" },
    ],
    subject: [{ part: "snowplow", x: 50, y: 44, scale: 1.6 }],
    near: [
      { part: "shockwave", x: 50, y: 90, scale: 0.5, paint: "blue-200" },
      { part: "seal", x: 10, y: 77, scale: 1 },
    ],
    light: [
      { part: "halo", x: 84, y: 60, scale: 0.6, paint: "orange-400" },
      { part: "rim-light", x: 50, y: 60, scale: 1.4, paint: "white" },
    ],
    particles: [{ part: "snow", x: 50, y: 50, scale: 1.4, count: 2, spread: [10, 10] }],
  },
};

/** KY's Intuition (Meditative+ #25, Rare): KY squints at three glowing blocks of indented line-work hovering over a cliff-side library balcony, one block lit brighter than the rest; cool indigo dusk. */
const KYS_INTUITION: SceneRecipe = {
  family: "meditative",
  sky: ["blue-900", "violet-700"],
  light: { x: 62, y: 48, r: 30, paint: "cyan-400", opacity: 0.3 },
  emblem: { x: 36, y: 66, size: 5, rotate: 0 },
  figure: { x: 36, y: 88, scale: 0.66, pose: "pose-stand", look: { ...figureLookFor(["KY"]), item: null } },
  layers: {
    sky: [{ part: "stars", x: 60, y: 22, scale: 1 }],
    far: [
      { part: "peaks", x: 50, y: 62, scale: 1, paint: "slate-900" },
      { part: "mist", x: 50, y: 66, scale: 0.9, paint: "violet-300" },
    ],
    mid: [
      { part: "bookshelf", x: 12, y: 84, scale: 1.2 },
      { part: "fence", x: 50, y: 88, scale: 1, paint: "stone-500" },
    ],
    subject: [
      { part: "code-block", x: 60, y: 34, scale: 0.9 },
      { part: "code-block", x: 64, y: 50, scale: 1.2 },
      { part: "code-block", x: 58, y: 66, scale: 0.9 },
    ],
    near: [
      { part: "candle", x: 86, y: 96, scale: 1.2 },
      { part: "seal", x: 92, y: 77, scale: 1 },
    ],
    light: [{ part: "halo", x: 64, y: 46, scale: 1.1, paint: "cyan-400" }],
    particles: [{ part: "motes", x: 60, y: 50, scale: 0.8, paint: "blue-200", props: true }],
  },
};

/** Scratch Pad (Meditative+ #26, Mythic): a vast blank paper sky over an ink-wash range: a lone coder kneels on a floating sheet before a single blinking bar of cyan light, building a card from nothing line by line, its frame rising like a scaffold of ink; stray strokes scatter as ink birds while unfinished peaks fade below. Stark black on white, nerve-wracking quiet. */
const SCRATCH_PAD: SceneRecipe = {
  family: "meditative",
  sky: ["white", "slate-100"],
  light: { x: 64, y: 54, r: 26, paint: "cyan-400", opacity: 0.25 },
  emblem: { x: 58, y: 62, size: 6, rotate: 0 },
  figure: { x: 38, y: 76, scale: 0.52, pose: "pose-kneel", look: { ...figureLookFor(["Human"]), outfit: "outfit-jacket", item: "item-brush", cloth: "slate-900" } },
  layers: {
    sky: [
      { part: "bird", x: 22, y: 24, scale: 0.5, count: 4, spread: [12, 6], paint: "slate-900" },
      { part: "bird", x: 80, y: 30, scale: 0.4, count: 3, spread: [8, 4], paint: "slate-900" },
    ],
    far: [
      { part: "peaks", x: 50, y: 90, scale: 1, fade: 0.55 },
      { part: "summit", x: 16, y: 84, scale: 0.8, fade: 0.5 },
      { part: "summit", x: 88, y: 88, scale: 0.6, flip: -1, fade: 0.4 },
      { part: "mist", x: 50, y: 82, scale: 1.1, paint: "white" },
    ],
    mid: [
      { part: "ink-pine", x: 86, y: 82, scale: 0.6, fade: 0.6 },
      { part: "mist", x: 40, y: 88, scale: 1, paint: "white" },
    ],
    subject: [
      { part: "paper-sheet", x: 50, y: 78, scale: 1.4 },
      { part: "card-frame", x: 58, y: 76, scale: 1.5 },
      { part: "cursor", x: 70, y: 66, scale: 1 },
    ],
    near: [
      { part: "plum-branch", x: 100, y: 16, scale: 0.8, flip: -1 },
      { part: "ink-pine", x: 6, y: 100, scale: 0.9 },
      { part: "seal", x: 90, y: 77, scale: 1 },
    ],
    light: [{ part: "halo", x: 68, y: 58, scale: 0.9, paint: "cyan-400" }],
    particles: [
      { part: "bird", x: 62, y: 40, scale: 0.3, count: 5, spread: [12, 7], paint: "slate-900" },
      { part: "sparks", x: 66, y: 58, scale: 0.8 },
    ],
  },
};

/** R3393000: Meditative+ #12 to #26's recipes, by id. */
export const MEDITATIVE_PLUS_RECIPES_2: Readonly<Record<string, SceneRecipe>> = {
  "meditativeplus-012": LIVING_ARMOR_POTION,
  "meditativeplus-013": LIVING_HEALTH_POTION,
  "meditativeplus-014": POTION_POUCH,
  "meditativeplus-015": BOOK_OF_POTIONS,
  "meditativeplus-016": JAZAKUS,
  "meditativeplus-016-1": JAZAKUS_PRIME,
  "meditativeplus-017": DOGPILE,
  "meditativeplus-018": COGNITIVE_OFFLOADING,
  "meditativeplus-019": GRUBBY_GLOVES,
  "meditativeplus-020": KYS_COMPETITION,
  "meditativeplus-020-1": LIL_KENNY,
  "meditativeplus-021": HARMONY_SPIRIT,
  "meditativeplus-022": MOUNTAINTOP_GARDEN,
  "meditativeplus-023": ENORMOUS_ELDERBERRY,
  "meditativeplus-024": BOB_WISCONSINS_STRONGEST,
  "meditativeplus-025": KYS_INTUITION,
  "meditativeplus-026": SCRATCH_PAD,
};
