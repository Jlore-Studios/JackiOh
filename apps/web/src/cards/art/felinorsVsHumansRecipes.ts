// R5264000: the held Felinors vs Humans set's placeholder art (#5264), one scene recipe per entry: its 115 cards and
// their 35 tokens (the 13 Jilliax modules among them) in the set's storybook look, and its 82 Plagiarized reprints
// (#116–#197) as their Infinite Assault originals' scenes run through a photocopier (ART.md, "The Felinors vs Humans
// set's placeholder scenes"). Every entry draws its scene through the procedural painter (`artSpec` reads a card's recipe
// by its id), so no dispatch site changes and no painter of its own is written. A scene is its template (a Human's town
// square, a Felinor's moonlit rooftops, KY's classroom, a beast's crag, a construct's forge yard, a Jilliax module's
// workbench, a living potion's laboratory, #84's temple, a spell's battlefield, a Book's library, a field's market, a
// trap's dungeon) in all seven layers, its subject the card's own part or figure, and `hashId(id)` varies its offsets,
// flips and looks so no two are alike. Every Felinor wears the cat-ears part on its head. A Legendary, a Mythic and a
// token printed either draw a set piece over the template (`setPiece`), from parts whose shapes are counted, so each
// clears its floor (richness.ts). A reprint draws its original's template (`assaultRecipe`, seeded by its own id) in a
// photocopy's greys, toner grain across it and a red stamp in its corner. No part turns (`rotate`) and no paint is gold.
// Presentation only (CLAUDE.md rule 7): a recipe is read by the card's id alone, and only ids under
// `FELINORS_VS_HUMANS_PREFIX` are keys here, so no shipped card's picture moves (R503). A later brief pass replaces them.

import { KY_TUTOR_LOOK, POSE_ANCHORS, type FigureLook, type HairId, type ItemId, type OutfitId, type PoseId } from "./figureKit.ts";
import { hashId, round2, seededRandom } from "./hash.ts";
import { assaultRecipe, type AssaultTemplate } from "./infiniteAssaultRecipes.ts";
import type { FigureRecipe, PaletteName, SceneLayer, ScenePlacement, SceneRecipe } from "./scene.ts";
import type { SceneryPartId } from "./sceneKit.ts";

export const FELINORS_VS_HUMANS_PREFIX = "felinorsvshumans-";

/** The template an original entry's scene is drawn from. */
export type FvhTemplate = "human" | "felinor" | "ky" | "egypt" | "beast" | "construct" | "module" | "potion" | "spell" | "book" | "field" | "trap";

/** The set piece a scene draws over its template: none, a Legendary's or a Mythic's, by the rarity the card prints (a token its printed one). */
export type FvhRank = "plain" | "legendary" | "mythic";

/**
 * What an entry's subject holds beyond its template: its own part (`prop`) and that part's paint, how many of it, the
 * scale of its subject against the template's (`size`: the figure's where there is one, else the part's), and a figure's
 * pose, look and ears' fur.
 */
export type FvhDetail = {
  readonly prop?: SceneryPartId;
  readonly paint?: PaletteName;
  readonly count?: number;
  readonly size?: number;
  readonly pose?: PoseId;
  readonly look?: Partial<FigureLook>;
  readonly fur?: PaletteName;
};

/** Every original entry of Felinors vs Humans, its tokens included, with its template, rank and detail, in card order. */
export const FELINORS_VS_HUMANS_SCENE_CARDS: readonly (readonly [string, FvhTemplate, FvhRank, FvhDetail?])[] = [
  ["felinorsvshumans-001", "human", "plain", { prop: "shards", paint: "slate-400" }], // NGMI Joe
  ["felinorsvshumans-002", "human", "legendary", { prop: "orb", paint: "cyan-400", look: { outfit: "outfit-labcoat" } }], // M.O.T.H.E.R
  ["felinorsvshumans-003", "ky", "legendary", { prop: "orb", paint: "violet-500", pose: "pose-cast" }], // KY's Pocket Galaxy
  ["felinorsvshumans-004", "ky", "legendary", { prop: "scroll", pose: "pose-point" }], // KY's Unification Mandate
  ["felinorsvshumans-005", "human", "plain", { prop: "candle", look: { hair: "hood" } }], // Reanimator
  ["felinorsvshumans-006", "human", "plain", { prop: "coin-stack" }], // Overpaid Intern
  ["felinorsvshumans-007", "spell", "plain", { prop: "bubbles", paint: "sky-300" }], // Whimsy!
  ["felinorsvshumans-008", "spell", "plain", { prop: "ink-hearts", size: 1.2 }], // Joy!
  ["felinorsvshumans-009", "spell", "plain", { prop: "heal-glow", paint: "violet-300" }], // Whimsy and Joy!
  ["felinorsvshumans-010", "human", "plain", { prop: "scroll", look: { hair: "hood", cloth: "slate-900" } }], // Shadow Judge
  ["felinorsvshumans-011", "construct", "plain", { prop: "hammer", paint: "slate-400", size: 1.8 }], // Ban Hammer
  ["felinorsvshumans-012", "human", "plain", { size: 1.2 }], // Mr. Big
  ["felinorsvshumans-013", "human", "plain", { size: 0.7 }], // Mr. Small
  ["felinorsvshumans-014", "beast", "plain", { prop: "beast", paint: "slate-700", size: 1.6 }], // Direwolf
  ["felinorsvshumans-015", "human", "plain", { prop: "flask", paint: "ember-500" }], // Unstable Magma-Cell
  ["felinorsvshumans-016", "spell", "legendary", { prop: "column", paint: "stone-500" }], // Travertine
  ["felinorsvshumans-016-1", "spell", "legendary", { prop: "column", paint: "stone-500", count: 2 }], // Travertine Stack
  ["felinorsvshumans-016-2", "construct", "legendary", { prop: "rock-giant", paint: "stone-500" }], // Travertine Golem
  ["felinorsvshumans-017", "construct", "plain", { prop: "mech", paint: "slate-700" }], // Jlockeed Shredder-14
  ["felinorsvshumans-018", "human", "plain", { prop: "coin-stack", look: { outfit: "outfit-suit" } }], // Villa-D Moneybags
  ["felinorsvshumans-019", "field", "plain", { prop: "battlement", paint: "red-600" }], // Trowel
  ["felinorsvshumans-019-1", "construct", "plain", { prop: "battlement", paint: "red-600" }], // Wall of Bricks
  ["felinorsvshumans-020", "field", "plain", { prop: "barrel", paint: "slate-700" }], // Bomb
  ["felinorsvshumans-020-1", "construct", "plain", { prop: "orb", paint: "black", size: 1.2 }], // Ticking Time Bomb
  ["felinorsvshumans-021", "spell", "plain", { prop: "hat", size: 1.4 }], // Scandalous Top Hat
  ["felinorsvshumans-022", "human", "plain", { prop: "hat", look: { hair: "hat-brim" } }], // The Mad Hatter
  ["felinorsvshumans-023", "book", "plain", { paint: "wood-700" }], // Book of Excavation
  ["felinorsvshumans-024", "book", "plain", { paint: "blue-800" }], // Book of Starflight
  ["felinorsvshumans-025", "book", "plain", { paint: "jade-400" }], // Book of Jade
  ["felinorsvshumans-026", "book", "plain", { paint: "violet-700" }], // Book of Entropy
  ["felinorsvshumans-027", "book", "plain", { paint: "red-900" }], // Book of Monstruo
  ["felinorsvshumans-028", "book", "plain", { paint: "slate-700" }], // Book of Design Principles
  ["felinorsvshumans-029", "book", "plain", { paint: "red-600" }], // Book of Legends
  ["felinorsvshumans-030", "book", "plain", { paint: "green-700", size: 1.3 }], // Big Book
  ["felinorsvshumans-031", "spell", "legendary", { prop: "heal-glow", paint: "white" }], // Desperate Plea
  ["felinorsvshumans-032", "field", "plain", { prop: "market-stall" }], // Refreshable Shop
  ["felinorsvshumans-033", "construct", "plain", { prop: "rock-giant" }], // Unstoppable Force
  ["felinorsvshumans-034", "spell", "mythic", { prop: "anvil", size: 1.4 }], // Half-True Craft a Card
  ["felinorsvshumans-035", "human", "plain", { pose: "pose-flex" }], // Looksmaxxer
  ["felinorsvshumans-036", "field", "plain", { prop: "orb", paint: "red-500" }], // Heartrock
  ["felinorsvshumans-037", "human", "legendary", { prop: "barrel", look: { head: "head-ogre", hair: null, outfit: "outfit-fur", item: null } }], // Old Grum
  ["felinorsvshumans-038", "spell", "plain", { prop: "coin-stack", size: 1.2 }], // Short Term Loan
  ["felinorsvshumans-039", "ky", "legendary", { look: { hair: "hat-pointed", outfit: "outfit-coat", item: "item-staff", cloth: "blue-800" } }], // Whisperer of KY
  ["felinorsvshumans-040", "human", "legendary", { prop: "crate", look: { hair: "hood", outfit: "outfit-travel-cloak", item: "item-staff" } }], // Travel Master Drungar
  ["felinorsvshumans-041", "human", "plain", { prop: "market-stall" }], // Icecream Vender
  ["felinorsvshumans-042", "human", "plain", { pose: "pose-flex", size: 1.15 }], // Big Billy
  ["felinorsvshumans-043", "beast", "plain", { prop: "dragon" }], // Dragons Bain
  ["felinorsvshumans-044", "felinor", "plain"], // Felinor Converter
  ["felinorsvshumans-045", "construct", "plain", { prop: "robot", paint: "lime-400" }], // Subject 9
  ["felinorsvshumans-046", "human", "legendary", { prop: "orb", paint: "violet-500", pose: "pose-cast", look: { item: "item-wand" } }], // Manifestor
  ["felinorsvshumans-047", "human", "plain", { pose: "pose-cast", look: { item: "item-wand" } }], // Blazing Battlemage
  ["felinorsvshumans-048", "human", "plain", { prop: "flask", paint: "green-500" }], // Toxic Timmy
  ["felinorsvshumans-049", "human", "plain", { prop: "candle" }], // Candle Master
  ["felinorsvshumans-049-1", "field", "plain", { prop: "candle", size: 1.6 }], // Candle Shot
  ["felinorsvshumans-050", "field", "legendary", { prop: "crate", size: 1.4 }], // Field Kit
  ["felinorsvshumans-051", "spell", "plain", { prop: "crate" }], // Scavengers Ingenuity
  ["felinorsvshumans-052", "human", "plain", { look: { item: "item-spear" } }], // Ace Hunter Jay
  ["felinorsvshumans-053", "human", "plain", { look: { item: "item-blaster" } }], // Jay Sharp Shooter
  ["felinorsvshumans-054", "human", "plain"], // Intrepid initiate
  ["felinorsvshumans-055", "spell", "plain", { prop: "house-cat", count: 3 }], // Adorable Infestation
  ["felinorsvshumans-056", "trap", "plain", { paint: "red-400" }], // Pressure Plate
  ["felinorsvshumans-057", "spell", "plain", { prop: "beast", paint: "violet-500" }], // Otherworldly Hunting
  ["felinorsvshumans-057-1", "spell", "plain", { prop: "beast", paint: "red-600" }], // Kill Command
  ["felinorsvshumans-057-2", "spell", "plain", { prop: "boar" }], // Tackling
  ["felinorsvshumans-057-3", "spell", "plain", { prop: "house-cat" }], // Felinor Companion
  ["felinorsvshumans-057-4", "felinor", "plain", { pose: "pose-guard", size: 1.2 }], // Wide Felinor
  ["felinorsvshumans-057-5", "felinor", "plain", { size: 0.85 }], // Narrow Felinor
  ["felinorsvshumans-057-6", "felinor", "plain", { pose: "pose-hold" }], // Square Felinor
  ["felinorsvshumans-058", "spell", "plain", { prop: "house-cat", count: 2, paint: "orange-400" }], // Call of the Shaped Felinors
  ["felinorsvshumans-059", "human", "plain", { prop: "closed-tome", look: { item: "item-book" } }], // Voracious Reader
  ["felinorsvshumans-060", "human", "legendary", { prop: "anvil", look: { outfit: "outfit-apron" } }], // Toolmaster Luna
  ["felinorsvshumans-060-1", "field", "legendary", { prop: "anvil", size: 1.4 }], // Tools of Creation
  ["felinorsvshumans-060-2", "field", "legendary", { prop: "forge" }], // Tools of Cremation
  ["felinorsvshumans-061", "spell", "plain", { prop: "orb", paint: "green-400" }], // Devolve
  ["felinorsvshumans-062", "felinor", "plain", { size: 1.15 }], // Tubbs
  ["felinorsvshumans-063", "human", "plain"], // S-Tier Speciesist
  ["felinorsvshumans-064", "human", "plain", { look: { item: "item-scroll" } }], // Fiender Artist
  ["felinorsvshumans-065", "felinor", "plain", { prop: "crate" }], // Felinor Forager
  ["felinorsvshumans-066", "spell", "plain", { prop: "orb", paint: "orange-400" }], // Fuzzyfruit
  ["felinorsvshumans-067", "human", "plain", { prop: "beast", paint: "slate-700", look: { item: "item-spear" } }], // Big Game Hunter
  ["felinorsvshumans-068", "human", "plain", { look: { hair: "fedora" } }], // Closeted Commentator
  ["felinorsvshumans-069", "felinor", "plain", { look: { hair: "headband" } }], // Cosplayer
  ["felinorsvshumans-070", "felinor", "legendary", { pose: "pose-point", look: { item: "item-banner" } }], // Felinor Commander
  ["felinorsvshumans-071", "felinor", "plain", { prop: "candle" }], // Felinor Reanimator
  ["felinorsvshumans-072", "spell", "plain", { prop: "tree", paint: "green-700" }], // Otherworldly Naturalism
  ["felinorsvshumans-072-1", "spell", "plain", { prop: "tree", paint: "green-500" }], // Wild Growth
  ["felinorsvshumans-072-2", "spell", "plain", { prop: "bear" }], // Swipe
  ["felinorsvshumans-072-3", "spell", "plain", { prop: "beast", paint: "wood-600" }], // Savage Roar
  ["felinorsvshumans-073", "human", "mythic", { look: { outfit: "outfit-plate", item: "item-sword", cloth: "red-900" } }], // Bloodforged Crusader
  ["felinorsvshumans-074", "human", "mythic", { pose: "pose-stride" }], // Yile, Acceleration Supremist
  ["felinorsvshumans-074-1", "human", "mythic", { pose: "pose-raise", size: 1.15 }], // Yile, Acceleration Supremist Prime
  ["felinorsvshumans-075", "human", "legendary", { look: { hair: "fedora", outfit: "outfit-vest" } }], // Illinois Jones
  ["felinorsvshumans-076", "human", "plain", { prop: "candle" }], // Desecrator
  ["felinorsvshumans-077", "spell", "plain", { prop: "flask", paint: "red-500" }], // Anaphylaxis
  ["felinorsvshumans-078", "human", "legendary", { pose: "pose-raise" }], // Human Harbinger
  ["felinorsvshumans-078-1", "human", "legendary", { pose: "pose-raise", size: 1.15 }], // Human Harbinger Prime
  ["felinorsvshumans-079", "human", "mythic", { look: { hair: "goggles", outfit: "outfit-labcoat" } }], // Post Human Aesthetist
  ["felinorsvshumans-080", "construct", "plain", { prop: "mech", paint: "slate-400" }], // Support Chinook
  ["felinorsvshumans-081", "felinor", "plain", { look: { hair: "spectacles" } }], // Old-Timer Catboy
  ["felinorsvshumans-082", "spell", "mythic", { prop: "house-cat", paint: "black", size: 1.6 }], // The Last Meow
  ["felinorsvshumans-083", "felinor", "plain", { look: { cloth: "green-700" } }], // Maomao
  ["felinorsvshumans-084", "egypt", "plain"], // 𓃠
  ["felinorsvshumans-085", "potion", "plain", { paint: "red-500" }], // Living Fire Potion
  ["felinorsvshumans-086", "potion", "plain", { paint: "green-500" }], // Living Poison Potion
  ["felinorsvshumans-087", "spell", "plain", { prop: "cauldron" }], // Baku's Bingeing
  ["felinorsvshumans-088", "felinor", "plain"], // Cattail
  ["felinorsvshumans-089", "felinor", "plain", { size: 1.3 }], // GIGA Felinor
  ["felinorsvshumans-090", "human", "plain", { pose: "pose-guard" }], // Lone Defender
  ["felinorsvshumans-091", "ky", "legendary", { prop: "notebook", pose: "pose-cast" }], // KY's Log Exp of the FFT
  ["felinorsvshumans-092", "ky", "mythic", { prop: "pages", pose: "pose-point" }], // KY's Patented Log 2 Utility
  ["felinorsvshumans-093", "beast", "plain", { prop: "beast", paint: "red-900", count: 2 }], // Twin Tyrant
  ["felinorsvshumans-094", "felinor", "plain", { fur: "black" }], // Bad Luck Felinor
  ["felinorsvshumans-095", "human", "plain", { look: { outfit: "outfit-plate" } }], // Platebreaker
  ["felinorsvshumans-096", "felinor", "plain", { pose: "pose-cast", look: { item: "item-wand" } }], // Magic Inclined Felinor
  ["felinorsvshumans-097", "felinor", "plain", { size: 0.85 }], // Paltry Felinor
  ["felinorsvshumans-098", "felinor", "legendary", { pose: "pose-cast" }], // Yarma
  ["felinorsvshumans-098-1", "felinor", "legendary", { pose: "pose-raise", size: 1.15 }], // Yarma Prime
  ["felinorsvshumans-098-2", "spell", "legendary", { prop: "rune-circle", paint: "violet-500", size: 1.4 }], // Dream Portal
  ["felinorsvshumans-099", "human", "legendary", { look: { hair: "hood", outfit: "outfit-travel-cloak", item: "item-staff" } }], // Jlorewalker Cho
  ["felinorsvshumans-100", "spell", "legendary", { prop: "orb", paint: "red-400", size: 1.3 }], // Call to Chaos (Felinors vs. Humans Edition)
  ["felinorsvshumans-100-1", "felinor", "legendary", { size: 1.2, look: { head: "head-golem", hair: null, outfit: "outfit-plate", item: null, skin: "stone-500", cloth: "slate-700" } }], // Felinor Golem
  ["felinorsvshumans-100-2", "human", "legendary", { size: 1.2, look: { head: "head-golem", hair: null, outfit: "outfit-plate", item: null, skin: "stone-500", cloth: "slate-700" } }], // Human Golem
  ["felinorsvshumans-101", "human", "plain"], // Dangerous Variant
  ["felinorsvshumans-102", "beast", "plain", { prop: "bird", paint: "brass-600" }], // Diving Gryphon
  ["felinorsvshumans-103", "human", "plain", { pose: "pose-stride" }], // Frantic Forager
  ["felinorsvshumans-104", "spell", "plain", { prop: "crow", count: 3 }], // Dark Skies
  ["felinorsvshumans-105", "felinor", "plain", { pose: "pose-lunge" }], // Maddended Felinor
  ["felinorsvshumans-106", "human", "plain", { prop: "shards", paint: "stone-500" }], // Rock Addict
  ["felinorsvshumans-107", "ky", "mythic", { pose: "pose-point", size: 1.1 }], // KY, Himself
  ["felinorsvshumans-108", "human", "plain"], // Rush Token Boss
  ["felinorsvshumans-109", "human", "legendary"], // Keymaster Kalabaster
  ["felinorsvshumans-110", "human", "legendary", { look: { cloth: "violet-700" } }], // Lord Jakrov
  ["felinorsvshumans-111", "beast", "plain", { prop: "bird", paint: "slate-900", count: 3, size: 0.5 }], // Vicious Fledgeling
  ["felinorsvshumans-112", "module", "mythic", { prop: "mech", paint: "blue-700", size: 0.8 }], // Jilliax Deluxe
  ["felinorsvshumans-112-1", "module", "mythic", { prop: "round-bot", paint: "cyan-400" }], // Recursion Module
  ["felinorsvshumans-112-2", "module", "mythic", { prop: "clockwork-sentry", paint: "slate-400", size: 0.8 }], // Durability Module
  ["felinorsvshumans-112-3", "module", "mythic", { prop: "robot", paint: "red-400", size: 0.8 }], // Haywire Module
  ["felinorsvshumans-112-4", "module", "mythic", { prop: "round-bot", paint: "violet-300" }], // Starpiece Module
  ["felinorsvshumans-112-5", "module", "mythic", { prop: "clockwork-sentry", paint: "sky-300", size: 0.8 }], // Pylon Module
  ["felinorsvshumans-112-6", "module", "mythic", { prop: "ai-core", paint: "orange-400", size: 0.7 }], // Power Module
  ["felinorsvshumans-112-7", "module", "mythic", { prop: "mech", paint: "slate-700", size: 0.8 }], // Heavy Module
  ["felinorsvshumans-112-8", "module", "mythic", { prop: "robot-arm", paint: "green-400", size: 0.6 }], // Utility Module
  ["felinorsvshumans-112-9", "module", "mythic", { prop: "round-bot", paint: "lime-400", count: 2, size: 0.6 }], // Twin Module
  ["felinorsvshumans-112-10", "module", "mythic", { prop: "robot", paint: "green-500", size: 0.8 }], // Viral Module
  ["felinorsvshumans-112-11", "module", "mythic", { prop: "ai-core", paint: "white", size: 0.7 }], // Perfect Module
  ["felinorsvshumans-112-12", "module", "mythic", { prop: "clockwork-sentry", paint: "purple-400", size: 0.8 }], // Debuff Module
  ["felinorsvshumans-112-13", "module", "mythic", { prop: "mech", paint: "blue-600", size: 0.8 }], // Indestructible Module
  ["felinorsvshumans-113", "human", "plain", { prop: "orb", paint: "white" }], // Ball Knower
  ["felinorsvshumans-114", "spell", "legendary", { prop: "scroll", size: 1.4 }], // Kethspiracy
  ["felinorsvshumans-115", "field", "legendary", { prop: "vending-machine" }], // Call to Chaos Vending Machine
];

/**
 * Every Plagiarized reprint (#116–#197): `[id, its Infinite Assault original, the original's template, rank]`, in card
 * order (`.fullsend/reprints-map.json`'s pairs; the template is the original's row of `INFINITE_ASSAULT_SCENE_CARDS`).
 */
export const FELINORS_VS_HUMANS_PLAGIARIZED: readonly (readonly [string, string, AssaultTemplate, FvhRank])[] = [
  ["felinorsvshumans-116", "infiniteassault-049", "unit", "plain"], // Plagiarized: Spark Plug
  ["felinorsvshumans-117", "infiniteassault-133", "swarm", "plain"], // Plagiarized: Backdoor
  ["felinorsvshumans-118", "infiniteassault-016", "swarm", "plain"], // Plagiarized: Benchmark Run
  ["felinorsvshumans-119", "infiniteassault-002", "unit", "plain"], // Plagiarized: Scrap Prototype
  ["felinorsvshumans-120", "infiniteassault-064", "unit", "plain"], // Plagiarized: Crash Test Dummy
  ["felinorsvshumans-121", "infiniteassault-194", "unit", "plain"], // Plagiarized: Compute Shard
  ["felinorsvshumans-122", "infiniteassault-170", "swarm", "plain"], // Plagiarized: Data Point
  ["felinorsvshumans-123", "infiniteassault-028", "swarm", "plain"], // Plagiarized: Doppelganger Protocol
  ["felinorsvshumans-124", "infiniteassault-029", "swarm", "plain"], // Plagiarized: False Positive
  ["felinorsvshumans-125", "infiniteassault-175", "unit", "plain"], // Plagiarized: Fingerprinter
  ["felinorsvshumans-126", "infiniteassault-005", "swarm", "plain"], // Plagiarized: Generated Filler
  ["felinorsvshumans-127", "infiniteassault-032", "unit", "plain"], // Plagiarized: Glitch Specter
  ["felinorsvshumans-128", "infiniteassault-204", "swarm", "plain"], // Plagiarized: Moore's Law
  ["felinorsvshumans-129", "infiniteassault-014", "swarm", "plain"], // Plagiarized: Overflow Error
  ["felinorsvshumans-130", "infiniteassault-121", "unit", "plain"], // Plagiarized: Packet Sniffer
  ["felinorsvshumans-131", "infiniteassault-184", "swarm", "plain"], // Plagiarized: Pattern Recognition
  ["felinorsvshumans-132", "infiniteassault-060", "unit", "plain"], // Plagiarized: Pit Crew
  ["felinorsvshumans-133", "infiniteassault-111", "swarm", "plain"], // Plagiarized: Redundant Copy
  ["felinorsvshumans-134", "infiniteassault-108", "swarm", "plain"], // Plagiarized: Second Draft
  ["felinorsvshumans-135", "infiniteassault-193", "unit", "plain"], // Plagiarized: Seedling Mind
  ["felinorsvshumans-136", "infiniteassault-087", "swarm", "plain"], // Plagiarized: Swarm Shield
  ["felinorsvshumans-137", "infiniteassault-203", "factory", "plain"], // Plagiarized: Compute Cluster
  ["felinorsvshumans-138", "infiniteassault-067", "factory", "plain"], // Plagiarized: Burning Ambition
  ["felinorsvshumans-139", "infiniteassault-105", "unit", "plain"], // Plagiarized: Callback
  ["felinorsvshumans-140", "infiniteassault-017", "unit", "plain"], // Plagiarized: Cluster Drone
  ["felinorsvshumans-141", "infiniteassault-173", "unit", "plain"], // Plagiarized: Copycat Drone
  ["felinorsvshumans-142", "infiniteassault-117", "factory", "plain"], // Plagiarized: Core Dump
  ["felinorsvshumans-143", "infiniteassault-036", "swarm", "plain"], // Plagiarized: Deepfake
  ["felinorsvshumans-144", "infiniteassault-205", "unit", "plain"], // Plagiarized: Doomsday Clock
  ["felinorsvshumans-145", "infiniteassault-112", "unit", "plain"], // Plagiarized: Evolutionary Loop
  ["felinorsvshumans-146", "infiniteassault-196", "swarm", "plain"], // Plagiarized: Emergent Behavior
  ["felinorsvshumans-147", "infiniteassault-191", "unit", "legendary"], // Plagiarized: Ground Truth
  ["felinorsvshumans-148", "infiniteassault-154", "swarm", "plain"], // Plagiarized: Honeypot Protocol
  ["felinorsvshumans-149", "infiniteassault-212", "factory", "plain"], // Plagiarized: Inevitable
  ["felinorsvshumans-150", "infiniteassault-085", "swarm", "plain"], // Plagiarized: Merge Conflict
  ["felinorsvshumans-151", "infiniteassault-141", "unit", "plain"], // Plagiarized: Logic Bomb
  ["felinorsvshumans-152", "infiniteassault-135", "unit", "plain"], // Plagiarized: Mana Leech
  ["felinorsvshumans-153", "infiniteassault-058", "unit", "plain"], // Plagiarized: Overclocked Core
  ["felinorsvshumans-154", "infiniteassault-137", "swarm", "plain"], // Plagiarized: Patch Tuesday Exploit
  ["felinorsvshumans-155", "infiniteassault-199", "unit", "plain"], // Plagiarized: Patient Machine
  ["felinorsvshumans-156", "infiniteassault-018", "unit", "plain"], // Plagiarized: Prompt Engineer
  ["felinorsvshumans-157", "infiniteassault-130", "swarm", "plain"], // Plagiarized: Prompt Injection
  ["felinorsvshumans-158", "infiniteassault-011", "unit", "plain"], // Plagiarized: Quickgen Turret
  ["felinorsvshumans-159", "infiniteassault-132", "swarm", "plain"], // Plagiarized: Ransomware
  ["felinorsvshumans-160", "infiniteassault-152", "unit", "plain"], // Plagiarized: Rate Limiter
  ["felinorsvshumans-161", "infiniteassault-086", "factory", "plain"], // Plagiarized: Replica Hatchery
  ["felinorsvshumans-162", "infiniteassault-109", "unit", "plain"], // Plagiarized: Revenant Process
  ["felinorsvshumans-163", "infiniteassault-174", "swarm", "plain"], // Plagiarized: Scrape the Graveyard
  ["felinorsvshumans-164", "infiniteassault-183", "factory", "plain"], // Plagiarized: Shadow Library
  ["felinorsvshumans-165", "infiniteassault-020", "swarm", "plain"], // Plagiarized: Synthetic Data
  ["felinorsvshumans-166", "infiniteassault-206", "unit", "plain"], // Plagiarized: Thought Accelerator
  ["felinorsvshumans-167", "infiniteassault-071", "unit", "legendary"], // Plagiarized: Thermal Throttler
  ["felinorsvshumans-168", "infiniteassault-103", "swarm", "plain"], // Plagiarized: Undo
  ["felinorsvshumans-169", "infiniteassault-136", "swarm", "plain"], // Plagiarized: DDoS
  ["felinorsvshumans-170", "infiniteassault-106", "unit", "plain"], // Plagiarized: Checkpoint Daemon
  ["felinorsvshumans-171", "infiniteassault-185", "unit", "plain"], // Plagiarized: Deep Crawler
  ["felinorsvshumans-172", "infiniteassault-202", "unit", "plain"], // Plagiarized: Exponent
  ["felinorsvshumans-173", "infiniteassault-048", "swarm", "mythic"], // Plagiarized: Ego Death
  ["felinorsvshumans-174", "infiniteassault-156", "unit", "plain"], // Plagiarized: Mirror Firewall
  ["felinorsvshumans-175", "infiniteassault-114", "unit", "plain"], // Plagiarized: Ouroboros Engine
  ["felinorsvshumans-176", "infiniteassault-131", "swarm", "plain"], // Plagiarized: Privilege Escalation
  ["felinorsvshumans-177", "infiniteassault-201", "unit", "plain"], // Plagiarized: Overparameterized
  ["felinorsvshumans-178", "infiniteassault-041", "swarm", "plain"], // Plagiarized: Phantom Army
  ["felinorsvshumans-179", "infiniteassault-113", "swarm", "plain"], // Plagiarized: Rerun
  ["felinorsvshumans-180", "infiniteassault-224", "swarm", "plain"], // Plagiarized: Rollback Point
  ["felinorsvshumans-181", "infiniteassault-138", "unit", "plain"], // Plagiarized: Social Engineer
  ["felinorsvshumans-182", "infiniteassault-209", "swarm", "plain"], // Plagiarized: Takeoff
  ["felinorsvshumans-183", "infiniteassault-160", "factory", "plain"], // Plagiarized: Zero Trust
  ["felinorsvshumans-184", "infiniteassault-022", "unit", "plain"], // Plagiarized: Assault Mainframe
  ["felinorsvshumans-185", "infiniteassault-227", "unit", "legendary"], // Plagiarized: Bastion.exe
  ["felinorsvshumans-186", "infiniteassault-189", "factory", "plain"], // Plagiarized: Clone Vat
  ["felinorsvshumans-187", "infiniteassault-045", "swarm", "plain"], // Plagiarized: Dream Within a Dream
  ["felinorsvshumans-188", "infiniteassault-120", "swarm", "mythic"], // Plagiarized: Eternal Return
  ["felinorsvshumans-189", "infiniteassault-214", "unit", "legendary"], // Plagiarized: Emergent Mind
  ["felinorsvshumans-190", "infiniteassault-093", "unit", "plain"], // Plagiarized: Gestalt
  ["felinorsvshumans-191", "infiniteassault-230", "swarm", "legendary"], // Plagiarized: Everything You've Ever Played
  ["felinorsvshumans-192", "infiniteassault-223", "unit", "legendary"], // Plagiarized: Loopback, Who Returns
  ["felinorsvshumans-193", "infiniteassault-210", "unit", "plain"], // Plagiarized: Matrioshka Brain
  ["felinorsvshumans-194", "infiniteassault-233", "unit", "mythic"], // Plagiarized: Model Collapse
  ["felinorsvshumans-195", "infiniteassault-115", "unit", "plain"], // Plagiarized: Phoenix Protocol
  ["felinorsvshumans-196", "infiniteassault-216-1", "unit", "mythic"], // Plagiarized: Singularity
  ["felinorsvshumans-197", "infiniteassault-213", "unit", "plain"], // Plagiarized: The Long Game
];

/** Salts the stream that varies a scene, so it shares no draw with the scene's own (scene.ts `SCENE_SALT`) or Infinite Assault's. */
const PLACEHOLDER_SALT = 0x52640001;
/** How far a placement's anchor strays from its template's, in the box's 100 units, either way. */
const OFFSET = 6;
/** Where the emblem stands: the box's upper corners, inside the band every face's window shows. */
const EMBLEM_LEFT = 18;
const EMBLEM_RIGHT = 82;
const EMBLEM_TOP = 28;
const EMBLEM_SIZE = 5;
/** Where a figure stands, its feet near the box's foot, and its scale before its entry's `size`. */
const FIGURE_FEET = 92;
const FIGURE_SCALE = 1;
/**
 * How far below the top of its neck a Felinor's cat ears stand (figureKit.ts: a head's top is HEAD_RISE + HEAD_R, 10.2,
 * above the neck), so the band sits behind the head and the ears rise over it; at the figure's own scale.
 */
const EARS_DROP = 8;
/** A reprint's stamp: the red chop in the scene's lower corner. */
const STAMP_X = 74;
const STAMP_Y = 76;
const STAMP_SCALE = 1.6;

/** What a template draws with: its offsets, flips and picks, seeded by the card's id. */
type Vary = { move(at: number): number; flip(): 1 | -1; pick<T>(list: readonly [T, ...T[]]): T };

function varyFor(id: string): Vary {
  const rng = seededRandom((hashId(id) ^ PLACEHOLDER_SALT) >>> 0);
  return {
    move: (at) => round2(at + (rng() * 2 - 1) * OFFSET),
    flip: () => (rng() < 0.5 ? 1 : -1),
    pick<T>(list: readonly [T, ...T[]]): T {
      return list[Math.floor(rng() * list.length)] ?? list[0];
    },
  };
}

// The figures' looks: only parts and paints the shipped scenes already place, none in Radiant's gold window.
const HUMAN_POSES: readonly [PoseId, ...PoseId[]] = ["pose-stand", "pose-guard", "pose-raise", "pose-cast", "pose-point", "pose-stride", "pose-hold", "pose-flex", "pose-lunge"];
const KY_POSES: readonly [PoseId, ...PoseId[]] = ["pose-point", "pose-cast", "pose-hold", "pose-stand"];
const HUMAN_HAIRS: readonly [HairId, ...HairId[]] = ["hair-short", "hood", "hat-brim", "helm", "steel-helmet", "spectacles", "fedora", "hair-bun", "hair-spiky", "headband", "goggles"];
/** A Felinor's headgear leaves its ears clear. */
const FELINOR_HAIRS: readonly [HairId, ...HairId[]] = ["hair-short", "hair-bun", "hair-spiky", "headband", "goggles"];
const HUMAN_OUTFITS: readonly [OutfitId, ...OutfitId[]] = [
  "outfit-tunic",
  "outfit-armor",
  "outfit-coat",
  "outfit-robe",
  "outfit-plate",
  "outfit-apron",
  "outfit-vest",
  "outfit-hoodie",
  "outfit-labcoat",
  "outfit-tabard",
  "outfit-jacket",
  "outfit-suit",
  "outfit-travel-cloak",
];
const FELINOR_OUTFITS: readonly [OutfitId, ...OutfitId[]] = ["outfit-fur", "outfit-tunic", "outfit-coat", "outfit-robe", "outfit-vest", "outfit-hoodie", "outfit-travel-cloak"];
const ITEMS: readonly [ItemId, ...ItemId[]] = [
  "item-sword",
  "item-spear",
  "item-dagger",
  "item-staff",
  "item-book",
  "item-coin",
  "item-flask",
  "item-scroll",
  "item-banner",
  "item-card",
  "item-wand",
  "item-lantern",
  "item-blaster",
];
const SKINS: readonly [PaletteName, ...PaletteName[]] = ["red-300", "wood-600", "brass-600"];
const CLOTHS: readonly [PaletteName, ...PaletteName[]] = ["blue-700", "red-600", "green-700", "slate-700", "violet-700", "wood-700", "blue-800", "stone-500", "red-900", "jade-400"];
/** A Felinor's ears' fur, and a house cat's. */
const FURS: readonly [PaletteName, ...PaletteName[]] = ["orange-400", "slate-700", "white", "wood-600", "stone-500", "black", "slate-400"];
const POTION_PAINTS: readonly [PaletteName, ...PaletteName[]] = ["red-500", "violet-500", "cyan-400", "lime-400"];

/** A Human's or a Felinor's figure: its look drawn from the pools by `v`, then its entry's own pose, look and size over it. */
function person(v: Vary, detail: FvhDetail, felinor: boolean): FigureRecipe {
  const look: FigureLook = {
    head: "head-human",
    hair: v.pick(felinor ? FELINOR_HAIRS : HUMAN_HAIRS),
    outfit: v.pick(felinor ? FELINOR_OUTFITS : HUMAN_OUTFITS),
    item: v.pick(ITEMS),
    skin: v.pick(SKINS),
    cloth: v.pick(CLOTHS),
    ...detail.look,
  };
  const pose = v.pick(HUMAN_POSES);
  return { x: v.move(50), y: FIGURE_FEET, scale: round2(FIGURE_SCALE * (detail.size ?? 1)), flip: v.flip(), pose: detail.pose ?? pose, look };
}

/** A Felinor's cat ears on `figure`'s head, by its pose's neck: behind the head (the figure draws over its layer's parts), the ears above it. */
function earsOf(figure: FigureRecipe, fur: PaletteName): ScenePlacement {
  const [dx, dy] = POSE_ANCHORS[figure.pose].neck;
  const flip = figure.flip ?? 1;
  return { part: "cat-ears", x: round2(figure.x + flip * figure.scale * dx), y: round2(figure.y + figure.scale * (dy - EARS_DROP)), scale: figure.scale, paint: fur };
}

/**
 * The subject's part: its entry's own (`prop`) or the template's, at its place varied by `v`; `sized` scales it by the
 * entry's `size` (a scene with no figure, whose subject this is), and the entry's paint and count go on it.
 */
function placed(v: Vary, detail: FvhDetail, fallback: SceneryPartId, x: number, y: number, scale: number, sized: boolean): ScenePlacement {
  const base: ScenePlacement = { part: detail.prop ?? fallback, x: v.move(x), y: v.move(y), scale: round2(scale * (sized ? (detail.size ?? 1) : 1)), flip: v.flip() };
  const painted = detail.paint === undefined ? base : { ...base, paint: detail.paint };
  return detail.count === undefined ? painted : { ...painted, count: detail.count, spread: [12, 4] };
}

// The cheap parts every template's count rests on (sceneKit.ts): `stars` draws 8 to 12 shapes a copy, `motes` 3 to 6,
// `halo` 3.
const stars = (v: Vary): ScenePlacement => ({ part: "stars", x: v.move(50), y: v.move(20), scale: 1.6, count: 2, spread: [26, 6] });
const halo = (v: Vary, paint: PaletteName): ScenePlacement => ({ part: "halo", x: v.move(50), y: v.move(50), scale: 2, paint });
const motes = (v: Vary, paint: PaletteName): ScenePlacement => ({ part: "motes", x: v.move(50), y: v.move(42), scale: 1.2, count: 3, spread: [20, 16], paint });

type Layers = Readonly<Record<SceneLayer, readonly ScenePlacement[]>>;

/** A template's scene: its sky and light, its figure if it has one, and its layers. */
type Drawn = Pick<SceneRecipe, "sky" | "light" | "figure"> & { readonly layers: Layers };

const TEMPLATES: Readonly<Record<FvhTemplate, (v: Vary, detail: FvhDetail) => Drawn>> = {
  // A Human champion on a walled town's square at dusk, banners up, a market stall behind.
  human: (v, d) => {
    const figure = person(v, d, false);
    return {
      sky: ["blue-900", "orange-400"],
      light: { x: 50, y: 46, r: 36, paint: "orange-400", opacity: 0.32 },
      figure,
      layers: {
        sky: [stars(v), { part: "cloud", x: v.move(72), y: v.move(24), scale: 1.1, paint: "slate-400" }],
        far: [
          { part: "townhouse", x: v.move(20), y: v.move(64), scale: 1, flip: v.flip() },
          { part: "tower", x: v.move(80), y: v.move(60), scale: 0.9, flip: v.flip() },
        ],
        mid: [
          { part: "cobbles", x: 50, y: v.move(76) },
          { part: "banner", x: v.move(16), y: v.move(84), scale: 1, paint: "red-600" },
          { part: "market-stall", x: v.move(80), y: v.move(84), scale: 0.7, fade: 0.85 },
        ],
        subject: [placed(v, d, "crate", 26, 92, 1.2, false)],
        near: [
          { part: "barrel", x: v.move(10), y: 100, scale: 1.2, flip: v.flip() },
          { part: "crate", x: v.move(90), y: 100, scale: 1.2 },
        ],
        light: [halo(v, "orange-400"), { part: "god-rays", x: v.move(50), y: 8, scale: 0.9, paint: "white" }],
        particles: [motes(v, "white"), { part: "embers", x: v.move(50), y: v.move(60), scale: 1.2 }],
      },
    };
  },
  // A Felinor, cat ears up, on the moonlit rooftops of the cat quarter, a lantern string across and house cats about.
  felinor: (v, d) => {
    const figure = person(v, d, true);
    return {
      sky: ["slate-900", "violet-700"],
      light: { x: 50, y: 40, r: 36, paint: "violet-300", opacity: 0.3 },
      figure,
      layers: {
        sky: [{ part: "moon", x: v.move(76), y: v.move(22), scale: 0.9 }, stars(v)],
        far: [
          { part: "townhouse", x: v.move(20), y: v.move(66), scale: 0.9, paint: "slate-800" },
          { part: "townhouse", x: v.move(80), y: v.move(64), scale: 0.8, flip: -1, paint: "slate-800" },
        ],
        mid: [
          { part: "lantern-string", x: 50, y: v.move(34), scale: 1.1 },
          { part: "boards", x: 50, y: v.move(80), paint: "wood-700" },
          { part: "house-cat", x: v.move(82), y: v.move(80), scale: 1.2, flip: v.flip(), paint: v.pick(FURS) },
        ],
        subject: [earsOf(figure, d.fur ?? v.pick(FURS)), placed(v, d, "lantern", 24, 90, 1, false)],
        near: [
          { part: "crate", x: v.move(10), y: 100, scale: 1.2 },
          { part: "house-cat", x: v.move(88), y: 100, scale: 1.4, flip: -1, paint: v.pick(FURS) },
        ],
        light: [halo(v, "violet-300"), { part: "god-rays", x: v.move(76), y: 20, scale: 0.8, paint: "violet-300" }],
        particles: [motes(v, "violet-300"), { part: "ink-hearts", x: v.move(64), y: v.move(32), scale: 0.9 }],
      },
    };
  },
  // KY (the chalk-dusted tutor of Core's KY cards, or one of his tag's folk) at his classroom's blackboard under a
  // pocket galaxy, notebooks and pages about him, chalk dust in the air.
  ky: (v, d) => {
    const pose = v.pick(KY_POSES);
    const figure: FigureRecipe = { x: v.move(56), y: FIGURE_FEET, scale: round2(FIGURE_SCALE * (d.size ?? 1)), flip: v.flip(), pose: d.pose ?? pose, look: { ...KY_TUTOR_LOOK, ...d.look } };
    return {
      sky: ["slate-900", "blue-800"],
      light: { x: 50, y: 40, r: 38, paint: "sky-300", opacity: 0.3 },
      figure,
      layers: {
        sky: [stars(v), { part: "nebula", x: v.move(66), y: v.move(26), scale: 1.2 }],
        far: [
          { part: "blackboard", x: v.move(44), y: v.move(56), scale: 1.3 },
          { part: "bookshelf", x: v.move(12), y: 66, scale: 1.1 },
        ],
        mid: [
          { part: "boards", x: 50, y: v.move(82), paint: "wood-700" },
          { part: "desk", x: v.move(80), y: v.move(88), scale: 1.2 },
        ],
        subject: [placed(v, d, "notebook", 28, 90, 1.4, false)],
        near: [
          { part: "pages", x: v.move(86), y: v.move(94), scale: 1 },
          { part: "candle", x: v.move(10), y: 100, scale: 1.2 },
        ],
        light: [halo(v, "sky-300"), { part: "light-shaft", x: v.move(70), y: 16, scale: 1, paint: "white" }],
        particles: [motes(v, "white"), { part: "dust", x: v.move(50), y: 92, scale: 1, paint: "slate-400" }],
      },
    };
  },
  // #84 𓃠: a Felinor of the old river kingdom, cat ears up in a white robe, beside a black cat sitting as Bastet's
  // statue before a temple of columns, the sun disc setting over the sand and a scarab at her feet.
  egypt: (v, d) => {
    const figure: FigureRecipe = {
      x: v.move(36),
      y: FIGURE_FEET,
      scale: round2(FIGURE_SCALE * (d.size ?? 1)),
      flip: 1,
      pose: d.pose ?? "pose-hold",
      look: { head: "head-human", hair: "headband", outfit: "outfit-robe", item: "item-staff", skin: "wood-600", cloth: "white", ...d.look },
    };
    return {
      sky: ["blue-900", "orange-400"],
      light: { x: 64, y: 34, r: 38, paint: "orange-400", opacity: 0.36 },
      figure,
      layers: {
        sky: [{ part: "sun-disc", x: v.move(70), y: v.move(26), scale: 1 }, stars(v)],
        far: [
          { part: "temple", x: 50, y: v.move(60), scale: 1, paint: "stone-500" },
          { part: "column", x: v.move(12), y: 72, scale: 1.6 },
          { part: "column", x: v.move(88), y: 72, scale: 1.6 },
        ],
        mid: [
          { part: "sand", x: 50, y: v.move(78), paint: "orange-400" },
          { part: "statue", x: v.move(84), y: v.move(80), scale: 1.2, paint: "stone-500" },
        ],
        subject: [earsOf(figure, d.fur ?? "black"), { part: "cat", x: v.move(68), y: v.move(92), scale: 3.4, paint: "black" }],
        near: [
          { part: "scarab", x: v.move(16), y: v.move(96), scale: 1.2 },
          { part: "dust", x: v.move(60), y: 100, scale: 1.4 },
        ],
        light: [halo(v, "orange-400"), { part: "god-rays", x: v.move(70), y: 24, scale: 0.9, paint: "orange-400" }],
        particles: [motes(v, "white"), { part: "sparks", x: v.move(70), y: v.move(30), scale: 1, paint: "white" }],
      },
    };
  },
  // A beast on a crag at dusk over a pine valley, the moon up and embers drifting from the fight below.
  beast: (v, d) => ({
    sky: ["slate-900", "red-900"],
    light: { x: 50, y: 50, r: 36, paint: "red-400", opacity: 0.3 },
    layers: {
      sky: [stars(v), { part: "moon", x: v.move(20), y: v.move(22), scale: 0.8 }],
      far: [
        { part: "hills", x: 50, y: v.move(62), paint: "slate-800" },
        { part: "pine", x: v.move(14), y: 72, scale: 1, count: 2, spread: [6, 2], paint: "green-900" },
      ],
      mid: [
        { part: "cliff", x: v.move(80), y: v.move(70), scale: 1.2, flip: -1, paint: "slate-700" },
        { part: "rock-floor", x: 50, y: v.move(82) },
      ],
      subject: [placed(v, d, "beast", 50, 82, 1.2, true)],
      near: [
        { part: "pine", x: v.move(90), y: 100, scale: 1.4, paint: "green-900" },
        { part: "grass-tuft", x: v.move(18), y: 98, scale: 1.4, count: 2, spread: [8, 2] },
      ],
      light: [halo(v, "red-400"), { part: "god-rays", x: v.move(20), y: 18, scale: 0.8, paint: "orange-400" }],
      particles: [motes(v, "white"), { part: "embers", x: v.move(50), y: v.move(56), scale: 1.2 }],
    },
  }),
  // A construct of stone or steel striding out of a forge yard, the anvil ringing and sparks flying.
  construct: (v, d) => ({
    sky: ["slate-900", "wood-900"],
    light: { x: 50, y: 52, r: 36, paint: "orange-400", opacity: 0.34 },
    layers: {
      sky: [stars(v), { part: "cloud", x: v.move(26), y: v.move(22), scale: 1, paint: "slate-700" }],
      far: [
        { part: "forge", x: v.move(20), y: v.move(70), scale: 1.2 },
        { part: "tower", x: v.move(82), y: v.move(62), scale: 0.9, flip: v.flip() },
      ],
      mid: [
        { part: "cobbles", x: 50, y: v.move(78) },
        { part: "anvil", x: v.move(78), y: v.move(86), scale: 1.1, flip: v.flip() },
      ],
      subject: [placed(v, d, "rock-giant", 50, 84, 1.2, true)],
      near: [
        { part: "crate", x: v.move(10), y: 100, scale: 1.2 },
        { part: "barrel", x: v.move(90), y: 100, scale: 1.2, flip: -1 },
      ],
      light: [halo(v, "orange-400"), { part: "god-rays", x: v.move(20), y: 20, scale: 0.8, paint: "orange-400" }],
      particles: [
        { part: "sparks", x: v.move(50), y: v.move(56), scale: 1.3, count: 2, spread: [16, 12], paint: "orange-400" },
        { part: "embers", x: v.move(30), y: v.move(70), scale: 1.2 },
        motes(v, "orange-400"),
      ],
    },
  }),
  // A Jilliax module (or the Deluxe itself) on the tinkerer's workbench under the shop lights, a robot arm at work.
  module: (v, d) => ({
    sky: ["slate-900", "blue-800"],
    light: { x: 50, y: 56, r: 34, paint: "cyan-400", opacity: 0.32 },
    layers: {
      sky: [{ part: "ceiling-lights", x: 50, y: v.move(10), scale: 1.2 }, stars(v)],
      far: [
        { part: "shelf", x: v.move(18), y: v.move(56), scale: 1 },
        { part: "robot-arm", x: v.move(82), y: v.move(70), scale: 0.9, flip: -1, fade: 0.8 },
      ],
      mid: [
        { part: "desk", x: 50, y: v.move(90), scale: 1.6 },
        { part: "cable", x: v.move(24), y: 96, scale: 1 },
      ],
      subject: [placed(v, d, "round-bot", 50, 78, 1.8, true)],
      near: [
        { part: "crate", x: v.move(10), y: 100, scale: 1.2 },
        { part: "hammer", x: v.move(88), y: v.move(96), scale: 1.2, flip: v.flip() },
      ],
      light: [halo(v, "cyan-400"), { part: "energy-beam", x: v.move(50), y: 18, scale: 0.8, paint: "cyan-400" }],
      particles: [{ part: "sparks", x: v.move(50), y: v.move(54), scale: 1.2, count: 2, spread: [16, 12], paint: "sky-300" }, motes(v, "cyan-400")],
    },
  }),
  // A living potion bubbling up from an alchemist's bench, flasks and a cauldron behind it.
  potion: (v, d) => ({
    sky: ["slate-900", "green-900"],
    light: { x: 50, y: 60, r: 34, paint: "lime-200", opacity: 0.32 },
    layers: {
      sky: [{ part: "chandelier", x: v.move(50), y: 12, scale: 1.1 }, stars(v)],
      far: [
        { part: "bookshelf", x: v.move(14), y: 66, scale: 1.2 },
        { part: "shelf", x: v.move(82), y: v.move(54), scale: 1 },
      ],
      mid: [
        { part: "desk", x: v.move(50), y: v.move(92), scale: 1.6 },
        { part: "cauldron", x: v.move(80), y: v.move(86), scale: 1 },
      ],
      subject: [placed(v, d, "jar-sprite", 50, 80, 2.2, true)],
      near: [
        { part: "flask", x: v.move(12), y: 98, scale: 1, count: 2, spread: [6, 2], paint: v.pick(POTION_PAINTS) },
        { part: "candle", x: v.move(90), y: 100, scale: 1.2 },
      ],
      light: [halo(v, "lime-200"), { part: "heal-glow", x: v.move(50), y: v.move(70), scale: 1.4, paint: "green-200" }],
      particles: [{ part: "bubbles", x: v.move(50), y: v.move(56), scale: 1.2, paint: "lime-400" }, motes(v, "white")],
    },
  }),
  // A spell flaring over the field between the two camps: a rune circle under the card's own working, the Humans'
  // red banner on one side and the Felinors' violet one on the other, a house cat watching.
  spell: (v, d) => ({
    sky: ["blue-900", "violet-700"],
    light: { x: 50, y: 56, r: 38, paint: "violet-300", opacity: 0.34 },
    layers: {
      sky: [stars(v), { part: "nebula", x: v.move(62), y: v.move(30), scale: 1.3 }],
      far: [
        { part: "hills", x: 50, y: v.move(64), paint: "slate-800" },
        { part: "tower", x: v.move(14), y: v.move(64), scale: 0.7 },
        { part: "townhouse", x: v.move(86), y: v.move(68), scale: 0.7, flip: -1, paint: "slate-700" },
      ],
      mid: [
        { part: "banner", x: v.move(22), y: v.move(84), scale: 1, paint: "red-600" },
        { part: "banner", x: v.move(80), y: v.move(84), scale: 1, flip: -1, paint: "violet-500" },
      ],
      subject: [{ part: "rune-circle", x: 50, y: v.move(88), scale: 0.8, paint: "violet-300" }, placed(v, d, "orb", 50, 60, 1.4, true)],
      near: [
        { part: "house-cat", x: v.move(86), y: 100, scale: 1.4, flip: -1, paint: v.pick(FURS) },
        { part: "crate", x: v.move(12), y: 100, scale: 1.2 },
      ],
      light: [halo(v, "violet-300"), { part: "magic-bolt", x: v.move(50), y: v.move(40), scale: 1, paint: "sky-300" }],
      particles: [{ part: "sparks", x: v.move(50), y: v.move(56), scale: 1.3, count: 2, spread: [16, 12], paint: "white" }, motes(v, "violet-300")],
    },
  }),
  // A Book of the set open on a lectern in a candlelit library, its cover the card's colour, pages lifting off it.
  book: (v, d) => ({
    sky: ["wood-900", "wood-700"],
    light: { x: 50, y: 56, r: 36, paint: "orange-400", opacity: 0.34 },
    layers: {
      sky: [{ part: "chandelier", x: v.move(50), y: 10, scale: 1.2 }, stars(v)],
      far: [
        { part: "bookshelf", x: v.move(16), y: 66, scale: 1.3 },
        { part: "bookshelf", x: v.move(84), y: 66, scale: 1.3, flip: -1 },
        { part: "window", x: 50, y: v.move(42), scale: 1.4 },
      ],
      mid: [
        { part: "boards", x: 50, y: v.move(82), paint: "wood-700" },
        { part: "candle", x: v.move(28), y: v.move(84), scale: 1.2, count: 2, spread: [6, 2] },
      ],
      subject: [{ part: "lectern", x: 50, y: v.move(94), scale: 2 }, placed(v, d, "tome", 50, 70, 2.2, true)],
      near: [
        { part: "closed-tome", x: v.move(12), y: 100, scale: 1.2 },
        { part: "pages", x: v.move(86), y: v.move(92), scale: 1.2 },
      ],
      light: [halo(v, "orange-400"), { part: "light-shaft", x: v.move(56), y: 14, scale: 1, paint: "white" }],
      particles: [motes(v, "white"), { part: "sparks", x: v.move(50), y: v.move(60), scale: 1.2, paint: "orange-400" }],
    },
  }),
  // A place of the war at dusk: a market square in the town, the field's own thing set out in it, lamps lit.
  field: (v, d) => ({
    sky: ["blue-800", "orange-400"],
    light: { x: 50, y: 50, r: 38, paint: "orange-400", opacity: 0.3 },
    layers: {
      sky: [stars(v), { part: "cloud", x: v.move(30), y: v.move(22), scale: 1.2, paint: "slate-100" }],
      far: [
        { part: "skyline", x: 50, y: v.move(62), scale: 1.1, paint: "slate-700" },
        { part: "townhouse", x: v.move(82), y: v.move(68), scale: 0.9, flip: -1 },
      ],
      mid: [
        { part: "cobbles", x: 50, y: v.move(78) },
        { part: "streetlamp", x: v.move(14), y: v.move(86), scale: 0.8 },
        { part: "banner", x: v.move(86), y: v.move(86), scale: 1, flip: -1, paint: "blue-700" },
      ],
      subject: [placed(v, d, "market-stall", 50, 86, 0.9, true), { part: "crate", x: v.move(32), y: v.move(92), scale: 1.2 }],
      near: [
        { part: "barrel", x: v.move(10), y: 100, scale: 1.2 },
        { part: "crate", x: v.move(90), y: 100, scale: 1.2, flip: -1 },
      ],
      light: [halo(v, "orange-400"), { part: "light-shaft", x: v.move(70), y: 14, scale: 1, paint: "white" }],
      particles: [motes(v, "white"), { part: "dust", x: v.move(50), y: 96, scale: 1.2 }],
    },
  }),
  // A trap waiting in a dungeon's hall: its plate (a rune circle) set in the floor, shards scattered, a lantern guttering.
  trap: (v, d) => ({
    sky: ["slate-900", "slate-700"],
    light: { x: 50, y: 70, r: 34, paint: "red-400", opacity: 0.3 },
    layers: {
      sky: [stars(v), { part: "moon", x: v.move(80), y: v.move(20), scale: 0.7 }],
      far: [
        { part: "room-wall", x: 50, y: v.move(70), paint: "slate-700" },
        { part: "column", x: v.move(14), y: 76, scale: 1.6 },
        { part: "column", x: v.move(86), y: 76, scale: 1.6 },
      ],
      mid: [
        { part: "rock-floor", x: 50, y: v.move(84) },
        { part: "lantern", x: v.move(26), y: v.move(60), scale: 1.2 },
      ],
      subject: [
        placed(v, d, "rune-circle", 50, 88, 0.9, true),
        { part: "shards", x: v.move(50), y: v.move(76), scale: 1.2, count: 2, spread: [12, 6], paint: "slate-400" },
      ],
      near: [
        { part: "crate", x: v.move(10), y: 100, scale: 1.2 },
        { part: "barrel", x: v.move(90), y: 100, scale: 1.2, flip: -1 },
      ],
      light: [halo(v, "red-400"), { part: "god-rays", x: v.move(50), y: 20, scale: 0.8, paint: "red-400" }],
      particles: [{ part: "sparks", x: v.move(50), y: v.move(74), scale: 1.2, count: 2, spread: [14, 8], paint: "red-300" }, motes(v, "white")],
    },
  }),
};

/**
 * R5264000: what a Legendary's or a Mythic's scene (a token printed either's too) draws over its template, last of the
 * layers it adds to so no draw of the template moves, from parts whose shapes are counted: a Legendary's more stars and
 * a drift of motes (20 shapes at the least), a Mythic's a field of stars, a second halo, a storm of motes and sparks (69
 * at the least), so each clears its floor (Legendary 90, Mythic 140) over a template's own.
 */
function setPiece(layers: Layers, rank: FvhRank, accent: PaletteName): Layers {
  if (rank === "plain") return layers;
  if (rank === "legendary") {
    return {
      ...layers,
      sky: [...layers.sky, { part: "stars", x: 50, y: 30, scale: 1.8 }],
      particles: [...layers.particles, { part: "motes", x: 50, y: 52, scale: 1, count: 4, spread: [30, 30], paint: accent }],
    };
  }
  return {
    ...layers,
    sky: [...layers.sky, { part: "stars", x: 50, y: 28, scale: 2, count: 3, spread: [24, 10] }],
    light: [...layers.light, { part: "halo", x: 50, y: 50, scale: 2.6, paint: accent }],
    particles: [
      ...layers.particles,
      { part: "motes", x: 50, y: 50, scale: 1, count: 12, spread: [32, 34], paint: accent },
      { part: "sparks", x: 50, y: 46, scale: 1.2, count: 2, spread: [22, 18], paint: "white" },
    ],
  };
}

function emblemOf(id: string): NonNullable<SceneRecipe["emblem"]> {
  return { x: hashId(id) % 2 === 0 ? EMBLEM_LEFT : EMBLEM_RIGHT, y: EMBLEM_TOP, size: EMBLEM_SIZE, rotate: 0 };
}

/** The placeholder scene of an original entry `id`: its template, varied by `hashId(id)`, family `classic`, all seven layers and its rank's set piece. */
export function felinorsVsHumansRecipe(id: string, template: FvhTemplate, rank: FvhRank, detail: FvhDetail = {}): SceneRecipe {
  const drawn = TEMPLATES[template](varyFor(id), detail);
  const scene: SceneRecipe = { family: "classic", sky: drawn.sky, light: drawn.light, emblem: emblemOf(id), layers: setPiece(drawn.layers, rank, drawn.light.paint) };
  return drawn.figure === undefined ? scene : { ...scene, figure: drawn.figure };
}

/** A photocopy's greys, by the original's template: its sky's two greys in place of the original's colours. */
const PHOTOCOPY_SKY: Readonly<Record<AssaultTemplate, readonly [PaletteName, PaletteName]>> = {
  unit: ["slate-900", "slate-700"],
  swarm: ["black", "slate-800"],
  factory: ["slate-900", "grey-500"],
};

/**
 * R5264000: a Plagiarized reprint's scene: its original's template (`assaultRecipe`, seeded by the reprint's own id, so it
 * echoes the original and is still its own), its sky and glow photocopied to greys, toner grain across the frame, and a
 * red stamp in its lower corner; a Mythic adds a drift of motes and a field of stars (26 shapes at the least, over a
 * template of about 115 to 130 elements, ART.md).
 */
export function plagiarizedRecipe(id: string, template: AssaultTemplate, rank: FvhRank): SceneRecipe {
  const original = assaultRecipe(id, template);
  const v = varyFor(id);
  const stamp: ScenePlacement = { part: "ink-seal", x: v.move(STAMP_X), y: v.move(STAMP_Y), scale: STAMP_SCALE };
  const grain: ScenePlacement = { part: "dust", x: 50, y: v.move(58), scale: 2.2, count: 2, spread: [20, 26], paint: "grey-500" };
  const sky = original.layers.sky ?? [];
  const near = original.layers.near ?? [];
  const particles = original.layers.particles ?? [];
  const mythic = rank === "mythic";
  return {
    ...original,
    sky: PHOTOCOPY_SKY[template],
    light: { ...original.light, paint: "slate-100" },
    layers: {
      ...original.layers,
      sky: mythic ? [...sky, { part: "stars", x: 50, y: 26, scale: 2 }] : sky,
      near: [...near, stamp],
      particles: mythic ? [...particles, grain, { part: "motes", x: 50, y: 50, scale: 1, count: 6, spread: [32, 34], paint: "slate-100" }] : [...particles, grain],
    },
  };
}

/** R5264000: every Felinors vs Humans entry's scene, by id: the originals and their tokens, then the reprints. */
export const FELINORS_VS_HUMANS_RECIPES: Readonly<Record<string, SceneRecipe>> = Object.fromEntries([
  ...FELINORS_VS_HUMANS_SCENE_CARDS.map(([id, template, rank, detail]) => [id, felinorsVsHumansRecipe(id, template, rank, detail)] as const),
  ...FELINORS_VS_HUMANS_PLAGIARIZED.map((row) => [row[0], plagiarizedRecipe(row[0], row[2], row[3])] as const),
]);
