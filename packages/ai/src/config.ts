// Every number the AI states (CLAUDE.md rule 9, in the package's own config). The weights are tuning
// defaults: tests pin rankings and puzzle outcomes, never these values.

import type { EmoteId } from "@jackioh/shared";
import type { SearchBudget } from "./types";

/** The practice AI's budget (SPEC §9.9): the same at every difficulty (R180). */
export const AI_BUDGET: SearchBudget = {
  nodes: 600,
  lethalNodes: 150,
  determinizations: 3,
  beamWidth: 4,
  rootBranching: 20,
  branching: 6,
  maxDepth: 8,
  finalists: 3,
};

/**
 * The quality gates' budget, and the shadow-ban sweep's: the browser's own, so that the gates
 * (docs/polish/3-ai.md B28–B31) and the sweep (R186) measure the AI that ships. A decision seldom
 * spends it all (the beam's shape, not the node count, bounds most turns), so a gate game costs
 * seconds, not minutes. Kept as its own name so that the gates can be given less, with the same
 * algorithm, if CI ever needs them faster.
 */
export const AI_GATE_BUDGET: SearchBudget = AI_BUDGET;

export const AI_SEARCH = {
  /** Plays identical but for `zone` keep the leftmost and rightmost lane only. */
  zoneVariants: 2,
  /** Opponent prompts auto-answered inside one simulated step before it counts as an error. */
  maxAutoAnswers: 8,
  /** Deepest line the lethal solver explores. */
  lethalMaxDepth: 10,
  /**
   * Nodes of the lethal allowance its depth-first walk in move order gets before the best-first walk
   * takes the rest (lethal.ts). At or above SearchBudget.lethalNodes the walk is depth-first alone.
   */
  lethalQuickNodes: 40,
  /** Moves the best-first lethal walk tries from each position it expands, in move order. */
  lethalWidth: 60,
  /** Lines per first action scored after the opponent's reply on determinization 0. */
  linesPerAction: 2,
  /** Seed of the throwaway determinization that lists candidates for the forced check. */
  probeSeed: "ai:probe",
} as const;

export const AI_EVAL = {
  /** won: +win − state.turn; lost: −win + state.turn. */
  win: 1_000_000,
  /** result "draw". */
  drawn: 0,
  /** heroValue(h) = h <= 0 ? −win : heroHealth × sqrt(h × HERO_HEALTH). */
  heroHealth: 1,
  /**
   * Per point of the enemy hero's health, on top of its concave value: damage to the enemy hero
   * shortens the race by the same amount whatever its health, and a draw at the turn cap is no win.
   */
  enemyHealth: 1,
  /** Share of a printed Taunt and a point of Armor that Defense Position's grants are worth (§4.1). */
  positionGrants: 0,
  /** Per point of heroArmorOf. */
  heroArmor: 0.6,
  /** Per point of unitView.attack. */
  attack: 1.2,
  /**
   * The share of a Defense-Position unit's attack that counts (§4.1: it cannot attack until a
   * switch spends its exertion, but it still strikes back in full).
   */
  defenseAttackShare: 0.5,
  /** Per point of unitView.health (current). */
  health: 1,
  /** Per point of unitView.armor. */
  armorPoint: 0.8,
  /** Per keyword in unitView.keywords (spent Divine Shield and Reborn are already gone). */
  keyword: {
    Taunt: 1.5,
    "Divine Shield": 2,
    Lifesteal: 1,
    Poisonous: 2,
    Reborn: 2,
    Charge: 0.5,
    Rush: 0.3,
    "First Strike": 1,
    Trample: 0.5,
    Cleave: 1,
    /** R346: its hits ignore Armor, worth about what Trample is. */
    Pierce: 0.5,
    Indestructible: 4,
    Immutable: 0.3,
    Stack: 0,
    Lucky: 0.2,
    /** E35: no Spell targets it or touches it — protection from removal, and from its owner's buffs. */
    "Immune to Spells": 1,
  },
  /** E6: per point of Spell Damage a unit has (a Spell its controller casts deals that much more). */
  spellDamage: 0.5,
  /**
   * B3.3: a card with a Brittle count n is worth (1 − brittleDiscount / n) of itself — half at
   * Brittle 1, the next tick away from crumbling — on the board, in the backrow and in the own hand.
   */
  brittleDiscount: 0.5,
  /**
   * B3.1: a readable backrow card with Animated is a Unit in waiting, worth this share of its unit
   * face's `unitWorth` on top of its backrow value. Animated, it stands in a unit zone and is a unit.
   */
  animatedShare: 0.5,
  /** Own hand card: handCard + handPerCost × min(queryCost(def), handCostCap). */
  handCard: 1,
  handPerCost: 0.3,
  handCostCap: 6,
  /** Added per own Radiant hand card. */
  radiantInHand: 0.5,
  /**
   * B3.4, R65: taken off an own hand card per crystal its `costMod` (a Degrade's +1, an Upgrade's −1)
   * moves its cost; a cheaper card gains it.
   */
  handCostDelta: 0.5,
  /** An unseen hand card is valued as if it cost this. */
  opponentHandCost: 2,
  /** A readable backrow card: backrowBase + backrowPerCost × queryCost(def). */
  backrowBase: 1.5,
  backrowPerCost: 0.8,
  /** A backrow card the seat cannot read (on the enemy's side of the ledger). */
  enemyFaceDown: 2,
  /** Per library card up to libraryComfort. */
  libraryCard: 0.1,
  libraryComfort: 10,
  /** Per crystal left at the end of the seat's own turn (terminal only). */
  unspentMana: 0.8,
  /** Per point of faceThreat(enemy) against the seat. */
  threatPerDamage: 0.4,
  /** When faceThreat(enemy) >= the seat's hero health. */
  lethalThreat: 150,
  pressurePerDamage: 0.5,
  lethalPressure: 20,
  /** The share of the threat terms that counts when `seat` swings first (evaluate's "seat" frame). */
  answerableThreat: 0.3,
  /** Lethal pressure when `seat` swings first: the enemy has no turn left to answer it. */
  lethalOnBoard: 20,
  /** From this turn on, damage on the enemy hero gains value, rising linearly to the turn cap. */
  closingFrom: 10,
  /** The extra value per point of enemy hero damage once the turn cap is reached (a draw scores 0). */
  closingWeight: 3,
} as const;

/** Every weight `evaluate` reads, as numbers (AI_EVAL's shape without its literal types). */
type Widened<T> = { readonly [K in keyof T]: T[K] extends number ? number : Widened<T[K]> };
export type EvalWeights = Widened<typeof AI_EVAL>;

/**
 * The greedy baseline's own evaluation (baselines.ts): AI_EVAL exactly as it stood when the quality
 * gates were fixed on the seed series `gate:v2`, frozen here. A baseline the AI is measured against
 * must not move when the AI is tuned, so tuning AI_EVAL changes the AI and never its yardstick.
 */
export const GREEDY_EVAL: EvalWeights = {
  win: 1_000_000,
  drawn: 0,
  heroHealth: 1,
  enemyHealth: 1,
  positionGrants: 0,
  heroArmor: 0.6,
  attack: 1.2,
  defenseAttackShare: 0.5,
  health: 1,
  armorPoint: 0.8,
  keyword: {
    Taunt: 1.5,
    "Divine Shield": 2,
    Lifesteal: 1,
    Poisonous: 2,
    Reborn: 2,
    Charge: 0.5,
    Rush: 0.3,
    "First Strike": 1,
    Trample: 0.5,
    Cleave: 1,
    // R346 came after the gates were fixed, so the frozen baseline gives Pierce nothing, as it did.
    Pierce: 0,
    Indestructible: 4,
    Immutable: 0.3,
    Stack: 0,
    Lucky: 0.2,
    // E35, E6, B3.1, B3.3 and B3.4 (patch v0.2.0) came after the gates were fixed, so the frozen
    // baseline gives each of these terms nothing, as it did.
    "Immune to Spells": 0,
  },
  spellDamage: 0,
  brittleDiscount: 0,
  animatedShare: 0,
  handCard: 1,
  handPerCost: 0.3,
  handCostCap: 6,
  radiantInHand: 0.5,
  handCostDelta: 0,
  opponentHandCost: 2,
  backrowBase: 1.5,
  backrowPerCost: 0.8,
  enemyFaceDown: 2,
  libraryCard: 0.1,
  libraryComfort: 10,
  unspentMana: 0.8,
  threatPerDamage: 0.4,
  lethalThreat: 150,
  pressurePerDamage: 0.5,
  lethalPressure: 20,
  answerableThreat: 0.3,
  lethalOnBoard: 20,
  closingFrom: 10,
  closingWeight: 3,
};

/** The opponent's reply that the best lines are scored after (reply.ts, SPEC §9.9). */
export const AI_REPLY = {
  /** Engine steps one reply may take: plays, attacks, prompt answers and the closing endTurn. */
  maxSteps: 12,
  /** Nodes `decide` reserves per reply when it splits the budget (a typical reply, not the most). */
  reserveSteps: 5,
  /** Plays of cards the line put in the opponent's hand that one reply step tries, in move order. */
  knownPlays: 8,
  /** The opponent's value per point of damage its attack would deal the seat's hero. */
  facePerDamage: 1,
  /** Per point an attack deals a unit it does not kill. */
  chipPerDamage: 0.3,
} as const;

export const AI_MULLIGAN = { keepMaxCost: 3 } as const;

/** The greedy baseline's mulligan, frozen with GREEDY_EVAL for the same reason. */
export const GREEDY_MULLIGAN = { keepMaxCost: 3 } as const;

export const AI_DETERMINIZE = {
  /**
   * Catalog ids never sampled into a hidden slot: #98 keeps its rolled power in memory (R43). Ids,
   * not indexes, since an index repeats across sets (B2.2, R387).
   */
  excludeDefIds: ["core-098"],
} as const;

// ---------------------------------------------------------------------------
// R645: the AI's emote personas. Every weight, chance, pool, cap, threshold and delay in the
// issue's §6 table, in one config object keyed by persona so a persona can be tuned — or a new
// one added — without touching personas.ts. Cosmetic only: none of this reaches the engine, the
// action log or a game record, and `decide`/`search.ts` never import it.
// ---------------------------------------------------------------------------

/** §6: the situations a persona may answer, named as personas.ts detects them. */
export const EMOTE_TRIGGERS = [
  /** The mulligan phase just ended. */
  "mulliganEnd",
  /** The AI's turn started while it is ahead (its health − yours ≥ leadHealth, or leadUnits+ more units). */
  "turnStartAhead",
  /** The AI dealt bigHit+ damage to your hero in one hit. */
  "dealtBigHit",
  /** The AI killed your highest-attack unit. */
  "killedTopUnit",
  /** The AI took bigHit+ damage to its hero in one hit. */
  "tookBigHit",
  /** The AI lost its highest-attack unit. */
  "lostTopUnit",
  /** Your turn ran past longTurnMs (once per turn). */
  "playerTurnLong",
  /** You made a big play: bigHit+ damage to its hero, or killsForBigPlay+ of its units in one turn. */
  "playerBigPlay",
  /** You conceded, or the AI dealt lethal. */
  "matchWon",
  /** The AI is about to lose. */
  "matchLost",
] as const;

export type EmoteTrigger = (typeof EMOTE_TRIGGERS)[number];

/** §6's reply table: what a player's emote is answered as, when a row exists for it at all. */
export const EMOTE_REPLY_KEYS = ["greetings", "compliment", "taunt", "apology"] as const;
export type EmoteReplyKey = (typeof EMOTE_REPLY_KEYS)[number];

/** §6's thresholds and timings — every number the trigger table shares. */
export const AI_EMOTE = {
  /** "its health − yours ≥ 10". */
  leadHealth: 10,
  /** "or 3+ more units". */
  leadUnits: 3,
  /** "10+ damage to a hero in one hit". */
  bigHit: 10,
  /** "you kill 2+ of its units in one turn". */
  killsForBigPlay: 2,
  /** "Your turn runs past 45s (once per turn)". */
  longTurnMs: 45_000,
  /** "a random 0.8–2.5s delay" — every rolled emote waits inside this window. */
  delayMinMs: 800,
  delayMaxMs: 2500,
} as const;

export type PersonaName = "balanced" | "polite" | "bm" | "silent";

export type PersonaSpec = {
  /** The share of AI opponents dealt this persona at match creation; weights sum to 1. */
  readonly weight: number;
  /**
   * §6's caps: most emotes within one of the AI's own turns (`ownTurn`) or one of the player's
   * (`otherTurn`), and per match (`match`) — the mulligan greeting and the end-of-match emotes
   * exempted from the last.
   */
  readonly caps: { readonly ownTurn: number; readonly otherTurn: number; readonly match: number };
  readonly triggers: Partial<Record<EmoteTrigger, { readonly chance: number; readonly pool: readonly EmoteId[] }>>;
  readonly replies: Partial<Record<EmoteReplyKey, { readonly chance: number; readonly pool: readonly EmoteId[] }>>;
};

/**
 * §6, verbatim: Balanced 40 / Polite 25 / BM 20 / Silent 15, and each row of the trigger and reply
 * tables. A dash in the table is an absent key here. Silent holds no rows at all: it sends nothing
 * in any situation, including Greetings and Well Played.
 */
export const AI_PERSONAS: Record<PersonaName, PersonaSpec> = {
  balanced: {
    weight: 0.4,
    caps: { ownTurn: 1, otherTurn: 1, match: 8 },
    triggers: {
      mulliganEnd: { chance: 1, pool: ["greetings"] },
      turnStartAhead: { chance: 0.1, pool: ["threaten", "laugh", "yawn"] },
      dealtBigHit: { chance: 0.35, pool: ["laugh", "threaten", "wahWah"] },
      killedTopUnit: { chance: 0.2, pool: ["laugh", "wahWah"] },
      tookBigHit: { chance: 0.25, pool: ["oops", "sob", "angry"] },
      lostTopUnit: { chance: 0.15, pool: ["sob", "angry", "oops"] },
      playerTurnLong: { chance: 0.2, pool: ["yawn"] },
      matchWon: { chance: 0.4, pool: ["wahWah", "laugh", "wellPlayed"] },
      matchLost: { chance: 1, pool: ["wellPlayed"] },
    },
    replies: {
      greetings: { chance: 0.7, pool: ["greetings"] },
      compliment: { chance: 0.3, pool: ["thanks"] },
      taunt: { chance: 0.25, pool: ["laugh", "wahWah", "yawn", "threaten"] },
    },
  },
  polite: {
    weight: 0.25,
    caps: { ownTurn: 1, otherTurn: 1, match: 6 },
    triggers: {
      mulliganEnd: { chance: 1, pool: ["greetings"] },
      dealtBigHit: { chance: 0.15, pool: ["oops"] },
      tookBigHit: { chance: 0.3, pool: ["wellPlayed", "oops"] },
      lostTopUnit: { chance: 0.2, pool: ["wellPlayed"] },
      playerBigPlay: { chance: 0.4, pool: ["wellPlayed"] },
      matchWon: { chance: 1, pool: ["wellPlayed"] },
      matchLost: { chance: 1, pool: ["wellPlayed"] },
    },
    replies: {
      greetings: { chance: 1, pool: ["greetings"] },
      compliment: { chance: 0.8, pool: ["thanks"] },
      taunt: { chance: 0.3, pool: ["oops", "greetings"] },
      apology: { chance: 0.5, pool: ["thanks"] },
    },
  },
  bm: {
    weight: 0.2,
    caps: { ownTurn: 2, otherTurn: 2, match: 20 },
    triggers: {
      mulliganEnd: { chance: 1, pool: ["threaten", "laugh"] },
      turnStartAhead: { chance: 0.5, pool: ["threaten", "laugh", "yawn", "wahWah"] },
      dealtBigHit: { chance: 0.8, pool: ["laugh", "wahWah", "threaten"] },
      killedTopUnit: { chance: 0.6, pool: ["laugh", "wahWah", "yawn"] },
      tookBigHit: { chance: 0.4, pool: ["angry", "threaten"] },
      lostTopUnit: { chance: 0.3, pool: ["angry"] },
      playerTurnLong: { chance: 0.7, pool: ["yawn"] },
      matchWon: { chance: 1, pool: ["wahWah", "laugh"] },
      matchLost: { chance: 0.5, pool: ["sob", "angry"] },
    },
    replies: {
      greetings: { chance: 0.6, pool: ["threaten", "laugh"] },
      compliment: { chance: 0.4, pool: ["yawn"] },
      taunt: { chance: 0.9, pool: ["laugh", "wahWah", "yawn", "threaten"] },
      apology: { chance: 0.7, pool: ["laugh", "wahWah"] },
    },
  },
  silent: {
    weight: 0.15,
    caps: { ownTurn: 0, otherTurn: 0, match: 0 },
    triggers: {},
    replies: {},
  },
};
