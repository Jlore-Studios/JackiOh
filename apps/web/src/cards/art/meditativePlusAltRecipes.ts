// R3397000: Meditative+'s alt scene recipes, one per card offered Alt art, each drawn from its brief (ART.md,
// "Meditative+ and Training Grounds Alt art restaged"): the card's own subject in another place, time or moment,
// under a sky sharing no colour with its normal picture's and a new camera (a close-up, a wide or distant view, or
// one from above), its figure the normal picture's person (`normalLookOf`), in the Meditative family's ink line
// with its one red seal where the normal picture keeps it, and at or above its rarity's floor. The ink-wash peaks
// are the normal pictures' and stay out where a brief sets its scene under a lake, in bamboo or on a picnic
// blanket. scene.ts `sceneRecipeOf` reads one under the card's Alt art seed alone, at every Alt art level.
// Presentation only (CLAUDE.md rule 7): a recipe is read by the card's id alone.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";
import { MEDITATIVE_PLUS_RECIPES } from "./sceneRecipesMeditativePlus.ts";

/** The look of the card's normal figure, so its Alt picture draws the same person; with none, the default person. */
function normalLookOf(id: string): FigureLook {
  return MEDITATIVE_PLUS_RECIPES[id]?.figure?.look ?? figureLookFor([]);
}

/** The look of the card's normal `k`th extra figure, so a person its brief keeps is the same one; with none, the default person. */
function normalExtraLookOf(id: string, k: number): FigureLook {
  return MEDITATIVE_PLUS_RECIPES[id]?.extras?.[k]?.look ?? figureLookFor([]);
}

/** A villager of the mountain towns come to see the painted card, in a plain tunic. */
const VILLAGER: FigureLook = { ...figureLookFor(["Human"]), hair: "hair-short", outfit: "outfit-tunic", item: null, cloth: "green-700" };

/** Bob's Plan (Meditative+ #1, Mythic), Alt art: after the win, close up: Bob dozes in a hammock at a summit hut, the finished plan folded into a sun hat over his eyes; pale mint afternoon, wide sky. The folded plan is his brimmed hat. */
const BOBS_PLAN_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "green-200"],
  light: { x: 76, y: 20, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 72, y: 34, size: 6, rotate: 0 },
  figure: { x: 48, y: 90, scale: 1.2, pose: "pose-sit", look: { ...normalLookOf("meditativeplus-001"), hair: "hat-brim" } },
  layers: {
    sky: [
      { part: "cloud", x: 30, y: 22, scale: 1.4, count: 3, spread: [26, 4], paint: "white" },
      { part: "cloud", x: 70, y: 30, scale: 1, count: 2, spread: [14, 3], paint: "slate-100" },
      { part: "bird", x: 30, y: 30, scale: 0.4, count: 4, spread: [12, 5], paint: "slate-700" },
      { part: "sun", x: 82, y: 18, scale: 0.6 },
    ],
    far: [
      { part: "mountains", x: 50, y: 62, paint: "slate-400" },
      { part: "summit", x: 16, y: 66, scale: 0.7, paint: "slate-700" },
      { part: "mist", x: 50, y: 60, scale: 1.6, count: 2, spread: [20, 3], paint: "white" },
    ],
    mid: [
      { part: "house", x: 84, y: 76, scale: 0.9, flip: -1 },
      { part: "rock-floor", x: 50, y: 74, paint: "stone-500" },
      { part: "ink-pine", x: 8, y: 76, scale: 1.2 },
      { part: "grass-tuft", x: 64, y: 80, count: 3, spread: [10, 2], paint: "green-700" },
    ],
    subject: [
      { part: "hammock", x: 48, y: 96, scale: 3 },
      { part: "cup", x: 80, y: 92, scale: 1.4 },
    ],
    near: [
      { part: "plum-branch", x: 100, y: 22, scale: 1.1, flip: -1 },
      { part: "grass-tuft", x: 12, y: 98, count: 3, spread: [8, 2], paint: "green-700" },
      { part: "flowers", x: 30, y: 99, count: 2, spread: [6, 1] },
      { part: "seal", x: 90, y: 77, scale: 1.1 },
    ],
    light: [
      { part: "god-rays", x: 82, y: 16, scale: 1.2, paint: "white" },
      { part: "halo", x: 46, y: 70, scale: 1.4, paint: "lime-200" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.3, count: 2, spread: [24, 12], paint: "white" },
      { part: "petals", x: 60, y: 44, scale: 1, paint: "green-400" },
    ],
  },
};

/** Bloodbloom (Meditative+ #6, Epic), Alt art: high summer, wide vista: a whole valley of blood-red blooms opening across green terraces as a lone mage walks through, petals trailing behind her; humid emerald haze. */
const BLOODBLOOM_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["green-700", "green-200"],
  light: { x: 50, y: 24, r: 40, paint: "white", opacity: 0.3 },
  emblem: { x: 74, y: 40, size: 5, rotate: 0 },
  extras: [{ layer: "subject", x: 40, y: 72, scale: 0.3, pose: "pose-stride", look: normalExtraLookOf("meditativeplus-006", 0) }],
  layers: {
    sky: [{ part: "cloud", x: 30, y: 24, scale: 1.2, count: 2, spread: [20, 3], paint: "green-200" }],
    far: [
      { part: "hills", x: 50, y: 50, paint: "green-700" },
      { part: "mist", x: 50, y: 50, scale: 1.8, count: 2, spread: [24, 2], paint: "green-200" },
    ],
    mid: [
      { part: "terrace", x: 50, y: 58 },
      { part: "bloom", x: 20, y: 64, scale: 0.7, count: 4, spread: [14, 3] },
      { part: "bloom", x: 80, y: 62, scale: 0.6, count: 4, spread: [12, 3] },
    ],
    subject: [
      { part: "bloom", x: 62, y: 76, scale: 1.1 },
      { part: "bloom", x: 50, y: 80, scale: 0.9, count: 5, spread: [30, 6] },
      { part: "bloom", x: 24, y: 78, scale: 0.8, count: 2, spread: [8, 2] },
    ],
    near: [
      { part: "tall-grass", x: 8, y: 98, scale: 1.4, paint: "green-700" },
      { part: "bloom", x: 82, y: 94, scale: 1.4 },
      { part: "seal", x: 90, y: 77 },
    ],
    light: [{ part: "light-shaft", x: 50, y: 18, scale: 1.2, paint: "white" }],
    particles: [
      { part: "petals", x: 28, y: 64, scale: 1.1, paint: "red-600" },
      { part: "motes", x: 50, y: 40, scale: 1.2, paint: "green-200" },
    ],
  },
};

/** Arch Mage (Meditative+ #8, Common), Alt art: off duty in autumn: the archmage reads alone on a garden bench under a maple, one fingertip lighting a tiny spark for her tea; rust-orange leaves, low angle. The normal picture's bearded mage, the same person. */
const ARCH_MAGE_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["brass-600", "orange-400"],
  light: { x: 22, y: 30, r: 34, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 70, y: 30, size: 5, rotate: 0 },
  figure: { x: 38, y: 90, scale: 1.05, pose: "pose-sit", look: { ...normalLookOf("meditativeplus-008"), item: "item-book", cloth: "red-900" } },
  layers: {
    far: [{ part: "tree", x: 70, y: 74, scale: 6, paint: "orange-400" }],
    mid: [
      { part: "field", x: 50, y: 82, paint: "wood-700" },
      { part: "fence", x: 84, y: 84, scale: 1.4, paint: "wood-800" },
    ],
    subject: [
      { part: "bench", x: 42, y: 92, scale: 2.1 },
      { part: "cup", x: 60, y: 79, scale: 1.4 },
      { part: "sparks", x: 61, y: 74, scale: 0.4, paint: "cyan-400" },
    ],
    near: [
      { part: "leaves", x: 20, y: 96, scale: 1.1 },
      { part: "grass-tuft", x: 82, y: 98, count: 2, spread: [6, 1], paint: "wood-600" },
      { part: "seal", x: 90, y: 77 },
    ],
    particles: [
      { part: "leaves", x: 60, y: 40, scale: 1.4 },
      { part: "petals", x: 30, y: 30, scale: 1.2, paint: "red-600" },
    ],
  },
};

/** Medina Grifter (Meditative+ #10, Common), Alt art: before the job: the grifter counts his purses on a misty rooftop at dawn, the bazaar's awnings still furled far below; cool blue-grey light, overhead view. */
const MEDINA_GRIFTER_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "blue-200"],
  light: { x: 80, y: 24, r: 30, paint: "white", opacity: 0.3 },
  emblem: { x: 70, y: 32, size: 5, rotate: 0 },
  figure: { x: 56, y: 78, scale: 0.9, pose: "pose-sit", look: normalLookOf("meditativeplus-010") },
  layers: {
    far: [
      { part: "awning", x: 16, y: 44, scale: 0.4, paint: "slate-400" },
      { part: "awning", x: 34, y: 42, scale: 0.35, flip: -1, paint: "slate-400" },
      { part: "awning", x: 84, y: 44, scale: 0.4, paint: "slate-400" },
      { part: "mist", x: 50, y: 46, scale: 1.6, count: 2, spread: [20, 3], paint: "white" },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 56, paint: "slate-700" }],
    subject: [
      { part: "pouch", x: 36, y: 82, scale: 1.1 },
      { part: "pouch", x: 74, y: 84, scale: 0.9, flip: -1 },
      { part: "coins", x: 50, y: 86, scale: 0.9 },
    ],
    near: [
      { part: "mist", x: 30, y: 96, scale: 1.2, paint: "blue-200" },
      { part: "seal", x: 90, y: 77 },
    ],
    particles: [{ part: "motes", x: 50, y: 40, scale: 1, paint: "white" }],
  },
};

/** Living Armor Potion (Meditative+ #12, Rare), Alt art: midday on a snowy ridge, wide shot: the plated potion bottle shields a shivering mountain goat from a hailstorm, ice ringing off its armor; white and pale blue. The hail falls as the kit's snow. */
const LIVING_ARMOR_POTION_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["blue-200", "white"],
  light: { x: 50, y: 20, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 28, y: 36, size: 5, rotate: 0 },
  figure: { x: 42, y: 74, scale: 0.42, pose: "pose-block", look: { ...normalLookOf("meditativeplus-012"), item: "item-shield" } },
  layers: {
    sky: [{ part: "cloud", x: 40, y: 22, scale: 1.4, count: 3, spread: [28, 3], paint: "slate-400" }],
    far: [{ part: "mountains", x: 50, y: 58, paint: "slate-400" }],
    mid: [{ part: "snowfield", x: 50, y: 70 }],
    subject: [
      { part: "goat", x: 54, y: 76, scale: 1 },
      { part: "sparks", x: 46, y: 56, scale: 0.6, paint: "white" },
    ],
    near: [
      { part: "cliff", x: 10, y: 92, paint: "slate-700" },
      { part: "seal", x: 90, y: 77 },
    ],
    particles: [
      { part: "snow", x: 50, y: 40, scale: 1.4, paint: "white" },
      { part: "snow", x: 50, y: 70, scale: 1.1, paint: "blue-200" },
    ],
  },
};

/** Living Health Potion (Meditative+ #13, Rare), Alt art: close-up on a spring picnic blanket: the red potion pot pours itself into tiny cups for a crowd of bandaged rabbits; fresh lime-green meadow all around. The bandages are left out. */
const LIVING_HEALTH_POTION_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["sky-400", "lime-200"],
  light: { x: 70, y: 24, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 72, y: 34, size: 5, rotate: 0 },
  figure: { x: 30, y: 100, scale: 1.6, pose: "pose-cast", look: normalLookOf("meditativeplus-013") },
  layers: {
    sky: [
      { part: "cloud", x: 64, y: 20, scale: 1, count: 2, spread: [16, 3], paint: "white" },
      { part: "sun", x: 84, y: 16, scale: 0.5 },
    ],
    far: [{ part: "hills", x: 50, y: 54, paint: "green-400" }],
    mid: [
      { part: "field", x: 50, y: 66, paint: "lime-400" },
      { part: "flowers", x: 84, y: 70, count: 3, spread: [10, 2] },
    ],
    subject: [
      { part: "picnic-blanket", x: 60, y: 92, scale: 1.6 },
      { part: "cup", x: 56, y: 80, scale: 1.3, count: 3, spread: [12, 2] },
      { part: "bunny", x: 72, y: 84, scale: 1.2, count: 3, spread: [12, 4] },
    ],
    near: [
      { part: "bunny", x: 88, y: 98, scale: 1.4, flip: -1 },
      { part: "flowers", x: 54, y: 99, count: 2, spread: [8, 1] },
      { part: "seal", x: 10, y: 77 },
    ],
    particles: [{ part: "petals", x: 60, y: 40, scale: 1.1, paint: "red-300" }],
  },
};

/** Potion Pouch (Meditative+ #14, Rare), Alt art: under a mountain lake, looking up: the pouch sinks slowly as its bottles drift free and fizz bubbles of colour toward the bright surface; deep turquoise. */
const POTION_POUCH_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["cyan-400", "blue-800"],
  light: { x: 50, y: 10, r: 44, paint: "white", opacity: 0.4 },
  emblem: { x: 26, y: 40, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "light-shaft", x: 40, y: 6, scale: 1.2, paint: "white" }],
    far: [
      { part: "rock-floor", x: 50, y: 86, paint: "slate-800" },
      { part: "reed", x: 14, y: 88, scale: 2.2, paint: "green-700" },
    ],
    mid: [
      { part: "reed", x: 84, y: 90, scale: 2, flip: -1, paint: "green-900" },
      { part: "pebbles", x: 60, y: 88, scale: 1.2 },
    ],
    subject: [
      { part: "pouch", x: 50, y: 66, scale: 1.3, flip: -1 },
      { part: "flask", x: 34, y: 46, scale: 0.6, paint: "red-500" },
      { part: "flask", x: 62, y: 36, scale: 0.55, paint: "violet-500" },
      { part: "flask", x: 70, y: 52, scale: 0.5, paint: "orange-400" },
    ],
    near: [{ part: "seal", x: 88, y: 77 }],
    light: [{ part: "god-rays", x: 50, y: 6, scale: 1.3, paint: "cyan-400" }],
    particles: [
      { part: "bubbles", x: 34, y: 34, scale: 1.1, paint: "red-300" },
      { part: "bubbles", x: 62, y: 24, scale: 1, paint: "violet-300" },
      { part: "bubbles", x: 70, y: 40, scale: 0.9, paint: "white" },
    ],
  },
};

/** Book of Potions (Meditative+ #15, Epic), Alt art: the book left open in a rainy bamboo grove, top-down: each raindrop that splashes its page sprouts a tiny potion; wet dark-green shade, quiet. */
const BOOK_OF_POTIONS_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["green-900", "green-700"],
  light: { x: 50, y: 50, r: 30, paint: "green-200", opacity: 0.25 },
  emblem: { x: 50, y: 32, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "bamboo", x: 12, y: 60, scale: 1.6, paint: "green-700" },
      { part: "bamboo", x: 88, y: 58, scale: 1.5, paint: "green-700" },
    ],
    mid: [
      { part: "field", x: 50, y: 50, paint: "green-900" },
      { part: "fern", x: 20, y: 74, scale: 1.6 },
      { part: "fern", x: 80, y: 72, scale: 1.4, flip: -1 },
    ],
    subject: [
      { part: "tome", x: 50, y: 66, scale: 2.4 },
      { part: "flask", x: 42, y: 54, scale: 0.35, paint: "cyan-400" },
      { part: "flask", x: 56, y: 52, scale: 0.3, paint: "red-500" },
      { part: "flask", x: 62, y: 58, scale: 0.3, paint: "violet-500" },
    ],
    near: [
      { part: "bamboo", x: 4, y: 100, scale: 2.2 },
      { part: "grass-tuft", x: 76, y: 98, count: 2, spread: [6, 1], paint: "green-900" },
      { part: "seal", x: 92, y: 77 },
    ],
    light: [{ part: "halo", x: 50, y: 58, scale: 1.2, paint: "green-200" }],
    particles: [
      { part: "rain", x: 50, y: 40, scale: 1.3, paint: "slate-100" },
      { part: "drops", x: 50, y: 56, scale: 1, paint: "blue-200" },
    ],
  },
};

/** Dogpile (Meditative+ #17, Rare), Alt art: the aftermath, wide and still: a smoking crater on a snowy ridge, one dizzy foe at its centre, spent sparks drifting down; pale lavender overcast. */
const DOGPILE_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["violet-300", "slate-100"],
  light: { x: 50, y: 30, r: 34, paint: "white", opacity: 0.3 },
  emblem: { x: 74, y: 36, size: 5, rotate: 0 },
  extras: [{ layer: "subject", x: 50, y: 74, scale: 0.3, pose: "pose-sit", look: { ...normalExtraLookOf("meditativeplus-017", 0), item: null } }],
  layers: {
    sky: [{ part: "cloud", x: 30, y: 24, scale: 1.4, count: 3, spread: [28, 3], paint: "violet-300" }],
    far: [{ part: "mountains", x: 50, y: 58, paint: "violet-500" }],
    mid: [{ part: "snowfield", x: 50, y: 68 }],
    subject: [{ part: "crater", x: 50, y: 76, scale: 1.2 }],
    near: [
      { part: "pebbles", x: 20, y: 94, scale: 1.2 },
      { part: "seal", x: 90, y: 77 },
    ],
    particles: [
      { part: "embers", x: 50, y: 46, scale: 1.2 },
      { part: "sparks", x: 38, y: 40, scale: 0.8, paint: "orange-400" },
      { part: "snow", x: 50, y: 50, scale: 1.2, paint: "white" },
    ],
  },
};

/** Grubby Gloves (Meditative+ #19, Common), Alt art: close-up: the muddy gloves pegged on a washing line in a sunny farmyard, a stolen card still pinched in one finger; bright sky blue. */
const GRUBBY_GLOVES_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["sky-400", "sky-300"],
  light: { x: 80, y: 20, r: 34, paint: "white", opacity: 0.4 },
  emblem: { x: 22, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 84, y: 18, scale: 0.6 },
      { part: "cloud", x: 36, y: 22, scale: 1, count: 2, spread: [14, 3], paint: "white" },
    ],
    far: [
      { part: "hills", x: 50, y: 62, paint: "green-400" },
      { part: "barn", x: 80, y: 70, scale: 0.6 },
    ],
    mid: [
      { part: "field", x: 50, y: 76, paint: "green-500" },
      { part: "fence", x: 22, y: 80, scale: 1.2, paint: "wood-600" },
    ],
    subject: [
      { part: "washing-line", x: 50, y: 86, scale: 3 },
      { part: "glove", x: 34, y: 50, scale: 1.6 },
      { part: "glove", x: 64, y: 52, scale: 1.6, flip: -1 },
      { part: "card", x: 50, y: 60, scale: 0.9, paint: "red-600" },
    ],
    near: [
      { part: "grass-tuft", x: 12, y: 98, count: 3, spread: [8, 2], paint: "green-700" },
      { part: "seal", x: 88, y: 77 },
    ],
    particles: [{ part: "motes", x: 50, y: 40, scale: 1, paint: "white" }],
  },
};

/** KY's Competition (Meditative+ #20, Rare), Alt art: after the contest, a wide back view: KY and Lil' Kenny on a cliff bench sharing a melon, an emptied hourglass between them; warm coral sunset. Their backs are drawn as the kit's figures face the viewer, and the melon is the kit's split melon on its board (#3401's). */
const KYS_COMPETITION_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["violet-500", "red-300"],
  light: { x: 50, y: 58, r: 40, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 26, y: 34, size: 5, rotate: 0 },
  figure: { x: 40, y: 72, scale: 0.4, pose: "pose-sit", look: normalLookOf("meditativeplus-020") },
  extras: [{ layer: "subject", x: 60, y: 72, scale: 0.34, flip: -1, pose: "pose-sit", look: normalLookOf("meditativeplus-020-1") }],
  layers: {
    sky: [
      { part: "cloud", x: 30, y: 26, scale: 1.3, count: 3, spread: [28, 4], paint: "red-300" },
      { part: "bird", x: 70, y: 30, scale: 0.4, count: 3, spread: [10, 4], paint: "slate-900" },
    ],
    far: [
      { part: "sea", x: 50, y: 60, paint: "violet-500" },
      { part: "sun", x: 50, y: 60, scale: 1 },
    ],
    mid: [{ part: "cliff", x: 60, y: 76, paint: "slate-800" }],
    subject: [
      { part: "bench", x: 50, y: 74, scale: 1.2 },
      { part: "hourglass", x: 50, y: 66, scale: 0.45 },
      { part: "melon", x: 50, y: 78, scale: 0.6 },
    ],
    near: [
      { part: "grass-tuft", x: 20, y: 96, count: 3, spread: [10, 2], paint: "slate-800" },
      { part: "seal", x: 92, y: 77 },
    ],
    light: [{ part: "god-rays", x: 50, y: 60, scale: 1.2, paint: "orange-400" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1, paint: "red-300" }],
  },
};

/** Bob, Wisconsin's Strongest Soldier (Meditative+ #24, Legendary), Alt art: Friday's fish fry by a frozen lake: Bob carries the whole picnic table one-handed, platters still on it, his snowplow parked behind; brick-red lantern glow at dusk. The platters are a pan and a loaf. */
const BOB_WISCONSINS_STRONGEST_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["blue-900", "red-600"],
  light: { x: 84, y: 60, r: 34, paint: "ember-500", opacity: 0.4 },
  emblem: { x: 70, y: 30, size: 6, rotate: 0 },
  figure: { x: 34, y: 90, scale: 0.74, pose: "pose-lift", look: normalLookOf("meditativeplus-024") },
  layers: {
    sky: [
      { part: "stars", x: 50, y: 16, scale: 1 },
      { part: "moon", x: 80, y: 20, scale: 0.5 },
    ],
    far: [
      { part: "ink-pine", x: 16, y: 62, scale: 0.6, count: 3, spread: [12, 1], paint: "slate-800" },
      { part: "ink-pine", x: 86, y: 62, scale: 0.5, count: 2, spread: [8, 1], paint: "slate-800" },
      { part: "mist", x: 50, y: 62, scale: 1.4, paint: "red-300" },
    ],
    mid: [
      { part: "ice-floor", x: 50, y: 66 },
      { part: "fence-post", x: 86, y: 86, scale: 1.4, flip: -1 },
      { part: "lantern", x: 79, y: 70, scale: 1.4, paint: "red-600" },
    ],
    subject: [
      { part: "snowplow", x: 70, y: 82, scale: 1 },
      { part: "table", x: 34, y: 44, scale: 1.3 },
      { part: "pan", x: 30, y: 35, scale: 0.8 },
      { part: "bread", x: 40, y: 35, scale: 0.8 },
    ],
    near: [
      { part: "snowfield", x: 50, y: 94 },
      { part: "seal", x: 10, y: 77 },
    ],
    light: [
      { part: "halo", x: 79, y: 74, scale: 1.2, paint: "ember-500" },
      { part: "rim-light", x: 34, y: 60, scale: 1.2, paint: "red-300" },
    ],
    particles: [{ part: "snow", x: 50, y: 40, scale: 1.3, paint: "white" }],
  },
};

/** KY's Intuition (Meditative+ #25, Rare), Alt art: close-up of KY's eyes mirrored in a still mountain pool, three blocks of line-work rippling across the water; fresh spring green and silver, from above. KY kneels over the pool; the kit draws no mirrored figure, so the reflection is left out. */
const KYS_INTUITION_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["green-400", "slate-100"],
  light: { x: 70, y: 60, r: 34, paint: "white", opacity: 0.35 },
  emblem: { x: 72, y: 30, size: 5, rotate: 0 },
  figure: { x: 24, y: 84, scale: 1.3, pose: "pose-kneel", look: normalLookOf("meditativeplus-025") },
  layers: {
    far: [{ part: "peaks", x: 50, y: 46, scale: 0.8, paint: "slate-400" }],
    mid: [{ part: "water", x: 50, y: 54, paint: "slate-400" }],
    subject: [
      { part: "code-block", x: 58, y: 62, scale: 0.9, fade: 0.8 },
      { part: "code-block", x: 72, y: 70, scale: 1, fade: 0.8 },
      { part: "code-block", x: 64, y: 80, scale: 0.9, fade: 0.8 },
    ],
    near: [
      { part: "grass-tuft", x: 10, y: 98, count: 3, spread: [6, 2], paint: "green-700" },
      { part: "seal", x: 92, y: 77 },
    ],
    particles: [{ part: "motes", x: 64, y: 70, scale: 0.9, paint: "slate-100" }],
  },
};

/** Scratch Pad (Meditative+ #26, Mythic), Alt art: the morning after: villagers crowd around a freshly painted card propped on a rock, its coder asleep beside the brush; warm peach sunrise, eye level. */
const SCRATCH_PAD_ALT: SceneRecipe = {
  family: "meditative",
  sky: ["red-300", "orange-400"],
  light: { x: 50, y: 60, r: 40, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 40, y: 44, size: 5, rotate: 0 },
  figure: { x: 70, y: 86, scale: 0.55, pose: "pose-sit", look: normalLookOf("meditativeplus-026") },
  extras: [
    { layer: "subject", x: 22, y: 80, scale: 0.5, pose: "pose-point", look: VILLAGER },
    { layer: "subject", x: 56, y: 78, scale: 0.46, flip: -1, pose: "pose-stand", look: { ...VILLAGER, hair: "hood", cloth: "blue-800" } },
    { layer: "mid", x: 30, y: 72, scale: 0.36, pose: "pose-raise", look: { ...VILLAGER, hair: "hair-long", cloth: "red-600" } },
  ],
  layers: {
    sky: [
      { part: "sun", x: 76, y: 52, scale: 0.9 },
      { part: "cloud", x: 26, y: 24, scale: 1.3, count: 2, spread: [20, 4], paint: "red-300" },
      { part: "bird", x: 60, y: 28, scale: 0.35, count: 3, spread: [12, 5], paint: "slate-900" },
    ],
    far: [
      { part: "peaks", x: 50, y: 64, scale: 0.9, paint: "violet-500" },
      { part: "mist", x: 50, y: 64, scale: 1.4, count: 2, spread: [20, 2], paint: "red-300" },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 74, paint: "wood-700" },
      { part: "ink-pine", x: 88, y: 76, scale: 0.8, flip: -1 },
    ],
    subject: [
      { part: "mossy-boulder", x: 38, y: 82, scale: 1.1 },
      { part: "card-frame", x: 38, y: 68, scale: 1.1 },
      { part: "brush-stroke", x: 80, y: 88, scale: 0.5 },
    ],
    near: [
      { part: "grass-tuft", x: 10, y: 98, count: 3, spread: [8, 2], paint: "wood-700" },
      { part: "seal", x: 90, y: 77 },
    ],
    light: [
      { part: "god-rays", x: 76, y: 52, scale: 1.3, paint: "orange-400" },
      { part: "halo", x: 38, y: 60, scale: 1.1, paint: "white" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.3, paint: "white" },
      { part: "sparks", x: 38, y: 58, scale: 0.8, paint: "white" },
    ],
  },
};

/** R3397000: Meditative+'s alt recipes, by card id. */
export const MEDITATIVE_PLUS_ALT_RECIPES: Readonly<Record<string, SceneRecipe>> = {
  "meditativeplus-001": BOBS_PLAN_ALT,
  "meditativeplus-006": BLOODBLOOM_ALT,
  "meditativeplus-008": ARCH_MAGE_ALT,
  "meditativeplus-010": MEDINA_GRIFTER_ALT,
  "meditativeplus-012": LIVING_ARMOR_POTION_ALT,
  "meditativeplus-013": LIVING_HEALTH_POTION_ALT,
  "meditativeplus-014": POTION_POUCH_ALT,
  "meditativeplus-015": BOOK_OF_POTIONS_ALT,
  "meditativeplus-017": DOGPILE_ALT,
  "meditativeplus-019": GRUBBY_GLOVES_ALT,
  "meditativeplus-020": KYS_COMPETITION_ALT,
  "meditativeplus-024": BOB_WISCONSINS_STRONGEST_ALT,
  "meditativeplus-025": KYS_INTUITION_ALT,
  "meditativeplus-026": SCRATCH_PAD_ALT,
};
