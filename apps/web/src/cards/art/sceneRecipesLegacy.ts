// R3393001: the Legacy cards' scene recipes (Training Grounds #41 to #60), each a scene of its own from its brief
// in issue #3393's table, in the Training Grounds family's look, placing its own yard touch (traininggrounds.ts
// `LEGACY_TOUCHES`) once as the part `LEGACY_TOUCH_PARTS` names. Its face wears its Core card's frozen emblem
// (svg.ts `legacySceneSvg`) and never draws its Core card's scene. A part id is written out in each recipe, as
// importing a value from traininggrounds.ts here would close a module cycle; `setScenes.test.ts` checks it.
// Presentation only (CLAUDE.md rule 7): a recipe is read by the card's id alone.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";
import type { SceneryPartId } from "./sceneKit.ts";
import type { LegacyTouch } from "./traininggrounds.ts";

/** R3393001: the kit part each Legacy card's yard touch is drawn as, the parts the kit held already standing in for a few. */
export const LEGACY_TOUCH_PARTS: Readonly<Record<LegacyTouch, SceneryPartId>> = {
  "rope-cordon": "rope-cordon",
  "crate-stack": "crate",
  "sword-in-sand": "practice-sword",
  "paw-prints": "paw-prints",
  "weapon-rack": "weapon-rack",
  "arrow-target": "target-board",
  "range-flags": "range-flags",
  "straw-heap": "straw-heap",
  hammock: "hammock",
  lantern: "lantern",
  "scratching-post": "scratching-post",
  "carrot-bucket": "bucket",
  "chalk-tally": "slate-board",
  "lifting-stone": "lifting-stone",
  "tattered-banner": "banner",
  shovel: "shovel",
  "water-barrel": "barrel",
  "pell-post": "pell-post",
  "fig-basket": "basket",
  scarecrow: "scarecrow",
};

/** A recruit of the yard in its plain tunic. */
const RECRUIT: FigureLook = { ...figureLookFor(["Human"]), item: null, cloth: "blue-800" };
/** A cat-folk recruit, in the yard's green. */
const FELINOR_RECRUIT: FigureLook = { ...figureLookFor(["Felinor"]), item: null, cloth: "green-700" };

/** Legacy: Speciesist (Training Grounds #41, Common): the Speciesist, sneering in a velvet coat, guards a rope cordon at the viewing stand, waving a fox-eared recruit away; cold grey light. */
const LEGACY_SPECIESIST: SceneRecipe = {
  family: "training-grounds",
  sky: ["slate-400", "slate-100"],
  light: { x: 78, y: 20, r: 32, paint: "white", opacity: 0.3 },
  emblem: { x: 47, y: 57, size: 6, rotate: 0 },
  figure: { x: 46, y: 86, scale: 0.85, pose: "pose-raise", look: { ...figureLookFor(["Human"]), outfit: "outfit-coat", item: null, cloth: "violet-700" } },
  extras: [{ layer: "mid", x: 80, y: 78, scale: 0.5, flip: -1, pose: "pose-stand", look: { ...FELINOR_RECRUIT, cloth: "blue-800" } }],
  layers: {
    sky: [{ part: "cloud", x: 26, y: 22, scale: 0.8, count: 2, spread: [16, 4], paint: "slate-100" }],
    far: [
      { part: "palisade", x: 50, y: 62, scale: 2 },
      { part: "walkway", x: 24, y: 66, scale: 1.1 },
    ],
    mid: [
      { part: "yard-sand", x: 50, y: 66 },
      { part: "plank-bench", x: 20, y: 74, scale: 1 },
    ],
    near: [
      { part: "rope-cordon", x: 50, y: 90, scale: 1.45 },
      { part: "grass-tuft", x: 88, y: 92, count: 2, spread: [5, 1], paint: "wood-600" },
    ],
    light: [{ part: "rim-light", x: 46, y: 60, scale: 1.1, paint: "slate-100" }],
  },
};

/** Legacy: Stockpile (Training Grounds #42, Common): a recruit naps on a stack of supply crates in the storehouse loft, two cards and a snack in hand; lazy late-morning light. */
const LEGACY_STOCKPILE: SceneRecipe = {
  family: "training-grounds",
  sky: ["wood-800", "wood-700"],
  light: { x: 66, y: 32, r: 34, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 42, y: 82, size: 6, rotate: 0 },
  figure: { x: 46, y: 83, scale: 0.72, pose: "pose-sit", look: { ...RECRUIT, hair: "hair-long", cloth: "green-700" } },
  layers: {
    far: [{ part: "room-wall", x: 50, y: 70, paint: "wood-600" }],
    mid: [
      { part: "tile-floor", x: 50, y: 70, paint: "brass-600" },
      { part: "cart", x: 82, y: 82, scale: 1.1, flip: -1 },
      { part: "wheat", x: 14, y: 82, scale: 1.2 },
    ],
    subject: [
      { part: "crate", x: 42, y: 91, scale: 2.6 },
      { part: "chest", x: 63, y: 91, scale: 1.1 },
    ],
    near: [
      { part: "card", x: 55, y: 71, scale: 0.55, count: 2, spread: [1.5, 1], props: true },
      { part: "bread", x: 36, y: 74, scale: 0.8, props: true },
    ],
    light: [
      { part: "light-shaft", x: 64, y: 14, paint: "white" },
      { part: "halo", x: 50, y: 60, scale: 1.6, paint: "orange-400" },
    ],
    particles: [{ part: "motes", x: 58, y: 50, scale: 1.1, paint: "white" }],
  },
};

/** Legacy: Mr. Vanilla (Training Grounds #43, Common): Mr. Vanilla, a plain man in a cream tunic, stands by a practice sword stuck in the bare sand square, utterly ordinary, content; flat daylight. */
const LEGACY_MR_VANILLA: SceneRecipe = {
  family: "training-grounds",
  sky: ["sky-300", "blue-200"],
  light: { x: 78, y: 20, r: 30, paint: "white", opacity: 0.25 },
  emblem: { x: 45, y: 57, size: 6, rotate: 0 },
  figure: { x: 44, y: 86, scale: 0.85, pose: "pose-stand", look: { ...figureLookFor(["Human"]), item: null, cloth: "slate-100" } },
  layers: {
    sky: [
      { part: "sun", x: 82, y: 18, scale: 0.7 },
      { part: "cloud", x: 30, y: 24, scale: 0.7, paint: "white" },
    ],
    far: [{ part: "palisade", x: 50, y: 62, scale: 2 }],
    mid: [{ part: "yard-sand", x: 50, y: 64 }],
    subject: [{ part: "practice-sword", x: 63, y: 86, scale: 1.4 }],
    near: [
      { part: "pebbles", x: 26, y: 92 },
      { part: "grass-tuft", x: 86, y: 92, paint: "wood-600" },
    ],
  },
};

/** Legacy: Duplicating Felinors (Training Grounds #44, Rare): three cat-folk recruits multiplying on the bunkhouse steps, paw prints doubling behind them, the third arriving unasked; playful afternoon. */
const LEGACY_DUPLICATING_FELINORS: SceneRecipe = {
  family: "training-grounds",
  sky: ["sky-300", "red-300"],
  light: { x: 78, y: 22, r: 32, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 41, y: 66, size: 5, rotate: 0 },
  figure: { x: 40, y: 84, scale: 0.6, pose: "pose-raise", look: FELINOR_RECRUIT },
  extras: [
    { layer: "subject", x: 55, y: 82, scale: 0.56, flip: -1, pose: "pose-stand", look: FELINOR_RECRUIT },
    { layer: "subject", x: 68, y: 85, scale: 0.55, flip: -1, pose: "pose-run", look: FELINOR_RECRUIT, fade: 0.45 },
  ],
  layers: {
    sky: [
      { part: "sun", x: 84, y: 16, scale: 0.6 },
      { part: "cloud", x: 24, y: 20, scale: 0.7, paint: "white" },
    ],
    far: [{ part: "palisade", x: 50, y: 62, scale: 2.1, paint: "wood-700" }],
    mid: [
      { part: "yard-sand", x: 50, y: 68 },
      { part: "barracks", x: 50, y: 71, scale: 1.4 },
      { part: "paw-prints", x: 48, y: 77, scale: 1.1 },
    ],
    near: [
      { part: "flowers", x: 14, y: 90, scale: 1.2 },
      { part: "grass-tuft", x: 86, y: 90, count: 2, spread: [4, 1], paint: "green-700" },
    ],
    light: [{ part: "halo", x: 68, y: 66, scale: 1.2, paint: "white" }],
    particles: [{ part: "sparks", x: 68, y: 64, scale: 1, count: 2, spread: [5, 6], paint: "white" }],
  },
};

/** Legacy: Jlockeed's Weapons (Training Grounds #45, Common): a Jlockeed vendor's tent pitched in the yard, its racks overflowing with polished rifles and blades; harsh steel-blue. */
const LEGACY_JLOCKEEDS_WEAPONS: SceneRecipe = {
  family: "training-grounds",
  sky: ["slate-700", "blue-800"],
  light: { x: 76, y: 20, r: 30, paint: "sky-300", opacity: 0.35 },
  emblem: { x: 50, y: 74, size: 6, rotate: 0 },
  extras: [{ layer: "mid", x: 34, y: 78, scale: 0.55, pose: "pose-guard", look: figureLookFor(["Jlockeed"]) }],
  layers: {
    sky: [{ part: "sun", x: 84, y: 16, scale: 0.6 }],
    far: [{ part: "palisade", x: 50, y: 62, scale: 2, paint: "slate-700" }],
    mid: [
      { part: "yard-sand", x: 50, y: 64, paint: "stone-500" },
      { part: "awning", x: 52, y: 80, scale: 1.7, paint: "blue-800" },
    ],
    subject: [
      { part: "weapon-rack", x: 50, y: 88, scale: 1.3 },
      { part: "axe", x: 67, y: 88, scale: 0.55 },
    ],
    near: [{ part: "crate", x: 20, y: 94, scale: 1.3 }],
    light: [{ part: "rim-light", x: 50, y: 72, scale: 1.2, paint: "sky-300" }],
    particles: [{ part: "sparks", x: 50, y: 66, scale: 1, count: 2, spread: [10, 4], paint: "white" }],
  },
};

/** Legacy: Hit Job (Training Grounds #46, Common): a hooded assassin's arrow thuds into an armoured dummy at the archery butts, clean and quiet, an hourglass tipping beside the bow; cold dusk. */
const LEGACY_HIT_JOB: SceneRecipe = {
  family: "training-grounds",
  sky: ["blue-900", "slate-400"],
  light: { x: 74, y: 30, r: 30, paint: "violet-300", opacity: 0.3 },
  emblem: { x: 53, y: 70, size: 5, rotate: 0 },
  extras: [{ layer: "near", x: 18, y: 98, scale: 0.7, pose: "pose-guard", look: { ...figureLookFor(["Human"]), hair: "hood", outfit: "outfit-coat", item: "item-bow", cloth: "slate-900" } }],
  layers: {
    sky: [{ part: "stars", x: 40, y: 18, scale: 1 }],
    far: [{ part: "palisade", x: 50, y: 62, scale: 2, paint: "wood-800" }],
    mid: [{ part: "yard-sand", x: 50, y: 64, paint: "stone-500" }],
    subject: [
      { part: "target-board", x: 66, y: 82, scale: 1.7 },
      { part: "straw-dummy", x: 52, y: 88, scale: 1.3, paint: "slate-400" },
      { part: "arrow", x: 44, y: 66, scale: 0.6 },
    ],
    near: [{ part: "hourglass", x: 32, y: 94, scale: 0.9 }],
    light: [{ part: "halo", x: 52, y: 66, scale: 1, paint: "violet-300" }],
  },
};

/** Legacy: Midrange Menace (Training Grounds #47, Common): a bulky armoured bruiser planted at the halfway mark of the long sprint field among range flags, bandaging himself whole between bouts; steady overcast noon. */
const LEGACY_MIDRANGE_MENACE: SceneRecipe = {
  family: "training-grounds",
  sky: ["slate-400", "slate-100"],
  light: { x: 76, y: 18, r: 34, paint: "white", opacity: 0.3 },
  emblem: { x: 50, y: 56, size: 6, rotate: 0 },
  figure: {
    x: 48,
    y: 86,
    scale: 0.9,
    pose: "pose-stand",
    look: { ...figureLookFor(["Legacy"]), hair: "helm", outfit: "outfit-plate", item: "item-scroll", cloth: "slate-400" },
  },
  layers: {
    sky: [{ part: "cloud", x: 40, y: 22, scale: 0.9, count: 3, spread: [26, 4], paint: "slate-100" }],
    far: [{ part: "fence", x: 50, y: 60, scale: 1.2, paint: "wood-700" }],
    mid: [
      { part: "lanes", x: 50, y: 60 },
      { part: "range-flags", x: 67, y: 73, scale: 1.4 },
    ],
    near: [{ part: "grass-tuft", x: 14, y: 90, count: 2, spread: [5, 1], paint: "wood-600" }],
    particles: [{ part: "dust", x: 50, y: 86, scale: 1.2, paint: "stone-500" }],
  },
};

/** Legacy: Jlockeed Shredder-10 (Training Grounds #48, Common): a clanking shredder on treads, its blades spinning, chews straw dummies beside the hay barn, sparks and straw flying; red evening. */
const LEGACY_JLOCKEED_SHREDDER: SceneRecipe = {
  family: "training-grounds",
  sky: ["red-900", "orange-400"],
  light: { x: 80, y: 30, r: 34, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 46, y: 58, size: 6, rotate: 0 },
  figure: {
    x: 46,
    y: 83,
    scale: 1.1,
    pose: "pose-treads",
    look: { ...figureLookFor(["Jlockeed"]), head: "head-automaton", hair: null, outfit: "outfit-machine", item: null, skin: "slate-400", cloth: "slate-700" },
  },
  layers: {
    sky: [{ part: "sun", x: 82, y: 30, scale: 0.6, paint: "red-300" }],
    far: [
      { part: "house", x: 18, y: 64, scale: 1.3, paint: "red-900" },
      { part: "palisade", x: 76, y: 64, scale: 1.2, paint: "wood-800" },
    ],
    mid: [{ part: "yard-sand", x: 50, y: 66, paint: "wood-600" }],
    subject: [{ part: "straw-dummy", x: 70, y: 82, scale: 1, flip: -1 }],
    near: [{ part: "straw-heap", x: 18, y: 96, scale: 1.4 }],
    particles: [
      { part: "sparks", x: 56, y: 64, scale: 1.1 },
      { part: "splinters", x: 64, y: 70, scale: 1, paint: "brass-600" },
    ],
  },
};

/** Legacy: Reoccurring Dream (Training Grounds #49, Rare): a recruit asleep in a hammock strung between two posts in the orchard corner, dreaming of a card turning bright above him; soft lilac dusk. */
const LEGACY_REOCCURRING_DREAM: SceneRecipe = {
  family: "training-grounds",
  sky: ["violet-700", "violet-300"],
  light: { x: 54, y: 34, r: 30, paint: "violet-300", opacity: 0.45 },
  emblem: { x: 56, y: 40, size: 6, rotate: 0 },
  extras: [{ layer: "subject", x: 47, y: 80, scale: 0.42, pose: "pose-sit", look: { ...RECRUIT, hair: "hood" } }],
  layers: {
    sky: [
      { part: "stars", x: 30, y: 18, scale: 1.1 },
      { part: "moon", x: 82, y: 18, scale: 0.6 },
    ],
    far: [
      { part: "fence", x: 50, y: 64, scale: 1.2, paint: "wood-800" },
      { part: "fruit-tree", x: 12, y: 70, scale: 1 },
      { part: "fruit-tree", x: 88, y: 68, scale: 0.9, flip: -1 },
    ],
    mid: [{ part: "field", x: 50, y: 70, paint: "green-900" }],
    subject: [
      { part: "hammock", x: 50, y: 86, scale: 1.3 },
      { part: "thought-bubble", x: 57, y: 42, scale: 2.2, paint: "slate-100" },
      { part: "card", x: 57, y: 47, scale: 0.9 },
    ],
    near: [{ part: "grass-tuft", x: 20, y: 90, count: 3, spread: [10, 1], paint: "green-700" }],
    light: [{ part: "halo", x: 57, y: 42, scale: 1.4, paint: "white" }],
    particles: [{ part: "sparks", x: 57, y: 40, scale: 0.9, count: 2, spread: [7, 4], paint: "white" }],
  },
};

/** Legacy: Glowy Jelly Bean (Training Grounds #50, Rare): a recruit holds a glowing jelly bean up in the watchtower's lantern room, the card in her other hand lighting up pearl-white; fresh lime-green glow. */
const LEGACY_GLOWY_JELLY_BEAN: SceneRecipe = {
  family: "training-grounds",
  sky: ["slate-800", "green-900"],
  light: { x: 60, y: 46, r: 32, paint: "lime-400", opacity: 0.4 },
  emblem: { x: 46, y: 62, size: 5, rotate: 0 },
  figure: { x: 46, y: 88, scale: 0.78, pose: "pose-spread", look: { ...RECRUIT, hair: "hair-long", cloth: "green-700" } },
  layers: {
    far: [{ part: "room-wall", x: 50, y: 76, paint: "wood-700" }],
    mid: [
      { part: "tile-floor", x: 50, y: 76, paint: "wood-600" },
      { part: "table", x: 80, y: 86, scale: 0.8 },
    ],
    subject: [{ part: "lantern", x: 74, y: 22, scale: 1.6 }],
    near: [
      { part: "pale-bean", x: 60, y: 50, scale: 1.3, props: true },
      { part: "card", x: 32, y: 54, scale: 0.7, props: true },
    ],
    light: [
      { part: "halo", x: 60, y: 48, scale: 1.4, paint: "lime-200" },
      { part: "halo", x: 32, y: 50, scale: 1.1, paint: "white" },
    ],
    particles: [{ part: "sparks", x: 60, y: 46, scale: 0.9, count: 2, spread: [5, 4], paint: "lime-200" }],
  },
};

/** Legacy: Big Felinor (Training Grounds #51, Rare): a massive cuddly cat-folk sprawls by a shredded scratching post in the common room, a kitten on his shoulder, glaring at everyone else; plum evening. */
const LEGACY_BIG_FELINOR: SceneRecipe = {
  family: "training-grounds",
  sky: ["violet-700", "red-900"],
  light: { x: 40, y: 44, r: 34, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 46, y: 64, size: 6, rotate: 0 },
  figure: { x: 44, y: 86, scale: 1, pose: "pose-sit", look: { ...figureLookFor(["Felinor"]), item: null, skin: "stone-500", cloth: "violet-500" } },
  layers: {
    far: [{ part: "room-wall", x: 50, y: 72, paint: "violet-700" }],
    mid: [
      { part: "tile-floor", x: 50, y: 72, paint: "wood-700" },
      { part: "hearth", x: 18, y: 76, scale: 1 },
    ],
    subject: [{ part: "scratching-post", x: 68, y: 86, scale: 1.3 }],
    near: [
      { part: "cat", x: 39, y: 54, scale: 0.9, paint: "orange-400" },
      { part: "pebbles", x: 82, y: 92, paint: "wood-600" },
    ],
    light: [{ part: "rim-light", x: 44, y: 62, scale: 1.4, paint: "orange-400" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1, paint: "red-300" }],
  },
};

/** Legacy: Snom Bunny Mind Control (Training Grounds #52, Rare): a fluffy white bunny with swirling hypnotic eyes leads a dazed ogre by the ear past a carrot bucket in the vegetable patch; pastel haze. */
const LEGACY_SNOM_BUNNY_MIND_CONTROL: SceneRecipe = {
  family: "training-grounds",
  sky: ["violet-300", "blue-200"],
  light: { x: 36, y: 60, r: 30, paint: "violet-300", opacity: 0.4 },
  emblem: { x: 60, y: 60, size: 6, rotate: 0 },
  figure: {
    x: 60,
    y: 84,
    scale: 0.8,
    flip: -1,
    pose: "pose-stand",
    look: { ...figureLookFor(["Legacy"]), head: "head-ogre", hair: null, outfit: "outfit-fur", item: null, skin: "green-500", cloth: "wood-700" },
  },
  layers: {
    sky: [{ part: "cloud", x: 30, y: 24, scale: 0.8, count: 2, spread: [30, 4], paint: "white" }],
    far: [
      { part: "hills", x: 50, y: 58, paint: "green-200" },
      { part: "fence", x: 50, y: 62, scale: 1, paint: "wood-600" },
    ],
    mid: [
      { part: "furrows", x: 50, y: 64, paint: "wood-600" },
      { part: "sprout", x: 16, y: 72, count: 3, spread: [10, 3] },
      { part: "sprout", x: 84, y: 74, count: 2, spread: [8, 3] },
    ],
    subject: [
      { part: "hypno-bunny", x: 38, y: 84, scale: 2.5 },
      { part: "void-swirl", x: 42, y: 66, scale: 0.45, paint: "violet-500" },
      { part: "void-swirl", x: 60, y: 38, scale: 0.35, paint: "violet-300" },
    ],
    near: [
      { part: "bucket", x: 80, y: 96, scale: 1.5 },
      { part: "mist", x: 50, y: 92, scale: 1, paint: "white" },
    ],
    particles: [{ part: "motes", x: 46, y: 60, scale: 0.9, paint: "violet-300" }],
  },
};

/** Legacy: Prejudiced Postdoc (Training Grounds #53, Rare): a bespectacled postdoc in a lab coat at the medical tent lifts a plain, featureless copy of a human recruit; chalk stick figures, sterile white. */
const LEGACY_PREJUDICED_POSTDOC: SceneRecipe = {
  family: "training-grounds",
  sky: ["slate-400", "white"],
  light: { x: 54, y: 40, r: 34, paint: "white", opacity: 0.45 },
  emblem: { x: 42, y: 62, size: 6, rotate: 0 },
  figure: { x: 42, y: 84, scale: 0.85, pose: "pose-raise", look: { ...figureLookFor(["Human"]), hair: "glasses", outfit: "outfit-labcoat", item: null, cloth: "sky-300" } },
  extras: [{ layer: "subject", x: 52, y: 41, scale: 0.36, pose: "pose-stand", fade: 0.6, look: { ...figureLookFor(["Human"]), hair: null, item: null, skin: "slate-100", cloth: "slate-100" } }],
  layers: {
    far: [
      { part: "camp-tent", x: 50, y: 66, scale: 2, paint: "slate-100" },
      { part: "palisade", x: 88, y: 64, scale: 1, paint: "wood-700" },
    ],
    mid: [
      { part: "yard-sand", x: 50, y: 68, paint: "stone-500" },
      { part: "cot", x: 80, y: 78, scale: 0.9 },
    ],
    subject: [{ part: "flask", x: 28, y: 86, scale: 0.9 }],
    near: [{ part: "slate-board", x: 76, y: 98, scale: 1.6 }],
    light: [{ part: "rim-light", x: 42, y: 62, scale: 1.2, paint: "white" }],
    particles: [{ part: "motes", x: 56, y: 46, scale: 0.8, paint: "white" }],
  },
};

/** Legacy: The Rock (Training Grounds #54, Common): a man of living granite, cracked and mossy, lifts a round training stone overhead in the strongman corner, cooking nothing at all; flat grey noon. */
const LEGACY_THE_ROCK: SceneRecipe = {
  family: "training-grounds",
  sky: ["slate-400", "slate-100"],
  light: { x: 78, y: 20, r: 32, paint: "white", opacity: 0.3 },
  emblem: { x: 48, y: 64, size: 6, rotate: 0 },
  figure: {
    x: 48,
    y: 85,
    scale: 0.9,
    pose: "pose-lift",
    look: { ...figureLookFor(["Human"]), head: "head-golem", hair: null, outfit: "outfit-stone", item: null, skin: "stone-500", cloth: "stone-500" },
  },
  layers: {
    sky: [{ part: "cloud", x: 30, y: 22, scale: 0.9, count: 2, spread: [24, 3], paint: "slate-100" }],
    far: [{ part: "palisade", x: 50, y: 62, scale: 2, paint: "wood-700" }],
    mid: [
      { part: "yard-sand", x: 50, y: 66 },
      { part: "weights", x: 18, y: 78, scale: 1 },
      { part: "plank-bench", x: 82, y: 78, scale: 0.9 },
    ],
    near: [
      { part: "lifting-stone", x: 48, y: 31, scale: 1.7 },
      { part: "grass-tuft", x: 86, y: 94, count: 2, spread: [5, 1], paint: "green-700" },
    ],
  },
};

/** Legacy: Reminisce (Training Grounds #55, Rare): an old soldier beneath a tattered plain banner in the memorial corner, faded ghost cards of past battles rising from a grave mound; rose dusk. */
const LEGACY_REMINISCE: SceneRecipe = {
  family: "training-grounds",
  sky: ["violet-700", "red-300"],
  light: { x: 64, y: 40, r: 32, paint: "red-300", opacity: 0.4 },
  emblem: { x: 62, y: 54, size: 5, rotate: 0 },
  figure: { x: 40, y: 86, scale: 0.78, pose: "pose-stand", look: { ...figureLookFor(["Human"]), outfit: "outfit-tabard", item: "item-spear", cloth: "slate-700" } },
  layers: {
    sky: [{ part: "cloud", x: 30, y: 22, scale: 0.8, count: 2, spread: [14, 4], paint: "red-300" }],
    far: [
      { part: "palisade", x: 50, y: 64, scale: 2, paint: "wood-800" },
      { part: "tombstone", x: 82, y: 66, scale: 0.6 },
    ],
    mid: [
      { part: "yard-sand", x: 50, y: 66, paint: "stone-500" },
      { part: "grave-mound", x: 62, y: 80, scale: 0.8 },
    ],
    subject: [
      { part: "banner", x: 28, y: 86, scale: 1.7, paint: "slate-100" },
      { part: "card", x: 62, y: 66, scale: 0.7, count: 3, spread: [8, 10], fade: 0.5 },
      { part: "wisp", x: 62, y: 72, scale: 0.8, count: 2, spread: [6, 4] },
    ],
    near: [
      { part: "flowers", x: 76, y: 90, scale: 1.1 },
      { part: "standing-stone", x: 90, y: 94, scale: 0.6 },
    ],
    light: [{ part: "halo", x: 62, y: 58, scale: 1.4, paint: "white" }],
    particles: [{ part: "motes", x: 62, y: 56, scale: 1, paint: "violet-300" }],
  },
};

/** Legacy: Corpse Eater (Training Grounds #56, Epic): a hunched ghoul leans on a shovel at the yard's burial pit, swelling as the spirits of fallen units drift into his open mouth; churned earth before him, the barracks beyond, a sickly green moon. */
const LEGACY_CORPSE_EATER: SceneRecipe = {
  family: "training-grounds",
  sky: ["slate-900", "green-900"],
  light: { x: 72, y: 22, r: 30, paint: "green-400", opacity: 0.4 },
  emblem: { x: 45, y: 58, size: 6, rotate: 0 },
  figure: {
    x: 44,
    y: 84,
    scale: 0.9,
    pose: "pose-guard",
    look: { ...figureLookFor(["Legacy"]), head: "head-goblin", hair: null, outfit: "outfit-patched", skin: "lime-400", cloth: "slate-700" },
  },
  layers: {
    sky: [
      { part: "moon", x: 74, y: 20, scale: 1, paint: "lime-200" },
      { part: "stars", x: 30, y: 18, scale: 1 },
    ],
    far: [
      { part: "barracks", x: 78, y: 62, scale: 0.9, paint: "wood-800" },
      { part: "palisade", x: 50, y: 63, scale: 2, paint: "wood-800" },
      { part: "dead-tree", x: 12, y: 62, scale: 1 },
    ],
    mid: [
      { part: "yard-sand", x: 50, y: 64, paint: "stone-500" },
      { part: "grave-mound", x: 24, y: 74, scale: 0.7 },
      { part: "grave-mound", x: 74, y: 76, scale: 0.8 },
      { part: "tombstone", x: 86, y: 74, scale: 0.6 },
    ],
    subject: [
      { part: "shovel", x: 53, y: 85, scale: 1.7 },
      { part: "ghost", x: 28, y: 40, scale: 0.7, fade: 0.6 },
      { part: "ghost", x: 64, y: 34, scale: 0.6, flip: -1, fade: 0.6 },
      { part: "wisp", x: 44, y: 34, scale: 0.8, count: 3, spread: [12, 5] },
    ],
    near: [
      { part: "rock-floor", x: 50, y: 90, paint: "wood-800" },
      { part: "bones", x: 22, y: 94, scale: 1 },
      { part: "pebbles", x: 74, y: 95 },
    ],
    light: [
      { part: "halo", x: 45, y: 42, scale: 1.4, paint: "green-400" },
      { part: "rim-light", x: 44, y: 62, scale: 1.3, paint: "green-200" },
    ],
    particles: [{ part: "spores", x: 44, y: 40, scale: 1.2, paint: "lime-200" }],
  },
};

/** Legacy: Recycling Initiative (Training Grounds #57, Epic): in the laundry yard a recruit tosses played cards into a water barrel that swirls like a whirlpool and spits fresh copies back into his hands; drying racks of cards, the fence behind, clean blue-green morning. */
const LEGACY_RECYCLING_INITIATIVE: SceneRecipe = {
  family: "training-grounds",
  sky: ["sky-300", "green-200"],
  light: { x: 78, y: 18, r: 32, paint: "white", opacity: 0.35 },
  emblem: { x: 60, y: 79, size: 6, rotate: 0 },
  figure: { x: 36, y: 86, scale: 0.72, pose: "pose-spread", look: { ...RECRUIT, cloth: "blue-700" } },
  layers: {
    sky: [
      { part: "sun", x: 84, y: 16, scale: 0.7 },
      { part: "cloud", x: 30, y: 20, scale: 0.7, paint: "white" },
    ],
    far: [
      { part: "fence", x: 50, y: 60, scale: 1.2, paint: "wood-700" },
      { part: "tree", x: 12, y: 60, scale: 1.1 },
      { part: "tree", x: 90, y: 58, scale: 0.9 },
    ],
    mid: [
      { part: "yard-sand", x: 50, y: 62 },
      { part: "fence", x: 74, y: 72, scale: 0.5, paint: "wood-600" },
      { part: "card", x: 74, y: 67, scale: 0.5, count: 4, spread: [10, 0.5] },
    ],
    subject: [
      { part: "barrel", x: 60, y: 90, scale: 1.6 },
      { part: "bubbles", x: 60, y: 62, scale: 1 },
    ],
    near: [
      { part: "card", x: 48, y: 52, scale: 0.6, count: 3, spread: [9, 5], props: true },
      { part: "grass-tuft", x: 14, y: 94, count: 2, spread: [4, 1], paint: "green-700" },
      { part: "pebbles", x: 82, y: 95 },
    ],
    light: [
      { part: "halo", x: 60, y: 68, scale: 1.3, paint: "sky-300" },
      { part: "rim-light", x: 36, y: 62, scale: 1.1, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 58, y: 58, scale: 1, count: 2, spread: [6, 4], paint: "white" },
      { part: "motes", x: 50, y: 40, scale: 1, paint: "white" },
    ],
  },
};

/** Legacy: True Strike (Training Grounds #58, Common): a lone fencer lunges one perfect thrust through the heart of a pell post in the fencing court, already turning to leave; crisp white light. */
const LEGACY_TRUE_STRIKE: SceneRecipe = {
  family: "training-grounds",
  sky: ["blue-200", "white"],
  light: { x: 76, y: 18, r: 34, paint: "white", opacity: 0.4 },
  emblem: { x: 64, y: 68, size: 5, rotate: 0 },
  figure: { x: 42, y: 86, scale: 0.8, pose: "pose-run", look: { ...figureLookFor(["Human"]), cloth: "white" } },
  layers: {
    sky: [{ part: "sun", x: 84, y: 16, scale: 0.6 }],
    far: [{ part: "palisade", x: 50, y: 62, scale: 2, paint: "wood-700" }],
    mid: [{ part: "tile-floor", x: 50, y: 64, paint: "slate-400" }],
    subject: [{ part: "pell-post", x: 64, y: 86, scale: 1.4 }],
    near: [{ part: "grass-tuft", x: 88, y: 92, paint: "wood-600" }],
    light: [{ part: "light-shaft", x: 60, y: 10, paint: "white" }],
  },
};

/** Legacy: Fig of Life (Training Grounds #59, Common): a ripe purple fig bursts with green healing light over a wounded recruit on an infirmary cot, a basket of figs beside him; gentle morning. */
const LEGACY_FIG_OF_LIFE: SceneRecipe = {
  family: "training-grounds",
  sky: ["blue-200", "green-200"],
  light: { x: 52, y: 42, r: 28, paint: "green-400", opacity: 0.45 },
  emblem: { x: 52, y: 38, size: 5, rotate: 0 },
  figure: { x: 46, y: 84, scale: 0.6, pose: "pose-sit", look: { ...RECRUIT, cloth: "slate-100" } },
  layers: {
    far: [{ part: "room-wall", x: 50, y: 74, paint: "slate-100" }],
    mid: [{ part: "tile-floor", x: 50, y: 74, paint: "stone-500" }],
    subject: [
      { part: "cot", x: 50, y: 88, scale: 2 },
      { part: "apple", x: 52, y: 46, scale: 2.2, paint: "violet-700" },
    ],
    near: [{ part: "basket", x: 76, y: 92, scale: 1.1 }],
    light: [
      { part: "halo", x: 52, y: 41, scale: 1.9, paint: "green-400" },
      { part: "light-shaft", x: 70, y: 10, paint: "white" },
    ],
    particles: [{ part: "sparks", x: 52, y: 44, scale: 1, count: 2, spread: [6, 4], paint: "green-200" }],
  },
};

/** Legacy: Rush Token Farm (Training Grounds #60, Common): a ploughed field behind the yard where a scarecrow watches rows of runner tokens sprout from the furrows and sprint off; dewy dawn, brisk. */
const LEGACY_RUSH_TOKEN_FARM: SceneRecipe = {
  family: "training-grounds",
  sky: ["blue-800", "red-300"],
  light: { x: 82, y: 44, r: 30, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 50, y: 62, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 84, y: 46, scale: 0.8 },
      { part: "cloud", x: 28, y: 24, scale: 0.8, paint: "red-300" },
    ],
    far: [{ part: "palisade", x: 50, y: 58, scale: 1.6, paint: "wood-800" }],
    mid: [
      { part: "furrows", x: 50, y: 60 },
      { part: "sprout", x: 50, y: 72, count: 4, spread: [30, 4] },
    ],
    subject: [
      { part: "scarecrow", x: 50, y: 86, scale: 1.2 },
      { part: "runner-token", x: 30, y: 78, scale: 0.9 },
      { part: "runner-token", x: 72, y: 76, scale: 0.8, count: 2, spread: [6, 3] },
    ],
    near: [{ part: "grass-tuft", x: 14, y: 94, count: 2, spread: [5, 1], paint: "green-700" }],
    particles: [{ part: "mist", x: 50, y: 74, scale: 0.8, paint: "white" }],
  },
};

/** R3393001: the Legacy cards' recipes, by id. */
export const LEGACY_RECIPES: Readonly<Record<string, SceneRecipe>> = {
  "traininggrounds-041": LEGACY_SPECIESIST,
  "traininggrounds-042": LEGACY_STOCKPILE,
  "traininggrounds-043": LEGACY_MR_VANILLA,
  "traininggrounds-044": LEGACY_DUPLICATING_FELINORS,
  "traininggrounds-045": LEGACY_JLOCKEEDS_WEAPONS,
  "traininggrounds-046": LEGACY_HIT_JOB,
  "traininggrounds-047": LEGACY_MIDRANGE_MENACE,
  "traininggrounds-048": LEGACY_JLOCKEED_SHREDDER,
  "traininggrounds-049": LEGACY_REOCCURRING_DREAM,
  "traininggrounds-050": LEGACY_GLOWY_JELLY_BEAN,
  "traininggrounds-051": LEGACY_BIG_FELINOR,
  "traininggrounds-052": LEGACY_SNOM_BUNNY_MIND_CONTROL,
  "traininggrounds-053": LEGACY_PREJUDICED_POSTDOC,
  "traininggrounds-054": LEGACY_THE_ROCK,
  "traininggrounds-055": LEGACY_REMINISCE,
  "traininggrounds-056": LEGACY_CORPSE_EATER,
  "traininggrounds-057": LEGACY_RECYCLING_INITIATIVE,
  "traininggrounds-058": LEGACY_TRUE_STRIKE,
  "traininggrounds-059": LEGACY_FIG_OF_LIFE,
  "traininggrounds-060": LEGACY_RUSH_TOKEN_FARM,
};
