// The corner props' pictures (#3348): one small inline SVG per prop id, drawn flat in the mat's look.
// A shape carries `pa`, `pb` or `pc`, and arena.css paints each of the three from the mat's own
// `--prop-a`, `--prop-b` and `--prop-c`, so this file holds no colour.

import type { ReactElement } from "react";

const svg = (...shapes: ReactElement[]): ReactElement => (
  <svg viewBox="0 0 48 48" aria-hidden="true" focusable="false">
    {shapes}
  </svg>
);

/** One picture per prop id (props.ts): at most six shapes each. */
export const PROP_ART: Readonly<Record<string, ReactElement>> = {
  "tavern-candle": svg(
    <rect key="body" className="pa" x="19" y="22" width="10" height="20" rx="2" />,
    <rect key="base" className="pb" x="14" y="41" width="20" height="4" rx="2" />,
    <rect key="wick" className="pb" x="23" y="19" width="2" height="5" />,
    <path key="flame" className="pc" d="M24 5c5 6 6 10 0 14c-6-4-5-8 0-14z" />,
  ),
  "tavern-mug": svg(
    <path key="handle" className="pb" fillRule="evenodd" d="M30 18h5a7 7 0 0 1 0 14h-5v-4h5a3 3 0 0 0 0-6h-5z" />,
    <rect key="body" className="pa" x="10" y="14" width="22" height="28" rx="3" />,
    <rect key="band" className="pb" x="10" y="24" width="22" height="3" />,
    <circle key="foam1" className="pc" cx="16" cy="13" r="5" />,
    <circle key="foam2" className="pc" cx="26" cy="12" r="5" />,
  ),
  "wood-dice": svg(
    <rect key="cube" className="pa" x="8" y="8" width="32" height="32" rx="6" />,
    <circle key="p1" className="pb" cx="17" cy="17" r="3.2" />,
    <circle key="p2" className="pb" cx="31" cy="17" r="3.2" />,
    <circle key="p3" className="pb" cx="24" cy="24" r="3.2" />,
    <circle key="p4" className="pb" cx="17" cy="31" r="3.2" />,
    <circle key="p5" className="pb" cx="31" cy="31" r="3.2" />,
  ),
  "wood-coin": svg(
    <circle key="rim" className="pa" cx="24" cy="24" r="18" />,
    <circle key="face" className="pb" cx="24" cy="24" r="13" />,
    <rect key="v" className="pc" x="22" y="16" width="4" height="16" rx="1" />,
    <rect key="h" className="pc" x="16" y="22" width="16" height="4" rx="1" />,
  ),
  "felt-chips": svg(
    <ellipse key="c4" className="pa" cx="24" cy="38" rx="16" ry="6" />,
    <ellipse key="c3" className="pb" cx="24" cy="30" rx="16" ry="6" />,
    <ellipse key="c2" className="pa" cx="24" cy="22" rx="16" ry="6" />,
    <ellipse key="c1" className="pc" cx="24" cy="14" rx="16" ry="6" />,
  ),
  "felt-ace": svg(
    <rect key="card" className="pc" x="10" y="5" width="28" height="38" rx="3" />,
    <path key="spade" className="pa" d="M24 13c8 7 10 11 6 15c-2 2-5 1-6-1c1 4 2 6 4 8h-8c2-2 3-4 4-8c-1 2-4 3-6 1c-4-4-2-8 6-15z" />,
    <circle key="pip" className="pb" cx="16" cy="12" r="2.5" />,
  ),
  "neon-ghost": svg(
    <path key="body" className="pa" d="M10 40V22a14 14 0 0 1 28 0v18l-5-4l-4 4l-5-4l-5 4l-4-4z" />,
    <circle key="eye1" className="pc" cx="18" cy="22" r="3" />,
    <circle key="eye2" className="pc" cx="30" cy="22" r="3" />,
    <ellipse key="mouth" className="pb" cx="24" cy="30" rx="3" ry="2" />,
  ),
  "neon-bolt": svg(
    <path key="bolt" className="pa" d="M28 4L10 28h12l-4 16l20-26H26z" />,
    <circle key="s1" className="pc" cx="38" cy="10" r="2" />,
    <circle key="s2" className="pb" cx="9" cy="38" r="2" />,
  ),
  "marble-urn": svg(
    <path key="body" className="pa" d="M16 10h16v4c6 4 8 10 6 18c-1 5-5 8-14 8s-13-3-14-8c-2-8 0-14 6-18z" />,
    <rect key="rim" className="pb" x="14" y="6" width="20" height="5" rx="2" />,
    <rect key="foot" className="pb" x="16" y="40" width="16" height="4" rx="2" />,
    <rect key="band" className="pc" x="12" y="24" width="24" height="3" />,
  ),
  "marble-laurel": svg(
    <path key="left" className="pa" d="M10 12c4 1 6 4 6 8c-4-1-6-4-6-8zM8 22c4 0 7 3 7 7c-4 0-7-3-7-7zM12 32c4-1 7 1 8 5c-4 1-7-1-8-5z" />,
    <path key="right" className="pa" d="M38 12c-4 1-6 4-6 8c4-1 6-4 6-8zM40 22c-4 0-7 3-7 7c4 0 7-3 7-7zM36 32c-4-1-7 1-8 5c4 1 7-1 8-5z" />,
    <path key="ribbon" className="pc" d="M24 38l-5 7l5-2l5 2z" />,
    <circle key="knot" className="pb" cx="24" cy="36" r="2.5" />,
  ),
  "chess-knight": svg(
    <rect key="foot" className="pb" x="12" y="39" width="24" height="5" rx="2" />,
    <path key="body" className="pa" d="M16 39c1-8 3-12 7-16c-4 1-7 2-9 6l-5-4c2-7 8-15 17-15c9 0 14 7 14 15c0 7-2 11-3 14z" />,
    <path key="mane" className="pb" d="M24 9l3 5l-4 1z" />,
    <circle key="eye" className="pc" cx="26" cy="16" r="2" />,
  ),
  "chess-clock": svg(
    <circle key="bell1" className="pb" cx="12" cy="10" r="5" />,
    <circle key="bell2" className="pb" cx="36" cy="10" r="5" />,
    <circle key="rim" className="pa" cx="24" cy="26" r="16" />,
    <circle key="face" className="pc" cx="24" cy="26" r="12" />,
    <rect key="h1" className="pb" x="23" y="16" width="2" height="11" />,
    <rect key="h2" className="pb" x="24" y="25" width="9" height="2" />,
  ),
  "desert-cactus": svg(
    <rect key="trunk" className="pa" x="19" y="8" width="10" height="34" rx="5" />,
    <path key="armL" className="pa" d="M19 28h-5a4 4 0 0 1-4-4v-6a3 3 0 0 1 6 0v4h3z" />,
    <path key="armR" className="pa" d="M29 22h5v-4a3 3 0 0 1 6 0v6a4 4 0 0 1-4 4h-7z" />,
    <rect key="sand" className="pb" x="10" y="40" width="28" height="5" rx="2" />,
    <circle key="flower" className="pc" cx="24" cy="8" r="3" />,
  ),
  "desert-skull": svg(
    <path key="dune" className="pc" d="M2 46c8-8 14-6 22-6s14-2 22 6z" />,
    <path key="cranium" className="pa" d="M24 5c-10 0-16 6-16 15c0 5 2 8 6 10v7h20v-7c4-2 6-5 6-10c0-9-6-15-16-15z" />,
    <circle key="eye1" className="pb" cx="17" cy="21" r="4" />,
    <circle key="eye2" className="pb" cx="31" cy="21" r="4" />,
    <path key="nose" className="pb" d="M24 27l-3 5h6z" />,
  ),
  "obsidian-crystal": svg(
    <path key="body" className="pa" d="M24 4l12 14l-12 26L12 18z" />,
    <path key="crown" className="pb" d="M24 4l12 14H12z" />,
    <path key="shine" className="pc" d="M24 4v40L12 18z" />,
  ),
  "obsidian-rune": svg(
    <rect key="stone" className="pa" x="10" y="6" width="28" height="36" rx="8" />,
    <rect key="stem" className="pc" x="22" y="12" width="4" height="26" />,
    <rect key="left" className="pc" x="15" y="18" width="4" height="14" transform="rotate(-35 17 25)" />,
    <rect key="right" className="pc" x="29" y="18" width="4" height="14" transform="rotate(35 31 25)" />,
  ),
  "map-compass": svg(
    <circle key="rim" className="pa" cx="24" cy="24" r="19" />,
    <circle key="face" className="pc" cx="24" cy="24" r="15" />,
    <path key="north" className="pb" d="M24 8l5 16H19z" />,
    <path key="south" className="pa" d="M24 40l-5-16h10z" />,
    <circle key="hub" className="pb" cx="24" cy="24" r="2.5" />,
  ),
  "map-serpent": svg(
    <path key="neck" className="pa" d="M14 38c-4-10-2-22 6-26c6-3 12 0 12 6c0 5-4 6-6 3c-1 8 4 12 8 17z" />,
    <path key="sea" className="pc" d="M2 38c6-4 10-4 14 0s10 4 14 0s10-4 16 0v8H2z" />,
    <circle key="eye" className="pb" cx="25" cy="14" r="1.8" />,
    <circle key="spot" className="pb" cx="17" cy="30" r="2" />,
  ),
  "astral-comet": svg(
    <path key="tail" className="pb" d="M25 11L4 40l31-17z" />,
    <circle key="head" className="pa" cx="32" cy="16" r="8" />,
    <circle key="s1" className="pc" cx="10" cy="14" r="2" />,
    <circle key="s2" className="pc" cx="38" cy="38" r="2" />,
  ),
  "astral-satellite": svg(
    <rect key="panelL" className="pb" x="2" y="20" width="13" height="8" />,
    <rect key="panelR" className="pb" x="33" y="20" width="13" height="8" />,
    <rect key="body" className="pa" x="18" y="18" width="12" height="12" rx="2" />,
    <rect key="mast" className="pc" x="23" y="9" width="2" height="9" />,
    <circle key="dish" className="pc" cx="24" cy="8" r="3" />,
  ),
  "stinkyscape-rocket": svg(
    <path key="flame" className="pc" d="M19 36h10l-5 10z" />,
    <path key="finL" className="pb" d="M17 26l-7 10h9z" />,
    <path key="finR" className="pb" d="M31 26l7 10h-9z" />,
    <path key="body" className="pa" d="M24 3c7 6 9 16 7 33H17c-2-17 0-27 7-33z" />,
    <circle key="port" className="pc" cx="24" cy="17" r="4" />,
  ),
  "stinkyscape-planet": svg(
    <ellipse key="ringBack" className="pc" cx="24" cy="26" rx="22" ry="5" />,
    <circle key="world" className="pa" cx="24" cy="24" r="12" />,
    <path key="band" className="pb" d="M13 20h22v4H13z" />,
    <path key="ringFront" className="pc" d="M2 26a22 5 0 0 0 44 0h-5a17 2.5 0 0 1-34 0z" />,
    <circle key="moon" className="pb" cx="40" cy="9" r="3" />,
  ),
};
