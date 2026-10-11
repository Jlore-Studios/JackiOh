// The web client's own copy of the shared wire helper (docs/v0.3.0/SURFACE.md §10.4), kept as
// TypeScript and unchanged but for its import paths; the server's port is crates/engine/src/wire/stats.rs.
//
// Card statistics (SPEC §9.11, R376–R378): the record a finished game leaves, and the card win rates
// read off a set of those records.
//
// Two writers make records. The server writes one for every live match that ends, once its result
// is in (`crates/server/src/api/game_records.rs`), and the AI's development runs write one for every
// game they play (`crates/ai/src/dev_run.rs`). Both take the `game` half from the engine's
// `summarizeGame`, which reads it off `(seed, decks, log)`. Everything below is pure data over those
// records, shared by the server's report and import scripts and the AI's run, so the numbers the
// three print are one computation.

import { PLAYER_IDS, type PlayerId } from "./catalog.ts";
import type { GameOverReason } from "./generated/GameOverReason.ts";

// ---------------------------------------------------------------------------
// The record (R376)
// ---------------------------------------------------------------------------

/**
 * R376, R378: where a game was played. `live` is a match the server ran; `dev` is a game of an
 * internal AI development run, which no live figure includes unless it is asked for.
 */
export const GAME_SOURCES = ["live", "dev"] as const;
export type GameSource = (typeof GAME_SOURCES)[number];

/** R257's three modes: the match type a record is filed under. Conquest is `bo3` on the wire. */
export const GAME_MODES = ["bo1", "bo3", "random"] as const;
export type GameMode = (typeof GAME_MODES)[number];

/** R376: who chose a seat's moves. */
export const PILOTS = ["human", "ai"] as const;
export type Pilot = (typeof PILOTS)[number];

/** §2.5's endings, as `gameOver` names them, for reading a record back. */
export const GAME_OVER_REASONS = [
  "hero-death",
  "both-heroes-dead",
  "concede",
  "draw-accepted",
  "turn-cap",
  "disconnect",
  "match-ceiling",
  "voided",
  "alt-win",
  "won-by-effect",
] as const satisfies readonly GameOverReason[];

type NoneMissing<T extends never> = T;

/** Fails to compile when `GameOverReason` gains a member the list above lacks, as `events.ts` does for its list. */
export type GameOverReasonsAreExhaustive = NoneMissing<Exclude<GameOverReason, (typeof GAME_OVER_REASONS)[number]>>;

/** R376: what one seat's cards did in one game, as catalog ids. */
export type SeatSummary = {
  /** The decklist the seat played. */
  deck: string[];
  /**
   * Its hand once both mulligans had resolved, as the first turn began: the cards it kept, its
   * replacements and, for the seat going second, The Coin (§2.1). Empty when the game ended before
   * a turn began.
   */
  opening: string[];
  /** Every card it drew into its hand after that, in order. A card burned or cast on its draw never reached the hand. */
  drawn: string[];
  /** Every card it played from its hand, in order: a cast (§2.4, R70) is not a play from hand. */
  played: string[];
  /** Turn number (1-indexed) each card in `played` was played on. */
  playedTurns?: number[];
};

/** R376: a finished game, read off its replay by the engine's `summarizeGame`. */
export type GameSummary = {
  /** The seat that took the first turn (§2.1: Player 1). */
  first: PlayerId;
  winner: PlayerId | "draw";
  reason: GameOverReason;
  /** The player-turn counter at the end (§2.5). */
  turns: number;
  seats: Record<PlayerId, SeatSummary>;
};

/** R378: how a development record's id begins, and a live one's (a match id) never does. */
export const DEV_RECORD_ID_PREFIX = "dev:";

/** R376: one game, filed by where, how and by whom it was played. */
export type GameRecord = {
  /** Unique among records: a live game's match id, a development game's `dev:<patch>:<series>:<n>`. */
  id: string;
  source: GameSource;
  mode: GameMode;
  /** R388's version of the cards the game was played with: the newest patch of the build that played it. */
  patch: string;
  pilots: Record<PlayerId, Pilot>;
  game: GameSummary;
};

// ---------------------------------------------------------------------------
// Which records a query reads (R377, R378)
// ---------------------------------------------------------------------------

/** R378: live data unless a development run is asked for by name; `all` is both. */
export const SOURCE_FILTERS = ["live", "dev", "all"] as const;
export type SourceFilter = (typeof SOURCE_FILTERS)[number];

/** R377: the seats a query counts. `unified` is every seat, human and AI combined. */
export const PILOT_FILTERS = ["human", "ai", "unified"] as const;
export type PilotFilter = (typeof PILOT_FILTERS)[number];

/** Any combination of match type, patch and pilot, over live data, development data or both. */
export type CardStatsFilter = {
  source: SourceFilter;
  /** One match type, or null for every mode. */
  mode: GameMode | null;
  /** One patch, or null for every patch. */
  patch: string | null;
  pilot: PilotFilter;
};

/** R378: what a query reads when it names nothing: live games of every mode, patch and pilot. */
export const DEFAULT_CARD_STATS_FILTER: Readonly<CardStatsFilter> = {
  source: "live",
  mode: null,
  patch: null,
  pilot: "unified",
};

/** The record sources a source filter reads. Only `dev` and `all` read a development run. */
export function sourcesOf(source: SourceFilter): GameSource[] {
  return source === "all" ? [...GAME_SOURCES] : [source];
}

/** Whether a record is one the filter reads: its source, its mode and its patch. Pilots are per seat. */
export function recordMatches(record: GameRecord, filter: CardStatsFilter): boolean {
  return (
    sourcesOf(filter.source).includes(record.source) &&
    (filter.mode === null || record.mode === filter.mode) &&
    (filter.patch === null || record.patch === filter.patch)
  );
}

/** The seats of a record the pilot filter counts, in seat order. */
export function seatsCounted(record: GameRecord, pilot: PilotFilter): PlayerId[] {
  return PLAYER_IDS.filter((seat) => pilot === "unified" || record.pilots[seat] === pilot);
}

// ---------------------------------------------------------------------------
// The breakdowns (R377)
// ---------------------------------------------------------------------------

/**
 * Games, wins and draws for one breakdown of one card. A game is one deck in one game, so a game in
 * which both decks held the card counts once for each, and a draw is a game that was not won.
 */
export type Tally = { games: number; wins: number; draws: number };

/**
 * R377's breakdowns, each over the decks that held the card:
 *  - `inDeck`: every one of them;
 *  - `openingHand`: the card was in the opening hand (`SeatSummary.opening`);
 *  - `goingFirst` and `goingSecond`: the deck's seat took the first turn, or did not;
 *  - `played`: the seat played the card from its hand at least once;
 *  - `drawnNotPlayed`: the card reached the seat's hand — in the opening hand or by a later draw —
 *    and the seat never played it. This is the played delta's baseline.
 */
export const BREAKDOWNS = ["inDeck", "openingHand", "goingFirst", "goingSecond", "played", "drawnNotPlayed"] as const;
export type Breakdown = (typeof BREAKDOWNS)[number];

export type CardStats = { card: string } & Record<Breakdown, Tally>;

export type CardStatsReport = {
  filter: CardStatsFilter;
  /** Records the filter read with at least one seat it counts. */
  games: number;
  /** Seats counted across them: the decks every card's figures are drawn from. */
  decks: number;
  /** One entry per card some counted deck held, by catalog id. */
  cards: CardStats[];
};

function emptyTally(): Tally {
  return { games: 0, wins: 0, draws: 0 };
}

function emptyStats(card: string): CardStats {
  return {
    card,
    inDeck: emptyTally(),
    openingHand: emptyTally(),
    goingFirst: emptyTally(),
    goingSecond: emptyTally(),
    played: emptyTally(),
    drawnNotPlayed: emptyTally(),
  };
}

type Outcome = "win" | "draw" | "loss";

function outcomeFor(game: GameSummary, seat: PlayerId): Outcome {
  if (game.winner === "draw") return "draw";
  return game.winner === seat ? "win" : "loss";
}

function count(tally: Tally, outcome: Outcome): void {
  tally.games += 1;
  if (outcome === "win") tally.wins += 1;
  if (outcome === "draw") tally.draws += 1;
}

/**
 * R377: every card's breakdowns over the records the filter reads. A card is counted by its catalog
 * id for the decks that held it, so a copy of it made in play counts as the card for those decks and
 * a card no counted deck held has no entry. `cards` lists the ids in catalog-id order.
 */
export function cardStats(records: readonly GameRecord[], filter: CardStatsFilter = DEFAULT_CARD_STATS_FILTER): CardStatsReport {
  const byCard = new Map<string, CardStats>();
  let games = 0;
  let decks = 0;

  for (const record of records) {
    if (!recordMatches(record, filter)) continue;
    const seats = seatsCounted(record, filter.pilot);
    if (seats.length === 0) continue;
    games += 1;

    for (const seat of seats) {
      decks += 1;
      const summary = record.game.seats[seat];
      const outcome = outcomeFor(record.game, seat);
      const opening = new Set(summary.opening);
      const inHand = new Set([...summary.opening, ...summary.drawn]);
      const played = new Set(summary.played);
      const first = record.game.first === seat;

      for (const card of new Set(summary.deck)) {
        let stats = byCard.get(card);
        if (stats === undefined) {
          stats = emptyStats(card);
          byCard.set(card, stats);
        }
        count(stats.inDeck, outcome);
        if (opening.has(card)) count(stats.openingHand, outcome);
        count(first ? stats.goingFirst : stats.goingSecond, outcome);
        if (played.has(card)) count(stats.played, outcome);
        else if (inHand.has(card)) count(stats.drawnNotPlayed, outcome);
      }
    }
  }

  const cards = [...byCard.values()].sort((a, b) => (a.card < b.card ? -1 : a.card > b.card ? 1 : 0));
  return { filter: { ...filter }, games, decks, cards };
}

/** Wins over games; null when there are no games, so an empty breakdown never reads as 0%. */
export function winRate(tally: Tally): number | null {
  return tally.games === 0 ? null : tally.wins / tally.games;
}

/**
 * R377: the played delta, the played win rate minus the drawn-but-not-played win rate, as a fraction
 * (0.082 is 8.2 points). Null when either side has no games.
 */
export function playedDelta(stats: CardStats): number | null {
  const played = winRate(stats.played);
  const baseline = winRate(stats.drawnNotPlayed);
  return played === null || baseline === null ? null : played - baseline;
}

// ---------------------------------------------------------------------------
// The report as text
// ---------------------------------------------------------------------------

/** A rate or a delta prints with this many decimal places: "52.1%", "+8.2". Presentation only. */
const DECIMALS = 1;
/** A fraction as a percentage. */
const PERCENT = 100;
/** What a breakdown with no games, or a delta it cannot compute, prints. */
const NO_FIGURE = "—";

const MODE_NAMES: Readonly<Record<GameMode, string>> = { bo1: "Best of 1", bo3: "Conquest", random: "All Random" };
const SOURCE_NAMES: Readonly<Record<SourceFilter, string>> = {
  live: "live games",
  dev: "AI development runs",
  all: "live games and AI development runs",
};
const PILOT_NAMES: Readonly<Record<PilotFilter, string>> = {
  human: "human pilots",
  ai: "AI pilots",
  unified: "human and AI pilots (unified)",
};

/** The column headings, in `BREAKDOWNS` order. */
export const BREAKDOWN_TITLES: Readonly<Record<Breakdown, string>> = {
  inDeck: "In deck",
  openingHand: "Opening hand",
  goingFirst: "Going first",
  goingSecond: "Going second",
  played: "Played",
  drawnNotPlayed: "Drawn, not played",
};

/** One line naming what a filter read. */
export function describeFilter(filter: CardStatsFilter): string {
  const mode = filter.mode === null ? "every mode" : MODE_NAMES[filter.mode];
  const patch = filter.patch === null ? "every patch" : `patch ${filter.patch}`;
  return `${SOURCE_NAMES[filter.source]}, ${mode}, ${patch}, ${PILOT_NAMES[filter.pilot]}`;
}

/** "52.1% (310)": the win rate with its sample size, the games behind it, always beside it. */
export function formatTally(tally: Tally): string {
  const rate = winRate(tally);
  const shown = rate === null ? NO_FIGURE : `${(rate * PERCENT).toFixed(DECIMALS)}%`;
  return `${shown} (${String(tally.games)})`;
}

/** "+8.2": the played delta in percentage points. */
export function formatDelta(delta: number | null): string {
  if (delta === null) return NO_FIGURE;
  const points = (delta * PERCENT).toFixed(DECIMALS);
  return delta > 0 ? `+${points}` : points;
}

/**
 * The report as a plain-text table: one row per card, each breakdown's win rate with its games
 * beside it, and the played delta. `nameOf` turns an id into the name shown after it.
 */
export function formatCardStats(report: CardStatsReport, nameOf: (card: string) => string | undefined = () => undefined): string {
  const header = ["Card", ...BREAKDOWNS.map((breakdown) => BREAKDOWN_TITLES[breakdown]), "Played Δ"];
  const rows = report.cards.map((stats) => {
    const name = nameOf(stats.card);
    return [
      name === undefined ? stats.card : `${stats.card} ${name}`,
      ...BREAKDOWNS.map((breakdown) => formatTally(stats[breakdown])),
      formatDelta(playedDelta(stats)),
    ];
  });
  const widths = header.map((title, column) => Math.max(title.length, ...rows.map((row) => (row[column] ?? "").length)));
  const line = (cells: readonly string[]): string =>
    cells
      .map((cell, column) => (column === 0 ? cell.padEnd(widths[column] ?? 0) : cell.padStart(widths[column] ?? 0)))
      .join("  ")
      .trimEnd();

  return [
    `Card win rates: ${describeFilter(report.filter)}.`,
    `${String(report.games)} games, ${String(report.decks)} decks.`,
    "Each cell is the win rate of the decks that held the card, with those games in brackets. A game counts once for",
    "each deck that held the card, and a draw is a game not won. Played Δ is the played win rate minus the drawn-but-",
    "not-played win rate, in points: drawn means the card reached the hand, in the opening hand or by a later draw.",
    "",
    line(header),
    ...rows.map(line),
  ].join("\n");
}

// ---------------------------------------------------------------------------
// Reading a record back (the import script, the AI run's own file)
// ---------------------------------------------------------------------------

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function ids(value: unknown, where: string): string[] {
  if (!Array.isArray(value) || !value.every((item) => typeof item === "string" && item.length > 0)) {
    throw new Error(`${where} is not a list of card ids`);
  }
  return [...(value as string[])];
}

function oneOf<T extends string>(value: unknown, allowed: readonly T[], where: string): T {
  if (typeof value !== "string" || !(allowed as readonly string[]).includes(value)) {
    throw new Error(`${where} is not one of ${allowed.join(", ")}`);
  }
  return value as T;
}

function text(value: unknown, where: string): string {
  if (typeof value !== "string" || value.length === 0) throw new Error(`${where} is not a non-empty string`);
  return value;
}

function seatSummary(value: unknown, where: string): SeatSummary {
  if (!isRecord(value)) throw new Error(`${where} is not a seat summary`);
  const summary: SeatSummary = {
    deck: ids(value["deck"], `${where}.deck`),
    opening: ids(value["opening"], `${where}.opening`),
    drawn: ids(value["drawn"], `${where}.drawn`),
    played: ids(value["played"], `${where}.played`),
  };
  const turns = value["playedTurns"];
  if (Array.isArray(turns)) {
    summary.playedTurns = turns.map((t, idx) => {
      if (typeof t !== "number" || !Number.isInteger(t) || t < 1) {
        throw new Error(`${where}.playedTurns[${String(idx)}] is not a positive whole number`);
      }
      return t;
    });
  }
  return summary;
}

/** A record read back from JSON, checked field by field. Throws an error naming the first bad field. */
export function parseGameRecord(value: unknown): GameRecord {
  if (!isRecord(value)) throw new Error("a game record is an object");
  const pilots = value["pilots"];
  const game = value["game"];
  if (!isRecord(pilots)) throw new Error("pilots is not an object");
  if (!isRecord(game)) throw new Error("game is not an object");
  const seats = game["seats"];
  if (!isRecord(seats)) throw new Error("game.seats is not an object");
  const turns = game["turns"];
  if (typeof turns !== "number" || !Number.isInteger(turns) || turns < 0) {
    throw new Error("game.turns is not a whole number");
  }
  return {
    id: text(value["id"], "id"),
    source: oneOf(value["source"], GAME_SOURCES, "source"),
    mode: oneOf(value["mode"], GAME_MODES, "mode"),
    patch: text(value["patch"], "patch"),
    pilots: { p1: oneOf(pilots["p1"], PILOTS, "pilots.p1"), p2: oneOf(pilots["p2"], PILOTS, "pilots.p2") },
    game: {
      first: oneOf(game["first"], PLAYER_IDS, "game.first"),
      winner: oneOf(game["winner"], [...PLAYER_IDS, "draw"] as const, "game.winner"),
      reason: oneOf(game["reason"], GAME_OVER_REASONS, "game.reason"),
      turns,
      seats: { p1: seatSummary(seats["p1"], "game.seats.p1"), p2: seatSummary(seats["p2"], "game.seats.p2") },
    },
  };
}

/** Records written one JSON object per line, as a development run writes them. Blank lines are skipped. */
export function parseGameRecordLines(contents: string): GameRecord[] {
  const records: GameRecord[] = [];
  contents.split("\n").forEach((line, index) => {
    if (line.trim() === "") return;
    try {
      records.push(parseGameRecord(JSON.parse(line) as unknown));
    } catch (error) {
      throw new Error(`line ${String(index + 1)}: ${error instanceof Error ? error.message : String(error)}`, {
        cause: error,
      });
    }
  });
  return records;
}
