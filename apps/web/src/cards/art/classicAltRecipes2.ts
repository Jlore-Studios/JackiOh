// R3388000: Classic's alt scene recipes, part 2 of three (#27 to #54), one per card offered Alt art, each drawn from its
// brief (#3388's table; ART.md, "Classic's Alt art restaged"): the card's own subject in another place, time or moment,
// under a sky sharing no colour with its normal picture's and a new camera (a close-up, a wide or distant view, or one
// from above), its figure the normal picture's person (classicAltRecipes1.ts `normalClassicLookOf`), in Classic's
// temple-stone look and at or above its rarity's floor. scene.ts `sceneRecipeOf` reads one under the card's Alt art seed
// alone, at every Alt art level. Presentation only (CLAUDE.md rule 7): a recipe is read by the card's id alone.

import { normalClassicExtraLookOf, normalClassicLookOf } from "./classicAltRecipes1.ts";
import { figureLookFor } from "./figureKit.ts";
import type { SceneRecipe } from "./scene.ts";

/** Pestilent Slime (Classic #27, Common), Alt art: Wide shot of the slime family basking on a sun-warmed rock beside a clear alpine stream, pustules glistening; blue sky and grey stone. Stand-ins: a `cascade` for the alpine stream's falls. */
const PESTILENT_SLIME_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "blue-200"],
  light: { x: 44, y: 62, r: 34, paint: "white", opacity: 0.4 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun-disc", x: 26, y: 24, scale: 1 },
      { part: "cloud", x: 72, y: 22, scale: 1.1, count: 2, spread: [16, 3], paint: "white" },
    ],
    far: [
      { part: "mountains", x: 50, y: 48, paint: "slate-400" },
      { part: "pine", x: 10, y: 60, scale: 1, count: 3, spread: [6, 2], paint: "green-700" },
    ],
    mid: [
      { part: "field", x: 50, y: 60, paint: "green-500" },
      { part: "cascade", x: 76, y: 46, scale: 1 },
      { part: "water", x: 50, y: 74, paint: "sky-300" },
      { part: "mossy-boulder", x: 44, y: 76, scale: 2.2, paint: "slate-400" },
    ],
    subject: [
      { part: "slime", x: 44, y: 62, scale: 0.6 },
      { part: "slime", x: 35, y: 63, scale: 0.35 },
      { part: "slime", x: 53, y: 63, scale: 0.3, flip: -1 },
    ],
    near: [
      { part: "mossy-boulder", x: 10, y: 98, scale: 1.4, paint: "slate-400" },
      { part: "grass-tuft", x: 88, y: 97, count: 3, spread: [8, 2], paint: "green-500" },
      { part: "pebbles", x: 30, y: 92 },
    ],
    light: [
      { part: "halo", x: 44, y: 60, scale: 1, paint: "white" },
      { part: "rim-light", x: 44, y: 56, scale: 1, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 44, y: 58, scale: 1, paint: "white" },
      { part: "spray", x: 76, y: 88, scale: 1.2, paint: "white" },
    ],
  },
};

/** Second Wind (Classic #28, Legendary), Alt art: After the storm: a still autumn road at dusk, the hero kneeling as fallen cards drift back up from the leaves around her; burnt orange and plum. */
const SECOND_WIND_ALT: SceneRecipe = {
  family: "classic",
  sky: ["orange-400", "violet-700"],
  light: { x: 50, y: 56, r: 40, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  figure: { x: 50, y: 76, scale: 0.5, pose: "pose-kneel", look: { ...normalClassicLookOf("classic-028") } },
  layers: {
    sky: [
      { part: "sun-disc", x: 50, y: 52, scale: 1.4 },
      { part: "cloud", x: 24, y: 22, scale: 1.4, count: 3, spread: [24, 4], paint: "violet-700" },
      { part: "cloud", x: 80, y: 26, scale: 1.1, count: 2, spread: [10, 3], paint: "violet-500" },
    ],
    far: [
      { part: "hills", x: 50, y: 56, paint: "violet-700" },
      { part: "tree", x: 12, y: 62, scale: 1.4, paint: "orange-400" },
      { part: "tree", x: 88, y: 60, scale: 1.3, flip: -1, paint: "red-600" },
      { part: "dead-tree", x: 70, y: 60, scale: 0.8, paint: "wood-800" },
    ],
    mid: [
      { part: "field", x: 50, y: 62, paint: "wood-700" },
      { part: "road", x: 50, y: 62 },
      { part: "tree", x: 24, y: 74, scale: 1.6, paint: "red-600" },
      { part: "tree", x: 78, y: 76, scale: 1.7, flip: -1, paint: "orange-400" },
      { part: "fence-post", x: 64, y: 72, scale: 0.8, count: 2, spread: [6, 1] },
    ],
    subject: [
      { part: "leaves", x: 50, y: 78, scale: 1.2 },
      { part: "playing-card", x: 50, y: 50, scale: 0.9, count: 6, spread: [22, 12], paint: "slate-100" },
    ],
    near: [
      { part: "leaves", x: 16, y: 94, scale: 1.4 },
      { part: "leaves", x: 84, y: 96, scale: 1.4 },
      { part: "tall-grass", x: 8, y: 99, scale: 1.4, paint: "wood-600" },
      { part: "pebbles", x: 60, y: 97 },
    ],
    light: [
      { part: "god-rays", x: 50, y: 52, scale: 0.9, paint: "orange-400" },
      { part: "halo", x: 50, y: 52, scale: 1.8, paint: "orange-400" },
      { part: "rim-light", x: 50, y: 62, scale: 0.7, paint: "orange-400" },
    ],
    particles: [
      { part: "leaves", x: 36, y: 40, scale: 1 },
      { part: "playing-card", x: 70, y: 34, scale: 0.6, count: 2, spread: [8, 6], fade: 0.6 },
      { part: "motes", x: 50, y: 56, scale: 1.2, paint: "white" },
    ],
  },
};

/** Cookie Guild (Classic #31, Common), Alt art: The guild's secret recruitment parade on a rainy morning street, the baker handing out cookies from under an umbrella; slate blue and frosting pink. Stand-ins: an `awning` for the umbrella. */
const COOKIE_GUILD_ALT: SceneRecipe = {
  family: "classic",
  sky: ["slate-400", "red-300"],
  light: { x: 50, y: 50, r: 34, paint: "red-300", opacity: 0.35 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  figure: { x: 44, y: 84, scale: 0.7, pose: "pose-hold", look: { ...normalClassicLookOf("classic-031") } },
  extras: [{ layer: "subject", x: 68, y: 86, scale: 0.6, flip: -1, pose: "pose-reach", look: { ...normalClassicExtraLookOf("classic-031", 0) } }],
  layers: {
    sky: [{ part: "cloud", x: 40, y: 22, scale: 1.4, count: 3, spread: [30, 4], paint: "slate-700" }],
    far: [
      { part: "townhouse", x: 16, y: 62, scale: 1, paint: "slate-700" },
      { part: "townhouse", x: 86, y: 62, scale: 1, paint: "blue-800" },
      { part: "bunting", x: 50, y: 34, scale: 1.4 },
    ],
    mid: [
      { part: "street", x: 50, y: 62, paint: "slate-700" },
      { part: "awning", x: 50, y: 86, scale: 2.3, paint: "red-300" },
      { part: "streetlamp", x: 82, y: 80, scale: 0.8 },
    ],
    subject: [{ part: "drops", x: 30, y: 80, scale: 0.8, paint: "sky-300" }],
    near: [
      { part: "drops", x: 14, y: 96, scale: 1.2, paint: "sky-300" },
      { part: "bread", x: 88, y: 97, scale: 1.2 },
    ],
    light: [{ part: "rim-light", x: 46, y: 60, scale: 0.9, paint: "red-300" }],
    particles: [
      { part: "rain", x: 50, y: 40, scale: 1.4, paint: "slate-100" },
      { part: "rain", x: 50, y: 72, scale: 1.2, paint: "slate-100" },
    ],
  },
};

/** Felinor Feelings (Classic #32, Rare), Alt art: Close-up of the kitten's paw on the kneeling warrior's helmet in a snowy pine forest at twilight; white and blue-grey. */
const FELINOR_FEELINGS_ALT: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "slate-400"],
  light: { x: 50, y: 40, r: 40, paint: "white", opacity: 0.35 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  figure: { x: 38, y: 63, scale: 0.9, pose: "pose-sit", look: { ...normalClassicLookOf("classic-032") } },
  extras: [{ layer: "mid", x: 54, y: 116, scale: 1.6, flip: -1, pose: "pose-genuflect", look: { ...normalClassicExtraLookOf("classic-032", 0) } }],
  layers: {
    sky: [
      { part: "moon", x: 76, y: 24, scale: 0.7 },
      { part: "stars", x: 50, y: 24, scale: 1, paint: "white" },
    ],
    far: [
      { part: "pine", x: 10, y: 66, scale: 2.2, count: 2, spread: [8, 2], paint: "slate-800" },
      { part: "pine", x: 90, y: 64, scale: 2, count: 2, spread: [8, 2], paint: "slate-800" },
      { part: "snowfield", x: 50, y: 64 },
    ],
    mid: [{ part: "pine", x: 26, y: 78, scale: 1.4, paint: "slate-700" }],
    near: [
      { part: "pine", x: 92, y: 104, scale: 2.4, flip: -1, paint: "slate-900" },
      { part: "snow", x: 50, y: 92, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 42, y: 44, scale: 1.2, paint: "white" },
      { part: "rim-light", x: 54, y: 50, scale: 1.4, paint: "slate-100" },
    ],
    particles: [{ part: "snow", x: 50, y: 40, scale: 1.4 }],
  },
};

/** Joro (Classic #33, Legendary), Alt art: Close-up portrait of Joro hanging upside down on his thread in a dewy morning meadow, grinning, silk sparkling; fresh green and pink. Stand-ins: he hangs upright from his `strings` (a figure cannot be turned); `sparks` for the sparkling silk. */
const JORO_ALT: SceneRecipe = {
  family: "classic",
  sky: ["green-200", "red-300"],
  light: { x: 50, y: 40, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 18, y: 76, size: 5, rotate: 0 },
  figure: { x: 50, y: 118, scale: 1.6, pose: "pose-raise", look: { ...normalClassicLookOf("classic-033") } },
  layers: {
    sky: [
      { part: "sun-disc", x: 82, y: 24, scale: 1 },
      { part: "cloud", x: 24, y: 22, scale: 1, count: 2, spread: [12, 3], paint: "white" },
    ],
    far: [
      { part: "hills", x: 50, y: 60, paint: "green-500" },
      { part: "tree", x: 10, y: 64, scale: 1.4, paint: "green-700" },
      { part: "tree", x: 90, y: 62, scale: 1.2, flip: -1, paint: "green-700" },
    ],
    mid: [
      { part: "field", x: 50, y: 70, paint: "green-500" },
      { part: "web", x: 50, y: 40, scale: 2.4, paint: "white" },
      { part: "flowers", x: 18, y: 82, scale: 1.6, count: 3, spread: [8, 2], paint: "red-300" },
      { part: "flowers", x: 84, y: 84, scale: 1.6, count: 3, spread: [8, 2], paint: "red-300" },
    ],
    subject: [
      { part: "strings", x: 68, y: 28, scale: 0.06, paint: "white" },
      { part: "strings", x: 32, y: 30, scale: 0.06, paint: "white" },
    ],
    near: [
      { part: "tall-grass", x: 10, y: 100, scale: 1.8, paint: "green-500" },
      { part: "tall-grass", x: 90, y: 100, scale: 1.6, paint: "green-500" },
      { part: "drops", x: 76, y: 60, scale: 0.8, paint: "white" },
    ],
    light: [
      { part: "light-shaft", x: 80, y: 0, scale: 1.2, paint: "white" },
      { part: "rim-light", x: 50, y: 60, scale: 1.4, paint: "red-300" },
    ],
    particles: [
      { part: "sparks", x: 50, y: 36, scale: 1.6, count: 2, spread: [18, 8], paint: "white" },
      { part: "petals", x: 50, y: 50, scale: 1.4, paint: "red-300" },
    ],
  },
};

/** Ancient Acquisition (Classic #34, Rare), Alt art: A diver hauls the glowing ancient card up from a sunken temple on the seabed, light shafting down from above; deep teal. Stand-ins: a `space-helmet` for the diving helmet. */
const ANCIENT_ACQUISITION_ALT: SceneRecipe = {
  family: "classic",
  sky: ["blue-900", "blue-800"],
  light: { x: 50, y: 30, r: 40, paint: "cyan-400", opacity: 0.35 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  extras: [
    {
      layer: "subject",
      x: 50,
      y: 66,
      scale: 0.55,
      pose: "pose-lift",
      look: { ...normalClassicExtraLookOf("classic-034", 0), hair: "space-helmet", item: null },
    },
  ],
  layers: {
    sky: [{ part: "light-shaft", x: 50, y: 0, scale: 1.4, paint: "cyan-400" }],
    far: [
      { part: "sand", x: 50, y: 74, paint: "jade-400" },
      { part: "temple", x: 50, y: 84, scale: 0.8, paint: "slate-400" },
    ],
    mid: [
      { part: "reed", x: 14, y: 82, scale: 1.8, count: 3, spread: [5, 1], paint: "jade-400" },
      { part: "fern", x: 86, y: 84, scale: 1.6, paint: "green-700" },
      { part: "koi", x: 80, y: 40, scale: 1 },
    ],
    subject: [
      { part: "card", x: 50, y: 30, scale: 1.4, paint: "slate-100" },
      { part: "bubbles", x: 56, y: 40, scale: 0.8, paint: "white" },
    ],
    near: [
      { part: "pebbles", x: 24, y: 96 },
      { part: "reed", x: 90, y: 100, scale: 2.2, count: 2, spread: [4, 1], paint: "green-700" },
    ],
    light: [
      { part: "halo", x: 50, y: 25, scale: 1.6, paint: "cyan-400" },
      { part: "light-shaft", x: 30, y: 0, scale: 1, paint: "white" },
    ],
    particles: [
      { part: "bubbles", x: 30, y: 50, scale: 1.2, paint: "white" },
      { part: "motes", x: 50, y: 50, scale: 1.4, paint: "cyan-400" },
    ],
  },
};

/** Prep (Classic #35, Common), Alt art: Close-up of the mage's hands tying a knotted measuring cord over a chalk circle beside a rain-streaked window; grey-blue and chalk white. */
const PREP_ALT: SceneRecipe = {
  family: "classic",
  sky: ["slate-700", "slate-400"],
  light: { x: 74, y: 34, r: 34, paint: "slate-100", opacity: 0.35 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 58, paint: "slate-700" }],
    far: [
      { part: "window", x: 74, y: 52, scale: 2 },
      { part: "rain", x: 74, y: 36, scale: 0.7, paint: "slate-400" },
    ],
    mid: [
      { part: "tile-floor", x: 50, y: 58, paint: "slate-800" },
      { part: "candle", x: 18, y: 62, scale: 1.4 },
    ],
    subject: [
      { part: "rune-circle", x: 50, y: 70, scale: 2.6, paint: "white" },
      { part: "red-thread", x: 33, y: 56, scale: 1.15 },
      { part: "open-hand", x: 12, y: 62, scale: 2.2 },
      { part: "open-hand", x: 88, y: 56, scale: 2.2, flip: -1 },
    ],
    near: [
      { part: "dust", x: 50, y: 80, scale: 1.4, paint: "white" },
      { part: "drops", x: 84, y: 92, scale: 0.8, paint: "slate-100" },
    ],
    light: [
      { part: "halo", x: 50, y: 68, scale: 1.6, paint: "white" },
      { part: "light-shaft", x: 74, y: 30, scale: 1, paint: "slate-100" },
    ],
    particles: [{ part: "motes", x: 50, y: 60, scale: 1.2, paint: "white" }],
  },
};

/** Jackiestan Auctioneer (Classic #38, Rare), Alt art: The auctioneer alone on an open-air block at a windy seaside fish market, gavel raised to wheeling gulls; sea blue and white. Stand-ins: a `crate-big` for the auction block. */
const JACKIESTAN_AUCTIONEER_ALT: SceneRecipe = {
  family: "classic",
  sky: ["sky-400", "slate-100"],
  light: { x: 30, y: 24, r: 40, paint: "white", opacity: 0.45 },
  emblem: { x: 84, y: 64, size: 5, rotate: 0 },
  figure: { x: 38, y: 74, scale: 0.7, pose: "pose-strike", look: { ...normalClassicLookOf("classic-038") } },
  layers: {
    sky: [{ part: "cloud", x: 70, y: 20, scale: 1.2, count: 2, spread: [20, 4], paint: "white" }],
    far: [
      { part: "sea", x: 50, y: 56, paint: "blue-600" },
      { part: "lighthouse", x: 90, y: 58, scale: 0.6 },
      { part: "boat", x: 14, y: 58, scale: 0.5 },
    ],
    mid: [
      { part: "cobbles", x: 50, y: 66, paint: "slate-400" },
      { part: "market-stall", x: 76, y: 84, scale: 1.1 },
      { part: "crate-big", x: 38, y: 88, scale: 0.4 },
    ],
    subject: [{ part: "gull", x: 60, y: 32, scale: 1.6, count: 5, spread: [28, 12] }],
    near: [
      { part: "crate", x: 10, y: 98, scale: 1.4 },
      { part: "barrel", x: 90, y: 99, scale: 1 },
    ],
    light: [{ part: "rim-light", x: 38, y: 50, scale: 0.9, paint: "white" }],
    particles: [
      { part: "gust", x: 50, y: 44, scale: 1.4, paint: "white" },
      { part: "spray", x: 20, y: 64, scale: 1.2, paint: "white" },
    ],
  },
};

/** Transmutable Toxins (Classic #42, Rare), Alt art: The lab's outflow: a pink toxic river through a jungle, frogs swelling and shrinking on its banks; magenta and deep green. Stand-ins: the `vat` spilling its brew for the lab's outflow. */
const TRANSMUTABLE_TOXINS_ALT: SceneRecipe = {
  family: "classic",
  sky: ["green-700", "purple-400"],
  light: { x: 28, y: 52, r: 34, paint: "purple-400", opacity: 0.4 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "palm", x: 8, y: 62, scale: 1.4, paint: "green-900" },
      { part: "bamboo", x: 90, y: 62, scale: 1.2, paint: "green-900" },
      { part: "tree", x: 70, y: 58, scale: 1.4, paint: "green-900" },
      { part: "vine", x: 50, y: 34, scale: 1.4, paint: "green-900" },
    ],
    mid: [
      { part: "field", x: 50, y: 60, paint: "green-900" },
      { part: "water", x: 50, y: 66, paint: "purple-400" },
      { part: "fern", x: 14, y: 76, scale: 1.8, paint: "green-700" },
      { part: "mossy-boulder", x: 64, y: 80, scale: 1.6, paint: "green-700" },
    ],
    subject: [
      { part: "vat", x: 28, y: 66, scale: 1, paint: "purple-400" },
      { part: "drops", x: 36, y: 64, scale: 0.8, paint: "purple-400" },
      { part: "frog", x: 64, y: 74, scale: 1.6 },
      { part: "frog", x: 82, y: 72, scale: 0.6, flip: -1 },
    ],
    near: [
      { part: "fern", x: 10, y: 100, scale: 2.4, paint: "green-900" },
      { part: "palm", x: 98, y: 110, scale: 1.8, flip: -1, paint: "green-900" },
    ],
    light: [
      { part: "halo", x: 28, y: 52, scale: 1.2, paint: "purple-400" },
      { part: "rim-light", x: 64, y: 64, scale: 1, paint: "purple-400" },
    ],
    particles: [
      { part: "bubbles", x: 50, y: 72, scale: 1.2, paint: "purple-400" },
      { part: "steam", x: 28, y: 36, scale: 1, paint: "purple-400" },
    ],
  },
};

/** Plague Nuke (Classic #43, Epic), Alt art: Close-up after the blast: a lone survivor's hand catches a falling blue mana crystal in drifting grey ash; ash grey and blue. Stand-ins: grey `spores` for the ash. */
const PLAGUE_NUKE_ALT: SceneRecipe = {
  family: "classic",
  sky: ["slate-400", "grey-500"],
  light: { x: 54, y: 52, r: 30, paint: "sky-400", opacity: 0.4 },
  emblem: { x: 82, y: 30, size: 5, rotate: 0 },
  layers: {
    far: [
      { part: "hills", x: 50, y: 58, paint: "slate-700" },
      { part: "dead-tree", x: 14, y: 62, scale: 0.9, paint: "slate-800" },
      { part: "ruin-wall", x: 84, y: 62, scale: 0.8, flip: -1, paint: "slate-700" },
    ],
    mid: [
      { part: "cracked-earth", x: 50, y: 66, paint: "grey-500" },
      { part: "smoke", x: 76, y: 66, scale: 1.2, paint: "slate-400" },
    ],
    subject: [
      { part: "spores", x: 50, y: 40, scale: 3.6, paint: "slate-100" },
      { part: "open-hand", x: 34, y: 76, scale: 3.6 },
      { part: "crystal", x: 56, y: 62, scale: 2.6, paint: "sky-400" },
    ],
    near: [
      { part: "smoke", x: 12, y: 98, scale: 1.4, paint: "slate-700" },
      { part: "broken-spear", x: 86, y: 98, scale: 1.2 },
    ],
    light: [
      { part: "halo", x: 50, y: 56, scale: 1.4, paint: "sky-400" },
      { part: "rim-light", x: 40, y: 70, scale: 1, paint: "sky-300" },
    ],
    particles: [
      { part: "spores", x: 30, y: 30, scale: 2, paint: "slate-100" },
      { part: "motes", x: 60, y: 50, scale: 1.4, paint: "white" },
    ],
  },
};

/** Nature Titan (Classic #45, Legendary), Alt art: Winter: the titan asleep under snow on a mountainside, one mossy hand thawing a ring of crocuses around a resting hero; white and violet. Stand-ins: violet `flowers` for the crocuses. */
const NATURE_TITAN_ALT: SceneRecipe = {
  family: "classic",
  sky: ["slate-100", "violet-300"],
  light: { x: 40, y: 70, r: 34, paint: "violet-300", opacity: 0.45 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  figure: { x: 64, y: 74, scale: 1.05, flip: -1, pose: "pose-sit", look: { ...normalClassicLookOf("classic-045") } },
  extras: [{ layer: "subject", x: 40, y: 76, scale: 0.34, pose: "pose-sit", look: { ...normalClassicExtraLookOf("classic-045", 0) } }],
  layers: {
    sky: [
      { part: "sun-disc", x: 82, y: 22, scale: 0.8 },
      { part: "cloud", x: 30, y: 22, scale: 1.2, count: 2, spread: [16, 3], paint: "white" },
    ],
    far: [
      { part: "mountains", x: 50, y: 50, paint: "violet-300" },
      { part: "pine", x: 10, y: 62, scale: 1.2, paint: "slate-700" },
      { part: "pine", x: 92, y: 60, scale: 1, paint: "slate-700" },
    ],
    mid: [
      { part: "snowfield", x: 50, y: 62 },
      { part: "mossy-boulder", x: 66, y: 76, scale: 2, paint: "white" },
    ],
    subject: [{ part: "flowers", x: 40, y: 75, scale: 1, count: 4, spread: [8, 1], paint: "violet-500" }],
    near: [{ part: "flowers", x: 40, y: 80, scale: 1.2, count: 3, spread: [9, 1], paint: "violet-300" }],
    light: [
      { part: "heal-glow", x: 46, y: 66, scale: 1.2, paint: "green-200" },
      { part: "halo", x: 40, y: 72, scale: 1.4, paint: "violet-300" },
      { part: "light-shaft", x: 30, y: 0, scale: 1.2, paint: "white" },
    ],
    particles: [
      { part: "snow", x: 50, y: 40, scale: 1.4 },
      { part: "petals", x: 40, y: 66, scale: 1, paint: "violet-300" },
    ],
  },
};

/** Divine Favor (Classic #46, Rare), Alt art: A fishing village at sunset, cards drifting down like petals into an old fisher's net; rose and deep blue. Stand-ins: a `cargo-net` for the fisher's net. */
const DIVINE_FAVOR_ALT: SceneRecipe = {
  family: "classic",
  sky: ["blue-900", "red-400"],
  light: { x: 50, y: 56, r: 40, paint: "red-400", opacity: 0.45 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  figure: { x: 36, y: 80, scale: 0.6, pose: "pose-reach", look: { ...normalClassicLookOf("classic-046"), hair: "hair-silver", cloth: "blue-800" } },
  layers: {
    sky: [
      { part: "sun-disc", x: 50, y: 56, scale: 1.4 },
      { part: "cloud", x: 72, y: 26, scale: 1.2, count: 2, spread: [16, 4], paint: "red-300" },
    ],
    far: [
      { part: "sea", x: 50, y: 58, paint: "blue-800" },
      { part: "whitewashed-house", x: 12, y: 62, scale: 0.8 },
      { part: "house", x: 86, y: 62, scale: 0.6 },
      { part: "boat", x: 66, y: 60, scale: 0.4 },
    ],
    mid: [
      { part: "boards", x: 50, y: 70, paint: "wood-700" },
      { part: "cargo-net", x: 58, y: 82, scale: 0.9 },
    ],
    subject: [{ part: "playing-card", x: 56, y: 40, scale: 0.9, count: 5, spread: [16, 16], paint: "slate-100" }],
    near: [
      { part: "bucket", x: 12, y: 97, scale: 1.2 },
      { part: "barrel", x: 88, y: 99, scale: 1 },
    ],
    light: [{ part: "rim-light", x: 36, y: 60, scale: 0.8, paint: "red-400" }],
    particles: [
      { part: "petals", x: 50, y: 40, scale: 1.4, paint: "red-300" },
      { part: "motes", x: 56, y: 60, scale: 1.2, paint: "white" },
    ],
  },
};

/** Hired Shrimp (Classic #48, Common), Alt art: Close-up of the shrimp, helmet off, counting coins on a barracks bunk in late afternoon light; warm brown and red. Stand-ins: a `cot` for the bunk; a `helmet` set down beside him for the one he took off. */
const HIRED_SHRIMP_ALT: SceneRecipe = {
  family: "classic",
  sky: ["wood-700", "red-600"],
  light: { x: 78, y: 30, r: 40, paint: "orange-400", opacity: 0.45 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [{ part: "room-wall", x: 50, y: 62, paint: "wood-800" }],
    far: [
      { part: "window", x: 80, y: 50, scale: 1.4 },
      { part: "weapon-rack", x: 14, y: 64, scale: 1.2 },
    ],
    mid: [
      { part: "boards", x: 50, y: 62, paint: "wood-700" },
      { part: "cot", x: 50, y: 88, scale: 4.4 },
    ],
    subject: [
      { part: "shrimp", x: 44, y: 84, scale: 3.2 },
      { part: "coins", x: 76, y: 80, scale: 1.8 },
      { part: "helmet", x: 18, y: 80, scale: 2.4 },
    ],
    near: [{ part: "coin", x: 68, y: 84, count: 3, spread: [10, 2] }],
    light: [
      { part: "light-shaft", x: 80, y: 30, scale: 1.4, paint: "orange-400" },
      { part: "rim-light", x: 50, y: 50, scale: 1.6, paint: "orange-400" },
    ],
    particles: [{ part: "motes", x: 60, y: 46, scale: 1.4, paint: "orange-400" }],
  },
};

/** Anti-Greed Machine (Classic #49, Common), Alt art: The automaton on patrol in a misty jungle ruin, vine-wrapped and mossy, blocking a treasure hunter's path; green and grey. */
const ANTI_GREED_MACHINE_ALT: SceneRecipe = {
  family: "classic",
  sky: ["green-700", "slate-400"],
  light: { x: 60, y: 50, r: 36, paint: "green-200", opacity: 0.35 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  figure: { x: 60, y: 82, scale: 0.8, flip: -1, pose: "pose-block", look: { ...normalClassicLookOf("classic-049") } },
  extras: [
    {
      layer: "subject",
      x: 26,
      y: 84,
      scale: 0.58,
      pose: "pose-hold",
      look: { ...figureLookFor(["Human"]), hair: "hat-brim", outfit: "outfit-jacket", item: "item-torch", cloth: "wood-600" },
    },
  ],
  layers: {
    far: [
      { part: "temple", x: 56, y: 66, scale: 1.2, paint: "slate-400" },
      { part: "vine", x: 40, y: 44, scale: 1.4, paint: "green-700" },
      { part: "vine", x: 72, y: 42, scale: 1.2, paint: "green-700" },
      { part: "palm", x: 8, y: 66, scale: 1.4, paint: "green-900" },
      { part: "mist", x: 50, y: 56, scale: 1.6, paint: "slate-100" },
    ],
    mid: [
      { part: "rock-floor", x: 50, y: 70, paint: "slate-700" },
      { part: "treasure-chest", x: 80, y: 84, scale: 1.4 },
      { part: "fern", x: 92, y: 82, scale: 1.6, paint: "green-700" },
    ],
    near: [
      { part: "vine", x: 56, y: 84, scale: 1.4, paint: "green-500" },
      { part: "vine", x: 64, y: 84, scale: 1.2, paint: "green-500" },
      { part: "mossy-boulder", x: 10, y: 98, scale: 1.6 },
      { part: "mist", x: 50, y: 94, scale: 1.4, paint: "slate-100" },
    ],
    light: [{ part: "light-shaft", x: 34, y: 0, scale: 1.2, paint: "green-200" }],
    particles: [{ part: "spore-motes", x: 50, y: 50, scale: 1.4, paint: "green-200" }],
  },
};

/** Back Breaker (Classic #51, Common), Alt art: A night storm on a moor: the brute splits a row of standing stones as lightning strikes; navy and white. */
const BACK_BREAKER_ALT: SceneRecipe = {
  family: "classic",
  sky: ["blue-900", "slate-900"],
  light: { x: 72, y: 30, r: 36, paint: "white", opacity: 0.35 },
  emblem: { x: 16, y: 30, size: 5, rotate: 0 },
  figure: { x: 38, y: 80, scale: 0.7, pose: "pose-strike", look: { ...normalClassicLookOf("classic-051") } },
  layers: {
    sky: [
      { part: "cloud", x: 40, y: 20, scale: 1.4, count: 3, spread: [30, 4], paint: "slate-800" },
      { part: "lightning", x: 72, y: 16, scale: 1.2, paint: "white" },
    ],
    far: [{ part: "hills", x: 50, y: 60, paint: "slate-800" }],
    mid: [
      { part: "field", x: 50, y: 66, paint: "slate-700" },
      { part: "standing-stone", x: 14, y: 76, scale: 1.1, paint: "slate-400" },
      { part: "standing-stone", x: 54, y: 78, scale: 1.3, paint: "slate-400" },
      { part: "standing-stone", x: 70, y: 76, scale: 1.2, paint: "slate-400" },
      { part: "standing-stone", x: 86, y: 74, scale: 1.1, paint: "slate-400" },
    ],
    subject: [{ part: "splinters", x: 54, y: 62, scale: 1.4, paint: "slate-100" }],
    near: [
      { part: "grass-tuft", x: 12, y: 97, count: 3, spread: [8, 2], paint: "slate-700" },
      { part: "pebbles", x: 70, y: 95 },
    ],
    light: [
      { part: "halo", x: 72, y: 18, scale: 1.4, paint: "white" },
      { part: "rim-light", x: 38, y: 56, scale: 1, paint: "white" },
    ],
    particles: [{ part: "rain", x: 50, y: 50, scale: 1.4, paint: "slate-100" }],
  },
};

/** Final Gambit (Classic #52, Epic), Alt art: Close-up of the glowing die frozen mid-tumble at a hero's fingertips, a calm sunrise lake behind; peach and soft blue. */
const FINAL_GAMBIT_ALT: SceneRecipe = {
  family: "classic",
  sky: ["red-300", "blue-200"],
  light: { x: 58, y: 50, r: 36, paint: "white", opacity: 0.5 },
  emblem: { x: 18, y: 30, size: 5, rotate: 0 },
  layers: {
    sky: [
      { part: "sun-disc", x: 74, y: 56, scale: 1.2 },
      { part: "cloud", x: 30, y: 24, scale: 1.2, count: 2, spread: [16, 4], paint: "white" },
    ],
    far: [
      { part: "hills", x: 50, y: 56, paint: "violet-300" },
      { part: "pine", x: 10, y: 60, scale: 0.8, count: 3, spread: [8, 1], paint: "slate-700" },
    ],
    mid: [{ part: "still-water", x: 50, y: 60 }],
    subject: [
      { part: "die", x: 58, y: 50, scale: 7, paint: "white" },
      { part: "open-hand", x: 34, y: 72, scale: 4 },
    ],
    near: [
      { part: "reed", x: 88, y: 98, scale: 2, count: 3, spread: [5, 1] },
      { part: "grass-tuft", x: 12, y: 98, count: 2, spread: [6, 1], paint: "green-700" },
    ],
    light: [
      { part: "halo", x: 58, y: 50, scale: 1.8, paint: "white" },
      { part: "god-rays", x: 74, y: 56, scale: 0.8, paint: "white" },
    ],
    particles: [
      { part: "sparks", x: 58, y: 50, scale: 1.6, paint: "white" },
      { part: "motes", x: 50, y: 60, scale: 1.2, paint: "white" },
    ],
  },
};

/** Rewind (Classic #54, Common), Alt art: A herald on a crowded autumn harvest-fair stage, his cry spiralling back through floating red leaves; red and brown. */
const REWIND_ALT: SceneRecipe = {
  family: "classic",
  sky: ["orange-400", "wood-600"],
  light: { x: 50, y: 40, r: 36, paint: "orange-400", opacity: 0.4 },
  emblem: { x: 84, y: 30, size: 5, rotate: 0 },
  figure: { x: 42, y: 72, scale: 0.7, pose: "pose-raise", look: { ...normalClassicLookOf("classic-054") } },
  extras: [{ layer: "subject", x: 58, y: 72, scale: 0.62, pose: "pose-raise", look: { ...normalClassicExtraLookOf("classic-054", 0) }, fade: 0.35 }],
  layers: {
    sky: [{ part: "bunting", x: 50, y: 26, scale: 1.6 }],
    far: [
      { part: "barn", x: 14, y: 62, scale: 0.9 },
      { part: "tree", x: 86, y: 62, scale: 1.4, paint: "red-600" },
      { part: "tent", x: 66, y: 60, scale: 0.8, paint: "red-600" },
    ],
    mid: [
      { part: "stage", x: 50, y: 72, paint: "wood-700" },
      { part: "pumpkin", x: 20, y: 74, scale: 1.2 },
      { part: "pumpkin", x: 78, y: 74, scale: 1 },
      { part: "hay-bale", x: 88, y: 76, scale: 0.9 },
    ],
    subject: [
      { part: "vortex", x: 56, y: 36, scale: 0.6, paint: "red-300" },
      { part: "echo-rings", x: 56, y: 36, scale: 0.8, paint: "white" },
    ],
    near: [{ part: "crowd", x: 50, y: 95, scale: 1.3, paint: "wood-800" }],
    light: [{ part: "rim-light", x: 42, y: 52, scale: 0.9, paint: "orange-400" }],
    particles: [
      { part: "leaves", x: 40, y: 36, scale: 1.2, paint: "red-600" },
      { part: "leaves", x: 70, y: 46, scale: 1, paint: "red-600" },
      { part: "petals", x: 50, y: 30, scale: 1.4, paint: "red-500" },
    ],
  },
};

/** R3388000: Classic's alt recipes #27 to #54, by card id. */
export const CLASSIC_ALT_RECIPES_2: Readonly<Record<string, SceneRecipe>> = {
  "classic-027": PESTILENT_SLIME_ALT,
  "classic-028": SECOND_WIND_ALT,
  "classic-031": COOKIE_GUILD_ALT,
  "classic-032": FELINOR_FEELINGS_ALT,
  "classic-033": JORO_ALT,
  "classic-034": ANCIENT_ACQUISITION_ALT,
  "classic-035": PREP_ALT,
  "classic-038": JACKIESTAN_AUCTIONEER_ALT,
  "classic-042": TRANSMUTABLE_TOXINS_ALT,
  "classic-043": PLAGUE_NUKE_ALT,
  "classic-045": NATURE_TITAN_ALT,
  "classic-046": DIVINE_FAVOR_ALT,
  "classic-048": HIRED_SHRIMP_ALT,
  "classic-049": ANTI_GREED_MACHINE_ALT,
  "classic-051": BACK_BREAKER_ALT,
  "classic-052": FINAL_GAMBIT_ALT,
  "classic-054": REWIND_ALT,
};
