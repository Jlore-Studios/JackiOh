// R1584000: the stats the floor below the tavern's showcase shows, one at a time. A stat the device
// holds no data for is left out. This module only chooses and words them; it takes the stage's types
// and nothing else of it, so the effects layer stays off the first screen until the showcase is drawn.
//
// R1743000: a stat's card is never a token, nor one the catalog no longer holds (R2649000: 65 stats).
// The order is a shuffled bag (`nextInBag`) drawn from the page's random source.
//
// R2648000: a stat names the right kind of card (Unbreakable, Glass Cannon and Workhorse a Unit), and
// `animationFor` fits a lunge to the card's type. R2649000's thirty are read off `track.ts`'s counters.

import catalogJson from "@jackioh/cards/catalog.json";
import { hasKeyword, type CardDefs, type KeywordKind } from "@jackioh/shared";

import { t } from "../i18n/i18n.ts";
import type { Side } from "../game/contract.ts";
import { ROTATION_POOL, below, shuffled, type RandomSource } from "../routes/landingFan.ts";
import { SHOWCASE_MIN_PLAYS } from "./config.ts";
import { NO_COUNTS, winPercent, type CardCounter, type CardCounters, type CardTally, type PlayerStats } from "./model.ts";
import type { PocketPick } from "./pocketPick.ts";
import type { StageAnimation } from "./showcaseStage.ts";

const CATALOG = catalogJson as unknown as CardDefs;

/** The stats, the ten of R1584000 first, R1743000's twenty-five after, then R2649000's thirty. */
export const SHOWCASE_KEYS = [
  "toppled",
  "coveted",
  "pocketPick",
  "faithful",
  "nemesis",
  "fallen",
  "record",
  "lastLaugh",
  "springloaded",
  "survivor",
  "glassCannon",
  "unbreakable",
  "grudgeMatch",
  "neverTried",
  "easyTarget",
  "closer",
  "hairTrigger",
  "rideOrDie",
  "familiarFace",
  "oneHitWonder",
  "secondFiddle",
  "bronzeMedal",
  "explorer",
  "collector",
  "workhorse",
  "heavyHitter",
  "overkill",
  "brawler",
  "medic",
  "cardShark",
  "shineOn",
  "luckyCharm",
  "jinx",
  "hotStreak",
  "marathon",
  "useItOrLoseIt",
  "justInTime",
  "sandCastle",
  "outlasted",
  "goodFengShui",
  "badFengShui",
  "hotPotato",
  "tradeInValue",
  "massProduced",
  "executioner",
  "giantSlayer",
  "secondWind",
  "bubbleWrap",
  "ironWall",
  "punchingBag",
  "doubleTap",
  "vampire",
  "martyr",
  "coldSnap",
  "brainFreeze",
  "springThaw",
  "shatter",
  "encore",
  "bargainHunter",
  "denied",
  "nope",
  "burntToast",
  "frequentFlyer",
  "liftoff",
  "brick",
] as const;
export type ShowcaseKey = (typeof SHOWCASE_KEYS)[number];

export type ShowcaseStat = {
  readonly key: ShowcaseKey;
  /** The card the stat is about, one the catalog holds. */
  readonly cardId: string;
  /** The stat's name and its sentence say themselves when they are called, never at import. */
  readonly title: () => string;
  readonly line: () => string;
  /** The board animation the card plays, and the side whose lane it stands in. */
  readonly animation: StageAnimation;
  readonly side: Side;
};

/** The record line: "12 games: 7 won, 4 lost, 1 drawn (58% won)". */
export function recordLine(stats: PlayerStats): string {
  const percent = winPercent(stats);
  const games = stats.games === 1 ? t("{n} game", { n: stats.games }) : t("{n} games", { n: stats.games });
  const counts = { games, won: stats.wins, lost: stats.losses, drawn: stats.draws, percent: percent ?? 0 };
  if (stats.draws > 0) {
    return percent === null
      ? t("{games}: {won} won, {lost} lost, {drawn} drawn", counts)
      : t("{games}: {won} won, {lost} lost, {drawn} drawn ({percent}% won)", counts);
  }
  return percent === null
    ? t("{games}: {won} won, {lost} lost", counts)
    : t("{games}: {won} won, {lost} lost ({percent}% won)", counts);
}

/** R1583000: "you play it 4× as often as everyone else", the ratio to a tenth; "nobody else plays it" at a base rate of 0. */
export function pocketLine(pick: PocketPick): string {
  if (pick.baseRate === 0) return t("nobody else plays it");
  return t("you play it {n}× as often as everyone else", { n: Math.round((pick.playerRate / pick.baseRate) * 10) / 10 });
}

function showable(id: string): boolean {
  const def = CATALOG[id];
  return def !== undefined && !def.token;
}

/** R2648000: the card's catalog type is Unit. */
function isUnit(id: string): boolean {
  return CATALOG[id]?.type === "Unit";
}

/** R2649000: the card's catalog type is Spell or Field Spell. */
function isSpell(id: string): boolean {
  const type = CATALOG[id]?.type;
  return type === "Spell" || type === "Field Spell";
}

/** R2648000: either face of the card prints one of `kinds`. */
function prints(id: string, ...kinds: KeywordKind[]): boolean {
  const def = CATALOG[id];
  return def !== undefined && kinds.some((kind) => hasKeyword(def.base.keywords, kind) || hasKeyword(def.radiant.keywords, kind));
}

/** R2648000: a lunge is a Unit's or an Animated card's; a Spell or Field Spell casts, a Trap or Field Trap is revealed. Anything else is kept. */
export function animationFor(defId: string, wanted: StageAnimation): StageAnimation {
  const type = CATALOG[defId]?.type;
  if (wanted !== "lunge" || type === undefined || type === "Unit" || prints(defId, "Animated", "Animated on your turn")) return wanted;
  return type === "Trap" || type === "Field Trap" ? "trap" : "cast";
}

function byId(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

/** The showable cards with a count above 0, the most first, a tie going to the lower id (`topCards`'s order). */
function ranked(counts: Readonly<Record<string, number>>): CardTally[] {
  return Object.entries(counts)
    .filter(([id, count]) => count > 0 && showable(id))
    .map(([id, count]) => ({ id, count }))
    .sort((a, b) => b.count - a.count || byId(a.id, b.id));
}

/** Each card's score by id. */
function column(stats: PlayerStats, score: (counts: CardCounters, id: string) => number): Record<string, number> {
  return Object.fromEntries(Object.entries(stats.cards).map(([id, counts]) => [id, score(counts, id)]));
}

/** The showable card with the most of one counter, or, with `at`, the one that many places below it. */
function topCard(stats: PlayerStats, counter: CardCounter, at = 0): CardTally | undefined {
  return ranked(column(stats, (counts) => counts[counter]))[at];
}

type Share = { readonly id: string; readonly part: number; readonly whole: number };

/**
 * R1743000: the showable card with the highest `part` per `whole` (a `whole` of at least SHOWCASE_MIN_PLAYS,
 * a `part` above 0), the part capped at the whole; a tie goes to the lower id.
 */
function topShare(
  stats: PlayerStats,
  part: (counts: CardCounters, id: string) => number,
  whole: (counts: CardCounters) => number,
): Share | undefined {
  let best: Share | undefined;
  for (const [id, counts] of Object.entries(stats.cards).sort(([a], [b]) => byId(a, b))) {
    const total = whole(counts);
    const share = Math.min(part(counts, id), total);
    if (!showable(id) || total < SHOWCASE_MIN_PLAYS || share === 0) continue;
    if (best === undefined || share * best.whole > best.part * total) best = { id, part: share, whole: total };
  }
  return best;
}

/** R1743000: one of `ids`, by the page's random source, none (and no draw) when there are none. */
function drawOne(ids: readonly string[], random: RandomSource): string | undefined {
  return ids.length === 0 ? undefined : [...ids].sort(byId)[below(random, ids.length)];
}

type StatMeta = Omit<ShowcaseStat, "cardId" | "line">;

function tallied(top: CardTally | undefined, stat: StatMeta, line: (n: number) => string): ShowcaseStat[] {
  return top === undefined ? [] : [{ ...stat, cardId: top.id, line: () => line(top.count) }];
}

function counted(stats: PlayerStats, counter: CardCounter, stat: StatMeta, line: (n: number) => string): ShowcaseStat[] {
  return tallied(topCard(stats, counter), stat, line);
}

function shared(share: Share | undefined, stat: StatMeta, line: (part: number, whole: number) => string): ShowcaseStat[] {
  return share === undefined ? [] : [{ ...stat, cardId: share.id, line: () => line(share.part, share.whole) }];
}

/** A stat about a card whose line states its own numbers. */
function about(id: string | undefined, stat: StatMeta, line: () => string): ShowcaseStat[] {
  return id === undefined ? [] : [{ ...stat, cardId: id, line }];
}

function countsOf(stats: PlayerStats, id: string): CardCounters {
  return stats.cards[id] ?? NO_COUNTS;
}

/** R1584000: the Pocket Pick once its request answers with a showable card; none before a game. */
export function pocketStat(stats: PlayerStats, pocket: PocketPick | null): ShowcaseStat[] {
  if (stats.games === 0 || pocket === null || !showable(pocket.id)) return [];
  return [
    {
      key: "pocketPick",
      cardId: pocket.id,
      title: () => t("Pocket Pick"),
      line: () => pocketLine(pocket),
      animation: "trap",
      side: "you",
    },
  ];
}

/**
 * One stat for each the device has data for, none before the first finished game. The Pocket Pick
 * (R1583000) may come after the draw through `pocketStat`, so its answer redraws nothing. R1743000: each
 * stat that picks at random draws once from `random`, only with a candidate.
 */
export function showcaseStats(stats: PlayerStats, pocket: PocketPick | null, random: RandomSource): ShowcaseStat[] {
  if (stats.games === 0) return [];
  const played = ranked(column(stats, (counts) => counts.played));
  const ids = (keep: (counts: CardCounters) => boolean): string[] =>
    Object.entries(stats.cards)
      .filter(([id, counts]) => showable(id) && keep(counts))
      .map(([id]) => id);
  const seenInPool = ROTATION_POOL.filter((def) => countsOf(stats, def.id).seen > 0).map((def) => def.id);
  const seen = topCard(stats, "seen");
  const grudge = ranked(column(stats, (c) => Math.min(c.played, c.playedAgainst)))[0];
  const faced = countsOf(stats, grudge?.id ?? "");
  const explored = ids((c) => c.played > 0);
  const longest = ranked(stats.longestGamePlayed)[0] ?? played[0];
  const nope = topCard(stats, "counterspells");
  const brick = ranked(column(stats, (c) => (c.drawn >= SHOWCASE_MIN_PLAYS ? c.drawn - c.played : 0)))[0];
  const bricked = countsOf(stats, brick?.id ?? "");
  const out: ShowcaseStat[] = [
    ...counted(stats, "defeated", { key: "toppled", title: () => t("Most Toppled"), animation: "death", side: "opponent" }, (n) =>
      t("taken down {n}×", { n }),
    ),
    ...counted(stats, "allured", { key: "coveted", title: () => t("Most Coveted"), animation: "allure", side: "you" }, (n) =>
      t("stolen {n}×", { n }),
    ),
    ...pocketStat(stats, pocket),
    ...counted(stats, "played", { key: "faithful", title: () => t("Old Faithful"), animation: "entrance", side: "you" }, (n) =>
      t("played {n}×", { n }),
    ),
    ...counted(stats, "playedAgainst", { key: "nemesis", title: () => t("Arch-Nemesis"), animation: "lunge", side: "opponent" }, (n) =>
      t("played against you {n}×", { n }),
    ),
    ...counted(stats, "destroyed", { key: "fallen", title: () => t("Fallen Hero"), animation: "death", side: "you" }, (n) =>
      t("lost {n}×", { n }),
    ),
    ...about(
      ranked(column(stats, (c) => c.playedInWin + c.playedInLoss))[0]?.id,
      { key: "record", title: () => t("The Record"), animation: "radiant", side: "you" },
      () => recordLine(stats),
    ),
    ...counted(stats, "lethal", { key: "lastLaugh", title: () => t("Last Laugh"), animation: "lunge", side: "you" }, (n) =>
      t("dealt the winning blow {n}×", { n }),
    ),
    ...counted(stats, "sprung", { key: "springloaded", title: () => t("Springloaded"), animation: "trap", side: "you" }, (n) =>
      t("sprung {n}×", { n }),
    ),
    ...counted(stats, "survived", { key: "survivor", title: () => t("Survivor"), animation: "shield", side: "you" }, (n) =>
      t("standing at the end of a win {n}×", { n }),
    ),
    // R1743000: from the counters the device already kept.
    ...shared(
      topShare(stats, (c, id) => (isUnit(id) ? c.destroyed : 0), (c) => c.played),
      { key: "glassCannon", title: () => t("Glass Cannon"), animation: "death", side: "you" },
      (n, m) => t("destroyed {n} of the {m} times you played it", { n, m }),
    ),
    ...tallied(
      ranked(
        column(stats, (c, id) =>
          c.destroyed === 0 && c.played >= SHOWCASE_MIN_PLAYS && isUnit(id) && !prints(id, "Indestructible") ? c.played : 0,
        ),
      )[0],
      { key: "unbreakable", title: () => t("Unbreakable"), animation: "shield", side: "you" },
      (n) => t("played {n}×, never destroyed", { n }),
    ),
    ...about(grudge?.id, { key: "grudgeMatch", title: () => t("Grudge Match"), animation: "lunge", side: "you" }, () =>
      t("you {n}×, them {m}×", { n: faced.played, m: faced.playedAgainst }),
    ),
    ...tallied(
      ranked(column(stats, (c) => (c.played === 0 ? c.playedAgainst : 0)))[0],
      { key: "neverTried", title: () => t("Never Tried It"), animation: "lunge", side: "opponent" },
      (n) => t("faced {n}×, never played it yourself", { n }),
    ),
    ...shared(
      topShare(stats, (c) => c.defeated, (c) => c.playedAgainst),
      { key: "easyTarget", title: () => t("Easy Target"), animation: "death", side: "opponent" },
      (n, m) => t("taken down {n} of the {m} times you faced it", { n, m }),
    ),
    ...shared(
      topShare(stats, (c) => c.lethal, (c) => c.played),
      { key: "closer", title: () => t("The Closer"), animation: "lunge", side: "you" },
      (n, m) => t("the winning blow in {n} of {m} plays", { n, m }),
    ),
    ...shared(
      topShare(stats, (c) => c.sprung, (c) => c.played),
      { key: "hairTrigger", title: () => t("Hair Trigger"), animation: "trap", side: "you" },
      (n, m) => t("sprung {n} of the {m} times you set it", { n, m }),
    ),
    ...shared(
      topShare(stats, (c) => c.survived, (c) => c.playedInWin),
      { key: "rideOrDie", title: () => t("Ride or Die"), animation: "shield", side: "you" },
      (n, m) => t("still standing at the end of {n} of the {m} wins you played it in", { n, m }),
    ),
    ...tallied(seen, { key: "familiarFace", title: () => t("Familiar Face"), animation: "radiant", side: "you" }, (n) =>
      stats.games === 1 ? t("in {n} of your {m} game", { n, m: stats.games }) : t("in {n} of your {m} games", { n, m: stats.games }),
    ),
    ...about(
      drawOne(ids((c) => c.played === 1), random),
      { key: "oneHitWonder", title: () => t("One-Hit Wonder"), animation: "entrance", side: "you" },
      () => t("played once. Just once."),
    ),
    ...tallied(played[1], { key: "secondFiddle", title: () => t("Second Fiddle"), animation: "entrance", side: "you" }, (n) =>
      t("your #2, played {n}×", { n }),
    ),
    ...tallied(played[2], { key: "bronzeMedal", title: () => t("Bronze Medal"), animation: "entrance", side: "you" }, (n) =>
      t("your #3, played {n}×", { n }),
    ),
    ...about(drawOne(explored, random), { key: "explorer", title: () => t("Explorer"), animation: "entrance", side: "you" }, () =>
      explored.length === 1 ? t("{n} different card played", { n: 1 }) : t("{n} different cards played", { n: explored.length }),
    ),
    ...about(
      drawOne(seenInPool, random),
      { key: "collector", title: () => t("Collector"), animation: "radiant", side: "you" },
      () => t("seen {n} of {m} cards", { n: seenInPool.length, m: ROTATION_POOL.length }),
    ),
    // R1743000: from the counters read off the views' events.
    ...tallied(
      ranked(column(stats, (c, id) => (isUnit(id) ? c.damageDealt : 0)))[0],
      { key: "workhorse", title: () => t("Workhorse"), animation: "lunge", side: "you" },
      (n) => t("dealt {n} damage", { n }),
    ),
    ...counted(stats, "biggestHit", { key: "heavyHitter", title: () => t("Heavy Hitter"), animation: "lunge", side: "you" }, (n) =>
      t("hit for {n} once", { n }),
    ),
    ...counted(stats, "overkill", { key: "overkill", title: () => t("Overkill"), animation: "lunge", side: "you" }, (n) =>
      t("hit {n} harder than it needed to", { n }),
    ),
    ...counted(stats, "attacks", { key: "brawler", title: () => t("Brawler"), animation: "lunge", side: "you" }, (n) =>
      t("attacked {n}×", { n }),
    ),
    ...counted(stats, "healing", { key: "medic", title: () => t("Medic"), animation: "shield", side: "you" }, (n) =>
      t("healed {n}", { n }),
    ),
    ...counted(stats, "drawn", { key: "cardShark", title: () => t("Card Shark"), animation: "entrance", side: "you" }, (n) =>
      t("drawn {n}×", { n }),
    ),
    ...counted(stats, "radiant", { key: "shineOn", title: () => t("Shine On"), animation: "radiant", side: "you" }, (n) =>
      t("made Radiant {n}×", { n }),
    ),
    ...shared(
      topShare(stats, (c) => c.playedInWin, (c) => c.playedInWin + c.playedInLoss),
      { key: "luckyCharm", title: () => t("Lucky Charm"), animation: "radiant", side: "you" },
      (n, m) => t("won {n} of the {m} games you played it in", { n, m }),
    ),
    ...shared(
      topShare(stats, (c) => c.playedInLoss, (c) => c.playedInWin + c.playedInLoss),
      { key: "jinx", title: () => t("Jinx"), animation: "death", side: "you" },
      (n, m) => t("lost {n} of the {m} games you played it in", { n, m }),
    ),
    // R1743000: from the game-level totals.
    ...about(
      stats.bestStreak > 0 ? topCard(stats, "playedInWin")?.id : undefined,
      { key: "hotStreak", title: () => t("Hot Streak"), animation: "radiant", side: "you" },
      () =>
        stats.bestStreak === 1
          ? t("best streak: {n} win", { n: stats.bestStreak })
          : t("best streak: {n} wins", { n: stats.bestStreak }),
    ),
    ...about(
      stats.longestGame > 0 ? longest?.id : undefined,
      { key: "marathon", title: () => t("Marathon"), animation: "entrance", side: "you" },
      () =>
        stats.longestGame === 1
          ? t("longest game: {n} turn", { n: stats.longestGame })
          : t("longest game: {n} turns", { n: stats.longestGame }),
    ),
    // R2649000: the game's own mechanics.
    ...counted(stats, "tempExpired", { key: "useItOrLoseIt", title: () => t("Use It or Lose It"), animation: "expire", side: "you" }, (n) =>
      t("slipped away unplayed {n}×", { n }),
    ),
    ...shared(
      topShare(stats, (c) => c.tempPlayed, (c) => c.tempPlayed + c.tempExpired),
      { key: "justInTime", title: () => t("Just in Time"), animation: "inTime", side: "you" },
      (n, m) => t("played {n} of the {m} times it was Temporary", { n, m }),
    ),
    ...counted(stats, "crumbled", { key: "sandCastle", title: () => t("Sand Castle"), animation: "crumble", side: "you" }, (n) =>
      t("crumbled {n}×", { n }),
    ),
    ...counted(stats, "crumbledAgainst", { key: "outlasted", title: () => t("Outlasted"), animation: "crumble", side: "opponent" }, (n) =>
      t("waited it out {n}×", { n }),
    ),
    ...counted(stats, "fengShuiGood", { key: "goodFengShui", title: () => t("Good Feng Shui"), animation: "radiant", side: "you" }, (n) =>
      t("blessed by Feng Shui {n}×", { n }),
    ),
    ...counted(stats, "fengShuiBad", { key: "badFengShui", title: () => t("Bad Feng Shui"), animation: "hit", side: "you" }, (n) =>
      t("cursed by Feng Shui {n}×", { n }),
    ),
    ...counted(stats, "traded", { key: "hotPotato", title: () => t("Hot Potato"), animation: "bounce", side: "you" }, (n) =>
      t("traded away {n}×", { n }),
    ),
    ...counted(stats, "tradeDrawn", { key: "tradeInValue", title: () => t("Trade-In Value"), animation: "entrance", side: "you" }, (n) =>
      t("pulled from a trade {n}×", { n }),
    ),
    ...counted(stats, "created", { key: "massProduced", title: () => t("Mass Produced"), animation: "stamp", side: "you" }, (n) =>
      t("created {n}×", { n }),
    ),
    ...counted(stats, "kills", { key: "executioner", title: () => t("Executioner"), animation: "lunge", side: "you" }, (n) =>
      n === 1 ? t("took down {n} enemy", { n }) : t("took down {n} enemies", { n }),
    ),
    ...counted(stats, "biggestKill", { key: "giantSlayer", title: () => t("Giant Slayer"), animation: "lunge", side: "you" }, (n) =>
      t("felled a {n}-health giant", { n }),
    ),
    ...counted(stats, "reborn", { key: "secondWind", title: () => t("Second Wind"), animation: "reborn", side: "you" }, (n) =>
      t("rose again {n}×", { n }),
    ),
    ...counted(stats, "shieldsLost", { key: "bubbleWrap", title: () => t("Bubble Wrap"), animation: "shield", side: "you" }, (n) =>
      t("shield popped {n}×", { n }),
    ),
    ...counted(stats, "armorAbsorbed", { key: "ironWall", title: () => t("Iron Wall"), animation: "armor", side: "you" }, (n) =>
      t("shrugged off {n} damage", { n }),
    ),
    ...counted(stats, "damageTaken", { key: "punchingBag", title: () => t("Punching Bag"), animation: "hit", side: "you" }, (n) =>
      t("took {n} damage", { n }),
    ),
    ...counted(stats, "doubleAttacks", { key: "doubleTap", title: () => t("Double Tap"), animation: "windfury", side: "you" }, (n) =>
      t("attacked twice in a turn {n}×", { n }),
    ),
    ...counted(stats, "lifesteal", { key: "vampire", title: () => t("Vampire"), animation: "heal", side: "you" }, (n) =>
      t("drained {n} health", { n }),
    ),
    ...counted(stats, "sacrificed", { key: "martyr", title: () => t("Martyr"), animation: "doom", side: "you" }, (n) =>
      t("gave itself up {n}×", { n }),
    ),
    ...counted(stats, "froze", { key: "coldSnap", title: () => t("Cold Snap"), animation: "lunge", side: "you" }, (n) =>
      n === 1 ? t("froze {n} enemy", { n }) : t("froze {n} enemies", { n }),
    ),
    ...counted(stats, "frozen", { key: "brainFreeze", title: () => t("Brain Freeze"), animation: "freeze", side: "you" }, (n) =>
      t("frozen {n}×", { n }),
    ),
    ...counted(stats, "thawed", { key: "springThaw", title: () => t("Spring Thaw"), animation: "thaw", side: "you" }, (n) =>
      t("thawed out {n}×", { n }),
    ),
    ...counted(stats, "shattered", { key: "shatter", title: () => t("Shatter"), animation: "lunge", side: "you" }, (n) =>
      n === 1 ? t("shattered {n} frozen enemy", { n }) : t("shattered {n} frozen enemies", { n }),
    ),
    ...counted(stats, "flashback", { key: "encore", title: () => t("Encore"), animation: "reborn", side: "you" }, (n) =>
      t("played from the graveyard {n}×", { n }),
    ),
    ...counted(stats, "manaSaved", { key: "bargainHunter", title: () => t("Bargain Hunter"), animation: "radiant", side: "you" }, (n) =>
      t("saved {n} mana", { n }),
    ),
    ...counted(stats, "countered", { key: "denied", title: () => t("Denied"), animation: "exile", side: "you" }, (n) =>
      t("countered {n}×", { n }),
    ),
    // A counter is a Trap's reveal, or a Spell's cast.
    ...tallied(
      nope,
      { key: "nope", title: () => t("Nope"), animation: nope !== undefined && isSpell(nope.id) ? "cast" : "trap", side: "you" },
      (n) => (n === 1 ? t("stopped {n} play cold", { n }) : t("stopped {n} plays cold", { n })),
    ),
    ...counted(stats, "burned", { key: "burntToast", title: () => t("Burnt Toast"), animation: "death", side: "you" }, (n) =>
      t("burned from a full hand {n}×", { n }),
    ),
    ...counted(stats, "bounced", { key: "frequentFlyer", title: () => t("Frequent Flyer"), animation: "bounce", side: "you" }, (n) =>
      t("bounced back to hand {n}×", { n }),
    ),
    ...counted(stats, "launched", { key: "liftoff", title: () => t("Liftoff"), animation: "launch", side: "you" }, (n) =>
      n === 1 ? t("flew in {n} launched Starship", { n }) : t("flew in {n} launched Starships", { n }),
    ),
    ...about(brick?.id, { key: "brick", title: () => t("Brick"), animation: "exile", side: "you" }, () =>
      t("drawn {n}×, played {m}×", { n: bricked.drawn, m: bricked.played }),
    ),
  ];
  return out.map((stat) => ({ ...stat, animation: animationFor(stat.cardId, stat.animation) }));
}

/** R1743000: the showcase's order: what is left of this round, and the stat shown now. */
export type ShowcaseBag = { readonly shown: ShowcaseKey | null; readonly left: readonly ShowcaseKey[] };

export const EMPTY_BAG: ShowcaseBag = Object.freeze({ shown: null, left: Object.freeze([]) });

/**
 * R1743000: the bag after one stat more: the next of the round's left (those `keys` still holds), or,
 * with none left, a new round shuffled by `random` that never starts with the stat just shown.
 */
export function nextInBag(bag: ShowcaseBag, keys: readonly ShowcaseKey[], random: RandomSource): ShowcaseBag {
  let left = bag.left.filter((key) => keys.includes(key));
  if (left.length === 0) {
    left = shuffled(keys, random);
    const first = left[0];
    if (first !== undefined && first === bag.shown && left.length > 1) left = [...left.slice(1), first];
  }
  return { shown: left[0] ?? null, left: left.slice(1) };
}
