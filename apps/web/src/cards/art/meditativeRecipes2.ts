// R3401000: Meditative's scenes, part 2 of 5: Meditative #15 to #30 with #19's, #22's, #28's and #30's tokens, each
// drawn from its brief in the issue's table (#3401) in Meditative's family, the ink wash (ART.md, "Meditative and
// Meditative+" and "Meditative drawn as scenes"): slate-900 ink on slate-100 paper, jade washes, a high and distant
// camera, and one red seal in the near layer. Each recipe keeps its subject within x 30 to 70 and y 25 to 85 and its
// emblem small on its subject; a Unit's character is a figure built on its tags' look, or its part of the scene kit in
// its subject layer. Each doc comment gives the brief in a line and any change made to fit the kits.

import { figureLookFor } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** Smelly Steven (Meditative #15, Common Unit): a rumpled student in a stained hoodie hunched in a library carrel, green stink clouds wilting the spellbooks around him, classmates edging away, fluorescent light. Changed: the stink is green smoke and spores over the shelved spellbooks, which the kit cannot wilt. */
const SMELLY_STEVEN: SceneRecipe = {
  family: "meditative",
  sky: ["slate-100", "slate-400"],
  light: { x: 50, y: 18, r: 44, paint: "white", opacity: 0.55 },
  emblem: { x: 50, y: 56, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 82,
    scale: 1,
    pose: "pose-perch",
    look: { ...figureLookFor(["Human"]), hair: "hair-long", outfit: "outfit-hoodie", item: "item-book", cloth: "lime-400" },
  },
  layers: {
    far: [
      { part: "bookshelf", x: 16, y: 66, scale: 1.4 },
      { part: "bookshelf", x: 84, y: 66, scale: 1.4 },
      { part: "bookshelf", x: 50, y: 60, scale: 1.1 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66 },
      { part: "ink-person", x: 18, y: 78, scale: 1.3, flip: -1 },
      { part: "ink-person", x: 84, y: 78, scale: 1.3 },
    ],
    subject: [{ part: "chair", x: 50, y: 84, scale: 1 }],
    near: [
      { part: "desk", x: 62, y: 88, scale: 1.5 },
      { part: "closed-tome", x: 64, y: 76, scale: 0.8 },
      { part: "smoke", x: 36, y: 62, scale: 1, paint: "lime-400" },
      { part: "smoke", x: 66, y: 62, scale: 0.9, paint: "green-400" },
      { part: "spore-motes", x: 50, y: 50, scale: 1.2, paint: "lime-400" },
      { part: "ink-seal", x: 82, y: 70 },
    ],
  },
};

/** Trenful Trickster (Meditative #16, Rare Unit): a grinning masked trickster in a patchwork robe tucking a spring snare under a riverboat's planks at dusk, a copper coin left as a tip. Changed: his mask is his hood (the figure kit has no carnival mask) and the spring snare a rope snare. */
const TRENFUL_TRICKSTER: SceneRecipe = {
  family: "meditative",
  sky: ["violet-700", "red-300"],
  light: { x: 70, y: 40, r: 34, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 46, y: 64, size: 5, rotate: 0 },
  figure: {
    x: 44,
    y: 82,
    scale: 1,
    pose: "pose-kneel",
    look: { ...figureLookFor([]), outfit: "outfit-patched", item: "item-coin", cloth: "red-600" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 56, scale: 0.8 },
      { part: "mist", x: 50, y: 58, scale: 0.8 },
    ],
    mid: [
      { part: "still-water", x: 50, y: 62 },
      { part: "boat", x: 80, y: 68, scale: 0.9, flip: -1 },
      { part: "reed", x: 12, y: 68, scale: 1.2 },
    ],
    subject: [{ part: "boards", x: 50, y: 74 }],
    near: [
      { part: "snare", x: 64, y: 84, scale: 1.4 },
      { part: "coin", x: 72, y: 82, scale: 0.8, paint: "brass-600" },
      { part: "paper-lantern", x: 22, y: 26, scale: 1 },
      { part: "ink-seal", x: 82, y: 72 },
    ],
  },
};

/** True Craft a Card (Meditative #17, Mythic Spell): a hermit inventor's cliffside workshop at night, a blank card floating over the bench as threads of pixel light weave into it, a half-made creature peeling up off it, sparks of colour like fireflies, ink-wash peaks beyond the round window, a cool blue-white glow on the maker's face. Changed: the threads of pixel light are pixel static and a spiral of square outlines round the card. */
const TRUE_CRAFT_A_CARD: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "blue-900"],
  light: { x: 58, y: 56, r: 32, paint: "sky-300", opacity: 0.5 },
  emblem: { x: 58, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 28,
    y: 86,
    scale: 1,
    pose: "pose-cast",
    look: { ...figureLookFor([]), hair: "goggles", outfit: "outfit-work-apron", item: "item-ink-brush", cloth: "slate-700" },
  },
  layers: {
    sky: [{ part: "stars", x: 50, y: 16, scale: 1.4 }],
    far: [
      { part: "ink-peaks", x: 66, y: 48, scale: 0.5, paint: "slate-800" },
      { part: "ink-round-window", x: 66, y: 38, scale: 1.2, paint: "slate-700" },
      { part: "moon", x: 72, y: 32, scale: 0.5 },
    ],
    mid: [
      { part: "boards", x: 50, y: 76 },
      { part: "pipes", x: 14, y: 60, scale: 0.6 },
      { part: "gear", x: 30, y: 36, scale: 0.7 },
      { part: "cog", x: 18, y: 44, scale: 0.8 },
      { part: "table", x: 58, y: 80, scale: 1.3, paint: "wood-700" },
      { part: "astrolabe", x: 74, y: 74, scale: 0.6 },
    ],
    subject: [
      { part: "card", x: 58, y: 70, scale: 1.7 },
      { part: "card-imp", x: 64, y: 50, scale: 1 },
      { part: "squares", x: 58, y: 60, scale: 0.7 },
      { part: "glitch", x: 58, y: 62, scale: 0.7 },
    ],
    near: [
      { part: "anvil", x: 84, y: 90, scale: 1 },
      { part: "taper", x: 44, y: 78, scale: 1 },
      { part: "ink-seal", x: 80, y: 74 },
    ],
    light: [
      { part: "halo", x: 58, y: 62, scale: 2.2, paint: "sky-300" },
      { part: "rim-light", x: 28, y: 62, scale: 1, paint: "sky-300" },
    ],
    particles: [
      { part: "sparks", x: 58, y: 56, scale: 1.4, count: 4, spread: [16, 12] },
      { part: "motes", x: 50, y: 50, scale: 1.2, count: 2, spread: [10, 6], paint: "cyan-400" },
      { part: "spore-motes", x: 66, y: 44, scale: 1, paint: "purple-400" },
    ],
  },
};

/** Expedition12 (Meditative #18, Epic Spell): a trekker's slim pack and an empty coin purse on a boulder, the trekker holding one card up into a sunbeam that turns it pearl-bright, a long snow pass winding away, thin crisp air. Changed: the boulder is a standing stone, the pack a parcel and the empty purse one copper coin left beside it. */
const EXPEDITION12: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "slate-100"],
  light: { x: 58, y: 22, r: 34, paint: "white", opacity: 0.6 },
  emblem: { x: 50, y: 54, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 84,
    scale: 0.95,
    pose: "pose-raise",
    look: { ...figureLookFor([]), hair: "hat-brim", outfit: "outfit-coat", item: "item-card", cloth: "blue-800" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 54, paint: "slate-400" },
      { part: "stairs", x: 62, y: 60, scale: 0.6 },
    ],
    mid: [
      { part: "snowfield", x: 50, y: 66 },
      { part: "pine", x: 14, y: 72, scale: 0.9, count: 2, spread: [6, 2] },
      { part: "pine", x: 86, y: 70, scale: 0.8 },
    ],
    near: [
      { part: "standing-stone", x: 20, y: 92, scale: 1.1, flip: -1 },
      { part: "parcel", x: 20, y: 76, scale: 0.6 },
      { part: "coin", x: 28, y: 84, scale: 0.5, paint: "brass-600" },
      { part: "ink-seal", x: 82, y: 72 },
    ],
    light: [
      { part: "light-shaft", x: 56, y: 8, scale: 1 },
      { part: "halo", x: 56, y: 34, scale: 1, paint: "white" },
    ],
    particles: [{ part: "snow", x: 50, y: 40, scale: 0.8 }],
  },
};

/** Expedition123 (Meditative #19, Epic Spell): a blank map unrolling on a camp stool, its lines drawing themselves in glowing ink, an exhausted explorer by a dying campfire, a red sandstone canyon where the horizon folds into a shimmering tear, dusk. Changed: the camp stool is a wooden chair. */
const EXPEDITION123: SceneRecipe = {
  family: "meditative",
  sky: ["violet-700", "orange-400"],
  light: { x: 50, y: 40, r: 30, paint: "violet-300", opacity: 0.5 },
  emblem: { x: 54, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 64,
    y: 82,
    scale: 0.85,
    flip: -1,
    pose: "pose-sit",
    look: { ...figureLookFor([]), hair: "bucket-hat", outfit: "outfit-coat", item: null, cloth: "wood-700" },
  },
  layers: {
    far: [
      { part: "mesa", x: 18, y: 58, scale: 0.9, paint: "red-900" },
      { part: "mesa", x: 84, y: 56, scale: 0.8, flip: -1, paint: "red-900" },
      { part: "rift", x: 50, y: 42, scale: 0.8 },
    ],
    mid: [{ part: "sand", x: 50, y: 64, paint: "orange-400" }],
    subject: [
      { part: "flames", x: 42, y: 84, scale: 1.5 },
      { part: "smoke", x: 42, y: 72, scale: 0.7 },
      { part: "embers", x: 42, y: 74, scale: 0.7 },
    ],
    near: [
      { part: "chair", x: 24, y: 96, scale: 0.9 },
      { part: "map", x: 24, y: 82, scale: 1.8 },
      { part: "heal-glow", x: 24, y: 82, scale: 0.8, paint: "cyan-400" },
      { part: "ink-seal", x: 82, y: 72 },
    ],
  },
};

/** Temporal Rift (Meditative #19's token, Token Spell): a tall shimmering tear folding open in the air above a still mountain lake, its reflected clouds drifting backwards, deep violet light. */
const TEMPORAL_RIFT: SceneRecipe = {
  family: "meditative",
  sky: ["violet-700", "slate-400"],
  light: { x: 50, y: 44, r: 34, paint: "violet-500", opacity: 0.5 },
  emblem: { x: 50, y: 44, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 28, y: 26, scale: 1 }],
    far: [{ part: "ink-peaks", x: 50, y: 60, scale: 0.8, paint: "slate-800" }],
    mid: [
      { part: "still-water", x: 50, y: 62 },
      { part: "cloud", x: 30, y: 74, scale: 0.8 },
      { part: "pine", x: 80, y: 63, scale: 0.9, count: 2, spread: [7, 1], paint: "slate-800" },
    ],
    subject: [{ part: "rift", x: 50, y: 44, scale: 1.3 }],
    near: [
      { part: "reed", x: 16, y: 88, scale: 1.2 },
      { part: "ink-seal", x: 80, y: 74 },
    ],
  },
};

/** Aestheticize the Game (Meditative #20, Mythic Spell): on a high painting terrace among ink-wash cliffs a robed artist sweeps a giant brush across the sky and the board below turns beautiful, one stroke raising a towering hero, a second filling a quiet garden of headstones with blossom, a third painting a proud army, colour bleeding out of grey ink, peach dawn, jade mist. Changed: the brushstroke is a jade ribbon of light, the hero a small armoured figure it raises. */
const AESTHETICIZE_THE_GAME: SceneRecipe = {
  family: "meditative",
  sky: ["red-300", "slate-100"],
  light: { x: 72, y: 26, r: 38, paint: "white", opacity: 0.5 },
  emblem: { x: 34, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 26,
    y: 86,
    scale: 0.85,
    pose: "pose-cast",
    look: { ...figureLookFor([]), hair: "hair-bun", outfit: "outfit-robe", item: "item-ink-brush", cloth: "slate-700" },
  },
  extras: [{ x: 56, y: 70, scale: 0.55, pose: "pose-raise", look: { ...figureLookFor(["Human"]), hair: "steel-helmet", outfit: "outfit-armor", cloth: "red-600" }, layer: "mid" }],
  layers: {
    sky: [
      { part: "sun", x: 78, y: 24, scale: 0.8 },
      { part: "ink-cloud", x: 26, y: 26, scale: 0.8 },
    ],
    far: [
      { part: "ink-peaks", x: 50, y: 52 },
      { part: "needle-peak", x: 12, y: 62, scale: 0.9 },
    ],
    mid: [
      { part: "mist", x: 50, y: 62, paint: "jade-400" },
      { part: "tombstone", x: 40, y: 72, scale: 0.6, count: 3, spread: [5, 2] },
      { part: "fruit-tree", x: 34, y: 70, scale: 0.4 },
      { part: "soldier", x: 76, y: 72, scale: 0.7, count: 4, spread: [8, 2] },
    ],
    subject: [{ part: "light-ribbon", x: 52, y: 44, scale: 1, paint: "jade-400" }],
    near: [
      { part: "ink-seal", x: 82, y: 76 },
    ],
    light: [
      { part: "light-shaft", x: 72, y: 20, scale: 1 },
      { part: "halo", x: 56, y: 56, scale: 1.4, paint: "white" },
    ],
    particles: [
      { part: "petals", x: 24, y: 62, scale: 0.8 },
      { part: "motes", x: 60, y: 40, scale: 1.2, paint: "jade-400" },
    ],
  },
};

/** API Key Fishing (Meditative #21, Rare Spell): a hooded angler on a fog-bound dock casting a line baited with a glowing key toward a rival's fan of cards across the grey water. */
const API_KEY_FISHING: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 70, y: 44, r: 30, paint: "white", opacity: 0.35 },
  emblem: { x: 34, y: 60, size: 5, rotate: 0 },
  figure: {
    x: 34,
    y: 84,
    scale: 1,
    pose: "pose-point",
    look: { ...figureLookFor([]), outfit: "outfit-robe", item: "item-rod", cloth: "slate-700" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 52, scale: 0.6, paint: "slate-400" },
      { part: "mist", x: 50, y: 56 },
    ],
    mid: [
      { part: "still-water", x: 50, y: 60 },
      { part: "ink-person", x: 78, y: 68, scale: 1.4 },
      { part: "ink-card-fan", x: 78, y: 58, scale: 0.5 },
    ],
    subject: [{ part: "boards", x: 30, y: 76 }],
    near: [
      { part: "halo", x: 66, y: 50, scale: 1, paint: "sky-300", props: true },
      { part: "key", x: 66, y: 50, scale: 0.9, props: true },
      { part: "mist", x: 60, y: 84, scale: 0.8 },
      { part: "ink-seal", x: 82, y: 74 },
    ],
  },
};

/** Mind Games (Meditative #22, Epic Spell): two players each hiding a hand behind a folding fan, three ghostly choices hovering over the table between them (a heap of mana, a fireball and a shield), a pagoda's top room with one hanging lamp. Changed: the players are large brush figures behind their fans and the heap of mana a cluster of violet crystals. */
const MIND_GAMES: SceneRecipe = {
  family: "meditative",
  sky: ["slate-800", "slate-700"],
  light: { x: 50, y: 30, r: 32, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 46, scale: 0.4 },
      { part: "ink-round-window", x: 50, y: 40, scale: 1.1, paint: "slate-700" },
      { part: "paper-screen", x: 18, y: 66, scale: 1.2 },
      { part: "paper-screen", x: 82, y: 66, scale: 1.2, flip: -1 },
    ],
    mid: [
      { part: "tatami", x: 50, y: 66 },
      { part: "table", x: 50, y: 80, scale: 1.3, paint: "wood-700" },
    ],
    subject: [
      { part: "crystal", x: 36, y: 62, scale: 0.8, count: 3, spread: [3, 1], paint: "violet-300" },
      { part: "fireball", x: 50, y: 52, scale: 0.8 },
      { part: "shield", x: 64, y: 58, scale: 0.7, paint: "violet-300" },
      { part: "halo", x: 50, y: 56, scale: 1.6, paint: "violet-300" },
    ],
    near: [
      { part: "ink-person", x: 14, y: 100, scale: 3 },
      { part: "ink-person", x: 86, y: 100, scale: 3, flip: -1 },
      { part: "folding-fan", x: 18, y: 76, scale: 1.2 },
      { part: "folding-fan", x: 82, y: 76, scale: 1.2, flip: -1 },
      { part: "paper-lantern", x: 50, y: 20, scale: 1 },
      { part: "ink-seal", x: 24, y: 58 },
    ],
  },
};

/** Fortify Mind (Meditative #22's token, Token Spell): a calm student meditating in a bare stone tower room, a translucent shield of thought forming around her head as she guesses, candlelight. */
const FORTIFY_MIND: SceneRecipe = {
  family: "meditative",
  sky: ["slate-800", "slate-700"],
  light: { x: 50, y: 52, r: 30, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 50, y: 66, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 82,
    scale: 1,
    pose: "pose-sit",
    look: { ...figureLookFor([]), hair: "hair-long", outfit: "outfit-robe", item: null, cloth: "blue-800" },
  },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 72, paint: "slate-700" },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 72 },
      { part: "candle", x: 30, y: 82, scale: 1 },
      { part: "candle", x: 70, y: 82, scale: 1 },
    ],
    subject: [{ part: "echo-rings", x: 50, y: 50, scale: 1, paint: "sky-300" }],
    near: [
      { part: "halo", x: 50, y: 52, scale: 1.4, paint: "sky-300" },
      { part: "ink-seal", x: 82, y: 74 },
    ],
    light: [{ part: "thought-shapes", x: 50, y: 50, scale: 0.8 }],
  },
};

/** Golly Bob Howdy (Meditative #23, Common Unit): a beaming farmer in overalls waving his straw hat at his neighbours, who puff into fluffy sheep on either side of him, sunny hillside pasture. */
const GOLLY_BOB_HOWDY: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "slate-100"],
  light: { x: 70, y: 22, r: 34, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 58, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 84,
    scale: 1,
    pose: "pose-raise",
    look: { ...figureLookFor(["Human"]), hair: "hat-brim", outfit: "outfit-overalls", item: null, cloth: "blue-600" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 50, scale: 0.5 },
      { part: "hills", x: 50, y: 58, paint: "jade-400" },
    ],
    mid: [
      { part: "field", x: 50, y: 70 },
      { part: "fence", x: 50, y: 72, scale: 0.8 },
    ],
    subject: [
      { part: "ink-sheep", x: 28, y: 84, scale: 1.2 },
      { part: "ink-sheep", x: 72, y: 84, scale: 1.2, flip: -1 },
      { part: "puff", x: 26, y: 70, scale: 0.6, paint: "white" },
      { part: "puff", x: 74, y: 70, scale: 0.6, paint: "white" },
    ],
    near: [
      { part: "flowers", x: 14, y: 90, scale: 1.2 },
      { part: "ink-seal", x: 84, y: 70 },
    ],
  },
};

/** Polymorph (Meditative #24, Common Spell): in a stone sheepfold an armoured warrior bursts into violet smoke and a bewildered sheep, an open spellbook sprouting wool on the wall beside it. Changed: the warrior is already gone, his dented helmet left in the violet smoke beside the sheep. */
const POLYMORPH: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 50, y: 50, r: 30, paint: "violet-300", opacity: 0.45 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "ink-peaks", x: 50, y: 56, scale: 0.6 }],
    mid: [
      { part: "field", x: 50, y: 70, paint: "stone-500" },
      { part: "stone-wall", x: 50, y: 72, scale: 1 },
    ],
    subject: [
      { part: "smoke-billow", x: 50, y: 58, scale: 1.8, paint: "violet-500" },
      { part: "ink-sheep", x: 50, y: 84, scale: 1.7 },
      { part: "helmet", x: 68, y: 86, scale: 1.2 },
    ],
    near: [
      { part: "tome", x: 22, y: 70, scale: 0.8 },
      { part: "puff", x: 22, y: 64, scale: 0.4, paint: "white" },
      { part: "sparks", x: 50, y: 56, scale: 1.2, paint: "violet-300" },
      { part: "ink-seal", x: 82, y: 74 },
    ],
  },
};

/** Blue-Eyes White Felinor (Meditative #25, Rare Unit): a regal white-furred cat-folk warrior with piercing blue eyes and a silver mantle, unbothered as thrown insults scatter off it like dry leaves, snowy ridge. */
const BLUE_EYES_WHITE_FELINOR: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 30, y: 26, r: 34, paint: "blue-200", opacity: 0.5 },
  emblem: { x: 50, y: 56, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 84,
    scale: 1.1,
    pose: "pose-guard",
    look: { ...figureLookFor(["Felinor"]), outfit: "outfit-robe", item: "item-sword", skin: "white", cloth: "slate-400" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 58, scale: 0.9, paint: "slate-400" },
      { part: "mist", x: 50, y: 62 },
    ],
    mid: [
      { part: "snowfield", x: 50, y: 72 },
      { part: "pine", x: 16, y: 76, scale: 0.9 },
    ],
    near: [
      { part: "leaves", x: 30, y: 50, scale: 1 },
      { part: "leaves", x: 70, y: 56, scale: 0.9 },
      { part: "ink-seal", x: 82, y: 72 },
    ],
    light: [{ part: "rim-light", x: 50, y: 60, scale: 1.1, paint: "blue-200" }],
    particles: [{ part: "snow", x: 50, y: 44, scale: 0.9 }],
  },
};

/** Alternate Fate (Meditative #26, Legendary Field Spell): a cracked, dry plateau where every old well has run empty; at dusk new stone wells shove up out of the earth one by one in a ring, each brimming with blue mana light that spills over the parched rocks, while two weary travellers stare and violet clouds part overhead. */
const ALTERNATE_FATE: SceneRecipe = {
  family: "meditative",
  sky: ["violet-700", "red-300"],
  light: { x: 50, y: 64, r: 40, paint: "sky-300", opacity: 0.4 },
  emblem: { x: 50, y: 70, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "ink-cloud", x: 24, y: 24, scale: 0.9, paint: "violet-300" },
      { part: "ink-cloud", x: 78, y: 22, scale: 0.8, paint: "violet-300" },
    ],
    far: [{ part: "ink-peaks", x: 50, y: 52, scale: 0.6, paint: "slate-700" }],
    mid: [
      { part: "cracked-earth", x: 50, y: 58 },
      { part: "well", x: 12, y: 64, scale: 0.5 },
      { part: "stone-well", x: 34, y: 66, scale: 0.7, paint: "sky-400" },
      { part: "stone-well", x: 66, y: 66, scale: 0.7, flip: -1, paint: "sky-400" },
    ],
    subject: [
      { part: "stone-well", x: 50, y: 76, scale: 1.1, paint: "sky-400" },
      { part: "stone-well", x: 26, y: 80, scale: 0.9, paint: "sky-400" },
      { part: "stone-well", x: 74, y: 80, scale: 0.9, flip: -1, paint: "sky-400" },
    ],
    near: [
      { part: "ink-person", x: 12, y: 90, scale: 1.8, count: 2, spread: [4, 1] },
      { part: "pebbles", x: 50, y: 92, scale: 1.4 },
      { part: "dust", x: 70, y: 90, scale: 1 },
      { part: "ink-seal", x: 84, y: 72 },
    ],
    light: [
      { part: "light-shaft", x: 50, y: 14, scale: 1, paint: "violet-300" },
      { part: "halo", x: 50, y: 66, scale: 1.6, paint: "sky-300" },
      { part: "halo", x: 26, y: 72, scale: 1, paint: "sky-300" },
      { part: "halo", x: 74, y: 72, scale: 1, paint: "sky-300" },
    ],
    particles: [{ part: "motes", x: 50, y: 56, scale: 1.4, paint: "sky-300", count: 2, spread: [14, 4] }],
  },
};

/** Clip-Farming Lawyer (Meditative #27, Common Unit): a slick lawyer in a pinstripe suit filming himself on a phone as he unlocks a courthouse gate, a copper coin flipping from his fingers. Changed: the courthouse is a temple front, its gate opened beside him with the key at its foot. */
const CLIP_FARMING_LAWYER: SceneRecipe = {
  family: "meditative",
  sky: ["slate-100", "slate-400"],
  light: { x: 50, y: 30, r: 36, paint: "white", opacity: 0.45 },
  emblem: { x: 48, y: 58, size: 5, rotate: 0 },
  figure: {
    x: 48,
    y: 86,
    scale: 1,
    pose: "pose-point",
    look: { ...figureLookFor(["Human"]), outfit: "outfit-stripes", item: "item-phone", cloth: "slate-800" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 50, scale: 0.4 },
      { part: "temple", x: 50, y: 66, scale: 1.2 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 70 },
      { part: "gate", x: 72, y: 80, scale: 0.8 },
    ],
    near: [
      { part: "key", x: 66, y: 72, scale: 0.4 },
      { part: "coin", x: 38, y: 58, scale: 0.7, paint: "brass-600" },
      { part: "ink-seal", x: 18, y: 72 },
    ],
  },
};

/** Shade-iris (Meditative #28, Common Unit): a shadowy violet iris creature with a petal hood and root legs, slipping a cursed seed between the cards of a rival's deck, twilight garden. */
const SHADE_IRIS: SceneRecipe = {
  family: "meditative",
  sky: ["violet-700", "slate-400"],
  light: { x: 50, y: 50, r: 30, paint: "violet-300", opacity: 0.4 },
  emblem: { x: 46, y: 64, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "moon", x: 76, y: 24, scale: 0.6 }],
    far: [{ part: "ink-peaks", x: 50, y: 58, scale: 0.7, paint: "slate-700" }],
    mid: [
      { part: "field", x: 50, y: 70, paint: "slate-700" },
      { part: "flowers", x: 18, y: 76, scale: 1.2, count: 2, spread: [6, 2] },
      { part: "stone-lantern", x: 84, y: 76, scale: 1 },
    ],
    subject: [
      { part: "iris-sprite", x: 46, y: 84, scale: 1.8 },
      { part: "deck", x: 64, y: 84, scale: 1.8 },
    ],
    near: [
      { part: "spore-motes", x: 58, y: 70, scale: 0.6, paint: "violet-300" },
      { part: "grass-tuft", x: 26, y: 92, scale: 1 },
      { part: "ink-seal", x: 82, y: 70 },
    ],
  },
};

/** Ancient Curse (Meditative #28's token, Token Spell): a cracked hand mirror leaking black smoke over a scattered deck on a cold stone shrine floor, grey dawn. */
const ANCIENT_CURSE: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 30, y: 30, r: 34, paint: "white", opacity: 0.35 },
  emblem: { x: 48, y: 72, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "altar", x: 50, y: 60, scale: 1.2 },
      { part: "stone-lantern", x: 20, y: 62, scale: 1.1 },
      { part: "stone-lantern", x: 80, y: 62, scale: 1.1 },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 62 }],
    subject: [
      { part: "playing-card", x: 62, y: 80, scale: 0.8, count: 4, spread: [12, 4] },
      { part: "hand-mirror", x: 42, y: 86, scale: 1.6 },
    ],
    near: [
      { part: "smoke", x: 46, y: 68, scale: 1.5, paint: "slate-900" },
      { part: "smoke-billow", x: 54, y: 60, scale: 1.7, paint: "slate-900" },
      { part: "ink-seal", x: 82, y: 74 },
    ],
  },
};

/** Forbiddenous Factory (Meditative #29, Rare Field Spell): a soot-black factory wedged in a mountain valley, chimneys venting green plague smoke, crates of cursed cards rolling off a conveyor into waiting carts. */
const FORBIDDENOUS_FACTORY: SceneRecipe = {
  family: "meditative",
  sky: ["slate-700", "slate-400"],
  light: { x: 50, y: 32, r: 30, paint: "lime-400", opacity: 0.3 },
  emblem: { x: 46, y: 66, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "ink-peaks", x: 50, y: 58, scale: 0.9 }],
    mid: [
      { part: "rock-floor", x: 50, y: 74 },
      { part: "smoke", x: 36, y: 40, scale: 1.1, paint: "green-400" },
      { part: "smoke", x: 52, y: 38, scale: 1, paint: "lime-400" },
    ],
    subject: [
      { part: "factory", x: 46, y: 80, scale: 1.1 },
      { part: "cart", x: 78, y: 84, scale: 1 },
      { part: "crate", x: 76, y: 74, scale: 0.6 },
    ],
    near: [
      { part: "crate", x: 20, y: 90, scale: 0.9 },
      { part: "playing-card", x: 30, y: 86, scale: 0.6, count: 2, spread: [4, 2] },
      { part: "ink-seal", x: 18, y: 66 },
    ],
    particles: [{ part: "spore-motes", x: 50, y: 50, scale: 1.2, paint: "green-400" }],
  },
};

/** Fickle E-Kitten (Meditative #30, Epic Unit): a sleek young cat-folk gamer in cat-ear headphones glancing over her shoulder at a shinier stranger, her current partner's outstretched hands, a crowded internet café of glowing screens, flirty pink light. */
const FICKLE_E_KITTEN: SceneRecipe = {
  family: "meditative",
  sky: ["violet-700", "red-300"],
  light: { x: 60, y: 44, r: 34, paint: "purple-400", opacity: 0.45 },
  emblem: { x: 48, y: 58, size: 5, rotate: 0 },
  figure: {
    x: 48,
    y: 86,
    scale: 1,
    flip: -1,
    pose: "pose-stand",
    look: { ...figureLookFor(["Felinor"]), hair: "headphones", outfit: "outfit-hoodie", item: null, cloth: "purple-400" },
  },
  extras: [{ x: 76, y: 80, scale: 0.8, pose: "pose-point", look: { ...figureLookFor(["Felinor"]), outfit: "outfit-jacket", item: null, cloth: "sky-400" }, layer: "mid" }],
  layers: {
    far: [
      { part: "monitors", x: 30, y: 56, scale: 1 },
      { part: "monitors", x: 72, y: 54, scale: 0.9 },
      { part: "ink-person", x: 50, y: 58, scale: 0.9, count: 4, spread: [36, 1] },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66 },
      { part: "hand-open", x: 24, y: 70, scale: 1 },
      { part: "hand-open", x: 30, y: 74, scale: 1 },
    ],
    near: [
      { part: "ink-card-fan", x: 20, y: 86, scale: 0.6 },
      { part: "ink-seal", x: 18, y: 66 },
    ],
    particles: [
      { part: "sparks", x: 76, y: 52, scale: 1.2, count: 2, spread: [6, 6], paint: "red-300" },
      { part: "motes", x: 50, y: 40, scale: 1, paint: "red-300" },
    ],
  },
};

/** Love Bomb (Meditative #30's token, Token Spell): a pink heart-shaped bomb with a lit fuse and a lipstick kiss, bursting into healing petals over a park bench. */
const LOVE_BOMB: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "slate-100"],
  light: { x: 50, y: 46, r: 30, paint: "red-300", opacity: 0.4 },
  emblem: { x: 50, y: 50, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 58, scale: 0.5 },
      { part: "tree", x: 18, y: 66, scale: 0.9 },
      { part: "tree", x: 84, y: 66, scale: 0.8 },
    ],
    mid: [
      { part: "field", x: 50, y: 70 },
      { part: "bench", x: 50, y: 82, scale: 1.1 },
    ],
    subject: [
      { part: "burst", x: 50, y: 50, scale: 1.4, paint: "red-300" },
      { part: "heal-glow", x: 50, y: 50, scale: 1.4, paint: "red-300" },
      { part: "heart-bomb", x: 50, y: 50, scale: 1.5 },
    ],
    near: [
      { part: "petals", x: 50, y: 58, scale: 1.2, count: 2, spread: [10, 6], paint: "red-300" },
      { part: "ink-seal", x: 82, y: 74 },
    ],
  },
};

export const MEDITATIVE_RECIPES_2: Readonly<Record<string, SceneRecipe>> = {
  "meditative-015": SMELLY_STEVEN,
  "meditative-016": TRENFUL_TRICKSTER,
  "meditative-017": TRUE_CRAFT_A_CARD,
  "meditative-018": EXPEDITION12,
  "meditative-019": EXPEDITION123,
  "meditative-019-1": TEMPORAL_RIFT,
  "meditative-020": AESTHETICIZE_THE_GAME,
  "meditative-021": API_KEY_FISHING,
  "meditative-022": MIND_GAMES,
  "meditative-022-1": FORTIFY_MIND,
  "meditative-023": GOLLY_BOB_HOWDY,
  "meditative-024": POLYMORPH,
  "meditative-025": BLUE_EYES_WHITE_FELINOR,
  "meditative-026": ALTERNATE_FATE,
  "meditative-027": CLIP_FARMING_LAWYER,
  "meditative-028": SHADE_IRIS,
  "meditative-028-1": ANCIENT_CURSE,
  "meditative-029": FORBIDDENOUS_FACTORY,
  "meditative-030": FICKLE_E_KITTEN,
  "meditative-030-1": LOVE_BOMB,
};
