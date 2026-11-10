// R3352000: Classic's scene recipes, part 3 of three (#61 to #90 but Corpse Plantation, #74, #3331's proof, with the
// Glitch), one per entry, each drawn from its brief (#3352's table) in Classic's temple-stone look (ART.md,
// "Classic's scenes"): its subject within x 30 to 70 and y 25 to 85, its foreground near, its middle in the mid and
// subject layers and its background far, richer the rarer.
// Presentation only (CLAUDE.md rule 7): a recipe is read by the card's id alone.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** A plain villager of the Classic scenes: bareheaded, in a rough tunic, empty-handed. */
const VILLAGER: FigureLook = { ...figureLookFor(["Human"]), item: null, cloth: "wood-700" };
/** A hooded rogue with a blade, for the gambling den's ambush. */
const AMBUSHER: FigureLook = { ...figureLookFor(["Human"]), hair: "hood", item: "item-dagger", cloth: "slate-800" };
/** A teammate of the EU Striker's, in the same blue. */
const TEAMMATE: FigureLook = { ...figureLookFor(["Human"]), item: null, cloth: "blue-800" };
/** A hooded siphoner of the sunken road, in a dark robe. */
const SIPHONER: FigureLook = { ...figureLookFor(["Human"]), hair: "hood", outfit: "outfit-robe", item: null, skin: "slate-400", cloth: "slate-900" };
/** A spectator of Big Max's arena, recoiling behind the wall. */
const SPECTATOR: FigureLook = { ...figureLookFor(["Human"]), item: null, cloth: "brass-600" };
/** An enemy soldier dropped to one knee behind his tall shield. */
const SHIELD_SOLDIER: FigureLook = { ...figureLookFor(["Human"]), hair: "helm", outfit: "outfit-armor", item: "item-pavise", cloth: "red-900" };
/** A town guard in helm and mail, spear in hand. */
const GUARD: FigureLook = { ...figureLookFor(["Human"]), hair: "helm", outfit: "outfit-armor", item: "item-spear", cloth: "blue-800" };
/** A small petitioner at the senate's door, hat in hand. */
const PETITIONER: FigureLook = { ...figureLookFor(["Human"]), hair: "hat-brim", outfit: "outfit-coat", item: "item-card", cloth: "wood-700" };

/** Plague Bringer Goliath (Classic #61, Rare): a house-sized giant in rotting plate armour mid-sneeze, a green spore storm blasting from its helm over a hamlet's thatched roofs; murky olive sky. */
const PLAGUE_BRINGER_GOLIATH: SceneRecipe = {
  family: "classic",
  sky: ["green-900", "brass-800"],
  light: { x: 64, y: 24, r: 34, paint: "lime-400", opacity: 0.3 },
  emblem: { x: 50, y: 50, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 1.32,
    pose: "pose-stand",
    look: { ...figureLookFor(["Plague"]), hair: "steel-helmet", outfit: "outfit-plate", item: null, skin: "green-200", cloth: "wood-700" },
  },
  extras: [
    { layer: "near", x: 20, y: 94, scale: 0.3, pose: "pose-run", flip: -1, look: VILLAGER },
    { layer: "near", x: 84, y: 95, scale: 0.28, pose: "pose-run", look: { ...VILLAGER, cloth: "brass-600" } },
  ],
  layers: {
    sky: [{ part: "cloud", x: 24, y: 22, scale: 1.2, count: 2, spread: [14, 4], paint: "green-900" }],
    far: [
      { part: "hills", x: 50, y: 66, paint: "green-900" },
      { part: "barn", x: 16, y: 70, scale: 0.55, paint: "brass-600" },
      { part: "house", x: 86, y: 70, scale: 0.5, paint: "brass-600" },
    ],
    mid: [
      { part: "field", x: 50, y: 72, paint: "green-700" },
      { part: "house", x: 22, y: 82, scale: 0.7, paint: "brass-600" },
      { part: "house", x: 80, y: 84, scale: 0.75, flip: -1, paint: "brass-600" },
      { part: "fence", x: 50, y: 88, scale: 1.2, paint: "wood-700" },
    ],
    subject: [{ part: "smoke-billow", x: 64, y: 28, scale: 1.6, count: 2, spread: [8, 4], paint: "green-400" }],
    near: [
      { part: "grass-tuft", x: 50, y: 99, count: 2, spread: [30, 1], paint: "green-900" },
    ],
    light: [{ part: "halo", x: 66, y: 24, scale: 1.8, paint: "lime-400" }],
    particles: [
      { part: "spores", x: 70, y: 30, scale: 1.6, count: 2, spread: [16, 8], paint: "lime-400" },
      { part: "spore-motes", x: 50, y: 50, scale: 1.4, paint: "green-200" },
    ],
  },
};

/** Living Bomb (Classic #62, Rare): a quarantine square: a throbbing fever-bomb with glowing green veins is chained to the central fountain as feverish villagers edge away; tense olive gloom. */
const LIVING_BOMB: SceneRecipe = {
  family: "classic",
  sky: ["green-900", "slate-700"],
  light: { x: 50, y: 52, r: 34, paint: "green-400", opacity: 0.4 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 20, y: 88, scale: 0.5, flip: -1, pose: "pose-block", look: { ...VILLAGER, skin: "green-200" } },
    { layer: "subject", x: 80, y: 87, scale: 0.48, pose: "pose-block", look: { ...VILLAGER, skin: "green-200", cloth: "brass-800" } },
    { layer: "mid", x: 70, y: 72, scale: 0.34, pose: "pose-run", look: { ...VILLAGER, cloth: "slate-700" } },
  ],
  layers: {
    sky: [{ part: "cloud", x: 50, y: 20, scale: 1.4, count: 2, spread: [30, 3], paint: "green-900" }],
    far: [
      { part: "townhouse", x: 16, y: 64, scale: 1, paint: "slate-700" },
      { part: "townhouse", x: 84, y: 64, scale: 0.9, flip: -1, paint: "slate-700" },
      { part: "flag", x: 50, y: 36, scale: 1, paint: "lime-400" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66, paint: "stone-500" },
      { part: "rope-cordon", x: 30, y: 80, scale: 1 },
      { part: "rope-cordon", x: 70, y: 80, scale: 1 },
    ],
    subject: [
      { part: "fountain", x: 50, y: 84, scale: 2, paint: "stone-500" },
      { part: "orb", x: 50, y: 58, scale: 2.6, paint: "green-700" },
      { part: "chain-length", x: 50, y: 62, scale: 1.2, paint: "slate-400" },
    ],
    near: [
      { part: "barrel", x: 10, y: 98, scale: 1 },
      { part: "pebbles", x: 64, y: 98 },
    ],
    light: [
      { part: "halo", x: 50, y: 54, scale: 1.8, paint: "green-400" },
      { part: "rim-light", x: 50, y: 60, scale: 1.2, paint: "lime-400" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 52, scale: 1.2, count: 2, spread: [12, 6], paint: "lime-400" },
      { part: "motes", x: 50, y: 40, scale: 1.4, paint: "green-200" },
    ],
  },
};

/** Crop Dusting (Classic #63, Common): a hidden hatch in a windmill's sail springs open, spraying a fine green plague mist low across every field and farmhouse; humid summer dusk; the windmill is a stone tower under a ring of blades. */
const CROP_DUSTING: SceneRecipe = {
  family: "classic",
  sky: ["blue-800", "orange-400"],
  light: { x: 84, y: 58, r: 30, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 50, y: 62, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 26, y: 26, scale: 1, count: 2, spread: [12, 3], paint: "violet-500" }],
    far: [
      { part: "hills", x: 50, y: 62, paint: "green-900" },
      { part: "barn", x: 18, y: 66, scale: 0.45 },
      { part: "whitewashed-house", x: 84, y: 66, scale: 0.45 },
    ],
    mid: [
      { part: "furrows", x: 50, y: 70, paint: "green-700" },
      { part: "wheat", x: 26, y: 80, count: 3, spread: [12, 2] },
      { part: "whitewashed-house", x: 76, y: 80, scale: 0.6, flip: -1 },
    ],
    subject: [
      { part: "tower", x: 50, y: 80, scale: 1.2, paint: "stone-500" },
      { part: "blade-ring", x: 50, y: 38, scale: 1.8, paint: "wood-600" },
      { part: "hatch", x: 58, y: 34, scale: 0.6 },
    ],
    near: [
      { part: "mist", x: 50, y: 86, scale: 2.2, count: 3, spread: [30, 4], paint: "green-400" },
      { part: "wheat", x: 86, y: 98, count: 2, spread: [6, 1] },
    ],
    light: [{ part: "rim-light", x: 50, y: 50, scale: 1.4, paint: "orange-400" }],
    particles: [
      { part: "spore-motes", x: 62, y: 50, scale: 1.4, paint: "green-200" },
      { part: "spores", x: 64, y: 42, scale: 1.2, count: 2, spread: [14, 10], paint: "lime-400" },
    ],
  },
};

/** Malzahar's Recycler (Classic #64, Rare): a grinding basalt mill-machine in a dim mill-house swallows two cards into its hopper and spits two fresh ones out; violet void glow; the machine is a basalt furnace with a hopper and cogs. */
const MALZAHARS_RECYCLER: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 56, r: 34, paint: "violet-500", opacity: 0.45 },
  emblem: { x: 50, y: 50, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 70, paint: "slate-800" },
      { part: "round-window", x: 18, y: 30, scale: 1 },
      { part: "pipes", x: 84, y: 40, scale: 1.2 },
      { part: "chain-length", x: 30, y: 16, scale: 1 },
    ],
    mid: [
      { part: "boards", x: 50, y: 72, paint: "wood-800" },
      { part: "cog", x: 30, y: 54, scale: 2.4, paint: "slate-700" },
      { part: "gear", x: 72, y: 50, scale: 1.8, paint: "slate-700" },
      { part: "crate", x: 86, y: 80, scale: 1.4 },
    ],
    subject: [
      { part: "forge", x: 50, y: 80, scale: 1.4, paint: "slate-700" },
      { part: "glow-chute", x: 50, y: 30, scale: 1.4, paint: "violet-500" },
      { part: "void-swirl", x: 50, y: 70, scale: 0.8, paint: "violet-300" },
      { part: "playing-card", x: 44, y: 26, scale: 1.4 },
      { part: "playing-card", x: 56, y: 24, scale: 1.4 },
      { part: "card", x: 34, y: 86, scale: 1.2 },
      { part: "card", x: 66, y: 85, scale: 1.2 },
    ],
    near: [
      { part: "pebbles", x: 20, y: 96, scale: 1.2 },
      { part: "sacks", x: 84, y: 98, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 50, y: 70, scale: 1.8, paint: "violet-500" },
      { part: "rim-light", x: 50, y: 56, scale: 1.4, paint: "purple-400" },
    ],
    particles: [
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "violet-300" },
      { part: "sparks", x: 50, y: 66, scale: 1, count: 2, spread: [16, 6], paint: "purple-400" },
    ],
  },
};

/** Ace in the Hole (Classic #65, Common): a silver coin spins above a hidden pit as a hooded ally leaps out of the hole, blade drawn; smoky lamplit gambling den. */
const ACE_IN_THE_HOLE: SceneRecipe = {
  family: "classic",
  sky: ["wood-900", "brass-800"],
  light: { x: 24, y: 28, r: 30, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 50, y: 82, size: 5, rotate: 0 },
  figure: { x: 50, y: 76, scale: 0.78, pose: "pose-lunge", look: AMBUSHER },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 66, paint: "wood-800" },
      { part: "lantern", x: 22, y: 18, scale: 1.2 },
      { part: "lantern", x: 80, y: 20, scale: 1.2 },
      { part: "shelf", x: 84, y: 62, scale: 1 },
    ],
    mid: [
      { part: "boards", x: 50, y: 66, paint: "wood-700" },
      { part: "wide-table", x: 20, y: 82, scale: 0.9 },
      { part: "card-fan", x: 20, y: 70, scale: 0.7 },
      { part: "chair", x: 82, y: 82, scale: 0.9 },
    ],
    subject: [
      { part: "hatch", x: 50, y: 86, scale: 1.8, paint: "wood-800" },
      { part: "coin", x: 50, y: 30, scale: 4.5, paint: "slate-100" },
    ],
    near: [
      { part: "coin-stack", x: 84, y: 97, scale: 1.2 },
      { part: "dice", x: 72, y: 98 },
      { part: "playing-card", x: 18, y: 96, scale: 1.2, count: 2, spread: [8, 2] },
    ],
    light: [
      { part: "halo", x: 50, y: 32, scale: 1.2, paint: "slate-100" },
      { part: "halo", x: 22, y: 22, scale: 1, paint: "orange-400" },
    ],
    particles: [
      { part: "smoke", x: 76, y: 44, scale: 1.4, count: 2, spread: [16, 6], paint: "stone-500" },
      { part: "sparks", x: 50, y: 32, scale: 0.8, paint: "slate-100" },
    ],
  },
};

/** EU Striker (Classic #66, Common): a striker in a blue tunic volleys a leather ball across an ancient stone ball court, teammates charging behind; bright spring afternoon. */
const EU_STRIKER: SceneRecipe = {
  family: "classic",
  sky: ["sky-300", "blue-200"],
  light: { x: 80, y: 22, r: 36, paint: "white", opacity: 0.4 },
  emblem: { x: 46, y: 60, size: 5, rotate: 0 },
  figure: { x: 46, y: 84, scale: 1, pose: "pose-run", look: { ...figureLookFor(["Human"]), item: null, cloth: "blue-600" } },
  extras: [
    { layer: "mid", x: 22, y: 76, scale: 0.5, pose: "pose-run", look: TEAMMATE },
    { layer: "mid", x: 80, y: 74, scale: 0.46, flip: -1, pose: "pose-run", look: { ...TEAMMATE, cloth: "red-600" } },
  ],
  layers: {
    sky: [
      { part: "sun", x: 82, y: 20, scale: 0.8 },
      { part: "cloud", x: 26, y: 22, scale: 0.9, paint: "white" },
    ],
    far: [
      { part: "stands", x: 16, y: 64, scale: 1.4, paint: "stone-500" },
      { part: "stands", x: 84, y: 64, scale: 1.4, flip: -1, paint: "stone-500" },
      { part: "stone-wall", x: 50, y: 64, scale: 1.4, paint: "stone-500" },
      { part: "ring-stone", x: 72, y: 56, scale: 1.4 },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 66, paint: "stone-500" }],
    subject: [
      { part: "ball", x: 64, y: 44, scale: 2, paint: "wood-700" },
      { part: "streaks", x: 60, y: 46, scale: 0.8, paint: "white" },
    ],
    near: [
      { part: "grass-tuft", x: 14, y: 98, count: 2, spread: [6, 1] },
      { part: "flowers", x: 86, y: 98, scale: 0.9 },
      { part: "pebbles", x: 60, y: 97 },
    ],
    light: [{ part: "halo", x: 82, y: 20, scale: 1.6, paint: "white" }],
    particles: [{ part: "dust", x: 46, y: 88, scale: 1.2, paint: "stone-500" }],
  },
};

/** Felinor Feeler (Classic #67, Common): a human mystic in a cat-eared hood presses one palm to the floor and every enemy soldier around her drops behind his shield; incense-hazed hall. */
const FELINOR_FEELER: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "violet-700"],
  light: { x: 60, y: 76, r: 32, paint: "violet-500", opacity: 0.4 },
  emblem: { x: 46, y: 66, size: 5, rotate: 0 },
  figure: {
    x: 46,
    y: 86,
    scale: 1.15,
    pose: "pose-crouch",
    look: { ...figureLookFor(["Human"]), hair: "cat-hood", outfit: "outfit-travel-cloak", item: null, cloth: "violet-700" },
  },
  extras: [
    { layer: "subject", x: 16, y: 86, scale: 0.58, pose: "pose-kneel", look: SHIELD_SOLDIER },
    { layer: "subject", x: 84, y: 86, scale: 0.58, flip: -1, pose: "pose-kneel", look: SHIELD_SOLDIER },
    { layer: "mid", x: 70, y: 72, scale: 0.42, flip: -1, pose: "pose-kneel", look: { ...SHIELD_SOLDIER, cloth: "slate-700" } },
  ],
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 64, paint: "slate-800" },
      { part: "column", x: 12, y: 66, scale: 1.6, paint: "stone-500" },
      { part: "column", x: 88, y: 66, scale: 1.6, paint: "stone-500" },
      { part: "banner", x: 40, y: 30, scale: 0.9, paint: "violet-700" },
      { part: "candle", x: 30, y: 62, scale: 1.2 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 64, paint: "slate-700" },
      { part: "rune-circle", x: 60, y: 86, scale: 1.4, paint: "violet-300" },
    ],
    near: [{ part: "steam-curl", x: 30, y: 98, scale: 1.4, paint: "violet-300" }],
    light: [{ part: "halo", x: 60, y: 80, scale: 1.4, paint: "violet-500" }],
    particles: [
      { part: "steam-curl", x: 34, y: 48, scale: 1.2, count: 2, spread: [32, 4], paint: "violet-300" },
      { part: "motes", x: 50, y: 40, scale: 1.2, paint: "violet-300" },
    ],
  },
};

/** Small Card Lobbyist (Classic #68, Common): a towering giant in a pinstriped velvet suit looms over small petitioners, tapping a heavy ledger, a toll chain in his fist; marble senate antechamber. */
const SMALL_CARD_LOBBYIST: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "slate-400"],
  light: { x: 20, y: 30, r: 34, paint: "white", opacity: 0.3 },
  emblem: { x: 54, y: 50, size: 5, rotate: 0 },
  figure: {
    x: 54,
    y: 86,
    scale: 1.35,
    pose: "pose-stand",
    look: { ...figureLookFor([]), hair: "hair-short", outfit: "outfit-suit", item: "item-book", cloth: "violet-700" },
  },
  extras: [
    { layer: "subject", x: 24, y: 88, scale: 0.42, pose: "pose-raise", look: PETITIONER },
    { layer: "subject", x: 32, y: 90, scale: 0.38, pose: "pose-hold", look: { ...PETITIONER, hair: "hair-short", cloth: "blue-800" } },
    { layer: "subject", x: 80, y: 89, scale: 0.4, flip: -1, pose: "pose-genuflect", look: { ...PETITIONER, cloth: "red-900" } },
  ],
  layers: {
    far: [
      { part: "column", x: 12, y: 74, scale: 1.4, paint: "slate-100" },
      { part: "column", x: 88, y: 74, scale: 1.4, paint: "slate-100" },
      { part: "arch", x: 50, y: 74, scale: 1.8, paint: "slate-100" },
      { part: "banner", x: 24, y: 40, scale: 1, paint: "violet-700" },
    ],
    mid: [{ part: "checker-floor", x: 50, y: 76, paint: "slate-100" }],
    subject: [{ part: "chain-length", x: 40, y: 58, scale: 1.4, paint: "slate-100" }],
    near: [
      { part: "coin-stack", x: 86, y: 98, scale: 1.2 },
      { part: "scroll", x: 14, y: 98, scale: 1 },
    ],
    light: [{ part: "light-shaft", x: 22, y: 0, paint: "white" }],
    particles: [{ part: "motes", x: 40, y: 40, scale: 1.2, paint: "white" }],
  },
};

/** Plague Charger (Classic #69, Rare): a boil-covered warboar in rusted barding charges headlong, coughing green spores with every snort; muddy plague-marsh road, overcast; the boils are spore clusters on its flank. */
const PLAGUE_CHARGER: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "stone-500"],
  light: { x: 76, y: 60, r: 30, paint: "lime-400", opacity: 0.3 },
  emblem: { x: 44, y: 62, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 50, y: 22, scale: 1.4, count: 3, spread: [30, 4], paint: "slate-400" }],
    far: [
      { part: "hills", x: 50, y: 60, paint: "slate-700" },
      { part: "dead-tree", x: 16, y: 62, scale: 1.4 },
      { part: "dead-tree", x: 86, y: 60, scale: 1.1, flip: -1 },
    ],
    mid: [
      { part: "road", x: 50, y: 64, paint: "wood-800" },
      { part: "reed", x: 18, y: 76, count: 3, spread: [8, 2], paint: "green-900" },
      { part: "ripples", x: 82, y: 76, scale: 0.8, paint: "green-900" },
    ],
    subject: [
      { part: "boar", x: 48, y: 80, scale: 2.8, paint: "wood-700" },
      { part: "spores", x: 42, y: 66, scale: 0.8, count: 2, spread: [8, 3], paint: "lime-400" },
      { part: "smoke-billow", x: 70, y: 72, scale: 1.2, paint: "green-400" },
    ],
    near: [
      { part: "reed", x: 88, y: 98, count: 2, spread: [5, 1], paint: "green-900" },
      { part: "dust", x: 30, y: 94, scale: 1.4, paint: "wood-700" },
      { part: "pebbles", x: 60, y: 98 },
    ],
    light: [{ part: "rim-light", x: 50, y: 66, scale: 1.4, paint: "lime-400" }],
    particles: [
      { part: "spores", x: 74, y: 64, scale: 1.2, count: 2, spread: [12, 6], paint: "lime-400" },
      { part: "drops", x: 36, y: 86, scale: 1, paint: "wood-700" },
    ],
  },
};

/** Book of Plague (Classic #70, Epic): a moth-eaten tome opens and plague moths pour from its pages; they settle on a monastery's scriptorium desks, staining the scrolls green; a monk flees down a long cloister behind; hushed, creeping dread. */
const BOOK_OF_PLAGUE: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "green-900"],
  light: { x: 50, y: 66, r: 30, paint: "green-400", opacity: 0.35 },
  emblem: { x: 50, y: 78, size: 5, rotate: 0 },
  extras: [{ layer: "far", x: 62, y: 48, scale: 0.26, pose: "pose-run", look: { ...figureLookFor([]), hair: "tonsure", outfit: "outfit-robe", cloth: "wood-700" } }],
  layers: {
    far: [
      { part: "aisle", x: 50, y: 46, paint: "stone-500" },
      { part: "column", x: 22, y: 60, scale: 1.4, paint: "stone-500" },
      { part: "column", x: 78, y: 60, scale: 1.4, paint: "stone-500" },
      { part: "arch", x: 50, y: 50, scale: 0.8, paint: "slate-700" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66, paint: "slate-700" },
      { part: "desk", x: 20, y: 76, scale: 1.4 },
      { part: "scroll", x: 20, y: 64, scale: 0.9, paint: "green-400" },
      { part: "lectern", x: 82, y: 78, scale: 1.3 },
      { part: "scroll", x: 82, y: 64, scale: 0.7, paint: "green-400" },
      { part: "moth", x: 20, y: 60, scale: 0.8, count: 2, spread: [8, 3], paint: "green-200" },
    ],
    subject: [
      { part: "tome", x: 50, y: 84, scale: 2.6, paint: "wood-700" },
      { part: "moth", x: 50, y: 52, scale: 1.2, count: 4, spread: [16, 14], paint: "green-200" },
    ],
    near: [
      { part: "scroll", x: 16, y: 97, scale: 1.2, paint: "green-400" },
      { part: "candle", x: 84, y: 98, scale: 1.4 },
      { part: "moth", x: 70, y: 40, scale: 1.4, count: 2, spread: [10, 8], paint: "green-200" },
    ],
    light: [
      { part: "halo", x: 50, y: 72, scale: 1.8, paint: "green-400" },
      { part: "halo", x: 84, y: 86, scale: 0.8, paint: "orange-400" },
    ],
    particles: [
      { part: "spore-motes", x: 50, y: 56, scale: 1.4, paint: "green-200" },
      { part: "dust", x: 50, y: 30, scale: 1, paint: "green-400" },
    ],
  },
};

/** Lane Eater (Classic #71, Common): a gaping worm-maw bursts up through a stone lane, swallowing flagstones, dinner plates and a startled guard whole; cracked aqueduct road, dust clouds. */
const LANE_EATER: SceneRecipe = {
  family: "classic",
  sky: ["blue-200", "stone-500"],
  light: { x: 22, y: 26, r: 34, paint: "white", opacity: 0.3 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  extras: [{ layer: "subject", x: 52, y: 44, scale: 0.4, flip: -1, pose: "pose-spread", look: GUARD }],
  layers: {
    far: [
      { part: "hills", x: 50, y: 58, paint: "stone-500" },
      { part: "bridge", x: 50, y: 46, scale: 1.8, paint: "stone-500" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 64, paint: "stone-500" },
      { part: "shards", x: 24, y: 72, scale: 1.2, paint: "stone-500" },
    ],
    subject: [
      { part: "maw", x: 50, y: 86, scale: 1.3 },
      { part: "jade-disc", x: 36, y: 40, scale: 0.8, paint: "slate-100" },
      { part: "jade-disc", x: 66, y: 34, scale: 0.7, paint: "slate-100" },
      { part: "shards", x: 62, y: 50, scale: 1, paint: "stone-500" },
    ],
    near: [
      { part: "dust-cloud", x: 22, y: 92, scale: 2, paint: "stone-500" },
      { part: "dust-cloud", x: 80, y: 92, scale: 1.8, paint: "stone-500" },
      { part: "pebbles", x: 50, y: 98 },
    ],
    particles: [{ part: "dust", x: 50, y: 70, scale: 1.4, paint: "stone-500" }],
  },
};

/** Grand Counterspell (Classic #72, Rare): a ring of four warding obelisks flares at once, shattering a spell, a rune-trap and a summoned tower mid-cast; cliffside sanctum, icy blue. */
const GRAND_COUNTERSPELL: SceneRecipe = {
  family: "classic",
  sky: ["blue-900", "sky-400"],
  light: { x: 50, y: 54, r: 38, paint: "sky-300", opacity: 0.45 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "stars", x: 30, y: 22, scale: 1 }],
    far: [
      { part: "peaks", x: 50, y: 58, paint: "slate-700" },
      { part: "cliff", x: 104, y: 50, scale: 0.8, paint: "slate-800" },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 66, paint: "slate-700" },
      { part: "rune-circle", x: 50, y: 74, scale: 1.8, paint: "sky-300" },
    ],
    subject: [
      { part: "standing-stone", x: 38, y: 68, scale: 1.2, paint: "slate-400" },
      { part: "standing-stone", x: 62, y: 68, scale: 1.2, paint: "slate-400" },
      { part: "tower", x: 50, y: 64, scale: 0.5, paint: "violet-500" },
      { part: "rune", x: 50, y: 74, scale: 1.2, paint: "violet-300" },
      { part: "fireball", x: 56, y: 42, scale: 1, paint: "orange-400" },
      { part: "shards", x: 50, y: 50, scale: 1.6, paint: "sky-300" },
      { part: "standing-stone", x: 30, y: 84, scale: 1.6, paint: "slate-400" },
      { part: "standing-stone", x: 70, y: 84, scale: 1.6, flip: -1, paint: "slate-400" },
    ],
    near: [
      { part: "pebbles", x: 50, y: 97, count: 2, spread: [30, 1] },
      { part: "shards", x: 16, y: 92, scale: 1, paint: "sky-300" },
    ],
    light: [
      { part: "halo", x: 50, y: 52, scale: 2.4, paint: "sky-300" },
      { part: "zap", x: 32, y: 66, scale: 0.6, paint: "white" },
      { part: "zap", x: 68, y: 66, scale: 0.6, flip: -1, paint: "white" },
    ],
    particles: [
      { part: "snow", x: 50, y: 40, scale: 1.2, paint: "white" },
      { part: "sparks", x: 50, y: 52, scale: 1.2, count: 2, spread: [14, 8], paint: "sky-300" },
    ],
  },
};

/** Nurse Cleaver (Classic #73, Common): a brisk field nurse in a bloodied white apron swings a heavy cleaver while wrapping a bandage round her own arm; battlefield tent hospital. */
const NURSE_CLEAVER: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "stone-500"],
  light: { x: 76, y: 30, r: 32, paint: "white", opacity: 0.3 },
  emblem: { x: 50, y: 60, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 84,
    scale: 1.05,
    pose: "pose-strike",
    look: { ...figureLookFor([]), hair: "kerchief", outfit: "outfit-apron", item: "item-cleaver", cloth: "white" },
  },
  layers: {
    far: [
      { part: "tent", x: 50, y: 66, scale: 2.8, paint: "stone-500" },
      { part: "camp-tent", x: 12, y: 66, scale: 1.2, paint: "stone-500" },
      { part: "camp-tent", x: 88, y: 66, scale: 1.1, flip: -1, paint: "stone-500" },
      { part: "flag", x: 80, y: 40, scale: 0.8, paint: "white" },
    ],
    mid: [
      { part: "yard-sand", x: 50, y: 66, paint: "wood-700" },
      { part: "cot", x: 18, y: 80, scale: 1.4 },
      { part: "soldier-fallen", x: 18, y: 74, scale: 1.1 },
      { part: "cot", x: 82, y: 80, scale: 1.3, flip: -1 },
    ],
    subject: [
      { part: "napkin", x: 42, y: 58, scale: 0.6, paint: "white" },
      { part: "drops", x: 52, y: 70, scale: 0.6, paint: "red-600" },
    ],
    near: [
      { part: "pail", x: 84, y: 98, scale: 1.2 },
      { part: "towels", x: 14, y: 98, scale: 1, paint: "white" },
    ],
    particles: [{ part: "dust", x: 50, y: 88, scale: 1, paint: "stone-500" }],
  },
};

/** Argusland (Classic #75, Rare): a highland fortress whose walls are studded with watchful stone eyes, incoming arrows splitting into harmless splinters at its gates; misty grey-green dawn. */
const ARGUSLAND: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "green-200"],
  light: { x: 14, y: 58, r: 30, paint: "white", opacity: 0.35 },
  emblem: { x: 50, y: 46, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 72, y: 22, scale: 1, count: 2, spread: [14, 3], paint: "slate-400" }],
    far: [
      { part: "mountains", x: 50, y: 56, paint: "slate-700" },
      { part: "mist", x: 50, y: 56, scale: 2.4, count: 2, spread: [30, 2], paint: "white" },
    ],
    mid: [
      { part: "hills", x: 50, y: 70, paint: "green-900" },
      { part: "pine", x: 14, y: 78, scale: 1.2, count: 2, spread: [6, 2] },
      { part: "pine", x: 88, y: 78, scale: 1.1 },
    ],
    subject: [
      { part: "fortress", x: 50, y: 76, scale: 1.7, paint: "stone-500" },
      { part: "eye", x: 36, y: 60, scale: 1.2 },
      { part: "eye", x: 64, y: 60, scale: 1.2 },
      { part: "eye", x: 50, y: 56, scale: 1 },
      { part: "eye", x: 22, y: 52, scale: 1 },
      { part: "eye", x: 78, y: 52, scale: 1 },
      { part: "splinters", x: 42, y: 68, scale: 1.2, paint: "wood-600" },
    ],
    near: [
      { part: "flying-arrow", x: 36, y: 66, scale: 1.1 },
      { part: "flying-arrow", x: 30, y: 58, scale: 0.9 },
      { part: "flying-arrow", x: 24, y: 72, scale: 1 },
      { part: "grass-tuft", x: 72, y: 98, count: 2, spread: [10, 1], paint: "green-900" },
    ],
    light: [{ part: "halo", x: 50, y: 58, scale: 1.6, paint: "cyan-400" }],
    particles: [
      { part: "splinters", x: 38, y: 62, scale: 0.8, paint: "wood-600" },
      { part: "mist", x: 50, y: 92, scale: 2.2, paint: "white" },
    ],
  },
};

/** Plague Bringer (Classic #76, Rare): a hooded plague rider on a sneezing, mangy mule gallops through a walled town's toll gate, a stolen keepsake swinging from his saddle; sickly haze. */
const PLAGUE_BRINGER: SceneRecipe = {
  family: "classic",
  sky: ["green-900", "slate-700"],
  light: { x: 22, y: 30, r: 34, paint: "lime-400", opacity: 0.3 },
  emblem: { x: 64, y: 30, size: 5, rotate: 0 },
  figure: {
    x: 48,
    y: 73,
    scale: 0.65,
    pose: "pose-sit",
    look: { ...figureLookFor(["Plague"]), hair: "hood", outfit: "outfit-poncho", item: null, skin: "green-200", cloth: "slate-800" },
  },
  layers: {
    sky: [
      { part: "cloud", x: 50, y: 18, scale: 1.4, count: 2, spread: [30, 4], paint: "slate-400" },
      { part: "mist", x: 50, y: 34, scale: 1.6, paint: "lime-400", fade: 0.5 },
    ],
    far: [
      { part: "stone-wall", x: 50, y: 66, scale: 1.4, paint: "stone-500" },
      { part: "tower", x: 14, y: 66, scale: 1.1, paint: "slate-400" },
      { part: "tower", x: 86, y: 66, scale: 1.1, flip: -1, paint: "slate-400" },
      { part: "gate", x: 50, y: 68, scale: 1.3, paint: "wood-800" },
      { part: "banner", x: 30, y: 40, scale: 0.7, paint: "green-700" },
    ],
    mid: [
      { part: "cobbles", x: 50, y: 70, paint: "slate-400" },
      { part: "fence-post", x: 24, y: 78, scale: 1.1, paint: "wood-700" },
      { part: "lantern", x: 78, y: 54, scale: 0.9 },
    ],
    subject: [
      { part: "mule", x: 50, y: 86, scale: 1.7, paint: "slate-400" },
      { part: "spores", x: 72, y: 72, scale: 0.9, count: 2, spread: [6, 3], paint: "lime-400" },
    ],
    near: [
      { part: "coins", x: 20, y: 94, scale: 1.1 },
      { part: "dust", x: 34, y: 92, scale: 1.4, paint: "wood-700" },
      { part: "pebbles", x: 82, y: 97 },
    ],
    particles: [{ part: "spore-motes", x: 50, y: 56, scale: 1.4, paint: "lime-400" }],
  },
};

/** Anti-Magic Monkey (Classic #77, Common): a scowling grey ape snatches a spellbook from a startled wizard and rips out its pages; jungle temple ruins, green-dappled light. */
const ANTI_MAGIC_MONKEY: SceneRecipe = {
  family: "classic",
  sky: ["green-900", "jade-400"],
  light: { x: 26, y: 24, r: 36, paint: "lime-400", opacity: 0.35 },
  emblem: { x: 18, y: 40, size: 5, rotate: 0 },
  figure: {
    x: 42,
    y: 86,
    scale: 1.05,
    pose: "pose-raise",
    look: { ...figureLookFor([]), head: "head-ape", hair: null, outfit: "outfit-fur", item: "item-book", skin: "grey-500", cloth: "grey-500" },
  },
  extras: [
    {
      layer: "subject",
      x: 76,
      y: 86,
      scale: 0.7,
      flip: -1,
      pose: "pose-block",
      look: { ...figureLookFor(["Human"]), hair: "hat-pointed", outfit: "outfit-robe", item: null, cloth: "blue-800" },
    },
  ],
  layers: {
    far: [
      { part: "temple", x: 50, y: 66, scale: 1.3, paint: "green-900" },
      { part: "vine", x: 16, y: 36, scale: 1.2, paint: "green-700" },
      { part: "vine", x: 86, y: 40, scale: 1, flip: -1, paint: "green-700" },
    ],
    mid: [
      { part: "fallen-column", x: 22, y: 78, scale: 0.9, paint: "stone-500" },
      { part: "fern", x: 88, y: 80, scale: 1.1, paint: "green-700" },
    ],
    near: [
      { part: "fern", x: 10, y: 98, scale: 1.2, paint: "green-900" },
      { part: "paper-sheet", x: 62, y: 94, scale: 0.5 },
    ],
    light: [{ part: "god-rays", x: 30, y: 0, scale: 1, paint: "lime-400" }],
    particles: [{ part: "paper-sheet", x: 58, y: 36, scale: 0.4, count: 3, spread: [12, 8] }],
  },
};

/** Mutate Spell (Classic #78, Rare): a mutation garden in a ruined greenhouse: twisted creatures in glass bell jars, a green plague helix weaving between them; eerie dusk; the helix is a twisting ribbon of light. */
const MUTATE_SPELL: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "violet-500"],
  light: { x: 50, y: 56, r: 34, paint: "green-400", opacity: 0.35 },
  emblem: { x: 50, y: 34, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "greenhouse", x: 50, y: 70, scale: 1.3, paint: "slate-400" },
      { part: "vine", x: 18, y: 50, scale: 1.4, count: 2, spread: [6, 4] },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 70, paint: "green-900" },
      { part: "fern", x: 14, y: 80, scale: 1.4 },
      { part: "bush", x: 86, y: 80, scale: 1 },
    ],
    subject: [
      { part: "grub", x: 33, y: 80, scale: 0.9, paint: "lime-400" },
      { part: "jar", x: 32, y: 82, scale: 3, paint: "slate-100" },
      { part: "beetle", x: 50, y: 74, scale: 1.4, paint: "violet-500" },
      { part: "jar", x: 50, y: 76, scale: 3.2, paint: "slate-100" },
      { part: "worm", x: 68, y: 80, scale: 2, paint: "red-400" },
      { part: "jar", x: 68, y: 82, scale: 3, paint: "slate-100" },
      { part: "light-ribbon", x: 50, y: 58, scale: 1.3, paint: "green-400" },
    ],
    near: [
      { part: "mushroom", x: 14, y: 98, scale: 1.6, paint: "green-700" },
      { part: "shards", x: 84, y: 94, scale: 1, paint: "slate-100" },
      { part: "pebbles", x: 50, y: 98 },
    ],
    light: [{ part: "halo", x: 50, y: 58, scale: 2, paint: "green-400" }],
    particles: [
      { part: "spore-motes", x: 50, y: 50, scale: 1.4, paint: "lime-200" },
      { part: "motes", x: 50, y: 66, scale: 1.2, paint: "green-200" },
    ],
  },
};

/** Risky Die (Classic #79, Common): a huge carved bone die tumbles across a tavern table; the cheap cards it scatters stay, the pricey ones vanish in smoke; smoky lamplight. */
const RISKY_DIE: SceneRecipe = {
  family: "classic",
  sky: ["wood-900", "wood-700"],
  light: { x: 76, y: 26, r: 32, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 50, y: 50, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 64, paint: "wood-800" },
      { part: "lantern", x: 76, y: 18, scale: 1.3 },
      { part: "shelf", x: 20, y: 56, scale: 1 },
      { part: "barrel", x: 86, y: 62, scale: 1 },
    ],
    mid: [{ part: "boards", x: 50, y: 64, paint: "wood-700" }],
    subject: [
      { part: "wide-table", x: 50, y: 88, scale: 2.4 },
      { part: "die", x: 50, y: 52, scale: 6, paint: "slate-100" },
      { part: "playing-card", x: 32, y: 68, scale: 1.4 },
      { part: "playing-card", x: 40, y: 72, scale: 1.3 },
      { part: "card", x: 68, y: 66, scale: 1.2, paint: "violet-500" },
      { part: "smoke", x: 70, y: 66, scale: 1.4, paint: "violet-300" },
    ],
    near: [
      { part: "cup", x: 16, y: 96, scale: 3 },
      { part: "coins", x: 84, y: 97, scale: 1.2 },
    ],
    light: [{ part: "halo", x: 76, y: 22, scale: 1.2, paint: "orange-400" }],
    particles: [
      { part: "smoke", x: 30, y: 40, scale: 1.2, paint: "stone-500" },
      { part: "motes", x: 50, y: 40, scale: 1, paint: "orange-400" },
    ],
  },
};

/** BOOM! Big Max (Classic #80, Legendary): BOOM! Big Max, a towering barrel-chested demolitions champion in scorched riveted plate, lands fist-first in the middle of a stone arena, the ground exploding into a smoking crater around him; three sacrificed allies' shields spin away in the blast and spectators recoil behind the walls; a hard white-orange flash and roiling dust; his fist is raised for the blow. */
const BOOM_BIG_MAX: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "orange-400"],
  light: { x: 50, y: 74, r: 44, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 54, size: 6, rotate: 0 },
  figure: {
    x: 50,
    y: 82,
    scale: 1.25,
    pose: "pose-strike",
    look: { ...figureLookFor(["Acclaimed"]), hair: "steel-helmet", outfit: "outfit-layered-plate", item: null, skin: "red-300", cloth: "slate-700" },
  },
  extras: [
    { layer: "far", x: 14, y: 55, scale: 0.28, pose: "pose-block", look: SPECTATOR },
    { layer: "far", x: 27, y: 54, scale: 0.26, pose: "pose-spread", look: { ...SPECTATOR, cloth: "blue-800" } },
    { layer: "far", x: 82, y: 55, scale: 0.27, flip: -1, pose: "pose-block", look: { ...SPECTATOR, cloth: "red-900" } },
  ],
  layers: {
    sky: [
      { part: "smoke-billow", x: 22, y: 22, scale: 1.6, paint: "slate-400" },
      { part: "cloud", x: 78, y: 20, scale: 1.4, paint: "slate-400" },
    ],
    far: [
      { part: "amphitheatre", x: 50, y: 50, scale: 1.6, paint: "stone-500" },
      { part: "ruin-wall", x: 18, y: 64, scale: 1.4, paint: "stone-500" },
      { part: "ruin-wall", x: 82, y: 64, scale: 1.4, flip: -1, paint: "stone-500" },
      { part: "banner", x: 36, y: 40, scale: 0.8, paint: "red-600" },
      { part: "banner", x: 62, y: 40, scale: 0.8, paint: "blue-700" },
    ],
    mid: [
      { part: "sand", x: 50, y: 66, paint: "stone-500" },
      { part: "shockwave", x: 50, y: 84, scale: 1.6, paint: "white" },
      { part: "shield", x: 18, y: 42, scale: 1.8, paint: "red-600" },
      { part: "shield", x: 82, y: 36, scale: 1.6, paint: "blue-700" },
      { part: "shield", x: 76, y: 66, scale: 1.4, paint: "brass-600" },
      { part: "dust-cloud", x: 24, y: 74, scale: 2, paint: "stone-500" },
    ],
    subject: [{ part: "blast-rings", x: 50, y: 84, scale: 2.4, paint: "orange-400" }],
    near: [
      { part: "shards", x: 18, y: 90, scale: 1.4, paint: "stone-500" },
      { part: "shards", x: 82, y: 88, scale: 1.2, flip: -1, paint: "stone-500" },
      { part: "broken-spear", x: 84, y: 98, scale: 0.8 },
      { part: "embers", x: 50, y: 94, scale: 1.6, count: 2, spread: [30, 2], paint: "ember-500" },
    ],
    light: [
      { part: "burst", x: 50, y: 82, scale: 2.2, paint: "white" },
      { part: "halo", x: 50, y: 80, scale: 2.6, paint: "orange-400" },
      { part: "rim-light", x: 50, y: 60, scale: 1.6, paint: "white" },
      { part: "god-rays", x: 50, y: 82, scale: 1, paint: "white" },
    ],
    particles: [
      { part: "embers", x: 50, y: 70, scale: 1.6, count: 2, spread: [30, 12], paint: "orange-400" },
      { part: "sparks", x: 50, y: 78, scale: 1.4, paint: "white" },
    ],
  },
};

/** The Power to Thrive (Classic #81, Rare): a hilltop orchard sanctuary: a healing fountain, a tree heavy with card-shaped leaves and an altar where a heavy stone floats light; gentle green sunlight. */
const THE_POWER_TO_THRIVE: SceneRecipe = {
  family: "classic",
  sky: ["sky-300", "green-200"],
  light: { x: 80, y: 20, r: 38, paint: "lime-200", opacity: 0.4 },
  emblem: { x: 50, y: 34, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 82, y: 18, scale: 0.8 },
      { part: "cloud", x: 22, y: 22, scale: 0.8, paint: "white" },
    ],
    far: [
      { part: "hills", x: 50, y: 64, paint: "green-700" },
      { part: "fruit-tree", x: 12, y: 66, scale: 0.7 },
      { part: "fruit-tree", x: 88, y: 66, scale: 0.65, flip: -1 },
    ],
    mid: [
      { part: "field", x: 50, y: 74, paint: "green-400" },
      { part: "flowers", x: 22, y: 80, count: 2, spread: [8, 2] },
    ],
    subject: [
      { part: "fruit-tree", x: 50, y: 76, scale: 1.6, paint: "green-700" },
      { part: "playing-card", x: 44, y: 40, scale: 1, count: 4, spread: [14, 10] },
      { part: "heal-fountain", x: 32, y: 86, scale: 1.4 },
      { part: "altar", x: 70, y: 86, scale: 1.2 },
      { part: "lifting-stone", x: 70, y: 60, scale: 0.6, paint: "stone-500" },
    ],
    near: [
      { part: "grass-tuft", x: 14, y: 98, count: 2, spread: [6, 1] },
      { part: "flowers", x: 86, y: 98, scale: 1 },
    ],
    light: [
      { part: "god-rays", x: 80, y: 6, scale: 0.9, paint: "lime-200" },
      { part: "halo", x: 70, y: 64, scale: 0.9, paint: "green-200" },
    ],
    particles: [
      { part: "heal-glow", x: 32, y: 70, scale: 1.2, paint: "green-200" },
      { part: "leaves", x: 56, y: 44, scale: 1, paint: "green-400" },
    ],
  },
};

/** Sheeople (Classic #82, Common): a meek sheep-headed villager in a woolly smock shuffles in line behind the flock, a card dropping behind it; foggy hillside pasture. */
const SHEEOPLE: SceneRecipe = {
  family: "classic",
  sky: ["slate-400", "slate-100"],
  light: { x: 24, y: 30, r: 34, paint: "white", opacity: 0.35 },
  emblem: { x: 80, y: 34, size: 5, rotate: 0 },
  figure: {
    x: 40,
    y: 84,
    scale: 0.85,
    pose: "pose-hold",
    look: { ...figureLookFor(["Human"]), head: "head-sheep", hair: null, outfit: "outfit-sweater", item: null, skin: "slate-700", cloth: "slate-100" },
  },
  layers: {
    sky: [{ part: "mist", x: 50, y: 34, scale: 1.8, count: 2, spread: [30, 4], paint: "white" }],
    far: [
      { part: "hills", x: 50, y: 62, paint: "green-700" },
      { part: "fence", x: 50, y: 66, scale: 1.2, paint: "wood-700" },
    ],
    mid: [
      { part: "field", x: 50, y: 70, paint: "green-400" },
      { part: "mist", x: 50, y: 70, scale: 2, paint: "white", fade: 0.6 },
    ],
    subject: [
      { part: "sheep", x: 64, y: 84, scale: 1.5 },
      { part: "sheep", x: 86, y: 82, scale: 1.4 },
      { part: "card", x: 20, y: 72, scale: 1.3, flip: -1 },
      { part: "streaks", x: 20, y: 64, scale: 0.4, paint: "white" },
    ],
    near: [
      { part: "grass-tuft", x: 14, y: 96, count: 2, spread: [6, 1], paint: "green-700" },
      { part: "mist", x: 50, y: 92, scale: 2, paint: "white", fade: 0.6 },
    ],
  },
};

/** Flame Lance (Classic #83, Common): a spear of fire skewers an armoured ogre and keeps flying through the wall behind it; castle rampart, smoky orange dusk. */
const FLAME_LANCE: SceneRecipe = {
  family: "classic",
  sky: ["red-900", "orange-400"],
  light: { x: 70, y: 50, r: 34, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 46, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 46,
    y: 84,
    scale: 1.05,
    pose: "pose-spread",
    look: { head: "head-ogre", hair: null, outfit: "outfit-armor", item: null, skin: "green-700", cloth: "slate-700" },
  },
  layers: {
    sky: [{ part: "smoke-billow", x: 24, y: 24, scale: 1.4, paint: "slate-700" }],
    far: [
      { part: "battlement", x: 50, y: 68, scale: 2.2, paint: "stone-500" },
      { part: "tower", x: 12, y: 68, scale: 1, paint: "stone-500" },
      { part: "burst", x: 86, y: 52, scale: 1.2, paint: "orange-400" },
      { part: "fireball", x: 96, y: 46, scale: 0.8, paint: "ember-500" },
    ],
    mid: [{ part: "cobbles", x: 50, y: 70, paint: "stone-500" }],
    subject: [{ part: "fireball", x: 70, y: 56, scale: 2.8, paint: "ember-500" }],
    near: [
      { part: "broken-spear", x: 22, y: 96, scale: 0.9 },
      { part: "pebbles", x: 78, y: 97 },
      { part: "smoke", x: 88, y: 96, scale: 1.2, paint: "stone-500" },
    ],
    light: [{ part: "halo", x: 66, y: 56, scale: 1.6, paint: "orange-400" }],
    particles: [
      { part: "embers", x: 60, y: 50, scale: 1.4, count: 2, spread: [20, 10], paint: "orange-400" },
      { part: "sparks", x: 84, y: 52, scale: 1, paint: "orange-400" },
    ],
  },
};

/** Lockdown (Classic #84, Rare): a prison citadel's inner yard sealing tight: iron portcullises dropping over every gate, bolts sliding home in every door; cold steel-blue lantern light; the bolts are padlocks on the doors. */
const LOCKDOWN: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "blue-800"],
  light: { x: 50, y: 30, r: 34, paint: "blue-600", opacity: 0.35 },
  emblem: { x: 50, y: 56, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "moon", x: 80, y: 18, scale: 0.7 }],
    far: [
      { part: "long-wall", x: 50, y: 52, paint: "slate-700" },
      { part: "tower", x: 10, y: 66, scale: 1.2, paint: "slate-700" },
      { part: "tower", x: 90, y: 66, scale: 1.2, paint: "slate-700" },
    ],
    mid: [
      { part: "cobbles", x: 50, y: 70, paint: "slate-700" },
      { part: "door", x: 20, y: 76, scale: 1.4, paint: "slate-800" },
      { part: "padlock", x: 22, y: 66, scale: 1.2, paint: "slate-400" },
      { part: "door", x: 80, y: 76, scale: 1.4, paint: "slate-800" },
      { part: "padlock", x: 78, y: 66, scale: 1.2, paint: "slate-400" },
      { part: "lantern", x: 30, y: 46, scale: 1.2 },
      { part: "lantern", x: 70, y: 46, scale: 1.2 },
    ],
    subject: [
      { part: "gate", x: 50, y: 80, scale: 1.9, paint: "slate-800" },
      { part: "portcullis", x: 50, y: 78, scale: 1.7, paint: "slate-400" },
      { part: "chain-length", x: 50, y: 40, scale: 1.2, paint: "slate-400" },
    ],
    near: [
      { part: "chain", x: 6, y: 94, scale: 1, paint: "slate-400" },
      { part: "key", x: 84, y: 96, scale: 1.6, paint: "slate-400" },
      { part: "pebbles", x: 50, y: 97 },
    ],
    light: [
      { part: "halo", x: 30, y: 50, scale: 1, paint: "blue-600" },
      { part: "halo", x: 70, y: 50, scale: 1, paint: "blue-600" },
    ],
    particles: [
      { part: "dust", x: 50, y: 82, scale: 1.4, paint: "slate-400" },
      { part: "motes", x: 50, y: 40, scale: 1.2, paint: "blue-200" },
    ],
  },
};

/** King Wagtoggle (Classic #85, Legendary): a round, grinning old king in a patched velvet cloak and a lopsided silver crown shakes a rival's hand across a banquet table while his other hand, behind his back, swaps the two players' decks; courtiers gasp in a warm crimson banquet hall, a sly moment of trade. */
const KING_WAGTOGGLE: SceneRecipe = {
  family: "classic",
  sky: ["red-900", "wood-800"],
  light: { x: 52, y: 34, r: 40, paint: "red-300", opacity: 0.4 },
  emblem: { x: 50, y: 26, size: 6, rotate: 0 },
  figure: {
    x: 38,
    y: 84,
    scale: 1.05,
    pose: "pose-point",
    look: { ...figureLookFor(["Human"]), hair: "king-crown", outfit: "outfit-travel-cloak", item: null, skin: "red-300", cloth: "red-600" },
  },
  extras: [
    {
      layer: "subject",
      x: 66,
      y: 84,
      scale: 0.95,
      flip: -1,
      pose: "pose-point",
      look: { ...figureLookFor(["Human"]), hair: "hair-short", outfit: "outfit-tabard", item: null, cloth: "blue-800" },
    },
    { layer: "mid", x: 16, y: 74, scale: 0.42, pose: "pose-block", look: { ...figureLookFor(["Human"]), hair: "hat-brim", outfit: "outfit-gown", item: null, cloth: "violet-700" } },
    { layer: "mid", x: 84, y: 74, scale: 0.4, flip: -1, pose: "pose-block", look: { ...figureLookFor(["Human"]), hair: "hair-long", outfit: "outfit-coat", item: null, cloth: "green-700" } },
  ],
  layers: {
    sky: [
      { part: "room-wall", x: 50, y: 66, paint: "red-900" },
      { part: "arch", x: 50, y: 66, scale: 2.4, paint: "wood-800" },
    ],
    far: [
      { part: "column", x: 8, y: 84, scale: 2.2, paint: "stone-500" },
      { part: "column", x: 92, y: 84, scale: 2.2, paint: "stone-500" },
      { part: "banner", x: 28, y: 30, scale: 1, paint: "red-600" },
      { part: "banner", x: 72, y: 30, scale: 1, flip: -1, paint: "red-600" },
      { part: "chandelier", x: 50, y: 6, scale: 1.3 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 74, paint: "wood-700" },
    ],
    subject: [
      { part: "deck", x: 26, y: 58, scale: 1.2 },
      { part: "deck", x: 18, y: 62, scale: 1.1, flip: -1 },
      { part: "streaks", x: 22, y: 58, scale: 0.6, paint: "white" },
    ],
    near: [
      { part: "wide-table", x: 52, y: 98, scale: 1.4, paint: "wood-700" },
      { part: "bread", x: 40, y: 85, scale: 1 },
      { part: "cup", x: 52, y: 85, scale: 0.9 },
      { part: "candle", x: 72, y: 86, scale: 1 },
    ],
    light: [
      { part: "halo", x: 50, y: 6, scale: 1.4, paint: "red-300" },
      { part: "rim-light", x: 50, y: 56, scale: 1.6, paint: "red-300" },
      { part: "halo", x: 22, y: 60, scale: 1, paint: "white" },
    ],
    particles: [
      { part: "motes", x: 50, y: 40, scale: 1.4, paint: "red-300" },
    ],
  },
};

/** Genn (Classic #86, Common): Genn, a huge, calm, bald giant in a plain undyed tunic, stands arms folded in an empty quarry, needing nothing else; flat honest daylight; his arms rest on his hips. */
const GENN: SceneRecipe = {
  family: "classic",
  sky: ["blue-200", "slate-100"],
  light: { x: 50, y: 20, r: 40, paint: "white", opacity: 0.3 },
  emblem: { x: 50, y: 52, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 1.36,
    pose: "pose-hips",
    look: { ...figureLookFor([]), hair: null, outfit: "outfit-tunic", item: null, skin: "brass-500", cloth: "slate-100" },
  },
  layers: {
    sky: [{ part: "cloud", x: 78, y: 20, scale: 0.8, paint: "white" }],
    far: [
      { part: "terraces", x: 50, y: 54, paint: "stone-500" },
      { part: "cliff", x: -6, y: 30, scale: 0.7, paint: "stone-500" },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 70, paint: "stone-500" },
      { part: "lifting-stone", x: 18, y: 80, scale: 2 },
      { part: "lifting-stone", x: 84, y: 82, scale: 1.6 },
    ],
    near: [
      { part: "pebbles", x: 18, y: 97 },
      { part: "shovel", x: 88, y: 98, scale: 1 },
    ],
    light: [{ part: "halo", x: 50, y: 20, scale: 1.6, paint: "white" }],
    particles: [{ part: "dust", x: 30, y: 84, scale: 1, paint: "stone-500" }],
  },
};

/** Plague Chalice (Classic #87, Epic): foreground, a stone chalice brimming with green plague foam on an altar, plague beads ringing its rim; middle, incoming spells fizzle against its mist in a crypt chapel; background, skull niches lit by sickly green candles; forbidding mood. */
const PLAGUE_CHALICE: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "green-900"],
  light: { x: 50, y: 60, r: 36, paint: "green-400", opacity: 0.4 },
  emblem: { x: 50, y: 78, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "room-wall", x: 50, y: 62, paint: "slate-800" },
      { part: "arch", x: 16, y: 50, scale: 0.7, paint: "slate-700" },
      { part: "arch", x: 84, y: 50, scale: 0.7, paint: "slate-700" },
      { part: "arch", x: 32, y: 40, scale: 0.6, paint: "slate-700" },
      { part: "arch", x: 68, y: 40, scale: 0.6, paint: "slate-700" },
      { part: "horned-skull", x: 16, y: 49, scale: 0.6 },
      { part: "horned-skull", x: 84, y: 49, scale: 0.6 },
      { part: "bones", x: 32, y: 39, scale: 0.8 },
      { part: "bones", x: 68, y: 39, scale: 0.8 },
      { part: "candle", x: 24, y: 58, scale: 1 },
      { part: "candle", x: 76, y: 58, scale: 1 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 62, paint: "slate-700" },
      { part: "mist", x: 50, y: 56, scale: 2.4, count: 2, spread: [24, 4], paint: "green-400" },
      { part: "fireball", x: 26, y: 40, scale: 0.9, paint: "orange-400" },
      { part: "orb", x: 76, y: 36, scale: 1.4, paint: "violet-500" },
      { part: "smoke", x: 32, y: 44, scale: 1, paint: "green-200" },
      { part: "column", x: 8, y: 76, scale: 1.8, paint: "slate-700" },
      { part: "column", x: 92, y: 76, scale: 1.8, paint: "slate-700" },
    ],
    subject: [
      { part: "altar", x: 50, y: 86, scale: 2.4, paint: "stone-500" },
      { part: "chalice", x: 50, y: 66, scale: 1.3, paint: "stone-500" },
      { part: "bubbles", x: 50, y: 52, scale: 1, paint: "green-400" },
    ],
    near: [
      { part: "candle", x: 18, y: 98, scale: 1.8 },
      { part: "candle", x: 84, y: 98, scale: 1.6 },
      { part: "bones", x: 66, y: 98, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 50, y: 58, scale: 2.2, paint: "green-400" },
      { part: "halo", x: 24, y: 52, scale: 0.6, paint: "green-200" },
      { part: "halo", x: 76, y: 52, scale: 0.6, paint: "green-200" },
      { part: "rim-light", x: 50, y: 70, scale: 1.4, paint: "lime-400" },
    ],
    particles: [
      { part: "spore-motes", x: 50, y: 50, scale: 1.4, paint: "green-200" },
      { part: "sparks", x: 74, y: 40, scale: 1, paint: "violet-300" },
      { part: "motes", x: 50, y: 30, scale: 1.2, paint: "lime-200" },
    ],
  },
};

/** Siphon Squad (Classic #88, Rare): a squad of hooded siphoners rises from a sunken roadway, draining red streams of strength from a crowded enemy formation; blood-red haze at dusk. */
const SIPHON_SQUAD: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "red-900"],
  light: { x: 50, y: 56, r: 36, paint: "red-500", opacity: 0.4 },
  emblem: { x: 50, y: 60, size: 5, rotate: 0 },
  figure: { x: 50, y: 84, scale: 0.86, pose: "pose-cast", look: SIPHONER },
  extras: [
    { layer: "subject", x: 30, y: 88, scale: 0.68, pose: "pose-raise", look: SIPHONER },
    { layer: "subject", x: 70, y: 88, scale: 0.68, flip: -1, pose: "pose-raise", look: { ...SIPHONER, cloth: "red-900" } },
  ],
  layers: {
    sky: [{ part: "sun-disc", x: 80, y: 30, scale: 0.6, paint: "red-500" }],
    far: [
      { part: "hills", x: 50, y: 56, paint: "red-900" },
      { part: "crowd", x: 50, y: 52, scale: 1.4, paint: "slate-700" },
      { part: "banner", x: 74, y: 38, scale: 0.8, paint: "slate-700" },
    ],
    mid: [
      { part: "road", x: 50, y: 64, paint: "wood-800" },
      { part: "stone-wall", x: 14, y: 76, scale: 1, paint: "slate-700" },
      { part: "stone-wall", x: 86, y: 76, scale: 1, paint: "slate-700" },
    ],
    subject: [{ part: "ink-whip", x: 50, y: 50, scale: 1.1, paint: "red-500" }],
    near: [
      { part: "pebbles", x: 82, y: 97 },
      { part: "pebbles", x: 16, y: 98 },
    ],
    light: [
      { part: "ink-whip", x: 34, y: 58, scale: 0.8, flip: -1, paint: "red-400" },
      { part: "ink-whip", x: 66, y: 58, scale: 0.8, paint: "red-400" },
      { part: "halo", x: 50, y: 58, scale: 1.6, paint: "red-500" },
    ],
    particles: [
      { part: "embers", x: 50, y: 50, scale: 1.4, count: 2, spread: [24, 8], paint: "red-400" },
      { part: "motes", x: 50, y: 40, scale: 1.2, paint: "red-300" },
    ],
  },
};

/** Paul Allen's Ghost (Classic #89, Rare): a faceless, translucent hooded spectre inside a shimmering pale shield bubble haunts an old toll-bridge, palm held out for a fee; foggy river night. */
const PAUL_ALLENS_GHOST: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "blue-900"],
  light: { x: 78, y: 20, r: 30, paint: "white", opacity: 0.3 },
  emblem: { x: 50, y: 58, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 80,
    scale: 0.92,
    fade: 0.6,
    pose: "pose-hold",
    look: { ...figureLookFor([]), head: "head-void", hair: "hood", outfit: "outfit-rags", item: null, skin: "slate-900", cloth: "slate-400" },
  },
  layers: {
    sky: [
      { part: "moon", x: 80, y: 18, scale: 0.8 },
      { part: "stars", x: 24, y: 20, scale: 1 },
    ],
    far: [
      { part: "hills", x: 50, y: 60, paint: "slate-800" },
      { part: "tower", x: 86, y: 64, scale: 0.7, paint: "slate-700" },
    ],
    mid: [
      { part: "water", x: 50, y: 70, paint: "blue-900" },
      { part: "bridge", x: 50, y: 68, scale: 1.6, paint: "stone-500" },
      { part: "mist", x: 50, y: 78, scale: 2.4, count: 2, spread: [30, 2], paint: "slate-400" },
    ],
    subject: [{ part: "shield-bubble", x: 50, y: 95, scale: 2.7, paint: "blue-200" }],
    near: [
      { part: "reed", x: 12, y: 98, count: 2, spread: [5, 1], paint: "slate-700" },
      { part: "coin-stack", x: 84, y: 97, scale: 1 },
      { part: "mist", x: 50, y: 96, scale: 2.4, paint: "slate-400" },
    ],
    light: [{ part: "halo", x: 50, y: 52, scale: 2, paint: "blue-200" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.4, paint: "blue-200" }],
  },
};

/** In Too Deep (Classic #90, Mythic): an endless quest-cavern spiralling down, a lone adventurer on a ledge over a bottomless shaft, each level below lit by a glowing quest-shrine whose reward beckons her deeper; ropes, ladders and abandoned packs of earlier adventurers dangle into the dark, black-blue water floods the stairway behind her, violet glow from the depths, a dizzying, irresistible pull downward. */
const IN_TOO_DEEP: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "blue-900"],
  light: { x: 58, y: 64, r: 40, paint: "violet-500", opacity: 0.5 },
  emblem: { x: 50, y: 24, size: 5, rotate: 0 },
  figure: {
    x: 26,
    y: 64,
    scale: 0.5,
    pose: "pose-hold",
    look: { ...figureLookFor(["Human"]), hair: "hair-long", outfit: "outfit-travel-cloak", item: "item-lantern", cloth: "blue-700" },
  },
  layers: {
    sky: [
      { part: "cavern", x: 50, y: 70, paint: "slate-800" },
      { part: "stars", x: 50, y: 14, scale: 0.8, count: 2, spread: [20, 4], paint: "violet-300", fade: 0.5 },
    ],
    far: [
      { part: "stairs", x: 14, y: 30, scale: 0.7, paint: "slate-700" },
      { part: "cascade", x: 12, y: 16, scale: 0.8, paint: "blue-800" },
      { part: "strings", x: 60, y: 62, scale: 0.8, paint: "wood-600", props: true },
    ],
    mid: [
      { part: "rock-floor", x: 22, y: 62, scale: 0.8, paint: "slate-700" },
      { part: "ladder", x: 44, y: 96, scale: 0.7, paint: "wood-800", props: true },
      { part: "sack", x: 12, y: 64, scale: 0.6, paint: "wood-700" },
    ],
    subject: [{ part: "spiral-shaft", x: 58, y: 62, scale: 1.05 }],
    near: [
      { part: "water", x: 50, y: 92, paint: "blue-900" },
      { part: "crystal", x: 10, y: 92, scale: 1.3, count: 2, spread: [6, 2], paint: "violet-500" },
      { part: "ladder", x: 90, y: 100, scale: 0.7, paint: "wood-700", props: true },
      { part: "pouch", x: 76, y: 96, scale: 1 },
    ],
    light: [
      { part: "halo", x: 58, y: 66, scale: 2.2, paint: "violet-500" },
      { part: "god-rays", x: 58, y: 68, scale: 1, paint: "violet-300" },
      { part: "rim-light", x: 26, y: 46, scale: 1, paint: "violet-300" },
      { part: "halo", x: 30, y: 46, scale: 0.7, paint: "white" },
    ],
    particles: [
      { part: "motes", x: 58, y: 58, scale: 1.6, count: 3, spread: [16, 12], paint: "violet-300" },
      { part: "crystal", x: 56, y: 50, scale: 0.6, count: 4, spread: [26, 18], paint: "cyan-400" },
    ],
  },
};

/** Glitch (Classic token): a card in a temple niche tears into shifting squares of static, half of it already gone from reality; pale void around it, magenta-cyan flicker. */
const GLITCH: SceneRecipe = {
  family: "classic",
  sky: ["slate-100", "violet-300"],
  light: { x: 50, y: 50, r: 34, paint: "cyan-400", opacity: 0.35 },
  emblem: { x: 44, y: 54, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "column", x: 22, y: 82, scale: 1.8, paint: "stone-500" },
      { part: "column", x: 78, y: 82, scale: 1.8, paint: "stone-500" },
      { part: "arch", x: 50, y: 82, scale: 1.8, paint: "stone-500" },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 82, paint: "slate-400" }],
    subject: [
      { part: "card", x: 48, y: 72, scale: 3 },
      { part: "squares", x: 50, y: 58, scale: 1.1, paint: "purple-400" },
      { part: "glitch", x: 56, y: 58, scale: 2, paint: "cyan-400" },
    ],
    near: [{ part: "pixels", x: 70, y: 60, scale: 1, paint: "purple-400" }],
    particles: [{ part: "pixels", x: 40, y: 40, scale: 0.8, paint: "cyan-400" }],
  },
};

export const CLASSIC_RECIPES_3: Readonly<Record<string, SceneRecipe>> = {
  "classic-061": PLAGUE_BRINGER_GOLIATH,
  "classic-062": LIVING_BOMB,
  "classic-063": CROP_DUSTING,
  "classic-064": MALZAHARS_RECYCLER,
  "classic-065": ACE_IN_THE_HOLE,
  "classic-066": EU_STRIKER,
  "classic-067": FELINOR_FEELER,
  "classic-068": SMALL_CARD_LOBBYIST,
  "classic-069": PLAGUE_CHARGER,
  "classic-070": BOOK_OF_PLAGUE,
  "classic-071": LANE_EATER,
  "classic-072": GRAND_COUNTERSPELL,
  "classic-073": NURSE_CLEAVER,
  "classic-075": ARGUSLAND,
  "classic-076": PLAGUE_BRINGER,
  "classic-077": ANTI_MAGIC_MONKEY,
  "classic-078": MUTATE_SPELL,
  "classic-079": RISKY_DIE,
  "classic-080": BOOM_BIG_MAX,
  "classic-081": THE_POWER_TO_THRIVE,
  "classic-082": SHEEOPLE,
  "classic-083": FLAME_LANCE,
  "classic-084": LOCKDOWN,
  "classic-085": KING_WAGTOGGLE,
  "classic-086": GENN,
  "classic-087": PLAGUE_CHALICE,
  "classic-088": SIPHON_SQUAD,
  "classic-089": PAUL_ALLENS_GHOST,
  "classic-090": IN_TOO_DEEP,
  "classic-t-glitch": GLITCH,
};
