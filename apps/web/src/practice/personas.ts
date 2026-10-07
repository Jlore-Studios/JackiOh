// The AI's emote personas (R645), the web client's own since v0.3.0 (docs/v0.3.0/SURFACE.md §9,
// §10.4): the Rust AI plays the game and these, which are presentation, stay TypeScript beside the
// practice page that drives them. Their config, which lived with the AI's other numbers, is here
// too (EMOTE_TRIGGERS, EMOTE_REPLY_KEYS, AI_EMOTE, AI_PERSONAS). `@jackioh/ai` re-exports all of
// it (`wire/ai.ts`), so `practice/emotes.ts` reads it as it did.

/**
 * R645: the AI's emote personas — Balanced, Polite, BM and Silent, and the cosmetic chatter each
 * produces (issue §6).
 *
 * This module is deliberately OUT OF the game:
 *
 *  - Nothing here touches `GameState`, `reduce`, the action log or a replay hash, and nothing an
 *    emote decides reaches a play decision. The engine and the AI's move search (`decide`,
 *    `search`, `reply`, `lethal`, …) are Rust and cannot import this file.
 *  - Its inputs are exactly what a watching client would have: `PlayerView`s and their redacted
 *    event windows. A persona that could read the true state would be a tell, not a personality.
 *  - Its randomness arrives as an injected `() => number` (production passes `Math.random`,
 *    per the issue; tests pass constants). It never shares `state.rng`, `createRng` or the
 *    determinizer's draws, so emotes cannot perturb a seeded game or a search.
 *  - Every number it reads lives in `AI_PERSONAS`/`AI_EMOTE` (below): weights, chances,
 *    pools, caps, thresholds and the 0.8–2.5 s delivery delay.
 *
 * The pure surface is three functions — `pickPersona`, `rollForTrigger`, `rollForReply` — plus
 * `createEmotePersona`, a small state machine the practice page drives: views in, emote intents
 * out. The page owns the timers (an intent's `delayMs`) and the wiring; this owns every rule.
 */

import type { EmoteId, GameEvent, PlayerId, PlayerView, SideView } from "../wire/index.ts";
import { emoteGate, opponentOf } from "../wire/index.ts";

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

/** An emote the persona wants on screen, `delayMs` after its trigger. */
export type AiEmote = { emote: EmoteId; delayMs: number };

const sideOf = (view: PlayerView, seat: PlayerId): SideView =>
  view.you.player === seat ? view.you : view.opponent;

const unitCount = (side: SideView): number =>
  side.units.filter((unit) => unit !== null).length;

const maxAttackOf = (side: SideView): number =>
  side.units.reduce((top, unit) => Math.max(top, unit?.attack ?? 0), 0);

// ---------------------------------------------------------------------------
// The pure rolls — §6's weights, chances, pools and delays, nothing else.
// ---------------------------------------------------------------------------

const PERSONA_ORDER = ["balanced", "polite", "bm", "silent"] as const;

/**
 * §6's weighted pick: Balanced 40%, Polite 25%, BM 20%, Silent 15%, in that order, so a draw lands
 * on the first interval it falls inside. The tutorial calls none of this — it is always Silent.
 */
export function pickPersona(rng: () => number): PersonaName {
  // Per-mille: the issue's boundaries (0.4, 0.65, 0.85) are exact in thousandths, which the float
  // sum is not (0.4+0.25+0.2 lands one ulp past 0.85). Comparing integers keeps a boundary draw
  // on the interval that opens there — 0.85 is Silent, not BM.
  const draw = Math.floor(rng() * 1000);
  let cumulative = 0;
  for (const name of PERSONA_ORDER) {
    cumulative += Math.round(AI_PERSONAS[name].weight * 1000);
    if (draw < cumulative) return name;
  }
  // A hostile rng() returning 1 cannot pick nothing; the last interval is closed from the left.
  return "silent";
}

/** The uniform pick from a pool, or null for an empty one. */
function pickFrom(pool: readonly EmoteId[], rng: () => number): EmoteId | null {
  const at = Math.min(pool.length - 1, Math.floor(rng() * pool.length));
  return pool[at] ?? null;
}

/** §6's "a random 0.8–2.5s delay", drawn inside the window. */
function rollDelay(rng: () => number): number {
  return AI_EMOTE.delayMinMs + rng() * (AI_EMOTE.delayMaxMs - AI_EMOTE.delayMinMs);
}

/**
 * One roll of §6's trigger table: `chance` first, then a uniform pick from the row's pool. A row
 * the persona does not have (a dash in the table, or Silent) returns null; so does a missed roll.
 * Caps and the rate limit are NOT here — they are the session's, applied in `admit`.
 */
export function rollForTrigger(
  persona: PersonaName,
  trigger: EmoteTrigger,
  rng: () => number,
): AiEmote | null {
  const spec = AI_PERSONAS[persona].triggers[trigger];
  if (spec === undefined || rng() >= spec.chance) return null;
  const emote = pickFrom(spec.pool, rng);
  return emote === null ? null : { emote, delayMs: rollDelay(rng) };
}

/** §6's reply table: which of its four rows a player's emote lands on, if any (a yawn earns none). */
const REPLY_KEY_OF: Partial<Record<EmoteId, EmoteReplyKey>> = {
  greetings: "greetings",
  wellPlayed: "compliment",
  thanks: "compliment",
  threaten: "taunt",
  laugh: "taunt",
  wahWah: "taunt",
  angry: "taunt",
  oops: "apology",
  sob: "apology",
};

export function replyKeyOf(emote: EmoteId): EmoteReplyKey | null {
  return REPLY_KEY_OF[emote] ?? null;
}

/** One roll of §6's reply table for the row `emote` lands on, or null like `rollForTrigger`. */
export function rollForReply(
  persona: PersonaName,
  emote: EmoteId,
  rng: () => number,
): (AiEmote & { key: EmoteReplyKey }) | null {
  const key = replyKeyOf(emote);
  if (key === null) return null;
  const spec = AI_PERSONAS[persona].replies[key];
  if (spec === undefined || rng() >= spec.chance) return null;
  const picked = pickFrom(spec.pool, rng);
  return picked === null ? null : { emote: picked, delayMs: rollDelay(rng), key };
}

// ---------------------------------------------------------------------------
// The session: trigger detection over view/event deltas, plus §6's caps and
// the same rate limit every player obeys.
// ---------------------------------------------------------------------------

export type EmotePersona = {
  readonly persona: PersonaName;
  /**
   * A fresh view arrived: `fresh` is the events that are new in `view` (the driver's
   * `newEventsSince` gives them), `prev` the view before it (null at game start). Emote intents,
   * already rolled and gate-checked.
   */
  onEvents: (
    fresh: readonly GameEvent[],
    prev: PlayerView | null,
    view: PlayerView,
    now: number,
  ) => AiEmote[];
  /**
   * The player sent an emote. Feeding the AI's own emotes back here is a caller bug — that is
   * what makes "the AI never replies to a reply" structural rather than a flag to forget.
   */
  onPlayerEmote: (emote: EmoteId, now: number) => AiEmote[];
  /**
   * The player's turn crossed the long-turn mark. The driver calls at most once per player turn;
   * §6's "(once per turn)" is additionally held here by the turn number.
   */
  onPlayerTurnLong: (view: PlayerView, now: number) => AiEmote[];
};

/**
 * Creates the persona one AI seat plays for a match. `rng` is the injected draw — Math.random in
 * production, a constant table in tests — and every roll the match makes goes through it in call
 * order, so a test that fixes the stream predicts every emote exactly.
 */
export function createEmotePersona(opts: {
  persona: PersonaName;
  seat: PlayerId;
  rng: () => number;
}): EmotePersona {
  const { persona, seat, rng } = opts;
  const spec = AI_PERSONAS[persona];

  // §6's bookkeeping: the player's own rate limit (R643), the turn and match caps, the reply
  // rules already spent, and the per-turn counters the kill triggers accumulate into.
  const sentAt: number[] = [];
  let sentMatch = 0;
  let sentOwnTurn = 0;
  let sentOtherTurn = 0;
  const repliesUsed = new Set<EmoteReplyKey>();
  let killsThisTurn = 0;
  let bigPlayRolledTurn = -1;
  let longTurnRolledTurn = -1;
  // The view a reply is answered under, for the turn bucket its caps count into. Replies can only
  // arrive while a board exists, but the field stays honest before the first view anyway.
  let lastMoment: { active: PlayerId; phase: PlayerView["phase"] } | null = null;

  /**
   * Whether `roll` may go out: §6's caps and R643's limiter, in order, minus the two rows §6
   * exempts from the per-match cap (the mulligan greeting and the end-of-match emotes). A blocked
   * emote is dropped, never queued — `emoteGate` is exactly the function the client and the
   * server run (R643), judged at the instant the page would fire it (`now + delayMs`).
   */
  function admit(
    roll: AiEmote | null,
    moment: { active: PlayerId; phase: PlayerView["phase"] },
    now: number,
    matchExempt: boolean,
  ): AiEmote | null {
    if (roll === null) return null;
    if (moment.phase === "over") {
      // The caps no longer bind a finished game, but §6's "every persona obeys the player rate
      // limit" still does — the exemption §6 gives the end-of-match emotes is the match cap's.
      if (!emoteGate(sentAt, now + roll.delayMs).ok) return null;
      sentAt.push(now + roll.delayMs);
      return roll;
    }
    const bucket = moment.active === seat ? "ownTurn" : "otherTurn";
    if ((bucket === "ownTurn" ? sentOwnTurn : sentOtherTurn) >= spec.caps[bucket]) return null;
    if (!matchExempt && sentMatch >= spec.caps.match) return null;
    if (!emoteGate(sentAt, now + roll.delayMs).ok) return null;
    if (bucket === "ownTurn") sentOwnTurn += 1;
    else sentOtherTurn += 1;
    if (!matchExempt) sentMatch += 1;
    sentAt.push(now + roll.delayMs);
    return roll;
  }

  function emit(
    roll: AiEmote | null,
    moment: { active: PlayerId; phase: PlayerView["phase"] },
    now: number,
    matchExempt: boolean,
  ): AiEmote[] {
    const admitted = admit(roll, moment, now, matchExempt);
    return admitted === null ? [] : [admitted];
  }

  /** Detect §6's triggers across one view step, in the table's order, and roll each that fires. */
  function onEvents(
    fresh: readonly GameEvent[],
    prev: PlayerView | null,
    view: PlayerView,
    now: number,
  ): AiEmote[] {
    const out: AiEmote[] = [];
    const you = sideOf(view, seat);
    const enemy = sideOf(view, opponentOf(seat));
    const moment = { active: view.active, phase: view.phase };
    lastMoment = moment;

    // 1. Mulligan ends: the window that showed `mulligan` is gone after one showed it. It can
    // only become true once, so no spent-flag is needed.
    if (prev?.mulligan !== undefined && view.mulligan === undefined) {
      out.push(...emit(rollForTrigger(persona, "mulliganEnd", rng), moment, now, true));
    }

    for (const event of fresh) {
      switch (event.type) {
        case "turnStarted": {
          // Per-turn counters reset on either turn starting — first, so an emote the same batch
          // rolls under the new turn's caps and kill count.
          if (event.player === seat) sentOwnTurn = 0;
          else sentOtherTurn = 0;
          killsThisTurn = 0;
          if (event.player !== seat) break;
          // 2. AI's turn starts while ahead: its health − yours ≥ 10, or 3+ more units.
          const ahead =
            you.hero.health - enemy.hero.health >= AI_EMOTE.leadHealth ||
            unitCount(you) - unitCount(enemy) >= AI_EMOTE.leadUnits;
          if (ahead) {
            out.push(...emit(rollForTrigger(persona, "turnStartAhead", rng), moment, now, false));
          }
          break;
        }
        case "damage": {
          if (event.targetId === `hero-${opponentOf(seat)}` && event.amount >= AI_EMOTE.bigHit) {
            // 3. AI deals 10+ to your hero in one hit.
            out.push(...emit(rollForTrigger(persona, "dealtBigHit", rng), moment, now, false));
          }
          if (event.targetId === `hero-${seat}` && event.amount >= AI_EMOTE.bigHit) {
            // 5. AI takes 10+ to its hero in one hit — which is also the damage half of 8, "you
            // make a big play", so the same hit rolls both rows (each rolls its own chance once).
            out.push(...emit(rollForTrigger(persona, "tookBigHit", rng), moment, now, false));
            out.push(...emit(rollForTrigger(persona, "playerBigPlay", rng), moment, now, false));
          }
          break;
        }
        case "healthLost":
        case "fatigue": {
          // 5's non-damage half: a "lose health" effect or a fatigue tick of 10+ lands the same.
          if (event.player === seat && event.amount >= AI_EMOTE.bigHit) {
            out.push(...emit(rollForTrigger(persona, "tookBigHit", rng), moment, now, false));
          }
          break;
        }
        case "destroyed": {
          if (event.killerId === null) break;
          if (event.controller === seat) {
            // 6. AI loses its highest-attack unit: the dying attack reaches the pre-view top.
            const top = prev === null ? event.attack : maxAttackOf(sideOf(prev, seat));
            if (event.attack >= top) {
              out.push(...emit(rollForTrigger(persona, "lostTopUnit", rng), moment, now, false));
            }
            // 8's second half: you kill 2+ of the AI's units in one turn — a unit the AI
            // controlled dying on the player's turn. Rolled once per turn, when the count first
            // reaches two.
            if (view.active !== seat) {
              killsThisTurn += 1;
              if (killsThisTurn >= AI_EMOTE.killsForBigPlay && bigPlayRolledTurn !== view.turn) {
                bigPlayRolledTurn = view.turn;
                out.push(
                  ...emit(rollForTrigger(persona, "playerBigPlay", rng), moment, now, false),
                );
              }
            }
          } else {
            // 4. AI kills your highest-attack unit, same comparison on the human side.
            const top = prev === null ? event.attack : maxAttackOf(sideOf(prev, opponentOf(seat)));
            if (event.attack >= top) {
              out.push(...emit(rollForTrigger(persona, "killedTopUnit", rng), moment, now, false));
            }
          }
          break;
        }
        case "gameOver": {
          if (event.winner === seat) {
            // 9. You concede, or the AI dealt lethal: any win is the same trigger, and both are
            // §6's exempt end-of-match emotes (as is 10, which fires at the same `gameOver`).
            out.push(...emit(rollForTrigger(persona, "matchWon", rng), moment, now, true));
          } else if (event.winner === opponentOf(seat)) {
            // 10. The AI is about to lose. A draw is nobody's loss, so "draw" wins land nowhere.
            out.push(...emit(rollForTrigger(persona, "matchLost", rng), moment, now, true));
          }
          break;
        }
        default:
          break;
      }
    }
    return out;
  }

  return {
    persona,
    onEvents,
    onPlayerEmote: (emote, now) => {
      // §6: each reply rule fires at most once per match — consumed only when an emote actually
      // goes out, so a missed roll leaves the rule live for a later emote of the same kind.
      const roll = rollForReply(persona, emote, rng);
      if (roll === null || repliesUsed.has(roll.key)) return [];
      const admitted = admit(
        roll,
        lastMoment ?? { active: opponentOf(seat), phase: "mulligan" },
        now,
        false,
      );
      if (admitted !== null) repliesUsed.add(roll.key);
      return admitted === null ? [] : [admitted];
    },
    onPlayerTurnLong: (view, now) => {
      lastMoment = { active: view.active, phase: view.phase };
      // §6's "(once per turn)": the turn number holds it even if the driver calls twice.
      if (view.active === seat || view.turn === longTurnRolledTurn) return [];
      longTurnRolledTurn = view.turn;
      const admitted = admit(
        rollForTrigger(persona, "playerTurnLong", rng),
        { active: view.active, phase: view.phase },
        now,
        false,
      );
      return admitted === null ? [] : [admitted];
    },
  };
}
