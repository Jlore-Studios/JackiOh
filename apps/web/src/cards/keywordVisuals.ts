// Keyword visuals on the board (issue #40, R438): what each keyword a unit has looks like on its minion, beside its
// chip. R2577000: only a keyword that can still act on the field or as it leaves is drawn there (`onField`).
// `KEYWORD_VISUALS` is total over `KeywordKind`: a new kind needs a treatment here and a drawing in KeywordFx.tsx and
// keywords.css. Four layers (frame, veil, glyph, stat) hold `LAYER_CAP` treatments each by `priority` (1 first); a keyword
// past it keeps its chip; `AMBIENT_MAX` move at once. Charge's and Rush's streaks move while the view's `canAct` is true,
// a mark of view state, never legality (CLAUDE.md rule 7). Treatments read the view alone (`animated`: a backrow card as a Unit, R383).

import type { Keyword, KeywordKind, UnitView } from "@jackioh/shared";

export type KeywordLayer = "frame" | "veil" | "glyph" | "stat";

/** A loop: the `@keyframes` in keywords.css, and when it runs. */
export type KeywordMotion = {
  keyframes: string;
  /** "always" while it is drawn and within the cap; "canAct" only while the view's `canAct` is also true. */
  when: "always" | "canAct";
};

export type KeywordVisual = {
  layer: KeywordLayer;
  /** 1 is drawn first and wins a capped slot over 2. Unique across the map. */
  priority: number;
  /** The shape it reads by, in words (it never rests on color alone). */
  shape: string;
  /** The loop, or `null` for a mark that holds still. */
  motion: KeywordMotion | null;
  numbered: boolean;
  /** R2577000: whether the keyword can still act while the card is on the field or as it leaves; if not, no treatment or chip (hover and inspect still list it). */
  onField: boolean;
};

/** `stat` is Armor's plate, which is always drawn. */
export const LAYER_CAP: Readonly<Record<KeywordLayer, number>> = { frame: 4, veil: 2, glyph: 2, stat: 1 };

/** How many treatments on one unit loop at once; the rest hold their still mark. */
export const AMBIENT_MAX = 2;

/** Brittle's cracks deepen as its count falls: one hairline at this or more, the full web at 1 (and 0). */
export const BRITTLE_CRACK_STAGES = 3;

/** #2270: the Brittle count at which a card on the field crumbles at its next tick (R385); game/Card.tsx draws it a gray aura. */
export const BRITTLE_LAST_COUNT = 1;

/** #2437: the Temporary count at which a card in a hand is discarded at its owner's next end of turn (R637, R2179001); game/Card.tsx draws it a fuchsia aura. A plain Temporary is Temporary 1. */
export const TEMPORARY_LAST_COUNT = 1;

/** The effects layer (fx.css) draws Divine Shield's cocoon and its pulse (R200); the bubble here holds still. */
export const KEYWORD_VISUALS: Readonly<Record<KeywordKind, KeywordVisual>> = {
  Taunt: {
    layer: "frame",
    priority: 1,
    shape: "a steel heater shield standing behind the portrait, and a steel ring",
    motion: null,
    numbered: false,
    onField: true,
  },
  "Divine Shield": {
    layer: "frame",
    priority: 2,
    shape: "a golden bubble over the portrait",
    motion: null,
    numbered: false,
    onField: true,
  },
  Armor: {
    layer: "stat",
    priority: 3,
    shape: "a steel plate tucked against the health gem, with the Armor total",
    motion: null,
    numbered: true,
    onField: true,
  },
  "Can't attack": {
    layer: "veil",
    priority: 4,
    shape: "two chains crossed over a greyed portrait",
    motion: { keyframes: "kw-chain-sway", when: "always" },
    numbered: false,
    onField: true,
  },
  "Immune to Spells": {
    layer: "frame",
    priority: 5,
    shape: "a hexagonal ward around the portrait, sparks on its corners",
    motion: { keyframes: "kw-ward-shimmer", when: "always" },
    numbered: false,
    onField: true,
  },
  Poisonous: {
    layer: "veil",
    priority: 6,
    shape: "green venom running from the portrait's top rim in drops",
    motion: { keyframes: "kw-venom-drip", when: "always" },
    numbered: false,
    onField: true,
  },
  Charge: {
    layer: "veil",
    priority: 7,
    shape: "long speed streaks across the portrait behind a double chevron",
    motion: { keyframes: "kw-streak-rush", when: "canAct" },
    numbered: false,
    onField: true,
  },
  Rush: {
    layer: "veil",
    priority: 8,
    shape: "three short motion dashes trailing off the portrait's right side",
    motion: { keyframes: "kw-dash-trail", when: "canAct" },
    numbered: false,
    onField: true,
  },
  Brittle: {
    layer: "veil",
    priority: 9,
    shape: "hairline cracks, more of them as the count falls, and the count on a shard",
    motion: { keyframes: "kw-crack-flicker", when: "always" },
    numbered: true,
    onField: true,
  },
  Lifesteal: {
    layer: "veil",
    priority: 10,
    shape: "crimson veins reaching in from the rim",
    motion: { keyframes: "kw-vein-pulse", when: "always" },
    numbered: false,
    onField: true,
  },
  Reborn: {
    layer: "frame",
    priority: 11,
    shape: "a ghostly echo of the portrait's ring rising above it",
    motion: { keyframes: "kw-echo-rise", when: "always" },
    numbered: false,
    onField: true,
  },
  Indestructible: {
    layer: "frame",
    priority: 12,
    shape: "a ring of stone blocks bolted round the portrait, runes between them",
    motion: { keyframes: "kw-rune-glow", when: "always" },
    numbered: false,
    onField: true,
  },
  "First Strike": {
    layer: "glyph",
    priority: 13,
    shape: "an upright blade with a glint at its tip",
    motion: { keyframes: "kw-glint-twinkle", when: "always" },
    numbered: false,
    onField: true,
  },
  Trample: {
    layer: "glyph",
    priority: 14,
    shape: "a hoofprint in a shockwave ring",
    motion: { keyframes: "kw-shockwave", when: "always" },
    numbered: false,
    onField: true,
  },
  Cleave: {
    layer: "glyph",
    priority: 15,
    shape: "a crescent arc, an axe's sweep",
    motion: { keyframes: "kw-crescent-swing", when: "always" },
    numbered: false,
    onField: true,
  },
  Pierce: {
    layer: "glyph",
    priority: 16,
    shape: "a spear point on its shaft",
    motion: { keyframes: "kw-spear-thrust", when: "always" },
    numbered: false,
    onField: true,
  },
  "Spell Damage": {
    layer: "veil",
    priority: 17,
    shape: "an arcane rune circle with the bonus at its center",
    motion: { keyframes: "kw-rune-turn", when: "always" },
    numbered: true,
    onField: true,
  },
  "Animated on your turn": {
    layer: "veil",
    priority: 18,
    shape: "a clock dial with a sweeping hand",
    motion: { keyframes: "kw-clock-sweep", when: "always" },
    numbered: false,
    onField: true,
  },
  Animated: {
    layer: "veil",
    priority: 19,
    shape: "two brass cogs meshing at the portrait's foot",
    motion: { keyframes: "kw-cog-turn", when: "always" },
    numbered: false,
    onField: true,
  },
  Lucky: {
    layer: "glyph",
    priority: 20,
    shape: "a four-leaf clover with its number",
    motion: { keyframes: "kw-clover-sparkle", when: "always" },
    numbered: true,
    onField: true,
  },
  Immutable: {
    layer: "glyph",
    priority: 21,
    shape: "a padlock, shut",
    motion: { keyframes: "kw-seal-glint", when: "always" },
    numbered: false,
    onField: true,
  },
  Stack: {
    layer: "frame",
    priority: 22,
    shape: "two card edges stepped out behind the portrait",
    motion: { keyframes: "kw-edge-shift", when: "always" },
    numbered: false,
    onField: false,
  },
  Windfury: {
    layer: "glyph",
    priority: 23,
    shape: "two curved gusts, one above the other",
    motion: { keyframes: "kw-gust-sway", when: "always" },
    numbered: false,
    onField: true,
  },
  Temporary: {
    layer: "glyph",
    priority: 24,
    shape: "an hourglass, its sand running out",
    motion: { keyframes: "kw-sand-fall", when: "always" },
    numbered: false,
    onField: false,
  },
  Deft: {
    layer: "glyph",
    priority: 25,
    shape: "a swift diagonal slash",
    motion: { keyframes: "kw-deft-dart", when: "always" },
    numbered: false,
    onField: true,
  },
  Untributable: {
    layer: "glyph",
    priority: 26,
    shape: "an anchor holding its ground",
    motion: { keyframes: "kw-untributable-anchor", when: "always" },
    numbered: false,
    onField: true,
  },
  Equip: {
    layer: "glyph",
    priority: 27,
    shape: "a horseshoe magnet, its poles pulling in",
    motion: { keyframes: "kw-magnet-pull", when: "always" },
    numbered: false,
    onField: false,
  },
  // #783's keywords (R783000, R783002, R783004, R783010, R783012).
  Ranged: {
    layer: "glyph",
    priority: 28,
    shape: "a drawn bow with an arrow nocked",
    motion: { keyframes: "kw-ranged-aim", when: "always" },
    numbered: false,
    onField: true,
  },
  Flying: {
    layer: "glyph",
    priority: 29,
    shape: "a pair of spread wings",
    motion: { keyframes: "kw-flying-hover", when: "always" },
    numbered: false,
    onField: true,
  },
  Cloaked: {
    layer: "glyph",
    priority: 30,
    shape: "a hooded cloak drawn over a face",
    motion: { keyframes: "kw-cloaked-fade", when: "always" },
    numbered: false,
    onField: false,
  },
  Crater: {
    layer: "glyph",
    priority: 31,
    shape: "a cratered bowl with rubble thrown up from its rim",
    motion: { keyframes: "kw-crater-quake", when: "always" },
    numbered: false,
    onField: false,
  },
  Flashback: {
    layer: "glyph",
    priority: 32,
    shape: "a circling arrow turning back on itself",
    motion: { keyframes: "kw-flashback-rewind", when: "always" },
    numbered: false,
    onField: false,
  },
  // R1838000: a hand keyword; a Unit that prints it carries it onto the field, where it does nothing.
  Tradable: {
    layer: "glyph",
    priority: 33,
    shape: "two arrows passing each other, one in and one out",
    motion: { keyframes: "kw-trade-swap", when: "always" },
    numbered: false,
    onField: false,
  },
  // R1852000: Stinkyscape's Starship Piece.
  "Starship Piece": {
    layer: "glyph",
    priority: 34,
    shape: "a riveted hull plate",
    motion: { keyframes: "kw-starship-piece-glint", when: "always" },
    numbered: false,
    onField: true,
  },
  // R1852003: a launched Starship.
  Starship: {
    layer: "glyph",
    priority: 35,
    shape: "a rocket nose up over its flame",
    motion: { keyframes: "kw-starship-thrust", when: "always" },
    numbered: false,
    onField: true,
  },
  // R2657000: Can't Defend, never in Defense Position, a shield split in two whose halves drift apart.
  "Can't Defend": {
    layer: "glyph",
    priority: 36,
    shape: "a shield split down the middle, its halves drifting apart",
    motion: { keyframes: "kw-cant-defend-split", when: "always" },
    numbered: false,
    onField: true,
  },
};

/** One treatment a unit gets. */
export type KeywordFxPlan = {
  kind: KeywordKind;
  visual: KeywordVisual;
  /** "on" loops, "off" holds its still mark (past `AMBIENT_MAX`, or Charge and Rush while `canAct` is false); `null` never moves. */
  motion: "on" | "off" | null;
  /** The number it prints, for a numbered keyword. */
  n?: number;
  /** Brittle only: how deep its cracks run, 1 to BRITTLE_CRACK_STAGES. */
  stage?: number;
};

export type KeywordFxSource = Pick<UnitView, "keywords" | "canAct" | "armor"> &
  Partial<Pick<UnitView, "vanilla" | "animated" | "brittle">>;

function numberOf(kind: KeywordKind, keywords: readonly Keyword[], source: KeywordFxSource): number | undefined {
  if (!KEYWORD_VISUALS[kind].numbered) return undefined;
  // The view's own totals where it has one: Armor's sum (§10.4) and the Brittle count as it stands.
  if (kind === "Armor") return source.armor;
  if (kind === "Brittle" && source.brittle !== undefined) return source.brittle;
  // Lucky X stacks and the view keeps each entry for the sum (layers.ts); the others are one entry.
  return keywords.reduce((sum, keyword) => (keyword.kind === kind && "n" in keyword ? sum + (keyword.n ?? 0) : sum), 0);
}

export function brittleStage(count: number): number {
  return Math.min(BRITTLE_CRACK_STAGES, Math.max(1, BRITTLE_CRACK_STAGES + 1 - count));
}

/**
 * #2437, R2179001: a card's Temporary X, the largest of its Temporary entries (a plain one counts
 * TEMPORARY_LAST_COUNT), as the engine's `temporary::temporary_number`; null with none.
 */
export function temporaryLeft(keywords: readonly Keyword[]): number | null {
  let most: number | null = null;
  for (const keyword of keywords) {
    if (keyword.kind !== "Temporary") continue;
    const n = keyword.n ?? TEMPORARY_LAST_COUNT;
    if (most === null || n > most) most = n;
  }
  return most;
}

/** The kinds a unit's treatments come from: its listed keywords once each, minus `onField: false`
 * (R2577000); Brittle for a given count (R385); Animated for a backrow card standing as a Unit
 * (`animated`, R383), except on a Vanilla unit (R243). */
export function treatedKinds(source: KeywordFxSource): KeywordKind[] {
  const kinds: KeywordKind[] = [];
  for (const keyword of source.keywords) {
    // R2577000: a keyword that does nothing on the field draws no treatment there.
    // R3546004: a kind with no visual (GIGA, until S26's chip) draws nothing, not a throw.
    if (KEYWORD_VISUALS[keyword.kind]?.onField === true && !kinds.includes(keyword.kind)) kinds.push(keyword.kind);
  }
  // R385: a Brittle count the view gives (one an effect gave, "Give Brittle N") is Brittle whether or
  // not the keyword list names it, a Vanilla unit's included (a Vanilla keeps a given count, §6.1).
  if (source.brittle !== undefined && !kinds.includes("Brittle")) kinds.push("Brittle");
  if (source.vanilla !== true && source.animated !== undefined && !kinds.includes("Animated") && !kinds.includes("Animated on your turn")) {
    // `home` is the lane an "Animated on your turn" card goes back to at its controller's cleanup.
    kinds.push(source.animated.home === undefined ? "Animated" : "Animated on your turn");
  }
  return kinds;
}

/** The treatments one unit draws, in priority order: each layer's first `LAYER_CAP` kinds, the first `AMBIENT_MAX` loops running. */
export function keywordFxPlan(source: KeywordFxSource): KeywordFxPlan[] {
  const kinds = treatedKinds(source)
    .filter((kind) => kind !== "Armor" || source.armor > 0)
    .sort((a, b) => KEYWORD_VISUALS[a].priority - KEYWORD_VISUALS[b].priority);

  const used: Record<KeywordLayer, number> = { frame: 0, veil: 0, glyph: 0, stat: 0 };
  let looping = 0;
  const plan: KeywordFxPlan[] = [];
  for (const kind of kinds) {
    const visual = KEYWORD_VISUALS[kind];
    if (used[visual.layer] >= LAYER_CAP[visual.layer]) continue;
    used[visual.layer] += 1;

    let motion: KeywordFxPlan["motion"] = null;
    if (visual.motion !== null) {
      const wanted = visual.motion.when === "always" || source.canAct;
      motion = wanted && looping < AMBIENT_MAX ? "on" : "off";
      if (motion === "on") looping += 1;
    }

    const entry: KeywordFxPlan = { kind, visual, motion };
    const n = numberOf(kind, source.keywords, source);
    if (n !== undefined) entry.n = n;
    if (kind === "Brittle") entry.stage = brittleStage(n ?? BRITTLE_CRACK_STAGES);
    plan.push(entry);
  }
  return plan;
}

/** The attributes every treatment carries, so tests and e2e find it by kind. */
export function keywordFxAttributes(entry: KeywordFxPlan): Record<string, string> {
  const attributes: Record<string, string> = {
    "data-keyword-fx": entry.kind,
    "data-kw-layer": entry.visual.layer,
  };
  if (entry.motion !== null) attributes["data-kw-motion"] = entry.motion;
  if (entry.n !== undefined) attributes["data-n"] = String(entry.n);
  if (entry.stage !== undefined) attributes["data-stage"] = String(entry.stage);
  return attributes;
}

/** R2789000: a callout's icon (fx/motifs/callout.ts): SVG path data in a 100×100 box, filled, or only stroked when `line` is set. */
export type KeywordIcon = { d: string; line?: true };

/** R2789000: the combat keywords' callout icons, R2791000 the tempo ones; a keyword with none draws no callout. */
export const KEYWORD_CALLOUT_ICONS: Readonly<Partial<Record<KeywordKind, KeywordIcon>>> = {
  "First Strike": { d: "M50 3 L65 22 L65 60 L35 60 L35 22 Z M20 60 H80 V72 H20 Z M43 72 H57 V96 H43 Z" },
  Cleave: { d: "M10 82 A48 48 0 0 1 90 18 A62 62 0 0 0 10 82 Z" },
  Trample: { d: "M30 34 C30 16 70 16 70 34 L70 62 C70 71 61 73 57 66 L55 46 C55 39 45 39 45 46 L43 66 C39 73 30 71 30 62 Z" },
  Pierce: { d: "M50 4 L64 34 L50 46 L36 34 Z M43 46 H57 V52 H43 Z M46 52 H54 V96 H46 Z" },
  Lifesteal: { d: "M50 90 C22 70 8 54 8 34 A21 21 0 0 1 50 24 A21 21 0 0 1 92 34 C92 54 78 70 50 90 Z" },
  Poisonous: { d: "M50 6 C60 30 78 46 78 64 A28 28 0 0 1 22 64 C22 46 40 30 50 6 Z" },
  Flying: {
    d: "M50 58 C38 36 20 28 4 30 C14 40 18 52 30 58 C22 60 16 66 12 72 C28 72 42 68 50 58 Z M50 58 C62 36 80 28 96 30 C86 40 82 52 70 58 C78 60 84 66 88 72 C72 72 58 68 50 58 Z",
  },
  Windfury: { d: "M8 36 H62 C84 36 84 12 66 14 M8 66 H70 C94 66 94 90 74 88", line: true },
  Ranged: { d: "M30 10 C66 26 66 74 30 90 M30 10 V90 M14 50 H84 M74 42 L86 50 L74 58", line: true },
  // Issue #2790: the defensive keywords' callout icons, in the same 100×100 box.
  "Divine Shield": { d: "M50 8 A42 42 0 1 0 50 92 A42 42 0 1 0 50 8 Z" },
  Armor: { d: "M28 20 H72 V58 L50 84 L28 58 Z" },
  Indestructible: { d: "M20 30 H80 V44 H20 Z M28 44 H72 V70 H28 Z M36 70 H64 V88 H36 Z" },
  Immutable: { d: "M32 44 H68 V84 H32 Z M40 44 V32 A10 10 0 0 1 60 32 V44" },
  Reborn: { d: "M20 60 L50 30 L80 60 M20 78 L50 48 L80 78", line: true },
  Brittle: { d: "M54 4 L42 34 L58 44 L40 72 L48 96", line: true },
  "Immune to Spells": { d: "M50 6 L88 28 V72 L50 94 L12 72 V28 Z" },
  Taunt: { d: "M30 14 H70 V52 C70 72 60 82 50 88 C40 82 30 72 30 52 Z" },
  "Can't Defend": { d: "M46 14 H30 V52 C30 72 38 82 44 88 Z M54 14 H70 V52 C70 72 62 82 56 88 Z" },
  "Can't attack": { d: "M50 8 A42 42 0 1 0 50 92 A42 42 0 1 0 50 8 Z M28 28 L72 72 M72 28 L28 72" },
  // R2791000: the tempo keywords' callout icons; a keyword with none draws no callout.
  Charge: { d: "M14 20 L38 50 L14 80 M38 20 L62 50 L38 80 M62 20 L86 50 L62 80", line: true },
  Rush: { d: "M20 50 H70 M55 30 L75 50 L55 70", line: true },
  Equip: { d: "M25 8 H45 V52 A5 5 0 0 0 55 52 V8 H75 V52 A25 25 0 0 1 25 52 Z" },
  Tradable: { d: "M50 8 A42 42 0 0 1 92 50 M92 50 L84 40 M92 50 L81 50 M50 92 A42 42 0 0 1 8 50 M8 50 L16 60 M8 50 L19 50", line: true },
  Stack: { d: "M10 70 H54 V82 H10 Z M24 52 H68 V64 H24 Z M38 34 H82 V46 H38 Z M52 16 H96 V28 H52 Z" },
  "Spell Damage": { d: "M38 4 H62 V38 H96 V62 H62 V96 H38 V62 H4 V38 H38 Z" },
  Lucky: { d: "M38 27 a11 11 0 1 0 0.1 0 M62 27 a11 11 0 1 0 0.1 0 M38 51 a11 11 0 1 0 0.1 0 M62 51 a11 11 0 1 0 0.1 0 M50 70 L50 94" },
  Deft: { d: "M22 74 L46 26 M78 74 L54 26 M50 84 a4 4 0 1 0 0.1 0", line: true },
  Flashback: { d: "M18 86 H82 M50 76 V24 M34 40 L50 24 L66 40", line: true },
  Temporary: { d: "M30 8 H70 L52 46 L70 92 H30 L48 46 Z" },
};
