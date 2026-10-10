// A card's motif: the small picture its name asks for (R503), read from the words of its printed
// name. Presentation only, from the public catalog's name: no rule reads it (CLAUDE.md rule 7).
//
// `motifFor` splits the name into lowercase words and walks MOTIF_WORDS in order; the first motif with
// a word that matches wins. A pattern is a whole word (its plural too), `word*` a word that starts
// so, `*word` one that ends so. The table runs from the concrete (a creature, an object) to the
// abstract (chaos, time), so "Plague Doctor" is spores rather than a cross.
//
// Drawn in procedural.ts: a card of a type theme wears the motif's glyph as its emblem, in the motif's
// own colours; a tribe or family keeps its emblem and carries the motif by its `arrangement`.

import type { EmblemGlyph } from "./emblems.ts";
import type { ArtThemeId } from "./themes.ts";

export type MotifId =
  | "flames" | "spores" | "mushroom" | "book" | "cat" | "sheep" | "wisps" | "rock" | "monkey" | "shrimp"
  | "turtle" | "lizard" | "snake" | "bat" | "web" | "grapes" | "fruit" | "pancake" | "bread" | "candy"
  | "crown" | "dice" | "clock" | "lock" | "broken-rune" | "tower" | "house" | "bomb" | "cross" | "coins"
  | "circuits" | "bones" | "storm" | "wind" | "frost" | "drum" | "notes" | "sun" | "moon" | "blood"
  | "eyes" | "portal" | "brain" | "target" | "heart" | "flag" | "halo" | "gears" | "gems" | "leaf"
  | "cycle" | "echo" | "helix" | "fangs" | "infinity" | "crate" | "blades" | "flask" | "scales" | "rise"
  | "fall" | "waves" | "mask" | "horns" | "cards" | "armor" | "jet";

export type MotifArrangement = "hero" | "scatter" | "rise" | "fall";

/** What a figure wears on its head; the three a motif can put there. */
export type MotifHeadgear = "ears" | "crown" | "horns";

export type Motif = {
  glyph: EmblemGlyph;
  arrangement: MotifArrangement;
  /** The motif's own colours, which read whatever the theme's hue. */
  fill: string;
  stroke: string;
  /** A pattern drawn behind everything as well. */
  pattern?: "circuits" | "waves";
  /** On a figure, what it wears instead of its theme's headgear. */
  headgear?: MotifHeadgear;
  /** Turns the glyph, in degrees (a falling chevron is a rising one upside down). */
  turn?: number;
};

export const MOTIFS: Readonly<Record<MotifId, Motif>> = {
  flames: { glyph: "flame", arrangement: "rise", fill: "#ffb347", stroke: "#7a1e00" },
  spores: { glyph: "spore", arrangement: "scatter", fill: "#b8f25a", stroke: "#28430c" },
  mushroom: { glyph: "mushroom", arrangement: "rise", fill: "#e0503c", stroke: "#3a0d06" },
  book: { glyph: "book", arrangement: "hero", fill: "#f3e3bf", stroke: "#3a2410" },
  cat: { glyph: "cat", arrangement: "hero", fill: "#f3c56e", stroke: "#3b1734", headgear: "ears" },
  sheep: { glyph: "sheep", arrangement: "hero", fill: "#f4f1e8", stroke: "#3a3630" },
  wisps: { glyph: "ghost", arrangement: "scatter", fill: "#dfe8ff", stroke: "#26304a" },
  rock: { glyph: "boulder", arrangement: "rise", fill: "#a7a29a", stroke: "#2b2824" },
  monkey: { glyph: "monkey", arrangement: "hero", fill: "#c08a55", stroke: "#2e1a08" },
  shrimp: { glyph: "shrimp", arrangement: "hero", fill: "#ff8a6a", stroke: "#4a1206" },
  turtle: { glyph: "turtle", arrangement: "hero", fill: "#7fbf6a", stroke: "#1b3a12" },
  lizard: { glyph: "lizard", arrangement: "hero", fill: "#8fd46a", stroke: "#1f3a0e" },
  snake: { glyph: "snake", arrangement: "hero", fill: "#7fd06a", stroke: "#173a0c" },
  bat: { glyph: "bat", arrangement: "scatter", fill: "#6a5a8c", stroke: "#120c1e" },
  web: { glyph: "web", arrangement: "hero", fill: "#eef0f4", stroke: "#20242c" },
  grapes: { glyph: "grapes", arrangement: "hero", fill: "#a466dc", stroke: "#2a0f45" },
  fruit: { glyph: "fruit", arrangement: "scatter", fill: "#e2513f", stroke: "#3d0f08" },
  pancake: { glyph: "pancakes", arrangement: "hero", fill: "#f2c27a", stroke: "#5a2c0a" },
  bread: { glyph: "bread", arrangement: "hero", fill: "#e0a95c", stroke: "#4a2a08" },
  candy: { glyph: "candy", arrangement: "scatter", fill: "#ff8ad0", stroke: "#4a0a36" },
  crown: { glyph: "crown", arrangement: "hero", fill: "#ffd24a", stroke: "#5a3a00", headgear: "crown" },
  dice: { glyph: "die", arrangement: "scatter", fill: "#f5f5f0", stroke: "#2a2a2a" },
  clock: { glyph: "clock", arrangement: "hero", fill: "#ece5cf", stroke: "#2c2618" },
  lock: { glyph: "padlock", arrangement: "hero", fill: "#e0bb4e", stroke: "#3a2a06" },
  "broken-rune": { glyph: "broken-rune", arrangement: "hero", fill: "#9fd8ff", stroke: "#0e2a44" },
  tower: { glyph: "tower", arrangement: "hero", fill: "#ddd6c6", stroke: "#2c2820" },
  house: { glyph: "house", arrangement: "hero", fill: "#e8c9a0", stroke: "#3a2410" },
  bomb: { glyph: "bomb", arrangement: "hero", fill: "#8290a6", stroke: "#12151b" },
  cross: { glyph: "cross", arrangement: "hero", fill: "#ff5a5a", stroke: "#4a0808" },
  coins: { glyph: "coin", arrangement: "scatter", fill: "#ffd24a", stroke: "#6b4a00" },
  circuits: { glyph: "chip", arrangement: "hero", fill: "#7df6ff", stroke: "#06303a", pattern: "circuits" },
  bones: { glyph: "bones", arrangement: "hero", fill: "#efe8d6", stroke: "#3a3326" },
  storm: { glyph: "bolt", arrangement: "fall", fill: "#fff27a", stroke: "#4a3a00" },
  wind: { glyph: "gust", arrangement: "scatter", fill: "#d8f2ff", stroke: "#1d3a4a" },
  frost: { glyph: "snowflake", arrangement: "scatter", fill: "#e6f7ff", stroke: "#244a66" },
  drum: { glyph: "drum", arrangement: "hero", fill: "#d99a5a", stroke: "#3a1a06" },
  notes: { glyph: "note", arrangement: "scatter", fill: "#ff9ae0", stroke: "#3a0a2e" },
  sun: { glyph: "sun", arrangement: "hero", fill: "#ffd75a", stroke: "#6a3a00" },
  moon: { glyph: "moon", arrangement: "hero", fill: "#f4f0d6", stroke: "#3a3620" },
  blood: { glyph: "drop", arrangement: "fall", fill: "#e0304a", stroke: "#3b0008" },
  eyes: { glyph: "eye", arrangement: "scatter", fill: "#f3f0ff", stroke: "#2a1a4a" },
  portal: { glyph: "portal", arrangement: "hero", fill: "#b98cff", stroke: "#241040" },
  brain: { glyph: "brain", arrangement: "hero", fill: "#f2a5b8", stroke: "#4a1022" },
  target: { glyph: "target", arrangement: "hero", fill: "#ff4d4d", stroke: "#3a0606" },
  heart: { glyph: "heart", arrangement: "scatter", fill: "#ff6b8a", stroke: "#4a0818" },
  flag: { glyph: "flag", arrangement: "hero", fill: "#ee5a44", stroke: "#3a0c06" },
  halo: { glyph: "halo", arrangement: "hero", fill: "#fff1b0", stroke: "#5a4a10" },
  gears: { glyph: "gear", arrangement: "hero", fill: "#c9ced8", stroke: "#23272f" },
  gems: { glyph: "crystal", arrangement: "scatter", fill: "#7fe3ff", stroke: "#0c3440" },
  leaf: { glyph: "leaf", arrangement: "scatter", fill: "#7fd46a", stroke: "#1d4a14" },
  cycle: { glyph: "cycle", arrangement: "hero", fill: "#8fe39a", stroke: "#123a18" },
  echo: { glyph: "echo", arrangement: "hero", fill: "#bfe6ff", stroke: "#12304a" },
  helix: { glyph: "helix", arrangement: "hero", fill: "#9ff0d0", stroke: "#0c3a2a" },
  fangs: { glyph: "jaws", arrangement: "hero", fill: "#f4efe4", stroke: "#4a0c0c" },
  infinity: { glyph: "infinity", arrangement: "hero", fill: "#c9b6ff", stroke: "#241a4a" },
  crate: { glyph: "crate", arrangement: "hero", fill: "#c99a5a", stroke: "#3a2208" },
  blades: { glyph: "blades", arrangement: "hero", fill: "#dfe6ee", stroke: "#1e242c" },
  flask: { glyph: "flask", arrangement: "hero", fill: "#9ff07a", stroke: "#1c3a0c" },
  scales: { glyph: "scales", arrangement: "hero", fill: "#e8c56a", stroke: "#3a2a06" },
  rise: { glyph: "chevrons", arrangement: "hero", fill: "#7dff9a", stroke: "#0c3a16" },
  fall: { glyph: "chevrons", arrangement: "hero", fill: "#ff7a7a", stroke: "#3a0c0c", turn: 180 },
  waves: { glyph: "bubbles", arrangement: "hero", fill: "#8fd8ff", stroke: "#0c2a44", pattern: "waves" },
  mask: { glyph: "mask", arrangement: "hero", fill: "#f4efe4", stroke: "#2a2030" },
  horns: { glyph: "horns", arrangement: "hero", fill: "#e0443a", stroke: "#3a0806", headgear: "horns" },
  cards: { glyph: "cards", arrangement: "hero", fill: "#f7f3ea", stroke: "#2a2420" },
  armor: { glyph: "shield", arrangement: "hero", fill: "#c9d6e8", stroke: "#1b2745" },
  jet: { glyph: "jet", arrangement: "hero", fill: "#c9d2dc", stroke: "#1b222c" },
};

/**
 * The words that call each motif up, in the order they are tried (first match wins). A name about
 * nothing drawable ("Guy Att", "Prep") has no motif and its theme's picture alone.
 */
export const MOTIF_WORDS: readonly (readonly [MotifId, readonly string[]])[] = [
  // Creatures and people first: the thing a card is.
  ["cat", ["felinor*", "cat", "kitten", "panther", "mrow", "lion", "tiger"]],
  ["sheep", ["sheep*", "sheeople", "lamb"]],
  ["monkey", ["monkey*", "ape", "chimp"]],
  ["shrimp", ["shrimp*", "prawn"]],
  ["turtle", ["turtle*", "turt*", "tortoise"]],
  ["lizard", ["lizard*", "gecko"]],
  ["snake", ["snake*", "serpent*", "viper", "cobra"]],
  ["bat", ["bat", "vampire*"]],
  ["web", ["joro", "spider*", "web"]],
  // Plague before anything its cards are also called (Doctor, Nuke, Goliath, Chalice, Eater).
  ["spores", ["plague*", "pestilen*", "slime", "toxin*", "toxic", "outbreak", "corpse*", "spore*", "virus", "viral", "infect*"]],
  ["fangs", ["eater", "muncher", "munch*", "devour*", "carnivor*", "hungry", "hunger", "bite*"]],
  ["flask", ["lab", "fusion", "fuse*", "experiment*", "potion*", "alchem*", "pickle"]],
  ["mushroom", ["mushroom*", "*shroom", "fung*"]],
  ["flames", ["flame*", "fire*", "burn*", "wildfire", "lance", "lava", "blaze", "inferno", "ember*", "pyro*"]],
  ["frost", ["frozen", "frost*", "ice", "icy", "snow*", "winter", "cold", "freeze"]],
  ["bones", ["bone*", "skull*", "skeleton", "grave*", "gy", "tomb*", "tumbas", "ghoul*", "death", "kill", "breaker"]],
  ["wisps", ["ghost*", "void*", "shadow*", "soul*", "spirit*", "phantom", "wraith", "curse*", "cloak*"]],
  ["rock", ["golem*", "titan*", "giant*", "goliath", "rock", "stone*", "boulder*"]],
  ["grapes", ["grape*", "vine*"]],
  ["fruit", ["fruit*", "apple*", "pear*", "papaya*", "fig", "produce", "orchard", "tree"]],
  ["pancake", ["pancake*", "waffle*", "spatula", "*spatula", "griddle", "syrup"]],
  ["bread", ["bread*", "butter*", "cookie*", "toast", "loaf"]],
  ["candy", ["jelly", "bean*", "candy", "sweet*", "treat*", "honey*", "sugar"]],
  // Objects and places.
  ["book", ["book*", "tome*", "archiv*", "tutor", "notebook*", "page*"]],
  ["crown", ["crown*", "king*", "queen*", "tyrant*", "royal*", "scepter", "emperor"]],
  ["lock", ["lock*", "*lock", "jail", "limit"]],
  ["broken-rune", ["counterspell*", "silence*", "magic", "jam*", "refus*", "suppress*", "nullify"]],
  ["tower", ["tower*", "castle", "wall*", "fort*"]],
  ["house", ["house*", "home*", "hogar", "farm*"]],
  ["bomb", ["bomb*", "boom*", "nuke*", "explos*", "blast*", "collateral"]],
  ["cross", ["heal*", "nurse*", "doctor*", "medic*", "surgery", "cure", "fauci"]],
  ["drum", ["drum*", "*drum"]],
  ["jet", ["fighter*", "jet*", "plane*"]],
  ["blades", ["blade*", "sword*", "execute", "brawl", "stab", "duel*", "shredder*", "weapon*"]],
  ["armor", ["armor*", "armour*", "shell*", "defen*", "fender", "shield*"]],
  ["flag", ["flag*", "banner*", "hurrah", "arms", "league"]],
  ["target", ["hunter", "hit", "strike", "striker", "*striker", "point*", "sniper", "aim"]],
  ["crate", ["box*", "crate*", "shipping", "dropship*", "pack", "package*", "gift*", "wraps"]],
  ["gears", ["machine*", "gear*", "mech*", "engine*", "tuning", "robot*"]],
  ["clock", ["clock*", "rewind*", "rollback", "tempo", "lag", "time*", "hourglass", "backward*", "forward"]],
  ["circuits", ["datacenter*", "ai", "hallucination*", "glitch*", "system*", "tech*", "ui", "code", "autocomplete", "assistant"]],
  ["cards", ["card*", "deck*", "draw", "ace", "combo*", "pile"]],
  ["mask", ["mask*", "trick*", "disguise"]],
  // Sky and weather.
  ["storm", ["storm*", "tesla", "thunder*", "lightning", "shock*"]],
  ["wind", ["wind", "whirlwind", "zeph*", "gust*", "tornado", "rush"]],
  ["waves", ["flood*", "deep", "sea", "ocean", "wave*", "tide*", "bubble*", "water"]],
  ["sun", ["sun*", "solar*", "dawn"]],
  ["moon", ["moon*", "lunar", "eclipse", "night*", "dream*"]],
  ["blood", ["blood*", "bleed*", "pain", "siphon"]],
  ["eyes", ["palantir", "argus*", "eye*", "watch*", "vision", "hallucinat*"]],
  ["portal", ["portal*", "exile*", "otherworld*", "nether", "rift", "banish*"]],
  ["leaf", ["nature*", "leaf", "leaves", "jungle*", "plant*", "growth", "grow*", "crop*", "thriv*", "garden", "forest", "mulch"]],
  ["infinity", ["forever*", "infini*", "ceaseless", "eternal*", "endless"]],
  ["gems", ["mana", "jewel*", "gem*", "crystal*", "resource*", "reserve*"]],
  ["coins", ["money", "tax*", "auction*", "lobbyist*", "greed*", "coin*", "dividend", "stockpile", "stimmy", "acquisition", "appropriation*", "income", "tokens"]],
  // The abstract last: what a card does rather than what it is.
  ["halo", ["divine*", "saint*", "holy", "angel*", "bless*", "aura", "favor"]],
  ["horns", ["devil*", "demon*", "fiend*", "pact", "menace"]],
  ["notes", ["pop", "song*", "music*", "sing*", "choir"]],
  ["heart", ["feel*", "love*", "heart*", "friend*", "helpful"]],
  ["brain", ["mind*", "memory", "knowledge", "brain*", "thought*", "think*", "reminisc*", "control*", "genius"]],
  ["helix", ["mutat*", "transmog*", "eugenic*", "adapt*", "gene*", "dna", "duplicat*", "clone*"]],
  ["scales", ["audit*", "justice", "punish*", "unbiased", "judge*", "trial", "test", "balance"]],
  ["rise", ["buff*", "upgrade*", "scaling", "stats", "boost*"]],
  ["fall", ["nerf*", "degrade*", "wither*", "weaken*"]],
  ["cycle", ["recycl*", "recurring", "reoccur*", "recur*", "cycle*", "loop*"]],
  ["echo", ["echo*", "ping"]],
  ["dice", ["dice", "die", "risky", "gambl*", "gambit", "chaos", "luck*", "random"]],
];

/** A name's words: lowercase runs of letters and digits ("KY's Papaya" is ky, s, papaya). */
export function nameWords(name: string): string[] {
  return name.toLowerCase().split(/[^a-z0-9]+/).filter((word) => word !== "");
}

function matches(pattern: string, word: string): boolean {
  if (pattern.endsWith("*")) return word.startsWith(pattern.slice(0, -1));
  if (pattern.startsWith("*")) return word.endsWith(pattern.slice(1));
  return word === pattern || word === `${pattern}s` || word === `${pattern}es`;
}

/**
 * The motifs a family's own picture already draws, which its cards' names therefore skip: every
 * Book card is called "Book of …", every Felinor card a Felinor, and the AI family is circuitry.
 */
export const THEME_OWN_MOTIFS: Readonly<Partial<Record<ArtThemeId, readonly MotifId[]>>> = {
  book: ["book"],
  felinor: ["cat"],
  pancake: ["pancake"],
  ai: ["circuits"],
};

/**
 * The motif a card's name calls up for a card of `theme`, or null when it names nothing drawable
 * (or there is no name to read).
 */
export function motifFor(name: string | null | undefined, theme?: ArtThemeId): MotifId | null {
  if (name === null || name === undefined) return null;
  const words = nameWords(name);
  const own = theme === undefined ? [] : (THEME_OWN_MOTIFS[theme] ?? []);
  for (const [motif, patterns] of MOTIF_WORDS) {
    if (own.includes(motif)) continue;
    if (patterns.some((pattern) => words.some((word) => matches(pattern, word)))) return motif;
  }
  return null;
}
