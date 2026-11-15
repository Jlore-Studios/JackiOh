// The Play hub's catalog of ways to play (#3729, R3729000, SPEC §9.5): the four groups and each card's name, line,
// time and need, in one place, so the screen restates none of them (CLAUDE.md rule 9). Nothing here is a rule
// (rule 7): the modes' rules are the engine's and the server's, and these are the words that say what a card is.

import type { QueueMode } from "../net/api.ts";
import { MODE_LABEL } from "../net/liveGame.ts";
import { paths } from "../net/navigate.ts";
import { t } from "../i18n/i18n.ts";
import { playTestid } from "./playTestids.ts";
import { TUTORIAL_LESSONS } from "../tutorial/lessons.ts";
import { dailyConstants } from "../wasm/index.ts";

/** R3729000: the hub's groups, in the order they are drawn. */
export type PlayGroup = "online" | "daily" | "friends" | "solo";

export const PLAY_GROUPS: readonly PlayGroup[] = ["online", "daily", "friends", "solo"];

/** A group's heading, worded at render. */
export function groupLabel(group: PlayGroup): string {
  switch (group) {
    case "online":
      return t("Online");
    case "daily":
      return t("Daily");
    case "friends":
      return t("With friends");
    case "solo":
      return t("Solo");
  }
}

/** What a card does when it is pressed. */
export type PlayAction =
  | { kind: "queue"; mode: QueueMode }
  | { kind: "link"; href: string }
  | { kind: "lobby" };

/** The cards, by id. */
export type PlayCardId =
  | "bo1"
  | "bo3"
  | "random"
  | "daily"
  | "lobby"
  | "challenge"
  | "practice"
  | "dungeon"
  | "tutorial";

export type PlayCard = {
  id: PlayCardId;
  group: PlayGroup;
  name: string;
  /** One sentence on what it is. */
  line: string;
  /** How long a game takes, as a short phrase. */
  time: string;
  /** What it takes to start. */
  needs: string;
  testid: string;
  action: PlayAction;
};

/**
 * How long one game of a card takes, in minutes, shortest and longest: what a player should plan for, a
 * presentation estimate and no rule (the spec fixes only the 75 s turn clock and the 120 minute ceiling, §2.8).
 * The daily card has none: its time is a move every few days (`DAILY_TIME_PER_MOVE_DAYS`).
 */
export const GAME_MINUTES: Readonly<Record<Exclude<PlayCardId, "daily">, readonly [number, number]>> = {
  bo1: [10, 20],
  bo3: [25, 50],
  random: [10, 20],
  lobby: [10, 20],
  challenge: [10, 20],
  practice: [10, 20],
  dungeon: [20, 40],
  tutorial: [3, 5],
};

/** R3729000: the testid of each card (`playTestid`); `e2e/support/testids.ts` mirrors them. */
export const PLAY_CARD_TESTID: Readonly<Record<PlayCardId, string>> = {
  bo1: playTestid.modeBo1,
  bo3: playTestid.modeBo3,
  random: playTestid.modeRandom,
  daily: playTestid.daily,
  lobby: playTestid.lobby,
  challenge: playTestid.challenge,
  practice: playTestid.practice,
  dungeon: playTestid.dungeon,
  tutorial: playTestid.tutorial,
};

/** A game's time as a card says it: "10–20 min". */
export function minutesText([min, max]: readonly [number, number]): string {
  return t("{min}–{max} min", { min: String(min), max: String(max) });
}

/** The daily card's time: "A move every 1 to 14 days", from the shortest and longest time per move. */
function dailyTime(): string {
  const days = dailyConstants().DAILY_TIME_PER_MOVE_DAYS;
  const min = days[0];
  const max = days.at(-1);
  if (min === undefined || max === undefined) return t("A move every few days");
  return t("A move every {min} to {max} days", { min: String(min), max: String(max) });
}

/** The tutorial's card opens its first lesson, which starts at once (`routes/practice.tsx`, `?lesson=`). */
function tutorialHref(): string {
  const first = TUTORIAL_LESSONS[0];
  return first === undefined ? paths.practice : `${paths.practice}?lesson=${first.id}`;
}

export type PlayCardsOptions = {
  /** A visitor with no account sees the solo cards only: the rest ask for an account. */
  signedIn: boolean;
  /** #3727: the daily games waiting on the player. */
  dailyWaiting: number;
};

/**
 * R3729000: the hub's cards in their groups' order. A visitor gets the solo cards and nothing that calls the
 * engine's module, since the signed-out hub does not wait for it.
 */
export function playCards({ signedIn, dailyWaiting }: PlayCardsOptions): readonly PlayCard[] {
  const solo: readonly PlayCard[] = [
    {
      id: "practice",
      group: "solo",
      name: t("Practice"),
      line: t("Play the AI with a deck you pick, at the level you choose."),
      time: minutesText(GAME_MINUTES.practice),
      needs: t("Nothing to bring"),
      testid: PLAY_CARD_TESTID.practice,
      action: { kind: "link", href: paths.practice },
    },
    {
      id: "dungeon",
      group: "solo",
      name: t("Dungeon run"),
      line: t("Fight your way down a dungeon, one fight at a time."),
      time: minutesText(GAME_MINUTES.dungeon),
      needs: t("Nothing to bring"),
      testid: PLAY_CARD_TESTID.dungeon,
      action: { kind: "link", href: paths.dungeon },
    },
    {
      id: "tutorial",
      group: "solo",
      name: t("Tutorial"),
      line: t("Learn the game one lesson at a time."),
      time: minutesText(GAME_MINUTES.tutorial),
      needs: t("Nothing to bring"),
      testid: PLAY_CARD_TESTID.tutorial,
      action: { kind: "link", href: tutorialHref() },
    },
  ];
  if (!signedIn) return solo;
  const queue = (id: "bo1" | "bo3" | "random", line: string, needs: string): PlayCard => ({
    id,
    group: "online",
    name: t(MODE_LABEL[id]),
    line,
    time: minutesText(GAME_MINUTES[id]),
    needs,
    testid: PLAY_CARD_TESTID[id],
    action: { kind: "queue", mode: id },
  });
  return [
    queue("bo1", t("One deck, one game, and rank on the line."), t("One saved deck")),
    queue("bo3", t("Win a game with each of your three decks."), t("A trio of decks")),
    queue("random", t("The server deals you both a deck, so only skill counts."), t("Nothing to bring")),
    {
      id: "daily",
      group: "daily",
      name: t("Daily games"),
      line: dailyWaiting > 0 ? t("Your move in {count}", { count: String(dailyWaiting) }) : t("A game played over days, one move at a time."),
      time: dailyTime(),
      needs: t("A deck, or nothing in All random"),
      testid: PLAY_CARD_TESTID.daily,
      action: { kind: "link", href: paths.daily },
    },
    {
      id: "lobby",
      group: "friends",
      name: t("Private lobby"),
      line: t("Make a lobby, share its code, and start when you are both ready."),
      time: minutesText(GAME_MINUTES.lobby),
      needs: t("A friend"),
      testid: PLAY_CARD_TESTID.lobby,
      action: { kind: "lobby" },
    },
    {
      id: "challenge",
      group: "friends",
      name: t("Challenge a friend"),
      line: t("Challenge a friend who is online, and play when they accept."),
      time: minutesText(GAME_MINUTES.challenge),
      needs: t("A friend who is online"),
      testid: PLAY_CARD_TESTID.challenge,
      action: { kind: "link", href: paths.together },
    },
    ...solo,
  ];
}
