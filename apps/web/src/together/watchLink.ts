// The watch link (#936, B50): `/watch/:matchId?friend=<id>`, the friend whose seat is watched riding
// on the query. Kept apart from `WatchRoute.tsx` so the shell reads it without loading the screen.

import { paths } from "../net/navigate.ts";

const FRIEND_PARAM = "friend";

/** The link a friend row's Watch follows. */
export function watchLink(matchId: string, friendId: string): string {
  return `${paths.watch(matchId)}?${new URLSearchParams({ [FRIEND_PARAM]: friendId }).toString()}`;
}

/** The friend a watch link names, off the address bar's query; empty when it names none. */
export function watchFriendOf(search: string): string {
  return new URLSearchParams(search).get(FRIEND_PARAM) ?? "";
}
