// The shadow-ban sweep (R186, R390, docs/polish/3-ai.md B25, docs/classic-sets.md B4.4): force one
// card into AI decks, play them against the greedy baseline, and flag what went wrong with that card.
// `scripts/sweep.ts` runs it over every non-token card of every set at every tier in AI_SWEEP.tiers
// and prints the rows that `shadowBan.ts` is filled from.
//
// Why more than one tier. The ban keeps a card out of the AI's decks at every difficulty, and a
// tier's resources decide what the AI can do with a card: at Easy's four crystals a 6-cost card is
// never affordable at all, so an Easy-only sweep has no evidence either way, and a card the AI
// never played on four crystals may well be played on seven. So each card is swept at the
// cheapest tier (Easy) and the richest (Hard): a flag at either bans it (the reason names the
// tier), and a card that was never once affordable at any tier is reported as unswept rather than
// passed as clean.
//
// Two passes (R390). Pass 1 (`sweepCard`) is the sweep above. A card is at risk (`atRiskIds`) when
// pass 1's numbers meet a flag's condition at half strength (`halfFlags`), or it is banned already
// or on `SHADOW_WATCH`. Pass 2 (`sweepAtRisk`) forces each at-risk card into more games, on seeds of
// its own, with every at-risk card's filler weight boosted, and counts every at-risk card the AI was
// dealt, forced or not. A `neverPlayed` or `selfHarm` ban needs pass 2's numbers; `error` and
// `timeout` ban from any game whose forced card the card was (`sweepVerdict`).
//
// Pure like the rest of src/: the clock arrives as `now`, which the script passes
// (`performance.now`) and a test leaves out, so a test's sweep never times out on a slow machine.

import { opponentOf, type PlayerId } from "@jackioh/shared";
import { AI_DIFFICULTY, createRng, effectiveCost, zoneCards, type GameState } from "@jackioh/engine";
import type { Difficulty } from "@jackioh/engine/config";
import { AI_GATE_BUDGET } from "./config";
import { buildAiDeck } from "./deck";
import { evaluate } from "./evaluate";
import { playMatch, type MatchConfig, type MatchHooks } from "./match";
import { SHADOW_BAN, SHADOW_WATCH } from "./shadowBan";

export type SweepFlag = "error" | "timeout" | "neverPlayed" | "selfHarm";

export const AI_SWEEP = {
  /** The tiers every card is swept at: the fewest resources and the most (see the header). */
  tiers: ["easy", "hard"] as readonly Difficulty[],
  /** Games per card and tier. Four left the neverPlayed flag at the mercy of one deal. */
  seedsPerCard: 8,
  decisionMs: 2000,
  maxActions: 600,
  minAffordableTurns: 3,
  selfHarmDelta: -40,
  /** selfHarm needs at least this many plays, so one play into a trap does not ban a card. */
  minHarmPlays: 4,
  /** R390: pass 2's games per at-risk card and tier. */
  seedsPerCardAtRisk: 24,
  /** R390: pass 2's filler draw multiplies every at-risk card's weight by this (as AI_DECK.themeBoost). */
  atRiskBoost: 4,
  /** R390: a neverPlayed ban needs this many affordable turns over pass 2's games, and no play. */
  banAffordableTurns: 6,
  /** R390: a selfHarm ban needs this many plays over pass 2's games. */
  banHarmPlays: 8,
} as const;

export type SweepStats = {
  defId: string;
  games: number;
  drawnGames: number;
  affordableTurns: number;
  plays: number;
  errors: number;
  timeouts: number;
  evalDeltaSum: number;
  evalDeltaCount: number;
};

/**
 * One card at one tier. `unswept` is a card that was never once affordable in hand, so its games
 * say nothing about whether the AI can play it (a report, never a ban flag).
 */
export type SweepResult = SweepStats & { tier: Difficulty; flags: SweepFlag[]; unswept: boolean };

/**
 * R390: an error or a timeout in a pass-2 game, listed against an at-risk card the AI was dealt as
 * filler in it. It bans the forced card only; the suspect is banned only if its own games repeat it.
 */
export type SweepSuspect = { defId: string; seed: string; forced: string; errors: number; timeouts: number };

/**
 * R390: one at-risk card's pass 2 at one tier. `cards` holds every at-risk card the AI was dealt in
 * these games, the forced one first, each counted over the games it was dealt in; only the forced
 * card's entry counts errors and timeouts, which `suspects` lists against the filler.
 */
export type SweepPass2 = { forced: string; tier: Difficulty; games: number; cards: SweepStats[]; suspects: SweepSuspect[] };

/** A card over every tier and pass it was swept at: the union of the flags, and the ban reason if any. */
export type SweepVerdict = {
  defId: string;
  flags: SweepFlag[];
  /** Never affordable at any tier: no evidence for or against it. */
  unswept: boolean;
  /** The SHADOW_BAN reason, `"<flags>: <tier>: <what it measured>; …"`, or null when unflagged. */
  reason: string | null;
  /** R390, R600: the SHADOW_WATCH entry (at risk by its own numbers, and not banned), or null. */
  watch: string | null;
};

const FLAG_ORDER: readonly SweepFlag[] = ["error", "timeout", "neverPlayed", "selfHarm"];

function mean(stats: SweepStats): number {
  return stats.evalDeltaSum / Math.max(1, stats.evalDeltaCount);
}

/** error: errors > 0; timeout: timeouts > 0; neverPlayed: affordableTurns >= minAffordableTurns && plays === 0;
 *  selfHarm: evalDeltaCount >= minHarmPlays && evalDeltaSum / evalDeltaCount < selfHarmDelta. In that order. */
export function sweepFlags(stats: SweepStats): SweepFlag[] {
  const flags: SweepFlag[] = [];
  if (stats.errors > 0) flags.push("error");
  if (stats.timeouts > 0) flags.push("timeout");
  if (stats.affordableTurns >= AI_SWEEP.minAffordableTurns && stats.plays === 0) flags.push("neverPlayed");
  if (stats.evalDeltaCount >= AI_SWEEP.minHarmPlays && mean(stats) < AI_SWEEP.selfHarmDelta) flags.push("selfHarm");
  return flags;
}

/**
 * R390: the judgement flags at half strength, which put a card at risk: `neverPlayed` when it sat
 * affordable on minAffordableTurns turns and was played at most once, `selfHarm` when its plays'
 * mean evaluation change is below half of selfHarmDelta.
 */
export function halfFlags(stats: SweepStats): SweepFlag[] {
  const flags: SweepFlag[] = [];
  if (stats.affordableTurns >= AI_SWEEP.minAffordableTurns && stats.plays <= 1) flags.push("neverPlayed");
  if (stats.evalDeltaCount > 0 && mean(stats) < AI_SWEEP.selfHarmDelta / 2) flags.push("selfHarm");
  return flags;
}

/** The flags a SHADOW_BAN reason starts with ("error, neverPlayed: easy: …" → error, neverPlayed). */
export function banFlags(reason: string): SweepFlag[] {
  const head = (reason.split(":")[0] ?? "").split(", ");
  return FLAG_ORDER.filter((flag) => head.includes(flag));
}

/**
 * R390: the at-risk cards — every card some tier's pass-1 numbers meet at half strength
 * (`halfFlags`), and every card on `ban` or `watch` (today's SHADOW_BAN and SHADOW_WATCH by
 * default). A pure function of pass 1's results and the two tables, sorted.
 */
export function atRiskIds(
  pass1: readonly SweepResult[],
  ban: Readonly<Record<string, string>> = SHADOW_BAN,
  watch: Readonly<Record<string, string>> = SHADOW_WATCH,
): string[] {
  const ids = new Set([...Object.keys(ban), ...Object.keys(watch)]);
  for (const result of pass1) if (halfFlags(result).length > 0) ids.add(result.defId);
  return [...ids].sort();
}

/**
 * R390: the cards pass 2's filler never deals — banned for `error` or `timeout` today, or flagged so
 * by pass 1 — so a known bug is never filler. A card banned for `neverPlayed` or `selfHarm` is at
 * risk, and pass 2's filler lifts its ban.
 */
export function pass2KeepOut(pass1: readonly SweepResult[], ban: Readonly<Record<string, string>> = SHADOW_BAN): string[] {
  const bug = (flags: readonly SweepFlag[]): boolean => flags.includes("error") || flags.includes("timeout");
  const ids = new Set(Object.keys(ban).filter((id) => bug(banFlags(ban[id] ?? ""))));
  for (const result of pass1) if (bug(result.flags)) ids.add(result.defId);
  return [...ids].sort();
}

/** What the AI's deck is dealt from in one sweep game, beside its forced card. */
type Filler = { banned: readonly string[]; boost?: { ids: readonly string[]; by: number } };

/** Game n (1-based) of a card's sweep: the AI sits p1 when n is odd and p2 when even. */
function sweptSeatOf(n: number): PlayerId {
  return n % 2 === 1 ? "p1" : "p2";
}

/** Game n of a card's sweep at a tier: the AI on that tier's handicap, greedy on Easy's (a human's). */
function sweepConfig(seed: string, defId: string, n: number, tier: Difficulty, filler: Filler): { config: MatchConfig; aiSeat: PlayerId } {
  const aiSeat = sweptSeatOf(n);
  const greedySeat = opponentOf(aiSeat);
  const handicap = AI_DIFFICULTY[tier];
  const easy = AI_DIFFICULTY.easy;

  // The swept card is forced in even when it is banned today, so a rerun can clear it.
  const aiDeck = buildAiDeck(createRng(`${seed}:deck:${aiSeat}`), handicap.deckSize, {
    include: [defId],
    banned: filler.banned.filter((id) => id !== defId),
    manaCap: handicap.manaCap,
    ...(filler.boost === undefined ? {} : { boost: filler.boost }),
  });
  // The greedy seat stands in for a human, whose random deck ignores the AI's ban.
  const greedyDeck = buildAiDeck(createRng(`${seed}:deck:${greedySeat}`), easy.deckSize, {
    banned: [],
    manaCap: easy.manaCap,
  });

  const ai = { kind: "ai", budget: AI_GATE_BUDGET } as const;
  const greedy = { kind: "greedy" } as const;
  return {
    aiSeat,
    config: {
      seed,
      decks: aiSeat === "p1" ? [aiDeck, greedyDeck] : [greedyDeck, aiDeck],
      handicaps: aiSeat === "p1" ? { p1: handicap, p2: easy } : { p1: easy, p2: handicap },
      controllers: aiSeat === "p1" ? { p1: ai, p2: greedy } : { p1: greedy, p2: ai },
      maxActions: AI_SWEEP.maxActions,
    },
  };
}

function emptyStats(defId: string): SweepStats {
  return { defId, games: 0, drawnGames: 0, affordableTurns: 0, plays: 0, errors: 0, timeouts: 0, evalDeltaSum: 0, evalDeltaCount: 0 };
}

/** `into` += `from`, field by field (defId stays). */
function addStats(into: SweepStats, from: SweepStats): void {
  into.games += from.games;
  into.drawnGames += from.drawnGames;
  into.affordableTurns += from.affordableTurns;
  into.plays += from.plays;
  into.errors += from.errors;
  into.timeouts += from.timeouts;
  into.evalDeltaSum += from.evalDeltaSum;
  into.evalDeltaCount += from.evalDeltaCount;
}

function holdsCard(state: GameState, seat: PlayerId, defId: string): boolean {
  return zoneCards(state, seat, "hand").some((card) => card.defId === defId);
}

/**
 * One sweep game, counted for each `tracked` card the AI's deck holds (games 1): drawn (drawnGames
 * 1), the turns it sat in hand affordable (effectiveCost <= mana at the AI's turn start), and its
 * plays (true-state evaluate delta for the playing seat, before → after). The game's errors (throws,
 * rejected or "fallback" AI actions; a game that cannot even be dealt is one) and timeouts (a
 * decision whose `now()` duration > decisionMs, or maxActions hit) come back beside them.
 */
function playSweepGame(
  seed: string,
  defId: string,
  n: number,
  tier: Difficulty,
  filler: Filler,
  tracked: readonly string[],
  now: (() => number) | undefined,
): { cards: SweepStats[]; errors: number; timeouts: number } {
  let setup: { config: MatchConfig; aiSeat: PlayerId };
  try {
    setup = sweepConfig(seed, defId, n, tier, filler);
  } catch {
    // The card cannot even be dealt (not a deck-legal card, or the pool is too small): an error.
    return { cards: [{ ...emptyStats(defId), games: 1 }], errors: 1, timeouts: 0 };
  }
  const { config, aiSeat } = setup;
  const dealt = new Set(aiSeat === "p1" ? config.decks[0] : config.decks[1]);
  const cards = tracked.filter((id) => dealt.has(id)).map((id) => ({ ...emptyStats(id), games: 1 }));
  let errors = 0;
  let timeouts = 0;
  // The last AI turn whose start was already counted, so a turn is counted once however many
  // actions it holds.
  let countedTurn: number | null = null;

  const hooks: MatchHooks = {
    afterAction(before, after, seat, action) {
      for (const stats of cards) {
        if (stats.drawnGames === 0 && (holdsCard(before, aiSeat, stats.defId) || holdsCard(after, aiSeat, stats.defId))) {
          stats.drawnGames = 1;
        }
      }

      // The AI's turn start: the first main-phase state of a new AI turn.
      if (after.result === null && after.active === aiSeat && after.phase === "main" && after.turn !== countedTurn) {
        countedTurn = after.turn;
        const mana = after.players[aiSeat].mana.current;
        const hand = zoneCards(after, aiSeat, "hand");
        for (const stats of cards) {
          if (hand.some((card) => card.defId === stats.defId && effectiveCost(after, card) <= mana)) stats.affordableTurns += 1;
        }
      }

      if (seat === aiSeat && action.type === "play") {
        const card = zoneCards(before, aiSeat, "hand").find((instance) => instance.id === action.instanceId);
        const stats = cards.find((entry) => entry.defId === card?.defId);
        if (stats !== undefined) {
          stats.plays += 1;
          stats.evalDeltaSum += evaluate(after, aiSeat) - evaluate(before, aiSeat);
          stats.evalDeltaCount += 1;
        }
      }
    },
  };
  if (now !== undefined) {
    hooks.timeDecision = <T>(seat: PlayerId, run: () => T): T => {
      const started = now();
      const result = run();
      if (seat === aiSeat && now() - started > AI_SWEEP.decisionMs) timeouts += 1;
      return result;
    };
  }

  try {
    const record = playMatch(config, hooks);
    errors += record.thrown.length + record.rejected.length + record.fallbacks;
    // A game still running at maxActions is a timeout; one a throw ended is already an error.
    if (record.result === null && record.thrown.length === 0) timeouts += 1;
  } catch {
    errors += 1;
  }
  return { cards, errors, timeouts };
}

/**
 * Pass 1: `seeds` games (default seedsPerCard) of an AI on `tier`'s handicap (default Easy) whose
 * deck includes `defId` against greedy on Easy's, at AI_GATE_BUDGET, on seeds
 * `sweep:<tier>:<id>:<n>`. Every other banned card stays out of the AI's deck, so its errors are not
 * charged to this one.
 */
export function sweepCard(
  defId: string,
  options: { seeds?: number; now?: () => number; tier?: Difficulty } = {},
): SweepResult {
  const seeds = options.seeds ?? AI_SWEEP.seedsPerCard;
  const tier = options.tier ?? "easy";
  const stats = emptyStats(defId);
  const filler: Filler = { banned: Object.keys(SHADOW_BAN) };

  for (let n = 1; n <= seeds; n += 1) {
    const game = playSweepGame(`sweep:${tier}:${defId}:${n}`, defId, n, tier, filler, [defId], options.now);
    addStats(stats, { ...(game.cards[0] ?? { ...emptyStats(defId), games: 1 }), errors: game.errors, timeouts: game.timeouts });
  }

  return { ...stats, tier, flags: sweepFlags(stats), unswept: stats.affordableTurns === 0 };
}

/**
 * Pass 2 (R390): `seeds` games (default seedsPerCardAtRisk) of `defId`, one of the `atRisk` cards, on
 * seeds `sweep2:<tier>:<id>:<n>`. Every at-risk card's filler weight is multiplied by atRiskBoost,
 * `keepOut` (`pass2KeepOut`) is never filler, and every at-risk card the AI was dealt is counted.
 */
export function sweepAtRisk(
  defId: string,
  atRisk: readonly string[],
  keepOut: readonly string[],
  options: { seeds?: number; now?: () => number; tier?: Difficulty } = {},
): SweepPass2 {
  const seeds = options.seeds ?? AI_SWEEP.seedsPerCardAtRisk;
  const tier = options.tier ?? "easy";
  const filler: Filler = { banned: keepOut, boost: { ids: atRisk, by: AI_SWEEP.atRiskBoost } };
  const tracked = [defId, ...atRisk.filter((id) => id !== defId)];
  const totals = new Map<string, SweepStats>([[defId, emptyStats(defId)]]);
  const suspects: SweepSuspect[] = [];

  for (let n = 1; n <= seeds; n += 1) {
    const seed = `sweep2:${tier}:${defId}:${n}`;
    const game = playSweepGame(seed, defId, n, tier, filler, tracked, options.now);
    for (const card of game.cards) {
      const forced = card.defId === defId;
      const total = totals.get(card.defId) ?? emptyStats(card.defId);
      totals.set(card.defId, total);
      addStats(total, { ...card, errors: forced ? game.errors : 0, timeouts: forced ? game.timeouts : 0 });
      if (!forced && game.errors + game.timeouts > 0) {
        suspects.push({ defId: card.defId, seed, forced: defId, errors: game.errors, timeouts: game.timeouts });
      }
    }
  }

  return { forced: defId, tier, games: seeds, cards: [...totals.values()], suspects };
}

/** R390: a card's pass-2 numbers at one tier, summed over every game it was dealt in, forced or filler. */
export function pass2Stats(pass2: readonly SweepPass2[], defId: string, tier: Difficulty): SweepStats {
  const total = emptyStats(defId);
  for (const result of pass2) {
    if (result.tier !== tier) continue;
    for (const card of result.cards) if (card.defId === defId) addStats(total, card);
  }
  return total;
}

/** What one flag measured, for the ban reason. */
function flagDetail(stats: SweepStats, flag: SweepFlag, pass2: boolean): string {
  const over = pass2 ? ` over ${String(stats.games)} pass-2 games` : "";
  switch (flag) {
    case "error":
      return `${String(stats.errors)} engine or search error(s) over ${String(stats.games)} games`;
    case "timeout":
      return `${String(stats.timeouts)} decision(s) over ${String(AI_SWEEP.decisionMs)} ms or game(s) past ${String(AI_SWEEP.maxActions)} actions`;
    case "neverPlayed":
      return `affordable in hand on ${String(stats.affordableTurns)} turns${over}, never played`;
    case "selfHarm":
      return `mean evaluate change ${mean(stats).toFixed(1)} over ${String(stats.evalDeltaCount)} play(s)${over}`;
  }
}

/** A card's numbers in a few words, for a SHADOW_WATCH entry. */
function numbersOf(stats: SweepStats): string {
  const played = stats.plays === 0 ? "never played" : `played ${String(stats.plays)} time(s)`;
  const harm = stats.evalDeltaCount === 0 ? "" : `, mean evaluate change ${mean(stats).toFixed(1)}`;
  return `affordable on ${String(stats.affordableTurns)} turns over ${String(stats.games)} games, ${played}${harm}`;
}

/**
 * One card's verdict over its tiers and passes (R186, R390). `pass1` is the card's own pass-1
 * results; `pass2` may be every pass-2 result of the sweep, since the card's numbers are read out of
 * each, forced or filler. Per tier: `error` and `timeout` from the card's own games of either pass;
 * `neverPlayed` from pass 2's numbers alone, banAffordableTurns affordable turns and no play;
 * `selfHarm` from pass 2's, at least banHarmPlays plays averaging below selfHarmDelta. A flag at any
 * tier bans the card, every flagging tier named. Unswept when no tier of either pass ever saw it
 * affordable. Watched (R600) when it is not banned and its own numbers this sweep, of either pass,
 * meet a flag at half strength.
 */
export function sweepVerdict(pass1: readonly SweepResult[], pass2: readonly SweepPass2[] = []): SweepVerdict {
  const defId = pass1[0]?.defId ?? pass2[0]?.forced ?? "";
  const tiers = [...new Set([...pass1.map((result) => result.tier), ...pass2.map((result) => result.tier)])];
  const flagged = new Set<SweepFlag>();
  const details: string[] = [];
  const watched: string[] = [];
  let affordable = 0;

  for (const tier of tiers) {
    const first = emptyStats(defId);
    for (const result of pass1) if (result.tier === tier) addStats(first, result);
    const second = pass2Stats(pass2, defId, tier);
    affordable += first.affordableTurns + second.affordableTurns;

    const both = { ...first };
    addStats(both, second);
    const parts: string[] = [];
    for (const flag of sweepFlags({ ...emptyStats(defId), games: both.games, errors: both.errors, timeouts: both.timeouts })) {
      flagged.add(flag);
      parts.push(flagDetail(both, flag, false));
    }
    if (second.affordableTurns >= AI_SWEEP.banAffordableTurns && second.plays === 0) {
      flagged.add("neverPlayed");
      parts.push(flagDetail(second, "neverPlayed", true));
    }
    if (second.evalDeltaCount >= AI_SWEEP.banHarmPlays && mean(second) < AI_SWEEP.selfHarmDelta) {
      flagged.add("selfHarm");
      parts.push(flagDetail(second, "selfHarm", true));
    }
    if (parts.length > 0) details.push(`${tier}: ${parts.join(", ")}`);

    const onTrack: string[] = [];
    if (halfFlags(first).length > 0) onTrack.push(`pass 1 ${numbersOf(first)}`);
    if (halfFlags(second).length > 0) onTrack.push(`pass 2 ${numbersOf(second)}`);
    if (onTrack.length > 0) watched.push(`${tier}: ${onTrack.join("; ")}`);
  }

  const flags = FLAG_ORDER.filter((flag) => flagged.has(flag));
  const reason = flags.length === 0 ? null : `${flags.join(", ")}: ${details.join("; ")}`;
  return {
    defId,
    flags,
    unswept: tiers.length > 0 && affordable === 0,
    reason,
    watch: reason === null && watched.length > 0 ? `at risk: ${watched.join("; ")}` : null,
  };
}
