// R3401000: Meditative's scenes, part 5 of 5: Meditative #80 to #102 with #91's, #93's, #95's, #96's and #97's tokens,
// each drawn from its brief in the issue's table (#3401) in Meditative's family, the ink wash (ART.md, "Meditative and
// Meditative+" and "Meditative drawn as scenes"): slate-900 ink on slate-100 paper, jade washes, a high and distant
// camera, and one red seal in the near layer. Each recipe keeps its subject within x 30 to 70 and y 25 to 85 and its
// emblem small on its subject; a Unit's character is a figure built on its tags' look, or its part of the scene kit in
// its subject layer (Volatility's jar, the Totem, the True Sheep and the nine buildings of Jlockeed's Evil Blueprints).
// Each doc comment gives the brief in a line and any change made to fit the kits.

import { figureLookFor } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** Aluneth (Meditative #80, Legendary Field Spell): a cliff-top library pavilion in a snowstorm, an eternal ice-blue flame in an iron lantern no wind can touch, frosty shelves of books, three tearing free and flying into a reader's outstretched hand, cold blue light. */
const ALUNETH: SceneRecipe = {
  family: "meditative",
  sky: ["slate-800", "blue-800"],
  light: { x: 50, y: 44, r: 30, paint: "sky-300", opacity: 0.45 },
  emblem: { x: 50, y: 47, size: 5, rotate: 0 },
  figure: {
    x: 68,
    y: 84,
    scale: 0.7,
    flip: -1,
    pose: "pose-cast",
    look: { ...figureLookFor([]), hair: "hair-bun", outfit: "outfit-robe", item: null, cloth: "blue-800" },
  },
  layers: {
    sky: [{ part: "cloud", x: 30, y: 22, scale: 1.2, paint: "slate-700" }],
    far: [
      { part: "ink-peaks", x: 50, y: 64, scale: 0.9 },
      { part: "cliff", x: 8, y: 70, scale: 1.2, paint: "slate-700" },
    ],
    mid: [
      { part: "ink-pavilion", x: 50, y: 88, scale: 2.4 },
      { part: "bookshelf", x: 26, y: 84, scale: 1.1, paint: "wood-700" },
      { part: "bookshelf", x: 75, y: 84, scale: 1.1, paint: "wood-700" },
    ],
    subject: [
      { part: "lantern", x: 48, y: 40, scale: 1.7 },
      { part: "closed-tome", x: 46, y: 58, scale: 0.45, count: 3, spread: [6, 4] },
    ],
    near: [
      { part: "snow", x: 18, y: 70, scale: 0.7 },
      { part: "ink-seal", x: 18, y: 76 },
    ],
    light: [
      { part: "halo", x: 50, y: 46, scale: 1.1, paint: "sky-300" },
      { part: "light-shaft", x: 50, y: 46, scale: 0.8, paint: "blue-200" },
    ],
    particles: [
      { part: "snow", x: 50, y: 36, scale: 1.4, count: 2, spread: [16, 8] },
      { part: "motes", x: 52, y: 58, scale: 0.6, paint: "sky-300" },
    ],
  },
};

/** Deadman's Hand (Meditative #81, Rare Spell): a ghostly dealer at a wind-battered cliffside card hut, shuffling a translucent copy of every card back into the deck, lantern smoke curling. */
const DEADMANS_HAND: SceneRecipe = {
  family: "meditative",
  sky: ["slate-700", "slate-400"],
  light: { x: 50, y: 56, r: 30, paint: "sky-300", opacity: 0.35 },
  emblem: { x: 50, y: 80, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 60, scale: 0.8 },
      { part: "mist", x: 50, y: 62, scale: 0.9 },
    ],
    mid: [
      { part: "cliff", x: 90, y: 54, scale: 1.1, flip: -1, paint: "slate-700" },
      { part: "tiled-house", x: 40, y: 66, scale: 1.6 },
      { part: "paper-lantern", x: 22, y: 44, scale: 1.2 },
    ],
    subject: [{ part: "ghost-figure", x: 52, y: 84, scale: 1.9 }],
    near: [
      { part: "wide-table", x: 50, y: 96, scale: 1.4, paint: "wood-700" },
      { part: "deck", x: 50, y: 85, scale: 1.1 },
      { part: "card-ring", x: 50, y: 74, scale: 0.9 },
      { part: "smoke", x: 22, y: 50, scale: 0.8, paint: "slate-400" },
      { part: "gust", x: 76, y: 40, scale: 0.9 },
      { part: "ink-seal", x: 80, y: 74 },
    ],
  },
};

/** Medina Outfitter (Meditative #82, Common Unit): a cheerful bazaar tailor draping bright sashes over each card in a customer's hand, under striped awnings in a sun-baked medina. */
const MEDINA_OUTFITTER: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "slate-100"],
  light: { x: 40, y: 26, r: 40, paint: "orange-400", opacity: 0.3 },
  emblem: { x: 44, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 44,
    y: 86,
    scale: 1,
    pose: "pose-hold",
    look: { ...figureLookFor(["Human"]), hair: "kerchief", outfit: "outfit-poncho", item: null, cloth: "orange-400" },
  },
  layers: {
    far: [
      { part: "whitewashed-house", x: 16, y: 64, scale: 1.2 },
      { part: "whitewashed-house", x: 84, y: 62, scale: 1.1, flip: -1 },
      { part: "bunting", x: 50, y: 30, scale: 1.2 },
    ],
    mid: [
      { part: "cobbles", x: 50, y: 70 },
      { part: "awning", x: 20, y: 80, scale: 1.2, paint: "red-600" },
      { part: "awning", x: 80, y: 80, scale: 1.2, flip: -1, paint: "jade-400" },
    ],
    near: [
      { part: "hand-open", x: 68, y: 80, scale: 1, flip: -1 },
      { part: "ink-card-fan", x: 66, y: 76, scale: 0.5 },
      { part: "ink-seal", x: 18, y: 74 },
    ],
  },
};

/** Volatility (Meditative #84, Common Unit): a jittery little creature of fizzing liquid in a glass jar body, swelling and shrinking wildly on an alchemist's tower workbench. */
const VOLATILITY: SceneRecipe = {
  family: "meditative",
  sky: ["slate-700", "slate-400"],
  light: { x: 50, y: 50, r: 30, paint: "jade-400", opacity: 0.35 },
  emblem: { x: 50, y: 62, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-round-window", x: 50, y: 40, scale: 1.4 },
      { part: "ink-peaks", x: 50, y: 50, scale: 0.4 },
      { part: "bookshelf", x: 14, y: 76, scale: 1.3, paint: "wood-700" },
    ],
    mid: [
      { part: "table", x: 50, y: 88, scale: 2.4, paint: "wood-700" },
      { part: "flask", x: 76, y: 76, scale: 0.8 },
      { part: "vial", x: 26, y: 76, scale: 1.1, paint: "violet-500" },
      { part: "candle", x: 84, y: 76, scale: 1 },
    ],
    subject: [{ part: "jar-sprite", x: 50, y: 77, scale: 2.2 }],
    near: [
      { part: "bubbles", x: 50, y: 40, scale: 0.9, paint: "jade-400" },
      { part: "ink-seal", x: 18, y: 74 },
    ],
  },
};

/** Playtester (Meditative #85, Rare Unit): a bleary playtester in a hoodie at a cluttered game table, three beetles crawling across the board, two glowing books beside her, late-night lamp. */
const PLAYTESTER: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "slate-800"],
  light: { x: 64, y: 54, r: 30, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 50, y: 64, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 84,
    scale: 1,
    pose: "pose-sit",
    look: { ...figureLookFor(["Human"]), hair: "hair-long", outfit: "outfit-hoodie", item: "item-card", cloth: "violet-700" },
  },
  layers: {
    far: [
      { part: "ink-round-window", x: 22, y: 40, scale: 1 },
      { part: "ink-peaks", x: 22, y: 46, scale: 0.25 },
      { part: "paper-screen", x: 84, y: 66, scale: 1.4 },
    ],
    mid: [{ part: "tatami", x: 50, y: 66 }],
    near: [
      { part: "wide-table", x: 50, y: 98, scale: 1.4, paint: "wood-700" },
      { part: "go-board", x: 46, y: 84, scale: 0.9 },
      { part: "beetle", x: 44, y: 81, scale: 0.8, count: 3, spread: [6, 1] },
      { part: "closed-tome", x: 70, y: 84, scale: 0.6, count: 2, spread: [1, 2] },
      { part: "lantern", x: 74, y: 62, scale: 0.9 },
      { part: "halo", x: 70, y: 82, scale: 0.4, paint: "sky-300" },
      { part: "ink-seal", x: 18, y: 74 },
    ],
  },
};

/** The True Sheep (Meditative #88, Epic Unit): a sheep with a silver halo of curled wool standing proud on a boulder, the rest of the flock turning away unimpressed, a high alpine pasture with a lone stone altar, bright cold morning. */
const THE_TRUE_SHEEP: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "slate-100"],
  light: { x: 50, y: 50, r: 26, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 66, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "cloud", x: 26, y: 24, scale: 0.9, paint: "white" }],
    far: [
      { part: "mountains", x: 50, y: 52, scale: 1 },
      { part: "altar", x: 82, y: 62, scale: 0.6 },
    ],
    mid: [
      { part: "hills", x: 50, y: 66, paint: "green-700" },
      { part: "ink-sheep", x: 22, y: 76, scale: 1.2, flip: -1 },
      { part: "ink-sheep", x: 78, y: 78, scale: 1.3 },
      { part: "ink-sheep", x: 88, y: 72, scale: 1 },
    ],
    subject: [
      { part: "mesa", x: 50, y: 82, scale: 0.42, paint: "stone-500" },
      { part: "ink-sheep", x: 50, y: 74, scale: 2.2 },
    ],
    near: [
      { part: "tall-grass", x: 30, y: 90, scale: 1.1 },
      { part: "flowers", x: 70, y: 88, scale: 1.2 },
      { part: "ink-seal", x: 80, y: 74 },
    ],
    light: [{ part: "halo", x: 50, y: 64, scale: 0.9, paint: "slate-100" }],
  },
};

/** Jlarna (Meditative #89, Rare Field Spell): a sleek pink lending kiosk at a mountain-pass trading post, handing shoppers mana crystals now while each drags four small iron weights on a chain. */
const JLARNA: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 50, y: 56, r: 28, paint: "red-300", opacity: 0.35 },
  emblem: { x: 50, y: 68, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 58, scale: 1 },
      { part: "road", x: 50, y: 62, scale: 0.8 },
    ],
    mid: [
      { part: "awning", x: 50, y: 78, scale: 1.6, paint: "red-300" },
      { part: "crystal", x: 46, y: 64, scale: 0.6, count: 3, spread: [6, 1], paint: "sky-400" },
    ],
    subject: [
      { part: "ink-person", x: 26, y: 82, scale: 1.5, flip: -1 },
      { part: "ink-person", x: 74, y: 82, scale: 1.5 },
      { part: "chain", x: 26, y: 82, scale: 0.4 },
      { part: "chain", x: 66, y: 82, scale: 0.4 },
      { part: "padlock", x: 35, y: 85, scale: 0.5, count: 2, spread: [2, 1] },
      { part: "padlock", x: 75, y: 85, scale: 0.5, count: 2, spread: [2, 1] },
    ],
    near: [
      { part: "halo", x: 46, y: 62, scale: 0.5, paint: "sky-300" },
      { part: "ink-seal", x: 80, y: 72 },
    ],
  },
};

/** Spell Basket (Meditative #90, Common Spell): a wicker picnic basket on a riverside meadow spilling half-built spells, sparks and loose glowing parts, a confused picnicker holding one piece. */
const SPELL_BASKET: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "slate-100"],
  light: { x: 50, y: 60, r: 26, paint: "jade-400", opacity: 0.35 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 52, scale: 0.8 },
      { part: "water", x: 50, y: 62 },
    ],
    mid: [
      { part: "field", x: 50, y: 70 },
      { part: "tall-grass", x: 14, y: 80, scale: 1.1 },
      { part: "ink-person", x: 30, y: 80, scale: 1.6 },
    ],
    subject: [
      { part: "basket", x: 50, y: 82, scale: 1.6 },
      { part: "thought-shapes", x: 52, y: 62, scale: 0.6 },
      { part: "sparks", x: 50, y: 66, scale: 1.2 },
      { part: "orb", x: 34, y: 68, scale: 0.3, paint: "jade-400" },
    ],
    near: [
      { part: "flowers", x: 70, y: 88, scale: 1.2 },
      { part: "ink-seal", x: 80, y: 72 },
    ],
  },
};

/** Medina Enforcer (Meditative #83, Rare Unit): a broad enforcer in a dark kaftan polishing the cards in a merchant's hand with a heavy ringed fist, in a shadowed medina alley. */
const MEDINA_ENFORCER: SceneRecipe = {
  family: "meditative",
  sky: ["slate-700", "slate-400"],
  light: { x: 50, y: 30, r: 26, paint: "orange-400", opacity: 0.25 },
  emblem: { x: 48, y: 60, size: 5, rotate: 0 },
  figure: {
    x: 46,
    y: 86,
    scale: 1.15,
    pose: "pose-hold",
    look: { ...figureLookFor(["Human"]), hair: "headband", outfit: "outfit-kaftan", item: null, cloth: "slate-900" },
  },
  layers: {
    far: [
      { part: "street", x: 50, y: 60 },
      { part: "lantern-string", x: 50, y: 34, scale: 0.8 },
    ],
    mid: [
      { part: "whitewashed-house", x: 10, y: 80, scale: 1.8, paint: "slate-400" },
      { part: "whitewashed-house", x: 90, y: 80, scale: 1.8, flip: -1, paint: "slate-400" },
      { part: "crate", x: 22, y: 84, scale: 0.9 },
      { part: "jar", x: 80, y: 84, scale: 1.4 },
    ],
    near: [
      { part: "hand-open", x: 70, y: 76, scale: 1.1, flip: -1 },
      { part: "ink-card-fan", x: 68, y: 72, scale: 0.55 },
      { part: "sparks", x: 64, y: 66, scale: 0.6, paint: "white" },
      { part: "coins", x: 28, y: 88, scale: 0.8 },
      { part: "ink-seal", x: 18, y: 72 },
    ],
  },
};

/** Mayor Medinamogger (Meditative #86, Legendary Unit): a square-jawed mayor in a sash and tall hat on a bazaar balcony flipping a silver coin high over the crowded square, arrows, spells and thrown fruit veering off at random below, bunting snapping in the hot wind under stark midday sun. */
const MAYOR_MEDINAMOGGER: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "slate-100"],
  light: { x: 50, y: 18, r: 44, paint: "white", opacity: 0.55 },
  emblem: { x: 50, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 1,
    pose: "pose-raise",
    look: { ...figureLookFor([]), hair: "tall-hat", outfit: "outfit-tailcoat", item: "item-silver-coin", cloth: "red-600" },
  },
  layers: {
    sky: [{ part: "sun", x: 18, y: 18, scale: 0.8 }],
    far: [
      { part: "whitewashed-house", x: 14, y: 54, scale: 0.9 },
      { part: "whitewashed-house", x: 86, y: 54, scale: 0.9, flip: -1 },
      { part: "ink-pavilion", x: 50, y: 52, scale: 0.6 },
    ],
    mid: [
      { part: "cobbles", x: 50, y: 58 },
      { part: "awning", x: 16, y: 70, scale: 0.8, paint: "red-600" },
      { part: "awning", x: 84, y: 70, scale: 0.8, flip: -1, paint: "jade-400" },
      { part: "crowd", x: 50, y: 74, scale: 0.6 },
    ],
    subject: [{ part: "bunting", x: 50, y: 40, scale: 1.4 }],
    near: [
      { part: "fence", x: 50, y: 96, scale: 1.8, paint: "stone-500" },
      { part: "ink-seal", x: 82, y: 76 },
    ],
    light: [
      { part: "halo", x: 54, y: 30, scale: 0.6, paint: "white" },
      { part: "god-rays", x: 18, y: 18, scale: 0.9, paint: "white" },
    ],
    particles: [
      { part: "flying-arrow", x: 24, y: 50, scale: 0.6 },
      { part: "flying-arrow", x: 80, y: 44, scale: 0.6, flip: -1 },
      { part: "fireball", x: 72, y: 60, scale: 0.5 },
      { part: "apple", x: 30, y: 62, scale: 0.9, count: 2, spread: [6, 6] },
      { part: "gust", x: 50, y: 30, scale: 1 },
    ],
  },
};

/** Tatches the Totem (Meditative #87, Legendary Unit): a tall carved totem bursting up out of the earth at a mountain crossroads, each tier by a different people, travellers of every tribe stopping as blessing light runs up the carvings, dusk. */
const TATCHES_THE_TOTEM: SceneRecipe = {
  family: "meditative",
  sky: ["violet-700", "red-300"],
  light: { x: 50, y: 50, r: 34, paint: "jade-400", opacity: 0.4 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  extras: [
    { layer: "mid", x: 22, y: 82, scale: 0.6, pose: "pose-stand", look: { ...figureLookFor(["Felinor"]), item: null, cloth: "jade-400" } },
    { layer: "mid", x: 78, y: 82, scale: 0.6, flip: -1, pose: "pose-stand", look: { ...figureLookFor(["KY"]), item: null } },
  ],
  layers: {
    sky: [{ part: "ink-cloud", x: 70, y: 22, scale: 0.9 }],
    far: [
      { part: "ink-peaks", x: 50, y: 60, scale: 1 },
      { part: "road", x: 50, y: 64 },
    ],
    mid: [
      { part: "cracked-earth", x: 50, y: 76 },
      { part: "ink-person", x: 34, y: 78, scale: 1.2 },
      { part: "ink-person", x: 66, y: 78, scale: 1.2, flip: -1 },
      { part: "ink-person", x: 88, y: 76, scale: 1.1, flip: -1 },
    ],
    subject: [{ part: "totem", x: 50, y: 86, scale: 1.25 }],
    near: [
      { part: "pebbles", x: 50, y: 88, scale: 1.4, props: true },
      { part: "dust", x: 50, y: 86, scale: 1.4, props: true },
      { part: "ink-seal", x: 80, y: 72 },
    ],
    light: [
      { part: "light-shaft", x: 50, y: 6, scale: 1, paint: "jade-400" },
      { part: "rim-light", x: 50, y: 60, scale: 1.2, paint: "jade-400" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 44, scale: 1.2, count: 3, spread: [6, 16], paint: "jade-400" },
      { part: "motes", x: 50, y: 50, scale: 1, paint: "white" },
    ],
  },
};

/** Windfast (Meditative #91, Epic Unit): a lean wind-spirit swordsman in flapping grey silks, eyes closed, ghostly allies blown in on gusts to strike ahead of him and whisked away, a plateau of tall grass bent flat under racing clouds. */
const WINDFAST: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 50, y: 40, r: 32, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 60, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 1,
    pose: "pose-guard",
    look: { ...figureLookFor(["Catalyst"]), head: "head-human", hair: "hair-long", outfit: "outfit-silks", item: "item-sword", cloth: "slate-400" },
  },
  layers: {
    sky: [{ part: "cloud", x: 30, y: 24, scale: 1.1, paint: "slate-400", count: 2, spread: [20, 4] }],
    far: [
      { part: "ink-peaks", x: 50, y: 60, scale: 0.8 },
      { part: "streaks", x: 80, y: 30, scale: 1 },
    ],
    mid: [
      { part: "field", x: 50, y: 68, paint: "green-700" },
      { part: "ghost-figure", x: 24, y: 76, scale: 1, flip: -1 },
      { part: "ghost-figure", x: 78, y: 74, scale: 0.9, flip: -1 },
      { part: "gust", x: 24, y: 60, scale: 1 },
      { part: "gust", x: 78, y: 58, scale: 0.9 },
    ],
    near: [
      { part: "tall-grass", x: 16, y: 92, scale: 1.4 },
      { part: "tall-grass", x: 86, y: 92, scale: 1.3 },
      { part: "leaves", x: 66, y: 50, scale: 0.9 },
      { part: "ink-seal", x: 80, y: 72 },
    ],
  },
};

/** Windfurious Prime (Meditative #91's token, Token Unit): a roaring cyclone-spirit with twin blades sweeping a crowd of allies into battle along a storm-lashed cliff edge. */
const WINDFURIOUS_PRIME: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "slate-700"],
  light: { x: 50, y: 46, r: 30, paint: "sky-300", opacity: 0.3 },
  emblem: { x: 50, y: 60, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 84,
    scale: 1.05,
    pose: "pose-spread",
    look: { ...figureLookFor(["Prime"]), head: "head-cloud", hair: null, outfit: "outfit-silks", item: "item-twin-blades", cloth: "slate-700", skin: "slate-400" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 62, scale: 0.8, paint: "slate-800" },
      { part: "lightning", x: 82, y: 22, scale: 0.8 },
    ],
    mid: [
      { part: "cliff", x: 50, y: 82, scale: 1.6, paint: "slate-700" },
      { part: "tornado", x: 50, y: 86, scale: 1.4 },
    ],
    subject: [
      { part: "soldier", x: 26, y: 82, scale: 0.8 },
      { part: "soldier", x: 76, y: 82, scale: 0.8, flip: -1 },
    ],
    near: [
      { part: "rain", x: 50, y: 50, scale: 1.3 },
      { part: "ink-seal", x: 18, y: 72 },
    ],
  },
};

/** Unan (Meditative #92, Rare Unit): a massive stoic guardian in layered stone armour stepping in front of a fleeing family to take a falling boulder on a mountain switchback. */
const UNAN: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 60, y: 30, r: 30, paint: "white", opacity: 0.4 },
  emblem: { x: 52, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 52,
    y: 86,
    scale: 1.15,
    pose: "pose-block",
    look: { ...figureLookFor([]), hair: "helm", outfit: "outfit-layered-plate", item: null, cloth: "stone-500" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 56, scale: 0.9 },
      { part: "pine", x: 76, y: 60, scale: 0.6, count: 2, spread: [6, 2] },
    ],
    mid: [
      { part: "cliff", x: 90, y: 50, scale: 1.1, flip: -1, paint: "slate-700" },
      { part: "rock-floor", x: 50, y: 78 },
      { part: "ink-person", x: 20, y: 84, scale: 1.4, flip: -1 },
      { part: "ink-person", x: 26, y: 85, scale: 1, flip: -1 },
      { part: "ink-person", x: 13, y: 84, scale: 1.3, flip: -1 },
    ],
    subject: [{ part: "asteroid", x: 63, y: 32, scale: 1.7, paint: "stone-500" }],
    near: [
      { part: "pebbles", x: 66, y: 44, scale: 0.8 },
      { part: "dust-cloud", x: 70, y: 28, scale: 1 },
      { part: "ink-seal", x: 80, y: 74 },
    ],
  },
};

/** Growing Felinor (Meditative #93, Common Unit): a tiny cat-folk kitten in a leaf cap striking a lion's pose at its reflection in a puddle in a tea garden. */
const GROWING_FELINOR: SceneRecipe = {
  family: "meditative",
  sky: ["slate-100", "green-200"],
  light: { x: 50, y: 50, r: 30, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 70, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 82,
    scale: 0.8,
    pose: "pose-flex",
    look: { ...figureLookFor(["Felinor"]), hair: "leaf-crown", outfit: "outfit-tunic", item: null, cloth: "green-700" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 54, scale: 0.7 },
      { part: "hedge", x: 16, y: 66, scale: 0.9, count: 2, spread: [6, 2] },
      { part: "hedge", x: 84, y: 66, scale: 0.9, count: 2, spread: [6, 2] },
    ],
    mid: [
      { part: "field", x: 50, y: 70, paint: "green-700" },
      { part: "stone-lantern", x: 20, y: 80, scale: 1.4 },
      { part: "paper-lantern", x: 80, y: 46, scale: 1 },
    ],
    near: [
      { part: "still-water", x: 50, y: 84, scale: 0.9 },
      { part: "teapot", x: 82, y: 90, scale: 0.9 },
      { part: "ink-seal", x: 80, y: 74 },
    ],
  },
};

/** Growing Felinor Sr (Meditative #93's token, Token Unit): a slightly bigger cat-folk youth proudly tying on a new leaf-green sash in a village dojo, still a little too big for him. */
const GROWING_FELINOR_SR: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 50, y: 44, r: 30, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 64, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 84,
    scale: 0.9,
    pose: "pose-hold",
    look: { ...figureLookFor(["Felinor"]), hair: "headband", outfit: "outfit-tabard", item: null, cloth: "green-400" },
  },
  layers: {
    far: [
      { part: "paper-screen", x: 20, y: 70, scale: 1.4 },
      { part: "paper-screen", x: 80, y: 70, scale: 1.4, flip: -1 },
      { part: "ink-round-window", x: 50, y: 42, scale: 1 },
      { part: "ink-peaks", x: 50, y: 48, scale: 0.3 },
    ],
    mid: [
      { part: "tatami", x: 50, y: 68 },
      { part: "weapon-rack", x: 84, y: 80, scale: 0.8 },
    ],
    near: [
      { part: "temple-bell", x: 16, y: 30, scale: 0.8 },
      { part: "ink-seal", x: 18, y: 74 },
    ],
  },
};

/** Growing Felinor Sr Sr (Meditative #93's token, Token Unit): a lanky teenage cat-folk lounging on a classroom desk with his feet up, a leaf crown askew, a lazy afternoon sunbeam. */
const GROWING_FELINOR_SR_SR: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 30, y: 30, r: 36, paint: "orange-400", opacity: 0.3 },
  emblem: { x: 50, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 84,
    scale: 1,
    pose: "pose-perch",
    look: { ...figureLookFor(["Felinor"]), hair: "leaf-crown", outfit: "outfit-sweater", item: null, cloth: "blue-800" },
  },
  layers: {
    far: [
      { part: "blackboard", x: 50, y: 56, scale: 1.4 },
      { part: "window", x: 14, y: 50, scale: 1.2 },
    ],
    mid: [
      { part: "tatami", x: 50, y: 70, paint: "wood-600" },
      { part: "desk", x: 22, y: 82, scale: 1.4 },
      { part: "desk", x: 80, y: 82, scale: 1.4 },
    ],
    subject: [{ part: "desk", x: 52, y: 86, scale: 2.2 }],
    near: [
      { part: "light-shaft", x: 14, y: 30, scale: 1, paint: "white" },
      { part: "motes", x: 30, y: 60, scale: 0.7, paint: "white" },
      { part: "ink-seal", x: 82, y: 74 },
    ],
  },
};

/** Growing Felinor Super Senior (Meditative #93's token, Token Unit): a grown cat-folk in a rumpled graduation gown napping under a pine on a college lawn, the campus emptying around him, late spring. */
const GROWING_FELINOR_SUPER_SENIOR: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "slate-100"],
  light: { x: 60, y: 30, r: 34, paint: "white", opacity: 0.45 },
  emblem: { x: 52, y: 70, size: 5, rotate: 0 },
  figure: {
    x: 52,
    y: 86,
    scale: 1,
    pose: "pose-sit",
    look: { ...figureLookFor(["Felinor"]), hair: "cap-feather", outfit: "outfit-robe", item: "item-scroll", cloth: "slate-900" },
  },
  layers: {
    far: [
      { part: "campus-hall", x: 78, y: 62, scale: 0.7 },
      { part: "hills", x: 50, y: 62, paint: "green-700" },
    ],
    mid: [
      { part: "field", x: 50, y: 70, paint: "green-700" },
      { part: "pine", x: 34, y: 86, scale: 2.6 },
      { part: "ink-person", x: 80, y: 72, scale: 1, count: 3, spread: [10, 2] },
    ],
    near: [
      { part: "petals", x: 50, y: 50, scale: 1, paint: "red-300" },
      { part: "ink-seal", x: 80, y: 76 },
    ],
  },
};

/** Shrinking Felinor (Meditative #94, Rare Unit): a burly cat-folk warrior shrinking mid-step on a misty mountain stair, a trail of ever-smaller copies of itself trotting behind. */
const SHRINKING_FELINOR: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 50, y: 30, r: 34, paint: "white", opacity: 0.4 },
  emblem: { x: 46, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 46,
    y: 86,
    scale: 1.1,
    pose: "pose-stride",
    look: { ...figureLookFor(["Felinor"]), hair: "steel-helmet", outfit: "outfit-plate", item: "item-sword", cloth: "slate-700" },
  },
  extras: [
    { layer: "mid", x: 60, y: 62, scale: 0.55, pose: "pose-stride", look: { ...figureLookFor(["Felinor"]), hair: "steel-helmet", outfit: "outfit-plate", item: "item-sword", cloth: "slate-700" } },
    { layer: "mid", x: 54, y: 50, scale: 0.32, pose: "pose-stride", look: { ...figureLookFor(["Felinor"]), hair: "steel-helmet", outfit: "outfit-plate", item: "item-sword", cloth: "slate-700" } },
  ],
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 52, scale: 0.9 },
      { part: "pine", x: 20, y: 56, scale: 0.8 },
    ],
    mid: [
      { part: "stairs", x: 54, y: 48, scale: 1 },
      { part: "mist", x: 50, y: 58, scale: 0.9 },
    ],
    near: [
      { part: "mist", x: 30, y: 86, scale: 0.8 },
      { part: "ink-seal", x: 80, y: 74 },
    ],
  },
};

/** Call to Chaos (Meditative Edition) (Meditative #95, Legendary Spell): a serene zen rock garden shattered mid-meditation, a spinning many-sided jade die with blank glowing faces crashing through the raked gravel, a jade woman, a stone golem, a wall of shields and a whirl of flying cards bursting out of it, flinging the monk backwards under storm-violet light. */
const CALL_TO_CHAOS_MEDITATIVE_EDITION: SceneRecipe = {
  family: "meditative",
  sky: ["violet-700", "slate-700"],
  light: { x: 50, y: 56, r: 34, paint: "jade-400", opacity: 0.45 },
  emblem: { x: 50, y: 58, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 30, y: 80, scale: 0.6, pose: "pose-spread", look: { head: "head-human", hair: "hair-long", outfit: "outfit-gown", item: null, skin: "jade-400", cloth: "jade-400" } },
    { layer: "subject", x: 72, y: 80, scale: 0.65, flip: -1, pose: "pose-strike", look: { head: "head-golem", hair: null, outfit: "outfit-rock", item: null, skin: "stone-500", cloth: "stone-500" } },
    { layer: "near", x: 14, y: 90, scale: 0.6, flip: -1, pose: "pose-spread", look: { head: "head-human", hair: "tonsure", outfit: "outfit-robe", item: null, skin: "red-300", cloth: "orange-400" } },
  ],
  layers: {
    sky: [
      { part: "ink-cloud", x: 30, y: 22, scale: 1 },
      { part: "lightning", x: 78, y: 18, scale: 0.7 },
    ],
    far: [
      { part: "ink-peaks", x: 50, y: 56, scale: 0.8, paint: "slate-800" },
      { part: "pagoda", x: 86, y: 60, scale: 0.6 },
    ],
    mid: [
      { part: "raked-gravel", x: 50, y: 66 },
    ],
    subject: [
      { part: "jade-die", x: 50, y: 60, scale: 1.8 },
      { part: "blast-rings", x: 50, y: 72, scale: 1 },
      { part: "shield", x: 50, y: 82, scale: 0.6, count: 3, spread: [8, 0] },
    ],
    near: [
      { part: "card-ring", x: 50, y: 52, scale: 1.1 },
      { part: "ink-seal", x: 82, y: 76 },
    ],
    light: [
      { part: "halo", x: 50, y: 60, scale: 1, paint: "jade-400" },
      { part: "light-shaft", x: 50, y: 10, scale: 0.9, paint: "violet-300" },
    ],
    particles: [
      { part: "shards", x: 50, y: 70, scale: 1.2 },
    ],
  },
};

/** CN Golem (Meditative #95's token, Token Unit): a hulking golem of carved stone and red lacquered panels smashing through a city gate with spiked fists, dust clouds at noon. */
const CN_GOLEM: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "slate-100"],
  light: { x: 50, y: 30, r: 36, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 58, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 1.15,
    pose: "pose-strike",
    look: { ...figureLookFor(["CN"]), head: "head-golem", hair: null, outfit: "outfit-layered-plate", item: null, skin: "stone-500", cloth: "red-600" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 54, scale: 0.6 },
      { part: "stone-wall", x: 14, y: 76, scale: 1 },
      { part: "stone-wall", x: 86, y: 76, scale: 1 },
    ],
    mid: [
      { part: "red-gate", x: 50, y: 82, scale: 2 },
      { part: "cobbles", x: 50, y: 78 },
    ],
    near: [
      { part: "dust-cloud", x: 30, y: 82, scale: 1.2, count: 2, spread: [6, 2] },
      { part: "shards", x: 66, y: 44, scale: 0.8, paint: "slate-400" },
      { part: "ink-seal", x: 80, y: 74 },
    ],
  },
};

/** Meditative Journey (Meditative #96, Rare Spell): a pilgrim setting a few cherished cards adrift on lotus-leaf boats down a temple stream, walking away with an empty satchel, morning mist. */
const MEDITATIVE_JOURNEY: SceneRecipe = {
  family: "meditative",
  sky: ["slate-100", "blue-200"],
  light: { x: 30, y: 30, r: 36, paint: "white", opacity: 0.45 },
  emblem: { x: 60, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 62,
    y: 82,
    scale: 0.8,
    flip: -1,
    pose: "pose-stride",
    look: { head: "head-human", hair: "hood", outfit: "outfit-robe", item: "item-sack", skin: "red-300", cloth: "stone-500" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 52, scale: 0.8 },
      { part: "pagoda", x: 82, y: 58, scale: 0.7 },
      { part: "mist", x: 50, y: 56, scale: 1 },
    ],
    mid: [
      { part: "field", x: 50, y: 64, paint: "green-700" },
      { part: "water", x: 30, y: 70 },
      { part: "lotus", x: 24, y: 76, scale: 0.9 },
      { part: "lotus", x: 38, y: 82, scale: 1.1 },
      { part: "playing-card", x: 24, y: 74, scale: 0.4 },
      { part: "playing-card", x: 38, y: 80, scale: 0.45 },
    ],
    near: [
      { part: "lotus", x: 18, y: 92, scale: 1.4 },
      { part: "playing-card", x: 18, y: 89, scale: 0.55 },
      { part: "reed", x: 8, y: 92, scale: 1.2 },
      { part: "mist", x: 70, y: 86, scale: 0.7 },
      { part: "ink-seal", x: 82, y: 74 },
    ],
  },
};

/** Journey Complete (Meditative #96's token, Token Spell): lotus-leaf boats drifting back up a sunlit stream, carrying pearl-bright cards into a waiting pilgrim's hands. */
const JOURNEY_COMPLETE: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "slate-100"],
  light: { x: 50, y: 30, r: 40, paint: "white", opacity: 0.55 },
  emblem: { x: 50, y: 70, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "sun", x: 50, y: 26, scale: 0.8 }],
    far: [{ part: "ink-peaks", x: 50, y: 54, scale: 0.8 }],
    mid: [
      { part: "water", x: 50, y: 58 },
      { part: "lotus", x: 44, y: 64, scale: 0.7 },
      { part: "lotus", x: 58, y: 70, scale: 0.9 },
      { part: "playing-card", x: 44, y: 62, scale: 0.35, paint: "white" },
      { part: "playing-card", x: 58, y: 68, scale: 0.4, paint: "white" },
    ],
    subject: [
      { part: "lotus", x: 50, y: 78, scale: 1.3 },
      { part: "playing-card", x: 50, y: 75, scale: 0.6, paint: "white" },
    ],
    near: [
      { part: "hand-open", x: 38, y: 86, scale: 1.3 },
      { part: "hand-open", x: 62, y: 86, scale: 1.3, flip: -1 },
      { part: "halo", x: 50, y: 72, scale: 0.4, paint: "white" },
      { part: "ink-seal", x: 82, y: 72 },
    ],
  },
};

/** Jlockeed's Evil Blueprints (Meditative #97, Mythic Spell): a war room hollowed out of a mountain, a vast cyan blueprint table whose lines lift into holograms of a church, a bunker, a wall, a prison and a headquarters tower as gauntleted engineers slot torn pieces together, red warning lamps sweeping and a villain's silhouette leaning over the head of the table, backlit. */
const JLOCKEEDS_EVIL_BLUEPRINTS: SceneRecipe = {
  family: "meditative",
  sky: ["black", "slate-900"],
  light: { x: 50, y: 40, r: 34, paint: "red-500", opacity: 0.35 },
  emblem: { x: 50, y: 82, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 66,
    scale: 0.7,
    pose: "pose-hold",
    look: { ...figureLookFor(["Jlockeed"]), head: "head-void", item: null, skin: "black", cloth: "black" },
  },
  extras: [
    { layer: "near", x: 16, y: 94, scale: 0.6, pose: "pose-hold", look: { ...figureLookFor(["Jlockeed"]), outfit: "outfit-armor", item: null, cloth: "slate-700" } },
    { layer: "near", x: 84, y: 94, scale: 0.6, flip: -1, pose: "pose-hold", look: { ...figureLookFor(["Jlockeed"]), outfit: "outfit-armor", item: null, cloth: "slate-700" } },
  ],
  layers: {
    sky: [{ part: "stars", x: 50, y: 22, scale: 0.8, paint: "red-400" }],
    far: [
      { part: "cave", x: 50, y: 44 },
    ],
    mid: [
      { part: "glass-church", x: 30, y: 56, scale: 0.32, paint: "cyan-400" },
      { part: "bunker", x: 70, y: 56, scale: 0.32, paint: "cyan-400" },
      { part: "sea-prison", x: 22, y: 70, scale: 0.28, paint: "cyan-400" },
      { part: "peak-fortress", x: 78, y: 70, scale: 0.3, paint: "cyan-400" },
    ],
    subject: [{ part: "blueprint", x: 50, y: 92, scale: 1.3 }],
    near: [
      { part: "ink-seal", x: 82, y: 70 },
      { part: "lantern", x: 22, y: 26, scale: 1, paint: "red-500" },
      { part: "lantern", x: 78, y: 26, scale: 1, paint: "red-500" },
    ],
    light: [
      { part: "halo", x: 50, y: 50, scale: 1, paint: "red-500" },
      { part: "light-shaft", x: 22, y: 28, scale: 0.7, paint: "red-400" },
      { part: "light-shaft", x: 78, y: 28, scale: 0.7, paint: "red-400" },
    ],
    particles: [
      { part: "motes", x: 50, y: 64, scale: 1.2, paint: "cyan-400" },
      { part: "sparks", x: 40, y: 80, scale: 0.8, paint: "cyan-400" },
    ],
  },
};

/** Showdown (Meditative #98, Rare Spell): two gunslingers facing off down one dusty frontier street at high noon, every side alley slammed shut with iron gates, a tumbleweed rolling. */
const SHOWDOWN: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "slate-100"],
  light: { x: 50, y: 20, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 40, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 38,
    y: 86,
    scale: 0.95,
    pose: "pose-guard",
    look: { head: "head-human", hair: "hat-brim", outfit: "outfit-coat", item: null, skin: "red-300", cloth: "wood-700" },
  },
  extras: [{ layer: "mid", x: 60, y: 66, scale: 0.42, flip: -1, pose: "pose-guard", look: { head: "head-human", hair: "fedora", outfit: "outfit-coat", item: null, skin: "red-300", cloth: "slate-900" } }],
  layers: {
    sky: [{ part: "sun-disc", x: 50, y: 18, scale: 1 }],
    far: [
      { part: "mesa", x: 50, y: 56, scale: 0.8, paint: "red-300" },
      { part: "street", x: 56, y: 58 },
    ],
    mid: [
      { part: "whitewashed-house", x: 12, y: 74, scale: 1.4, paint: "wood-700" },
      { part: "whitewashed-house", x: 88, y: 74, scale: 1.4, flip: -1, paint: "wood-700" },
      { part: "portcullis", x: 26, y: 68, scale: 0.7 },
      { part: "portcullis", x: 76, y: 68, scale: 0.7 },
    ],
    near: [
      { part: "bush", x: 70, y: 86, scale: 0.6, paint: "wood-600" },
      { part: "dust", x: 66, y: 88, scale: 1 },
      { part: "ink-seal", x: 84, y: 74 },
    ],
  },
};

/** Paranoia (Meditative #99, Epic Field Spell): red string pinned between cards on a cork wall, a hunched figure peering through blinds with cloaked spell-cards hidden in his coat, a rainy hillside town of watching windows in dim green light. */
const PARANOIA: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "green-900"],
  light: { x: 50, y: 46, r: 30, paint: "green-400", opacity: 0.25 },
  emblem: { x: 48, y: 64, size: 5, rotate: 0 },
  figure: {
    x: 48,
    y: 88,
    scale: 0.95,
    pose: "pose-hold",
    look: { head: "head-human", hair: "fedora", outfit: "outfit-coat", item: "item-card", skin: "red-300", cloth: "slate-700" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 50, scale: 0.6, paint: "slate-800" },
      { part: "tiled-house", x: 30, y: 52, scale: 0.6 },
      { part: "tiled-house", x: 62, y: 48, scale: 0.55 },
      { part: "tiled-house", x: 80, y: 54, scale: 0.6 },
      { part: "eye", x: 46, y: 36, scale: 0.6, paint: "green-400" },
      { part: "rain", x: 50, y: 40, scale: 1.2 },
    ],
    mid: [
      { part: "paper-screen", x: 30, y: 82, scale: 1.8, paint: "slate-400" },
      { part: "paper-screen", x: 68, y: 82, scale: 1.8, flip: -1, paint: "slate-400" },
    ],
    near: [
      { part: "playing-card", x: 12, y: 38, scale: 0.8 },
      { part: "playing-card", x: 22, y: 60, scale: 0.8 },
      { part: "playing-card", x: 88, y: 44, scale: 0.8 },
      { part: "red-thread", x: 12, y: 36, scale: 0.4 },
      { part: "red-thread", x: 22, y: 58, scale: 0.3, flip: -1 },
      { part: "red-thread", x: 88, y: 42, scale: 0.35, flip: -1 },
      { part: "ink-seal", x: 84, y: 76 },
    ],
    light: [{ part: "halo", x: 50, y: 50, scale: 0.8, paint: "green-400" }],
  },
};

/** Greaser (Meditative #100, Common Unit): a broad-shouldered greaser in a leather jacket combing his slick pompadour beside his motorbike at a mountain road diner, sunset glow. */
const GREASER: SceneRecipe = {
  family: "meditative",
  sky: ["violet-700", "red-300"],
  light: { x: 50, y: 56, r: 34, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 42, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 42,
    y: 86,
    scale: 1.05,
    pose: "pose-raise",
    look: { ...figureLookFor([]), head: "head-human", hair: "pompadour", outfit: "outfit-jacket", item: "item-comb", cloth: "black" },
  },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 58, scale: 0.8, paint: "slate-800" },
      { part: "diner", x: 50, y: 70, scale: 1.1 },
    ],
    mid: [{ part: "road", x: 50, y: 70 }],
    near: [
      { part: "motorbike", x: 72, y: 88, scale: 1.4, flip: -1 },
      { part: "ink-seal", x: 18, y: 74 },
    ],
  },
};

/** Gachaholic (Meditative #101, Common Unit): a hollow-eyed gamer hunched over a prize-capsule machine in a bright arcade, cranking for one more glowing capsule, pockets turned out. */
const GACHAHOLIC: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "violet-700"],
  light: { x: 66, y: 50, r: 30, paint: "cyan-400", opacity: 0.35 },
  emblem: { x: 42, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 40,
    y: 86,
    scale: 1,
    pose: "pose-hold",
    look: { ...figureLookFor(["CN", "Human"]), hair: "hair-short", outfit: "outfit-sweater", item: "item-coin", cloth: "slate-700" },
  },
  layers: {
    far: [
      { part: "monitors", x: 20, y: 60, scale: 0.8 },
      { part: "monitors", x: 84, y: 58, scale: 0.7 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66 },
      { part: "capsule-machine", x: 68, y: 88, scale: 1.8 },
    ],
    near: [
      { part: "jelly-bean", x: 74, y: 84, scale: 0.6, paint: "cyan-400" },
      { part: "halo", x: 68, y: 66, scale: 0.6, paint: "violet-300" },
      { part: "ink-seal", x: 18, y: 74 },
    ],
  },
};

/** Catboy Maid SSR+ (Meditative #102, Common Unit): a cheerful cat-eared boy in a frilly maid apron serving tea and jade sweets at a cat café, sparkles of rare-pull light around him. */
const CATBOY_MAID_SSR: SceneRecipe = {
  family: "meditative",
  sky: ["red-300", "slate-100"],
  light: { x: 50, y: 50, r: 34, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 62, size: 5, rotate: 0 },
  figure: {
    x: 50,
    y: 86,
    scale: 1,
    pose: "pose-hold",
    look: { ...figureLookFor(["CN", "Felinor"]), head: "head-felinor", hair: null, outfit: "outfit-apron", item: "item-tray", skin: "orange-400", cloth: "blue-800" },
  },
  layers: {
    far: [
      { part: "ink-round-window", x: 22, y: 44, scale: 0.9 },
      { part: "paper-lantern", x: 80, y: 30, scale: 1 },
    ],
    mid: [
      { part: "boards", x: 50, y: 68 },
      { part: "table", x: 78, y: 84, scale: 1 },
      { part: "teapot", x: 76, y: 76, scale: 0.7 },
      { part: "rice-cake", x: 84, y: 77, scale: 0.4 },
    ],
    near: [
      { part: "house-cat", x: 24, y: 88, scale: 1.2 },
      { part: "sparks", x: 50, y: 50, scale: 1.4, count: 2, spread: [14, 10], paint: "white" },
      { part: "ink-seal", x: 82, y: 72 },
    ],
  },
};

/** Empty Plot (Meditative #97's token, Token Unit): a vacant fenced hillside lot of bare dirt, stakes and string marking out a foundation, a crow on a post, overcast. */
const EMPTY_PLOT: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 50, y: 40, r: 34, paint: "white", opacity: 0.35 },
  emblem: { x: 50, y: 74, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "ink-cloud", x: 50, y: 26, scale: 1 }],
    far: [
      { part: "ink-peaks", x: 50, y: 56, scale: 0.8 },
      { part: "hills", x: 50, y: 64, paint: "green-700" },
    ],
    subject: [
      { part: "stake-plot", x: 50, y: 84, scale: 1.7 },
      { part: "crow", x: 24, y: 60, scale: 1.3 },
    ],
    near: [
      { part: "grass-tuft", x: 20, y: 90, scale: 1, count: 2, spread: [8, 2] },
      { part: "ink-seal", x: 82, y: 74 },
    ],
  },
};

/** Wishing Well (Meditative #97's token, Token Unit): a mossy stone wishing well in a village square, a silver coin tumbling above it, a faint shine glowing up from the depths. */
const WISHING_WELL: SceneRecipe = {
  family: "meditative",
  sky: ["slate-400", "slate-100"],
  light: { x: 50, y: 56, r: 28, paint: "jade-400", opacity: 0.4 },
  emblem: { x: 50, y: 78, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 52, scale: 0.6 },
      { part: "tiled-house", x: 16, y: 64, scale: 1.1 },
      { part: "tiled-house", x: 84, y: 64, scale: 1, flip: -1 },
    ],
    mid: [{ part: "cobbles", x: 50, y: 66 }],
    subject: [
      { part: "stone-well", x: 50, y: 86, scale: 2.6 },
      { part: "coin", x: 50, y: 36, scale: 1.3 },
    ],
    near: [
      { part: "halo", x: 50, y: 62, scale: 0.7, paint: "jade-400" },
      { part: "sparks", x: 50, y: 38, scale: 0.8, paint: "white" },
      { part: "ink-seal", x: 82, y: 74 },
    ],
  },
};

/** School (Meditative #97's token, Token Unit): a stern brick schoolhouse with tall windows and a bell, small creatures filing in through its doors, crisp morning. */
const SCHOOL: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "slate-100"],
  light: { x: 30, y: 26, r: 36, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 52, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "ink-peaks", x: 50, y: 54, scale: 0.8 }],
    mid: [{ part: "field", x: 50, y: 70, paint: "green-700" }],
    subject: [
      { part: "schoolhouse", x: 50, y: 78, scale: 1.3 },
      { part: "bunny", x: 42, y: 86, scale: 0.35 },
      { part: "house-cat", x: 36, y: 87, scale: 0.8 },
      { part: "fox", x: 60, y: 88, scale: 0.4, flip: -1 },
    ],
    near: [
      { part: "pine", x: 12, y: 88, scale: 1.2 },
      { part: "ink-seal", x: 84, y: 76 },
    ],
  },
};

/** Mega Church (Meditative #97's token, Token Unit): a gleaming glass megachurch on a hill, a giant offering plate at its doors, crowds queuing with open wallets, white glare. */
const MEGA_CHURCH: SceneRecipe = {
  family: "meditative",
  sky: ["slate-100", "white"],
  light: { x: 50, y: 30, r: 40, paint: "white", opacity: 0.6 },
  emblem: { x: 50, y: 54, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 56, scale: 0.6 },
      { part: "hills", x: 50, y: 66, paint: "green-700" },
    ],
    subject: [
      { part: "glass-church", x: 50, y: 76, scale: 1.3 },
      { part: "coins", x: 50, y: 82, scale: 0.9 },
    ],
    near: [
      { part: "ink-person", x: 22, y: 88, scale: 1.2, count: 3, spread: [8, 1] },
      { part: "ink-person", x: 76, y: 88, scale: 1.2, flip: -1, count: 2, spread: [6, 1] },
      { part: "ink-seal", x: 84, y: 72 },
    ],
    light: [{ part: "god-rays", x: 50, y: 24, scale: 1, paint: "white" }],
  },
};

/** Bunker (Meditative #97's token, Token Unit): a squat concrete bunker dug into a hillside lane, gun slits glaring, sandbags and barbed wire, steel-grey dusk. */
const BUNKER: SceneRecipe = {
  family: "meditative",
  sky: ["slate-700", "slate-400"],
  light: { x: 50, y: 62, r: 26, paint: "orange-400", opacity: 0.25 },
  emblem: { x: 50, y: 70, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 60, scale: 0.8, paint: "slate-800" },
      { part: "mist", x: 50, y: 62, scale: 0.9 },
    ],
    mid: [{ part: "road", x: 50, y: 74, scale: 0.8 }],
    subject: [{ part: "bunker", x: 50, y: 80, scale: 1.5 }],
    near: [
      { part: "fence-post", x: 16, y: 90, scale: 1 },
      { part: "ink-seal", x: 82, y: 74 },
    ],
  },
};

/** University (Meditative #97's token, Token Unit): a sprawling hillside university of grey stone halls and a spire, graduates tossing caps skyward, bright spring noon. */
const UNIVERSITY: SceneRecipe = {
  family: "meditative",
  sky: ["sky-300", "slate-100"],
  light: { x: 50, y: 26, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 60, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 50, scale: 0.6 },
      { part: "hills", x: 50, y: 62, paint: "green-700" },
    ],
    subject: [{ part: "campus-hall", x: 50, y: 76, scale: 1.25 }],
    near: [
      { part: "ink-person", x: 26, y: 88, scale: 1.3, count: 3, spread: [10, 1] },
      { part: "ink-person", x: 70, y: 88, scale: 1.3, flip: -1, count: 2, spread: [8, 1] },
      { part: "confetti", x: 50, y: 34, scale: 1, paint: "slate-900" },
      { part: "petals", x: 76, y: 50, scale: 0.8, paint: "red-300" },
      { part: "ink-seal", x: 84, y: 72 },
    ],
  },
};

/** The Great Wall (Meditative #97's token, Token Unit): an endless crenellated wall snaking over ink-wash ridges, its gates bricked shut, a lone guard locked outside on the wrong side, misty morning. */
const THE_GREAT_WALL: SceneRecipe = {
  family: "meditative",
  sky: ["slate-100", "slate-400"],
  light: { x: 70, y: 30, r: 34, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 56, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "ink-peaks", x: 50, y: 46, scale: 0.9 }],
    mid: [{ part: "mist", x: 50, y: 52, scale: 1 }],
    subject: [{ part: "long-wall", x: 50, y: 60, scale: 1.2 }],
    near: [
      { part: "mist", x: 50, y: 76, scale: 1 },
      { part: "soldier", x: 50, y: 88, scale: 1 },
      { part: "ink-seal", x: 82, y: 76 },
    ],
  },
};

/** Prison (Meditative #97's token, Token Unit): a grim stone prison on a sea crag, a captured warrior being stacked into a cell beneath a guard tower, cold rain. */
const PRISON: SceneRecipe = {
  family: "meditative",
  sky: ["slate-800", "slate-400"],
  light: { x: 60, y: 40, r: 30, paint: "sky-300", opacity: 0.25 },
  emblem: { x: 46, y: 70, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "ink-peaks", x: 50, y: 60, scale: 0.6, paint: "slate-800" },
      { part: "sea", x: 50, y: 66 },
    ],
    subject: [
      { part: "sea-prison", x: 50, y: 84, scale: 1.4 },
      { part: "soldier", x: 66, y: 80, scale: 0.4 },
    ],
    near: [
      { part: "rain", x: 50, y: 46, scale: 1.4 },
      { part: "ink-seal", x: 18, y: 76 },
    ],
  },
};

/** Jlockeed's Headquarters (Meditative #97's token, Token Unit): a towering gunmetal corporate fortress on a mountain peak, missile racks at its corners, contractors' trucks streaming in, red beacon lights. */
const JLOCKEEDS_HEADQUARTERS: SceneRecipe = {
  family: "meditative",
  sky: ["slate-900", "slate-700"],
  light: { x: 50, y: 40, r: 34, paint: "red-500", opacity: 0.3 },
  emblem: { x: 50, y: 64, size: 5, rotate: 0 },
  layers: {
    far: [{ part: "ink-peaks", x: 50, y: 70, scale: 1, paint: "slate-800" }],
    mid: [
      { part: "mist", x: 50, y: 80, scale: 1 },
      { part: "ink-cloud", x: 20, y: 40, scale: 0.6 },
    ],
    subject: [{ part: "peak-fortress", x: 50, y: 80, scale: 1.3 }],
    near: [
      { part: "cart", x: 34, y: 88, scale: 0.7 },
      { part: "cart", x: 66, y: 90, scale: 0.8, flip: -1 },
      { part: "ink-seal", x: 84, y: 74 },
    ],
    light: [{ part: "halo", x: 50, y: 40, scale: 0.6, paint: "red-500" }],
  },
};

export const MEDITATIVE_RECIPES_5: Readonly<Record<string, SceneRecipe>> = {
  "meditative-080": ALUNETH,
  "meditative-081": DEADMANS_HAND,
  "meditative-082": MEDINA_OUTFITTER,
  "meditative-083": MEDINA_ENFORCER,
  "meditative-084": VOLATILITY,
  "meditative-085": PLAYTESTER,
  "meditative-086": MAYOR_MEDINAMOGGER,
  "meditative-087": TATCHES_THE_TOTEM,
  "meditative-088": THE_TRUE_SHEEP,
  "meditative-089": JLARNA,
  "meditative-090": SPELL_BASKET,
  "meditative-091": WINDFAST,
  "meditative-091-1": WINDFURIOUS_PRIME,
  "meditative-092": UNAN,
  "meditative-093": GROWING_FELINOR,
  "meditative-093-1": GROWING_FELINOR_SR,
  "meditative-093-2": GROWING_FELINOR_SR_SR,
  "meditative-093-3": GROWING_FELINOR_SUPER_SENIOR,
  "meditative-094": SHRINKING_FELINOR,
  "meditative-095": CALL_TO_CHAOS_MEDITATIVE_EDITION,
  "meditative-095-1": CN_GOLEM,
  "meditative-096": MEDITATIVE_JOURNEY,
  "meditative-096-1": JOURNEY_COMPLETE,
  "meditative-097": JLOCKEEDS_EVIL_BLUEPRINTS,
  "meditative-097-1": EMPTY_PLOT,
  "meditative-097-2": WISHING_WELL,
  "meditative-097-3": SCHOOL,
  "meditative-097-4": MEGA_CHURCH,
  "meditative-097-5": BUNKER,
  "meditative-097-6": UNIVERSITY,
  "meditative-097-7": THE_GREAT_WALL,
  "meditative-097-8": PRISON,
  "meditative-097-9": JLOCKEEDS_HEADQUARTERS,
  "meditative-098": SHOWDOWN,
  "meditative-099": PARANOIA,
  "meditative-100": GREASER,
  "meditative-101": GACHAHOLIC,
  "meditative-102": CATBOY_MAID_SSR,
};
