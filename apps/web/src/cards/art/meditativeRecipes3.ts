// R3401000: Meditative's scenes, part 3 of 5: Meditative #31 to #49 with #39's, #45's and #49's tokens, each drawn
// from its brief in the issue's table (#3401) in Meditative's family, the ink wash (ART.md, "Meditative and
// Meditative+" and "Meditative drawn as scenes"): slate-900 ink on slate-100 paper, jade washes, a high and distant
// camera, and one red seal in the near layer. Each recipe keeps its subject within x 30 to 70 and y 25 to 85 and its
// emblem small on its subject; a Unit's character is a figure built on its tags' look, or its part of the scene kit
// in its subject layer. Each doc comment gives the brief in a line and any change made to fit the kits.

import { figureLookFor } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** The Conductor (Meditative #31, Rare Unit): a beaming train conductor in a navy cap waving aboard a crowd of conjured, glowing passengers at a misty mountain railway platform. */
const THE_CONDUCTOR: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 30, y: 30, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 58, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 85,
    scale: 0.95,
    pose: "pose-point",
    look: { ...figureLookFor(["Human"]), hair: "conductor-cap", outfit: "outfit-coat", item: null, cloth: "blue-900" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 50, scale: 0.9 },
      { part: "mist", x: 40, y: 52, scale: 0.9 },
    ],
    mid: [
      { part: "train", x: 50, y: 70, scale: 1.3, paint: "red-900" },
      { part: "station", x: 50, y: 82, scale: 1.4 },
    ],
    subject: [
      { part: "ghost-figure", x: 30, y: 80, scale: 0.7 },
      { part: "ghost-figure", x: 22, y: 79, scale: 0.6 },
      { part: "ghost-figure", x: 70, y: 80, scale: 0.65 },
    ],
    near: [
      { part: "mist", x: 70, y: 88, scale: 0.8 },
      { part: "ink-seal", x: 80, y: 72 },
    ],
  },
};

/** Spiritually 中国 (Meditative #32, Epic Spell): a startled traveller's hand of cards all turning into the same red-edged card, lion dancers and scholars stepping out of drifting ink, a river town of tiled roofs and pagodas hung with festival lanterns, bright morning. */
const SPIRITUALLY_CN: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "slate-100"],
  light: { x: 70, y: 24, r: 34, paint: "white", opacity: 0.5 },
  emblem: { x: 48, y: 66, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 74, y: 24, scale: 0.6 }],
    far: [
      { part: "ink-peaks", x: 50, y: 46, scale: 0.7 },
      { part: "pagoda", x: 22, y: 54, scale: 0.75 },
      { part: "tiled-house", x: 42, y: 56, scale: 0.6 },
      { part: "pagoda", x: 82, y: 56, scale: 0.6 },
      { part: "lantern-string", x: 50, y: 44, scale: 0.9 },
      { part: "water", x: 50, y: 58 },
    ],
    mid: [
      { part: "cobbles", x: 50, y: 66 },
      { part: "ink-cloud", x: 58, y: 74, scale: 0.8 },
    ],
    subject: [
      { part: "lion-dancer", x: 46, y: 76, scale: 1.1 },
      { part: "ink-person", x: 64, y: 74, scale: 1.2, flip: -1 },
      { part: "ink-person", x: 70, y: 73, scale: 1.1, flip: -1 },
    ],
    near: [
      { part: "hand-open", x: 26, y: 86, scale: 1.3 },
      { part: "ink-card-fan", x: 26, y: 82, scale: 0.7, paint: "red-300" },
      { part: "ink-seal", x: 82, y: 76 },
    ],
  },
};

/** First Day of 学校 (Meditative #33, Rare Spell): a nervous child with a brand-new backpack at a red school gate under plum blossom, clutching a fresh brush and pencil case, spring morning. */
const FIRST_DAY_OF_SCHOOL: SceneRecipe = {
  family: "meditative",
  sky: ["blue-200", "slate-100"],
  light: { x: 30, y: 26, r: 36, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 70, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 0.75,
    pose: "pose-hold",
    look: { ...figureLookFor(["Human"]), hair: "hair-short", outfit: "outfit-tunic", item: "item-ink-brush", cloth: "blue-600" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 52, scale: 0.7 },
      { part: "tiled-house", x: 50, y: 62, scale: 0.9 },
    ],
    mid: [
      { part: "cobbles", x: 50, y: 66 },
      { part: "red-gate", x: 50, y: 70, scale: 1.5 },
    ],
    near: [
      { part: "blossom-branch", x: 2, y: 30, scale: 1.3 },
      { part: "blossom-branch", x: 98, y: 38, scale: 1, flip: -1 },
      { part: "petals", x: 62, y: 40, scale: 1, paint: "red-300" },
      { part: "ink-seal", x: 80, y: 74 },
    ],
  },
};

/** 高考 (Meditative #34, Rare Spell): a vast exam hall where a few students stay seated and glowing as the other desks crumble to ash, tense pale light through tall windows. */
const GAOKAO: SceneRecipe = {
  family: "meditative",
  sky: ["slate-800", "slate-700"],
  light: { x: 50, y: 40, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 52, paint: "slate-700" },
      { part: "window", x: 20, y: 46, scale: 1.2 },
      { part: "window", x: 80, y: 46, scale: 1.2 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 52 },
      { part: "desk", x: 26, y: 60, scale: 0.9, count: 3, spread: [12, 1] },
      { part: "dust", x: 26, y: 58, scale: 0.8, count: 2, spread: [10, 1], paint: "slate-700" },
      { part: "desk", x: 74, y: 60, scale: 0.9, count: 2, spread: [10, 1] },
      { part: "ink-person", x: 72, y: 58, scale: 1 },
      { part: "halo", x: 72, y: 56, scale: 0.6, paint: "white" },
      { part: "embers", x: 30, y: 54, scale: 1, paint: "slate-700" },
    ],
    subject: [
      { part: "halo", x: 50, y: 70, scale: 1.1, paint: "white" },
      { part: "ink-person", x: 50, y: 72, scale: 1.5 },
      { part: "desk", x: 50, y: 78, scale: 1.4 },
    ],
    near: [
      { part: "desk", x: 18, y: 90, scale: 1.6 },
      { part: "dust", x: 18, y: 84, scale: 1.1, paint: "slate-700" },
      { part: "embers", x: 82, y: 80, scale: 1.2, paint: "slate-400" },
      { part: "light-shaft", x: 50, y: 22, scale: 1, paint: "white" },
      { part: "ink-seal", x: 80, y: 74 },
    ],
  },
};

/** RCTA (CN) (Meditative #35, Common Spell): a bewildered knight in a records office taking a red seal stamp on his shoulder, his armour blooming with red silk and jade trim. The picture's seal is the stamp on his shoulder. */
const RCTA_CN: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 56, y: 40, r: 30, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 60, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 1,
    pose: "pose-stand",
    look: { ...figureLookFor(["Human"]), hair: "helm", outfit: "outfit-armor", item: null, cloth: "red-600" },
  },
  layers: {
    mid: [
      { part: "tatami", x: 50, y: 70 },
      { part: "bookshelf", x: 18, y: 72, scale: 1.5 },
      { part: "bookshelf", x: 82, y: 72, scale: 1.5 },
      { part: "table", x: 82, y: 84, scale: 0.8 },
      { part: "scroll", x: 82, y: 77, scale: 0.6 },
      { part: "lantern", x: 30, y: 30, scale: 0.8 },
    ],
    subject: [{ part: "light-ribbon", x: 50, y: 62, scale: 0.8, paint: "red-600" }],
    near: [
      { part: "petals", x: 50, y: 56, scale: 0.6, paint: "jade-400" },
      { part: "ink-seal", x: 56, y: 47, scale: 0.8 },
    ],
  },
};

/** CN Peptides (Meditative #36, Common Spell): an apothecary's vial pouring over a nervous warrior who is half bulking up and half sprouting tentacles, in a herbalist's cluttered back room. */
const CN_PEPTIDES: SceneRecipe = {
  family: "meditative",
  sky: ["slate-700", "slate-400"],
  light: { x: 50, y: 30, r: 32, paint: "jade-400", opacity: 0.35 },
  emblem: { x: 50, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 0.95,
    pose: "pose-flex",
    look: { ...figureLookFor(["Human"]), hair: "hair-short", outfit: "outfit-muscle", item: null, cloth: "green-700" },
  },
  layers: {
    mid: [
      { part: "boards", x: 50, y: 72 },
      { part: "shelf", x: 18, y: 60, scale: 1 },
      { part: "shelf", x: 82, y: 60, scale: 1 },
      { part: "jar", x: 20, y: 76, scale: 1, count: 3, spread: [6, 1] },
    ],
    subject: [{ part: "vine", x: 62, y: 76, scale: 2.2, paint: "violet-500" }],
    near: [
      { part: "hand-hold", x: 34, y: 40, scale: 0.9 },
      { part: "vial", x: 34, y: 40, scale: 1.2, paint: "jade-400" },
      { part: "drops", x: 40, y: 45, scale: 0.6, paint: "jade-400" },
      { part: "ink-seal", x: 82, y: 74 },
    ],
  },
};

/** CN in a Bottle (Meditative #37, Epic Spell): a corked glass bottle washed up on a pebbled river bank, a tiny lantern-lit city swirling inside it, a fisherman reaching down for it, karst peaks in morning haze. */
const CN_IN_A_BOTTLE: SceneRecipe = {
  family: "meditative",
  sky: ["slate-100", "blue-200"],
  light: { x: 30, y: 30, r: 36, paint: "white", opacity: 0.5 },
  emblem: { x: 40, y: 78, size: 5, rotate: 0 },
  extras: [
    {
      layer: "subject",
      x: 60,
      y: 82,
      scale: 0.85,
      flip: -1,
      pose: "pose-kneel",
      look: { ...figureLookFor(["CN"]), hair: "hat-brim", outfit: "outfit-poncho", item: null, cloth: "slate-700" },
    },
  ],
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 50, scale: 0.8 },
      { part: "needle-peak", x: 18, y: 56, scale: 0.7 },
      { part: "needle-peak", x: 34, y: 54, scale: 0.55 },
      { part: "needle-peak", x: 80, y: 56, scale: 0.8 },
      { part: "mist", x: 50, y: 54, scale: 1.1, count: 2, spread: [20, 2] },
    ],
    mid: [
      { part: "water", x: 50, y: 62 },
      { part: "boat", x: 78, y: 66, scale: 0.5 },
      { part: "reed", x: 14, y: 76, scale: 1.2, count: 2, spread: [5, 1] },
    ],
    subject: [{ part: "sand", x: 50, y: 76 }],
    near: [
      { part: "bottle", x: 40, y: 79, scale: 2 },
      { part: "pebbles", x: 40, y: 86, scale: 1.4, count: 2, spread: [14, 3] },
      { part: "ink-seal", x: 82, y: 76 },
    ],
  },
};

/** H1B Printer (Meditative #38, Rare Field Spell): an office copy room where a humming printer feeds out tiny workers instead of paper, a queue of applicants stretching down a grey corridor. */
const H1B_PRINTER: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 50, y: 40, r: 34, paint: "jade-400", opacity: 0.3 },
  emblem: { x: 48, y: 72, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 58, paint: "slate-400" },
      { part: "doorway", x: 72, y: 60, scale: 1.4 },
    ],
    mid: [
      { part: "tile-floor", x: 72, y: 58 },
      { part: "ink-person", x: 70, y: 62, scale: 0.7, count: 2, spread: [2, 1] },
      { part: "ink-person", x: 62, y: 66, scale: 0.9 },
      { part: "ink-person", x: 56, y: 68, scale: 0.95, flip: -1 },
      { part: "ink-person", x: 80, y: 70, scale: 1.05, flip: -1 },
    ],
    subject: [
      { part: "printer", x: 46, y: 82, scale: 1.8 },
      { part: "ink-person", x: 30, y: 84, scale: 0.55, count: 3, spread: [6, 1] },
    ],
    near: [
      { part: "sparks", x: 32, y: 74, scale: 0.6, paint: "jade-400" },
      { part: "ink-seal", x: 82, y: 76 },
    ],
  },
};

/** 赌石 Addict (Meditative #39, Rare Unit): a wiry man with a loupe and torch crouched over a grey boulder at a dusty border-town stone stall, eyes feverish, sweat on his brow. */
const GAMBLING_STONE_ADDICT: SceneRecipe = {
  family: "meditative",
  sky: ["orange-400", "slate-100"],
  light: { x: 36, y: 66, r: 24, paint: "white", opacity: 0.45 },
  emblem: { x: 56, y: 64, size: 5, rotate: 0 },
  figure: {
    x: 58,
    y: 86,
    scale: 0.95,
    flip: -1,
    pose: "pose-kneel",
    look: { ...figureLookFor(["CN"]), hair: "headband", outfit: "outfit-work-apron", item: "item-loupe", cloth: "brass-800" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 50, scale: 0.6, paint: "stone-500" },
      { part: "tiled-house", x: 20, y: 58, scale: 0.8 },
      { part: "tiled-house", x: 84, y: 58, scale: 0.7, flip: -1 },
    ],
    mid: [
      { part: "sand", x: 50, y: 62 },
      { part: "awning", x: 50, y: 72, scale: 1.4, paint: "red-600" },
      { part: "jade-boulder", x: 76, y: 80, scale: 0.6, paint: "stone-500" },
    ],
    subject: [
      { part: "jade-boulder", x: 36, y: 86, scale: 1.2, paint: "stone-500" },
      { part: "drops", x: 54, y: 40, scale: 0.3, paint: "sky-300" },
    ],
    near: [
      { part: "dust", x: 20, y: 88, scale: 1.2 },
      { part: "ink-seal", x: 82, y: 76 },
    ],
  },
};

/** Auspicious Rock (Meditative #39's token, Token Spell): a grey river stone half-sawn on a workshop bench, green light leaking from the cut, stone dust hanging in a sunbeam. */
const AUSPICIOUS_ROCK: SceneRecipe = {
  family: "meditative",
  sky: ["slate-700", "slate-400"],
  light: { x: 52, y: 60, r: 26, paint: "jade-400", opacity: 0.45 },
  emblem: { x: 46, y: 72, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 62, paint: "slate-700" },
      { part: "window", x: 24, y: 40, scale: 1 },
    ],
    mid: [
      { part: "boards", x: 50, y: 70 },
      { part: "wide-table", x: 50, y: 88, scale: 2, paint: "wood-700" },
    ],
    subject: [
      { part: "jade-boulder", x: 50, y: 78, scale: 1.6 },
      { part: "saw", x: 66, y: 66, scale: 1.2 },
    ],
    near: [
      { part: "light-shaft", x: 24, y: 34, scale: 1, paint: "white" },
      { part: "motes", x: 40, y: 60, scale: 0.8 },
      { part: "ink-seal", x: 82, y: 76 },
    ],
  },
};

/** Jade (Meditative #39's token, Token Spell): a polished green jade disc in an open palm beside a courtyard pond, a soft wisp of mana rising from it. */
const JADE: SceneRecipe = {
  family: "meditative",
  sky: ["slate-100", "blue-200"],
  light: { x: 50, y: 50, r: 30, paint: "jade-400", opacity: 0.35 },
  emblem: { x: 50, y: 58, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-round-window", x: 50, y: 40, scale: 1.4 },
      { part: "ink-peaks", x: 50, y: 46, scale: 0.4 },
    ],
    mid: [
      { part: "still-water", x: 50, y: 60 },
      { part: "koi", x: 30, y: 72, scale: 1.1 },
      { part: "stone-lantern", x: 82, y: 66, scale: 1 },
    ],
    subject: [
      { part: "hand-open", x: 50, y: 74, scale: 2 },
      { part: "jade-disc", x: 50, y: 64, scale: 1.3 },
    ],
    near: [{ part: "ink-seal", x: 80, y: 78 }],
  },
};

/** Dud (Meditative #39's token, Token Spell): a plain grey rock, sawn open to nothing, hurled by a furious buyer down a dusty back alley. */
const DUD: SceneRecipe = {
  family: "meditative",
  sky: ["stone-500", "slate-100"],
  light: { x: 50, y: 40, r: 30, paint: "white", opacity: 0.35 },
  emblem: { x: 60, y: 52, size: 5, rotate: 0 },
  figure: {
    x: 40,
    y: 86,
    scale: 0.95,
    pose: "pose-cast",
    look: { ...figureLookFor(["CN"]), hair: "hair-short", outfit: "outfit-jacket", item: null, cloth: "red-900" },
  },
  layers: {
    far: [
      { part: "tiled-house", x: 12, y: 72, scale: 1.7 },
      { part: "tiled-house", x: 88, y: 72, scale: 1.7, flip: -1 },
    ],
    mid: [{ part: "cobbles", x: 50, y: 70 }],
    subject: [
      { part: "jade-boulder", x: 62, y: 56, scale: 0.9, paint: "stone-500" },
      { part: "streaks", x: 58, y: 52, scale: 0.6, flip: -1 },
    ],
    near: [
      { part: "dust", x: 34, y: 88, scale: 1.2 },
      { part: "ink-seal", x: 82, y: 76 },
    ],
  },
};

/** Red Jade (Meditative #39's token, Token Spell): a rare blood-red jade seam glistening in a dark mountain mine wall, a miner's lamp catching its glow. */
const RED_JADE: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "slate-800"],
  light: { x: 50, y: 60, r: 26, paint: "red-600", opacity: 0.4 },
  emblem: { x: 50, y: 66, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "cave", x: 50, y: 70 }],
    mid: [
      { part: "rock-floor", x: 50, y: 80 },
      { part: "pebbles", x: 30, y: 82, scale: 1.2 },
    ],
    subject: [
      { part: "halo", x: 50, y: 62, scale: 2.4, paint: "red-500" },
      { part: "jade-boulder", x: 50, y: 74, scale: 1.7, paint: "red-600" },
      { part: "crystal", x: 36, y: 72, scale: 1, paint: "red-500", count: 2, spread: [4, 2] },
      { part: "crystal", x: 64, y: 72, scale: 0.9, paint: "red-500" },
    ],
    near: [
      { part: "lantern", x: 72, y: 36, scale: 1.1 },
      { part: "sparks", x: 52, y: 64, scale: 0.8, paint: "red-300" },
      { part: "ink-seal", x: 20, y: 76 },
    ],
  },
};

/** Jade Beauty (Meditative #39's token, Token Unit): a towering jade-skinned woman in silk robes stepping out of a cracked boulder, enemy soldiers kneeling spellbound, green mist in a mountain quarry. */
const JADE_BEAUTY: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 50, y: 44, r: 34, paint: "jade-400", opacity: 0.4 },
  emblem: { x: 50, y: 56, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 84,
    scale: 1.15,
    pose: "pose-spread",
    look: { ...figureLookFor(["CN"]), hair: "veil", outfit: "outfit-gown", item: null, skin: "jade-400", cloth: "green-700" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 50, scale: 0.7 },
      { part: "cliff", x: 10, y: 46, scale: 1.2, paint: "stone-500" },
      { part: "cliff", x: 90, y: 48, scale: 1.1, flip: -1, paint: "stone-500" },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 74 },
      { part: "mist", x: 50, y: 72, scale: 1, paint: "jade-400" },
    ],
    subject: [{ part: "jade-boulder", x: 50, y: 86, scale: 2.2, paint: "jade-400" }],
    near: [
      { part: "soldier", x: 20, y: 86, scale: 1 },
      { part: "soldier", x: 80, y: 86, scale: 1, flip: -1 },
      { part: "ink-seal", x: 82, y: 70 },
    ],
  },
};

/** Feng Shui (Meditative #40, Legendary Field Spell): a courtyard house laid out on a great compass, water, wood, fire, metal and earth at their points round a koi pond; a servant slides a carved couch three inches left and the five elements swirl into balance, a shaft of sun breaking through the clouds onto the tiled floor. */
const FENG_SHUI: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 50, y: 30, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 72, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "cloud", x: 26, y: 24, scale: 1.4, paint: "slate-400" },
      { part: "cloud", x: 74, y: 22, scale: 1.2, paint: "slate-400" },
    ],
    far: [
      { part: "tiled-house", x: 50, y: 50, scale: 1.3 },
      { part: "tiled-house", x: 14, y: 54, scale: 1.1 },
      { part: "tiled-house", x: 86, y: 54, scale: 1.1, flip: -1 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 54 },
      { part: "rune-circle", x: 50, y: 72, scale: 2, paint: "jade-400" },
      { part: "koi", x: 50, y: 72, scale: 1 },
      { part: "fountain", x: 50, y: 62, scale: 0.4 },
      { part: "bamboo", x: 22, y: 70, scale: 0.4 },
      { part: "flames", x: 78, y: 70, scale: 0.6 },
      { part: "coins", x: 72, y: 80, scale: 0.6 },
      { part: "pebbles", x: 28, y: 80, scale: 0.6 },
    ],
    subject: [
      { part: "bench", x: 40, y: 81, scale: 0.7, paint: "red-900" },
      { part: "ink-person", x: 56, y: 81, scale: 1.5, flip: -1 },
    ],
    near: [
      { part: "light-ribbon", x: 50, y: 70, scale: 1, paint: "jade-400" },
      { part: "ink-seal", x: 82, y: 78 },
    ],
    light: [
      { part: "light-shaft", x: 50, y: 18, scale: 1.2, paint: "white" },
      { part: "halo", x: 50, y: 72, scale: 2.2, paint: "white" },
    ],
    particles: [
      { part: "leaves", x: 30, y: 60, scale: 0.7 },
      { part: "motes", x: 50, y: 62, scale: 1, paint: "white" },
    ],
  },
};

/** CN Smuggler (Meditative #41, Rare Unit): a sly smuggler pushing two handcarts across a border bridge, the second piled with rough rocks under a tarp, foggy dawn. */
const CN_SMUGGLER: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "red-300"],
  light: { x: 76, y: 36, r: 30, paint: "red-300", opacity: 0.35 },
  emblem: { x: 46, y: 60, size: 5, rotate: 0 },
  figure: {
    x: 46,
    y: 84,
    scale: 0.95,
    pose: "pose-cast",
    look: { ...figureLookFor(["CN"]), hair: "hood", outfit: "outfit-coat", item: null, cloth: "slate-700" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 54, scale: 0.7 },
      { part: "red-gate", x: 84, y: 66, scale: 0.8 },
      { part: "mist", x: 50, y: 60, scale: 1.1, count: 2, spread: [18, 2] },
    ],
    mid: [
      { part: "water", x: 50, y: 70 },
      { part: "bridge", x: 50, y: 84, scale: 1.2 },
    ],
    subject: [
      { part: "cart", x: 66, y: 85, scale: 1.1 },
      { part: "cart", x: 82, y: 86, scale: 1, paint: "wood-800" },
      { part: "jade-boulder", x: 82, y: 79, scale: 0.6, paint: "stone-500" },
    ],
    near: [
      { part: "mist", x: 30, y: 88, scale: 0.8 },
      { part: "ink-seal", x: 18, y: 74 },
    ],
  },
};

/** CN Flea Market (Meditative #42, Legendary Spell): a night market bursting into life in a riverside alley: lanterns flare on in a wave, stall awnings unfurl, a merchant slaps cards down on red cloth, a buyer haggles with a fistful of copper coins, steam rises from food carts, red and jade light shimmers on wet cobbles. */
const CN_FLEA_MARKET: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 62, r: 36, paint: "red-500", opacity: 0.35 },
  emblem: { x: 46, y: 70, size: 5, rotate: 0 },
  figure: {
    x: 64,
    y: 84,
    scale: 0.85,
    flip: -1,
    pose: "pose-point",
    look: { ...figureLookFor(["CN"]), hair: "kerchief", outfit: "outfit-coat", item: "item-coin", cloth: "jade-400" },
  },
  layers: {
    sky: [{ part: "stars", x: 50, y: 22, scale: 1.2 }],
    far: [
      { part: "tiled-house", x: 22, y: 56, scale: 0.8 },
      { part: "water", x: 50, y: 58 },
    ],
    mid: [
      { part: "cobbles", x: 50, y: 66 },
      { part: "lantern-string", x: 50, y: 36, scale: 1 },
      { part: "cart", x: 84, y: 76, scale: 0.8 },
      { part: "steam", x: 84, y: 66, scale: 0.8 },
    ],
    subject: [
      { part: "awning", x: 44, y: 80, scale: 1.3, paint: "red-600" },
      { part: "ink-person", x: 38, y: 72, scale: 1.2 },
      { part: "playing-card", x: 46, y: 70, scale: 0.6, count: 2, spread: [4, 1] },
    ],
    near: [
      { part: "paper-lantern", x: 10, y: 24, scale: 1.3 },
      { part: "paper-lantern", x: 90, y: 26, scale: 1.2 },
      { part: "coins", x: 70, y: 88, scale: 0.8 },
      { part: "ink-seal", x: 82, y: 76 },
    ],
    light: [
      { part: "halo", x: 50, y: 40, scale: 2.4, paint: "red-500" },
      { part: "halo", x: 44, y: 66, scale: 2, paint: "jade-400" },
    ],
    particles: [
      { part: "sparks", x: 30, y: 36, scale: 1, paint: "red-300" },
      { part: "motes", x: 50, y: 80, scale: 1, paint: "jade-400" },
    ],
  },
};

/** CN Jade Market (Meditative #43, Rare Spell): a bustling jade market street where every stall heaps identical grey stones, smiling buyers hugging armfuls away, hazy midday. */
const CN_JADE_MARKET: SceneRecipe = {
  family: "meditative",
  sky: ["slate-100", "slate-400"],
  light: { x: 50, y: 30, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 50, scale: 0.6 },
      { part: "tiled-house", x: 30, y: 56, scale: 0.7 },
      { part: "tiled-house", x: 70, y: 56, scale: 0.7, flip: -1 },
    ],
    mid: [
      { part: "cobbles", x: 50, y: 62 },
      { part: "awning", x: 18, y: 70, scale: 0.9, paint: "jade-400" },
      { part: "awning", x: 82, y: 70, scale: 0.9, paint: "red-600" },
      { part: "jade-boulder", x: 16, y: 64, scale: 0.4, paint: "stone-500" },
      { part: "jade-boulder", x: 84, y: 64, scale: 0.4, paint: "stone-500" },
    ],
    subject: [
      { part: "awning", x: 50, y: 80, scale: 1.2, paint: "jade-400" },
      { part: "jade-boulder", x: 50, y: 72, scale: 0.45, paint: "stone-500", count: 3, spread: [6, 1] },
    ],
    near: [
      { part: "ink-person", x: 28, y: 88, scale: 1.4 },
      { part: "jade-boulder", x: 28, y: 82, scale: 0.3, paint: "stone-500" },
      { part: "ink-person", x: 70, y: 88, scale: 1.3, flip: -1 },
      { part: "ink-seal", x: 82, y: 72 },
    ],
  },
};

/** CN Jade Well (Meditative #44, Rare Field Spell): a moss-covered stone well in a bamboo grove, its water glowing green, a rough rock rising in the bucket, morning mist. */
const CN_JADE_WELL: SceneRecipe = {
  family: "meditative",
  sky: ["slate-100", "jade-400"],
  light: { x: 50, y: 58, r: 30, paint: "jade-400", opacity: 0.45 },
  emblem: { x: 50, y: 76, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 50, scale: 0.6 },
      { part: "bamboo", x: 18, y: 62, scale: 0.8, count: 2, spread: [8, 1] },
      { part: "bamboo", x: 82, y: 62, scale: 0.8, count: 2, spread: [8, 1] },
      { part: "mist", x: 50, y: 60, scale: 1.1 },
    ],
    mid: [
      { part: "field", x: 50, y: 68, paint: "green-900" },
      { part: "bamboo", x: 10, y: 90, scale: 1.6 },
      { part: "bamboo", x: 90, y: 88, scale: 1.5 },
    ],
    subject: [
      { part: "halo", x: 50, y: 66, scale: 2.2, paint: "jade-400" },
      { part: "stone-well", x: 50, y: 84, scale: 2.4 },
      { part: "jade-boulder", x: 58, y: 66, scale: 0.35, paint: "stone-500" },
    ],
    near: [
      { part: "mist", x: 50, y: 86, scale: 0.9 },
      { part: "ink-seal", x: 80, y: 74 },
    ],
  },
};

/** Knowledge Breaker (Meditative #45, Legendary Unit): a slight robed figure in a cracked porcelain mask stands palms outward in an academy lecture hall; scrolls unravel and books bleed their ink into the air, students clutch their heads as their knowledge streams out of them, and the hall's lamps dim to chalk-white. */
const KNOWLEDGE_BREAKER: SceneRecipe = {
  family: "meditative",
  sky: ["slate-800", "slate-700"],
  light: { x: 50, y: 50, r: 36, paint: "slate-100", opacity: 0.35 },
  emblem: { x: 50, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 84,
    scale: 0.9,
    pose: "pose-spread",
    look: { ...figureLookFor(["CN"]), head: "head-automaton", hair: "hood", outfit: "outfit-robe", item: null, skin: "slate-100", cloth: "slate-900" },
  },
  layers: {
    sky: [{ part: "smoke-billow", x: 50, y: 24, scale: 1.4, paint: "slate-900" }],
    far: [
      { part: "room-wall", x: 50, y: 60, paint: "slate-700" },
      { part: "blackboard", x: 50, y: 50, scale: 1.4 },
      { part: "bookshelf", x: 14, y: 62, scale: 1.4 },
      { part: "bookshelf", x: 86, y: 62, scale: 1.4 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 60 },
      { part: "desk", x: 24, y: 72, scale: 1 },
      { part: "desk", x: 76, y: 72, scale: 1 },
      { part: "ink-person", x: 24, y: 70, scale: 1.2 },
      { part: "ink-person", x: 76, y: 70, scale: 1.2, flip: -1 },
    ],
    subject: [
      { part: "scroll", x: 30, y: 46, scale: 0.6 },
      { part: "tome", x: 70, y: 42, scale: 0.5 },
    ],
    near: [
      { part: "lantern", x: 12, y: 24, scale: 1, paint: "slate-100" },
      { part: "lantern", x: 88, y: 24, scale: 1, paint: "slate-100" },
      { part: "ink-person", x: 14, y: 90, scale: 1.6 },
      { part: "ink-seal", x: 82, y: 76 },
    ],
    light: [
      { part: "halo", x: 50, y: 50, scale: 3, paint: "slate-100" },
      { part: "light-ribbon", x: 50, y: 30, scale: 1, paint: "slate-100" },
    ],
    particles: [
      { part: "drops", x: 70, y: 48, scale: 0.8, paint: "slate-900" },
      { part: "drops", x: 30, y: 52, scale: 0.8, paint: "slate-900" },
      { part: "motes", x: 50, y: 40, scale: 1.2, paint: "slate-100" },
    ],
  },
};

/** Knowledge Breaker Prime (Meditative #45's token, Token Unit): a towering masked titan bursting through a mountain archive tower, scrolls exploding into ash around it, traps snapping open at its feet, white glare. */
const KNOWLEDGE_BREAKER_PRIME: SceneRecipe = {
  family: "meditative",
  sky: ["slate-100", "slate-400"],
  light: { x: 50, y: 34, r: 44, paint: "white", opacity: 0.6 },
  emblem: { x: 50, y: 56, size: 6, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 1.2,
    pose: "pose-block",
    look: { ...figureLookFor(["CN"]), head: "head-automaton", hair: "horns", outfit: "outfit-plate", item: null, skin: "slate-100", cloth: "slate-800" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 56, scale: 0.8 },
      { part: "tower", x: 22, y: 70, scale: 1.4, paint: "slate-700" },
      { part: "tower", x: 80, y: 70, scale: 1.2, paint: "slate-700" },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 78 },
      { part: "ruin-wall", x: 50, y: 82, scale: 1.2 },
    ],
    subject: [{ part: "god-rays", x: 50, y: 30, scale: 1, paint: "white" }],
    near: [
      { part: "scroll", x: 24, y: 48, scale: 0.6 },
      { part: "embers", x: 24, y: 44, scale: 1.2, paint: "slate-400" },
      { part: "embers", x: 76, y: 40, scale: 1.2, paint: "slate-400" },
      { part: "snare", x: 32, y: 86, scale: 1.2 },
      { part: "snare", x: 68, y: 87, scale: 1.1 },
      { part: "ink-seal", x: 82, y: 70 },
    ],
  },
};

/** Conjure Intellect (Meditative #46, Epic Spell): a meditating scholar's upturned palms, three glowing shapes forming above them (a chalk slate, a red silk scroll and a leather-bound book), a misty hermitage terrace at dawn. */
const CONJURE_INTELLECT: SceneRecipe = {
  family: "meditative",
  sky: ["red-300", "slate-100"],
  light: { x: 50, y: 50, r: 36, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 52, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 76, y: 30, scale: 0.7 }],
    far: [
      { part: "ink-peaks", x: 50, y: 52, scale: 0.9 },
      { part: "ink-pavilion", x: 22, y: 62, scale: 0.7 },
      { part: "pine", x: 82, y: 62, scale: 0.8 },
      { part: "mist", x: 50, y: 60, scale: 1.1, count: 2, spread: [18, 2] },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66, paint: "slate-400" },
      { part: "stone-lantern", x: 84, y: 76, scale: 1 },
    ],
    subject: [
      { part: "halo", x: 34, y: 54, scale: 2, paint: "white" },
      { part: "slate-board", x: 34, y: 61, scale: 0.85 },
      { part: "halo", x: 50, y: 42, scale: 2, paint: "red-300" },
      { part: "scroll", x: 50, y: 45, scale: 1, paint: "red-600" },
      { part: "halo", x: 66, y: 54, scale: 2, paint: "white" },
      { part: "tome", x: 66, y: 58, scale: 0.85, paint: "wood-700" },
    ],
    near: [
      { part: "hand-open", x: 40, y: 82, scale: 2 },
      { part: "hand-open", x: 60, y: 82, scale: 2, flip: -1 },
      { part: "motes", x: 50, y: 66, scale: 0.8, paint: "white" },
      { part: "ink-seal", x: 82, y: 70 },
    ],
  },
};

/** 饕餮 (Meditative #47, Legendary Unit): a taotie, the jawless horned beast of ancient ritual bronzes, rises from a verdigris cauldron in a dark ancestral temple; its maw drags a struggling enemy warrior into itself through swirling green-bronze smoke as the carved spirals on its brow glow, torchlight flickering on the pillars. */
const TAOTIE: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "slate-800"],
  light: { x: 50, y: 50, r: 34, paint: "green-400", opacity: 0.3 },
  emblem: { x: 50, y: 78, size: 6, rotate: 0 },
  extras: [
    {
      layer: "near",
      x: 70,
      y: 58,
      scale: 0.42,
      flip: -1,
      pose: "pose-run",
      look: { ...figureLookFor(["Human"]), hair: "steel-helmet", outfit: "outfit-armor", item: "item-sword", cloth: "slate-400" },
    },
  ],
  layers: {
    sky: [{ part: "smoke-billow", x: 50, y: 24, scale: 1.4, paint: "green-900" }],
    far: [
      { part: "room-wall", x: 50, y: 66, paint: "slate-800" },
      { part: "column", x: 14, y: 76, scale: 1.4 },
      { part: "column", x: 86, y: 76, scale: 1.4 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 70, paint: "slate-700" },
      { part: "pedestal", x: 24, y: 80, scale: 0.8 },
      { part: "flames", x: 24, y: 72, scale: 0.8 },
      { part: "pedestal", x: 76, y: 80, scale: 0.8 },
      { part: "flames", x: 76, y: 72, scale: 0.8 },
    ],
    subject: [
      { part: "taotie", x: 50, y: 70, scale: 1.3 },
      { part: "cauldron", x: 50, y: 86, scale: 1.8, paint: "green-700" },
    ],
    near: [
      { part: "smoke", x: 62, y: 70, scale: 1.4, paint: "green-700" },
      { part: "smoke", x: 36, y: 72, scale: 1.2, paint: "green-900" },
      { part: "ink-seal", x: 20, y: 72 },
    ],
    light: [
      { part: "halo", x: 50, y: 50, scale: 1.6, paint: "jade-400" },
      { part: "rim-light", x: 50, y: 56, scale: 1.4, paint: "orange-400" },
    ],
    particles: [
      { part: "embers", x: 24, y: 66, scale: 1 },
      { part: "embers", x: 76, y: 66, scale: 1 },
    ],
  },
};

/** Tranquility (Meditative #48, Epic Spell): a monk breathing out on a lake pavilion, the water glassy around him, a storm frozen mid-strike with its lightning hanging motionless, black clouds paused over the mountains, calm silver light. */
const TRANQUILITY: SceneRecipe = {
  family: "meditative",
  sky: ["slate-800", "slate-400"],
  light: { x: 50, y: 60, r: 36, paint: "slate-100", opacity: 0.45 },
  emblem: { x: 50, y: 70, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 82,
    scale: 0.8,
    pose: "pose-sit",
    look: { ...figureLookFor(["Human"]), hair: "tonsure", outfit: "outfit-robe", item: null, cloth: "slate-400" },
  },
  layers: {
    sky: [
      { part: "cloud", x: 24, y: 24, scale: 1.6, paint: "slate-800" },
      { part: "cloud", x: 70, y: 22, scale: 1.8, paint: "slate-900" },
    ],
    far: [
      { part: "ink-peaks", x: 50, y: 54, scale: 0.9 },
      { part: "lightning", x: 70, y: 28, scale: 1, paint: "sky-300" },
      { part: "cloud", x: 50, y: 34, scale: 1.2, paint: "slate-800" },
    ],
    mid: [{ part: "still-water", x: 50, y: 60 }],
    subject: [{ part: "ink-pavilion", x: 50, y: 84, scale: 1.3 }],
    near: [
      { part: "reed", x: 14, y: 90, scale: 1.4 },
      { part: "reed", x: 86, y: 90, scale: 1.3 },
      { part: "ink-seal", x: 82, y: 72 },
    ],
  },
};

/** YileGPT (Meditative #49, Mythic Unit): a slender scholar-spirit of flowing ink and soft blue light, its hooded robe trailing into glowing brush-strokes, sits at a calligraphy desk in a cloud-top pavilion; as its brush moves a glitching drop of ink falls off the page and scampers across the board toward the enemy side, the ink-wash mountains behind it flickering like a screen; cyan and vermilion light. */
const YILE_GPT: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "blue-800"],
  light: { x: 50, y: 50, r: 38, paint: "cyan-400", opacity: 0.35 },
  emblem: { x: 46, y: 62, size: 6, rotate: 0 },
  figure: {
    x: 46,
    y: 84,
    scale: 1,
    pose: "pose-perch",
    look: { ...figureLookFor(["CN"]), head: "head-void", hair: "hood", outfit: "outfit-robe", item: "item-ink-brush", skin: "slate-900", cloth: "blue-800" },
  },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 22, scale: 1.2 },
      { part: "ink-cloud", x: 20, y: 30, scale: 0.8, paint: "slate-700" },
      { part: "ink-cloud", x: 80, y: 26, scale: 0.7, paint: "slate-700" },
    ],
    far: [
      { part: "ink-peaks", x: 50, y: 54, scale: 1 },
      { part: "glitch", x: 26, y: 40, scale: 1.2 },
      { part: "glitch", x: 74, y: 44, scale: 1 },
      { part: "mist", x: 50, y: 58, scale: 1.2, count: 2, spread: [20, 2] },
    ],
    mid: [
      { part: "ink-pavilion", x: 46, y: 78, scale: 2.2 },
      { part: "cloud", x: 12, y: 76, scale: 1.4, paint: "slate-400" },
      { part: "cloud", x: 88, y: 74, scale: 1.4, paint: "slate-400" },
      { part: "paper-lantern", x: 28, y: 54, scale: 1, paint: "red-500" },
      { part: "paper-lantern", x: 64, y: 54, scale: 1, paint: "red-500" },
    ],
    subject: [
      { part: "light-ribbon", x: 40, y: 72, scale: 0.8, paint: "cyan-400" },
      { part: "brush", x: 30, y: 74, scale: 0.6 },
    ],
    near: [
      { part: "desk", x: 50, y: 90, scale: 2.4, paint: "wood-800" },
      { part: "scroll", x: 50, y: 74, scale: 0.8 },
      { part: "ink-imp", x: 74, y: 76, scale: 1.4 },
      { part: "streaks", x: 70, y: 74, scale: 0.4 },
      { part: "ink-seal", x: 18, y: 76 },
    ],
    light: [
      { part: "halo", x: 46, y: 50, scale: 2.4, paint: "cyan-400" },
      { part: "rim-light", x: 46, y: 60, scale: 1.4, paint: "red-500" },
    ],
    particles: [
      { part: "motes", x: 46, y: 50, scale: 1.2, paint: "cyan-400" },
      { part: "squares", x: 70, y: 70, scale: 0.4, paint: "red-500" },
      { part: "sparks", x: 46, y: 70, scale: 0.8, paint: "sky-300" },
    ],
  },
};

/** YileGPT Prime (Meditative #49's token, Token Unit): a colossal ink-and-light scholar unbound, a hundred glowing brushes orbiting it above a crumbling pavilion, data mist pouring down the mountainside. */
const YILE_GPT_PRIME: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 44, r: 40, paint: "cyan-400", opacity: 0.4 },
  emblem: { x: 50, y: 52, size: 6, rotate: 0 },
  figure: {
    x: 50,
    y: 82,
    scale: 1.15,
    pose: "pose-raise",
    look: { ...figureLookFor(["CN"]), head: "head-void", hair: "hood", outfit: "outfit-gown", item: "item-ink-brush", skin: "slate-900", cloth: "sky-300" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 56, scale: 1 },
      { part: "mist", x: 30, y: 66, scale: 1, paint: "cyan-400" },
      { part: "glitch", x: 76, y: 46, scale: 1 },
    ],
    mid: [
      { part: "ink-pavilion", x: 50, y: 88, scale: 1.7 },
      { part: "ruin-wall", x: 16, y: 86, scale: 1 },
      { part: "fallen-column", x: 80, y: 86, scale: 0.7 },
      { part: "dust", x: 66, y: 84, scale: 0.8 },
    ],
    subject: [
      { part: "brush", x: 22, y: 50, scale: 0.6, count: 3, spread: [6, 14] },
      { part: "brush", x: 78, y: 50, scale: 0.6, count: 3, spread: [6, 14], flip: -1 },
    ],
    near: [
      { part: "mist", x: 50, y: 88, scale: 1, paint: "sky-300" },
      { part: "ink-seal", x: 82, y: 74 },
    ],
  },
};

/** Yile's Virus (Meditative #49's token, Token Unit): a small boxy glitch creature with flickering pixel spots, sparking at its neighbours from a defensive crouch on a cracked stone floor. */
const YILES_VIRUS: SceneRecipe = {
  family: "meditative",
  sky: ["slate-800", "slate-700"],
  light: { x: 50, y: 64, r: 30, paint: "cyan-400", opacity: 0.35 },
  emblem: { x: 48, y: 74, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 54, scale: 0.7, paint: "slate-800" },
      { part: "glitch", x: 30, y: 42, scale: 1 },
    ],
    mid: [{ part: "cracked-earth", x: 50, y: 64, paint: "slate-400" }],
    subject: [
      { part: "glitch-critter", x: 50, y: 82, scale: 2.6 },
      { part: "zap", x: 58, y: 72, scale: 0.6, paint: "cyan-400" },
      { part: "zap", x: 42, y: 72, scale: 0.6, flip: -1, paint: "cyan-400" },
    ],
    near: [
      { part: "ink-person", x: 16, y: 82, scale: 1.4 },
      { part: "ink-person", x: 84, y: 82, scale: 1.4, flip: -1 },
      { part: "ink-seal", x: 80, y: 66 },
    ],
  },
};

/** AI Girlfriend (Meditative #49's token, Token Unit): a calm hologram martial artist in a sparring dojo, turning aside a ring of attackers with one raised palm, soft cyan glow. */
const AI_GIRLFRIEND: SceneRecipe = {
  family: "meditative",
  sky: ["slate-700", "slate-400"],
  light: { x: 50, y: 50, r: 34, paint: "cyan-400", opacity: 0.4 },
  emblem: { x: 50, y: 58, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 84,
    scale: 1,
    pose: "pose-point",
    look: { ...figureLookFor(["CN"]), hair: "hair-bun", outfit: "outfit-silks", item: null, skin: "sky-300", cloth: "cyan-400" },
  },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 62, paint: "wood-700" },
      { part: "paper-screen", x: 20, y: 64, scale: 1.3 },
      { part: "paper-screen", x: 80, y: 64, scale: 1.3, flip: -1 },
    ],
    mid: [
      { part: "tatami", x: 50, y: 64 },
      { part: "ink-person", x: 30, y: 72, scale: 1.3 },
      { part: "ink-person", x: 70, y: 72, scale: 1.3, flip: -1 },
    ],
    subject: [{ part: "halo", x: 50, y: 56, scale: 3, paint: "cyan-400" }],
    near: [
      { part: "ink-person", x: 18, y: 90, scale: 1.7 },
      { part: "ink-person", x: 82, y: 90, scale: 1.7, flip: -1 },
      { part: "glitch", x: 62, y: 44, scale: 0.4 },
      { part: "ink-seal", x: 82, y: 70 },
    ],
  },
};

export const MEDITATIVE_RECIPES_3: Readonly<Record<string, SceneRecipe>> = {
  "meditative-031": THE_CONDUCTOR,
  "meditative-032": SPIRITUALLY_CN,
  "meditative-033": FIRST_DAY_OF_SCHOOL,
  "meditative-034": GAOKAO,
  "meditative-035": RCTA_CN,
  "meditative-036": CN_PEPTIDES,
  "meditative-037": CN_IN_A_BOTTLE,
  "meditative-038": H1B_PRINTER,
  "meditative-039": GAMBLING_STONE_ADDICT,
  "meditative-039-1": AUSPICIOUS_ROCK,
  "meditative-039-2": JADE,
  "meditative-039-3": DUD,
  "meditative-039-4": RED_JADE,
  "meditative-039-5": JADE_BEAUTY,
  "meditative-040": FENG_SHUI,
  "meditative-041": CN_SMUGGLER,
  "meditative-042": CN_FLEA_MARKET,
  "meditative-043": CN_JADE_MARKET,
  "meditative-044": CN_JADE_WELL,
  "meditative-045": KNOWLEDGE_BREAKER,
  "meditative-045-1": KNOWLEDGE_BREAKER_PRIME,
  "meditative-046": CONJURE_INTELLECT,
  "meditative-047": TAOTIE,
  "meditative-048": TRANQUILITY,
  "meditative-049": YILE_GPT,
  "meditative-049-1": YILE_GPT_PRIME,
  "meditative-049-2": YILES_VIRUS,
  "meditative-049-3": AI_GIRLFRIEND,
};
