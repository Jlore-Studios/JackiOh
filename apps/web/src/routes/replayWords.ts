// How a listed online game is worded, shared by `/replays` and the account's match history (#3733). Kept in
// a module of its own so the account, which is in the entry chunk, need not import `replays.tsx`.

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

/** #3698: the opponent as the row and its replay both name them: the username, "a deleted account" when it is gone. */
export function opponentName(row: OnlineReplayListing): string {
  return row.opponent.username === "" ? t("a deleted account") : row.opponent.username;
}

/** The row's reason it cannot be shown, if any, worded as the player words it. */
export function unavailableText(reason: PracticeReplayListing["unavailable"]): string | null {
  return reason === undefined ? null : replayFailureText(new ReplayFailure(reason));
}
