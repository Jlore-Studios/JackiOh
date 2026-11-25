// How a listed online game is worded, shared by `/replays`, the account's match history (#3733) and the Daily
// screen's list and board (R4621000). Kept in a module of its own so the account screen need not import `replays.tsx`.

import { t } from "../i18n/i18n.ts";
import type { OnlineReplayListing, QueueMode } from "../net/api.ts";
import type { PracticeReplayListing } from "../practice/protocol.ts";
import { ReplayFailure, replayFailureText } from "../replay/source.ts";

export const RESULT_WORDS: Readonly<Record<PracticeReplayListing["result"], string>> = {
  win: "Win",
  loss: "Loss",
  draw: "Draw",
};

export const MODE_WORDS: Readonly<Record<QueueMode, string>> = {
  bo1: "Best of 1",
  bo3: "Conquest",
  random: "All random",
};

/** R4621000, #3698: a player's username as a row or a board names them: "a deleted account" when it is empty. */
export function playerName(username: string): string {
  return username === "" ? t("a deleted account") : username;
}

/** #3698: the opponent as the row and its replay both name them: the username, "a deleted account" when it is gone. */
export function opponentName(row: OnlineReplayListing): string {
  return playerName(row.opponent.username);
}

/** The row's reason it cannot be shown, if any, worded as the player words it. */
export function unavailableText(reason: PracticeReplayListing["unavailable"]): string | null {
  return reason === undefined ? null : replayFailureText(new ReplayFailure(reason));
}
