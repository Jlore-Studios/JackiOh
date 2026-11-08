// R3392000: Core's scenes, part 3 of 3: Core #75 to #99, the tokens Super-Virus (#90-1), Combo-Fodder (#93-1) and Chaos Golem (#95-1), and the six core tokens (core-t-*), each drawn from its brief in the
// issue's table (#3392) in Core's family, the glass orb (ART.md, "Core" and "Core's scenes"). Each recipe keeps
// its subject within x 30 to 70 and y 25 to 85, its light from the upper left unless its brief names another
// source, and its emblem small on a prop or a chest; a Unit's character is a figure built on its tags' look, or
// a creature part in its subject layer. Each doc comment gives the brief in a line and any change made to fit
// the kits.

import { figureLookFor, KY_TUTOR_LOOK } from "./figureKit.ts";
import type { PaletteName, SceneRecipe, ScenePlacement } from "./scene.ts";
import { ROTUNDA_DOORS, type SceneryPartId } from "./sceneKit.ts";

/** Transmogulate (Core #83, Legendary Spell): a prismatic wave bursts from a cracked crucible across a counting house, crowning a hand of cards, a deck and a chess set as the alchemist shields his eyes. The pearl and rose light is a burst and rays; the hand, deck and chess set are placed fans, stacks and pieces with crowns above them. */
const TRANSMOGULATE: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "stone-500"],
  light: { x: 50, y: 58, r: 44, paint: "red-300", opacity: 0.5 },
  emblem: { x: 50, y: 72, size: 6, rotate: 0 },
  figure: {
    x: 24,
    y: 84,
    scale: 0.85,
    pose: "pose-raise",
    look: { ...figureLookFor(["Human"]), hair: "goggles", outfit: "outfit-labcoat", item: null, cloth: "slate-100" },
  },
  layers: {
    sky: [
      { part: "light-shaft", x: 22, y: 6, scale: 1.2, paint: "white" },
      { part: "chandelier", x: 50, y: 8, scale: 1.6 },
    ],
    far: [
      { part: "room-wall", x: 50, y: 66 },
      { part: "window", x: 22, y: 50, scale: 1.1 },
      { part: "shelf", x: 50, y: 44, scale: 1.2 },
    ],
    mid: [
      { part: "counter", x: 28, y: 72, scale: 1.1 },
      { part: "counter", x: 76, y: 72, scale: 1.1 },
      { part: "jar", x: 36, y: 62, scale: 0.8 },
    ],
    subject: [
      { part: "crucible", x: 50, y: 78, scale: 1.9 },
      { part: "wave", x: 50, y: 76, scale: 1.1, paint: "purple-400" },
      { part: "card-fan", x: 70, y: 52, scale: 1.2 },
      { part: "crown", x: 70, y: 40, scale: 1.4 },
      { part: "deck", x: 33, y: 52, scale: 1.8 },
      { part: "crown", x: 33, y: 42, scale: 1.4 },
      { part: "chess-piece", x: 54, y: 46, scale: 1.4 },
      { part: "crown", x: 54, y: 38, scale: 1.2 },
    ],
    near: [
      { part: "vial", x: 40, y: 86, scale: 1.4, count: 2, spread: [28, 3] },
      { part: "shards", x: 50, y: 82, scale: 1.4, count: 2, spread: [30, 4] },
      { part: "coin", x: 60, y: 90, scale: 1, count: 3, spread: [28, 2] },
    ],
    light: [
      { part: "burst", x: 50, y: 58, scale: 2.8, paint: "white" },
      { part: "god-rays", x: 50, y: 46, scale: 2.2, paint: "red-300" },
      { part: "rim-light", x: 50, y: 60, scale: 1.4, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 56, scale: 1.4, count: 2, spread: [24, 12] },
      { part: "confetti", x: 50, y: 50, scale: 1.4, paint: "red-300" },
      { part: "motes", x: 50, y: 60, scale: 1.4, paint: "white" },
    ],
  },
};

/** Unlicensed Experimentation (Core #85, Legendary Trap): a floor hatch springs open in a tiled lab and robotic clamps haul an enemy knight into a crackling fusion chamber that welds him to a hulking guardian, as a goggled scientist cackles by an office chair. The guardian stands in the pod; the chair's face is an eye on its back. */
const UNLICENSED_EXPERIMENTATION: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "blue-200"],
  light: { x: 54, y: 50, r: 40, paint: "green-400", opacity: 0.4 },
  emblem: { x: 78, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 48,
    y: 66,
    scale: 0.8,
    pose: "pose-lunge",
    look: { ...figureLookFor(["Human"]), hair: "steel-helmet", outfit: "outfit-plate", item: "item-rapier", cloth: "slate-400" },
  },
  cast: [
    { x: 74, y: 76, scale: 0.9, pose: "pose-guard", look: { ...figureLookFor(["Human"]), head: "head-robot", hair: null, outfit: "outfit-rock", item: "item-club", skin: "slate-400", cloth: "slate-700" }, layer: "mid" },
    { x: 20, y: 86, scale: 0.8, pose: "pose-raise", look: { ...figureLookFor(["Human"]), hair: "goggles", outfit: "outfit-labcoat", item: null, cloth: "white" }, layer: "near" },
  ],
  layers: {
    sky: [
      { part: "light-shaft", x: 30, y: 4, scale: 1.2, paint: "white" },
      { part: "bulbs", x: 50, y: 9, scale: 1.6 },
    ],
    far: [
      { part: "tile-wall", x: 50, y: 70 },
      { part: "window", x: 24, y: 52, scale: 0.9 },
      { part: "shelf", x: 82, y: 40, scale: 0.9 },
    ],
    mid: [
      { part: "pod", x: 76, y: 76, scale: 1.3 },
      { part: "chair", x: 34, y: 84, scale: 1.1 },
      { part: "eye", x: 34, y: 80, scale: 1.2 },
    ],
    subject: [
      { part: "hatch", x: 48, y: 80, scale: 1.5 },
      { part: "clamp", x: 42, y: 80, scale: 1.1 },
      { part: "clamp", x: 55, y: 80, scale: 1.1, flip: -1 },
      { part: "zap", x: 52, y: 56, scale: 0.9, paint: "green-400" },
    ],
    near: [
      { part: "vial", x: 60, y: 88, scale: 1.6, count: 2, spread: [24, 2] },
      { part: "shards", x: 44, y: 88, scale: 1.4, count: 2, spread: [30, 3] },
    ],
    light: [
      { part: "halo", x: 50, y: 56, scale: 1.6, paint: "green-400" },
      { part: "zap", x: 70, y: 50, scale: 0.8, paint: "green-400", flip: -1 },
      { part: "rim-light", x: 48, y: 56, scale: 1.1, paint: "green-200" },
    ],
    particles: [
      { part: "sparks", x: 60, y: 60, scale: 1.4, count: 2, spread: [24, 12], paint: "green-400" },
      { part: "motes", x: 50, y: 66, scale: 1.3, paint: "green-200" },
      { part: "steam", x: 78, y: 58, scale: 1.2, paint: "white" },
    ],
  },
};

/** Pocket Chaos (Core #87, Legendary Spell): two rival duellists face off in a hedge-maze gazebo as a velvet pocket bursts open between them and a vortex of dice and cards swaps their armies, one duellist's deck tearing from his belt toward the other. */
const POCKET_CHAOS: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 50, y: 48, r: 40, paint: "white", opacity: 0.55 },
  emblem: { x: 50, y: 66, size: 5, rotate: 0 },
  figure: {
    x: 26,
    y: 84,
    scale: 0.95,
    pose: "pose-lunge",
    look: { ...figureLookFor(["Human"]), hair: "cap-feather", outfit: "outfit-blazer", item: "item-rapier", cloth: "blue-700" },
  },
  cast: [
    { x: 74, y: 84, scale: 0.95, flip: -1, pose: "pose-lunge", look: { ...figureLookFor(["Human"]), hair: "hair-long", outfit: "outfit-waistcoat", item: "item-rapier", cloth: "red-600" }, layer: "subject" },
  ],
  layers: {
    sky: [
      { part: "cloud", x: 18, y: 16, scale: 1.1 },
    ],
    far: [
      { part: "hedge", x: 10, y: 66, scale: 1.8 },
      { part: "hedge", x: 90, y: 66, scale: 1.8 },
      { part: "hedge", x: 50, y: 62, scale: 1.4, count: 3, spread: [28, 0] },
      { part: "pavilion", x: 50, y: 66, scale: 1.7 },
    ],
    mid: [
      { part: "field", x: 50, y: 70 },
      { part: "soldier", x: 36, y: 76, scale: 0.9, },
      { part: "soldier", x: 64, y: 76, scale: 0.9, flip: -1 },
    ],
    subject: [
      { part: "pocket", x: 50, y: 56, scale: 2.4 },
      { part: "vortex", x: 50, y: 58, scale: 2.2 },
      { part: "deck", x: 36, y: 60, scale: 1.6 },
      { part: "card", x: 42, y: 56, scale: 1.2, count: 3, spread: [8, 6] },
      { part: "streaks", x: 46, y: 62, scale: 0.8 },
    ],
    near: [
      { part: "dice", x: 50, y: 66, scale: 2.2, count: 3, spread: [26, 12] },
      { part: "card", x: 50, y: 72, scale: 1.4, count: 2, spread: [30, 6] },
      { part: "hedge", x: 8, y: 94, scale: 1.4 },
      { part: "hedge", x: 92, y: 94, scale: 1.4 },
    ],
    light: [
      { part: "burst", x: 50, y: 56, scale: 2.6, paint: "white" },
      { part: "god-rays", x: 50, y: 40, scale: 2, paint: "white" },
      { part: "zap", x: 50, y: 50, scale: 0.9, paint: "white" },
    ],
    particles: [
      { part: "leaves", x: 50, y: 56, scale: 1.8, count: 2, spread: [16, 8] },
      { part: "sparks", x: 50, y: 52, scale: 1.4, count: 2, spread: [24, 10] },
      { part: "motes", x: 50, y: 58, scale: 1.4, paint: "white" },
    ],
  },
};

/** Felinor Fiender (Core #92, Legendary Unit): a tall man in a long coat beneath lit townhouse windows at dusk lifts binoculars as cat-folk tumble down a drainpipe and stack onto his shoulders, his shadow swelling. The cat-folk are cat parts on his shoulders and the pipe a pale column; the shadow a dark cloud on the street. */
const FELINOR_FIENDER: SceneRecipe = {
  family: "core",
  sky: ["blue-800", "orange-400"],
  light: { x: 80, y: 30, r: 40, paint: "violet-300", opacity: 0.45 },
  emblem: { x: 50, y: 64, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 1.05,
    pose: "pose-hold",
    look: { ...figureLookFor(["Human"]), hair: "hat-brim", outfit: "outfit-coat", item: "item-binoculars", cloth: "slate-800" },
  },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 10, scale: 1.4 },
      { part: "cloud", x: 24, y: 24, scale: 1.2, paint: "violet-300" },
      { part: "moon", x: 82, y: 14, scale: 0.7 },
    ],
    far: [
      { part: "skyline", x: 50, y: 56, scale: 1.3, paint: "slate-800" },
      { part: "townhouse", x: 50, y: 74, scale: 1.8 },
    ],
    mid: [
      { part: "street", x: 50, y: 76 },
      { part: "column", x: 20, y: 76, scale: 1.1, paint: "slate-400" },
      { part: "cat", x: 20, y: 52, scale: 1.1 },
      { part: "streetlamp", x: 12, y: 82, scale: 1.1 },
      { part: "streetlamp", x: 88, y: 82, scale: 1.1, flip: -1 },
    ],
    subject: [
      { part: "cat", x: 48, y: 36, scale: 1.3 },
      { part: "cat", x: 52, y: 31, scale: 1.2, flip: -1 },
      { part: "cat", x: 49, y: 26, scale: 1.1 },
      { part: "cat", x: 64, y: 62, scale: 1 },
    ],
    near: [
      { part: "cloud", x: 30, y: 90, scale: 1.8, paint: "slate-900" },
      { part: "leaves", x: 80, y: 84, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 12, y: 54, scale: 1.4, paint: "violet-300" },
      { part: "halo", x: 88, y: 54, scale: 1.4, paint: "violet-300" },
      { part: "rim-light", x: 50, y: 58, scale: 1.2, paint: "violet-300" },
    ],
    particles: [
      { part: "leaves", x: 50, y: 60, scale: 1.8, count: 2, spread: [20, 10] },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "violet-300" },
      { part: "embers", x: 20, y: 70, scale: 1.2, paint: "orange-400" },
    ],
  },
};

/** Combo-Index (Core #93, Legendary Field Spell): a glass stadium of stacked rings lighting from dim to blazing as a whirlwind of played cards spirals up, a lone performer on the central stage under spotlights. The cards' whirl is a vortex with cards and a fan (a ring of cards is a Mythic's). */
const COMBO_INDEX: SceneRecipe = {
  family: "core",
  sky: ["slate-900", "blue-900"],
  light: { x: 50, y: 28, r: 42, paint: "cyan-400", opacity: 0.5 },
  emblem: { x: 50, y: 83, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 78,
    scale: 0.7,
    pose: "pose-spread",
    look: { ...figureLookFor(["Human"]), hair: "hat-brim", outfit: "outfit-blazer", item: "item-card", cloth: "violet-700" },
  },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 12, scale: 2 },
      { part: "nebula", x: 24, y: 20 },
    ],
    far: [
      { part: "skyline", x: 50, y: 62, scale: 1.2, paint: "slate-800" },
      { part: "stadium", x: 50, y: 70, scale: 1 },
    ],
    mid: [
      { part: "cobbles", x: 50, y: 72 },
      { part: "crowd", x: 50, y: 78, scale: 1.2 },
      { part: "bulbs", x: 50, y: 20, scale: 1.4 },
    ],
    subject: [
      { part: "stage", x: 50, y: 78 },
      { part: "vortex", x: 50, y: 48, scale: 2.2, paint: "cyan-400" },
      { part: "card-fan", x: 38, y: 62, scale: 1.2 },
      { part: "card", x: 50, y: 50, scale: 1.3, count: 6, spread: [26, 22] },
    ],
    near: [
      { part: "spotlight", x: 12, y: 4, scale: 1.4 },
      { part: "spotlight", x: 88, y: 4, scale: 1.4, flip: -1 },
      { part: "deck", x: 22, y: 88, scale: 1.4 },
      { part: "deck", x: 78, y: 88, scale: 1.4 },
    ],
    light: [
      { part: "god-rays", x: 50, y: 6, scale: 2.4, paint: "cyan-400" },
      { part: "halo", x: 50, y: 30, scale: 1.8, paint: "white" },
      { part: "rim-light", x: 50, y: 62, scale: 0.9, paint: "cyan-400" },
    ],
    particles: [
      { part: "confetti", x: 50, y: 44, scale: 2, count: 2, spread: [14, 10] },
      { part: "sparks", x: 50, y: 34, scale: 1.6, count: 3, spread: [24, 10] },
      { part: "motes", x: 50, y: 56, scale: 1.4, paint: "cyan-400" },
    ],
  },
};

/** Call to Chaos, Core Edition (Core #95, Legendary Spell): in a crystal casino hall a cloaked figure yanks the lever of a towering chaos wheel and a golem's fist, a stampede of sprinters, a shower of cards and a healing fountain erupt from its reels under magenta light. */
const CALL_TO_CHAOS: SceneRecipe = {
  family: "core",
  sky: ["slate-800", "violet-700"],
  light: { x: 50, y: 46, r: 44, paint: "purple-400", opacity: 0.55 },
  emblem: { x: 50, y: 56, size: 5, rotate: 0 },
  figure: {
    x: 22,
    y: 86,
    scale: 0.9,
    pose: "pose-raise",
    look: { ...figureLookFor(["Human"]), hair: "hood", outfit: "outfit-coat", item: null, cloth: "violet-700" },
  },
  layers: {
    sky: [
      { part: "chandelier", x: 50, y: 6, scale: 1.8 },
      { part: "stars", x: 50, y: 20, scale: 1.8 },
    ],
    far: [
      { part: "glass-wall", x: 14, y: 70, scale: 1.5 },
      { part: "glass-wall", x: 86, y: 70, scale: 1.5 },
      { part: "checker-floor", x: 50, y: 70 },
    ],
    mid: [
      { part: "bulbs", x: 50, y: 22, scale: 1.8 },
      { part: "fountain", x: 82, y: 80, scale: 1.3 },
      { part: "rush-token", x: 66, y: 82, scale: 1.2, flip: -1, count: 2, spread: [10, 4] },
    ],
    subject: [
      { part: "chaos-wheel", x: 50, y: 78, scale: 1.45 },
      { part: "fist", x: 70, y: 52, scale: 2 },
      { part: "card", x: 36, y: 44, scale: 1.4, count: 4, spread: [8, 14] },
    ],
    near: [
      { part: "dice", x: 60, y: 92, scale: 2, count: 3, spread: [30, 2] },
      { part: "coin", x: 30, y: 94, scale: 1.4, count: 3, spread: [40, 2] },
      { part: "shockwave", x: 66, y: 52, scale: 1.1 },
    ],
    light: [
      { part: "burst", x: 50, y: 46, scale: 2.4, paint: "purple-400" },
      { part: "god-rays", x: 50, y: 28, scale: 2.2, paint: "red-300" },
      { part: "heal-glow", x: 82, y: 70, scale: 1.2 },
    ],
    particles: [
      { part: "confetti", x: 50, y: 44, scale: 1.8, paint: "red-300" },
      { part: "sparks", x: 50, y: 40, scale: 1.6, count: 2, spread: [22, 10], paint: "purple-400" },
      { part: "motes", x: 50, y: 56, scale: 1.4, paint: "white" },
    ],
  },
};

/** My Pawn (Core #96, Mythic Trap): a vast glass chessboard floats above a mirrored sea; an enemy king's sword freezes inches from a fallen hero as puppet strings snap taut from the sky to every enemy piece and a gloved hand tilts one finger. The hooded mastermind is the gloved hand alone (its hood a small figure's, far above), the king a small armoured figure. */
const MY_PAWN: SceneRecipe = {
  family: "core",
  sky: ["slate-900", "blue-900"],
  light: { x: 50, y: 14, r: 44, paint: "violet-300", opacity: 0.5 },
  emblem: { x: 50, y: 58, size: 5, rotate: 0 },
  cast: [
    { x: 56, y: 64, scale: 0.55, flip: -1, pose: "pose-lunge", look: { ...figureLookFor(["Human"]), hair: "steel-helmet", outfit: "outfit-plate", item: "item-sword", cloth: "slate-700" }, layer: "subject" },
    { x: 50, y: 20, scale: 0.4, pose: "pose-stand", look: { head: "head-void", hair: "hood", outfit: "outfit-void", item: null, skin: "black", cloth: "slate-900" }, layer: "far" },
  ],
  layers: {
    sky: [
      { part: "stars", x: 24, y: 24, scale: 2, count: 2, spread: [50, 10] },
      { part: "nebula", x: 78, y: 30, scale: 1.2 },
      { part: "moon", x: 84, y: 14, scale: 0.6 },
    ],
    far: [
      { part: "water", x: 50, y: 70 },
      { part: "skyline", x: 50, y: 70, scale: 1, paint: "slate-800" },
      { part: "cloud", x: 16, y: 56, scale: 1.2, paint: "violet-300" },
      { part: "cloud", x: 86, y: 52, scale: 1.2, paint: "violet-300" },
    ],
    mid: [
      { part: "chessboard", x: 50, y: 66, scale: 1.15 },
      { part: "chess-piece", x: 32, y: 62, scale: 1.4, count: 2, spread: [12, 6] },
      { part: "chess-piece", x: 66, y: 66, scale: 1.4, count: 2, spread: [12, 6] },
      { part: "soldier", x: 26, y: 70, scale: 0.8, flip: 1 },
      { part: "soldier", x: 74, y: 70, scale: 0.8, flip: -1 },
    ],
    subject: [
      { part: "puppet-hand", x: 50, y: 2, scale: 1.1 },
      { part: "strings", x: 50, y: 40, scale: 1.2, paint: "slate-100" },
      { part: "soldier-fallen", x: 42, y: 67, scale: 1.4 },
      { part: "chess-piece", x: 54, y: 52, scale: 1.8 },
    ],
    near: [
      { part: "chess-piece", x: 16, y: 92, scale: 2.2 },
      { part: "chess-piece", x: 84, y: 90, scale: 2.4 },
      { part: "shards", x: 50, y: 82, scale: 1.6, count: 2, spread: [34, 4] },
    ],
    light: [
      { part: "light-shaft", x: 50, y: 2, scale: 1.4, paint: "violet-300" },
      { part: "rim-light", x: 50, y: 60, scale: 1.2, paint: "slate-100" },
      { part: "halo", x: 50, y: 66, scale: 1.8, paint: "violet-500" },
    ],
    particles: [
      { part: "shards", x: 50, y: 40, scale: 1.6, count: 3, spread: [30, 14] },
      { part: "motes", x: 50, y: 50, scale: 1.6, paint: "violet-300" },
      { part: "sparks", x: 50, y: 56, scale: 1.4, count: 2, spread: [24, 8], paint: "slate-100" },
    ],
  },
};

/** Zephrys (Core #97, Mythic Spell): a towering wind spirit of cloud and sky-blue silk uncoils from a cyclone above a desert mesa, one finger to his chin, a hundred cards whirling round him in rings and one brilliant card descending into an outstretched hand far below, a rainbow in the distance. */
const ZEPHRYS: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 50, y: 36, r: 44, paint: "white", opacity: 0.6 },
  emblem: { x: 50, y: 50, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 64,
    scale: 0.9,
    pose: "pose-hold",
    look: { head: "head-cloud", hair: null, outfit: "outfit-robe", item: null, skin: "white", cloth: "sky-300" },
  },
  layers: {
    sky: [
      { part: "rainbow", x: 74, y: 70, scale: 1.1 },
      { part: "cloud", x: 18, y: 18, scale: 1.3 },
      { part: "cloud", x: 84, y: 28, scale: 1.1 },
    ],
    far: [
      { part: "mesa", x: 20, y: 72, scale: 1.4 },
      { part: "mesa", x: 86, y: 70, scale: 1, flip: -1 },
      { part: "standing-stone", x: 62, y: 72, scale: 1.2 },
    ],
    mid: [
      { part: "sand", x: 50, y: 72 },
      { part: "gust", x: 28, y: 50, scale: 1.2 },
      { part: "gust", x: 74, y: 44, scale: 1.2 },
      { part: "streaks", x: 78, y: 60, scale: 0.9 },
      { part: "gull", x: 70, y: 20, scale: 1.2 },
    ],
    subject: [
      { part: "cyclone", x: 50, y: 82, scale: 1.4 },
      { part: "card-ring", x: 50, y: 40, scale: 1.9 },
      { part: "card-ring", x: 50, y: 62, scale: 1.5 },
      { part: "card-ring", x: 50, y: 78, scale: 1.1 },
      { part: "card-ring", x: 50, y: 28, scale: 1.2 },
      { part: "card-ring", x: 50, y: 52, scale: 1.7 },
    ],
    near: [
      { part: "open-hand", x: 44, y: 90, scale: 1.8 },
      { part: "card", x: 50, y: 86, scale: 2, paint: "white" },
      { part: "dust", x: 20, y: 90, scale: 1.4 },
      { part: "dust", x: 82, y: 90, scale: 1.4 },
      { part: "shockwave", x: 50, y: 92, scale: 1.4 },
    ],
    light: [
      { part: "god-rays", x: 50, y: 20, scale: 2.4, paint: "white" },
      { part: "halo", x: 50, y: 82, scale: 1.2, paint: "white" },
      { part: "lightning", x: 66, y: 20, scale: 0.8, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 70, scale: 1.4, count: 3, spread: [24, 12] },
      { part: "motes", x: 50, y: 62, scale: 1.6, paint: "white" },
      { part: "card", x: 50, y: 50, scale: 0.9, count: 7, spread: [38, 28] },
    ],
  },
};

const HEROIC_AT = { x: 50, y: 56, s: 1.2 } as const;
/** A small icon placed in doorway `i` of Heroic Power's rotunda (its centre offset from the rotunda's anchor, times its scale). */
const inDoor = (i: number, part: SceneryPartId, scale: number, dy = 0, paint?: PaletteName): ScenePlacement => {
  const [dx, oy] = ROTUNDA_DOORS[i] ?? [0, 0];
  return { part, x: HEROIC_AT.x + dx * HEROIC_AT.s, y: HEROIC_AT.y + (oy + dy) * HEROIC_AT.s, scale, ...(paint === undefined ? {} : { paint }) };
};

/** Heroic Power (Core #98, Mythic Field Spell): a white marble rotunda of thirteen arched doorways, each glowing with its own power (a map, a bleeding palm, a bow, a ranch gate, a cat cafe, a dart, a stitched doll, a shield, a burning insect, a chalkboard, a fruit tree, a trapdoor, an eye), an unbreakable crystal pedestal at the centre holding one orb as a hero steps through, rainbow light. */
const HEROIC_POWER: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 50, y: 40, r: 46, paint: "white", opacity: 0.55 },
  emblem: { x: 50, y: 78, size: 5, rotate: 0 },
  figure: {
    x: 28,
    y: 90,
    scale: 0.8,
    pose: "pose-lunge",
    look: { ...figureLookFor(["Human"]), hair: "steel-helmet", outfit: "outfit-plate", item: "item-sword", cloth: "blue-700" },
  },
  layers: {
    sky: [
      { part: "rainbow", x: 50, y: 52, scale: 1.5 },
      { part: "cloud", x: 14, y: 14, scale: 1.2 },
      { part: "cloud", x: 86, y: 18, scale: 1 },
    ],
    far: [
      { part: "hills", x: 50, y: 56 },
      { part: "rotunda", x: HEROIC_AT.x, y: HEROIC_AT.y, scale: HEROIC_AT.s },
    ],
    mid: [
      inDoor(0, "map", 0.9),
      inDoor(1, "open-hand", 0.6, 3),
      inDoor(1, "drops", 0.5, 0),
      inDoor(2, "bow", 0.8),
      inDoor(3, "gate", 0.45, 5),
      inDoor(4, "cat", 0.9, 4),
      inDoor(4, "cup", 0.8, 5),
      inDoor(5, "arrow", 0.4),
      inDoor(7, "doll", 1, 5),
      inDoor(8, "shield", 0.8),
      inDoor(9, "bee", 0.8, -2),
      inDoor(9, "flame", 0.7, 5),
      inDoor(10, "blackboard", 0.3, 5),
      inDoor(11, "tree", 0.5, 6),
      inDoor(12, "hatch", 0.35, 6),
      inDoor(6, "eye", 0.9, -22),
    ],
    subject: [
      { part: "pedestal", x: 50, y: 78, scale: 1.6 },
      { part: "crystal", x: 50, y: 70, scale: 1.3 },
      { part: "orb", x: 50, y: 64, scale: 0.9 },
    ],
    near: [
      { part: "cobbles", x: 50, y: 82 },
      { part: "column", x: 10, y: 96, scale: 1.4 },
      { part: "column", x: 90, y: 96, scale: 1.4 },
    ],
    light: [
      { part: "halo", x: 50, y: 64, scale: 1.4, paint: "white" },
      { part: "god-rays", x: 50, y: 20, scale: 2.4, paint: "sky-300" },
      { part: "rim-light", x: 50, y: 70, scale: 1, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 60, scale: 1.6, count: 2, spread: [30, 10] },
      { part: "motes", x: 50, y: 50, scale: 1.6, paint: "white" },
      { part: "confetti", x: 50, y: 70, scale: 1.6, paint: "sky-300" },
    ],
  },
};

/** Craft a Card (Core #99, Mythic Spell): on a floating island workshop two creature-cards march along glowing rails from opposite sides into one hammer strike of blinding white light, welding into a hulking hybrid beast that leaps out as a fresh card into a craftsman's palm, blue sparks spraying, gears turning, glue pots toppling. The crystal forge is the anvil and the island. */
const CRAFT_A_CARD: SceneRecipe = {
  family: "core",
  sky: ["blue-800", "sky-400"],
  light: { x: 50, y: 50, r: 44, paint: "white", opacity: 0.55 },
  emblem: { x: 50, y: 76, size: 5, rotate: 0 },
  figure: {
    x: 78,
    y: 72,
    scale: 0.65,
    pose: "pose-spread",
    look: { ...figureLookFor(["Human"]), hair: "bucket-hat", outfit: "outfit-apron", item: null, cloth: "orange-400" },
  },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 14, scale: 2 },
      { part: "cloud", x: 16, y: 80, scale: 1.4 },
      { part: "cloud", x: 86, y: 86, scale: 1.2 },
    ],
    far: [
      { part: "gear", x: 18, y: 22, scale: 2.2 },
      { part: "gear", x: 82, y: 20, scale: 1.6 },
      { part: "planet", x: 88, y: 40, scale: 0.7 },
    ],
    mid: [
      { part: "floating-island", x: 50, y: 66, scale: 1.3 },
      { part: "rails", x: 50, y: 70 },
      { part: "jar", x: 26, y: 72, scale: 1, count: 2, spread: [8, 0] },
      { part: "crystal", x: 24, y: 70, scale: 1.4, count: 3, spread: [10, 2] },
    ],
    subject: [
      { part: "anvil", x: 50, y: 70, scale: 1.8 },
      { part: "card-imp", x: 30, y: 62, scale: 1.8 },
      { part: "card-imp", x: 70, y: 62, scale: 1.8, flip: -1 },
      { part: "beast", x: 50, y: 52, scale: 1.8 },
      { part: "hammer", x: 50, y: 28, scale: 1.8 },
    ],
    near: [
      { part: "open-hand", x: 72, y: 90, scale: 1.4, flip: -1 },
      { part: "card", x: 62, y: 86, scale: 1.6 },
      { part: "gear", x: 16, y: 90, scale: 1.4 },
      { part: "jar", x: 36, y: 90, scale: 1.4 },
    ],
    light: [
      { part: "burst", x: 50, y: 50, scale: 3, paint: "white" },
      { part: "god-rays", x: 50, y: 40, scale: 2.4, paint: "white" },
      { part: "shockwave", x: 50, y: 56, scale: 1.8 },
    ],
    particles: [
      { part: "sparks", x: 50, y: 56, scale: 1.6, count: 3, spread: [28, 14], paint: "cyan-400" },
      { part: "motes", x: 50, y: 50, scale: 1.6, paint: "sky-300" },
      { part: "steam", x: 50, y: 46, scale: 1.2, paint: "white" },
    ],
  },
};

/** Infinite Reserves (Core #75, Rare Field Spell): an endless warehouse back room, shelves of boxed cards stretching to a vanishing point, a little Rush Token jogging out of an aisle. */
const INFINITE_RESERVES: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "stone-500"],
  light: { x: 50, y: 40, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 52, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "bulbs", x: 50, y: 8, scale: 1.4 }],
    far: [{ part: "aisle", x: 50, y: 78 }],
    mid: [
      { part: "shelf", x: 14, y: 82, scale: 1.2 },
      { part: "shelf", x: 86, y: 82, scale: 1.2 },
      { part: "crate", x: 30, y: 82, scale: 1.4, count: 2, spread: [4, 0] },
      { part: "card", x: 68, y: 82, scale: 1.2, count: 2, spread: [6, 0] },
    ],
    subject: [{ part: "rush-token", x: 50, y: 82, scale: 1.6 }],
    near: [
      { part: "deck", x: 18, y: 92, scale: 1.6 },
      { part: "crate", x: 84, y: 92, scale: 1.6 },
    ],
    light: [{ part: "light-shaft", x: 50, y: 6, scale: 1.4, paint: "white" }],
    particles: [{ part: "motes", x: 50, y: 56, scale: 1.4, paint: "white" }],
  },
};

/** Field of Dreams (Core #76, Rare Spell): a moonlit meadow of tall pale flowers where ghostly old cards drift up from the blooms into a waiting hand. */
const FIELD_OF_DREAMS: SceneRecipe = {
  family: "core",
  sky: ["slate-900", "blue-900"],
  light: { x: 30, y: 20, r: 36, paint: "blue-200", opacity: 0.4 },
  emblem: { x: 50, y: 70, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "moon", x: 26, y: 20, scale: 1 },
      { part: "stars", x: 60, y: 22, scale: 1.6 },
    ],
    far: [
      { part: "hills", x: 50, y: 64, paint: "blue-900" },
      { part: "tree", x: 84, y: 62, scale: 1 },
    ],
    mid: [
      { part: "field", x: 50, y: 70 },
      { part: "flower", x: 20, y: 74, scale: 1.4, count: 4, spread: [60, 6] },
      { part: "ghost", x: 70, y: 70, scale: 0.8 },
    ],
    subject: [
      { part: "open-hand", x: 50, y: 88, scale: 2 },
      { part: "card", x: 50, y: 74, scale: 2, paint: "slate-100" },
      { part: "card", x: 40, y: 66, scale: 1.4, paint: "blue-200" },
      { part: "card", x: 61, y: 58, scale: 1.2, paint: "blue-200" },
    ],
    near: [{ part: "flower", x: 12, y: 94, scale: 2, count: 2, spread: [76, 3] }],
    light: [{ part: "halo", x: 50, y: 74, scale: 1.4, paint: "blue-200" }],
    particles: [{ part: "motes", x: 50, y: 56, scale: 1.6, count: 2, spread: [14, 10], paint: "blue-200" }],
  },
};

/** Professor Curvature (Core #77, Rare Unit): a tweedy professor bending a glowing chalk bell curve over his head like a rainbow, under a bright observatory dome. */
const PROFESSOR_CURVATURE: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "slate-100"],
  light: { x: 34, y: 28, r: 36, paint: "white", opacity: 0.5 },
  emblem: { x: 44, y: 66, size: 5, rotate: 0 },
  figure: {
    x: 44,
    y: 88,
    scale: 1,
    pose: "pose-raise",
    look: { ...figureLookFor(["Human"]), hair: "spectacles", outfit: "outfit-waistcoat", item: "item-chalk", cloth: "wood-700" },
  },
  layers: {
    sky: [{ part: "cloud", x: 80, y: 16, scale: 1 }],
    far: [
      { part: "dome", x: 50, y: 76, scale: 1.8 },
      { part: "telescope", x: 82, y: 78, scale: 1.6 },
    ],
    mid: [
      { part: "blackboard", x: 80, y: 86, scale: 0.8 },
      { part: "stars", x: 22, y: 40, scale: 0.8 },
    ],
    subject: [{ part: "bell-curve", x: 44, y: 42, scale: 1.2, paint: "cyan-400" }],
    near: [{ part: "notebook", x: 18, y: 92, scale: 1.4 }],
    light: [{ part: "halo", x: 44, y: 28, scale: 1.4, paint: "cyan-400" }],
    particles: [{ part: "sparks", x: 44, y: 36, scale: 1.2, count: 2, spread: [22, 6], paint: "cyan-400" }],
  },
};

/** /fullsend (Core #78, Epic Spell): a daredevil on a rocket-powered cart hurls cards ahead like confetti down a road through rolling hills, blue mana fuel splashing, a cliff edge and open sky behind where burning cards drift. */
const FULLSEND: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 28, y: 22, r: 38, paint: "white", opacity: 0.5 },
  emblem: { x: 40, y: 82, size: 5, rotate: 0 },
  figure: {
    x: 42,
    y: 76,
    scale: 0.85,
    pose: "pose-lunge",
    look: { ...figureLookFor(["Human"]), hair: "goggles", outfit: "outfit-bomber", item: "item-card", cloth: "red-600" },
  },
  layers: {
    sky: [
      { part: "cloud", x: 24, y: 20, scale: 1.2 },
      { part: "cloud", x: 78, y: 16, scale: 1 },
    ],
    far: [
      { part: "cliff", x: 16, y: 56, scale: 1.2 },
      { part: "card", x: 60, y: 24, scale: 1.2, count: 3, spread: [24, 10], paint: "orange-400" },
      { part: "flame", x: 56, y: 30, scale: 0.9, count: 2, spread: [20, 8] },
    ],
    mid: [
      { part: "hills", x: 50, y: 60 },
      { part: "road", x: 56, y: 62 },
      { part: "pine", x: 90, y: 62, scale: 1 },
    ],
    subject: [{ part: "rocket-cart", x: 42, y: 82, scale: 2.1 }],
    near: [
      { part: "spray", x: 28, y: 84, scale: 1.3, paint: "cyan-400" },
      { part: "streaks", x: 26, y: 78, scale: 1 },
    ],
    light: [{ part: "halo", x: 28, y: 80, scale: 1.3, paint: "cyan-400" }],
    particles: [
      { part: "confetti", x: 66, y: 66, scale: 1.4 },
      { part: "card", x: 70, y: 70, scale: 1, count: 4, spread: [18, 10] },
    ],
  },
};

/** Twinspell (Core #79, Rare Field Spell): a twin-towered belfry where two identical bells ring in unison, rings of blue echo-light rippling out over a bright plaza. */
const TWINSPELL: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "slate-100"],
  light: { x: 30, y: 24, r: 36, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 72, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 20, y: 18, scale: 1.2 }],
    far: [
      { part: "hills", x: 50, y: 66 },
      { part: "tree", x: 10, y: 70, scale: 1.2 },
      { part: "tree", x: 90, y: 70, scale: 1.2 },
    ],
    mid: [{ part: "cobbles", x: 50, y: 76 }],
    subject: [
      { part: "belfry", x: 50, y: 78, scale: 1.4 },
      { part: "bell", x: 31.8, y: 46.5, scale: 1.2 },
      { part: "bell", x: 68.2, y: 46.5, scale: 1.2 },
    ],
    near: [{ part: "flower", x: 20, y: 94, scale: 1.6, count: 3, spread: [60, 2] }],
    light: [
      { part: "echo-rings", x: 31.8, y: 48, scale: 1.4, paint: "cyan-400" },
      { part: "echo-rings", x: 68.2, y: 48, scale: 1.4, paint: "cyan-400" },
    ],
    particles: [{ part: "sparks", x: 50, y: 44, scale: 1.4, count: 2, spread: [30, 6], paint: "cyan-400" }],
  },
};

/** Zao Gao (Core #80, Rare Spell): a steaming plate of rice cakes in a bright dim-sum kitchen, one cake hopping off and sprouting into a sprinting little warrior with a shield. */
const ZAO_GAO: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "stone-500"],
  light: { x: 36, y: 30, r: 36, paint: "white", opacity: 0.5 },
  emblem: { x: 36, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 66,
    y: 74,
    scale: 0.5,
    pose: "pose-run",
    look: { ...figureLookFor(["CN"]), hair: "headband", outfit: "outfit-tunic", item: "item-shield", cloth: "green-700" },
  },
  layers: {
    sky: [{ part: "bulbs", x: 50, y: 8, scale: 1.2 }],
    far: [
      { part: "room-wall", x: 50, y: 68 },
      { part: "shelf", x: 22, y: 50, scale: 1 },
      { part: "window", x: 78, y: 54, scale: 0.9 },
    ],
    mid: [
      { part: "counter", x: 50, y: 78, scale: 1.5 },
      { part: "jar", x: 84, y: 66, scale: 1.2 },
      { part: "pan", x: 16, y: 64, scale: 1.2 },
    ],
    subject: [
      { part: "rice-cake", x: 36, y: 70, scale: 1.9 },
      { part: "steam", x: 36, y: 56, scale: 1.4 },
    ],
    near: [{ part: "cup", x: 14, y: 92, scale: 2 }],
    light: [{ part: "halo", x: 66, y: 62, scale: 0.8, paint: "white" }],
    particles: [{ part: "sparks", x: 52, y: 66, scale: 1.2, count: 2, spread: [14, 6] }],
  },
};

/** Radiant Saintess (Core #81, Epic Unit): a serene saintess in white robes dissolving into silver-white light as she falls, kneeling soldiers whose armour begins to shine, a clean marble cathedral behind with tall clear windows. */
const RADIANT_SAINTESS: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "blue-200"],
  light: { x: 50, y: 40, r: 40, paint: "white", opacity: 0.6 },
  emblem: { x: 50, y: 56, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 80,
    scale: 1,
    pose: "pose-spread",
    look: { ...figureLookFor(["Human"]), hair: "hair-long", outfit: "outfit-robe", item: null, cloth: "white" },
  },
  layers: {
    sky: [{ part: "light-shaft", x: 50, y: 4, scale: 1.4, paint: "white" }],
    far: [
      { part: "tile-wall", x: 50, y: 70 },
      { part: "window", x: 22, y: 56, scale: 1.4 },
      { part: "window", x: 78, y: 56, scale: 1.4 },
      { part: "column", x: 8, y: 70, scale: 1.4 },
      { part: "column", x: 92, y: 70, scale: 1.4 },
      { part: "arch", x: 50, y: 70, scale: 1.2 },
    ],
    mid: [
      { part: "soldier", x: 22, y: 80, scale: 1.7, flip: 1, count: 2, spread: [10, 3] },
      { part: "soldier", x: 78, y: 80, scale: 1.7, flip: -1, count: 2, spread: [-10, 3] },
    ],
    subject: [{ part: "halo", x: 50, y: 50, scale: 1.3, paint: "white" }],
    near: [{ part: "shards", x: 50, y: 88, scale: 1.4, count: 2, spread: [30, 3], paint: "white" }],
    light: [
      { part: "god-rays", x: 50, y: 30, scale: 1.8, paint: "white" },
      { part: "rim-light", x: 50, y: 56, scale: 1.2, paint: "white" },
    ],
    particles: [
      { part: "motes", x: 50, y: 60, scale: 1.6, count: 2, spread: [16, 10], paint: "white" },
      { part: "sparks", x: 50, y: 56, scale: 1.2, count: 2, spread: [24, 10] },
    ],
  },
};

/** KY's Trial (Core #82, Rare Spell): KY smiling knowingly behind an examiner's desk as three sealed envelopes float before a nervous student in a vaulted exam hall. */
const KYS_TRIAL: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "stone-500"],
  light: { x: 50, y: 30, r: 38, paint: "white", opacity: 0.45 },
  emblem: { x: 56, y: 70, size: 5, rotate: 0 },
  figure: { x: 58, y: 80, scale: 0.8, pose: "pose-stand", look: KY_TUTOR_LOOK },
  cast: [
    { x: 24, y: 90, scale: 0.7, pose: "pose-guard", look: { ...figureLookFor(["Human"]), hair: "hair-short", outfit: "outfit-hoodie", item: null, cloth: "blue-700" }, layer: "near" },
  ],
  layers: {
    sky: [{ part: "light-shaft", x: 50, y: 2, scale: 1.4, paint: "white" }],
    far: [
      { part: "room-wall", x: 50, y: 70 },
      { part: "arch", x: 50, y: 70, scale: 1.6 },
    ],
    mid: [{ part: "column", x: 90, y: 72, scale: 1.3 }],
    subject: [
      { part: "table", x: 54, y: 86, scale: 1.7 },
      { part: "envelope", x: 40, y: 54, scale: 1.8 },
      { part: "envelope", x: 52, y: 46, scale: 1.8 },
      { part: "envelope", x: 64, y: 54, scale: 1.8 },
    ],
    light: [{ part: "halo", x: 52, y: 50, scale: 1.2, paint: "white" }],
    particles: [{ part: "sparks", x: 52, y: 50, scale: 1.2, count: 2, spread: [14, 6] }],
  },
};

/** Going Long (Core #84, Rare Field Spell): a muddy trench line across an open sports field, a padded jersey and helmet hung on a post, a ball arcing far overhead; bright sky. The trench is tilled furrows, the jersey a red banner. */
const GOING_LONG: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 30, y: 22, r: 36, paint: "white", opacity: 0.5 },
  emblem: { x: 36, y: 62, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "cloud", x: 24, y: 18, scale: 1.2 },
      { part: "cloud", x: 82, y: 14, scale: 1 },
    ],
    far: [
      { part: "skyline", x: 70, y: 62, scale: 0.7, paint: "blue-200" },
      { part: "hills", x: 50, y: 62 },
    ],
    mid: [
      { part: "field", x: 50, y: 64 },
      { part: "furrows", x: 50, y: 70, paint: "wood-700" },
      { part: "fence-post", x: 82, y: 70, scale: 1.2 },
    ],
    subject: [
      { part: "fence-post", x: 36, y: 80, scale: 2.2 },
      { part: "banner", x: 36, y: 68, scale: 0.8, paint: "red-600" },
      { part: "helmet", x: 37, y: 61, scale: 3 },
    ],
    near: [{ part: "grass-tuft", x: 20, y: 94, scale: 1.6, count: 4, spread: [64, 2] }],
    light: [{ part: "halo", x: 66, y: 24, scale: 1, paint: "white" }],
    particles: [
      { part: "ball", x: 66, y: 24, scale: 2.4 },
      { part: "streaks", x: 76, y: 28, scale: 1, flip: -1 },
    ],
  },
};

/** "Miss" Mrow (Core #86, Epic Unit): a prim Felinor lady in a lace collar, claws out, a ghostly leash looping a brute's wrist, the brute's raised club, a tidy parlour behind with teacups toppling off a lace-covered table. The lace collar is her cardigan's. */
const MISS_MROW: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "stone-500"],
  light: { x: 30, y: 28, r: 38, paint: "white", opacity: 0.5 },
  emblem: { x: 34, y: 66, size: 5, rotate: 0 },
  figure: {
    x: 34,
    y: 88,
    scale: 1,
    pose: "pose-lunge",
    look: { ...figureLookFor(["Felinor"]), hair: null, outfit: "outfit-cardigan", item: null, cloth: "red-300" },
  },
  cast: [
    { x: 68, y: 82, scale: 0.9, flip: -1, pose: "pose-raise", look: { ...figureLookFor(["Human"]), head: "head-ghoul", hair: null, outfit: "outfit-rags", item: "item-club", skin: "green-700", cloth: "wood-700" }, layer: "mid" },
  ],
  layers: {
    sky: [{ part: "bulbs", x: 50, y: 8, scale: 1.2 }],
    far: [
      { part: "room-wall", x: 50, y: 70 },
      { part: "window", x: 20, y: 54, scale: 1.1 },
      { part: "table", x: 78, y: 70, scale: 1.1 },
      { part: "cup", x: 70, y: 62, scale: 1.4, count: 3, spread: [16, 1] },
    ],
    mid: [{ part: "chain", x: 40, y: 54, scale: 1.2, paint: "slate-100" }],
    subject: [{ part: "chair", x: 90, y: 82, scale: 1.2 }],
    near: [
      { part: "cup", x: 52, y: 92, scale: 2.4, count: 3, spread: [30, 2] },
      { part: "spray", x: 60, y: 86, scale: 1, paint: "white" },
    ],
    light: [{ part: "halo", x: 34, y: 54, scale: 1.1, paint: "white" }],
    particles: [{ part: "motes", x: 50, y: 60, scale: 1.4, paint: "white" }],
  },
};

/** Twisting Nether (Core #88, Epic Spell): city towers and soldiers bending flat like paper, the sky folding in half along a glowing seam, a swirling violet-black void drawing everything in. */
const TWISTING_NETHER: SceneRecipe = {
  family: "core",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 40, r: 42, paint: "violet-500", opacity: 0.55 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 50, y: 20, scale: 1.8 }],
    far: [
      { part: "void-swirl", x: 50, y: 40, scale: 2.2 },
      { part: "skyline", x: 50, y: 70, scale: 1.2, paint: "slate-800" },
    ],
    mid: [
      { part: "fold-seam", x: 50, y: 50 },
      { part: "paper-tower", x: 18, y: 78, scale: 1.3 },
      { part: "paper-tower", x: 82, y: 78, scale: 1.3, flip: -1 },
    ],
    subject: [
      { part: "paper-tower", x: 38, y: 82, scale: 1.8 },
      { part: "paper-tower", x: 62, y: 82, scale: 1.8, flip: -1 },
    ],
    near: [
      { part: "soldier-fallen", x: 24, y: 92, scale: 1.4, count: 2, spread: [12, 0] },
      { part: "soldier-fallen", x: 76, y: 92, scale: 1.4, flip: -1 },
    ],
    light: [{ part: "halo", x: 50, y: 40, scale: 1.8, paint: "purple-400" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.6, count: 2, spread: [14, 12], paint: "violet-300" }],
  },
};

/** Corpse Eater (Core #89, Epic Unit): a hunched, spore-crusted ghoul with a napkin tucked in its collar, grinning, glowing wisps drifting from fallen soldiers into its mouth, a trampled cornfield behind under a pale clean sky. */
const CORPSE_EATER: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "slate-100"],
  light: { x: 30, y: 24, r: 36, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 84,
    scale: 1.05,
    pose: "pose-kneel",
    look: { ...figureLookFor(["Human"]), head: "head-ghoul", hair: null, outfit: "outfit-apron", item: null, skin: "green-700", cloth: "white" },
  },
  layers: {
    sky: [{ part: "cloud", x: 70, y: 18, scale: 1.2 }],
    far: [
      { part: "hills", x: 50, y: 62 },
      { part: "wheat", x: 12, y: 66, scale: 1.8, count: 4, spread: [76, 3] },
    ],
    mid: [
      { part: "field", x: 50, y: 70 },
      { part: "soldier-fallen", x: 20, y: 80, scale: 1.4 },
      { part: "soldier-fallen", x: 80, y: 82, scale: 1.4, flip: -1 },
    ],
    subject: [
      { part: "wisp", x: 30, y: 66, scale: 1.4, count: 2, spread: [10, 8] },
      { part: "wisp", x: 70, y: 68, scale: 1.4, count: 2, spread: [-10, 8] },
    ],
    near: [{ part: "wheat", x: 14, y: 94, scale: 2, count: 3, spread: [72, 2] }],
    light: [{ part: "halo", x: 50, y: 52, scale: 1.1, paint: "green-200" }],
    particles: [{ part: "spores", x: 50, y: 60, scale: 1.6, count: 2, spread: [16, 8], paint: "green-400" }],
  },
};

/** Super-Virus Injection (Core #90, Rare Spell): a gloved hand pushing a glowing green syringe into a rival's deck on the counter of a traditional herbal pharmacy, spores puffing out. The glove is a white open hand. */
const SUPER_VIRUS_INJECTION: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "stone-500"],
  light: { x: 34, y: 28, r: 36, paint: "green-200", opacity: 0.45 },
  emblem: { x: 66, y: 62, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "bulbs", x: 50, y: 8, scale: 1.2 }],
    far: [
      { part: "room-wall", x: 50, y: 68 },
      { part: "shelf", x: 24, y: 52, scale: 1.3 },
      { part: "shelf", x: 76, y: 52, scale: 1.3 },
    ],
    mid: [
      { part: "counter", x: 50, y: 82, scale: 2 },
      { part: "jar", x: 14, y: 74, scale: 1.6 },
    ],
    subject: [
      { part: "deck", x: 66, y: 76, scale: 3.4 },
      { part: "syringe", x: 56, y: 66, scale: 2.4, paint: "green-400" },
      { part: "open-hand", x: 38, y: 70, scale: 2.2, paint: "white" },
    ],
    near: [{ part: "spores", x: 68, y: 68, scale: 1.6, paint: "green-400" }],
    light: [{ part: "halo", x: 60, y: 68, scale: 1.2, paint: "green-400" }],
    particles: [{ part: "motes", x: 60, y: 60, scale: 1.2, paint: "green-200" }],
  },
};

/** Super-Virus (Core token, Spell): a sneezing card spraying green spores over a shuffling deck, tiny copies of itself multiplying, in a clean white hospital waiting room. */
const SUPER_VIRUS: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "blue-200"],
  light: { x: 44, y: 34, r: 36, paint: "green-200", opacity: 0.45 },
  emblem: { x: 50, y: 70, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "bulbs", x: 50, y: 8, scale: 1.2 }],
    far: [
      { part: "tile-wall", x: 50, y: 74 },
      { part: "window", x: 80, y: 56, scale: 1.1 },
    ],
    mid: [{ part: "bench", x: 24, y: 82, scale: 1.4 }],
    subject: [
      { part: "deck", x: 66, y: 84, scale: 2.4 },
      { part: "card", x: 44, y: 76, scale: 3, paint: "green-400" },
    ],
    near: [{ part: "card", x: 62, y: 62, scale: 1.4, paint: "green-400", count: 4, spread: [22, 16] }],
    light: [{ part: "spray", x: 52, y: 62, scale: 1.4, paint: "green-400" }],
    particles: [{ part: "spores", x: 56, y: 62, scale: 1.8, count: 2, spread: [14, 8], paint: "green-400" }],
  },
};

/** Fed Fauci (Core #91, Rare Unit): a weary public-health official in a rumpled suit and surgical mask, each bruise glowing with blue mana, gripping a podium in a white briefing room. */
const FED_FAUCI: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "blue-200"],
  light: { x: 40, y: 34, r: 36, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 78, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 88,
    scale: 1,
    pose: "pose-guard",
    look: { ...figureLookFor(["Human"]), hair: "face-mask", outfit: "outfit-suit", item: null, cloth: "slate-400" },
  },
  layers: {
    sky: [{ part: "bulbs", x: 50, y: 8, scale: 1.2 }],
    far: [
      { part: "tile-wall", x: 50, y: 72 },
      { part: "window", x: 20, y: 54, scale: 1.2 },
      { part: "banner", x: 80, y: 34, scale: 1.2, paint: "blue-700" },
    ],
    mid: [{ part: "chair", x: 86, y: 82, scale: 1.4 }],
    subject: [{ part: "podium", x: 50, y: 96, scale: 2.4 }],
    near: [{ part: "motes", x: 50, y: 60, scale: 1.2, paint: "cyan-400" }],
    light: [{ part: "halo", x: 50, y: 56, scale: 1.2, paint: "cyan-400" }],
    particles: [{ part: "sparks", x: 50, y: 58, scale: 1, count: 2, spread: [10, 8], paint: "cyan-400" }],
  },
};

/** Combo-Fodder (Core token, Spell): a tiny card-shaped imp diving through a juggler's hoop into a red bolt, a thread of healing light looping back, in a bright circus ring. The hoop is a ring of light. */
const COMBO_FODDER: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "slate-100"],
  light: { x: 40, y: 34, r: 36, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 78, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "bunting", x: 50, y: 12, scale: 1.6 }],
    far: [
      { part: "banner", x: 14, y: 70, scale: 1.4, paint: "red-500" },
      { part: "banner", x: 86, y: 70, scale: 1.4, paint: "blue-700", flip: -1 },
    ],
    mid: [{ part: "boards", x: 50, y: 72 }],
    subject: [
      { part: "halo", x: 42, y: 56, scale: 1, paint: "orange-400" },
      { part: "card-imp", x: 42, y: 56, scale: 2.6 },
      { part: "zap", x: 56, y: 56, scale: 0.9, paint: "red-500" },
    ],
    near: [{ part: "heal-glow", x: 30, y: 60, scale: 1.4 }],
    light: [{ part: "spotlight", x: 50, y: 6, scale: 1.4 }],
    particles: [{ part: "confetti", x: 50, y: 56, scale: 1.4 }],
  },
};

/** Genn's Greed (Core #94, Epic Spell): a greedy noble in a fur collar sweeping paired silver coin stacks into a velvet sack, lone single coins fading into wisps on the table, a mirrored vault of perfectly paired shelves behind in cool silver light. */
const GENNS_GREED: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "blue-200"],
  light: { x: 40, y: 30, r: 38, paint: "slate-100", opacity: 0.55 },
  emblem: { x: 42, y: 66, size: 5, rotate: 0 },
  figure: {
    x: 40,
    y: 86,
    scale: 0.95,
    pose: "pose-spread",
    look: { ...figureLookFor(["Human"]), hair: "cap-feather", outfit: "outfit-fur", item: null, cloth: "violet-700" },
  },
  layers: {
    sky: [{ part: "chandelier", x: 50, y: 6, scale: 1.4 }],
    far: [
      { part: "tile-wall", x: 50, y: 70 },
      { part: "shelf", x: 14, y: 50, scale: 1 },
      { part: "shelf", x: 86, y: 50, scale: 1, flip: -1 },
      { part: "mirror", x: 36, y: 62, scale: 1.2 },
      { part: "mirror", x: 64, y: 62, scale: 1.2, flip: -1 },
    ],
    mid: [
      { part: "table", x: 76, y: 82, scale: 1.1 },
      { part: "coin", x: 70, y: 72, scale: 1.4, count: 3, spread: [14, 4], paint: "slate-100" },
      { part: "puff", x: 76, y: 66, scale: 0.4 },
    ],
    subject: [
      { part: "sack", x: 62, y: 86, scale: 1.8 },
      { part: "coin-stack", x: 78, y: 92, scale: 1.4, count: 2, spread: [6, 0] },
    ],
    near: [{ part: "coin", x: 20, y: 94, scale: 1.4, count: 3, spread: [44, 2] }],
    light: [{ part: "rim-light", x: 40, y: 62, scale: 1.1, paint: "slate-100" }],
    particles: [{ part: "sparks", x: 62, y: 70, scale: 1.2, count: 2, spread: [14, 8] }],
  },
};

/** Chaos Golem (Core token, Unit): a towering golem of mismatched rubble and dice, wrapped in a bubble shield, charging out of a smouldering fireworks yard through bright smoke. */
const CHAOS_GOLEM: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "slate-100"],
  light: { x: 34, y: 26, r: 36, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 1.25,
    pose: "pose-lunge",
    look: { ...figureLookFor(["Human"]), head: "head-stone", hair: null, outfit: "outfit-rock", item: null, skin: "slate-400", cloth: "stone-500" },
  },
  layers: {
    sky: [{ part: "cloud", x: 22, y: 20, scale: 1.2 }],
    far: [
      { part: "fence", x: 50, y: 70, scale: 1.4 },
      { part: "burst", x: 20, y: 40, scale: 1.4, paint: "red-300" },
      { part: "burst", x: 82, y: 34, scale: 1.1, paint: "cyan-400" },
    ],
    mid: [
      { part: "field", x: 50, y: 74 },
      { part: "smoke", x: 22, y: 72, scale: 1.4 },
      { part: "smoke", x: 80, y: 70, scale: 1.2 },
    ],
    subject: [
      { part: "dice", x: 36, y: 66, scale: 2.4 },
      { part: "dice", x: 62, y: 74, scale: 2.4 },
    ],
    near: [{ part: "crate", x: 14, y: 92, scale: 1.6 }],
    light: [{ part: "halo", x: 50, y: 62, scale: 2.2, paint: "cyan-400" }],
    particles: [{ part: "sparks", x: 50, y: 46, scale: 1.4, count: 2, spread: [22, 10] }],
  },
};

/** Rush Token (Core token, Unit #t-rush): a small round-bodied sprinter with springy legs and a headband, kicking up spray as it dashes along a bright sandy beach. */
const RUSH_TOKEN: SceneRecipe = {
  family: "core",
  sky: ["sky-400", "blue-200"],
  light: { x: 30, y: 24, r: 36, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 56, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "cloud", x: 24, y: 18, scale: 1.2 },
      { part: "gull", x: 74, y: 28, scale: 1.4 },
    ],
    far: [{ part: "water", x: 50, y: 54 }],
    mid: [{ part: "sand", x: 50, y: 66 }],
    subject: [{ part: "rush-token", x: 52, y: 78, scale: 3 }],
    near: [
      { part: "spray", x: 36, y: 84, scale: 1.4 },
      { part: "spray", x: 80, y: 90, scale: 1 },
    ],
    light: [{ part: "halo", x: 52, y: 62, scale: 1, paint: "white" }],
    particles: [{ part: "dust", x: 32, y: 84, scale: 1.2 }],
  },
};

/** Sheep Token (Core token, Unit #t-sheep): a fluffy, slightly puzzled white sheep standing on a sunny stone altar step, a daisy in its mouth. */
const SHEEP_TOKEN: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "slate-100"],
  light: { x: 30, y: 24, r: 36, paint: "white", opacity: 0.55 },
  emblem: { x: 50, y: 56, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 24, y: 20, scale: 1.2 }],
    far: [
      { part: "column", x: 14, y: 70, scale: 1.4 },
      { part: "column", x: 86, y: 70, scale: 1.4 },
    ],
    mid: [{ part: "rock-floor", x: 50, y: 78, paint: "slate-400" }],
    subject: [
      { part: "sheep", x: 48, y: 80, scale: 3 },
      { part: "flower", x: 62, y: 74, scale: 0.9, paint: "white" },
    ],
    near: [{ part: "flower", x: 20, y: 94, scale: 1.4, count: 3, spread: [60, 2] }],
    light: [{ part: "light-shaft", x: 30, y: 6, scale: 1.2, paint: "white" }],
    particles: [{ part: "motes", x: 50, y: 60, scale: 1.2, paint: "white" }],
  },
};

/** Felinor Token (Core token, Unit #t-felinor): a tiny curious cat-folk soldier in an oversized helmet peering over a toy-fort rampart, a twig spear in paw; bright morning. */
const FELINOR_TOKEN: SceneRecipe = {
  family: "core",
  sky: ["sky-300", "slate-100"],
  light: { x: 30, y: 22, r: 36, paint: "white", opacity: 0.55 },
  emblem: { x: 50, y: 78, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 90,
    scale: 0.8,
    pose: "pose-hold",
    look: { ...figureLookFor(["Felinor"]), hair: "steel-helmet", outfit: "outfit-tunic", item: "item-spear", cloth: "violet-700" },
  },
  layers: {
    sky: [{ part: "cloud", x: 76, y: 18, scale: 1.1 }],
    far: [
      { part: "hills", x: 50, y: 62 },
      { part: "tree", x: 14, y: 66, scale: 1.2 },
    ],
    mid: [{ part: "field", x: 50, y: 68 }],
    near: [{ part: "battlement", x: 50, y: 92, scale: 1.5, paint: "red-400" }],
    light: [{ part: "halo", x: 50, y: 56, scale: 1, paint: "white" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.2, paint: "white" }],
  },
};

/** Bread Token (Core token, Unit #t-bread): a plump crusty loaf with stubby little legs waddling off a cooling rack in a warm, flour-dusted bakery. */
const BREAD_TOKEN: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "stone-500"],
  light: { x: 34, y: 28, r: 36, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 50, y: 58, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "bulbs", x: 50, y: 8, scale: 1.2 }],
    far: [
      { part: "room-wall", x: 50, y: 70 },
      { part: "window", x: 22, y: 54, scale: 1.1 },
      { part: "shelf", x: 78, y: 48, scale: 1 },
    ],
    mid: [{ part: "table", x: 40, y: 82, scale: 1.6 }],
    subject: [{ part: "loaf", x: 56, y: 80, scale: 3 }],
    near: [{ part: "puff", x: 24, y: 90, scale: 0.8, paint: "white" }],
    light: [{ part: "light-shaft", x: 22, y: 4, scale: 1.2, paint: "white" }],
    particles: [{ part: "motes", x: 50, y: 64, scale: 1.4, paint: "white" }],
  },
};

/** The Coin (Core token, Spell #t-coin): a single silver coin spinning on a polished tabletop, blue mana spraying from its edge as it flips; clean white light. */
const THE_COIN: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "blue-200"],
  light: { x: 50, y: 40, r: 34, paint: "white", opacity: 0.6 },
  emblem: { x: 50, y: 84, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "light-shaft", x: 50, y: 2, scale: 1.4, paint: "white" }],
    far: [{ part: "room-wall", x: 50, y: 72 }],
    mid: [
      { part: "table", x: 50, y: 88, scale: 2.8 },
      { part: "coin-stack", x: 82, y: 78, scale: 1.6 },
    ],
    subject: [{ part: "coin", x: 50, y: 58, scale: 6 }],
    near: [{ part: "spray", x: 50, y: 58, scale: 2, paint: "cyan-400" }],
    light: [{ part: "halo", x: 50, y: 58, scale: 1.6, paint: "cyan-400" }],
    particles: [{ part: "sparks", x: 50, y: 56, scale: 1.4, count: 2, spread: [16, 8], paint: "cyan-400" }],
  },
};

/** Ghoul Token (Core token, Unit #t-ghoul): a gangly ghoul stitched from mismatched leftover limbs, clutching a sharpened bone spike, crouched in a tidy surgeon's scrap bin. The bone spike is the figure kit's dagger, its blade bone-pale, since the kit holds no bone. */
const GHOUL_TOKEN: SceneRecipe = {
  family: "core",
  sky: ["slate-100", "stone-500"],
  light: { x: 34, y: 28, r: 36, paint: "green-200", opacity: 0.4 },
  emblem: { x: 50, y: 82, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 84,
    scale: 1.1,
    pose: "pose-kneel",
    look: { ...figureLookFor(["Human"]), head: "head-ghoul", hair: null, outfit: "outfit-rags", item: "item-dagger", skin: "green-700", cloth: "wood-700" },
  },
  layers: {
    sky: [{ part: "bulbs", x: 50, y: 8, scale: 1.2 }],
    far: [
      { part: "tile-wall", x: 50, y: 72 },
      { part: "shelf", x: 80, y: 46, scale: 1 },
    ],
    mid: [{ part: "vial", x: 22, y: 78, scale: 1.6, count: 2, spread: [6, 0] }],
    near: [{ part: "bin", x: 50, y: 96, scale: 2.8 }],
    light: [{ part: "halo", x: 50, y: 66, scale: 1, paint: "green-200" }],
    particles: [{ part: "spores", x: 50, y: 60, scale: 1.2, paint: "green-400" }],
  },
};

export const CORE_SCENES_3: Readonly<Record<string, SceneRecipe>> = {
  "core-075": INFINITE_RESERVES,
  "core-076": FIELD_OF_DREAMS,
  "core-077": PROFESSOR_CURVATURE,
  "core-078": FULLSEND,
  "core-079": TWINSPELL,
  "core-080": ZAO_GAO,
  "core-081": RADIANT_SAINTESS,
  "core-082": KYS_TRIAL,
  "core-083": TRANSMOGULATE,
  "core-084": GOING_LONG,
  "core-085": UNLICENSED_EXPERIMENTATION,
  "core-086": MISS_MROW,
  "core-087": POCKET_CHAOS,
  "core-088": TWISTING_NETHER,
  "core-089": CORPSE_EATER,
  "core-090": SUPER_VIRUS_INJECTION,
  "core-090-1": SUPER_VIRUS,
  "core-091": FED_FAUCI,
  "core-092": FELINOR_FIENDER,
  "core-093": COMBO_INDEX,
  "core-093-1": COMBO_FODDER,
  "core-094": GENNS_GREED,
  "core-095": CALL_TO_CHAOS,
  "core-095-1": CHAOS_GOLEM,
  "core-096": MY_PAWN,
  "core-097": ZEPHRYS,
  "core-098": HEROIC_POWER,
  "core-099": CRAFT_A_CARD,
  "core-t-bread": BREAD_TOKEN,
  "core-t-coin": THE_COIN,
  "core-t-felinor": FELINOR_TOKEN,
  "core-t-ghoul": GHOUL_TOKEN,
  "core-t-rush": RUSH_TOKEN,
  "core-t-sheep": SHEEP_TOKEN,
};
