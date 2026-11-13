// Where on Play together another screen links to (#936 B59): `/replays`' "Shared with me" opens its
// list of shared replays. Apart from `TogetherRoute.tsx`, so the link does not load the page's code.

import { paths } from "../net/navigate.ts";

/** The id of the "Shared with me" list, and the fragment that opens the page at it. */
export const SHARED_ANCHOR = "shared";

/** The query that opens the shared replay of one match on Play together (R3729002); the inbox's Watch sends it. */
export const REPLAY_PARAM = "replay";

/** Play together at its "Shared with me" list; with `matchId`, that match's shared replay opens as the page loads. */
export function sharedWithMePath(matchId?: string): string {
  const query = matchId === undefined ? "" : `?${REPLAY_PARAM}=${encodeURIComponent(matchId)}`;
  return `${paths.together}${query}#${SHARED_ANCHOR}`;
}
