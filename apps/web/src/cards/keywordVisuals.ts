// Keyword visuals on the board (issue #40, R438): what every keyword a unit has looks like on its
// board minion, beside the two-letter chip that names it.
//
// `KEYWORD_VISUALS` is a total map over `KeywordKind`, so a new keyword kind does not compile until
// it has a treatment here and its drawing in KeywordFx.tsx and keywords.css. A treatment sits on one
// of four layers of the minion, each a plane of its own, so several compose without covering each
// other or the attack, health and name:
//
//   frame  around or behind the portrait: Taunt's steel shield, Divine Shield's bubble, the Reborn
//          echo, the Immune to Spells ward, the Indestructible plating, the Stack's card edges.
//   veil   over the portrait's art, clipped to its oval and under the name plate and the gems:
//          chains, cracks, venom, veins, speed streaks, runes and clockwork.
//   glyph  a small shaped emblem in the row along the top edge, after the cost gem: blade, spear,
//          crescent, hoofprint, lock, clover.
//   stat   Armor's steel plate beside the health gem (a number of its own).
//
// Each layer holds at most so many treatments (`LAYER_CAP`), taken in `priority` order (1 first):
// the keywords that decide what an opponent can do (Taunt, Divine Shield, Can't attack, Immune to
// Spells, Poisonous) first, flavour last. A keyword past its layer's cap still has its chip (the
// chips' own "+n" fold and the hover preview list every keyword), so nothing is lost, only not drawn
// twice. And at most `AMBIENT_MAX` treatments on one unit move at a time, again by priority; the rest
// hold still. Charge's and Rush's streaks move only while the view's `canAct` is true, as a mark of
// the view's state and never as a claim about legality (that is `legalActions`', CLAUDE.md rule 7).
//
// The treatments read the view and nothing else: the unit's keywords as the view lists them (a
// Vanilla unit's list already leaves out what it lost, and a Vanilla unit shows nothing else),
// `canAct`, `armor` (the plate's number), `brittle` (the Brittle count as it stands; the keyword's
// number where the view has none) and `animated` (a backrow card standing in a unit zone as a Unit,
// R383). Under reduced motion, the OS setting or the settings panel's, every loop stops and the
// still mark stays (keywords.css).

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
  /** The shape it reads by, in words (it never rests on colour alone). */
  shape: string;
  /** The loop, or `null` for a mark that holds still (Taunt's shield, the Divine Shield bubble, Armor's plate). */
  motion: KeywordMotion | null;
  /** Whether the treatment prints the keyword's number. */
  numbered: boolean;
};

/** How many treatments each layer draws on one unit. `stat` is Armor's plate, which is always drawn. */
export const LAYER_CAP: Readonly<Record<KeywordLayer, number>> = { frame: 4, veil: 2, glyph: 2, stat: 1 };

/** How many treatments on one unit loop at once; the rest hold their still mark. */
export const AMBIENT_MAX = 2;

/**
 * Brittle's cracks deepen as its count falls: one hairline at `BRITTLE_CRACK_STAGES` or more, and
 * one more crack for each step below, down to the full web at 1 (and 0, the moment it crumbles).
 */
export const BRITTLE_CRACK_STAGES = 3;

/** The effects layer (fx.css) draws Divine Shield's cocoon and its pulse (R200); the bubble here holds still. */
export const KEYWORD_VISUALS: Readonly<Record<KeywordKind, KeywordVisual>> = {
  Taunt: {
    layer: "frame",
    priority: 1,
    shape: "a steel heater shield standing behind the portrait, and a steel ring",
    motion: null,
    numbered: false,
  },
  "Divine Shield": {
    layer: "frame",
    priority: 2,
    shape: "a golden bubble over the portrait",
    motion: null,
    numbered: false,
  },
  Armor: {
    layer: "stat",
    priority: 3,
    shape: "a steel plate tucked against the health gem, with the Armor total",
    motion: null,
    numbered: true,
  },
  "Can't attack": {
    layer: "veil",
    priority: 4,
    shape: "two chains crossed over a greyed portrait",
    motion: { keyframes: "kw-chain-sway", when: "always" },
    numbered: false,
  },
  "Immune to Spells": {
    layer: "frame",
    priority: 5,
    shape: "a hexagonal ward around the portrait, sparks on its corners",
    motion: { keyframes: "kw-ward-shimmer", when: "always" },
    numbered: false,
  },
  Poisonous: {
    layer: "veil",
    priority: 6,
    shape: "green venom running from the portrait's top rim in drops",
    motion: { keyframes: "kw-venom-drip", when: "always" },
    numbered: false,
  },
  Charge: {
    layer: "veil",
    priority: 7,
    shape: "long speed streaks across the portrait behind a double chevron",
    motion: { keyframes: "kw-streak-rush", when: "canAct" },
    numbered: false,
  },
  Rush: {
    layer: "veil",
    priority: 8,
    shape: "three short motion dashes trailing off the portrait's right side",
    motion: { keyframes: "kw-dash-trail", when: "canAct" },
    numbered: false,
  },
  Brittle: {
    layer: "veil",
    priority: 9,
    shape: "hairline cracks, more of them as the count falls, and the count on a shard",
    motion: { keyframes: "kw-crack-flicker", when: "always" },
    numbered: true,
  },
  Lifesteal: {
    layer: "veil",
    priority: 10,
    shape: "crimson veins reaching in from the rim",
    motion: { keyframes: "kw-vein-pulse", when: "always" },
    numbered: false,
  },
  Reborn: {
    layer: "frame",
    priority: 11,
    shape: "a ghostly echo of the portrait's ring rising above it",
    motion: { keyframes: "kw-echo-rise", when: "always" },
    numbered: false,
  },
  Indestructible: {
    layer: "frame",
    priority: 12,
    shape: "a ring of stone blocks bolted round the portrait, runes between them",
    motion: { keyframes: "kw-rune-glow", when: "always" },
    numbered: false,
  },
  "First Strike": {
    layer: "glyph",
    priority: 13,
    shape: "an upright blade with a glint at its tip",
    motion: { keyframes: "kw-glint-twinkle", when: "always" },
    numbered: false,
  },
  Trample: {
    layer: "glyph",
    priority: 14,
    shape: "a hoofprint in a shockwave ring",
    motion: { keyframes: "kw-shockwave", when: "always" },
    numbered: false,
  },
  Cleave: {
    layer: "glyph",
    priority: 15,
    shape: "a crescent arc, an axe's sweep",
    motion: { keyframes: "kw-crescent-swing", when: "always" },
    numbered: false,
  },
  Pierce: {
    layer: "glyph",
    priority: 16,
    shape: "a spear point on its shaft",
    motion: { keyframes: "kw-spear-thrust", when: "always" },
    numbered: false,
  },
  "Spell Damage": {
    layer: "veil",
    priority: 17,
    shape: "an arcane rune circle with the bonus at its centre",
    motion: { keyframes: "kw-rune-turn", when: "always" },
    numbered: true,
  },
  "Animated on your turn": {
    layer: "veil",
    priority: 18,
    shape: "a clock dial with a sweeping hand",
    motion: { keyframes: "kw-clock-sweep", when: "always" },
    numbered: false,
  },
  Animated: {
    layer: "veil",
    priority: 19,
    shape: "two brass cogs meshing at the portrait's foot",
    motion: { keyframes: "kw-cog-turn", when: "always" },
    numbered: false,
  },
  Lucky: {
    layer: "glyph",
    priority: 20,
    shape: "a four-leaf clover with its number",
    motion: { keyframes: "kw-clover-sparkle", when: "always" },
    numbered: true,
  },
  Immutable: {
    layer: "glyph",
    priority: 21,
    shape: "a padlock, shut",
    motion: { keyframes: "kw-seal-glint", when: "always" },
    numbered: false,
  },
  Stack: {
    layer: "frame",
    priority: 22,
    shape: "two card edges stepped out behind the portrait",
    motion: { keyframes: "kw-edge-shift", when: "always" },
    numbered: false,
  },
  Windfury: {
    layer: "glyph",
    priority: 23,
    shape: "two curved gusts, one above the other",
    motion: { keyframes: "kw-gust-sway", when: "always" },
    numbered: false,
  },
  Temporary: {
    layer: "glyph",
    priority: 24,
    shape: "an hourglass, its sand running out",
    motion: { keyframes: "kw-sand-fall", when: "always" },
    numbered: false,
  },
  Deft: {
    layer: "glyph",
    priority: 25,
    shape: "a swift diagonal slash",
    motion: { keyframes: "kw-deft-dart", when: "always" },
    numbered: false,
  },
};

/** One treatment a unit gets: its keyword, whether it loops now, and what it prints. */
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

/** What of a unit's view the treatments read. */
export type KeywordFxSource = Pick<UnitView, "keywords" | "canAct" | "armor"> &
  Partial<Pick<UnitView, "vanilla" | "animated" | "brittle">>;

function numberOf(kind: KeywordKind, keywords: readonly Keyword[], source: KeywordFxSource): number | undefined {
  if (!KEYWORD_VISUALS[kind].numbered) return undefined;
  // The view's own totals where it has one: Armor's sum (§10.4) and the Brittle count as it stands.
  if (kind === "Armor") return source.armor;
  if (kind === "Brittle" && source.brittle !== undefined) return source.brittle;
  // Lucky X stacks and the view keeps each entry for the sum (layers.ts); the others are one entry.
  return keywords.reduce((sum, keyword) => (keyword.kind === kind && "n" in keyword ? sum + keyword.n : sum), 0);
}

/** R385's cracks: one at BRITTLE_CRACK_STAGES or more, the full web at 1. */
export function brittleStage(count: number): number {
  return Math.min(BRITTLE_CRACK_STAGES, Math.max(1, BRITTLE_CRACK_STAGES + 1 - count));
}

/**
 * The kinds a unit's treatments come from: its keywords as the view lists them, once each; Brittle
 * for a unit the view gives a Brittle count (R385); and the Animated treatment for a backrow card the
 * view says is standing as a Unit (`animated`, R383) whose keywords do not already name it, except on
 * a Vanilla unit (R243), whose text is gone.
 */
export function treatedKinds(source: KeywordFxSource): KeywordKind[] {
  const kinds: KeywordKind[] = [];
  for (const keyword of source.keywords) if (!kinds.includes(keyword.kind)) kinds.push(keyword.kind);
  // R385: a Brittle count the view gives (one an effect gave, "Give Brittle N") is Brittle whether or
  // not the keyword list names it, a Vanilla unit's included (a Vanilla keeps a given count, §6.1).
  if (source.brittle !== undefined && !kinds.includes("Brittle")) kinds.push("Brittle");
  if (source.vanilla !== true && source.animated !== undefined && !kinds.includes("Animated") && !kinds.includes("Animated on your turn")) {
    // `home` is the lane an "Animated on your turn" card goes back to at its controller's cleanup.
    kinds.push(source.animated.home === undefined ? "Animated" : "Animated on your turn");
  }
  return kinds;
}

/**
 * The treatments one unit draws, in priority order: each layer's first `LAYER_CAP` kinds (Armor's
 * plate only while the view's `armor` is above 0, as the plate itself), the first `AMBIENT_MAX`
 * loops among them running and the rest still.
 */
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
