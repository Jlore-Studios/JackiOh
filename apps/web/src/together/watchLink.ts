// The watch link (#936, B50): `/watch/:matchId?friend=<id>`, the friend whose seat is watched riding
// on the query, and `&coop=1` when the game is a co-op one, drawn on the co-op board (R2881000). Kept
// apart from `WatchRoute.tsx` so the shell reads it without loading the screen.

import { paths } from "../net/navigate.ts";

const FRIEND_PARAM = "friend";
const COOP_PARAM = "coop";
const COOP_ON = "1";

/** The link a friend row's Watch follows; `coop` for a friend's co-op game (R2881000). */
export function watchLink(matchId: string, friendId: string, coop = false): string {
  const query = new URLSearchParams({ [FRIEND_PARAM]: friendId });
  if (coop) query.set(COOP_PARAM, COOP_ON);
  return `${paths.watch(matchId)}?${query.toString()}`;
}

/** The friend a watch link names, off the address bar's query. */
export function watchFriendOf(search: string): string {
  return new URLSearchParams(search).get(FRIEND_PARAM) ?? "";
}

/** Whether a watch link names a co-op game (R2881000), off the address bar's query. */
export function watchCoopOf(search: string): boolean {
  return new URLSearchParams(search).get(COOP_PARAM) === COOP_ON;
}
