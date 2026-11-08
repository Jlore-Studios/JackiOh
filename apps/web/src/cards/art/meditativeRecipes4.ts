// R3401000: Meditative's scenes, part 4 of 5: Meditative #50 to #79 with #70's and #71's tokens, each drawn from its
// brief in the issue's table (#3401) in Meditative's family, the ink wash (ART.md, "Meditative and Meditative+" and
// "Meditative drawn as scenes"): slate-900 ink on slate-100 paper, jade washes, a high and distant camera, and one red
// seal in the near layer. Each recipe keeps its subject within x 30 to 70 and y 25 to 85 and its emblem small on its
// subject; a Unit's character is a figure built on its tags' look, or its part of the scene kit in its subject layer.
// Each doc comment gives the brief in a line and any change made to fit the kits.

import { figureLookFor } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** CN Tech (Meditative #50, Rare Spell): a sleek glowing terminal in a bamboo study, plucking a single card out of a whirlwind of cards with a thin beam of light. */
const CN_TECH: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 50, y: 50, r: 30, paint: "cyan-400", opacity: 0.3 },
  emblem: { x: 50, y: 63, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 50, scale: 0.6 },
      { part: "bamboo", x: 10, y: 74, scale: 1.4 },
      { part: "bamboo", x: 90, y: 74, scale: 1.3, flip: -1 },
    ],
    mid: [
      { part: "tatami", x: 50, y: 70 },
      { part: "paper-screen", x: 22, y: 72, scale: 1.1 },
      { part: "paper-screen", x: 78, y: 72, scale: 1.1, flip: -1 },
    ],
    subject: [
      { part: "desk", x: 50, y: 80, scale: 1.8 },
      { part: "terminal", x: 50, y: 72, scale: 1.1 },
      { part: "card-ring", x: 50, y: 38, scale: 0.9 },
      { part: "playing-card", x: 50, y: 44, scale: 0.7 },
    ],
    near: [
      { part: "bamboo", x: 92, y: 96, scale: 1.2, flip: -1 },
      { part: "ink-seal", x: 20, y: 74 },
    ],
    light: [{ part: "light-shaft", x: 50, y: 48, scale: 0.35, paint: "cyan-400" }],
  },
};

/**
 * Devin Bot (Meditative #51, Legendary Unit): a small round robot sitting cross-legged on a meditation cushion in a
 * sunlit dojo, eyes closed, as dozens of tiny robots, critters and creatures pop into being in a ring around it and
 * spill across the floor mats, morning light through paper screens throwing their small shadows long.
 */
const DEVIN_BOT: SceneRecipe = {
  family: "meditative",
  sky: ["slate-100", "white"],
  light: { x: 50, y: 40, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 66, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "mist", x: 50, y: 30, scale: 0.8 }],
    far: [
      { part: "paper-screen", x: 18, y: 62, scale: 1.7 },
      { part: "paper-screen", x: 82, y: 62, scale: 1.7, flip: -1 },
    ],
    mid: [
      { part: "tatami", x: 50, y: 62 },
      { part: "ink-imp", x: 30, y: 70, scale: 0.8 },
      { part: "beetle", x: 70, y: 70, scale: 1, count: 2, spread: [5, 2] },
    ],
    subject: [
      { part: "round-bot", x: 50, y: 78, scale: 2 },
      { part: "card-imp", x: 34, y: 66, scale: 0.6 },
      { part: "card-imp", x: 66, y: 64, scale: 0.6 },
      { part: "bunny", x: 32, y: 82, scale: 0.45 },
      { part: "round-bot", x: 68, y: 82, scale: 0.5 },
    ],
    near: [
      { part: "ink-imp", x: 24, y: 88, scale: 1.1 },
      { part: "beetle", x: 60, y: 90, scale: 1.4, flip: -1 },
      { part: "round-bot", x: 40, y: 90, scale: 0.6 },
      { part: "ink-seal", x: 82, y: 74 },
    ],
    light: [
      { part: "light-shaft", x: 32, y: 20, scale: 1.1 },
      { part: "light-shaft", x: 68, y: 20, scale: 1.1 },
      { part: "halo", x: 50, y: 64, scale: 0.9, paint: "jade-400" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 62, scale: 1.4, count: 3, spread: [18, 8] },
      { part: "motes", x: 50, y: 50, scale: 1 },
    ],
  },
};

/** Economic Anxiety (Meditative #52, Rare Field Spell): a crumbling stock-exchange courtyard with a sacrificial altar at its centre, worried traders in suits sizing up rivals, storm-dark sky. */
const ECONOMIC_ANXIETY: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "slate-700"],
  light: { x: 50, y: 52, r: 30, paint: "red-600", opacity: 0.25 },
  emblem: { x: 50, y: 70, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "cloud", x: 30, y: 22, scale: 1.2, paint: "slate-800" },
      { part: "lightning", x: 76, y: 18, scale: 0.8 },
    ],
    far: [{ part: "temple", x: 50, y: 60, scale: 1.3, paint: "slate-400" }],
    mid: [
      { part: "cobbles", x: 50, y: 64 },
      { part: "column", x: 14, y: 70, scale: 1 },
      { part: "fallen-column", x: 80, y: 74, scale: 0.8 },
      { part: "monitors", x: 50, y: 58, scale: 0.6 },
    ],
    subject: [
      { part: "altar", x: 50, y: 76, scale: 1.5 },
      { part: "ink-person", x: 34, y: 76, scale: 1.3 },
      { part: "ink-person", x: 66, y: 76, scale: 1.3, flip: -1 },
      { part: "ink-person", x: 26, y: 72, scale: 1 },
    ],
    near: [
      { part: "coins", x: 40, y: 88, scale: 0.9 },
      { part: "ink-seal", x: 80, y: 82 },
    ],
  },
};

/** Prestige (Meditative #53, Common Spell): a stage magician's hand whisking the shine off one card like a silk cloth and draping it over another, deep red velvet curtain backstage. */
const PRESTIGE: SceneRecipe = {
  family: "meditative",
  sky: ["red-900", "slate-900"],
  light: { x: 50, y: 46, r: 32, paint: "red-400", opacity: 0.35 },
  emblem: { x: 60, y: 60, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "banner", x: 14, y: 84, scale: 2.2, paint: "red-600", count: 2, spread: [6, 0] },
      { part: "banner", x: 86, y: 84, scale: 2.2, paint: "red-600", flip: -1, count: 2, spread: [6, 0] },
    ],
    mid: [{ part: "stage", x: 50, y: 72 }],
    subject: [
      { part: "playing-card", x: 40, y: 66, scale: 1.4 },
      { part: "playing-card", x: 60, y: 66, scale: 1.4 },
      { part: "light-ribbon", x: 50, y: 52, scale: 0.6, paint: "slate-100" },
      { part: "hand-hold", x: 50, y: 46, scale: 1.2 },
    ],
    near: [
      { part: "sparks", x: 60, y: 56, scale: 1 },
      { part: "ink-seal", x: 80, y: 76 },
    ],
  },
};

/** Money Machine (Meditative #54, Common Field Spell): a clanking copper coin press in a market square spitting coins to both sides of a counter, its owner quietly pocketing a few extra. */
const MONEY_MACHINE: SceneRecipe = {
  family: "meditative",
  sky: ["slate-100", "blue-200"],
  light: { x: 40, y: 30, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 62, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 50, scale: 0.7 },
      { part: "tiled-house", x: 16, y: 58, scale: 1 },
      { part: "tiled-house", x: 84, y: 58, scale: 0.9, flip: -1 },
    ],
    mid: [
      { part: "cobbles", x: 50, y: 62 },
      { part: "awning", x: 18, y: 70, scale: 0.9 },
    ],
    subject: [
      { part: "counter", x: 50, y: 80, scale: 1, paint: "wood-700" },
      { part: "coin-press", x: 50, y: 70, scale: 1.2, paint: "brass-600" },
      { part: "coins", x: 32, y: 70, scale: 0.6 },
      { part: "coins", x: 68, y: 70, scale: 0.6 },
      { part: "ink-person", x: 64, y: 70, scale: 1.2, flip: -1 },
    ],
    near: [
      { part: "coin", x: 40, y: 84, scale: 1, count: 2, spread: [12, 3] },
      { part: "ink-seal", x: 80, y: 76 },
    ],
  },
};

/** Dragon Fruit (Meditative #55, Rare Spell): a smug pink dragon fruit with tiny dragon frills, split open on a temple altar, a shining card glowing among its speckled flesh. */
const DRAGON_FRUIT: SceneRecipe = {
  family: "meditative",
  sky: ["slate-700", "slate-400"],
  light: { x: 50, y: 56, r: 30, paint: "purple-400", opacity: 0.35 },
  emblem: { x: 50, y: 56, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 54, scale: 0.7 },
      { part: "pagoda", x: 50, y: 66, scale: 1.2 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 70 },
      { part: "red-gate", x: 50, y: 72, scale: 2 },
      { part: "paper-lantern", x: 18, y: 24, scale: 1.4 },
      { part: "paper-lantern", x: 82, y: 24, scale: 1.4 },
    ],
    subject: [
      { part: "altar", x: 50, y: 84, scale: 2.4 },
      { part: "dragon-fruit", x: 50, y: 64, scale: 2, paint: "red-300" },
    ],
    near: [
      { part: "incense", x: 22, y: 88, scale: 1 },
      { part: "incense", x: 78, y: 88, scale: 1, flip: -1 },
      { part: "ink-seal", x: 80, y: 70 },
    ],
    light: [{ part: "halo", x: 50, y: 56, scale: 0.6, paint: "purple-400" }],
  },
};

/** House Party (Meditative #56, Common Spell): a sturdy timber house overflowing with cheering armoured defenders crowding its porch and roof, lanterns strung up, an evening party in a hill village. */
const HOUSE_PARTY: SceneRecipe = {
  family: "meditative",
  sky: ["violet-700", "red-300"],
  light: { x: 50, y: 50, r: 34, paint: "red-300", opacity: 0.35 },
  emblem: { x: 50, y: 66, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 56, scale: 0.8 },
      { part: "tiled-house", x: 14, y: 62, scale: 0.6 },
      { part: "tiled-house", x: 86, y: 62, scale: 0.6, flip: -1 },
    ],
    mid: [{ part: "hills", x: 50, y: 74, paint: "green-900" }],
    subject: [
      { part: "tiled-house", x: 50, y: 82, scale: 2.3, paint: "wood-800" },
      { part: "soldier", x: 34, y: 84, scale: 0.75, count: 3, spread: [4, 1] },
      { part: "soldier", x: 66, y: 84, scale: 0.75, flip: -1, count: 3, spread: [4, 1] },
      { part: "soldier", x: 50, y: 50, scale: 0.6, count: 3, spread: [12, 1] },
    ],
    near: [
      { part: "lantern-string", x: 50, y: 42, scale: 1.3 },
      { part: "ink-seal", x: 82, y: 78 },
    ],
    particles: [{ part: "confetti", x: 50, y: 44, scale: 1 }],
  },
};

/** Clip-Farming Critikal (Meditative #57, Common Unit): a hyped young streamer in a headset rewinding a glowing card back into his hand, ring-light glare in a cluttered bedroom. */
const CLIP_FARMING_CRITIKAL: SceneRecipe = {
  family: "meditative",
  sky: ["slate-700", "slate-400"],
  light: { x: 50, y: 46, r: 30, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 64, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 1,
    pose: "pose-point",
    look: { ...figureLookFor(["Human"]), hair: "headphones", outfit: "outfit-hoodie", item: "item-card", cloth: "violet-700" },
  },
  layers: {
    far: [{ part: "room-wall", x: 50, y: 70, paint: "slate-400" }],
    mid: [
      { part: "boards", x: 50, y: 70 },
      { part: "bed", x: 18, y: 78, scale: 0.9 },
      { part: "monitors", x: 80, y: 74, scale: 0.8 },
    ],
    subject: [
      { part: "shelf", x: 20, y: 62, scale: 0.7 },
      { part: "streaks", x: 64, y: 44, scale: 0.5, flip: -1, paint: "cyan-400" },
    ],
    near: [
      { part: "parcel", x: 84, y: 92, scale: 0.8 },
      { part: "ink-seal", x: 82, y: 58 },
    ],
    light: [{ part: "halo", x: 50, y: 46, scale: 1.2, paint: "white" }],
  },
};

/** Permanent Underclassman (Meditative #58, Common Unit): a sheepish freshman in a beanie with an armful of textbooks, leading a tiny summoned creature across a campus quad, autumn leaves falling. */
const PERMANENT_UNDERCLASSMAN: SceneRecipe = {
  family: "meditative",
  sky: ["slate-100", "orange-400"],
  light: { x: 30, y: 30, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 64, size: 5, rotate: 0 },
  figure: {
    x: 48,
    y: 85,
    scale: 1,
    pose: "pose-hold",
    look: { ...figureLookFor(["Human"]), hair: "beanie", outfit: "outfit-sweater", item: "item-book", cloth: "orange-400" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 50, scale: 0.6 },
      { part: "campus-hall", x: 50, y: 62, scale: 1.2 },
    ],
    mid: [
      { part: "field", x: 50, y: 66, paint: "green-700" },
      { part: "tree", x: 12, y: 74, scale: 1.1, paint: "orange-400" },
      { part: "tree", x: 88, y: 72, scale: 1, paint: "red-600" },
    ],
    subject: [{ part: "card-imp", x: 68, y: 78, scale: 1 }],
    near: [
      { part: "leaves", x: 30, y: 50, scale: 1.1, count: 2, spread: [16, 10] },
      { part: "ink-seal", x: 80, y: 76 },
    ],
  },
};

/** Permanent Upperclassman (Meditative #59, Rare Unit): a swaggering senior in a faded team jacket and graduation sash, a hulking summoned beast at his heels, on stadium bleachers at sunset. */
const PERMANENT_UPPERCLASSMAN: SceneRecipe = {
  family: "meditative",
  sky: ["violet-700", "orange-400"],
  light: { x: 50, y: 56, r: 36, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 44, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 44,
    y: 86,
    scale: 1.05,
    pose: "pose-point",
    look: { ...figureLookFor(["Human"]), hair: "cap-feather", outfit: "outfit-bomber", item: null, cloth: "blue-800" },
  },
  layers: {
    sky: [{ part: "sun", x: 76, y: 46, scale: 1.1 }],
    far: [
      { part: "ink-peaks", x: 50, y: 56, scale: 0.6 },
      { part: "stadium", x: 50, y: 70, scale: 0.8 },
    ],
    mid: [{ part: "amphitheatre", x: 50, y: 82, scale: 1.2 }],
    subject: [{ part: "beast", x: 68, y: 87, scale: 2.1, flip: -1, paint: "slate-800" }],
    near: [
      { part: "banner", x: 14, y: 92, scale: 1.2, paint: "blue-800" },
      { part: "ink-seal", x: 82, y: 74 },
    ],
    particles: [{ part: "confetti", x: 50, y: 34, scale: 0.9 }],
  },
};

/**
 * Eschews (Meditative #60, Epic Field Spell): an abandoned desk buried in unopened parcels and dodged chores; books,
 * robots and scroll-spirits flying out of an open drawer on their own; a deserted mountain resort lobby, late afternoon dust.
 */
const ESCHEWS: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "orange-400"],
  light: { x: 70, y: 36, r: 34, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 50, y: 70, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 64, paint: "slate-400" },
      { part: "column", x: 14, y: 66, scale: 1 },
      { part: "column", x: 86, y: 66, scale: 1 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 64 },
      { part: "deckchair", x: 22, y: 72, scale: 0.8 },
      { part: "closed-tome", x: 38, y: 54, scale: 1.1 },
      { part: "scroll", x: 64, y: 50, scale: 0.9 },
      { part: "round-bot", x: 56, y: 42, scale: 0.9 },
      { part: "ghost", x: 36, y: 40, scale: 1 },
    ],
    subject: [
      { part: "desk", x: 50, y: 84, scale: 3 },
      { part: "parcel", x: 40, y: 66, scale: 0.8 },
      { part: "parcel", x: 60, y: 66, scale: 0.7, flip: -1 },
      { part: "envelope-drift", x: 50, y: 64, scale: 0.8 },
    ],
    near: [
      { part: "parcel", x: 18, y: 94, scale: 1.2 },
      { part: "envelope-drift", x: 72, y: 92, scale: 1 },
      { part: "ink-seal", x: 82, y: 74 },
    ],
    light: [{ part: "light-shaft", x: 72, y: 20, scale: 1 }],
    particles: [{ part: "motes", x: 60, y: 52, scale: 1.2 }],
  },
};

/** Joon Juicer (Meditative #61, Common Unit): a grinning street juicer squeezing glowing fruit into his neighbour's cup at a riverside juice stall, the neighbour swelling with strength, sunny. */
const JOON_JUICER: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "slate-100"],
  light: { x: 30, y: 26, r: 36, paint: "white", opacity: 0.5 },
  emblem: { x: 40, y: 64, size: 5, rotate: 0 },
  figure: {
    x: 40,
    y: 86,
    scale: 1,
    pose: "pose-hold",
    look: { ...figureLookFor(["Human"]), hair: "kerchief", outfit: "outfit-work-apron", item: "item-flask", cloth: "green-700" },
  },
  extras: [
    {
      layer: "subject",
      x: 66,
      y: 86,
      scale: 0.9,
      flip: -1,
      pose: "pose-flex",
      look: { ...figureLookFor(["Human"]), hair: "hair-short", outfit: "outfit-muscle", item: null, cloth: "blue-700" },
    },
  ],
  layers: {
    sky: [{ part: "sun", x: 80, y: 24, scale: 0.8 }],
    far: [
      { part: "ink-peaks", x: 50, y: 52, scale: 0.6 },
      { part: "still-water", x: 50, y: 60 },
    ],
    mid: [
      { part: "boards", x: 50, y: 72 },
      { part: "awning", x: 20, y: 80, scale: 1.2 },
    ],
    near: [
      { part: "apple", x: 30, y: 92, scale: 1.2, count: 3, spread: [6, 1] },
      { part: "ink-seal", x: 82, y: 72 },
    ],
    light: [{ part: "heal-glow", x: 66, y: 66, scale: 0.8, paint: "jade-400" }],
  },
};

/** Small Time Recruits (Meditative #62, Rare Spell): a tiny eager recruit in oversized armour stepping out of a deck of cards, clutching a dumpling, at a dusty roadside recruiting tent. Changed: the dumpling is a plate of rice cakes by his deck, the kit having no dumpling. */
const SMALL_TIME_RECRUITS: SceneRecipe = {
  family: "meditative",
  sky: ["slate-100", "orange-400"],
  light: { x: 40, y: 30, r: 34, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 66, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 74,
    scale: 0.6,
    pose: "pose-stride",
    look: { head: "head-human", hair: "steel-helmet", outfit: "outfit-layered-plate", item: "item-spear", skin: "red-300", cloth: "slate-400" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 54, scale: 0.7 },
      { part: "tent", x: 76, y: 66, scale: 1.4 },
      { part: "banner", x: 90, y: 68, scale: 1, paint: "red-600" },
    ],
    mid: [
      { part: "road", x: 50, y: 60 },
      { part: "dust", x: 30, y: 70, scale: 1 },
    ],
    subject: [
      { part: "deck", x: 50, y: 82, scale: 2.4 },
      { part: "playing-card", x: 36, y: 72, scale: 0.8 },
      { part: "playing-card", x: 64, y: 70, scale: 0.8 },
      { part: "rice-cake", x: 62, y: 76, scale: 0.4 },
    ],
    near: [
      { part: "dust", x: 70, y: 90, scale: 1.2 },
      { part: "ink-seal", x: 18, y: 74 },
    ],
    particles: [{ part: "sparks", x: 50, y: 70, scale: 1.2 }],
  },
};

/**
 * Skull of J'Nari (Meditative #63, Legendary Field Spell): a cave shrine high in the mountains where a huge horned
 * skull on an altar chatters mid-sentence; violet spirit-fire pours from its eye sockets and hauls a warrior up through
 * the floor stones, candles and offerings scattered, cold light falling through a crack in the ceiling.
 */
const SKULL_OF_JNARI: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 46, r: 34, paint: "violet-500", opacity: 0.4 },
  emblem: { x: 50, y: 68, size: 5, rotate: 0 },
  extras: [
    {
      layer: "subject",
      x: 74,
      y: 94,
      scale: 0.8,
      flip: -1,
      pose: "pose-raise",
      look: { head: "head-human", hair: "helm", outfit: "outfit-armor", item: "item-sword", skin: "red-300", cloth: "violet-300" },
    },
  ],
  layers: {
    sky: [{ part: "mist", x: 50, y: 30, scale: 0.8 }],
    far: [
      { part: "ink-peaks", x: 50, y: 50, scale: 0.6 },
      { part: "cave", x: 50, y: 58, scale: 1.1 },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 70 },
      { part: "candle", x: 26, y: 72, scale: 1, count: 2, spread: [4, 1] },
      { part: "candle", x: 74, y: 72, scale: 1, count: 2, spread: [4, 1] },
    ],
    subject: [
      { part: "altar", x: 46, y: 80, scale: 2.2 },
      { part: "horned-skull", x: 46, y: 62, scale: 1.4 },
      { part: "flames", x: 42, y: 54, scale: 0.5, paint: "purple-400" },
      { part: "flames", x: 50, y: 54, scale: 0.5, paint: "purple-400" },
      { part: "light-ribbon", x: 62, y: 58, scale: 0.45, paint: "violet-300" },
    ],
    near: [
      { part: "pebbles", x: 72, y: 84, scale: 1.2 },
      { part: "bones", x: 22, y: 90, scale: 1 },
      { part: "candle", x: 80, y: 92, scale: 1.4 },
      { part: "ink-seal", x: 16, y: 72 },
    ],
    light: [
      { part: "light-shaft", x: 58, y: 8, scale: 0.9, paint: "blue-200" },
      { part: "halo", x: 46, y: 54, scale: 0.8, paint: "purple-400" },
    ],
    particles: [
      { part: "embers", x: 52, y: 48, scale: 1.4, count: 2, spread: [10, 6], paint: "violet-300" },
      { part: "spore-motes", x: 64, y: 70, scale: 1, paint: "violet-300" },
    ],
  },
};

/** Traitorous Blood (Meditative #64, Common Trap): on a muddy battlefield ridge, a charging soldier's spear swerves mid-thrust into his own comrade beside him, yanked by a taut red blood-thread. */
const TRAITOROUS_BLOOD: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "red-300"],
  light: { x: 50, y: 40, r: 34, paint: "red-300", opacity: 0.3 },
  emblem: { x: 60, y: 64, size: 5, rotate: 0 },
  figure: {
    x: 60,
    y: 84,
    scale: 0.95,
    flip: -1,
    pose: "pose-lunge",
    look: { head: "head-human", hair: "steel-helmet", outfit: "outfit-armor", item: "item-spear", skin: "red-300", cloth: "slate-700" },
  },
  extras: [
    {
      layer: "subject",
      x: 34,
      y: 84,
      scale: 0.9,
      pose: "pose-block",
      look: { head: "head-human", hair: "steel-helmet", outfit: "outfit-armor", item: null, skin: "red-300", cloth: "slate-400" },
    },
  ],
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 56, scale: 0.6 },
      { part: "soldier", x: 20, y: 62, scale: 0.5, count: 3, spread: [8, 1] },
      { part: "banner", x: 84, y: 64, scale: 0.9, paint: "red-600" },
    ],
    mid: [
      { part: "hills", x: 50, y: 70, paint: "wood-700" },
      { part: "dust", x: 70, y: 82, scale: 1 },
    ],
    subject: [{ part: "red-thread", x: 34, y: 60, scale: 1.1 }],
    near: [
      { part: "drops", x: 38, y: 66, scale: 0.9, paint: "red-600" },
      { part: "ink-seal", x: 82, y: 74 },
    ],
  },
};

/**
 * Keymaster Keenus (Meditative #65, Legendary Unit): a wiry old gatekeeper in a patched robe hung with iron keys strides
 * down a cliff monastery's hall of endless unlocked doors; mid-step he snaps one glowing key onto a warrior's armour like
 * a charm, and the warrior's blade flares with power, the candlelit corridor stretching away.
 */
const KEYMASTER_KEENUS: SceneRecipe = {
  family: "meditative",
  sky: ["slate-700", "slate-400"],
  light: { x: 50, y: 44, r: 30, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 42, y: 64, size: 5, rotate: 0 },
  figure: {
    x: 42,
    y: 86,
    scale: 1,
    pose: "pose-stride",
    look: { ...figureLookFor([]), hair: "hair-bun", outfit: "outfit-patched", item: "item-key", cloth: "wood-700" },
  },
  extras: [
    {
      layer: "subject",
      x: 68,
      y: 86,
      scale: 0.95,
      flip: -1,
      pose: "pose-guard",
      look: { head: "head-human", hair: "hair-short", outfit: "outfit-armor", item: "item-sword", skin: "red-300", cloth: "slate-400" },
    },
  ],
  layers: {
    sky: [{ part: "mist", x: 50, y: 30, scale: 0.6 }],
    far: [
      { part: "room-wall", x: 50, y: 56, paint: "slate-700" },
      { part: "doorway", x: 44, y: 56, scale: 0.6 },
      { part: "doorway", x: 56, y: 56, scale: 0.6, flip: -1 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 56 },
      { part: "doorway", x: 34, y: 62, scale: 1.1 },
      { part: "doorway", x: 66, y: 62, scale: 1.1, flip: -1 },
      { part: "doorway", x: 18, y: 70, scale: 1.7 },
      { part: "doorway", x: 82, y: 70, scale: 1.7, flip: -1 },
      { part: "candle", x: 26, y: 66, scale: 0.8 },
      { part: "candle", x: 74, y: 66, scale: 0.8 },
    ],
    near: [
      { part: "candle", x: 12, y: 92, scale: 1.4 },
      { part: "candle", x: 88, y: 92, scale: 1.4 },
      { part: "ink-seal", x: 82, y: 78 },
    ],
    light: [
      { part: "halo", x: 58, y: 56, scale: 0.7, paint: "jade-400" },
      { part: "light-shaft", x: 66, y: 22, scale: 0.6, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 62, y: 52, scale: 1.2, count: 2, spread: [6, 6] },
      { part: "motes", x: 50, y: 60, scale: 0.9 },
    ],
  },
};

/** Fiery Waraxe (Meditative #66, Common Field Spell): a flaming battle axe wrenching itself out of a log beside a mountain campfire, swinging on its own, embers swirling at night. */
const FIERY_WARAXE: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "blue-900"],
  light: { x: 54, y: 60, r: 30, paint: "ember-500", opacity: 0.35 },
  emblem: { x: 52, y: 70, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "moon", x: 78, y: 24, scale: 0.7 },
      { part: "stars", x: 34, y: 26, scale: 1 },
    ],
    far: [{ part: "ink-peaks", x: 50, y: 58, scale: 0.8 }],
    mid: [
      { part: "pine", x: 14, y: 74, scale: 1.4, paint: "slate-800" },
      { part: "pine", x: 86, y: 72, scale: 1.2, paint: "slate-800" },
    ],
    subject: [
      { part: "stump", x: 48, y: 82, scale: 2.2 },
      { part: "axe", x: 46, y: 76, scale: 1.3 },
      { part: "flames", x: 60, y: 52, scale: 0.8 },
      { part: "flames", x: 72, y: 84, scale: 1.2 },
    ],
    near: [
      { part: "embers", x: 62, y: 50, scale: 1.4, count: 2, spread: [8, 8] },
      { part: "ink-seal", x: 20, y: 74 },
    ],
  },
};

/** Sentient Cat Ears (Meditative #67, Epic Unit): a pair of fluffy living cat ears leaping onto a startled warrior's helmet, little pink hearts drifting off them, a cosy hat shop lined with shelves of hats, warm afternoon light. */
const SENTIENT_CAT_EARS: SceneRecipe = {
  family: "meditative",
  sky: ["orange-400", "slate-100"],
  light: { x: 50, y: 34, r: 36, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 50, y: 64, size: 5, rotate: 0 },
  extras: [
    {
      layer: "mid",
      x: 50,
      y: 88,
      scale: 1,
      pose: "pose-spread",
      look: { head: "head-human", hair: "steel-helmet", outfit: "outfit-tabard", item: null, skin: "red-300", cloth: "blue-800" },
    },
  ],
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 70, paint: "wood-700" },
      { part: "shelf", x: 18, y: 60, scale: 1 },
      { part: "shelf", x: 82, y: 60, scale: 1 },
    ],
    mid: [
      { part: "boards", x: 50, y: 70 },
      { part: "counter", x: 82, y: 82, scale: 0.6, paint: "wood-700" },
      { part: "hat", x: 18, y: 66, scale: 1, count: 2, spread: [6, 2] },
    ],
    subject: [{ part: "cat-ears", x: 50, y: 32, scale: 1.4 }],
    near: [
      { part: "hat", x: 82, y: 90, scale: 1.4 },
      { part: "ink-hearts", x: 62, y: 30, scale: 0.9 },
      { part: "ink-seal", x: 18, y: 76 },
    ],
  },
};

/** Catnip (Meditative #68, Rare Field Trap): a catnip bush bursting up through a garden courtyard's paving, green pollen exploding as nearby cat-folk go wild-eyed and pounce, late afternoon. */
const CATNIP: SceneRecipe = {
  family: "meditative",
  sky: ["orange-400", "slate-100"],
  light: { x: 50, y: 50, r: 34, paint: "lime-200", opacity: 0.35 },
  emblem: { x: 50, y: 70, size: 5, rotate: 0 },
  figure: {
    x: 26,
    y: 86,
    scale: 0.85,
    pose: "pose-lunge",
    look: { ...figureLookFor(["Felinor"]), item: null, cloth: "jade-400" },
  },
  extras: [
    {
      layer: "subject",
      x: 76,
      y: 86,
      scale: 0.85,
      flip: -1,
      pose: "pose-strike",
      look: { ...figureLookFor(["Felinor"]), item: null, skin: "slate-400", cloth: "red-600" },
    },
  ],
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 52, scale: 0.6 },
      { part: "tiled-house", x: 50, y: 62, scale: 1.6 },
    ],
    mid: [
      { part: "cobbles", x: 50, y: 64 },
      { part: "stone-lantern", x: 14, y: 70, scale: 1 },
    ],
    subject: [
      { part: "bush", x: 50, y: 84, scale: 2.4, paint: "green-500" },
      { part: "rock-floor", x: 50, y: 90 },
    ],
    near: [
      { part: "cat", x: 64, y: 92, scale: 1.2, flip: -1 },
      { part: "ink-seal", x: 84, y: 66 },
    ],
    particles: [
      { part: "spores", x: 50, y: 56, scale: 1.6, paint: "lime-400" },
      { part: "spore-motes", x: 40, y: 64, scale: 1.2, paint: "green-400" },
    ],
  },
};

/** The Maestro (Meditative #69, Rare Unit): a gaunt conductor in a tailcoat raising his baton before a choir coughing green plague mist, in a candlelit opera hall. */
const THE_MAESTRO: SceneRecipe = {
  family: "meditative",
  sky: ["red-900", "slate-900"],
  light: { x: 40, y: 50, r: 34, paint: "orange-400", opacity: 0.3 },
  emblem: { x: 62, y: 64, size: 5, rotate: 0 },
  figure: {
    x: 62,
    y: 86,
    scale: 1.05,
    flip: -1,
    pose: "pose-raise",
    look: { ...figureLookFor(["Human"]), hair: "hair-long", outfit: "outfit-tailcoat", item: "item-baton", cloth: "slate-900" },
  },
  layers: {
    far: [
      { part: "banner", x: 10, y: 84, scale: 2.2, paint: "red-600" },
      { part: "banner", x: 90, y: 84, scale: 2.2, paint: "red-600", flip: -1 },
      { part: "chandelier", x: 40, y: 20, scale: 1.1 },
    ],
    mid: [
      { part: "stage", x: 50, y: 70 },
      { part: "ink-person", x: 32, y: 68, scale: 1.2, count: 4, spread: [12, 1] },
      { part: "ink-person", x: 28, y: 62, scale: 1, count: 3, spread: [12, 1] },
      { part: "mist", x: 30, y: 60, scale: 0.6, paint: "green-400" },
    ],
    near: [
      { part: "candle", x: 14, y: 90, scale: 1.4, count: 2, spread: [4, 1] },
      { part: "ink-seal", x: 82, y: 76 },
    ],
    particles: [{ part: "spore-motes", x: 30, y: 56, scale: 1.2, paint: "green-400" }],
  },
};

/** I'M WILL BE YOUR DOOM (Meditative #70, Common Unit): a hulking horned doom-knight looming in a ruined watchtower under a red sky, while a clumsy little minion stumbles off toward the enemy. */
const IM_WILL_BE_YOUR_DOOM: SceneRecipe = {
  family: "meditative",
  sky: ["red-900", "red-600"],
  light: { x: 50, y: 40, r: 34, paint: "red-400", opacity: 0.3 },
  emblem: { x: 46, y: 64, size: 5, rotate: 0 },
  figure: {
    x: 46,
    y: 86,
    scale: 1.2,
    pose: "pose-guard",
    look: { ...figureLookFor([]), head: "head-skull", hair: "horns", outfit: "outfit-plate", item: "item-sword", cloth: "slate-900" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 60, scale: 0.7, paint: "slate-900" },
      { part: "tower", x: 18, y: 72, scale: 1.2, paint: "slate-700" },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 74 },
      { part: "ruin-wall", x: 82, y: 80, scale: 1 },
    ],
    subject: [{ part: "goblin", x: 74, y: 78, scale: 1.5 }],
    near: [
      { part: "pebbles", x: 30, y: 90, scale: 1 },
      { part: "ink-seal", x: 82, y: 66 },
    ],
  },
};

/** Ready… I'm (Meditative #70's token, Token Unit): a tiny bleary-eyed goblin half-dressed in armour, yawning in a muddy trench at dawn, a dripping poisoned dagger in hand. */
const READY_IM: SceneRecipe = {
  family: "meditative",
  sky: ["red-300", "slate-100"],
  light: { x: 70, y: 40, r: 34, paint: "red-300", opacity: 0.35 },
  emblem: { x: 50, y: 66, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 0.85,
    pose: "pose-stand",
    look: { ...figureLookFor([]), head: "head-goblin", hair: null, outfit: "outfit-armor", item: "item-dagger", skin: "green-500", cloth: "wood-700" },
  },
  layers: {
    sky: [{ part: "sun", x: 74, y: 46, scale: 0.8 }],
    far: [{ part: "ink-peaks", x: 50, y: 58, scale: 0.6 }],
    mid: [
      { part: "field", x: 50, y: 66, paint: "wood-800" },
      { part: "stone-wall", x: 50, y: 72, scale: 1.2 },
      { part: "fence-post", x: 20, y: 76, scale: 1 },
    ],
    near: [
      { part: "drops", x: 60, y: 70, scale: 0.6, paint: "green-400" },
      { part: "ink-seal", x: 80, y: 74 },
    ],
  },
};

/**
 * Pareto Optimality (Meditative #71, Mythic Field Spell): a stern study hall in a mountain academy, rows of practice desks
 * under a vast ink-wash scroll of one sweeping curve; at the front an elder's bamboo cane hangs alone in mid-air over the
 * lectern, and as one student lays down a poor card it whips down the aisle in a streak of white light, every other
 * student frozen mid-breath, cold blue dawn through lattice windows.
 */
const PARETO_OPTIMALITY: SceneRecipe = {
  family: "meditative",
  sky: ["blue-900", "slate-700"],
  light: { x: 50, y: 34, r: 36, paint: "blue-200", opacity: 0.35 },
  emblem: { x: 56, y: 58, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "mist", x: 50, y: 22, scale: 0.7 }],
    far: [
      { part: "paper-screen", x: 10, y: 58, scale: 1.4 },
      { part: "paper-screen", x: 26, y: 58, scale: 1.4 },
      { part: "paper-screen", x: 74, y: 58, scale: 1.4, flip: -1 },
      { part: "paper-screen", x: 90, y: 58, scale: 1.4, flip: -1 },
      { part: "bell-curve", x: 50, y: 48, scale: 1.1, paint: "slate-100" },
    ],
    mid: [
      { part: "boards", x: 50, y: 58 },
      { part: "lectern", x: 50, y: 62, scale: 1.1 },
      { part: "desk", x: 30, y: 66, scale: 1, count: 2, spread: [6, 0] },
      { part: "desk", x: 70, y: 66, scale: 1, count: 2, spread: [6, 0] },
      { part: "ink-person", x: 30, y: 64, scale: 0.9, count: 2, spread: [6, 0] },
      { part: "ink-person", x: 70, y: 64, scale: 0.9, count: 2, spread: [6, 0] },
    ],
    subject: [
      { part: "desk", x: 24, y: 76, scale: 1.4 },
      { part: "desk", x: 76, y: 76, scale: 1.4 },
      { part: "ink-person", x: 24, y: 74, scale: 1.2 },
      { part: "ink-person", x: 76, y: 74, scale: 1.2, flip: -1 },
      { part: "cane", x: 52, y: 64, scale: 0.9 },
      { part: "streaks", x: 58, y: 64, scale: 0.8, flip: -1, paint: "white" },
      { part: "playing-card", x: 70, y: 70, scale: 0.6 },
    ],
    near: [
      { part: "desk", x: 14, y: 94, scale: 2.2 },
      { part: "desk", x: 86, y: 94, scale: 2.2 },
      { part: "zap", x: 46, y: 70, scale: 0.4, flip: -1 },
      { part: "ink-seal", x: 50, y: 82 },
    ],
    light: [
      { part: "light-shaft", x: 12, y: 30, scale: 0.9, paint: "blue-200" },
      { part: "light-shaft", x: 88, y: 30, scale: 0.9, paint: "blue-200" },
      { part: "halo", x: 54, y: 62, scale: 0.7, paint: "white" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.2, paint: "blue-200" },
      { part: "sparks", x: 54, y: 62, scale: 1, count: 2, spread: [6, 4] },
    ],
  },
};

/** The Cane (Meditative #71's token, Token Field Spell): a lacquered bamboo cane hovering by itself above a school desk, wrapped in red cord, crackling with sharp white energy. */
const THE_CANE: SceneRecipe = {
  family: "meditative",
  sky: ["slate-700", "slate-400"],
  light: { x: 50, y: 48, r: 30, paint: "white", opacity: 0.35 },
  emblem: { x: 50, y: 76, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 66, paint: "slate-400" },
      { part: "blackboard", x: 26, y: 60, scale: 1 },
    ],
    mid: [{ part: "boards", x: 50, y: 66 }],
    subject: [
      { part: "desk", x: 50, y: 86, scale: 2.6 },
      { part: "cane", x: 52, y: 54, scale: 1 },
      { part: "zap", x: 38, y: 46, scale: 0.4, flip: -1 },
      { part: "zap", x: 62, y: 54, scale: 0.4 },
    ],
    near: [
      { part: "sparks", x: 50, y: 50, scale: 1.4, count: 2, spread: [10, 4] },
      { part: "ink-seal", x: 82, y: 76 },
    ],
  },
};

/** The Banisher (Meditative #72, Common Unit): a faceless hooded figure with a void-black blade in a ruined moonlit temple, a struck foe unravelling into a portal behind it. */
const THE_BANISHER: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "blue-900"],
  light: { x: 70, y: 56, r: 30, paint: "violet-500", opacity: 0.35 },
  emblem: { x: 42, y: 64, size: 5, rotate: 0 },
  figure: {
    x: 42,
    y: 86,
    scale: 1,
    pose: "pose-strike",
    look: { ...figureLookFor([]), head: "head-void", outfit: "outfit-robe", item: "item-sword", skin: "black", cloth: "slate-800" },
  },
  layers: {
    sky: [
      { part: "moon", x: 24, y: 24, scale: 0.8 },
      { part: "stars", x: 60, y: 22, scale: 1 },
    ],
    far: [
      { part: "ink-peaks", x: 50, y: 58, scale: 0.7, paint: "slate-800" },
      { part: "column", x: 86, y: 72, scale: 1 },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 74 },
      { part: "ruin-wall", x: 14, y: 80, scale: 1 },
      { part: "portal", x: 70, y: 80, scale: 1.1 },
      { part: "ghost-figure", x: 70, y: 78, scale: 0.9, paint: "violet-300" },
    ],
    near: [
      { part: "fallen-column", x: 76, y: 92, scale: 0.8 },
      { part: "ink-seal", x: 20, y: 70 },
    ],
  },
};

/** Plate Packer (Meditative #73, Common Unit): a burly weightlifter in a temple courtyard heaving a barbell stacked with armour plates, monks watching wide-eyed, morning sun. */
const PLATE_PACKER: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "slate-100"],
  light: { x: 70, y: 30, r: 36, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 1,
    pose: "pose-strike",
    look: { ...figureLookFor(["Human"]), hair: "headband", outfit: "outfit-muscle", item: null, cloth: "red-600" },
  },
  layers: {
    sky: [{ part: "sun", x: 82, y: 24, scale: 0.8 }],
    far: [
      { part: "ink-peaks", x: 50, y: 54, scale: 0.6 },
      { part: "pagoda", x: 16, y: 66, scale: 0.9 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 68 },
      { part: "ink-person", x: 76, y: 74, scale: 1.3, count: 3, spread: [8, 1], flip: -1 },
      { part: "ink-person", x: 24, y: 74, scale: 1.3, count: 2, spread: [6, 1] },
    ],
    subject: [{ part: "barbell", x: 50, y: 28, scale: 1.1, paint: "slate-700" }],
    near: [
      { part: "incense", x: 84, y: 90, scale: 1 },
      { part: "ink-seal", x: 20, y: 76 },
    ],
  },
};

/** Montaña Giant (Meditative #74, Epic Unit): a drift of sealed, unanswered envelopes at a traveller's feet; a giant of rock and pine rising out of the mountainside, growing taller; its vast shadow falling across a valley at sunset. */
const MONTANA_GIANT: SceneRecipe = {
  family: "meditative",
  sky: ["violet-700", "orange-400"],
  light: { x: 76, y: 50, r: 36, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 50, y: 60, size: 6, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 80, y: 52, scale: 1 }],
    far: [
      { part: "ink-peaks", x: 50, y: 60, scale: 0.9 },
      { part: "mist", x: 50, y: 62, scale: 1 },
    ],
    mid: [
      { part: "mountains", x: 50, y: 76, paint: "slate-700" },
      { part: "mist", x: 30, y: 80, scale: 0.9, paint: "slate-900" },
    ],
    subject: [
      { part: "rock-giant", x: 50, y: 82, scale: 1.2 },
      { part: "pine", x: 26, y: 84, scale: 0.8, paint: "slate-800" },
    ],
    near: [
      { part: "ink-person", x: 22, y: 90, scale: 1.4 },
      { part: "envelope-drift", x: 30, y: 92, scale: 1.2 },
      { part: "envelope-drift", x: 66, y: 94, scale: 1 },
      { part: "ink-seal", x: 82, y: 78 },
    ],
  },
};

/**
 * Ever Growing Tree (Meditative #75, Legendary Field Spell): a colossal tree on a mountaintop swallowing everything near
 * it: a teahouse, a waterfall and a stone shrine caught in its roots and branches, half absorbed into the bark, as a new
 * root coils around a ruined arch; leaves glow green against a stormy sky.
 */
const EVER_GROWING_TREE: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "slate-700"],
  light: { x: 50, y: 44, r: 40, paint: "jade-400", opacity: 0.35 },
  emblem: { x: 50, y: 70, size: 6, rotate: 0 },
  layers: {
    sky: [
      { part: "ink-cloud", x: 26, y: 20, scale: 1 },
      { part: "ink-cloud", x: 78, y: 24, scale: 0.9 },
      { part: "lightning", x: 84, y: 18, scale: 0.7 },
    ],
    far: [{ part: "ink-peaks", x: 50, y: 74, scale: 0.8 }],
    mid: [
      { part: "needle-peak", x: 50, y: 98, scale: 1.4 },
      { part: "rock-floor", x: 50, y: 84 },
      { part: "waterfall", x: 22, y: 82, scale: 0.8 },
    ],
    subject: [
      { part: "great-tree", x: 50, y: 84, scale: 0.95 },
      { part: "tiled-house", x: 68, y: 48, scale: 0.55 },
      { part: "stone-lantern", x: 36, y: 82, scale: 0.9 },
      { part: "arch", x: 76, y: 84, scale: 0.7 },
      { part: "vine", x: 76, y: 84, scale: 0.8, paint: "wood-800" },
    ],
    near: [
      { part: "pebbles", x: 30, y: 92, scale: 1 },
      { part: "ink-seal", x: 16, y: 72 },
    ],
    light: [
      { part: "halo", x: 50, y: 40, scale: 1.6, paint: "jade-400" },
      { part: "light-shaft", x: 50, y: 18, scale: 0.8, paint: "lime-200" },
    ],
    particles: [
      { part: "leaves", x: 50, y: 40, scale: 1.2, count: 2, spread: [18, 8] },
      { part: "motes", x: 50, y: 50, scale: 1.2, paint: "green-200" },
    ],
  },
};

/** Do or Die (Meditative #76, Common Spell): a pickpocket tying a crimson thread from her finger to one card in a rival's hand, aboard a crowded river ferry, dice rattling nearby. */
const DO_OR_DIE: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 50, y: 40, r: 34, paint: "white", opacity: 0.4 },
  emblem: { x: 66, y: 56, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 54, scale: 0.7 },
      { part: "still-water", x: 50, y: 60 },
    ],
    mid: [
      { part: "boat", x: 50, y: 76, scale: 2.6, paint: "slate-100" },
      { part: "ink-person", x: 30, y: 70, scale: 1.2, count: 3, spread: [8, 1] },
      { part: "ink-person", x: 70, y: 70, scale: 1.2, flip: -1, count: 2, spread: [6, 1] },
    ],
    subject: [
      { part: "ink-card-fan", x: 66, y: 66, scale: 0.9 },
      { part: "red-thread", x: 36, y: 64, scale: 0.9 },
      { part: "hand-open", x: 34, y: 68, scale: 1.1 },
    ],
    near: [
      { part: "dice", x: 50, y: 86, scale: 1.3 },
      { part: "ink-seal", x: 82, y: 78 },
    ],
  },
};

/** Bulk Booster (Meditative #77, Common Spell): a torn sack of booster packs spilling an avalanche of cards over a startled merchant in a warehouse, dust motes in sunbeams. */
const BULK_BOOSTER: SceneRecipe = {
  family: "meditative",
  sky: ["slate-700", "slate-400"],
  light: { x: 30, y: 30, r: 34, paint: "white", opacity: 0.35 },
  emblem: { x: 42, y: 70, size: 5, rotate: 0 },
  figure: {
    x: 70,
    y: 86,
    scale: 0.85,
    flip: -1,
    pose: "pose-spread",
    look: { head: "head-human", hair: "fedora", outfit: "outfit-waistcoat", item: null, skin: "red-300", cloth: "brass-800" },
  },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 64, paint: "wood-700" },
      { part: "crate", x: 14, y: 68, scale: 1.2, count: 2, spread: [3, 0] },
      { part: "crate", x: 86, y: 68, scale: 1.1 },
    ],
    mid: [{ part: "boards", x: 50, y: 64 }],
    subject: [
      { part: "booster-sack", x: 40, y: 82, scale: 2 },
      { part: "playing-card", x: 52, y: 60, scale: 0.8, count: 4, spread: [10, 8] },
      { part: "playing-card", x: 56, y: 80, scale: 0.8, count: 3, spread: [10, 3] },
    ],
    near: [
      { part: "deck", x: 24, y: 90, scale: 1.4 },
      { part: "ink-seal", x: 84, y: 66 },
    ],
    light: [{ part: "light-shaft", x: 30, y: 18, scale: 0.9 }],
    particles: [{ part: "motes", x: 34, y: 50, scale: 1.1 }],
  },
};

/**
 * Occidentless Mandate (Meditative #78, Epic Spell): an official's red seal slamming down on a scroll; foreign knights,
 * cowboys and robots fading into wind-blown ash across a palace square; a vermilion palace gate, banners snapping, harsh
 * noon. The scroll's seal is the picture's one red seal.
 */
const OCCIDENTLESS_MANDATE: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "slate-100"],
  light: { x: 50, y: 22, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 54, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 50, y: 20, scale: 0.8 }],
    far: [
      { part: "ink-peaks", x: 50, y: 52, scale: 0.6 },
      { part: "red-gate", x: 50, y: 64, scale: 2.2 },
      { part: "banner", x: 16, y: 66, scale: 1.2, paint: "red-600" },
      { part: "banner", x: 84, y: 66, scale: 1.2, paint: "red-600", flip: -1 },
    ],
    mid: [
      { part: "cobbles", x: 50, y: 64 },
      { part: "ghost-figure", x: 30, y: 74, scale: 0.7, paint: "slate-400" },
      { part: "soldier", x: 66, y: 72, scale: 0.8, flip: -1 },
      { part: "round-bot", x: 44, y: 72, scale: 0.7 },
    ],
    subject: [
      { part: "ghost-figure", x: 56, y: 72, scale: 0.7, paint: "brass-600" },
      { part: "gust", x: 50, y: 64, scale: 1.4 },
      { part: "dust", x: 40, y: 70, scale: 1.2, paint: "slate-400" },
    ],
    near: [
      { part: "scroll", x: 50, y: 86, scale: 2.4 },
      { part: "ink-seal", x: 56, y: 82 },
      { part: "hand-hold", x: 58, y: 76, scale: 1.4 },
    ],
  },
};

/**
 * Touched by KY (Meditative #79, Legendary Unit): a wide-eyed student in a chalk-dusted classroom at night, a ghostly
 * chalk-light hand on his head; his skull opens like a box lid and glowing circles and triangles spill out, his fan of
 * cards growing wider than his arms can hold, the blackboard behind him alive with chalk shapes. Changed: his opening
 * skull is drawn as the shapes pouring up from his head, the figure kit's heads having no lid.
 */
const TOUCHED_BY_KY: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "blue-900"],
  light: { x: 50, y: 32, r: 34, paint: "cyan-400", opacity: 0.3 },
  emblem: { x: 50, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 1,
    pose: "pose-hold",
    look: { ...figureLookFor(["KY"]), hair: "hair-short", outfit: "outfit-blazer", item: "item-card", cloth: "blue-800" },
  },
  layers: {
    sky: [{ part: "stars", x: 80, y: 24, scale: 0.6 }],
    far: [
      { part: "room-wall", x: 50, y: 66, paint: "slate-800" },
      { part: "blackboard", x: 50, y: 56, scale: 1.6 },
    ],
    mid: [
      { part: "boards", x: 50, y: 66 },
      { part: "desk", x: 18, y: 76, scale: 1.4 },
      { part: "desk", x: 82, y: 76, scale: 1.4 },
    ],
    subject: [{ part: "thought-shapes", x: 50, y: 26, scale: 0.9 }],
    near: [
      { part: "ink-card-fan", x: 50, y: 70, scale: 1.4 },
      { part: "open-hand", x: 38, y: 24, scale: 0.9, paint: "slate-100" },
      { part: "ink-seal", x: 82, y: 64 },
    ],
    light: [
      { part: "halo", x: 50, y: 30, scale: 1.1, paint: "cyan-400" },
      { part: "light-shaft", x: 50, y: 12, scale: 0.5, paint: "slate-100" },
    ],
    particles: [
      { part: "motes", x: 50, y: 40, scale: 1, paint: "slate-100" },
      { part: "sparks", x: 50, y: 30, scale: 1.2, count: 2, spread: [12, 6] },
    ],
  },
};

export const MEDITATIVE_RECIPES_4: Readonly<Record<string, SceneRecipe>> = {
  "meditative-050": CN_TECH,
  "meditative-051": DEVIN_BOT,
  "meditative-052": ECONOMIC_ANXIETY,
  "meditative-053": PRESTIGE,
  "meditative-054": MONEY_MACHINE,
  "meditative-055": DRAGON_FRUIT,
  "meditative-056": HOUSE_PARTY,
  "meditative-057": CLIP_FARMING_CRITIKAL,
  "meditative-058": PERMANENT_UNDERCLASSMAN,
  "meditative-059": PERMANENT_UPPERCLASSMAN,
  "meditative-060": ESCHEWS,
  "meditative-061": JOON_JUICER,
  "meditative-062": SMALL_TIME_RECRUITS,
  "meditative-063": SKULL_OF_JNARI,
  "meditative-064": TRAITOROUS_BLOOD,
  "meditative-065": KEYMASTER_KEENUS,
  "meditative-066": FIERY_WARAXE,
  "meditative-067": SENTIENT_CAT_EARS,
  "meditative-068": CATNIP,
  "meditative-069": THE_MAESTRO,
  "meditative-070": IM_WILL_BE_YOUR_DOOM,
  "meditative-070-1": READY_IM,
  "meditative-071": PARETO_OPTIMALITY,
  "meditative-071-1": THE_CANE,
  "meditative-072": THE_BANISHER,
  "meditative-073": PLATE_PACKER,
  "meditative-074": MONTANA_GIANT,
  "meditative-075": EVER_GROWING_TREE,
  "meditative-076": DO_OR_DIE,
  "meditative-077": BULK_BOOSTER,
  "meditative-078": OCCIDENTLESS_MANDATE,
  "meditative-079": TOUCHED_BY_KY,
};
