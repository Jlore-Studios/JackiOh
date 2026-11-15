// What a live challenge's Accept joins with and where it goes next (R1831000, R3729002): the toast
// (`ChallengeToast.tsx`) and the inbox (`inbox/InboxPanel.tsx`) answer a challenge the same way, so the two
// steps they share live here. CLAUDE.md rule 7: whether a deck may be played is the server's, at the room.

import type { ModeChoice, QueueMode, SavedDeck, SavedTrio } from "../net/api.ts";
import { readPlayLean } from "../game/playLean.ts";
import { paths } from "../net/navigate.ts";
import { choiceOf, pickDeck, pickTrio, readChallengeChoice } from "./challengeChoice.ts";

/** The player's saved decks and trios, as `GET /api/decks` answered. */
export type SavedPicks = { decks: readonly SavedDeck[]; trios: readonly SavedTrio[] };

/** The game a join started: the match, else the series, else `/play`. */
export function gameAfterJoin(answer: { matchId: string | null; seriesId: string | null }): string {
  if (typeof answer.matchId === "string" && answer.matchId.length > 0) return paths.match(answer.matchId);
  if (typeof answer.seriesId === "string" && answer.seriesId.length > 0) return paths.series(answer.seriesId);
  return paths.play;
}

/**
 * What a challenge in `mode` is accepted with when the player picks nothing: the deck or trio last used for an
 * accepted challenge, else the first (`pickDeck`, `pickTrio`); for All random, the device's online "More cards
 * from" pick (R2031000). Null while a Best of 1 or Conquest challenge has no deck or trio to join with (`saved`
 * unread, or none saved): the way in is then the room's lobby.
 */
export function rememberedChoice(mode: QueueMode, saved: SavedPicks | null): ModeChoice | null {
  if (mode === "random") return choiceOf("random", null, null, readPlayLean());
  if (saved === null) return null;
  const remembered = readChallengeChoice();
  return choiceOf(mode, pickDeck(saved.decks, remembered.deckId)?.id ?? null, pickTrio(saved.trios, remembered.trioId)?.id ?? null);
}
