// R3393000: the Meditative+ set's scene recipes (ART.md, "Scene recipes and kits"), one per entry, its tokens
// included, each drawn from its brief in issue #3393's table in the Meditative family's look (ART.md's art bible):
// ink-wash peaks in mist behind, a high and distant camera, and one red seal in the near layer. A Unit is a
// figure-kit character. Each is placed in the 100-unit box as sceneRecipes.ts's header says, and reaches its
// rarity's floor (richness.ts `ART_DETAIL_MIN`) with its placements' counts.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";
import { MEDITATIVE_PLUS_RECIPES_2 } from "./sceneRecipesMeditativePlus2.ts";

/** Bob, the broad soldier of Bob's plans, in a quilted field jacket (the patched tunic quilted in army green). */
const BOB: FigureLook = { ...figureLookFor(["Human"]), outfit: "outfit-patched", item: null, cloth: "green-700" };
/** A mage of the mountain academies, in a plain long robe. */
const MAGE: FigureLook = { ...figureLookFor(["Human"]), hair: "hair-long", outfit: "outfit-robe", item: null };

/** Bob's Plan (Meditative+ #1, Mythic): over an ink-wash range at dawn, Bob slaps a scroll on a cliff-edge war table and strokes one red-ink arrow up the tallest peak to its summit flag, three ghostly plans (a base camp, a star chart unrolling into a galaxy, a towel and dumbbell) lifting into the sky; cinnabar light, total confidence. */
const BOBS_PLAN: SceneRecipe = {
  family: "meditative",
  sky: ["red-900", "orange-400"],
  light: { x: 50, y: 30, r: 36, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 58, y: 73, size: 6, rotate: 0 },
  figure: { x: 36, y: 85, scale: 0.6, pose: "pose-strike", look: BOB },
  layers: {
    sky: [
      { part: "cloud", x: 30, y: 40, scale: 1.1, count: 3, spread: [24, 4] },
      { part: "stars", x: 50, y: 12, scale: 1.2 },
      { part: "camp-tent", x: 18, y: 32, scale: 1.1, fade: 0.5 },
      { part: "star-map", x: 50, y: 24, scale: 0.8, fade: 0.55 },
      { part: "nebula", x: 64, y: 20, scale: 0.7, fade: 0.6 },
      { part: "towels", x: 82, y: 32, scale: 1.1, fade: 0.5 },
      { part: "weights", x: 82, y: 25, scale: 0.9, fade: 0.5 },
    ],
    far: [
      { part: "peaks", x: 50, y: 70 },
      { part: "summit", x: 50, y: 64, scale: 1.08 },
      { part: "flag", x: 50, y: 30, scale: 0.55 },
    ],
    mid: [
      { part: "mist", x: 50, y: 68, scale: 2.2, count: 2, spread: [20, 3] },
      { part: "ink-pine", x: 12, y: 74, scale: 1.1 },
      { part: "ink-pine", x: 88, y: 72, scale: 0.9, flip: -1 },
    ],
    subject: [
      { part: "table", x: 58, y: 83, scale: 2.4 },
      { part: "scroll", x: 58, y: 76, scale: 1.7 },
      { part: "brush-stroke", x: 54, y: 73, scale: 1.4 },
    ],
    near: [
      { part: "cliff", x: 50, y: 90, paint: "slate-800" },
      { part: "plum-branch", x: 0, y: 94, scale: 1.2 },
      { part: "grass-tuft", x: 76, y: 96, count: 2, spread: [6, 1], paint: "slate-700" },
      { part: "seal", x: 90, y: 77, scale: 1.1 },
    ],
    light: [
      { part: "god-rays", x: 50, y: 30, scale: 1.4, paint: "orange-400" },
      { part: "rim-light", x: 36, y: 62, scale: 0.9, paint: "red-300" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.4, count: 3, spread: [24, 16] },
      { part: "embers", x: 56, y: 60, scale: 1.2, count: 2, spread: [10, 8] },
    ],
  },
};

/** Base Camp (Meditative+ #1-1, Token): glowing orange tents in a snowy saddle below the summit, a climber feeding two coins into a lonely vending machine; thin blue dusk air. */
const BASE_CAMP: SceneRecipe = {
  family: "meditative",
  sky: ["blue-900", "blue-600"],
  light: { x: 40, y: 70, r: 30, paint: "orange-400", opacity: 0.3 },
  emblem: { x: 62, y: 70, size: 5, rotate: 0 },
  extras: [{ layer: "subject", x: 54, y: 82, scale: 0.4, pose: "pose-stand", look: { ...figureLookFor(["Human"]), hair: "hood", outfit: "outfit-parka", item: "item-coin", cloth: "red-600" } }],
  layers: {
    sky: [{ part: "stars", x: 50, y: 16, scale: 1.2 }],
    far: [
      { part: "peaks", x: 50, y: 62 },
      { part: "summit", x: 50, y: 56, scale: 0.8 },
    ],
    mid: [{ part: "hills", x: 50, y: 70, paint: "slate-100" }],
    subject: [
      { part: "camp-tent", x: 34, y: 78, scale: 1.6 },
      { part: "camp-tent", x: 44, y: 72, scale: 1 },
      { part: "vending-machine", x: 64, y: 82, scale: 1.5 },
    ],
    near: [{ part: "seal", x: 12, y: 77 }],
    light: [{ part: "halo", x: 36, y: 74, scale: 1.4, paint: "orange-400" }],
    particles: [{ part: "snow", x: 50, y: 50, scale: 1.4 }],
  },
};

/** Spa Day (Meditative+ #1-2, Token): a steaming mountain hot spring, a bather in a mud mask and cucumber slices, something ticking softly under the folded towels beside her; jade steam. */
const SPA_DAY: SceneRecipe = {
  family: "meditative",
  sky: ["slate-700", "green-200"],
  light: { x: 50, y: 60, r: 30, paint: "jade-400", opacity: 0.3 },
  emblem: { x: 44, y: 62, size: 5, rotate: 0 },
  figure: { x: 44, y: 84, scale: 0.6, pose: "pose-sit", look: { ...figureLookFor(["Human"]), hair: "kerchief", outfit: "outfit-wraps", item: null, skin: "stone-500", cloth: "white" } },
  layers: {
    far: [{ part: "peaks", x: 50, y: 62 }],
    mid: [
      { part: "mossy-boulder", x: 18, y: 80, scale: 1.3 },
      { part: "mossy-boulder", x: 84, y: 78, scale: 1.1, flip: -1 },
    ],
    near: [
      { part: "water", x: 50, y: 74, paint: "jade-400", fade: 0.85 },
      { part: "towels", x: 72, y: 80, scale: 1.5 },
      { part: "seal", x: 90, y: 77 },
    ],
    light: [{ part: "steam-curl", x: 44, y: 72, scale: 1.4, paint: "jade-400", count: 2, spread: [14, 2] }],
    particles: [{ part: "bubbles", x: 46, y: 78, scale: 1.2 }],
  },
};

/** Garage Sale (Meditative+ #1-3, Token): Bob's trestle table in a cliffside village lane, each odd item topped with one penny, most tumbling into a bin; grey drizzle, comic resignation. */
const GARAGE_SALE: SceneRecipe = {
  family: "meditative",
  sky: ["slate-700", "slate-400"],
  light: { x: 50, y: 40, r: 30, paint: "slate-100", opacity: 0.25 },
  emblem: { x: 46, y: 60, size: 5, rotate: 0 },
  extras: [{ layer: "subject", x: 26, y: 84, scale: 0.5, pose: "pose-stand", look: BOB }],
  layers: {
    far: [{ part: "peaks", x: 50, y: 58 }],
    mid: [
      { part: "house", x: 18, y: 72, scale: 1 },
      { part: "house", x: 84, y: 70, scale: 0.8, flip: -1 },
    ],
    subject: [
      { part: "table", x: 48, y: 82, scale: 2.6 },
      { part: "crate", x: 42, y: 73, scale: 1.4 },
      { part: "coins", x: 42, y: 66, scale: 0.5 },
      { part: "gear", x: 53, y: 72, scale: 0.9 },
      { part: "coins", x: 52, y: 64, scale: 0.5 },
      { part: "barrel", x: 72, y: 84, scale: 1.4 },
      { part: "flask", x: 70, y: 66, scale: 1 },
      { part: "scroll", x: 76, y: 72, scale: 1 },
    ],
    near: [{ part: "seal", x: 90, y: 77 }],
    particles: [{ part: "rain", x: 50, y: 50, scale: 1.4, count: 2, spread: [20, 10] }],
  },
};

/** Leg Day (Meditative+ #1-4, Token): a soldier mid-lunge on a thousand-step stone stair up a misty peak, legs enormous, arms ordinary, his squad swelling behind him; brisk morning light. */
const LEG_DAY: SceneRecipe = {
  family: "meditative",
  sky: ["blue-600", "sky-300"],
  light: { x: 70, y: 24, r: 30, paint: "white", opacity: 0.35 },
  emblem: { x: 50, y: 62, size: 5, rotate: 0 },
  figure: { x: 50, y: 86, scale: 0.7, pose: "pose-run", look: { ...BOB, outfit: "outfit-muscle", hair: "sweatband" } },
  extras: [
    { layer: "mid", x: 36, y: 72, scale: 0.36, pose: "pose-run", look: { ...BOB, outfit: "outfit-muscle" } },
    { layer: "mid", x: 64, y: 70, scale: 0.32, pose: "pose-run", look: { ...BOB, outfit: "outfit-muscle", hair: "flap-cap" } },
  ],
  layers: {
    far: [
      { part: "peaks", x: 50, y: 58 },
      { part: "summit", x: 50, y: 50, scale: 0.8 },
    ],
    mid: [{ part: "stairs", x: 50, y: 64, paint: "slate-700" }],
    near: [{ part: "seal", x: 10, y: 77 }],
    particles: [{ part: "dust", x: 50, y: 88, scale: 1.2 }],
  },
};

/** Star Chart (Meditative+ #1-5, Token): a hand-drawn star map unrolled on a peak-top observatory, a route of nodes across it, the universe's edge circled twice in red pen; violet twilight. */
const STAR_CHART: SceneRecipe = {
  family: "meditative",
  sky: ["violet-700", "violet-300"],
  light: { x: 50, y: 70, r: 30, paint: "violet-300", opacity: 0.3 },
  emblem: { x: 50, y: 66, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 18, scale: 1.3, count: 2, spread: [20, 4] },
      { part: "planet", x: 80, y: 20, scale: 0.8 },
    ],
    far: [{ part: "peaks", x: 50, y: 68 }],
    mid: [
      { part: "dome", x: 82, y: 70, scale: 1 },
      { part: "rock-floor", x: 50, y: 78, paint: "slate-700" },
    ],
    subject: [
      { part: "star-map", x: 52, y: 76, scale: 1.7 },
      { part: "astrolabe", x: 24, y: 80, scale: 1.3 },
    ],
    near: [{ part: "seal", x: 90, y: 77 }],
    particles: [{ part: "motes", x: 50, y: 60, scale: 1.2 }],
  },
};

/** NA Tech (Meditative+ #2, Epic): in a cluttered mountain-hut workshop a tinkerer's giant stapler punches a cracked, wilting card and a blazing champion card into one humming bundle, hologram blueprints flickering, snow peaks through a round window; cold blue lamplight. */
const NA_TECH: SceneRecipe = {
  family: "meditative",
  sky: ["blue-900", "blue-200"],
  light: { x: 50, y: 66, r: 32, paint: "cyan-400", opacity: 0.3 },
  emblem: { x: 50, y: 64, size: 6, rotate: 0 },
  extras: [{ layer: "subject", x: 28, y: 88, scale: 0.55, pose: "pose-raise", look: { ...figureLookFor(["Human"]), hair: "goggles", outfit: "outfit-apron", item: null, cloth: "wood-600" } }],
  layers: {
    far: [
      { part: "peaks", x: 50, y: 56 },
      { part: "summit", x: 74, y: 52, scale: 0.6 },
    ],
    mid: [
      { part: "moon-wall", x: 74, y: 74, paint: "slate-700" },
      { part: "bookshelf", x: 12, y: 74, scale: 1.6 },
      { part: "pipes", x: 34, y: 50, scale: 1.1 },
      { part: "code-block", x: 50, y: 40, scale: 1.1, fade: 0.55 },
    ],
    subject: [
      { part: "stapler", x: 52, y: 82, scale: 1.3 },
      { part: "card", x: 44, y: 72, scale: 1.4 },
      { part: "card", x: 58, y: 72, scale: 1.4 },
      { part: "flames", x: 60, y: 70, scale: 0.9 },
      { part: "lightning", x: 51, y: 66, scale: 0.5, paint: "cyan-400" },
    ],
    near: [
      { part: "crate", x: 10, y: 98, scale: 2 },
      { part: "gear", x: 24, y: 96, scale: 1.3 },
      { part: "seal", x: 90, y: 77 },
    ],
    light: [{ part: "halo", x: 52, y: 70, scale: 1.4, paint: "cyan-400" }],
    particles: [{ part: "sparks", x: 52, y: 68, scale: 1.3, count: 2, spread: [8, 4] }],
  },
};

/** Wondrous Wand (Meditative+ #3, Epic): on a pine-clad crag above a misty gorge, a bearded hermit waves a willow wand twice and cards spiral up out of the mist into his hand on teal sparks, ink-wash peaks fading in layers behind; hopeful morning light. */
const WONDROUS_WAND: SceneRecipe = {
  family: "meditative",
  sky: ["sky-400", "slate-100"],
  light: { x: 56, y: 44, r: 32, paint: "cyan-400", opacity: 0.3 },
  emblem: { x: 40, y: 66, size: 6, rotate: 0 },
  figure: { x: 40, y: 86, scale: 0.66, pose: "pose-raise", look: { ...figureLookFor([]), hair: "beard", outfit: "outfit-robe", item: "item-wand", cloth: "slate-700" } },
  layers: {
    sky: [{ part: "cloud", x: 24, y: 24, scale: 1, count: 2, spread: [16, 4] }],
    far: [{ part: "peaks", x: 50, y: 64 }],
    mid: [
      { part: "mist", x: 60, y: 74, scale: 2.4, count: 2, spread: [16, 3] },
      { part: "ink-pine", x: 84, y: 72, scale: 1, flip: -1 },
    ],
    subject: [
      { part: "card", x: 62, y: 66, scale: 1, count: 4, spread: [8, 14] },
      { part: "light-ribbon", x: 56, y: 56, scale: 0.4, paint: "cyan-400" },
    ],
    near: [
      { part: "cliff", x: 50, y: 88, paint: "slate-800" },
      { part: "ink-pine", x: 12, y: 96, scale: 1.6 },
      { part: "seal", x: 90, y: 77 },
    ],
    light: [{ part: "halo", x: 54, y: 50, scale: 1.2, paint: "cyan-400" }],
    particles: [
      { part: "sparks", x: 58, y: 56, scale: 1.3, count: 2, spread: [8, 10], paint: "cyan-400" },
      { part: "motes", x: 50, y: 40, scale: 1.2 },
    ],
  },
};

/** Jlockeed's Supply Drop (Meditative+ #4, Epic): a battered military crate drifts down by parachute onto a high plateau, late as ever, three smoke puffs below it, a waiting soldier in the foreground, its lid bursting with rifles and bandages; parting storm clouds, grey-blue light. */
const SUPPLY_DROP: SceneRecipe = {
  family: "meditative",
  sky: ["slate-700", "blue-200"],
  light: { x: 62, y: 24, r: 34, paint: "blue-200", opacity: 0.4 },
  emblem: { x: 52, y: 54, size: 6, rotate: 0 },
  extras: [{ layer: "near", x: 20, y: 100, scale: 0.9, pose: "pose-stand", look: figureLookFor(["Jlockeed"]) }],
  layers: {
    sky: [
      { part: "cloud", x: 22, y: 22, scale: 1.4, count: 3, spread: [18, 5], paint: "slate-700" },
      { part: "god-rays", x: 62, y: 20, scale: 1.2, paint: "blue-200" },
    ],
    far: [{ part: "peaks", x: 50, y: 66 }],
    mid: [
      { part: "hills", x: 50, y: 74, paint: "slate-700" },
      { part: "smoke", x: 50, y: 78, scale: 1.2, count: 3, spread: [12, 1], paint: "slate-400" },
    ],
    subject: [
      { part: "parachute", x: 52, y: 52, scale: 1.2 },
      { part: "crate", x: 52, y: 60, scale: 2.2 },
      { part: "weapon-rack", x: 52, y: 56, scale: 0.6 },
    ],
    near: [
      { part: "mossy-boulder", x: 84, y: 98, scale: 1.6 },
      { part: "seal", x: 90, y: 77 },
    ],
    particles: [{ part: "dust", x: 50, y: 80, scale: 1.2 }],
  },
};

/** Spirit Lash (Meditative+ #5, Rare): a ghostly ink whip cracks across a moonlit valley of fighters, every shadow on the ground flinching at once; one pale violet streak, cold dread. */
const SPIRIT_LASH: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "blue-900"],
  light: { x: 50, y: 50, r: 34, paint: "violet-300", opacity: 0.3 },
  emblem: { x: 50, y: 48, size: 6, rotate: 0 },
  extras: [
    { layer: "mid", x: 30, y: 78, scale: 0.3, pose: "pose-guard", look: { ...figureLookFor(["Human"]), cloth: "slate-700" } },
    { layer: "mid", x: 50, y: 80, scale: 0.32, pose: "pose-kneel", look: { ...figureLookFor(["Human"]), hair: "hood", item: null, cloth: "red-900" } },
    { layer: "mid", x: 70, y: 78, scale: 0.3, flip: -1, pose: "pose-guard", look: { ...figureLookFor(["Human"]), hair: "helm", item: "item-spear", cloth: "blue-800" } },
  ],
  layers: {
    sky: [
      { part: "moon", x: 78, y: 18, scale: 1 },
      { part: "stars", x: 30, y: 16, scale: 1.2 },
    ],
    far: [{ part: "peaks", x: 50, y: 64 }],
    mid: [{ part: "hills", x: 50, y: 74, paint: "slate-800" }],
    subject: [
      { part: "ink-whip", x: 50, y: 50, scale: 1.3 },
      { part: "wisp", x: 34, y: 60, scale: 1.2, count: 2, spread: [20, 4] },
    ],
    near: [{ part: "seal", x: 90, y: 77 }],
    light: [{ part: "halo", x: 50, y: 52, scale: 1.3, paint: "violet-300" }],
  },
};

/** Bloodbloom (Meditative+ #6, Epic): at a snowbound mountain shrine a crimson flower bursts from a drop of a mage's blood on fresh snow, her next spell gathering in her palm in the middle ground; bare plum branches, grey sky, stark red on white. */
const BLOODBLOOM: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 50, y: 74, r: 28, paint: "red-600", opacity: 0.25 },
  emblem: { x: 50, y: 64, size: 6, rotate: 0 },
  extras: [{ layer: "mid", x: 30, y: 76, scale: 0.42, pose: "pose-raise", look: { ...MAGE, cloth: "red-900" } }],
  layers: {
    sky: [{ part: "cloud", x: 30, y: 22, scale: 1.2, count: 2, spread: [20, 4], paint: "slate-400" }],
    far: [{ part: "peaks", x: 50, y: 62 }],
    mid: [
      { part: "hills", x: 50, y: 72, paint: "white" },
      { part: "lacquer-pavilion", x: 72, y: 74, scale: 0.9 },
      { part: "orb", x: 35, y: 52, scale: 0.5 },
    ],
    subject: [{ part: "bloom", x: 50, y: 80, scale: 2.4 }],
    near: [
      { part: "plum-branch", x: 0, y: 92, scale: 1.4 },
      { part: "plum-branch", x: 100, y: 30, scale: 1.1, flip: -1 },
      { part: "seal", x: 90, y: 77 },
    ],
    light: [{ part: "halo", x: 50, y: 76, scale: 1.3, paint: "red-600" }],
    particles: [{ part: "snow", x: 50, y: 50, scale: 1.4, count: 2, spread: [20, 10] }],
  },
};

/** Life Mage (Meditative+ #7, Common): a gentle young mage in pale robes inside a soft shield bubble, mending a sparrow's wing in a meadow below the peaks; calm green dawn. */
const LIFE_MAGE: SceneRecipe = {
  family: "meditative",
  sky: ["green-700", "green-200"],
  light: { x: 54, y: 62, r: 30, paint: "green-200", opacity: 0.35 },
  emblem: { x: 46, y: 69, size: 6, rotate: 0 },
  figure: { x: 46, y: 83, scale: 0.6, pose: "pose-kneel", look: { ...MAGE, cloth: "slate-100" } },
  layers: {
    far: [{ part: "peaks", x: 50, y: 60 }],
    mid: [
      { part: "hills", x: 50, y: 72, paint: "green-500" },
      { part: "flowers", x: 20, y: 80, scale: 1.2, count: 2, spread: [8, 2] },
    ],
    subject: [
      { part: "shield-bubble", x: 50, y: 85, scale: 1.3, paint: "green-200" },
      { part: "bird", x: 57, y: 58, scale: 0.7 },
    ],
    near: [
      { part: "grass-tuft", x: 80, y: 96, count: 2, spread: [6, 1] },
      { part: "seal", x: 12, y: 77 },
    ],
  },
};

/** Arch Mage (Meditative+ #8, Common): a stern grey-haired archmage lecturing at a stone lectern on a cliff-top academy terrace, spellfire crackling from one raised finger; dramatic slate-blue thunder. */
const ARCH_MAGE: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "blue-700"],
  light: { x: 48, y: 46, r: 30, paint: "cyan-400", opacity: 0.3 },
  emblem: { x: 56, y: 70, size: 6, rotate: 0 },
  figure: { x: 56, y: 86, scale: 0.66, flip: -1, pose: "pose-raise", look: { ...MAGE, hair: "beard", cloth: "blue-800" } },
  layers: {
    sky: [{ part: "lightning", x: 80, y: 30, scale: 1.2 }],
    far: [
      { part: "peaks", x: 50, y: 62 },
      { part: "temple", x: 82, y: 70, scale: 0.8 },
    ],
    subject: [
      { part: "lectern", x: 36, y: 86, scale: 2 },
      { part: "sparks", x: 48, y: 47, scale: 1.2, paint: "cyan-400" },
    ],
    near: [{ part: "seal", x: 90, y: 77 }],
  },
};

/** Magic Amplifier (Meditative+ #9, Common): a mountain amphitheatre ringed by tall cracked rune-stones that hum and glow, sound waves rippling across both facing stands at once; violet dusk, booming. */
const MAGIC_AMPLIFIER: SceneRecipe = {
  family: "meditative",
  sky: ["violet-700", "violet-500"],
  light: { x: 50, y: 66, r: 32, paint: "violet-300", opacity: 0.35 },
  emblem: { x: 50, y: 60, size: 6, rotate: 0 },
  layers: {
    far: [{ part: "peaks", x: 50, y: 60 }],
    mid: [
      { part: "stands", x: 18, y: 78, scale: 1.5 },
      { part: "stands", x: 82, y: 78, scale: 1.5, flip: -1 },
    ],
    subject: [
      { part: "standing-stone", x: 40, y: 72, scale: 1.3 },
      { part: "standing-stone", x: 60, y: 72, scale: 1.3, flip: -1 },
      { part: "standing-stone", x: 30, y: 82, scale: 1.8 },
      { part: "standing-stone", x: 70, y: 82, scale: 1.8, flip: -1 },
      { part: "ripples", x: 50, y: 64, scale: 1.4 },
    ],
    near: [{ part: "seal", x: 90, y: 77 }],
    light: [{ part: "halo", x: 50, y: 66, scale: 1.4, paint: "violet-300" }],
  },
};

/** Medina Grifter (Meditative+ #10, Common): a grinning grifter in a striped robe lifts a purse off a passerby in a crowded mountain bazaar of awnings and spice sacks; hazy afternoon. */
const MEDINA_GRIFTER: SceneRecipe = {
  family: "meditative",
  sky: ["brass-600", "red-300"],
  light: { x: 70, y: 30, r: 34, paint: "orange-400", opacity: 0.3 },
  emblem: { x: 42, y: 70, size: 6, rotate: 0 },
  figure: { x: 42, y: 86, scale: 0.64, pose: "pose-stand", look: { ...figureLookFor(["Human"]), hair: "kerchief", outfit: "outfit-robe", item: "item-purse", cloth: "red-600" } },
  extras: [{ layer: "subject", x: 62, y: 86, scale: 0.6, flip: -1, pose: "pose-run", look: { ...figureLookFor(["Human"]), hair: "hat-brim", outfit: "outfit-coat", item: null, cloth: "blue-800" } }],
  layers: {
    far: [{ part: "peaks", x: 50, y: 58 }],
    mid: [
      { part: "awning", x: 18, y: 74, scale: 1.3 },
      { part: "awning", x: 84, y: 72, scale: 1.2, flip: -1 },
    ],
    near: [
      { part: "sacks", x: 16, y: 98, scale: 1.4 },
      { part: "seal", x: 90, y: 77 },
    ],
    particles: [{ part: "dust", x: 50, y: 60, scale: 1.4 }],
  },
};

/** Living Mana Potion (Meditative+ #11, Rare): a tall blue potion flask on stubby legs, a swirl of stars inside, sloshing as it sprints along a narrow cliff path; electric azure glow. */
const LIVING_MANA_POTION: SceneRecipe = {
  family: "meditative",
  sky: ["blue-800", "sky-400"],
  light: { x: 50, y: 60, r: 30, paint: "cyan-400", opacity: 0.4 },
  emblem: { x: 50, y: 70, size: 6, rotate: 0 },
  figure: { x: 50, y: 84, scale: 0.72, pose: "pose-run", look: { ...figureLookFor(["Potion"]), head: "head-eyes", hair: null, outfit: "outfit-flask", item: null, skin: "sky-300", cloth: "blue-600" } },
  layers: {
    far: [{ part: "peaks", x: 50, y: 62 }],
    mid: [{ part: "mist", x: 50, y: 74, scale: 2.4, count: 2, spread: [18, 3] }],
    subject: [{ part: "stars", x: 50, y: 66, scale: 0.4 }],
    near: [
      { part: "cliff", x: 50, y: 88, paint: "slate-800" },
      { part: "seal", x: 90, y: 77 },
    ],
    light: [{ part: "halo", x: 50, y: 64, scale: 1.4, paint: "cyan-400" }],
    particles: [
      { part: "sparks", x: 50, y: 60, scale: 1.2, count: 2, spread: [14, 10], paint: "cyan-400" },
      { part: "bubbles", x: 40, y: 56, scale: 1.2 },
    ],
  },
};

/** R3393000: the Meditative+ cards' recipes, by id, #12 on from sceneRecipesMeditativePlus2.ts. */
export const MEDITATIVE_PLUS_RECIPES: Readonly<Record<string, SceneRecipe>> = {
  "meditativeplus-001": BOBS_PLAN,
  "meditativeplus-001-1": BASE_CAMP,
  "meditativeplus-001-2": SPA_DAY,
  "meditativeplus-001-3": GARAGE_SALE,
  "meditativeplus-001-4": LEG_DAY,
  "meditativeplus-001-5": STAR_CHART,
  "meditativeplus-002": NA_TECH,
  "meditativeplus-003": WONDROUS_WAND,
  "meditativeplus-004": SUPPLY_DROP,
  "meditativeplus-005": SPIRIT_LASH,
  "meditativeplus-006": BLOODBLOOM,
  "meditativeplus-007": LIFE_MAGE,
  "meditativeplus-008": ARCH_MAGE,
  "meditativeplus-009": MAGIC_AMPLIFIER,
  "meditativeplus-010": MEDINA_GRIFTER,
  "meditativeplus-011": LIVING_MANA_POTION,
  ...MEDITATIVE_PLUS_RECIPES_2,
};
