// Where on Play together another screen links to (#936 B59): `/replays`' "Shared with me" opens the
// page at its list of the replays friends shared. Apart from `TogetherRoute.tsx`, so the link does
// not load the page's code.

import { paths } from "../net/navigate.ts";

/** The id of the "Shared with me" list, and the fragment that opens the page at it. */
export const SHARED_ANCHOR = "shared";

export function sharedWithMePath(): string {
  return `${paths.together}#${SHARED_ANCHOR}`;
}
