// The dungeon map's node icons (#3229): one drawn glyph per `RunNodeKind` on a 24-unit grid, single
// color in `currentColor`, so `run.css` colors each by the node's `data-kind` from the palette. The
// glyph is decoration only: the button's `aria-label` names the kind, so no name rests on it.
// Presentation only (CLAUDE.md rule 7).

import type { ReactElement } from "react";

import type { RunNodeKind } from "@jackioh/run";

/** Each kind's shapes, a new kind in the run's enum breaks the build here, not the screen. */
const GLYPHS: Readonly<Record<RunNodeKind, ReactElement>> = {
  fight: (
    <>
      <path d="M4 4 L14.5 14.5 M12.6 16.4 L16.4 12.6 M15.5 15.5 L19 19 M20 4 L9.5 14.5 M11.4 16.4 L7.6 12.6 M8.5 15.5 L5 19" />
      <circle cx="20" cy="20" r="1.4" fill="currentColor" stroke="none" />
      <circle cx="4" cy="20" r="1.4" fill="currentColor" stroke="none" />
    </>
  ),
  elite: (
    <>
      <path d="M12 3 L20 6 V12 C20 17 16 20.5 12 22 C8 20.5 4 17 4 12 V6 Z" />
      <path d="M12 8 L14.5 12 L12 16 L9.5 12 Z" fill="currentColor" />
    </>
  ),
  ghost: (
    <>
      <path d="M5 21 V11 A7 7 0 0 1 19 11 V21 L16.5 19 L14 21 L12 19 L10 21 L7.5 19 Z" />
      <circle cx="9.5" cy="11" r="1.3" fill="currentColor" stroke="none" />
      <circle cx="14.5" cy="11" r="1.3" fill="currentColor" stroke="none" />
    </>
  ),
  miniBoss: (
    <>
      <path d="M12 6 C7.5 6 4.5 9 4.5 12.5 C4.5 15 6 16.8 7.8 17.7 V21 H16.2 V17.7 C18 16.8 19.5 15 19.5 12.5 C19.5 9 16.5 6 12 6 Z M6.5 8.5 C4.5 7.5 3.5 5.5 3.5 3 C6 3.5 7.5 4.7 8.6 6.5 M17.5 8.5 C19.5 7.5 20.5 5.5 20.5 3 C18 3.5 16.5 4.7 15.4 6.5 M10.7 21 V18 M13.3 21 V18" />
      <circle cx="9.2" cy="12.6" r="1.8" fill="currentColor" stroke="none" />
      <circle cx="14.8" cy="12.6" r="1.8" fill="currentColor" stroke="none" />
    </>
  ),
  boss: (
    <path d="M3.5 18 L2.5 7 L8 11.5 L12 5 L16 11.5 L21.5 7 L20.5 18 Z M4 21.5 H20" />
  ),
  shop: (
    <>
      <path d="M2.5 4 H5 L7.6 15 H18 L20.5 7.5 H5.8" />
      <circle cx="9.5" cy="19.2" r="1.5" fill="currentColor" stroke="none" />
      <circle cx="16.5" cy="19.2" r="1.5" fill="currentColor" stroke="none" />
    </>
  ),
  campfire: (
    <path d="M12 2 C13 6 17 8 17 12.5 C17 15.5 14.8 17.5 12 17.5 C9.2 17.5 7 15.5 7 12.5 C7 10.2 8.2 9 9 7.8 C9.3 9.3 9.8 10.2 10.5 10.7 C10.5 7.5 11 4.5 12 2 Z M5 22 L19 18.5 M5 18.5 L19 22" />
  ),
  event: (
    <>
      <circle cx="12" cy="12" r="9.5" />
      <path d="M9.2 9.4 A2.9 2.9 0 1 1 13.2 12.1 C12.4 12.6 12 13.2 12 14.4" />
      <circle cx="12" cy="17.2" r="1.1" fill="currentColor" stroke="none" />
    </>
  ),
  chest: (
    <>
      <path d="M3.5 11 V9 C3.5 6 5.5 4 8.5 4 H15.5 C18.5 4 20.5 6 20.5 9 V11 Z M3.5 11 V20 H20.5 V11" />
      <rect x="10.5" y="9" width="3" height="4.2" rx="0.6" fill="currentColor" />
    </>
  ),
  shrine: (
    <path d="M2 5 C4 7.5 20 7.5 22 5 M5 11 H19 M6.5 7.5 V21 M17.5 7.5 V21 M12 8 V11" />
  ),
};

/** One node kind's icon: 24 px, `currentColor`, hidden from assistive tech. */
export function NodeIcon({ kind }: { kind: RunNodeKind }): ReactElement {
  return (
    <svg
      className="dungeon-run-map__glyph"
      data-icon={kind}
      viewBox="0 0 24 24"
      width="24"
      height="24"
      aria-hidden="true"
      focusable="false"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      {GLYPHS[kind]}
    </svg>
  );
}
