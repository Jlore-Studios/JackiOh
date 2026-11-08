// R3401000: Meditative's scenes, part 1 of 5: Meditative #1 to #14, each drawn from its brief in the issue's table
// (#3401) in Meditative's family, the ink wash (ART.md, "Meditative and Meditative+" and "Meditative drawn as
// scenes"): slate-900 ink on slate-100 paper, jade washes, a high and distant camera, and one red seal in the near
// layer. Each recipe keeps its subject within x 30 to 70 and y 25 to 85 and its emblem small on its subject; a Unit's
// character is a figure built on its tags' look, or its part of the scene kit in its subject layer. Each doc comment
// gives the brief in a line and any change made to fit the kits.

import { figureLookFor } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** Disruptive Disruptor (Meditative #1, Common Field Spell): a mountain monastery's council room, robed monks around a long low table, each hand of cards bound with red cord, incense haze, calm grey morning. */
const DISRUPTIVE_DISRUPTOR: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 50, y: 30, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 68, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-round-window", x: 50, y: 44, scale: 1.2 },
      { part: "ink-peaks", x: 50, y: 52, scale: 0.4 },
    ],
    mid: [
      { part: "tatami", x: 50, y: 64 },
      { part: "paper-screen", x: 14, y: 70, scale: 1.3 },
      { part: "paper-screen", x: 86, y: 70, scale: 1.3, flip: -1 },
    ],
    subject: [
      { part: "ink-person", x: 34, y: 70, scale: 1.3 },
      { part: "ink-person", x: 66, y: 70, scale: 1.3, flip: -1 },
      { part: "table", x: 50, y: 76, scale: 1.2, paint: "wood-700" },
      { part: "ink-card-fan", x: 44, y: 68, scale: 0.35 },
      { part: "red-thread", x: 44, y: 66, scale: 0.25 },
    ],
    near: [
      { part: "incense", x: 24, y: 84, scale: 1.1 },
      { part: "ink-seal", x: 80, y: 74 },
    ],
  },
};

/** Rampaging Rhino (Meditative #2, Common Unit): a scarred, plate-armoured rhino charging down a misty river gorge, loose cards spilling from its saddlebags, spray flying, cold dawn light. */
const RAMPAGING_RHINO: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "blue-200"],
  light: { x: 70, y: 26, r: 34, paint: "white", opacity: 0.4 },
  emblem: { x: 46, y: 70, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "ink-peaks", x: 50, y: 56, scale: 0.9 }],
    mid: [
      { part: "cliff", x: 12, y: 50, scale: 1.1, paint: "slate-700" },
      { part: "cliff", x: 88, y: 52, scale: 1, flip: -1, paint: "slate-700" },
      { part: "still-water", x: 50, y: 74 },
      { part: "mist", x: 50, y: 66, scale: 0.8 },
    ],
    subject: [{ part: "rhino", x: 50, y: 80, scale: 1.1, flip: -1 }],
    near: [
      { part: "spray", x: 32, y: 80, count: 2, spread: [6, 3] },
      { part: "playing-card", x: 68, y: 72, scale: 0.6 },
      { part: "ink-seal", x: 80, y: 70 },
    ],
  },
};

/** Jlockwork Machine (Meditative #3, Rare Unit): a towering gunmetal clockwork sentry, periscope eye fixed on the far side, nervously flinging cards from its back hopper into a furnace, foggy watchtower yard. */
const JLOCKWORK_MACHINE: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 30, y: 28, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 48, y: 62, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 56, scale: 0.7 },
      { part: "tower", x: 84, y: 64, scale: 1, paint: "slate-700" },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 76 },
      { part: "stone-wall", x: 70, y: 76, scale: 0.5 },
      { part: "mist", x: 50, y: 70, scale: 0.9 },
    ],
    subject: [
      { part: "clockwork-sentry", x: 48, y: 84, scale: 1.2 },
      { part: "hearth", x: 20, y: 82, scale: 0.8 },
      { part: "playing-card", x: 28, y: 48, scale: 0.6, count: 3, spread: [6, 6] },
    ],
    near: [
      { part: "embers", x: 20, y: 66, scale: 1.2 },
      { part: "ink-seal", x: 80, y: 76 },
    ],
  },
};

/** Juicy Kumquat Melon (Meditative #4, Epic Spell): a split kumquat melon on a teahouse cutting board, six glistening slices fanned smallest to largest, a cleaver and a steaming teapot, terraced orchards through an open window, bright noon. */
const JUICY_KUMQUAT_MELON: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "slate-100"],
  light: { x: 50, y: 34, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 46, y: 74, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 42, scale: 0.5 },
      { part: "terraces", x: 50, y: 48 },
      { part: "fruit-tree", x: 32, y: 50, scale: 0.35 },
      { part: "fruit-tree", x: 66, y: 49, scale: 0.3 },
      { part: "ink-round-window", x: 50, y: 44, scale: 1.5 },
    ],
    mid: [
      { part: "paper-screen", x: 14, y: 76, scale: 1.6 },
      { part: "paper-screen", x: 86, y: 76, scale: 1.6, flip: -1 },
      { part: "wide-table", x: 50, y: 96, scale: 2.2, paint: "wood-700" },
      { part: "teapot", x: 72, y: 77, scale: 1.2 },
      { part: "spatula", x: 28, y: 78, scale: 0.9 },
    ],
    subject: [{ part: "melon", x: 47, y: 79, scale: 1.7 }],
    near: [
      { part: "steam", x: 74, y: 67, scale: 0.8 },
      { part: "ink-seal", x: 18, y: 72 },
    ],
  },
};

/** Death by 1000 cuts (Meditative #5, Rare Spell): a storm of countless paper-thin wind blades whipping through a bamboo grove around an armoured warrior, each leaving a tiny nick, cold blue dusk. */
const DEATH_BY_1000_CUTS: SceneRecipe = {
  family: "meditative",
  sky: ["slate-800", "blue-800"],
  light: { x: 68, y: 30, r: 34, paint: "blue-200", opacity: 0.3 },
  emblem: { x: 50, y: 52, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 84,
    scale: 0.95,
    pose: "pose-guard",
    look: { ...figureLookFor(["Human"]), hair: "steel-helmet", outfit: "outfit-armor", item: "item-sword", cloth: "slate-400" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 58, scale: 0.8 },
      { part: "bamboo", x: 12, y: 66, scale: 1.1 },
      { part: "bamboo", x: 88, y: 64, scale: 1.1 },
    ],
    mid: [
      { part: "mist", x: 50, y: 74, scale: 0.9 },
      { part: "bamboo", x: 24, y: 84, scale: 1.5 },
      { part: "bamboo", x: 78, y: 86, scale: 1.6 },
    ],
    near: [
      { part: "shards", x: 50, y: 56, scale: 1.1, count: 4, spread: [18, 14] },
      { part: "gust", x: 50, y: 50, scale: 1.2, count: 2, spread: [10, 14] },
      { part: "ink-seal", x: 80, y: 74 },
    ],
  },
};

/** Me no Likey (Meditative #6, Epic Spell): a sulking hand shoving a ribbon-wrapped little warrior across a go board to the far player, blue mana crystals sliding back as payment, a lamplit pavilion over a lotus pond, playful afternoon. */
const ME_NO_LIKEY: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "slate-100"],
  light: { x: 30, y: 30, r: 38, paint: "orange-400", opacity: 0.25 },
  emblem: { x: 58, y: 66, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 46, scale: 0.5 },
      { part: "still-water", x: 50, y: 52 },
      { part: "ink-pavilion", x: 66, y: 54, scale: 0.9 },
      { part: "paper-lantern", x: 58, y: 40, scale: 0.7 },
      { part: "paper-lantern", x: 74, y: 40, scale: 0.7 },
      { part: "koi", x: 30, y: 58, scale: 0.8 },
    ],
    mid: [
      { part: "boards", x: 50, y: 64 },
      { part: "ink-person", x: 72, y: 70, scale: 1.2, flip: -1 },
    ],
    subject: [
      { part: "go-board", x: 50, y: 84, scale: 2.4 },
      { part: "crystal", x: 38, y: 75, scale: 0.7, count: 3, spread: [4, 1], paint: "sky-400" },
      { part: "soldier", x: 58, y: 74, scale: 0.85 },
      { part: "red-thread", x: 52, y: 67, scale: 0.4 },
    ],
    near: [
      { part: "hand-open", x: 40, y: 72, scale: 1.4 },
      { part: "streaks", x: 64, y: 68, scale: 0.4, flip: -1 },
      { part: "ink-seal", x: 82, y: 74 },
    ],
  },
};

/** Introspection (Meditative #7, Legendary Field Spell): a meditation cave behind a waterfall, a monk cross-legged in still water while glowing thought-shapes orbit him, mending light pouring over kneeling allies on one side and white darts snapping into shadowy foes on the other, shafts of dawn through the falling curtain. */
const INTROSPECTION: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 50, y: 40, r: 42, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 68, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 80,
    scale: 0.95,
    pose: "pose-sit",
    look: { head: "head-human", hair: "tonsure", outfit: "outfit-robe", item: null, skin: "red-300", cloth: "orange-400" },
  },
  layers: {
    sky: [
      { part: "sun", x: 50, y: 30, scale: 0.8, paint: "red-300" },
      { part: "ink-cloud", x: 32, y: 34, scale: 0.6 },
    ],
    far: [
      { part: "waterfall", x: 50, y: 72, scale: 2.4 },
      { part: "mist", x: 50, y: 66, scale: 1 },
    ],
    mid: [
      { part: "cave", x: 50, y: 34 },
      { part: "still-water", x: 50, y: 72 },
      { part: "ink-person", x: 20, y: 78, scale: 1.1 },
      { part: "ink-person", x: 28, y: 80, scale: 1.1 },
      { part: "ghost-figure", x: 76, y: 80, scale: 0.7, paint: "slate-900" },
      { part: "ghost-figure", x: 85, y: 79, scale: 0.6, paint: "slate-900" },
    ],
    subject: [{ part: "thought-shapes", x: 50, y: 52, scale: 1.1 }],
    near: [
      { part: "reed", x: 10, y: 86, scale: 1.2 },
      { part: "flying-arrow", x: 74, y: 64, scale: 0.6, paint: "white", count: 3, spread: [3, 6] },
      { part: "ink-seal", x: 88, y: 82 },
    ],
    light: [
      { part: "light-shaft", x: 44, y: 16, scale: 0.9, paint: "white" },
      { part: "light-shaft", x: 58, y: 16, scale: 0.8, paint: "white" },
      { part: "heal-glow", x: 24, y: 68, scale: 1.2 },
      { part: "halo", x: 50, y: 60, scale: 1.4, paint: "white" },
    ],
    particles: [
      { part: "spray", x: 50, y: 70, scale: 1.2, count: 2, spread: [10, 2] },
      { part: "motes", x: 50, y: 50, scale: 0.8, paint: "white" },
    ],
  },
};

/** Reach the Summit (Meditative #8, Mythic Spell): a lone climber in a wind-torn robe hauls herself onto a needle-thin peak above a sea of ink-wash cloud, eleven ledges spiralling down below each lit by its own flame, the lowest a healing spring, the constellations bending down toward her open hand, cold violet sky, white rim light. */
const REACH_THE_SUMMIT: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "violet-700"],
  light: { x: 54, y: 26, r: 34, paint: "violet-300", opacity: 0.4 },
  emblem: { x: 50, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 46,
    scale: 0.42,
    pose: "pose-raise",
    look: { head: "head-human", hair: "hair-long", outfit: "outfit-robe", item: null, skin: "red-300", cloth: "slate-400" },
  },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 22, scale: 1.6 },
      { part: "stars", x: 58, y: 30, scale: 0.6, paint: "violet-300" },
    ],
    far: [
      { part: "ink-peaks", x: 50, y: 72, scale: 0.7 },
      { part: "needle-peak", x: 22, y: 82, scale: 0.8 },
      { part: "needle-peak", x: 80, y: 84, scale: 0.7 },
      { part: "ink-cloud", x: 24, y: 70, scale: 0.9 },
      { part: "ink-cloud", x: 78, y: 72, scale: 0.9 },
    ],
    mid: [
      { part: "needle-peak", x: 50, y: 108, scale: 1.4 },
      { part: "flame", x: 45, y: 54, scale: 0.4 },
      { part: "flame", x: 55, y: 57, scale: 0.4 },
      { part: "flame", x: 44, y: 60, scale: 0.4 },
      { part: "flame", x: 56, y: 63, scale: 0.4 },
      { part: "flame", x: 43, y: 66, scale: 0.45 },
      { part: "flame", x: 57, y: 69, scale: 0.45 },
      { part: "flame", x: 42, y: 72, scale: 0.45 },
      { part: "flame", x: 58, y: 75, scale: 0.5 },
      { part: "flame", x: 41, y: 78, scale: 0.5 },
      { part: "flame", x: 59, y: 80, scale: 0.5 },
      { part: "heal-fountain", x: 50, y: 87, scale: 0.5 },
    ],
    near: [
      { part: "ink-cloud", x: 24, y: 86, scale: 1.3 },
      { part: "ink-seal", x: 80, y: 72 },
    ],
    light: [
      { part: "rim-light", x: 50, y: 46, scale: 0.6, paint: "white" },
      { part: "halo", x: 54, y: 30, scale: 1.2, paint: "violet-300" },
      { part: "light-shaft", x: 56, y: 16, scale: 0.6, paint: "white" },
      { part: "heal-glow", x: 50, y: 83, scale: 0.8 },
    ],
    particles: [
      { part: "motes", x: 54, y: 30, scale: 0.5, paint: "white" },
      { part: "sparks", x: 53, y: 32, scale: 0.8, paint: "violet-300" },
    ],
  },
};

/** Joint Filing (Meditative #9, Epic Field Spell): two brushes poised over one shared blank scroll on a scholar's desk, a smiling accountant with an abacus between a sunrise window and a sunset window, a hillside tax office courtyard, warm paper light. */
const JOINT_FILING: SceneRecipe = {
  family: "meditative",
  sky: ["slate-100", "red-300"],
  light: { x: 50, y: 44, r: 40, paint: "orange-400", opacity: 0.25 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 82,
    scale: 0.75,
    pose: "pose-hold",
    look: { head: "head-human", hair: "spectacles", outfit: "outfit-robe", item: null, skin: "red-300", cloth: "blue-800" },
  },
  layers: {
    sky: [
      { part: "sun", x: 22, y: 50, scale: 0.6 },
      { part: "sun", x: 78, y: 46, scale: 0.6, paint: "red-400" },
    ],
    far: [
      { part: "ink-peaks", x: 50, y: 54, scale: 0.5 },
      { part: "tiled-house", x: 50, y: 50, scale: 0.8 },
      { part: "ink-round-window", x: 22, y: 48, scale: 0.9 },
      { part: "ink-round-window", x: 78, y: 48, scale: 0.9 },
    ],
    mid: [
      { part: "tatami", x: 50, y: 66 },
      { part: "paper-screen", x: 50, y: 66, scale: 0.9 },
    ],
    subject: [{ part: "abacus", x: 62, y: 70, scale: 0.7 }],
    near: [
      { part: "wide-table", x: 50, y: 98, scale: 2.2, paint: "wood-700" },
      { part: "scroll", x: 50, y: 74, scale: 1.5 },
      { part: "brush", x: 43, y: 70, scale: 0.9 },
      { part: "brush", x: 57, y: 70, scale: 0.9, flip: -1 },
      { part: "ink-seal", x: 82, y: 76 },
    ],
  },
};

/** Double Counting (Meditative #10, Epic Field Spell): an abacus whose every bead has a twin reflected in still water, two identical clerks mirroring each other at one counting table, a lakeside counting house on stilts, soft doubled mist, morning. */
const DOUBLE_COUNTING: SceneRecipe = {
  family: "meditative",
  sky: ["slate-100", "blue-200"],
  light: { x: 50, y: 32, r: 38, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 70, size: 5, rotate: 0 },
  figure: {
    x: 33,
    y: 80,
    scale: 0.6,
    pose: "pose-hold",
    look: { head: "head-human", hair: "hair-short", outfit: "outfit-robe", item: "item-scroll", skin: "red-300", cloth: "slate-700" },
  },
  extras: [
    {
      layer: "subject",
      x: 67,
      y: 80,
      scale: 0.6,
      flip: -1,
      pose: "pose-hold",
      look: { head: "head-human", hair: "hair-short", outfit: "outfit-robe", item: "item-scroll", skin: "red-300", cloth: "slate-700" },
    },
  ],
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 50, scale: 0.6 },
      { part: "mist", x: 50, y: 44, scale: 0.9 },
      { part: "still-water", x: 50, y: 56 },
      { part: "fence", x: 50, y: 58, scale: 0.5, paint: "slate-700" },
      { part: "tiled-house", x: 50, y: 54, scale: 1.1 },
      { part: "mist", x: 50, y: 60, scale: 0.9 },
    ],
    mid: [{ part: "boards", x: 50, y: 70 }],
    subject: [{ part: "table", x: 50, y: 80, scale: 1.2, paint: "wood-700" }],
    near: [
      { part: "abacus", x: 30, y: 82, scale: 1 },
      { part: "abacus", x: 70, y: 82, scale: 1, flip: -1, paint: "slate-400" },
      { part: "ink-seal", x: 82, y: 70 },
    ],
  },
};

/** Double Header (Meditative #11, Legendary Field Spell): a mountainside arena carved into terraces where two matches play at once on twin fields, a spectator's hand holding up a torn ticket stub as a ghostly copy peels off it into a second hand, lanterns swaying, evening floodlight through drifting cloud. */
const DOUBLE_HEADER: SceneRecipe = {
  family: "meditative",
  sky: ["blue-900", "violet-500"],
  light: { x: 50, y: 30, r: 40, paint: "blue-200", opacity: 0.35 },
  emblem: { x: 40, y: 58, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "cloud", x: 26, y: 24, scale: 1.2, count: 2, spread: [8, 3] },
      { part: "cloud", x: 76, y: 28, scale: 1, paint: "slate-400" },
    ],
    far: [
      { part: "ink-peaks", x: 50, y: 52, scale: 0.8 },
      { part: "terraces", x: 50, y: 56 },
      { part: "amphitheatre", x: 30, y: 62, scale: 0.42 },
      { part: "amphitheatre", x: 70, y: 62, scale: 0.42 },
    ],
    mid: [
      { part: "ink-person", x: 30, y: 63, scale: 0.4, count: 4, spread: [8, 1] },
      { part: "ink-person", x: 70, y: 63, scale: 0.4, count: 4, spread: [8, 1] },
      { part: "lantern-string", x: 50, y: 34, scale: 1.4 },
    ],
    subject: [
      { part: "ticket-stub", x: 41, y: 73, scale: 1.4 },
      { part: "ticket-stub", x: 57, y: 66, scale: 1.4, paint: "blue-200" },
      { part: "hand-hold", x: 40, y: 77, scale: 1.6 },
      { part: "hand-open", x: 62, y: 70, scale: 1.5, flip: -1 },
    ],
    near: [{ part: "ink-seal", x: 84, y: 80 }],
    light: [
      { part: "light-shaft", x: 30, y: 18, scale: 0.9, paint: "white" },
      { part: "light-shaft", x: 70, y: 18, scale: 0.9, paint: "white" },
      { part: "halo", x: 58, y: 54, scale: 1.2, paint: "white" },
    ],
    particles: [
      { part: "motes", x: 52, y: 58, scale: 0.6, paint: "blue-200" },
      { part: "sparks", x: 50, y: 62, scale: 1, paint: "white" },
    ],
  },
};

/** Fear Mongerer (Meditative #12, Epic Unit): a gaunt man in a long grey coat pinching out a lantern flame, villagers on a stone bridge staring up as dusk falls far too early, a mountain village, the sun plunging behind the ridge. */
const FEAR_MONGERER: SceneRecipe = {
  family: "meditative",
  sky: ["violet-700", "red-400"],
  light: { x: 28, y: 58, r: 36, paint: "red-400", opacity: 0.35 },
  emblem: { x: 50, y: 54, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 1,
    pose: "pose-hold",
    look: { ...figureLookFor(["Human"]), hair: "hat-brim", outfit: "outfit-coat", item: "item-lantern", cloth: "slate-400" },
  },
  layers: {
    sky: [{ part: "sun", x: 28, y: 58, scale: 1, paint: "red-400" }],
    far: [
      { part: "ink-peaks", x: 50, y: 60, scale: 0.9 },
      { part: "tiled-house", x: 18, y: 64, scale: 0.6 },
      { part: "tiled-house", x: 82, y: 63, scale: 0.55 },
      { part: "pagoda", x: 90, y: 64, scale: 0.4 },
    ],
    mid: [
      { part: "bridge", x: 50, y: 70, scale: 0.9 },
      { part: "ink-person", x: 32, y: 70, scale: 0.8 },
      { part: "ink-person", x: 70, y: 70, scale: 0.8, flip: -1 },
      { part: "ink-person", x: 76, y: 70, scale: 0.75, flip: -1 },
    ],
    near: [
      { part: "mist", x: 50, y: 82, scale: 0.9 },
      { part: "smoke", x: 61, y: 52, scale: 0.4 },
      { part: "ink-seal", x: 80, y: 76 },
    ],
  },
};

/** Gatling Pea (Meditative #13, Epic Unit): a thick, faceless pea vine in pod armour coiled round a trellis, its pods ringed like spinning barrels, spraying hard peas, terraced vegetable beds, a farmhouse under turning autumn trees, harvest light. */
const GATLING_PEA: SceneRecipe = {
  family: "meditative",
  sky: ["red-300", "slate-100"],
  light: { x: 72, y: 30, r: 38, paint: "orange-400", opacity: 0.3 },
  emblem: { x: 46, y: 64, size: 6, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 48, scale: 0.5 },
      { part: "tiled-house", x: 76, y: 54, scale: 0.9 },
      { part: "tree", x: 64, y: 55, scale: 0.8, paint: "orange-400" },
      { part: "tree", x: 90, y: 55, scale: 0.8, paint: "red-600" },
    ],
    mid: [
      { part: "terraces", x: 50, y: 60 },
      { part: "sprout", x: 24, y: 68, count: 3, spread: [10, 2] },
      { part: "sprout", x: 76, y: 70, count: 3, spread: [10, 2] },
    ],
    subject: [{ part: "pea-vine", x: 46, y: 84, scale: 1.35 }],
    near: [
      { part: "leaves", x: 30, y: 40, scale: 1 },
      { part: "grass-tuft", x: 14, y: 86, scale: 1.2 },
      { part: "ink-seal", x: 18, y: 74 },
    ],
  },
};

/** Prime Time (Meditative #14, Epic Spell): a scholar's hand drawing one card from a fanned deck where only a scattered few glow, a water clock dripping in uneven beats, a temple bell tower ringing the hour, clear afternoon. */
const PRIME_TIME: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "slate-100"],
  light: { x: 70, y: 28, r: 36, paint: "white", opacity: 0.45 },
  emblem: { x: 58, y: 58, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 56, scale: 0.7 },
      { part: "ink-pavilion", x: 72, y: 58, scale: 1 },
      { part: "temple-bell", x: 72, y: 42, scale: 0.8 },
      { part: "echo-rings", x: 72, y: 47, scale: 0.8, paint: "slate-400" },
    ],
    mid: [
      { part: "tatami", x: 50, y: 68 },
      { part: "water-clock", x: 26, y: 78, scale: 1.2 },
    ],
    subject: [
      { part: "ink-card-fan", x: 46, y: 82, scale: 1.3 },
      { part: "halo", x: 40, y: 70, scale: 0.4, paint: "sky-300" },
      { part: "halo", x: 50, y: 68, scale: 0.4, paint: "sky-300" },
      { part: "card", x: 58, y: 64, scale: 0.9 },
      { part: "hand-hold", x: 58, y: 78, scale: 1.3 },
    ],
    near: [
      { part: "halo", x: 58, y: 58, scale: 0.6, paint: "white" },
      { part: "ink-seal", x: 82, y: 74 },
    ],
  },
};

export const MEDITATIVE_RECIPES_1: Readonly<Record<string, SceneRecipe>> = {
  "meditative-001": DISRUPTIVE_DISRUPTOR,
  "meditative-002": RAMPAGING_RHINO,
  "meditative-003": JLOCKWORK_MACHINE,
  "meditative-004": JUICY_KUMQUAT_MELON,
  "meditative-005": DEATH_BY_1000_CUTS,
  "meditative-006": ME_NO_LIKEY,
  "meditative-007": INTROSPECTION,
  "meditative-008": REACH_THE_SUMMIT,
  "meditative-009": JOINT_FILING,
  "meditative-010": DOUBLE_COUNTING,
  "meditative-011": DOUBLE_HEADER,
  "meditative-012": FEAR_MONGERER,
  "meditative-013": GATLING_PEA,
  "meditative-014": PRIME_TIME,
};
