// R3392000: Core's scenes, part 2 of 3: Core #38 to #74 and the tokens KY's Empty Notebook (#51-1) and Spikey Pillow (#65-1), each drawn from its brief in the
// issue's table (#3392) in Core's family, the glass orb (ART.md, "Core" and "Core's scenes"). Each recipe keeps
// its subject within x 30 to 70 and y 25 to 85, its light from the upper left unless its brief names another
// source, and its emblem small on a prop or a chest; a Unit's character is a figure built on its tags' look, or
// a creature part in its subject layer. Each doc comment gives the brief in a line and any change made to fit
// the kits.

import { figureLookFor, KY_TUTOR_LOOK } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** Quickstriker (Core #38, Epic Field Spell): a ring of blades circling in a gust over a wind-scoured duelling platform of slashed practice posts, a distant fortress behind. Practice posts are the old `fence-post`. */
const QUICKSTRIKER: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 30, y: 24, r: 40, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 59, size: 6, rotate: 0 },
  layers: {
    sky: [
      { part: "cloud", x: 24, y: 24, scale: 1.1 },
      { part: "cloud", x: 78, y: 18, scale: 0.8 },
    ],
    far: [
      { part: "hills", x: 50, y: 62, paint: "blue-600" },
      { part: "fortress", x: 76, y: 58, scale: 1.1 },
      { part: "banner", x: 70, y: 46, scale: 0.7, flip: -1 },
      { part: "banner", x: 84, y: 45, scale: 0.7, flip: -1 },
    ],
    mid: [
      { part: "boards", x: 50, y: 70 },
      { part: "fence-post", x: 28, y: 70, scale: 1.1 },
      { part: "fence-post", x: 72, y: 70, scale: 1.1 },
      { part: "gust", x: 36, y: 66, scale: 1.2 },
    ],
    subject: [
      { part: "gust", x: 50, y: 46, scale: 2.1 },
      { part: "blade", x: 50, y: 48, scale: 1.3, count: 7, spread: [20, 11] },
    ],
    near: [
      { part: "fence-post", x: 18, y: 88, scale: 1.4 },
      { part: "fence-post", x: 82, y: 90, scale: 1.5 },
      { part: "fence-post", x: 56, y: 92, scale: 1.3 },
    ],
    light: [{ part: "halo", x: 50, y: 48, scale: 2.2, paint: "cyan-400" }],
    particles: [{ part: "leaves", x: 40, y: 56, scale: 1.2, count: 2, spread: [20, 10] }],
  },
};

/** Recycling Initiative (Core #39, Epic Spell): a glowing chute swallows used cards, looping glass pipes carry them back as fresh copies, in a bright sorting plant with tall windows and potted ferns. */
const RECYCLING_INITIATIVE: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "blue-200"],
  light: { x: 30, y: 26, r: 42, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 66, size: 5, rotate: 0 },
  layers: {
    sky: [],
    far: [
      { part: "room-wall", x: 50, y: 70, paint: "slate-100" },
      { part: "window", x: 22, y: 62, scale: 2 },
      { part: "window", x: 50, y: 62, scale: 2 },
      { part: "window", x: 78, y: 62, scale: 2 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 72 },
      { part: "fern", x: 10, y: 74, scale: 2.2 },
      { part: "fern", x: 92, y: 74, scale: 2.2, flip: -1 },
      { part: "pipe", x: 44, y: 46, scale: 1.3 },
      { part: "pipe", x: 82, y: 56, scale: 1, flip: -1 },
    ],
    subject: [
      { part: "chute", x: 32, y: 56, scale: 1.6 },
      { part: "card", x: 32, y: 60, scale: 0.9, count: 3, spread: [4, 6] },
      { part: "card-fan", x: 68, y: 78, scale: 1.6 },
    ],
    near: [
      { part: "card", x: 22, y: 86, scale: 1.1, count: 3, spread: [8, 3] },
      { part: "conveyor", x: 50, y: 92, scale: 0.9 },
    ],
    light: [{ part: "halo", x: 32, y: 62, scale: 1.2, paint: "cyan-400" }],
    particles: [{ part: "sparks", x: 56, y: 50, scale: 1.4, count: 3, spread: [24, 8] }],
  },
};

/** Echoes of the Forgotten (Core #40, Rare Field Spell): a hall of tall mirrors where faint ghost-figures stand in rows, each glass cracking more as a cold beam reaches out. */
const ECHOES_OF_THE_FORGOTTEN: SceneRecipe = {
  family: "core",
  sky: ["slate-700", "slate-400"],
  light: { x: 22, y: 22, r: 40, paint: "cyan-400", opacity: 0.35 },
  emblem: { x: 50, y: 28, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 72, paint: "slate-700" },
      { part: "mirror", x: 18, y: 72, scale: 1.7 },
      { part: "mirror", x: 50, y: 72, scale: 1.9 },
      { part: "mirror", x: 82, y: 72, scale: 1.7 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 74 },
      { part: "ghost", x: 34, y: 76, scale: 1.2 },
      { part: "ghost", x: 66, y: 76, scale: 1.2 },
      { part: "ghost", x: 50, y: 70, scale: 0.9 },
    ],
    subject: [
      { part: "light-shaft", x: 20, y: 14, scale: 2.2, paint: "cyan-400" },
      { part: "shards", x: 50, y: 50, scale: 2.4 },
    ],
    near: [
      { part: "ghost", x: 20, y: 90, scale: 1.4 },
      { part: "ghost", x: 82, y: 92, scale: 1.4 },
    ],
    light: [{ part: "echo-rings", x: 50, y: 52, scale: 2.2, paint: "cyan-400" }],
    particles: [{ part: "motes", x: 50, y: 60, scale: 1.6, paint: "slate-100" }],
  },
};

/** Sheepish (Core #41, Epic Trap): a puff of pink wool bursts as a charging armoured warrior becomes a startled sheep, a sprung trap rune glowing in the grass of a green meadow with grazing sheep and a dry-stone wall. Pink is red-300. */
const SHEEPISH: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 28, y: 24, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 85, size: 5, rotate: 0 },
  figure: { x: 30, y: 82, scale: 0.8, pose: "pose-run", look: { ...figureLookFor(["Human"]), hair: "steel-helmet", outfit: "outfit-plate", item: "item-sword", cloth: "slate-400" } },
  layers: {
    sky: [{ part: "cloud", x: 72, y: 20, scale: 0.9 }],
    far: [
      { part: "hills", x: 50, y: 56, paint: "green-500" },
      { part: "stone-wall", x: 66, y: 62, scale: 1.1 },
      { part: "sheep", x: 18, y: 62, scale: 0.7 },
      { part: "sheep", x: 40, y: 60, scale: 0.6, flip: -1 },
    ],
    mid: [
      { part: "field", x: 50, y: 66 },
      { part: "rune", x: 56, y: 82, scale: 2.6, paint: "cyan-400" },
      { part: "grass-tuft", x: 20, y: 74, count: 3, spread: [20, 5] },
    ],
    subject: [
      { part: "sheep", x: 62, y: 80, scale: 1.8, flip: -1 },
      { part: "puff", x: 48, y: 66, scale: 1.7, paint: "red-300" },
    ],
    near: [{ part: "flower", x: 40, y: 92, scale: 1.4, count: 4, spread: [30, 3] }],
    light: [],
    particles: [{ part: "puff", x: 54, y: 56, scale: 0.7, count: 3, spread: [14, 8], paint: "red-300" }],
  },
};

/** Selective Breeding (Core #42, Common Spell): a lab-coated breeder in a bright greenhouse tosses scrawny seedlings down a chute while a few tall plants in the rows begin to shimmer. */
const SELECTIVE_BREEDING: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "slate-100"],
  light: { x: 28, y: 26, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 62, y: 52, size: 5, rotate: 0 },
  figure: { x: 58, y: 82, scale: 0.85, pose: "pose-raise", look: { ...figureLookFor(["Human"]), hair: "spectacles", outfit: "outfit-labcoat", item: "item-clipboard", cloth: "white" } },
  layers: {
    far: [{ part: "greenhouse", x: 50, y: 66, scale: 1.2 }],
    mid: [
      { part: "furrows", x: 50, y: 72 },
      { part: "flower", x: 74, y: 76, scale: 2, count: 2, spread: [10, 3] },
    ],
    subject: [
      { part: "chute", x: 26, y: 64, scale: 1.4 },
      { part: "sprout", x: 26, y: 72, scale: 1.2, count: 2, spread: [6, 8] },
    ],
    near: [],
    light: [{ part: "heal-glow", x: 76, y: 66, scale: 1.4 }],
    particles: [],
  },
};

/** Big Felinor (Core #43, Rare Unit): a huge, burly cat-folk warrior cradles a kitten in one arm and swats a goblin away with a giant paw, on a cobbled village square. */
const BIG_FELINOR: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "slate-100"],
  light: { x: 28, y: 26, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 51, y: 56, size: 5, rotate: 0 },
  figure: { x: 50, y: 86, scale: 1.1, pose: "pose-lunge", look: { ...figureLookFor(["Felinor"]), outfit: "outfit-fur", item: null, cloth: "violet-700" } },
  layers: {
    sky: [{ part: "cloud", x: 78, y: 22, scale: 0.8 }],
    far: [
      { part: "house", x: 22, y: 62, scale: 1.4 },
      { part: "house", x: 80, y: 62, scale: 1.2, flip: -1 },
      { part: "well", x: 56, y: 60, scale: 0.9 },
    ],
    mid: [{ part: "cobbles", x: 50, y: 70 }],
    subject: [
      { part: "cat", x: 39, y: 59, scale: 1.3 },
      { part: "goblin", x: 77, y: 52, scale: 1.7 },
    ],
    near: [{ part: "crate", x: 16, y: 90, scale: 1.4 }],
    light: [{ part: "burst", x: 72, y: 57, scale: 0.8, paint: "white" }],
    particles: [{ part: "dust", x: 70, y: 62, scale: 1.2 }],
  },
};

/** True Strike (Core #44, Common Spell): an arrow pierces clean through a raised shield and the straw dummy behind it at a riverbank archery range, the archer already walking away. */
const TRUE_STRIKE: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "blue-200"],
  light: { x: 28, y: 26, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 36, y: 38, size: 5, rotate: 0 },
  cast: [{ x: 20, y: 80, scale: 0.55, flip: -1, pose: "pose-run", look: { ...figureLookFor(["Human"]), hair: "hat-brim", outfit: "outfit-jacket", item: null, cloth: "green-700" }, layer: "mid" }],
  layers: {
    far: [
      { part: "hills", x: 50, y: 56, paint: "green-700" },
      { part: "water", x: 50, y: 62 },
      { part: "reed", x: 14, y: 64, scale: 1.5, count: 2, spread: [8, 2] },
    ],
    mid: [{ part: "field", x: 50, y: 74 }],
    subject: [
      { part: "dummy", x: 58, y: 82, scale: 2.1 },
      { part: "shield", x: 58, y: 62, scale: 1.9 },
      { part: "arrow", x: 70, y: 62, scale: 2.2 },
    ],
    near: [{ part: "grass-tuft", x: 20, y: 92, count: 2, spread: [60, 2] }],
    light: [],
    particles: [],
  },
};

/** Deft Duelist (Core #45, Rare Unit): a nimble duellist in a feathered cap bows mid-sidestep, twin daggers spinning, on a sunlit palace balcony over the sea. */
const DEFT_DUELIST: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 26, y: 24, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 56, size: 5, rotate: 0 },
  figure: { x: 50, y: 84, scale: 1, pose: "pose-lunge", look: { ...figureLookFor(["Human"]), hair: "cap-feather", outfit: "outfit-waistcoat", item: "item-dagger", cloth: "red-600" } },
  layers: {
    sky: [{ part: "gull", x: 22, y: 30, scale: 1.4 }],
    far: [
      { part: "water", x: 50, y: 52 },
      { part: "ship", x: 76, y: 52, scale: 1 },
    ],
    mid: [
      { part: "stone-wall", x: 50, y: 70, scale: 2, paint: "slate-100" },
      { part: "column", x: 14, y: 72, scale: 1.4 },
      { part: "column", x: 86, y: 72, scale: 1.4 },
    ],
    subject: [{ part: "dagger", x: 68, y: 52, scale: 1.6, count: 2, spread: [6, 8] }],
    near: [{ part: "tile-floor", x: 50, y: 86 }],
    light: [],
    particles: [],
  },
};

/** Suppressive Aura (Core #46, Rare Field Spell): a grey-violet fog presses down on a sunken amphitheatre, warrior statues shrinking and bowing under it, a cracked rune glowing in the floor. */
const SUPPRESSIVE_AURA: SceneRecipe = {
  family: "core",
  sky: ["slate-400", "violet-300"],
  light: { x: 50, y: 30, r: 40, paint: "violet-300", opacity: 0.35 },
  emblem: { x: 50, y: 60, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "hills", x: 50, y: 52, paint: "slate-700" }],
    mid: [
      { part: "amphitheatre", x: 50, y: 66, scale: 1.2 },
      { part: "statue", x: 26, y: 64, scale: 1.2 },
      { part: "statue", x: 74, y: 64, scale: 1.2, flip: -1 },
    ],
    subject: [
      { part: "rune", x: 50, y: 68, scale: 2.4, paint: "violet-500" },
      { part: "statue", x: 38, y: 76, scale: 1.4 },
      { part: "statue", x: 62, y: 76, scale: 1.4, flip: -1 },
    ],
    near: [{ part: "mist", x: 50, y: 40, scale: 1.8, count: 2, spread: [14, 6], paint: "slate-400" }],
    light: [{ part: "mist", x: 50, y: 82, scale: 1.8, count: 2, spread: [20, 4], paint: "violet-300" }],
    particles: [{ part: "motes", x: 50, y: 56, scale: 1.4, paint: "violet-300" }],
  },
};

/** Fig of Life (Core #47, Common Spell): a ripe purple fig splits open over a wounded knight's palm, green healing light spilling out, in a sun-dappled orchard. The fig is an extra part (`fig`). */
const FIG_OF_LIFE: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "lime-200"],
  light: { x: 28, y: 24, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 30, y: 40, size: 5, rotate: 0 },
  figure: { x: 40, y: 84, scale: 1, pose: "pose-kneel", look: { ...figureLookFor(["Human"]), hair: "steel-helmet", outfit: "outfit-plate", item: null, cloth: "slate-400" } },
  layers: {
    far: [
      { part: "hills", x: 50, y: 60, paint: "green-500" },
      { part: "tree", x: 14, y: 62, scale: 1.6 },
      { part: "tree", x: 84, y: 62, scale: 1.8, flip: -1 },
    ],
    mid: [
      { part: "field", x: 50, y: 70 },
    ],
    near: [
      { part: "fig", x: 56, y: 52, scale: 3 },
      { part: "heal-glow", x: 56, y: 56, scale: 2.2 },
      { part: "grass-tuft", x: 30, y: 92, count: 2, spread: [40, 2] },
    ],
    light: [],
    particles: [],
  },
};

/** 5pek Controller (Core #48, Common Spell): a huge two-stick game controller tilts as toy soldiers on a tabletop battlefield all flip from standing to crouching at once, in a bright playroom. */
const FIVE_PEK_CONTROLLER: SceneRecipe = {
  family: "core",
  sky: ["blue-200", "slate-100"],
  light: { x: 28, y: 24, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 41, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "room-wall", x: 50, y: 66, paint: "blue-200" }],
    mid: [
      { part: "dice", x: 84, y: 64, scale: 2.4 },
    ],
    subject: [
      { part: "table", x: 50, y: 84, scale: 2.2 },
      { part: "controller", x: 50, y: 40, scale: 1.5 },
      { part: "soldier", x: 50, y: 68, scale: 1.4, count: 4, spread: [26, 2] },
    ],
    near: [],
    light: [],
    particles: [],
  },
};

/** Snom Bunny Mind Control (Core #49, Rare Spell): a fluffy white snow bunny with hypnotic swirling eyes, a dazed enemy knight trailing after it across a sparkling snowfield. */
const SNOM_BUNNY_MIND_CONTROL: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "slate-100"],
  light: { x: 70, y: 22, r: 38, paint: "white", opacity: 0.45 },
  emblem: { x: 62, y: 60, size: 5, rotate: 0 },
  figure: { x: 26, y: 80, scale: 0.8, pose: "pose-run", look: { ...figureLookFor(["Human"]), hair: "steel-helmet", outfit: "outfit-plate", item: "item-sword", cloth: "slate-400" } },
  layers: {
    sky: [{ part: "cloud", x: 30, y: 20, scale: 0.9 }],
    far: [
      { part: "hills", x: 50, y: 58, paint: "slate-100" },
      { part: "pine", x: 84, y: 58, scale: 1.6 },
      { part: "pine", x: 92, y: 60, scale: 1.2 },
    ],
    mid: [{ part: "snow", x: 50, y: 66 }],
    subject: [
      { part: "bunny", x: 66, y: 82, scale: 2.1, flip: -1 },
      { part: "vortex", x: 56, y: 62, scale: 0.9, paint: "violet-300" },
    ],
    near: [{ part: "grass-tuft", x: 30, y: 92, count: 3, spread: [30, 2], paint: "slate-100" }],
    light: [],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.6, paint: "white" }],
  },
};


/** K-Pop Fanatic (Core #50, Epic Unit): a starry-eyed fan in a sequined bomber jacket waves a heart-shaped light stick; a rival's treasure chest drifts toward her on pink sparkles before a packed concert arena under sweeping stage lights. Pink is red-300 and purple-400. */
const K_POP_FANATIC: SceneRecipe = {
  family: "core",
  sky: ["slate-900", "violet-700"],
  light: { x: 46, y: 24, r: 40, paint: "violet-300", opacity: 0.45 },
  emblem: { x: 78, y: 48, size: 5, rotate: 0 },
  figure: { x: 56, y: 86, scale: 0.95, pose: "pose-raise", look: { ...figureLookFor(["Human"]), hair: "hair-long", outfit: "outfit-bomber", item: "item-light-stick", cloth: "purple-400" } },
  layers: {
    sky: [{ part: "stars", x: 50, y: 20, scale: 2 }],
    far: [
      { part: "stadium", x: 50, y: 56, scale: 1.1 },
      { part: "spotlight", x: 18, y: 8, scale: 1.2, paint: "cyan-400" },
      { part: "spotlight", x: 82, y: 8, scale: 1.2, flip: -1, paint: "red-300" },
    ],
    mid: [{ part: "crowd", x: 50, y: 78, scale: 1.4 }],
    subject: [
      { part: "chest", x: 28, y: 60, scale: 2.4 },
      { part: "sparks", x: 28, y: 62, scale: 1.6, count: 2, spread: [8, 6], paint: "red-300" },
    ],
    near: [{ part: "bulbs", x: 50, y: 92, scale: 1.8 }],
    light: [{ part: "halo", x: 28, y: 58, scale: 1.2, paint: "purple-400" }],
    particles: [{ part: "confetti", x: 50, y: 40, scale: 1.8 }],
  },
};

/** KY's Private Tutor (Core #51, Epic Spell): KY, the cardigan-wearing tutor, points chalk at a fan of floating face-down cards; a polished desk with an hourglass and inkwell, a sunny home study with tall windows and globes behind. The inkwell is the jar. */
const KYS_PRIVATE_TUTOR: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "stone-500"],
  light: { x: 26, y: 24, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 62, y: 38, size: 5, rotate: 0 },
  figure: { x: 30, y: 86, scale: 0.95, pose: "pose-point", look: KY_TUTOR_LOOK },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 74, paint: "blue-200" },
      { part: "window", x: 26, y: 62, scale: 2.2 },
      { part: "window", x: 74, y: 62, scale: 2.2 },
      { part: "bookcase", x: 50, y: 74, scale: 1.3 },
    ],
    mid: [
      { part: "globe", x: 90, y: 76, scale: 2 },
      { part: "tile-floor", x: 50, y: 78 },
    ],
    subject: [
      { part: "card-fan", x: 62, y: 54, scale: 2.2 },
      { part: "table", x: 70, y: 92, scale: 1.4 },
      { part: "hourglass", x: 62, y: 74, scale: 2 },
      { part: "jar", x: 78, y: 74, scale: 1.4 },
    ],
    near: [{ part: "card", x: 50, y: 40, scale: 1, count: 2, spread: [14, 4] }],
    light: [{ part: "light-shaft", x: 24, y: 8, scale: 1.6, paint: "white" }],
    particles: [{ part: "motes", x: 56, y: 50, scale: 1.4, paint: "white" }],
  },
};

/** KY's Empty Notebook (Core token, Spell): a blank spiral notebook lies open on a park bench, its empty pages fluttering in the breeze, a dropped pencil in a sunbeam. The pencil is a small `arrow`, tip down. */
const KYS_EMPTY_NOTEBOOK: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "lime-200"],
  light: { x: 26, y: 22, r: 38, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 76, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "hills", x: 50, y: 56, paint: "green-500" },
      { part: "tree", x: 20, y: 60, scale: 1.6 },
      { part: "tree", x: 82, y: 60, scale: 1.4, flip: -1 },
    ],
    mid: [{ part: "field", x: 50, y: 68 }],
    subject: [
      { part: "bench", x: 50, y: 82, scale: 2.3 },
      { part: "notebook", x: 50, y: 70, scale: 2.4 },
    ],
    near: [
      { part: "arrow", x: 70, y: 90, scale: 0.8 },
      { part: "grass-tuft", x: 24, y: 92, count: 2, spread: [14, 2] },
    ],
    light: [{ part: "light-shaft", x: 70, y: 10, scale: 1.6, paint: "white" }],
    particles: [{ part: "leaves", x: 40, y: 50, scale: 1, count: 2, spread: [12, 6] }],
  },
};

/** Silly Silas (Core #52, Legendary Unit): a lanky dancer in a striped suit mid-spin on a giant rotating stage of square tiles, his heel cracking a crater into the floor; every piece slides one square sideways in a whirl, soldiers swapping sides, a spotlight cutting through confetti in violet and teal. The stage is the floating `chessboard`. */
const SILLY_SILAS: SceneRecipe = {
  family: "core",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 30, r: 44, paint: "violet-300", opacity: 0.5 },
  emblem: { x: 50, y: 56, size: 5, rotate: 0 },
  figure: { x: 50, y: 78, scale: 1, pose: "pose-spread", look: { ...figureLookFor(["Human"]), hair: "hair-long", outfit: "outfit-stripes", item: null, cloth: "slate-100" } },
  layers: {
    sky: [{ part: "stars", x: 50, y: 20, scale: 2.2 }],
    far: [
      { part: "crowd", x: 50, y: 56, scale: 1.8 },
      { part: "bulbs", x: 50, y: 30, scale: 2 },
    ],
    mid: [
      { part: "chessboard", x: 50, y: 74, scale: 1.2 },
      { part: "chess-piece", x: 26, y: 72, scale: 1.8, count: 3, spread: [6, 4] },
      { part: "soldier", x: 74, y: 72, scale: 1.4, count: 3, spread: [8, 3], flip: -1 },
      { part: "soldier", x: 30, y: 78, scale: 1.4, count: 2, spread: [6, 2] },
    ],
    subject: [
      { part: "shockwave", x: 52, y: 78, scale: 1.8, paint: "cyan-400" },
      { part: "shards", x: 52, y: 78, scale: 1.4 },
    ],
    near: [{ part: "streaks", x: 60, y: 90, scale: 1.2, count: 2, spread: [10, 3], paint: "violet-300" }],
    light: [
      { part: "spotlight", x: 50, y: 4, scale: 1.6 },
      { part: "god-rays", x: 50, y: 6, scale: 1.4, paint: "violet-300" },
      { part: "rim-light", x: 50, y: 56, scale: 1.2, paint: "cyan-400" },
    ],
    particles: [
      { part: "confetti", x: 50, y: 46, scale: 2, count: 3, spread: [16, 12] },
      { part: "sparks", x: 50, y: 70, scale: 1.4, count: 2, spread: [18, 6], paint: "cyan-400" },
    ],
  },
};

/** Reno (Core #53, Common Unit): a cheerful restorer in a canvas apron sweeps a shattered glass statue back together with a beam of green light, in a bright museum hall. */
const RENO: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "blue-200"],
  light: { x: 26, y: 24, r: 38, paint: "white", opacity: 0.45 },
  emblem: { x: 36, y: 58, size: 5, rotate: 0 },
  figure: { x: 34, y: 86, scale: 0.95, pose: "pose-cast", look: { ...figureLookFor(["Human"]), hair: "hair-short", outfit: "outfit-apron", item: null, cloth: "stone-500" } },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 72, paint: "slate-100" },
      { part: "column", x: 14, y: 72, scale: 1.4 },
      { part: "column", x: 90, y: 72, scale: 1.4 },
      { part: "window", x: 50, y: 62, scale: 1.6 },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 76 }],
    subject: [
      { part: "statue", x: 68, y: 80, scale: 1.8, paint: "sky-300" },
      { part: "shards", x: 68, y: 66, scale: 1.4 },
      { part: "zap", x: 50, y: 56, scale: 0.65, paint: "green-400" },
    ],
    near: [],
    light: [{ part: "heal-glow", x: 68, y: 66, scale: 1.8 }],
    particles: [],
  },
};

/** Straaza (Core #54, Common Unit): a great sapphire-scaled dragon mother coils around a nest of hatchlings atop a sea stack, offering two forward in her claws, bright surf below. The sea stack is a slate `mesa` over `water`. */
const STRAAZA: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 28, y: 24, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 36, y: 74, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 78, y: 22, scale: 0.9 }],
    far: [
      { part: "water", x: 50, y: 60 },
      { part: "gull", x: 24, y: 30, scale: 1.4 },
    ],
    mid: [{ part: "mesa", x: 48, y: 90, scale: 1.4, paint: "slate-700" }],
    subject: [
      { part: "nest", x: 36, y: 66, scale: 1.1 },
      { part: "dragon", x: 46, y: 66, scale: 1.1 },
      { part: "hatchling", x: 62.5, y: 48.4, scale: 1.1 },
      { part: "hatchling", x: 67, y: 48.6, scale: 1.1 },
    ],
    near: [],
    light: [],
    particles: [{ part: "spray", x: 28, y: 82, scale: 1.4, count: 2, spread: [30, 4] }],
  },
};

/** Lava Golem (Core #55, Rare Unit): a hulking golem of cooling black rock and glowing lava seams, three dented helmets fused into its chest, stomps out of a volcanic forge. */
const LAVA_GOLEM: SceneRecipe = {
  family: "core",
  sky: ["slate-900", "orange-400"],
  light: { x: 30, y: 66, r: 40, paint: "orange-400", opacity: 0.5 },
  emblem: { x: 54, y: 66, size: 5, rotate: 0 },
  figure: { x: 52, y: 86, scale: 1.1, pose: "pose-lunge", look: { ...figureLookFor(["Human"]), head: "head-stone", hair: null, outfit: "outfit-rock", item: null, skin: "slate-800", cloth: "slate-800" } },
  layers: {
    far: [
      { part: "hills", x: 50, y: 58, paint: "slate-800" },
      { part: "forge", x: 24, y: 72, scale: 1.8 },
    ],
    mid: [
      { part: "lava", x: 50, y: 78 },
      { part: "rock-floor", x: 50, y: 86 },
    ],
    subject: [],
    near: [
      { part: "helmet", x: 47, y: 56, scale: 1.4 },
      { part: "helmet", x: 53, y: 52, scale: 1.4 },
      { part: "helmet", x: 56, y: 58, scale: 1.4, flip: -1 },
      { part: "flame", x: 16, y: 90, scale: 1.4 },
    ],
    light: [],
    particles: [{ part: "embers", x: 44, y: 60, scale: 2, count: 2, spread: [14, 8] }],
  },
};

/** Jilliax (Core #56, Common Unit): a gleaming chrome robot knight with a toothy grille mouth, a shimmering bubble shield and rocket heels charges across a clean steel hangar. The hangar is a slate `tile-wall` between columns. */
const JILLIAX: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "slate-400"],
  light: { x: 26, y: 24, r: 38, paint: "white", opacity: 0.45 },
  emblem: { x: 52, y: 56, size: 5, rotate: 0 },
  figure: { x: 52, y: 86, scale: 1.08, pose: "pose-run", look: { ...figureLookFor(["Human"]), head: "head-robot", hair: "visor", outfit: "outfit-plate", item: "item-sword", skin: "slate-400", cloth: "slate-100" } },
  layers: {
    far: [
      { part: "tile-wall", x: 50, y: 70, paint: "slate-400" },
      { part: "column", x: 20, y: 72, scale: 1.6 },
      { part: "column", x: 82, y: 72, scale: 1.6 },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 76 }],
    subject: [],
    near: [
      { part: "halo", x: 54, y: 60, scale: 1.6, paint: "cyan-400" },
      { part: "flame", x: 44, y: 86, scale: 1.2, flip: -1 },
    ],
    light: [{ part: "streaks", x: 40, y: 80, scale: 1.2, count: 2, spread: [4, 6], paint: "white" }],
    particles: [],
  },
};

/** Conjure KY (Core #57, Common Spell): KY, chalk raised mid-lecture, is yanked through a swirling blue portal into a summoner's circle in a bright lakeside pavilion, a pointer stick flying. The pointer stick is an `arrow`. */
const CONJURE_KY: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 28, y: 24, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 64, y: 78, size: 5, rotate: 0 },
  figure: { x: 52, y: 80, scale: 0.9, pose: "pose-raise", look: KY_TUTOR_LOOK },
  layers: {
    far: [
      { part: "water", x: 50, y: 58 },
      { part: "pavilion", x: 80, y: 62, scale: 1.2 },
      { part: "tree", x: 14, y: 60, scale: 1.5 },
    ],
    mid: [{ part: "field", x: 50, y: 70 }],
    subject: [
      { part: "rune", x: 54, y: 82, scale: 3.4, paint: "cyan-400" },
      { part: "portal", x: 26, y: 78, scale: 1.7, paint: "blue-600" },
    ],
    near: [{ part: "arrow", x: 74, y: 44, scale: 1, flip: -1 }],
    light: [],
    particles: [{ part: "vortex", x: 30, y: 62, scale: 0.9, paint: "cyan-400" }],
  },
};

/** Rush Token Farm (Core #58, Common Field Spell): a neat farm of tilled rows where little round Rush Tokens sprout like turnips and sprint off, a red barn behind, on a bright spring day. */
const RUSH_TOKEN_FARM: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "lime-200"],
  light: { x: 28, y: 24, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 22, y: 62, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 72, y: 20, scale: 0.9 }],
    far: [
      { part: "hills", x: 50, y: 54, paint: "green-500" },
      { part: "barn", x: 72, y: 60, scale: 1.4 },
    ],
    mid: [{ part: "furrows", x: 50, y: 62 }],
    subject: [
      { part: "rush-token", x: 40, y: 80, scale: 2, count: 3, spread: [16, 4] },
      { part: "sprout", x: 30, y: 70, scale: 1.4, count: 3, spread: [20, 3] },
    ],
    near: [{ part: "fence", x: 50, y: 94, scale: 1.2 }],
    light: [],
    particles: [{ part: "dust", x: 66, y: 82, scale: 1.2 }],
  },
};

/** Unbiased Immigration (Core #59, Rare Field Spell): a bright harbour arrival hall with its front doors flung open, travellers from every land queueing in, each holding a wrapped gift. The travellers are one figure and two smaller ones. */
const UNBIASED_IMMIGRATION: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "slate-100"],
  light: { x: 50, y: 40, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 38, size: 5, rotate: 0 },
  figure: { x: 50, y: 88, scale: 0.85, pose: "pose-hold", look: { ...figureLookFor(["Human"]), hair: "hat-brim", outfit: "outfit-coat", item: null, cloth: "green-700" } },
  cast: [
    { x: 34, y: 76, scale: 0.55, pose: "pose-hold", look: { ...figureLookFor(["Felinor"]), item: null }, layer: "mid" },
    { x: 68, y: 74, scale: 0.5, flip: -1, pose: "pose-hold", look: { ...figureLookFor(["Human"]), hair: "bucket-hat", outfit: "outfit-sweater", item: null, cloth: "red-600" }, layer: "mid" },
  ],
  layers: {
    far: [
      { part: "water", x: 50, y: 52 },
      { part: "ship", x: 62, y: 52, scale: 1.1 },
    ],
    mid: [
      { part: "aisle", x: 50, y: 66 },
      { part: "door", x: 22, y: 76, scale: 3, flip: -1 },
      { part: "door", x: 78, y: 76, scale: 3 },
    ],
    subject: [{ part: "gift", x: 55.5, y: 62, scale: 2.4 }],
    near: [],
    light: [],
    particles: [],
  },
};

/** Bear Honeypot (Core #60, Epic Trap): a dripping honeypot on a forest stump as a grinning bear bursts out of the ferns; a startled raider's hand reaches for the pot; sunlit pine woods and buzzing bees behind. */
const BEAR_HONEYPOT: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "lime-200"],
  light: { x: 26, y: 22, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 60, y: 82, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 78, y: 22, scale: 0.8 }],
    far: [
      { part: "hills", x: 50, y: 56, paint: "green-700" },
      { part: "pine", x: 16, y: 60, scale: 2.2 },
      { part: "pine", x: 30, y: 58, scale: 1.8 },
      { part: "pine", x: 84, y: 60, scale: 2.2 },
      { part: "bee", x: 66, y: 42, scale: 1.8, count: 3, spread: [12, 6] },
    ],
    mid: [
      { part: "field", x: 50, y: 66 },
      { part: "fern", x: 20, y: 74, scale: 2.4, count: 3, spread: [10, 3] },
    ],
    subject: [
      { part: "bear", x: 36, y: 84, scale: 1.5 },
      { part: "stump", x: 62, y: 86, scale: 3.2 },
      { part: "honeypot", x: 62, y: 70, scale: 2.6 },
      { part: "open-hand", x: 90, y: 62, scale: 2.4, flip: -1 },
    ],
    near: [{ part: "fern", x: 14, y: 92, scale: 2.6, count: 3, spread: [60, 2] }],
    light: [],
    particles: [],
  },
};


/** Prejudiced Postdoc (Core #61, Rare Unit): a frazzled postdoc in a lab coat cranks a copier that spits out a blank-faced clone of a soldier, in a cramped university copy room. */
const PREJUDICED_POSTDOC: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "stone-500"],
  light: { x: 26, y: 24, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 62, y: 56, size: 5, rotate: 0 },
  figure: { x: 34, y: 86, scale: 0.95, pose: "pose-raise", look: { ...figureLookFor(["Human"]), hair: "spectacles", outfit: "outfit-labcoat", item: null, cloth: "white" } },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 74, paint: "stone-500" },
      { part: "shelf", x: 20, y: 74, scale: 1.6 },
      { part: "window", x: 84, y: 60, scale: 1.4 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 78 },
    ],
    subject: [
      { part: "copier", x: 64, y: 84, scale: 2.1 },
      { part: "soldier", x: 82, y: 84, scale: 1.7, paint: "slate-100" },
      { part: "card", x: 58, y: 62, scale: 1.2, count: 2, spread: [10, 3] },
    ],
    near: [],
    light: [{ part: "light-shaft", x: 22, y: 10, scale: 1.4, paint: "white" }],
    particles: [{ part: "sparks", x: 64, y: 62, scale: 1.2 }],
  },
};

/** Friend of Felinors (Core #62, Common Spell): a garden party where kitten-sized cat-folk pour over the fence and fill every chair at a long picnic table, bunting fluttering overhead. The cat-folk are `cat` parts. */
const FRIEND_OF_FELINORS: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "lime-200"],
  light: { x: 28, y: 24, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 66, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "hills", x: 50, y: 56, paint: "green-500" },
      { part: "bunting", x: 50, y: 28, scale: 1.8 },
    ],
    mid: [
      { part: "field", x: 50, y: 70 },
    ],
    subject: [
      { part: "chair", x: 38, y: 80, scale: 1.5, count: 3, spread: [18, 0] },
      { part: "cat", x: 38, y: 74, scale: 2.6, count: 3, spread: [18, 0] },
      { part: "table", x: 50, y: 86, scale: 1.9 },
    ],
    near: [{ part: "grass-tuft", x: 14, y: 92, count: 3, spread: [16, 2] }],
    light: [],
    particles: [],
  },
};

/** Plastic Surgery (Core #63, Common Spell): a surgeon stitches a soldier on a gleaming operating table, the soldier's arm swelling with muscle and sprouting an unexpected spike. The arm is a red `fist` with a crystal spike; the table is a slate-100 `table`. */
const PLASTIC_SURGERY: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "blue-200"],
  light: { x: 40, y: 24, r: 38, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 52, size: 5, rotate: 0 },
  figure: { x: 24, y: 86, scale: 0.95, pose: "pose-cast", look: { ...figureLookFor(["Human"]), hair: "face-mask", outfit: "outfit-labcoat", item: null, cloth: "green-400" } },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 74, paint: "blue-200" },
      { part: "window", x: 76, y: 58, scale: 1.6 },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 80 }],
    subject: [
      { part: "table", x: 56, y: 82, scale: 2, paint: "slate-100" },
      { part: "soldier-fallen", x: 50, y: 64, scale: 1.8 },
      { part: "fist", x: 74, y: 62, scale: 2.4, paint: "red-300" },
      { part: "crystal", x: 78, y: 58, scale: 0.9, paint: "slate-100" },
    ],
    near: [],
    light: [{ part: "halo", x: 50, y: 34, scale: 2, paint: "white" }],
    particles: [],
  },
};

/** Gifted Program (Core #64, Rare Field Spell): a treehouse classroom for prodigies: tiny desks with shimmering cards, one small student smugly holding hers aloft, ribbons pinned up, sunlight through leaves. The ribbons are bunting. */
const GIFTED_PROGRAM: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "lime-200"],
  light: { x: 26, y: 22, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 56, y: 30, size: 5, rotate: 0 },
  figure: { x: 58, y: 86, scale: 0.75, pose: "pose-raise", look: { ...figureLookFor(["Human"]), hair: "spectacles", outfit: "outfit-sweater", item: "item-card", cloth: "red-600" } },
  layers: {
    far: [
      { part: "tree", x: 12, y: 70, scale: 2 },
      { part: "tree", x: 90, y: 70, scale: 2, flip: -1 },
      { part: "treehouse", x: 52, y: 72, scale: 1.3 },
    ],
    mid: [
      { part: "boards", x: 50, y: 78 },
      { part: "table", x: 30, y: 84, scale: 0.8 },
      { part: "table", x: 82, y: 82, scale: 0.8 },
      { part: "card", x: 30, y: 74, scale: 1, count: 2, spread: [6, 1] },
    ],
    subject: [],
    near: [{ part: "bunting", x: 50, y: 24, scale: 2 }],
    light: [{ part: "light-shaft", x: 22, y: 8, scale: 1.6, paint: "white" }],
    particles: [{ part: "leaves", x: 50, y: 44, scale: 1.2, count: 2, spread: [20, 10] }],
  },
};

/** Masochism Mask (Core #65, Epic Field Spell): a spiked crimson mask hangs on a glass pedestal; a cushion of thorns and a dripping blade lie on a polished altar; a clean white shrine whose walls shimmer with faint card shapes stands behind. The thorn cushion is the Spikey Pillow part. */
const MASOCHISM_MASK: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "blue-200"],
  light: { x: 30, y: 24, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 74, paint: "slate-100" },
      { part: "column", x: 16, y: 74, scale: 1.8 },
      { part: "column", x: 84, y: 74, scale: 1.8 },
      { part: "card", x: 30, y: 44, scale: 1.4, count: 3, spread: [14, 10] },
      { part: "card", x: 74, y: 44, scale: 1.4, count: 3, spread: [8, 10] },
    ],
    mid: [
      { part: "counter", x: 50, y: 78, scale: 2.1, paint: "slate-100" },
      { part: "spiky-pillow", x: 28, y: 72, scale: 1.1 },
      { part: "blade", x: 72, y: 66, scale: 1.1 },
    ],
    subject: [
      { part: "pedestal", x: 50, y: 90, scale: 3.4, paint: "sky-300" },
      { part: "mask", x: 50, y: 56, scale: 3 },
    ],
    near: [{ part: "drops", x: 70, y: 74, scale: 1, paint: "red-500" }],
    light: [{ part: "halo", x: 50, y: 56, scale: 1.6, paint: "red-300" }],
    particles: [],
  },
};

/** Spikey Pillow (Core token, Unit): a plump velvet pillow bristling with steel spikes on a sunny window seat, a house cat eyeing it warily from the floor. The seat is a wood `counter`. */
const SPIKEY_PILLOW: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "slate-100"],
  light: { x: 28, y: 22, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 74, paint: "slate-100" },
      { part: "window", x: 50, y: 62, scale: 3.2 },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 86 }],
    subject: [
      { part: "counter", x: 50, y: 84, scale: 1.5, paint: "wood-700" },
      { part: "spiky-pillow", x: 50, y: 70, scale: 1.6 },
    ],
    near: [{ part: "cat", x: 20, y: 92, scale: 2.6 }],
    light: [{ part: "light-shaft", x: 24, y: 8, scale: 1.6, paint: "white" }],
    particles: [],
  },
};

/** The Rock (Core #66, Common Unit): a literal boulder with a weathered, raised-eyebrow face, wearing a tiny apron beside an empty frying pan on a sunny mountain ledge. */
const THE_ROCK: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 28, y: 22, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 76, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 74, y: 22, scale: 1 }],
    far: [
      { part: "hills", x: 50, y: 62, paint: "slate-400" },
      { part: "cliff", x: 90, y: 62, scale: 1.4, flip: -1 },
    ],
    mid: [{ part: "rock-floor", x: 50, y: 82, paint: "slate-400" }],
    subject: [
      { part: "boulder", x: 46, y: 84, scale: 1.6 },
      { part: "pan", x: 76, y: 86, scale: 2.4 },
    ],
    near: [{ part: "grass-tuft", x: 16, y: 92, count: 2, spread: [12, 2] }],
    light: [],
    particles: [],
  },
};

/** Zoomerbin Oomen (Core #67, Rare Unit): a grinning teen in an oversized hoodie and bucket hat slips a hidden snare under a ramp at a sunny skate park. */
const ZOOMERBIN_OOMEN: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 28, y: 22, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 56, y: 60, size: 5, rotate: 0 },
  figure: { x: 36, y: 86, scale: 1, pose: "pose-kneel", look: { ...figureLookFor(["Human"]), hair: "bucket-hat", outfit: "outfit-hoodie", item: null, cloth: "violet-700" } },
  layers: {
    sky: [{ part: "cloud", x: 72, y: 22, scale: 0.8 }],
    far: [
      { part: "skyline", x: 50, y: 56, scale: 1.2 },
      { part: "tree", x: 90, y: 62, scale: 1.3 },
    ],
    mid: [
      { part: "road", x: 50, y: 66, paint: "slate-400" },
      { part: "bicycle", x: 18, y: 72, scale: 1.2 },
    ],
    subject: [
      { part: "ramp", x: 76, y: 84, scale: 2.2 },
      { part: "snare", x: 54, y: 82, scale: 2.4 },
    ],
    near: [],
    light: [],
    particles: [{ part: "dust", x: 56, y: 80, scale: 1 }],
  },
};

/** Twisted Sorcerer (Core #68, Common Unit): a gaunt sorcerer with coiled horns and a gnarled spiral staff casts a jagged bolt from a crumbling clifftop tower, in stormy blue light. */
const TWISTED_SORCERER: SceneRecipe = {
  family: "core",
  sky: ["slate-800", "blue-800"],
  light: { x: 40, y: 30, r: 40, paint: "cyan-400", opacity: 0.35 },
  emblem: { x: 40, y: 58, size: 5, rotate: 0 },
  figure: { x: 40, y: 80, scale: 1, pose: "pose-cast", look: { ...figureLookFor(["Human"]), head: "head-ghoul", hair: "horns", outfit: "outfit-robe", item: "item-staff", skin: "slate-400", cloth: "violet-700" } },
  layers: {
    sky: [{ part: "cloud", x: 30, y: 20, scale: 1.2, paint: "slate-700" }],
    far: [
      { part: "tower", x: 80, y: 72, scale: 1.4 },
      { part: "cloud", x: 78, y: 28, scale: 1, paint: "slate-700" },
    ],
    mid: [{ part: "cliff", x: 50, y: 80, scale: 1.4 }],
    subject: [{ part: "zap", x: 56, y: 52, scale: 1.1, paint: "cyan-400" }],
    near: [{ part: "rock-floor", x: 50, y: 94 }],
    light: [],
    particles: [{ part: "drops", x: 30, y: 40, scale: 1.4, count: 2, spread: [20, 10] }],
  },
};

/** Call to Arms (Core #69, Common Spell): a trumpeter blasts a call from a castle wall as a stream of small soldiers marches out of the gate under bright banners, in a clear sky. */
const CALL_TO_ARMS: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 28, y: 22, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 72, size: 5, rotate: 0 },
  cast: [{ x: 32, y: 54, scale: 0.65, pose: "pose-cast", look: { ...figureLookFor(["Human"]), hair: "steel-helmet", outfit: "outfit-armor", item: "item-trumpet", cloth: "red-600" }, layer: "mid" }],
  layers: {
    sky: [{ part: "cloud", x: 76, y: 22, scale: 0.9 }],
    far: [],
    mid: [
      { part: "battlement", x: 50, y: 80, scale: 1.7 },
      { part: "banner", x: 62, y: 52, scale: 1.1 },
    ],
    subject: [
      { part: "gate", x: 72, y: 82, scale: 1.3 },
      { part: "soldier", x: 56, y: 90, scale: 1.5, count: 3, spread: [12, 2] },
    ],
    near: [],
    light: [],
    particles: [],
  },
};

/** Spiteful Stab (Core #70, Common Spell): a scarred, bandaged hero thrusts a dagger that grows longer and keener with every scar, in a dusty gladiator pit under hard sunlight. */
const SPITEFUL_STAB: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 70, y: 20, r: 36, paint: "white", opacity: 0.55 },
  emblem: { x: 40, y: 58, size: 5, rotate: 0 },
  figure: { x: 38, y: 86, scale: 1.05, pose: "pose-lunge", look: { ...figureLookFor(["Human"]), hair: "headband", outfit: "outfit-rags", item: "item-dagger", cloth: "stone-500" } },
  layers: {
    far: [
      { part: "stone-wall", x: 50, y: 62, scale: 2 },
      { part: "column", x: 12, y: 64, scale: 1.4 },
      { part: "column", x: 88, y: 64, scale: 1.4 },
    ],
    mid: [{ part: "sand", x: 50, y: 66 }],
    subject: [{ part: "dagger", x: 72, y: 54, scale: 3 }],
    near: [{ part: "dust", x: 60, y: 90, scale: 1.4 }],
    light: [{ part: "rim-light", x: 38, y: 58, scale: 1.2, paint: "white" }],
    particles: [{ part: "sparks", x: 84, y: 48, scale: 1, count: 2, spread: [4, 6] }],
  },
};

/** Intern Stimmy (Core #71, Rare Field Trap): a jittery intern bursts out of a supply closet with a clipboard and coffee, dragging a startled recruit along by the sleeve, in a bright office. */
const INTERN_STIMMY: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "blue-200"],
  light: { x: 28, y: 22, r: 38, paint: "white", opacity: 0.45 },
  emblem: { x: 46, y: 50, size: 5, rotate: 0 },
  figure: { x: 46, y: 86, scale: 0.95, pose: "pose-run", look: { ...figureLookFor(["Human"]), hair: "spectacles", outfit: "outfit-sweater", item: "item-clipboard", cloth: "green-700" } },
  cast: [{ x: 70, y: 86, scale: 0.85, flip: -1, pose: "pose-stand", look: { ...figureLookFor(["Human"]), hair: "hair-long", outfit: "outfit-blazer", item: null, cloth: "slate-700" }, layer: "subject" }],
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 74, paint: "slate-100" },
      { part: "window", x: 76, y: 60, scale: 1.8 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 80 },
      { part: "door", x: 20, y: 82, scale: 3.6 },
    ],
    subject: [{ part: "cup", x: 58, y: 54, scale: 2.2 }],
    near: [{ part: "card", x: 30, y: 90, scale: 1.2, count: 3, spread: [10, 3] }],
    light: [],
    particles: [{ part: "streaks", x: 40, y: 76, scale: 1, count: 2, spread: [4, 8], paint: "white" }],
  },
};

/** Reminisce (Core #72, Rare Spell): an old soldier gazes into a glowing photo album as one cherished card floats out of a sunlit memory, in a quiet attic. The photo album is a `tome`. */
const REMINISCE: SceneRecipe = {
  family: "core",
  sky: ["stone-500", "slate-400"],
  light: { x: 28, y: 24, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 58, y: 38, size: 5, rotate: 0 },
  figure: { x: 38, y: 86, scale: 1, pose: "pose-sit", look: { ...figureLookFor(["Human"]), hair: "steel-helmet", outfit: "outfit-waistcoat", item: null, cloth: "wood-700" } },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 76, paint: "stone-500" },
      { part: "window", x: 74, y: 56, scale: 2 },
    ],
    mid: [
      { part: "boards", x: 50, y: 78 },
      { part: "crate", x: 84, y: 82, scale: 1.8 },
      { part: "lantern", x: 20, y: 36, scale: 1 },
    ],
    subject: [
      { part: "chair", x: 38, y: 86, scale: 1.9 },
      { part: "tome", x: 47, y: 72, scale: 2.2 },
      { part: "card", x: 52, y: 52, scale: 1.8 },
    ],
    near: [],
    light: [
      { part: "light-shaft", x: 24, y: 8, scale: 1.6, paint: "white" },
      { part: "halo", x: 50, y: 62, scale: 1.2, paint: "green-200" },
    ],
    particles: [{ part: "motes", x: 56, y: 56, scale: 1.2, paint: "white" }],
  },
};

/** Anti-oneshot Armor (Core #73, Rare Field Spell): a frosted steel bastion of layered glassy plate walls, a giant hammer blow shattering into harmless sparks against them, in cold, clean light. */
const ANTI_ONESHOT_ARMOR: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "slate-100"],
  light: { x: 28, y: 24, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 36, y: 72, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 24, y: 22, scale: 0.9 }],
    far: [{ part: "hills", x: 50, y: 62, paint: "slate-100" }],
    mid: [{ part: "snow", x: 50, y: 78 }],
    subject: [
      { part: "bastion", x: 40, y: 84, scale: 1.4 },
      { part: "hammer", x: 76, y: 38, scale: 1.8 },
      { part: "shards", x: 68, y: 54, scale: 1.4 },
    ],
    near: [{ part: "burst", x: 70, y: 56, scale: 0.8, paint: "white" }],
    light: [],
    particles: [{ part: "sparks", x: 70, y: 56, scale: 1.4, count: 2, spread: [8, 8] }],
  },
};

/** Adaptive UI (Core #74, Rare Spell): a hologram panel unfolds four tiles that fire a bolt, mend a wound, deal a card and hatch a ghoul, in a bright control room. */
const ADAPTIVE_UI: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "slate-400"],
  light: { x: 50, y: 40, r: 40, paint: "cyan-400", opacity: 0.35 },
  emblem: { x: 50, y: 34, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 74, paint: "slate-400" },
      { part: "window", x: 14, y: 56, scale: 1.8 },
      { part: "window", x: 86, y: 56, scale: 1.8 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 80 },
      { part: "gear", x: 16, y: 80, scale: 1.4 },
    ],
    subject: [
      { part: "hologram", x: 50, y: 56, scale: 1.9 },
      { part: "zap", x: 34, y: 48, scale: 0.4, paint: "cyan-400" },
      { part: "heal-glow", x: 45, y: 56, scale: 0.9 },
      { part: "card", x: 57, y: 58, scale: 1.2 },
      { part: "ghost", x: 69, y: 62, scale: 0.7, paint: "green-400" },
    ],
    near: [],
    light: [{ part: "halo", x: 50, y: 56, scale: 2.4, paint: "cyan-400" }],
    particles: [],
  },
};


export const CORE_SCENES_2: Readonly<Record<string, SceneRecipe>> = {
  "core-038": QUICKSTRIKER,
  "core-039": RECYCLING_INITIATIVE,
  "core-040": ECHOES_OF_THE_FORGOTTEN,
  "core-041": SHEEPISH,
  "core-042": SELECTIVE_BREEDING,
  "core-043": BIG_FELINOR,
  "core-044": TRUE_STRIKE,
  "core-045": DEFT_DUELIST,
  "core-046": SUPPRESSIVE_AURA,
  "core-047": FIG_OF_LIFE,
  "core-048": FIVE_PEK_CONTROLLER,
  "core-049": SNOM_BUNNY_MIND_CONTROL,
  "core-050": K_POP_FANATIC,
  "core-051": KYS_PRIVATE_TUTOR,
  "core-051-1": KYS_EMPTY_NOTEBOOK,
  "core-052": SILLY_SILAS,
  "core-053": RENO,
  "core-054": STRAAZA,
  "core-055": LAVA_GOLEM,
  "core-056": JILLIAX,
  "core-057": CONJURE_KY,
  "core-058": RUSH_TOKEN_FARM,
  "core-059": UNBIASED_IMMIGRATION,
  "core-060": BEAR_HONEYPOT,
  "core-061": PREJUDICED_POSTDOC,
  "core-062": FRIEND_OF_FELINORS,
  "core-063": PLASTIC_SURGERY,
  "core-064": GIFTED_PROGRAM,
  "core-065": MASOCHISM_MASK,
  "core-065-1": SPIKEY_PILLOW,
  "core-066": THE_ROCK,
  "core-067": ZOOMERBIN_OOMEN,
  "core-068": TWISTED_SORCERER,
  "core-069": CALL_TO_ARMS,
  "core-070": SPITEFUL_STAB,
  "core-071": INTERN_STIMMY,
  "core-072": REMINISCE,
  "core-073": ANTI_ONESHOT_ARMOR,
  "core-074": ADAPTIVE_UI,
};
