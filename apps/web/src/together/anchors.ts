// Where on the Friends page another screen links to (#936 B59, R3732002): the page is five tabs by
// `?section=`, and `/replays`' "Shared with me" opens the last of them. Apart from `TogetherRoute.tsx`, so a
// link does not load the page's code.

import { paths } from "../net/navigate.ts";

/** The id of the "Shared with me" list, and the fragment that opens the page at it. */
export const SHARED_ANCHOR = "shared";

/** The query that opens the shared replay of one match on Play together (R3729002); the inbox's Watch sends it. */
export const REPLAY_PARAM = "replay";

/** The page's tabs, in order: the friends, the requests in and out, who the player lately played, the blocked, the shared replays. */
export const FRIENDS_SECTIONS = ["friends", "requests", "recent", "blocked", "shared"] as const;
export type FriendsSection = (typeof FRIENDS_SECTIONS)[number];

/**
 * The tab a URL names: `?section=`, else `#shared` (the old link to the shared replays), else the friends.
 * `search` and `hash` are `window.location`'s, with their `?` and `#`.
 */
export function friendsSectionOf(search: string, hash = ""): FriendsSection {
  const named = new URLSearchParams(search).get("section");
  const found = FRIENDS_SECTIONS.find((section) => section === named);
  if (found !== undefined) return found;
  return hash === `#${SHARED_ANCHOR}` ? "shared" : "friends";
}

/** Play together at one tab. */
export function friendsSectionPath(section: FriendsSection): string {
  return `${paths.together}?section=${section}`;
}

/** Play together at its "Shared with me" list; with `matchId`, that match's shared replay opens as the page loads (R3729002). */
export function sharedWithMePath(matchId?: string): string {
  const path = friendsSectionPath("shared");
  return matchId === undefined ? path : `${path}&${REPLAY_PARAM}=${encodeURIComponent(matchId)}`;
}
