// The mark on a Locked zone (SPEC §6.3 Lock; BUILD M5-T1's `data-locked`): a padlock, so a zone that
// takes no summon reads as locked at a glance rather than as a small empty square.
//
// Presentation only (CLAUDE.md rule 7). Zone.tsx decides whether to draw it from the view's `locks`
// flag; this file knows nothing about why a zone is locked. A reserved zone (R64) keeps its own
// round mark in board.css, since it is held rather than locked.

import type { ReactElement } from "react";

import "./lock.css";

export default function LockIcon(): ReactElement {
  return (
    <svg className="zone-lock" role="img" aria-label="Locked zone" viewBox="0 0 24 24">
      <title>Locked zone</title>
      <path className="zone-lock-shackle" d="M7.5 11V7.5a4.5 4.5 0 0 1 9 0V11" />
      <rect className="zone-lock-body" x="4.5" y="10.5" width="15" height="11" rx="2.4" />
      <circle className="zone-lock-keyhole" cx="12" cy="15.2" r="1.6" />
      <path className="zone-lock-keyhole" d="M12 15.6v3" />
    </svg>
  );
}
