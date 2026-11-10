// R3352000: Classic's scene recipes, part 2 of three (#31 to #60), one per entry, each drawn from its brief (#3352's
// table) in Classic's temple-stone look (ART.md, "Classic's scenes"): its subject within x 30 to 70 and y 25 to 85,
// its foreground near, its middle in the mid and subject layers and its background far, richer the rarer.
// Presentation only (CLAUDE.md rule 7): a recipe is read by the card's id alone.

import { figureLookFor, type FigureLook } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** A soldier of the Classic scenes: helmed, in mail, a sword in hand. */
const WARRIOR: FigureLook = { ...figureLookFor(["Human"]), hair: "helm", outfit: "outfit-armor", cloth: "slate-700" };
/** A townsfolk of the Classic scenes, bareheaded in a plain tunic. */
const VILLAGER: FigureLook = { ...figureLookFor(["Human"]), item: null, cloth: "wood-700" };

/** Felinor Feelings (Classic #32, Rare): a tiny Felinor kitten raises one paw and a hulking enemy warrior kneels and follows it across a sunlit cobbled lane; smug, rosy light. */
const FELINOR_FEELINGS: SceneRecipe = {
  family: "classic",
  sky: ["sky-300", "red-300"],
  light: { x: 24, y: 28, r: 42, paint: "red-300", opacity: 0.45 },
  emblem: { x: 82, y: 40, size: 5, rotate: 0 },
  figure: { x: 38, y: 82, scale: 0.42, pose: "pose-raise", look: { ...figureLookFor(["Felinor"]), item: null, cloth: "red-300" } },
  extras: [{ layer: "subject", x: 60, y: 84, scale: 1.05, flip: -1, pose: "pose-genuflect", look: { ...WARRIOR, item: "item-sword", cloth: "red-900" } }],
  layers: {
    sky: [{ part: "cloud", x: 70, y: 18, scale: 0.8, count: 2, spread: [14, 3], paint: "white" }],
    far: [
      { part: "townhouse", x: 14, y: 62, scale: 1.1, paint: "stone-500" },
      { part: "townhouse", x: 86, y: 62, scale: 1.2, paint: "slate-400" },
      { part: "banner", x: 82, y: 34, scale: 1.2, paint: "red-600" },
      { part: "tower", x: 50, y: 56, scale: 0.7 },
    ],
    mid: [
      { part: "cobbles", x: 50, y: 64, paint: "stone-500" },
      { part: "flowers", x: 20, y: 70, count: 2, spread: [8, 2], paint: "red-300" },
      { part: "streetlamp", x: 74, y: 68, scale: 0.7 },
    ],
    subject: [{ part: "paw-prints", x: 44, y: 86, scale: 1.2, paint: "wood-700" }],
    near: [
      { part: "pebbles", x: 18, y: 96 },
      { part: "flowers", x: 86, y: 97, count: 2, spread: [6, 1], paint: "red-300" },
    ],
    light: [
      { part: "light-shaft", x: 22, y: 0, paint: "white" },
      { part: "halo", x: 38, y: 58, scale: 1.4, paint: "red-300" },
    ],
    particles: [{ part: "petals", x: 46, y: 44, scale: 1.2, paint: "red-300" }],
  },
};

/** Ancient Acquisition (Classic #34, Rare): a treasure-hunter's trowel lifts a glowing ancient card out of a sand-buried tomb, dust swirling around it; desert dig site, low copper sun. */
const ANCIENT_ACQUISITION: SceneRecipe = {
  family: "classic",
  sky: ["brass-800", "orange-400"],
  light: { x: 50, y: 50, r: 34, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 52, y: 40, size: 5, rotate: 0 },
  extras: [{ layer: "subject", x: 36, y: 86, scale: 0.9, pose: "pose-cast", look: { ...figureLookFor(["Human"]), hair: "hat-brim", outfit: "outfit-vest", item: "item-spade", cloth: "wood-600" } }],
  layers: {
    sky: [{ part: "sun", x: 84, y: 48, scale: 1.1, paint: "orange-400" }],
    far: [
      { part: "mesa", x: 18, y: 58, scale: 1.3, paint: "brass-600" },
      { part: "mesa", x: 84, y: 60, scale: 1, paint: "wood-600" },
      { part: "column", x: 70, y: 62, scale: 0.8, paint: "stone-500" },
    ],
    mid: [
      { part: "sand", x: 50, y: 62, paint: "brass-500" },
      { part: "arch", x: 50, y: 80, scale: 1.6, paint: "stone-500" },
      { part: "ruin-wall", x: 16, y: 74, scale: 0.9, paint: "stone-500" },
      { part: "fallen-column", x: 82, y: 76, scale: 0.9 },
    ],
    subject: [
      { part: "shovel", x: 70, y: 82, scale: 1.4, paint: "wood-600" },
      { part: "card", x: 52, y: 52, scale: 2.2, paint: "brass-600" },
    ],
    near: [
      { part: "sand", x: 50, y: 84, paint: "brass-600" },
      { part: "bones", x: 22, y: 94, scale: 1.2 },
      { part: "pebbles", x: 78, y: 95, count: 2, spread: [8, 2] },
    ],
    light: [
      { part: "halo", x: 52, y: 44, scale: 1.8, paint: "white" },
      { part: "rim-light", x: 50, y: 76, scale: 1.4, paint: "orange-400" },
    ],
    particles: [
      { part: "dust", x: 52, y: 58, scale: 1.4, paint: "brass-500" },
      { part: "motes", x: 52, y: 40, scale: 1.2, paint: "white" },
    ],
  },
};

/** Prep (Classic #35, Common): a young mage measures a chalk circle twice with a knotted cord before casting, a spell held ready in her other hand; sunlit cliffside terrace. */
const PREP: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "blue-200"],
  light: { x: 22, y: 26, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 50, y: 80, size: 5, rotate: 0 },
  figure: { x: 40, y: 80, scale: 0.95, pose: "pose-genuflect", look: { ...figureLookFor(["Human"]), hair: "hair-long", outfit: "outfit-robe", item: "item-chalk", cloth: "blue-800" } },
  layers: {
    sky: [{ part: "sun", x: 20, y: 22, scale: 0.8 }],
    far: [
      { part: "sea", x: 50, y: 58, paint: "blue-600" },
      { part: "cliff", x: 92, y: 56, scale: 0.9 },
    ],
    mid: [{ part: "terrace", x: 50, y: 66, paint: "stone-500" }],
    subject: [
      { part: "rune-circle", x: 58, y: 82, scale: 1.8, paint: "white" },
      { part: "red-thread", x: 58, y: 81, scale: 1.4, paint: "wood-600" },
      { part: "sparks", x: 30, y: 52, scale: 1.2, paint: "sky-300" },
    ],
    near: [
      { part: "stone-lantern", x: 88, y: 96, scale: 0.9 },
      { part: "grass-tuft", x: 12, y: 97, count: 2, spread: [6, 1] },
    ],
    light: [{ part: "halo", x: 30, y: 52, scale: 1, paint: "sky-300" }],
    particles: [{ part: "motes", x: 50, y: 60, paint: "white" }],
  },
};

/** Burn (Classic #36, Common): a flung fire rune engulfs a charging bandit as a card flutters back from the smoke to the caster; dry scrubland watchpost, harsh late sun. */
const BURN: SceneRecipe = {
  family: "classic",
  sky: ["red-900", "orange-400"],
  light: { x: 86, y: 40, r: 36, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 50, y: 56, size: 5, rotate: 0 },
  figure: { x: 56, y: 84, scale: 0.9, flip: -1, pose: "pose-run", look: { ...figureLookFor(["Human"]), hair: "kerchief", outfit: "outfit-vest", item: "item-knife", cloth: "wood-700" } },
  extras: [{ layer: "near", x: 12, y: 96, scale: 0.8, pose: "pose-cast", look: { ...figureLookFor(["Human"]), hair: "hood", outfit: "outfit-robe", item: null, cloth: "red-900" } }],
  layers: {
    sky: [{ part: "sun", x: 86, y: 42, scale: 0.9, paint: "orange-400" }],
    far: [
      { part: "tower", x: 78, y: 62, scale: 0.9 },
      { part: "palisade", x: 78, y: 64, scale: 0.6 },
      { part: "mesa", x: 22, y: 60, scale: 0.9, paint: "wood-600" },
    ],
    mid: [
      { part: "cracked-earth", x: 50, y: 64, paint: "stone-500" },
      { part: "cactus", x: 88, y: 76, scale: 0.8 },
      { part: "bush", x: 24, y: 74, scale: 0.7, paint: "wood-600" },
    ],
    subject: [
      { part: "fireball", x: 54, y: 60, scale: 1.8 },
      { part: "rune", x: 54, y: 56, scale: 1.2, paint: "orange-400" },
      { part: "smoke", x: 62, y: 44, scale: 1.1, paint: "slate-400" },
      { part: "card", x: 34, y: 46, scale: 1.4 },
      { part: "flames", x: 56, y: 84, scale: 1.1 },
    ],
    near: [
      { part: "tumbleweed", x: 84, y: 96 },
      { part: "pebbles", x: 50, y: 97 },
    ],
    light: [{ part: "rim-light", x: 54, y: 66, scale: 1.4, paint: "ember-500" }],
    particles: [{ part: "embers", x: 54, y: 52, scale: 1.3, paint: "orange-400" }],
  },
};

/** Last Hurrah (Classic #37, Epic): a reveller hurls an entire deck into the air, cards raining, over a torchlit victory feast in a castle courtyard of overturned tables, pale dawn creeping over the battlements; wild, bittersweet. */
const LAST_HURRAH: SceneRecipe = {
  family: "classic",
  sky: ["blue-900", "violet-300"],
  light: { x: 50, y: 56, r: 40, paint: "orange-400", opacity: 0.35 },
  emblem: { x: 50, y: 30, size: 5, rotate: 0 },
  figure: { x: 50, y: 88, scale: 1.05, pose: "pose-raise", look: { ...figureLookFor(["Human"]), hair: "hair-long", outfit: "outfit-gown", item: null, cloth: "red-600" } },
  extras: [
    { layer: "mid", x: 24, y: 72, scale: 0.42, pose: "pose-raise", look: { ...VILLAGER, item: "item-flask" } },
    { layer: "mid", x: 76, y: 73, scale: 0.4, flip: -1, pose: "pose-flex", look: { ...WARRIOR, item: null } },
  ],
  layers: {
    sky: [
      { part: "stars", x: 50, y: 14, scale: 0.8, paint: "white" },
    ],
    far: [
      { part: "battlement", x: 22, y: 56, scale: 1.1, paint: "slate-700" },
      { part: "battlement", x: 78, y: 56, scale: 1.1, paint: "slate-700" },
      { part: "tower", x: 50, y: 54, scale: 0.9, paint: "slate-700" },
      { part: "banner", x: 50, y: 36, scale: 1.1, paint: "red-600" },
    ],
    mid: [
      { part: "cobbles", x: 50, y: 62, paint: "slate-700" },
      { part: "wide-table", x: 70, y: 76, scale: 0.9, flip: -1 },
      { part: "lantern", x: 12, y: 58, scale: 1.1 },
      { part: "lantern", x: 88, y: 58, scale: 1.1 },
      { part: "bunting", x: 50, y: 44, scale: 1.4 },
    ],
    subject: [
      { part: "deck", x: 56, y: 32, scale: 1.6 },
      { part: "playing-card", x: 50, y: 40, scale: 1.3, count: 6, spread: [24, 16] },
    ],
    near: [
      { part: "playing-card", x: 50, y: 70, scale: 1.8, count: 2, spread: [40, 18] },
      { part: "barrel", x: 12, y: 98, scale: 1.1 },
      { part: "cup", x: 86, y: 98, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 12, y: 52, scale: 1.2, paint: "orange-400" },
      { part: "halo", x: 88, y: 52, scale: 1.2, paint: "orange-400" },
      { part: "rim-light", x: 50, y: 58, scale: 1.6, paint: "red-300" },
    ],
    particles: [{ part: "embers", x: 50, y: 60, scale: 1.2, paint: "orange-400" }],
  },
};

/** Jackiestan Auctioneer (Classic #38, Rare): a booming auctioneer in a plum waistcoat bursts from a trapdoor onto the block, gavel slamming, bidders' cards flying; packed auction rotunda, lamplit. */
const JACKIESTAN_AUCTIONEER: SceneRecipe = {
  family: "classic",
  sky: ["wood-900", "wood-700"],
  light: { x: 50, y: 40, r: 36, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 50, y: 82, size: 5, rotate: 0 },
  figure: { x: 50, y: 76, scale: 0.95, pose: "pose-strike", look: { ...figureLookFor(["Human"]), outfit: "outfit-waistcoat", item: "item-mallet", cloth: "violet-700" } },
  layers: {
    far: [
      { part: "column", x: 12, y: 60, scale: 1.3, paint: "stone-500" },
      { part: "column", x: 88, y: 60, scale: 1.3, paint: "stone-500" },
      { part: "arch", x: 50, y: 60, scale: 1.6, paint: "wood-700" },
      { part: "chandelier", x: 50, y: 14, scale: 1.2, paint: "orange-400" },
      { part: "lantern", x: 12, y: 30, scale: 0.9 },
      { part: "lantern", x: 88, y: 30, scale: 0.9 },
    ],
    mid: [
      { part: "boards", x: 50, y: 62, paint: "wood-700" },
      { part: "stands", x: 20, y: 72, scale: 1.2, paint: "wood-600" },
      { part: "stands", x: 80, y: 72, scale: 1.2, flip: -1, paint: "wood-600" },
    ],
    subject: [
      { part: "podium", x: 50, y: 92, scale: 1.4, paint: "wood-700" },
      { part: "hatch", x: 50, y: 78, scale: 1.3, paint: "wood-700" },
      { part: "playing-card", x: 50, y: 34, scale: 1.3, count: 5, spread: [22, 12] },
    ],
    near: [{ part: "crowd", x: 50, y: 102, scale: 1.2, paint: "wood-900" }],
    light: [
      { part: "halo", x: 50, y: 20, scale: 1.6, paint: "orange-400" },
      { part: "spotlight", x: 50, y: 0, scale: 1, paint: "white" },
    ],
    particles: [{ part: "sparks", x: 46, y: 50, scale: 1.2, paint: "orange-400" }],
  },
};

/** Outbreak (Classic #39, Epic): one villager sneezes on a harbour quay, a green haze spreading across fish stalls and moored boats as the crowd coughs and flees, a watchtower bell swinging above plague-flagged rooftops under a sickly olive sky. */
const OUTBREAK: SceneRecipe = {
  family: "classic",
  sky: ["green-900", "brass-600"],
  light: { x: 64, y: 30, r: 36, paint: "lime-200", opacity: 0.3 },
  emblem: { x: 18, y: 32, size: 5, rotate: 0 },
  figure: { x: 42, y: 88, scale: 1.05, pose: "pose-hold", look: { ...VILLAGER, hair: "kerchief", outfit: "outfit-apron", cloth: "slate-400" } },
  extras: [
    { layer: "mid", x: 24, y: 76, scale: 0.42, flip: -1, pose: "pose-run", look: { ...VILLAGER, cloth: "red-900" } },
    { layer: "mid", x: 78, y: 76, scale: 0.4, pose: "pose-run", look: { ...VILLAGER, hair: "hood", cloth: "blue-800" } },
  ],
  layers: {
    far: [
      { part: "townhouse", x: 14, y: 56, scale: 0.9, paint: "slate-700" },
      { part: "townhouse", x: 86, y: 56, scale: 0.9, paint: "wood-700" },
      { part: "belfry", x: 56, y: 54, scale: 0.6, paint: "stone-500" },
      { part: "flag", x: 18, y: 32, scale: 1.4, paint: "lime-400" },
      { part: "flag", x: 86, y: 30, scale: 1.3, paint: "lime-400" },
    ],
    mid: [
      { part: "sea", x: 50, y: 58, paint: "slate-700" },
      { part: "boat", x: 16, y: 66, scale: 0.7 },
      { part: "boat", x: 86, y: 66, scale: 0.6, flip: -1 },
      { part: "boards", x: 50, y: 74, paint: "wood-700" },
      { part: "awning", x: 66, y: 76, scale: 0.75, paint: "green-700" },
      { part: "mist", x: 50, y: 70, scale: 2.2, paint: "lime-400" },
    ],
    subject: [
      { part: "puff", x: 58, y: 42, scale: 1.3, paint: "lime-400" },
      { part: "spores", x: 62, y: 44, scale: 1.2, paint: "lime-200" },
    ],
    near: [
      { part: "barrel", x: 12, y: 98, scale: 1.1 },
      { part: "crate", x: 86, y: 98, scale: 1.1 },
    ],
    light: [
      { part: "halo", x: 58, y: 42, scale: 1.4, paint: "lime-200" },
      { part: "rim-light", x: 42, y: 70, scale: 1.4, paint: "lime-400" },
    ],
    particles: [{ part: "spore-motes", x: 50, y: 60, scale: 1.6, paint: "lime-400" }],
  },
};

/** State of the Game (Classic #41, Common): an unbreakable granite sentinel stands calm and unchanged as the seasons whirl around it, ivy and snow on its shoulders at once; windswept mountaintop plinth. */
const STATE_OF_THE_GAME: SceneRecipe = {
  family: "classic",
  sky: ["blue-900", "sky-300"],
  light: { x: 76, y: 24, r: 36, paint: "white", opacity: 0.35 },
  emblem: { x: 50, y: 80, size: 5, rotate: 0 },
  figure: { x: 50, y: 76, scale: 1.05, pose: "pose-stand", look: { ...figureLookFor([]), head: "head-stone", hair: null, outfit: "outfit-stone", item: null, skin: "stone-500", cloth: "stone-500" } },
  layers: {
    sky: [{ part: "cloud", x: 24, y: 22, scale: 0.8, count: 2, spread: [10, 4], paint: "slate-100" }],
    far: [{ part: "peaks", x: 50, y: 60, paint: "slate-400" }],
    mid: [{ part: "summit", x: 50, y: 80, scale: 1.2, paint: "slate-700" }],
    subject: [{ part: "pedestal", x: 50, y: 88, scale: 2.2, paint: "slate-700" }],
    near: [
      { part: "vine", x: 44, y: 38, scale: 0.9, paint: "green-500" },
      { part: "snow", x: 57, y: 38, scale: 0.5, paint: "white" },
      { part: "grass-tuft", x: 14, y: 98, count: 2, spread: [6, 1], paint: "wood-600" },
    ],
    light: [{ part: "halo", x: 50, y: 50, scale: 1.8, paint: "white" }],
    particles: [
      { part: "leaves", x: 26, y: 44, scale: 1.3, paint: "orange-400" },
      { part: "petals", x: 74, y: 56, scale: 1.2, paint: "red-300" },
      { part: "snow", x: 50, y: 26, scale: 1, paint: "white" },
      { part: "gust", x: 50, y: 48, scale: 1.6, paint: "white" },
    ],
  },
};

/** Divine Favor (Classic #46, Rare): a halo of white light pours cards down from the sky into a lone pilgrim's open hands; mountain shrine, soft morning rays, quiet fortune. */
const DIVINE_FAVOR: SceneRecipe = {
  family: "classic",
  sky: ["blue-800", "red-300"],
  light: { x: 50, y: 14, r: 40, paint: "white", opacity: 0.5 },
  emblem: { x: 50, y: 32, size: 5, rotate: 0 },
  figure: { x: 50, y: 88, scale: 0.95, pose: "pose-lift", look: { ...figureLookFor(["Human"]), hair: "hood", outfit: "outfit-robe", item: null, cloth: "slate-400" } },
  layers: {
    sky: [{ part: "cloud", x: 20, y: 30, scale: 0.9, count: 2, spread: [8, 4], paint: "red-300" }],
    far: [
      { part: "peaks", x: 50, y: 62, paint: "slate-700" },
      { part: "temple", x: 80, y: 66, scale: 0.6, paint: "stone-500" },
    ],
    mid: [
      { part: "stairs", x: 50, y: 66, paint: "stone-500" },
      { part: "stone-lantern", x: 22, y: 78, scale: 0.8 },
      { part: "stone-lantern", x: 78, y: 78, scale: 0.8 },
    ],
    subject: [{ part: "playing-card", x: 50, y: 30, scale: 1.5, count: 5, spread: [8, 22], paint: "slate-100" }],
    near: [
      { part: "pebbles", x: 20, y: 96 },
      { part: "grass-tuft", x: 84, y: 98, count: 2, spread: [6, 1] },
    ],
    light: [
      { part: "halo", x: 50, y: 12, scale: 2.4, paint: "white" },
      { part: "god-rays", x: 50, y: 4, scale: 1, paint: "white" },
    ],
    particles: [{ part: "motes", x: 50, y: 40, scale: 1.4, paint: "white" }],
  },
};

/** Anti-Greed Machine (Classic #49, Common): a towering iron automaton with a single card slot in its chest guards a treasury, one claw holding back a grasping hand; furnace-lit treasury hall. */
const ANTI_GREED_MACHINE: SceneRecipe = {
  family: "classic",
  sky: ["wood-900", "red-900"],
  light: { x: 16, y: 62, r: 34, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 80, y: 72, size: 5, rotate: 0 },
  figure: { x: 50, y: 86, scale: 1.25, pose: "pose-guard", look: { ...figureLookFor([]), head: "head-automaton", hair: null, outfit: "outfit-machine", item: null, skin: "slate-400", cloth: "slate-700" } },
  layers: {
    far: [
      { part: "forge", x: 14, y: 64, scale: 1.1 },
      { part: "column", x: 86, y: 60, scale: 1.2, paint: "stone-500" },
      { part: "arch", x: 50, y: 60, scale: 1.5, paint: "slate-800" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 62, paint: "slate-700" },
      { part: "coin-stack", x: 24, y: 78, scale: 1.1 },
      { part: "treasure-chest", x: 80, y: 80, scale: 1 },
    ],
    subject: [{ part: "coins", x: 50, y: 90, scale: 1.2 }],
    near: [
      { part: "card", x: 50, y: 54, scale: 0.8, paint: "slate-900" },
      { part: "hand-open", x: 70, y: 48, scale: 1.4, flip: -1 },
      { part: "coin", x: 18, y: 96, count: 3, spread: [8, 2] },
    ],
    light: [
      { part: "halo", x: 14, y: 56, scale: 1.4, paint: "orange-400" },
      { part: "rim-light", x: 50, y: 62, scale: 1.4, paint: "ember-500" },
    ],
    particles: [{ part: "embers", x: 20, y: 52, scale: 1.2, paint: "orange-400" }],
  },
};

/** Back Breaker (Classic #51, Common): a hulking wrestler with boulder shoulders smashes the back row of temple pillars with a two-handed slam; rubble-strewn colonnade, dusty sunbeams. */
const BACK_BREAKER: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "stone-500"],
  light: { x: 80, y: 20, r: 38, paint: "white", opacity: 0.35 },
  emblem: { x: 50, y: 30, size: 5, rotate: 0 },
  figure: { x: 50, y: 88, scale: 1.2, pose: "pose-strike", look: { ...figureLookFor(["Human"]), hair: "headband", outfit: "outfit-muscle", item: null, skin: "wood-600", cloth: "red-900" } },
  layers: {
    far: [
      { part: "ruin-wall", x: 50, y: 58, scale: 1.4, paint: "slate-700" },
      { part: "column", x: 18, y: 62, scale: 1.2 },
      { part: "column", x: 82, y: 62, scale: 1.2 },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 64, paint: "stone-500" },
      { part: "column", x: 30, y: 68, scale: 1.1 },
      { part: "column", x: 66, y: 68, scale: 1.1 },
      { part: "fallen-column", x: 78, y: 74, scale: 0.9 },
    ],
    subject: [
      { part: "splinters", x: 64, y: 42, scale: 1.4, paint: "stone-500" },
      { part: "shards", x: 40, y: 30, scale: 1.2, paint: "stone-500" },
    ],
    near: [
      { part: "fallen-column", x: 16, y: 96, scale: 1.1 },
      { part: "pebbles", x: 80, y: 96, count: 2, spread: [8, 2] },
    ],
    light: [
      { part: "light-shaft", x: 76, y: 0, paint: "white" },
      { part: "light-shaft", x: 30, y: 0, paint: "white" },
    ],
    particles: [{ part: "dust", x: 50, y: 74, scale: 1.4, paint: "stone-500" }],
  },
};

/** Final Gambit (Classic #52, Epic): a giant's warhammer stops inches above a fallen hero, held back by a flipped die blazing white, as the hero rises, wounds closing, on a ruined clifftop duelling ground under a breaking storm; desperate, turning-point mood. */
const FINAL_GAMBIT: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "slate-700"],
  light: { x: 50, y: 42, r: 34, paint: "white", opacity: 0.45 },
  emblem: { x: 50, y: 44, size: 5, rotate: 0 },
  figure: { x: 46, y: 86, scale: 0.95, pose: "pose-genuflect", look: { ...WARRIOR, hair: "hair-short", item: "item-sword", cloth: "blue-800" } },
  extras: [{ layer: "mid", x: 84, y: 86, scale: 1.2, flip: -1, pose: "pose-strike", look: { ...figureLookFor([]), head: "head-ogre", hair: null, outfit: "outfit-fur", item: null, skin: "slate-400", cloth: "wood-800" } }],
  layers: {
    sky: [
      { part: "cloud", x: 30, y: 18, scale: 1.2, count: 2, spread: [16, 4], paint: "slate-800" },
      { part: "lightning", x: 18, y: 14, scale: 0.9, paint: "white" },
    ],
    far: [
      { part: "cliff", x: 10, y: 58, scale: 0.8, paint: "slate-700" },
      { part: "ruin-wall", x: 86, y: 60, scale: 1, flip: -1, paint: "slate-700" },
      { part: "column", x: 24, y: 62, scale: 0.9 },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 66, paint: "slate-700" },
      { part: "fallen-column", x: 22, y: 76, scale: 0.8 },
    ],
    subject: [
      { part: "hammer", x: 54, y: 30, scale: 2.2, paint: "slate-400" },
      { part: "die", x: 50, y: 46, scale: 4, paint: "white" },
      { part: "heal-glow", x: 46, y: 66, scale: 1.6, paint: "green-400" },
    ],
    near: [
      { part: "broken-spear", x: 22, y: 96, scale: 1.1 },
      { part: "shield", x: 80, y: 98, scale: 1 },
      { part: "pebbles", x: 52, y: 98 },
    ],
    light: [
      { part: "halo", x: 50, y: 42, scale: 1.8, paint: "white" },
      { part: "light-shaft", x: 72, y: 0, paint: "slate-100" },
    ],
    particles: [{ part: "sparks", x: 50, y: 40, scale: 1.4, paint: "white" }],
  },
};

/** Rewind (Classic #54, Common): a herald's battle cry spirals visibly back into his mouth and bursts out again, a ghostly echo beside him; misty mountain pass, pale blue. */
const REWIND: SceneRecipe = {
  family: "classic",
  sky: ["blue-800", "blue-200"],
  light: { x: 40, y: 34, r: 34, paint: "sky-300", opacity: 0.4 },
  emblem: { x: 46, y: 64, size: 5, rotate: 0 },
  figure: { x: 46, y: 86, scale: 1, pose: "pose-raise", look: { ...figureLookFor(["Human"]), outfit: "outfit-tabard", item: "item-banner", cloth: "blue-700" } },
  extras: [{ layer: "subject", x: 64, y: 84, scale: 0.92, pose: "pose-raise", look: { ...figureLookFor(["Human"]), outfit: "outfit-tabard", item: "item-banner", cloth: "sky-300" }, fade: 0.35 }],
  layers: {
    far: [
      { part: "mountains", x: 50, y: 58, paint: "slate-400" },
      { part: "mist", x: 30, y: 58, scale: 2, paint: "white" },
    ],
    mid: [
      { part: "cliff", x: 12, y: 62, scale: 0.7, paint: "slate-700" },
      { part: "snowfield", x: 50, y: 70, paint: "slate-400" },
    ],
    subject: [
      { part: "vortex", x: 56, y: 40, scale: 0.7, paint: "sky-300" },
      { part: "echo-rings", x: 56, y: 40, scale: 0.9, paint: "white" },
    ],
    near: [
      { part: "mist", x: 70, y: 92, scale: 1.8, paint: "blue-200" },
      { part: "pine", x: 90, y: 99, scale: 0.9, paint: "slate-700" },
    ],
    light: [{ part: "halo", x: 56, y: 40, scale: 1.2, paint: "sky-300" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.2, paint: "white" }],
  },
};

/** Echo (Classic #57, Epic): a caster's spell repeats as a translucent mirror twin inside an echoing cave temple of carved faces, each mouth repeating the blast, sound-rings rippling away into a lavender haze; resonant, uncanny mood. */
const ECHO: SceneRecipe = {
  family: "classic",
  sky: ["violet-700", "violet-300"],
  light: { x: 50, y: 50, r: 36, paint: "violet-300", opacity: 0.45 },
  emblem: { x: 50, y: 50, size: 5, rotate: 0 },
  figure: { x: 36, y: 86, scale: 0.95, pose: "pose-cast", look: { ...figureLookFor(["Human"]), hair: "hood", outfit: "outfit-robe", item: "item-wand", cloth: "violet-700" } },
  extras: [{ layer: "subject", x: 64, y: 86, scale: 0.95, flip: -1, pose: "pose-cast", look: { ...figureLookFor(["Human"]), hair: "hood", outfit: "outfit-robe", item: "item-wand", cloth: "violet-300" }, fade: 0.4 }],
  layers: {
    far: [
      { part: "echo-rings", x: 50, y: 40, scale: 2.6, paint: "violet-300" },
      { part: "mist", x: 50, y: 46, scale: 2.4, paint: "violet-300" },
    ],
    mid: [
      { part: "cave", x: 50, y: 32, paint: "slate-800" },
      { part: "mask", x: 10, y: 40, scale: 1.3, paint: "stone-500" },
      { part: "mask", x: 90, y: 40, scale: 1.3, flip: -1, paint: "stone-500" },
      { part: "mask", x: 50, y: 18, scale: 1.1, paint: "stone-500" },
      { part: "burst", x: 12, y: 42, scale: 0.6, paint: "violet-300" },
      { part: "burst", x: 88, y: 42, scale: 0.6, paint: "violet-300" },
      { part: "rock-floor", x: 50, y: 70, paint: "slate-700" },
    ],
    subject: [
      { part: "orb", x: 50, y: 56, scale: 1.2, paint: "violet-500" },
      { part: "burst", x: 50, y: 52, scale: 1.2, paint: "white" },
    ],
    near: [
      { part: "standing-stone", x: 10, y: 99, scale: 0.7, paint: "slate-800" },
      { part: "standing-stone", x: 90, y: 99, scale: 0.6, flip: -1, paint: "slate-800" },
      { part: "pebbles", x: 50, y: 98 },
    ],
    light: [
      { part: "halo", x: 50, y: 52, scale: 1.8, paint: "violet-300" },
      { part: "rim-light", x: 50, y: 66, scale: 1.6, paint: "purple-400" },
    ],
    particles: [{ part: "sparks", x: 50, y: 50, scale: 1.4, count: 2, spread: [14, 8], paint: "violet-300" }],
  },
};

/** Common Resources (Classic #58, Common): a shared village well where two rival farmsteads draw from one bucket, its rope hauling a card up from the bottom; common pasture, plain noon. */
const COMMON_RESOURCES: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "blue-200"],
  light: { x: 50, y: 10, r: 40, paint: "white", opacity: 0.35 },
  emblem: { x: 50, y: 52, size: 5, rotate: 0 },
  figure: { x: 32, y: 86, scale: 0.85, pose: "pose-hold", look: { ...VILLAGER, hair: "hat-brim", outfit: "outfit-overalls", cloth: "red-600" } },
  extras: [{ layer: "subject", x: 68, y: 86, scale: 0.85, flip: -1, pose: "pose-hold", look: { ...VILLAGER, hair: "bucket-hat", outfit: "outfit-overalls", cloth: "blue-700" } }],
  layers: {
    sky: [
      { part: "sun", x: 50, y: 10, scale: 0.8 },
      { part: "cloud", x: 80, y: 24, scale: 0.7, paint: "white" },
    ],
    far: [
      { part: "hills", x: 50, y: 58, paint: "green-700" },
      { part: "barn", x: 14, y: 62, scale: 0.8, paint: "red-600" },
      { part: "barn", x: 86, y: 62, scale: 0.8, flip: -1, paint: "blue-800" },
    ],
    mid: [
      { part: "field", x: 50, y: 64, paint: "green-500" },
      { part: "fence", x: 20, y: 72, scale: 0.8 },
      { part: "fence", x: 80, y: 72, scale: 0.8 },
    ],
    subject: [
      { part: "well", x: 50, y: 84, scale: 1.8 },
      { part: "card", x: 50, y: 56, scale: 1.3 },
    ],
    near: [
      { part: "bucket", x: 50, y: 96, scale: 1.2 },
      { part: "grass-tuft", x: 14, y: 98, count: 2, spread: [6, 1] },
      { part: "flowers", x: 86, y: 98 },
    ],
    light: [{ part: "halo", x: 50, y: 52, scale: 1, paint: "white" }],
  },
};

/** Pile On (Classic #60, Epic): warriors, traps and banners tumble out of a burst supply wagon in a heap and pile onto a narrow bridge, crowding every lane, while a fortress gate is thrown open and more pour out; chaotic, rowdy afternoon. */
const PILE_ON: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "blue-200"],
  light: { x: 80, y: 22, r: 36, paint: "white", opacity: 0.35 },
  emblem: { x: 58, y: 56, size: 5, rotate: 0 },
  extras: [
    { layer: "subject", x: 38, y: 74, scale: 0.55, pose: "pose-spread", look: { ...WARRIOR, item: null } },
    { layer: "subject", x: 64, y: 82, scale: 0.6, flip: -1, pose: "pose-run", look: { ...WARRIOR, item: "item-sword", cloth: "red-900" } },
  ],
  layers: {
    sky: [
      { part: "sun", x: 82, y: 20, scale: 0.8 },
    ],
    far: [
      { part: "hills", x: 50, y: 48, paint: "green-700" },
      { part: "fortress", x: 50, y: 47, scale: 1.1, paint: "stone-500" },
      { part: "soldier", x: 46, y: 52, scale: 0.36, count: 2, spread: [8, 1] },
    ],
    mid: [
      { part: "water", x: 50, y: 58, paint: "blue-600" },
      { part: "bridge", x: 50, y: 60, scale: 1.3, paint: "stone-500" },
      { part: "soldier", x: 26, y: 60, scale: 0.6, count: 2, spread: [48, 1] },
      { part: "banner", x: 76, y: 52, scale: 0.8, paint: "red-600" },
    ],
    subject: [
      { part: "cart", x: 50, y: 86, scale: 2, paint: "wood-600" },
      { part: "crate-big", x: 40, y: 82, scale: 1 },
      { part: "banner", x: 58, y: 70, scale: 1.2, paint: "red-600" },
      { part: "snare", x: 52, y: 78, scale: 2.4 },
    ],
    near: [
      { part: "crate", x: 14, y: 98, scale: 1.2 },
      { part: "shield", x: 30, y: 98, scale: 1 },
    ],
    light: [{ part: "rim-light", x: 50, y: 70, scale: 1.6, paint: "orange-400" }],
    particles: [
      { part: "dust", x: 50, y: 84, scale: 1.4, paint: "stone-500" },
    ],
  },
};

/** Cookie Guild (Classic #31, Common): a guild baker in a frosting-trimmed brown robe and sugar-crystal badge offers a new recruit a sealed cookie; secret bakery cellar, warm oven glow. */
const COOKIE_GUILD: SceneRecipe = {
  family: "classic",
  sky: ["wood-900", "wood-800"],
  light: { x: 16, y: 60, r: 36, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 84, y: 34, size: 5, rotate: 0 },
  figure: { x: 42, y: 86, scale: 1, pose: "pose-hold", look: { ...figureLookFor(["Human"]), hair: "toque", outfit: "outfit-robe", item: "item-cookie", cloth: "wood-700" } },
  extras: [{ layer: "subject", x: 70, y: 86, scale: 0.78, flip: -1, pose: "pose-stand", look: { ...VILLAGER, hair: "hood", cloth: "slate-400" } }],
  layers: {
    far: [
      { part: "tile-wall", x: 50, y: 50, paint: "stone-500" },
      { part: "hearth", x: 16, y: 66, scale: 1.1 },
      { part: "banner", x: 84, y: 28, scale: 1.1, paint: "wood-700" },
    ],
    mid: [
      { part: "boards", x: 50, y: 66, paint: "wood-700" },
      { part: "shelf", x: 84, y: 66, scale: 1 },
    ],
    subject: [{ part: "sacks", x: 22, y: 84, scale: 1 }],
    near: [
      { part: "crystal", x: 42, y: 54, scale: 0.4, paint: "white" },
      { part: "bread", x: 86, y: 97, scale: 1.4 },
      { part: "barrel", x: 10, y: 99, scale: 1 },
    ],
    light: [
      { part: "halo", x: 16, y: 56, scale: 1.6, paint: "orange-400" },
      { part: "rim-light", x: 46, y: 62, scale: 1.4, paint: "orange-400" },
    ],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.2, paint: "white" }],
  },
};

/** Joro (Classic #33, Legendary): Joro, a grinning spider-folk acrobat with four lanky arms and a patched violet vest, drops on a silver thread between an ally knight and an incoming arcane bolt, taking the hit with a wink; a web-strung bell tower interior, the bolt's blue glare cutting through dusty shafts of evening light. */
const JORO: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "wood-700"],
  light: { x: 80, y: 48, r: 34, paint: "sky-400", opacity: 0.4 },
  emblem: { x: 31, y: 66, size: 5, rotate: 0 },
  figure: { x: 52, y: 74, scale: 0.9, pose: "pose-spread", look: { ...figureLookFor([]), head: "head-human", hair: "hair-spiky", outfit: "outfit-spider", item: null, skin: "red-300", cloth: "violet-700" } },
  extras: [{ layer: "subject", x: 24, y: 92, scale: 0.9, pose: "pose-guard", look: { ...WARRIOR, item: "item-shield", cloth: "blue-800" } }],
  layers: {
    sky: [{ part: "round-window", x: 80, y: 30, scale: 1.2, paint: "orange-400" }],
    far: [
      { part: "tile-wall", x: 50, y: 50, paint: "slate-700" },
      { part: "arch", x: 18, y: 62, scale: 1.3, paint: "stone-500" },
      { part: "web", x: 14, y: 16, scale: 0.8, paint: "slate-100" },
      { part: "web", x: 88, y: 70, scale: 0.7, paint: "slate-100" },
    ],
    mid: [
      { part: "boards", x: 50, y: 74, paint: "wood-700" },
      { part: "bell", x: 50, y: 4, scale: 2.2, paint: "brass-600" },
      { part: "ladder", x: 88, y: 84, scale: 1 },
    ],
    subject: [
      { part: "web", x: 50, y: 36, scale: 1.4, paint: "slate-100" },
      { part: "strings", x: 52, y: 32, scale: 0.3, paint: "slate-100" },
      { part: "fireball", x: 76, y: 46, scale: 1.4, flip: -1, paint: "sky-400" },
      { part: "burst", x: 62, y: 46, scale: 1, paint: "sky-300" },
    ],
    near: [
      { part: "crate", x: 86, y: 99, scale: 1.2 },
      { part: "pebbles", x: 50, y: 98 },
    ],
    light: [
      { part: "light-shaft", x: 74, y: 22, scale: 1.2, paint: "orange-400" },
      { part: "light-shaft", x: 86, y: 22, scale: 1, paint: "orange-400" },
      { part: "halo", x: 70, y: 46, scale: 1.6, paint: "sky-400" },
      { part: "rim-light", x: 52, y: 52, scale: 1.4, paint: "sky-300" },
    ],
    particles: [
      { part: "dust", x: 70, y: 60, scale: 1.2, paint: "orange-400" },
      { part: "sparks", x: 64, y: 44, scale: 1.2, paint: "sky-300" },
    ],
  },
};

/** MC Tech (Classic #40, Rare): a wiry tinkerer in goggles and a leather apron reels an enemy's clockwork contraption in with a magnetic grapple arm; cluttered stone workshop, teal sparks. */
const MC_TECH: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "slate-700"],
  light: { x: 58, y: 50, r: 34, paint: "cyan-400", opacity: 0.4 },
  emblem: { x: 18, y: 68, size: 5, rotate: 0 },
  figure: { x: 36, y: 86, scale: 1, pose: "pose-cast", look: { ...figureLookFor(["Human"]), hair: "goggles", outfit: "outfit-work-apron", item: "item-grapple", cloth: "wood-600" } },
  layers: {
    far: [
      { part: "stone-wall", x: 50, y: 62, scale: 2.2, paint: "stone-500" },
      { part: "shelf", x: 16, y: 50, scale: 1 },
      { part: "pipes", x: 84, y: 40, scale: 1, paint: "brass-600" },
      { part: "cog", x: 70, y: 26, scale: 1.4, paint: "slate-400" },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 66, paint: "slate-700" },
      { part: "anvil", x: 16, y: 80, scale: 1 },
      { part: "crate", x: 86, y: 80, scale: 1 },
    ],
    subject: [
      { part: "clockwork-sentry", x: 66, y: 82, scale: 1.2 },
      { part: "zap", x: 58, y: 54, scale: 0.6, paint: "cyan-400" },
    ],
    near: [
      { part: "gear", x: 14, y: 97, scale: 1.2, paint: "brass-600" },
      { part: "cog", x: 86, y: 98, scale: 1, paint: "slate-400" },
    ],
    light: [{ part: "halo", x: 56, y: 52, scale: 1.4, paint: "cyan-400" }],
    particles: [{ part: "sparks", x: 58, y: 54, scale: 1.2, count: 2, spread: [10, 8], paint: "cyan-400" }],
  },
};

/** Transmutable Toxins (Classic #42, Rare): an alchemist's plague laboratory, bubbling green vats, ally homunculi swelling stronger in one tank while hostile ones shrivel in the next; acid-green glow. */
const TRANSMUTABLE_TOXINS: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "green-900"],
  light: { x: 40, y: 54, r: 36, paint: "lime-400", opacity: 0.4 },
  emblem: { x: 38, y: 82, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "tile-wall", x: 50, y: 50, paint: "slate-700" },
      { part: "shelf", x: 14, y: 48, scale: 1 },
      { part: "shelf", x: 86, y: 48, scale: 1 },
      { part: "pipes", x: 50, y: 22, scale: 1.4, paint: "brass-600" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66, paint: "slate-700" },
      { part: "cauldron", x: 86, y: 80, scale: 1 },
      { part: "flask", x: 16, y: 78, scale: 1.2 },
    ],
    subject: [
      { part: "vat", x: 38, y: 80, scale: 1.6, paint: "green-400" },
      { part: "vat", x: 64, y: 80, scale: 1.2, paint: "green-900" },
      { part: "bubbles", x: 38, y: 50, scale: 1.2, paint: "lime-400" },
    ],
    near: [
      { part: "slime", x: 84, y: 98, scale: 0.5 },
      { part: "vial", x: 14, y: 97, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 38, y: 58, scale: 1.6, paint: "lime-400" },
      { part: "rim-light", x: 50, y: 66, scale: 1.4, paint: "green-400" },
    ],
    particles: [
      { part: "steam", x: 64, y: 46, scale: 1, paint: "slate-400" },
      { part: "spore-motes", x: 50, y: 40, scale: 1.2, paint: "lime-400" },
    ],
  },
};

/** Plague Nuke (Classic #43, Epic): a vast green cloud of plague spores erupts over a battlefield, every warrior toppled in its blast ring, bright blue mana crystals rising from the ashes into a scorched sky; apocalyptic, eerie calm. */
const PLAGUE_NUKE: SceneRecipe = {
  family: "classic",
  sky: ["red-900", "brass-800"],
  light: { x: 50, y: 46, r: 40, paint: "lime-400", opacity: 0.4 },
  emblem: { x: 50, y: 44, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "crystal", x: 18, y: 34, scale: 1.4, count: 2, spread: [8, 10], paint: "sky-400" }],
    far: [
      { part: "hills", x: 50, y: 60, paint: "slate-800" },
      { part: "crystal", x: 84, y: 40, scale: 1.6, count: 2, spread: [8, 10], paint: "sky-400" },
      { part: "dead-tree", x: 12, y: 62, scale: 0.8, paint: "slate-900" },
      { part: "ruin-wall", x: 86, y: 62, scale: 0.8, flip: -1, paint: "slate-700" },
    ],
    mid: [
      { part: "cracked-earth", x: 50, y: 66, paint: "stone-500" },
      { part: "blast-rings", x: 50, y: 76, scale: 2.2, paint: "lime-400" },
      { part: "soldier-fallen", x: 22, y: 78, scale: 1.2 },
      { part: "soldier-fallen", x: 78, y: 80, scale: 1.2, flip: -1 },
    ],
    subject: [
      { part: "blast-crater", x: 50, y: 80, scale: 0.8 },
      { part: "smoke-billow", x: 50, y: 66, scale: 2.4, paint: "green-400" },
      { part: "spores", x: 50, y: 44, scale: 1.6, paint: "lime-200" },
    ],
    near: [
      { part: "broken-spear", x: 16, y: 98, scale: 1.2 },
      { part: "soldier-fallen", x: 84, y: 99, scale: 1.4 },
      { part: "smoke", x: 50, y: 99, scale: 1.2, paint: "green-700" },
    ],
    light: [
      { part: "halo", x: 50, y: 48, scale: 2.2, paint: "lime-400" },
      { part: "rim-light", x: 50, y: 76, scale: 1.6, paint: "green-400" },
    ],
    particles: [
      { part: "spore-motes", x: 50, y: 50, scale: 1.8, paint: "lime-400" },
      { part: "motes", x: 80, y: 40, scale: 1.2, paint: "sky-300" },
    ],
  },
};

/** Back from the GY (Classic #44, Legendary): a rain-soaked hillside graveyard where a necromancer's lantern bursts violet and a row of fallen warriors claws up from their graves at once, soil and broken coffin lids flying, while the spell card itself crumbles to ash behind her; lightning splits the clouds, cold violet light on black earth. */
const BACK_FROM_THE_GY: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "violet-700"],
  light: { x: 58, y: 46, r: 38, paint: "violet-500", opacity: 0.45 },
  emblem: { x: 70, y: 38, size: 5, rotate: 0 },
  figure: { x: 50, y: 86, scale: 1, pose: "pose-raise", look: { ...figureLookFor(["Human"]), hair: "hood", outfit: "outfit-robe", item: "item-lantern", skin: "slate-100", cloth: "slate-900" } },
  layers: {
    sky: [
      { part: "cloud", x: 30, y: 16, scale: 1.2, count: 2, spread: [14, 4], paint: "slate-800" },
      { part: "lightning", x: 80, y: 10, scale: 0.9, paint: "violet-300" },
    ],
    far: [
      { part: "hills", x: 50, y: 56, paint: "slate-900" },
      { part: "dead-tree", x: 86, y: 58, scale: 0.9, paint: "slate-900" },
      { part: "tombstone", x: 30, y: 58, scale: 1.2, count: 3, spread: [40, 2], paint: "slate-700" },
    ],
    mid: [
      { part: "field", x: 50, y: 66, paint: "slate-900" },
      { part: "skeleton", x: 20, y: 80, scale: 0.8 },
      { part: "skeleton", x: 80, y: 80, scale: 0.75, flip: -1 },
      { part: "grave-mound", x: 20, y: 82, scale: 1.4 },
      { part: "grave-mound", x: 80, y: 82, scale: 1.4 },
      { part: "bone-hand", x: 34, y: 76, scale: 0.9 },
      { part: "bone-hand", x: 66, y: 76, scale: 0.9, flip: -1 },
    ],
    subject: [
      { part: "card", x: 68, y: 50, scale: 1.8, paint: "violet-700", fade: 0.6 },
      { part: "embers", x: 70, y: 40, scale: 1.2, paint: "slate-400" },
    ],
    near: [
      { part: "grave-mound", x: 12, y: 99, scale: 2 },
      { part: "splinters", x: 22, y: 86, scale: 1.2, paint: "wood-700" },
      { part: "bone-hand", x: 88, y: 99, scale: 1.2, flip: -1 },
      { part: "plank-rug", x: 84, y: 90, scale: 0.6, paint: "wood-800" },
    ],
    light: [
      { part: "halo", x: 60, y: 34, scale: 1.8, paint: "violet-500" },
      { part: "rim-light", x: 50, y: 66, scale: 1.6, paint: "violet-300" },
    ],
    particles: [
      { part: "rain", x: 50, y: 40, scale: 1.4, paint: "slate-400" },
      { part: "sparks", x: 60, y: 32, scale: 1.2, paint: "violet-300" },
      { part: "dust", x: 20, y: 80, scale: 1, paint: "wood-800" },
    ],
  },
};

/** Nature Titan (Classic #45, Legendary): a mountain-sized titan of moss, root and boulder strides out of an ancient overgrown temple, each footfall bursting into wildflowers, cupping a wounded hero in one leafy palm as healing light seeps between its fingers and seed-cards drift from its shoulders; dawn mist, green-shafted forest light. */
const NATURE_TITAN: SceneRecipe = {
  family: "classic",
  sky: ["green-900", "green-200"],
  light: { x: 62, y: 46, r: 34, paint: "green-200", opacity: 0.45 },
  emblem: { x: 84, y: 80, size: 5, rotate: 0 },
  figure: { x: 50, y: 88, scale: 1.3, pose: "pose-stride", look: { ...figureLookFor([]), head: "head-golem", hair: "leaf-crown", outfit: "outfit-leaves", item: null, skin: "stone-500", cloth: "green-700" } },
  extras: [{ layer: "near", x: 62, y: 52, scale: 0.28, pose: "pose-sit", look: { ...WARRIOR, item: null, cloth: "red-900" } }],
  layers: {
    sky: [{ part: "mist", x: 50, y: 30, scale: 2.4, paint: "white" }],
    far: [
      { part: "temple", x: 50, y: 64, scale: 1.4, paint: "stone-500" },
      { part: "vine", x: 34, y: 34, scale: 1, paint: "green-700" },
      { part: "great-tree", x: 10, y: 66, scale: 0.9 },
      { part: "pine", x: 90, y: 64, scale: 1, paint: "green-900" },
    ],
    mid: [
      { part: "field", x: 50, y: 68, paint: "green-700" },
      { part: "fern", x: 20, y: 80, scale: 1 },
      { part: "bush", x: 82, y: 80, scale: 0.9 },
      { part: "mist", x: 50, y: 70, scale: 2.2, paint: "white" },
    ],
    subject: [{ part: "flowers", x: 50, y: 90, scale: 1.2, count: 2, spread: [16, 1], paint: "red-300" }],
    near: [
      { part: "heal-glow", x: 62, y: 48, scale: 1.2, paint: "green-200" },
      { part: "fern", x: 8, y: 100, scale: 1.3 },
      { part: "mossy-boulder", x: 86, y: 99, scale: 1 },
    ],
    light: [
      { part: "light-shaft", x: 20, y: 0, scale: 1.2, paint: "green-200" },
      { part: "light-shaft", x: 72, y: 0, scale: 1.1, paint: "green-200" },
      { part: "halo", x: 62, y: 48, scale: 1.4, paint: "green-200" },
    ],
    particles: [
      { part: "playing-card", x: 34, y: 26, scale: 0.8, count: 3, spread: [10, 14], paint: "green-700" },
      { part: "leaves", x: 66, y: 22, scale: 1.2, paint: "green-500" },
      { part: "petals", x: 50, y: 84, scale: 1.2, paint: "red-300" },
    ],
  },
};

/** Recurring Felinor (Classic #47, Rare): a sleek tabby Felinor rogue in a hooded scarf springs back out of a just-sprung trapdoor, grinning; rain-slick slate rooftops at twilight, cool teal. */
const RECURRING_FELINOR: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "jade-400"],
  light: { x: 50, y: 70, r: 32, paint: "cyan-400", opacity: 0.4 },
  emblem: { x: 84, y: 70, size: 5, rotate: 0 },
  figure: { x: 50, y: 74, scale: 0.95, pose: "pose-run", look: { ...figureLookFor(["Felinor"]), hair: "hood", outfit: "outfit-travel-cloak", item: null, skin: "orange-400", cloth: "slate-700" } },
  layers: {
    sky: [{ part: "moon", x: 80, y: 18, scale: 0.8 }],
    far: [
      { part: "townhouse", x: 14, y: 58, scale: 1, paint: "slate-800" },
      { part: "townhouse", x: 86, y: 60, scale: 0.9, paint: "slate-800" },
      { part: "tower", x: 30, y: 54, scale: 0.7, paint: "slate-800" },
    ],
    mid: [{ part: "tile-floor", x: 50, y: 70, paint: "slate-700" }],
    subject: [{ part: "hatch", x: 50, y: 84, scale: 1.6, paint: "slate-400" }],
    near: [
      { part: "streetlamp", x: 88, y: 100, scale: 0.8 },
      { part: "drops", x: 20, y: 96, scale: 1.2, paint: "sky-300" },
    ],
    light: [
      { part: "halo", x: 50, y: 74, scale: 1.4, paint: "cyan-400" },
      { part: "rim-light", x: 50, y: 70, scale: 1.4, paint: "sky-300" },
    ],
    particles: [{ part: "rain", x: 50, y: 40, scale: 1.4, paint: "sky-300" }],
  },
};

/** Hired Shrimp (Classic #48, Common): a tiny pink shrimp mercenary in a dented helmet brandishes a short sword and a coin pouch, chest puffed; tide-pool harbour rocks, bright sea light. */
const HIRED_SHRIMP: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "blue-200"],
  light: { x: 74, y: 22, r: 38, paint: "white", opacity: 0.4 },
  emblem: { x: 80, y: 76, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun", x: 78, y: 18, scale: 0.8 },
      { part: "gull", x: 24, y: 24, scale: 0.6 },
    ],
    far: [
      { part: "sea", x: 50, y: 56, paint: "blue-600" },
      { part: "lighthouse", x: 88, y: 56, scale: 0.4 },
      { part: "boat", x: 16, y: 58, scale: 0.4 },
    ],
    mid: [
      { part: "still-water", x: 50, y: 76, scale: 1.2, paint: "sky-300" },
      { part: "mossy-boulder", x: 14, y: 82, scale: 0.8, paint: "slate-400" },
      { part: "mossy-boulder", x: 86, y: 84, scale: 0.9, flip: -1, paint: "slate-400" },
    ],
    subject: [
      { part: "mossy-boulder", x: 50, y: 88, scale: 1.2 },
      { part: "shrimp", x: 50, y: 80, scale: 2.5 },
    ],
    near: [
      { part: "pebbles", x: 20, y: 97, count: 2, spread: [8, 2] },
      { part: "spray", x: 84, y: 92, scale: 1.4, paint: "white" },
    ],
    light: [{ part: "halo", x: 50, y: 50, scale: 1.4, paint: "white" }],
    particles: [{ part: "bubbles", x: 30, y: 70, scale: 1, paint: "white" }],
  },
};

/** Voidwalker (Classic #50, Rare): a gaunt pilgrim made of starless void, cloak trailing, walks a cypress-lined burial road as each gravestone behind it sinks into nothing; dead-grey dusk. */
const VOIDWALKER: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "grey-500"],
  light: { x: 50, y: 56, r: 34, paint: "slate-400", opacity: 0.35 },
  emblem: { x: 22, y: 64, size: 5, rotate: 0 },
  figure: { x: 50, y: 86, scale: 1.1, pose: "pose-stride", look: { ...figureLookFor([]), head: "head-void", hair: "hood", outfit: "outfit-travel-cloak", item: null, skin: "black", cloth: "slate-900" } },
  layers: {
    sky: [{ part: "cloud", x: 70, y: 22, scale: 1, count: 2, spread: [14, 4], paint: "slate-400" }],
    far: [
      { part: "hills", x: 50, y: 60, paint: "slate-700" },
      { part: "pine", x: 12, y: 64, scale: 1.1, count: 2, spread: [10, 1], paint: "slate-800" },
      { part: "pine", x: 88, y: 64, scale: 1.1, count: 2, spread: [10, 1], paint: "slate-800" },
    ],
    mid: [
      { part: "cobbles", x: 50, y: 66, paint: "slate-400" },
      { part: "tombstone", x: 22, y: 72, scale: 1.6, paint: "stone-500" },
      { part: "tombstone", x: 34, y: 70, scale: 1.4, paint: "stone-500", fade: 0.5 },
      { part: "tombstone", x: 68, y: 70, scale: 1.4, paint: "stone-500", fade: 0.25 },
      { part: "void-swirl", x: 68, y: 70, scale: 0.6, paint: "slate-900" },
    ],
    subject: [{ part: "void-swirl", x: 38, y: 74, scale: 0.5, paint: "violet-700" }],
    near: [
      { part: "pine", x: 6, y: 100, scale: 1.4, paint: "slate-900" },
      { part: "mist", x: 70, y: 94, scale: 1.8, paint: "slate-400" },
    ],
    light: [{ part: "rim-light", x: 50, y: 60, scale: 1.4, paint: "slate-100" }],
    particles: [{ part: "motes", x: 50, y: 50, scale: 1.2, paint: "slate-400" }],
  },
};

/** Plague Crawler (Classic #53, Common): a many-legged plague crawler in tiny spectacles scratches its itchy pustules while reading a miniature tome; damp library basement, green lamp. */
const PLAGUE_CRAWLER: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "green-900"],
  light: { x: 50, y: 24, r: 36, paint: "green-400", opacity: 0.4 },
  emblem: { x: 60, y: 70, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "stone-wall", x: 50, y: 62, scale: 2.2, paint: "slate-700" },
      { part: "bookshelf", x: 14, y: 64, scale: 1 },
      { part: "bookshelf", x: 86, y: 64, scale: 1 },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 66, paint: "slate-800" },
      { part: "lantern", x: 50, y: 14, scale: 1.2, paint: "green-400" },
      { part: "barrel", x: 82, y: 80, scale: 0.9 },
    ],
    subject: [
      { part: "crawler", x: 48, y: 82, scale: 2.4 },
      { part: "tome", x: 80, y: 86, scale: 0.8 },
    ],
    near: [
      { part: "closed-tome", x: 14, y: 98, scale: 1.2 },
      { part: "still-water", x: 70, y: 96, scale: 0.6, paint: "slate-700" },
    ],
    light: [{ part: "halo", x: 50, y: 22, scale: 1.4, paint: "green-400" }],
    particles: [
      { part: "drops", x: 30, y: 40, scale: 1, paint: "blue-200" },
      { part: "spore-motes", x: 46, y: 66, scale: 1, paint: "lime-400" },
    ],
  },
};

/** Book of Wildfire (Classic #55, Epic): a tome's pages catch fire and become a burning forest, flames leaping between pines and chasing deer, the silhouettes of other books forming in the smoke of a red hilltop sky; wild, hungry blaze. */
const BOOK_OF_WILDFIRE: SceneRecipe = {
  family: "classic",
  sky: ["red-900", "orange-400"],
  light: { x: 50, y: 66, r: 36, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 50, y: 82, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "smoke", x: 30, y: 30, scale: 1.6, paint: "slate-700" }],
    far: [
      { part: "hills", x: 50, y: 58, paint: "wood-800" },
      { part: "closed-tome", x: 28, y: 30, scale: 2, paint: "slate-900", fade: 0.4 },
      { part: "closed-tome", x: 72, y: 24, scale: 1.6, paint: "slate-900", fade: 0.3 },
      { part: "smoke", x: 72, y: 26, scale: 1.4, paint: "slate-700" },
    ],
    mid: [
      { part: "pine", x: 14, y: 72, scale: 1, count: 2, spread: [10, 2], paint: "green-900" },
      { part: "pine", x: 84, y: 72, scale: 1, count: 2, spread: [10, 2], paint: "green-900" },
      { part: "flames", x: 22, y: 70, scale: 1.2 },
      { part: "flames", x: 66, y: 70, scale: 1.1 },
      { part: "leaping-deer", x: 84, y: 76, scale: 1.2 },
    ],
    subject: [
      { part: "tome", x: 50, y: 86, scale: 3, paint: "red-900" },
      { part: "pine", x: 44, y: 76, scale: 0.6, paint: "green-900" },
      { part: "pine", x: 56, y: 75, scale: 0.55, paint: "green-900" },
      { part: "flames", x: 50, y: 74, scale: 1.6 },
    ],
    near: [
      { part: "log", x: 14, y: 98, scale: 1.2 },
      { part: "flames", x: 88, y: 99, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 50, y: 66, scale: 2, paint: "orange-400" },
      { part: "rim-light", x: 50, y: 70, scale: 1.6, paint: "ember-500" },
    ],
    particles: [{ part: "embers", x: 50, y: 50, scale: 1.4, count: 2, spread: [24, 10], paint: "orange-400" }],
  },
};

/** Spell Tyrant (Classic #56, Legendary): the Spell Tyrant, a towering sorcerer-king in a jagged violet crown and robes stitched from old spell scrolls, raises his staff in a ruined throne hall as three ghostly spells rise from a cracked grave-urn at his feet, blaze out on his command and crumble to ash; storm-violet backlight, an imperious last show. */
const SPELL_TYRANT: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "violet-700"],
  light: { x: 50, y: 40, r: 40, paint: "violet-500", opacity: 0.45 },
  emblem: { x: 30, y: 74, size: 5, rotate: 0 },
  figure: { x: 54, y: 86, scale: 1.15, pose: "pose-raise", look: { ...figureLookFor(["Human"]), hair: "jagged-crown", outfit: "outfit-robe", item: "item-staff", skin: "slate-100", cloth: "violet-700" } },
  layers: {
    sky: [
      { part: "lightning", x: 16, y: 8, scale: 0.8, paint: "violet-300" },
      { part: "cloud", x: 76, y: 16, scale: 1, paint: "slate-800" },
    ],
    far: [
      { part: "ruin-wall", x: 14, y: 62, scale: 1.2, paint: "slate-700" },
      { part: "column", x: 84, y: 62, scale: 1.3, paint: "stone-500" },
      { part: "throne", x: 54, y: 64, scale: 2, paint: "slate-700" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66, paint: "slate-700" },
      { part: "fallen-column", x: 80, y: 76, scale: 0.9 },
      { part: "scroll", x: 70, y: 82, scale: 1.2, paint: "slate-100" },
    ],
    subject: [
      { part: "jar", x: 32, y: 84, scale: 2.4, paint: "stone-500" },
      { part: "splinters", x: 32, y: 76, scale: 0.8, paint: "slate-400" },
      { part: "ghost", x: 30, y: 56, scale: 1, paint: "violet-300", fade: 0.6 },
      { part: "fireball", x: 40, y: 36, scale: 1, paint: "violet-500", fade: 0.6 },
      { part: "rune", x: 68, y: 30, scale: 1.1, paint: "violet-300", fade: 0.6 },
    ],
    near: [
      { part: "fallen-column", x: 14, y: 98, scale: 1 },
      { part: "scroll", x: 86, y: 98, scale: 1.2, paint: "slate-100" },
    ],
    light: [
      { part: "halo", x: 54, y: 40, scale: 2.4, paint: "violet-500" },
      { part: "rim-light", x: 54, y: 60, scale: 1.6, paint: "violet-300" },
    ],
    particles: [
      { part: "embers", x: 40, y: 30, scale: 1.2, paint: "slate-400" },
      { part: "sparks", x: 66, y: 24, scale: 1.2, paint: "violet-300" },
    ],
  },
};

/** Plague Doctor (Classic #59, Common): a beak-masked plague doctor in a waxed black coat tallies the sick on an abacus, a sealed bill in hand; candlelit sickhouse ward, sickly green. */
const PLAGUE_DOCTOR: SceneRecipe = {
  family: "classic",
  sky: ["slate-900", "green-900"],
  light: { x: 22, y: 50, r: 34, paint: "lime-200", opacity: 0.35 },
  emblem: { x: 84, y: 40, size: 5, rotate: 0 },
  figure: { x: 46, y: 86, scale: 1, pose: "pose-hold", look: { ...figureLookFor(["Human"]), hair: "beak-mask", outfit: "outfit-coat", item: "item-abacus", cloth: "black" } },
  layers: {
    far: [
      { part: "stone-wall", x: 50, y: 62, scale: 2.2, paint: "slate-800" },
      { part: "window", x: 84, y: 46, scale: 1.2, paint: "green-400" },
      { part: "candle", x: 22, y: 50, scale: 1.2, count: 2, spread: [8, 1] },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 66, paint: "slate-700" },
      { part: "cot", x: 16, y: 78, scale: 1.4 },
      { part: "cot", x: 84, y: 78, scale: 1.4, flip: -1 },
    ],
    subject: [{ part: "envelope", x: 37, y: 58, scale: 1.1 }],
    near: [
      { part: "candle", x: 12, y: 98, scale: 1.4 },
      { part: "bucket", x: 86, y: 98, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 22, y: 46, scale: 1.4, paint: "lime-200" },
      { part: "rim-light", x: 46, y: 62, scale: 1.4, paint: "green-400" },
    ],
    particles: [{ part: "spore-motes", x: 50, y: 50, scale: 1.2, paint: "lime-400" }],
  },
};

export const CLASSIC_RECIPES_2: Readonly<Record<string, SceneRecipe>> = {
  "classic-031": COOKIE_GUILD,
  "classic-032": FELINOR_FEELINGS,
  "classic-033": JORO,
  "classic-034": ANCIENT_ACQUISITION,
  "classic-035": PREP,
  "classic-036": BURN,
  "classic-037": LAST_HURRAH,
  "classic-038": JACKIESTAN_AUCTIONEER,
  "classic-039": OUTBREAK,
  "classic-040": MC_TECH,
  "classic-041": STATE_OF_THE_GAME,
  "classic-042": TRANSMUTABLE_TOXINS,
  "classic-043": PLAGUE_NUKE,
  "classic-044": BACK_FROM_THE_GY,
  "classic-045": NATURE_TITAN,
  "classic-046": DIVINE_FAVOR,
  "classic-047": RECURRING_FELINOR,
  "classic-048": HIRED_SHRIMP,
  "classic-049": ANTI_GREED_MACHINE,
  "classic-050": VOIDWALKER,
  "classic-051": BACK_BREAKER,
  "classic-052": FINAL_GAMBIT,
  "classic-053": PLAGUE_CRAWLER,
  "classic-054": REWIND,
  "classic-055": BOOK_OF_WILDFIRE,
  "classic-056": SPELL_TYRANT,
  "classic-057": ECHO,
  "classic-058": COMMON_RESOURCES,
  "classic-059": PLAGUE_DOCTOR,
  "classic-060": PILE_ON,
};
