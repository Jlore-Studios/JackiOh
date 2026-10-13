// R645 presentation-only emote personas (docs/v0.3.0/SURFACE.md §9, §10.4).
// `@jackioh/ai` re-exports this config through `wire/ai.ts` for `practice/emotes.ts`.

/**
 * R645: cosmetic persona chatter reads redacted views and injected randomness only; it cannot
 * affect game state, replay state, or AI search. Config owns every number; the page owns timers.
 * R1344: rolls draw only from the AI seat's dealt hand, or send nothing.
 */

import type { EmoteId, GameEvent, PlayerId, PlayerView, SideView } from "../wire/index.ts";
import { EMOTE_IDS, emoteGate, handHolds, opponentOf } from "../wire/index.ts";

// R645: §6 persona configuration; cosmetic only.

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

/** §6 persona tables; absent rows send nothing. R1344 adds MN03 emoji after §6 emotes; Polite never taunts. */
export const AI_PERSONAS: Record<PersonaName, PersonaSpec> = {
  balanced: {
    weight: 0.4,
    caps: { ownTurn: 1, otherTurn: 1, match: 8 },
    triggers: {
      mulliganEnd: { chance: 1, pool: ["greetings", "wave", "salute"] },
      turnStartAhead: { chance: 0.1, pool: ["threaten", "laugh", "yawn", "cool"] },
      dealtBigHit: { chance: 0.35, pool: ["laugh", "threaten", "wahWah", "fire"] },
      killedTopUnit: { chance: 0.2, pool: ["laugh", "wahWah", "skull"] },
      tookBigHit: { chance: 0.25, pool: ["oops", "sob", "angry", "sweat", "gasp"] },
      lostTopUnit: { chance: 0.15, pool: ["sob", "angry", "oops", "facepalm", "shrug"] },
      playerTurnLong: { chance: 0.2, pool: ["yawn", "thinking"] },
      matchWon: { chance: 0.4, pool: ["wahWah", "laugh", "wellPlayed", "party", "salute"] },
      matchLost: { chance: 1, pool: ["wellPlayed", "salute", "clap"] },
    },
    replies: {
      greetings: { chance: 0.7, pool: ["greetings", "wave"] },
      compliment: { chance: 0.3, pool: ["thanks", "thumbsUp"] },
      taunt: { chance: 0.25, pool: ["laugh", "wahWah", "yawn", "threaten", "cool"] },
    },
  },
  polite: {
    weight: 0.25,
    caps: { ownTurn: 1, otherTurn: 1, match: 6 },
    triggers: {
      mulliganEnd: { chance: 1, pool: ["greetings", "wave", "salute"] },
      dealtBigHit: { chance: 0.15, pool: ["oops", "sweat"] },
      tookBigHit: { chance: 0.3, pool: ["wellPlayed", "oops", "clap", "gasp"] },
      lostTopUnit: { chance: 0.2, pool: ["wellPlayed", "clap"] },
      playerBigPlay: { chance: 0.4, pool: ["wellPlayed", "clap", "thumbsUp"] },
      matchWon: { chance: 1, pool: ["wellPlayed", "salute", "heart"] },
      matchLost: { chance: 1, pool: ["wellPlayed", "clap", "salute"] },
    },
    replies: {
      greetings: { chance: 1, pool: ["greetings", "wave"] },
      compliment: { chance: 0.8, pool: ["thanks", "heart", "thumbsUp"] },
      taunt: { chance: 0.3, pool: ["oops", "greetings", "shrug"] },
      apology: { chance: 0.5, pool: ["thanks", "thumbsUp"] },
    },
  },
  bm: {
    weight: 0.2,
    caps: { ownTurn: 2, otherTurn: 2, match: 20 },
    triggers: {
      mulliganEnd: { chance: 1, pool: ["threaten", "laugh", "cool", "skull"] },
      turnStartAhead: { chance: 0.5, pool: ["threaten", "laugh", "yawn", "wahWah", "cool"] },
      dealtBigHit: { chance: 0.8, pool: ["laugh", "wahWah", "threaten", "fire", "skull"] },
      killedTopUnit: { chance: 0.6, pool: ["laugh", "wahWah", "yawn", "skull"] },
      tookBigHit: { chance: 0.4, pool: ["angry", "threaten"] },
      lostTopUnit: { chance: 0.3, pool: ["angry"] },
      playerTurnLong: { chance: 0.7, pool: ["yawn"] },
      matchWon: { chance: 1, pool: ["wahWah", "laugh", "party", "cool"] },
      matchLost: { chance: 0.5, pool: ["sob", "angry"] },
    },
    replies: {
      greetings: { chance: 0.6, pool: ["threaten", "laugh", "cool"] },
      compliment: { chance: 0.4, pool: ["yawn", "cool"] },
      taunt: { chance: 0.9, pool: ["laugh", "wahWah", "yawn", "threaten", "skull", "fire"] },
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

// §6 pure rolls

const PERSONA_ORDER = ["balanced", "polite", "bm", "silent"] as const;

/** §6 weighted pick; tutorial callers are always Silent. */
export function pickPersona(rng: () => number): PersonaName {
  // Integer thresholds keep 0.85 in Silent despite floating-point summation.
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

/** R1344: the part of a row's pool the seat's hand holds, in the pool's order. */
export function poolInHand(pool: readonly EmoteId[], hand: readonly EmoteId[]): EmoteId[] {
  return pool.filter((emote) => handHolds(hand, emote));
}

/** §6 trigger roll; R1344 filters the pool by hand. Caps and rate limits are applied in `admit`. */
export function rollForTrigger(
  persona: PersonaName,
  trigger: EmoteTrigger,
  rng: () => number,
  hand: readonly EmoteId[] = EMOTE_IDS,
): AiEmote | null {
  const spec = AI_PERSONAS[persona].triggers[trigger];
  if (spec === undefined || rng() >= spec.chance) return null;
  const emote = pickFrom(poolInHand(spec.pool, hand), rng);
  return emote === null ? null : { emote, delayMs: rollDelay(rng) };
}

/** §6 reply rows; R1344 classifies MN03 emoji by the same sense. */
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
  wave: "greetings",
  clap: "compliment",
  thumbsUp: "compliment",
  heart: "compliment",
  salute: "compliment",
  fire: "taunt",
  skull: "taunt",
  cool: "taunt",
  party: "taunt",
  facepalm: "apology",
  sweat: "apology",
};

export function replyKeyOf(emote: EmoteId): EmoteReplyKey | null {
  return REPLY_KEY_OF[emote] ?? null;
}

/** §6 reply roll, with R1344 hand filtering. */
export function rollForReply(
  persona: PersonaName,
  emote: EmoteId,
  rng: () => number,
  hand: readonly EmoteId[] = EMOTE_IDS,
): (AiEmote & { key: EmoteReplyKey }) | null {
  const key = replyKeyOf(emote);
  if (key === null) return null;
  const spec = AI_PERSONAS[persona].replies[key];
  if (spec === undefined || rng() >= spec.chance) return null;
  const picked = pickFrom(poolInHand(spec.pool, hand), rng);
  return picked === null ? null : { emote: picked, delayMs: rollDelay(rng), key };
}

// §6 session triggers, caps, and rate limit

export type EmotePersona = {
  readonly persona: PersonaName;
  /** New events and their surrounding views produce gate-checked emote intents. */
  onEvents: (
    fresh: readonly GameEvent[],
    prev: PlayerView | null,
    view: PlayerView,
    now: number,
  ) => AiEmote[];
  /** Player emotes only: the AI cannot reply to a reply. */
  onPlayerEmote: (emote: EmoteId, now: number) => AiEmote[];
  /** §6 long-turn trigger, held once per turn by the turn number. */
  onPlayerTurnLong: (view: PlayerView, now: number) => AiEmote[];
};

/** Creates one match persona. R1341/R1344 constrain every roll to its dealt hand. */
export function createEmotePersona(opts: {
  persona: PersonaName;
  seat: PlayerId;
  rng: () => number;
  hand?: readonly EmoteId[];
}): EmotePersona {
  const { persona, seat, rng, hand = EMOTE_IDS } = opts;
  const spec = AI_PERSONAS[persona];

  // §6/R643 bookkeeping for caps, replies, and kill triggers.
  const sentAt: number[] = [];
  let sentMatch = 0;
  let sentOwnTurn = 0;
  let sentOtherTurn = 0;
  const repliesUsed = new Set<EmoteReplyKey>();
  let killsThisTurn = 0;
  let bigPlayRolledTurn = -1;
  let longTurnRolledTurn = -1;
  // Replies use the most recent turn bucket for their caps.
  let lastMoment: { active: PlayerId; phase: PlayerView["phase"] } | null = null;

  /** §6 caps and R643's shared limiter; blocked emotes are dropped at `now + delayMs`. */
  function admit(
    roll: AiEmote | null,
    moment: { active: PlayerId; phase: PlayerView["phase"] },
    now: number,
    matchExempt: boolean,
  ): AiEmote | null {
    if (roll === null) return null;
    if (moment.phase === "over") {
      // §6 exempts finished games from caps, not R643's rate limit.
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

    // Mulligan end is the transition away from a previous mulligan view.
    if (prev?.mulligan !== undefined && view.mulligan === undefined) {
      out.push(...emit(rollForTrigger(persona, "mulliganEnd", rng, hand), moment, now, true));
    }

    for (const event of fresh) {
      switch (event.type) {
        case "turnStarted": {
          // Reset before same-batch rolls use the new turn's caps and kill count.
          if (event.player === seat) sentOwnTurn = 0;
          else sentOtherTurn = 0;
          killsThisTurn = 0;
          if (event.player !== seat) break;
          const ahead =
            you.hero.health - enemy.hero.health >= AI_EMOTE.leadHealth ||
            unitCount(you) - unitCount(enemy) >= AI_EMOTE.leadUnits;
          if (ahead) {
            out.push(...emit(rollForTrigger(persona, "turnStartAhead", rng, hand), moment, now, false));
          }
          break;
        }
        case "damage": {
          if (event.targetId === `hero-${opponentOf(seat)}` && event.amount >= AI_EMOTE.bigHit) {
            out.push(...emit(rollForTrigger(persona, "dealtBigHit", rng, hand), moment, now, false));
          }
          if (event.targetId === `hero-${seat}` && event.amount >= AI_EMOTE.bigHit) {
            // The same hit independently rolls tookBigHit and playerBigPlay.
            out.push(...emit(rollForTrigger(persona, "tookBigHit", rng, hand), moment, now, false));
            out.push(...emit(rollForTrigger(persona, "playerBigPlay", rng, hand), moment, now, false));
          }
          break;
        }
        case "healthLost":
        case "fatigue": {
          if (event.player === seat && event.amount >= AI_EMOTE.bigHit) {
            out.push(...emit(rollForTrigger(persona, "tookBigHit", rng, hand), moment, now, false));
          }
          break;
        }
        case "destroyed": {
          if (event.killerId === null) break;
          if (event.controller === seat) {
            const top = prev === null ? event.attack : maxAttackOf(sideOf(prev, seat));
            if (event.attack >= top) {
              out.push(...emit(rollForTrigger(persona, "lostTopUnit", rng, hand), moment, now, false));
            }
            // Roll playerBigPlay once when a player turn reaches its kill threshold.
            if (view.active !== seat) {
              killsThisTurn += 1;
              if (killsThisTurn >= AI_EMOTE.killsForBigPlay && bigPlayRolledTurn !== view.turn) {
                bigPlayRolledTurn = view.turn;
                out.push(
                  ...emit(rollForTrigger(persona, "playerBigPlay", rng, hand), moment, now, false),
                );
              }
            }
          } else {
            const top = prev === null ? event.attack : maxAttackOf(sideOf(prev, opponentOf(seat)));
            if (event.attack >= top) {
              out.push(...emit(rollForTrigger(persona, "killedTopUnit", rng, hand), moment, now, false));
            }
          }
          break;
        }
        case "gameOver": {
          if (event.winner === seat) {
            // §6 exempts end-of-match emotes from the match cap.
            out.push(...emit(rollForTrigger(persona, "matchWon", rng, hand), moment, now, true));
          } else if (event.winner === opponentOf(seat)) {
            out.push(...emit(rollForTrigger(persona, "matchLost", rng, hand), moment, now, true));
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
      // §6 reply rules are consumed only when their emote is sent.
      const roll = rollForReply(persona, emote, rng, hand);
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
        rollForTrigger(persona, "playerTurnLong", rng, hand),
        { active: view.active, phase: view.phase },
        now,
        false,
      );
      return admitted === null ? [] : [admitted];
    },
  };
}
