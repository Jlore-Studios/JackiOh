// The "blocked" mark (#37): a card dropped on a Locked zone it could not go into is refused with a
// red X where it landed, instead of silently snapping back to the hand.
//
// Presentation only (CLAUDE.md rule 7). DragLayer decides when a drop was onto a locked zone (it
// reads the board's `data-locked`, which Zone.tsx writes off the view); this file draws the mark and
// takes it down again. The mark never takes a hit, so `document.elementsFromPoint` sees through it.
//
// It holds for BLOCKED_MARK_MS whatever the motion setting, because that is how long a player needs
// to read it. Reduced motion (`--anim-scale: 0`) only removes the shake, never the mark.

import { useEffect, type ReactElement } from "react";

import "./blocked.css";

/** How long the mark stays on the zone before it is taken down. */
export const BLOCKED_MARK_MS = 700;

/** Where a refused drop landed: the locked zone's centre and its size. `key` renews the mark. */
export type Blocked = { key: number; testid: string; x: number; y: number; size: number };

export default function BlockedMark(props: { blocked: Blocked | null; onDone: () => void }): ReactElement | null {
  const { blocked, onDone } = props;

  useEffect(() => {
    if (blocked === null) return undefined;
    const timer = setTimeout(onDone, BLOCKED_MARK_MS);
    return () => clearTimeout(timer);
  }, [blocked, onDone]);

  if (blocked === null) return null;
  return (
    <div className="drag-layer drag-blocked-layer" aria-hidden="true" style={{ pointerEvents: "none" }}>
      <svg
        key={blocked.key}
        className="drag-blocked"
        data-testid="drag-blocked"
        data-zone={blocked.testid}
        viewBox="0 0 24 24"
        style={{ left: blocked.x, top: blocked.y, width: blocked.size, height: blocked.size }}
      >
        <circle className="drag-blocked-ring" cx="12" cy="12" r="10" />
        <path className="drag-blocked-x" d="M7.5 7.5l9 9M16.5 7.5l-9 9" />
      </svg>
    </div>
  );
}
