// SPEC §14.14, R3013002: a card's proficiency stamp, drawn by code: one design per level, the same for every card,
// each more ornate than the one below it. Level 1 is a ring; 2 sets a star in it; 3 adds an inner dashed ring; 4
// sets the ring on a twelve-point rosette; 5 hangs two ribbon tails under it. It is filled and inked in its
// level's colours (stamps.css, the palette's and none in Radiant's gold window, R2725004) and sits in a fixed slot of
// the face, clear of every part (stamps.css). Drawn state only (CLAUDE.md rule 7).

import type { ReactElement } from "react";

import { t } from "../i18n/i18n.ts";
import { isStampLevel } from "./stamps.ts";

import "./stamps.css";

/** The drawing's box, a square of these units, its centre at half of it. */
const VIEW = 100;
const CENTRE = VIEW / 2;
/** The outer ring's radius and the inner dashed ring's, and the ink's width. */
const RING_R = 38;
const INNER_RING_R = 29;
const INK_WIDTH = 6;
const INNER_INK_WIDTH = 3;
/** The inner ring's dash and gap. */
const DASH = "5 4";
/** The star: its points and its tips' and notches' radii. */
const STAR_POINTS = 5;
const STAR_OUTER_R = 20;
const STAR_INNER_R = 8.5;
/** The rosette: its points and its tips' and notches' radii. */
const ROSETTE_POINTS = 12;
const ROSETTE_OUTER_R = 46;
const ROSETTE_INNER_R = 38;
/** The ribbon tails under a level 5 stamp: where each hangs from and how far it reaches. */
const TAIL_TOP = 66;
const TAIL_BOTTOM = 99;
const TAIL_INNER_X = 8;
const TAIL_OUTER_X = 30;
const TAIL_NOTCH = 7;
/** The levels that add each part. */
const STAR_FROM = 2;
const INNER_RING_FROM = 3;
const ROSETTE_FROM = 4;
const TAILS_FROM = 5;

/** A star of `points` points about the centre, its first tip straight up, as an SVG path. */
function starPath(points: number, outer: number, inner: number): string {
  const corners = Array.from({ length: points * 2 }, (_, at) => {
    const radius = at % 2 === 0 ? outer : inner;
    const angle = (Math.PI * at) / points - Math.PI / 2;
    return `${(CENTRE + radius * Math.cos(angle)).toFixed(1)} ${(CENTRE + radius * Math.sin(angle)).toFixed(1)}`;
  });
  return `M${corners.join("L")}Z`;
}

/** One ribbon tail, on the left (`side` -1) or the right (1), its foot notched. */
function tailPath(side: -1 | 1): string {
  const x = (offset: number): string => (CENTRE + side * offset).toFixed(1);
  const middle = (TAIL_INNER_X + TAIL_OUTER_X) / 2;
  return `M${x(TAIL_INNER_X)} ${TAIL_TOP}L${x(TAIL_OUTER_X)} ${TAIL_TOP}L${x(TAIL_OUTER_X)} ${TAIL_BOTTOM}L${x(middle)} ${TAIL_BOTTOM - TAIL_NOTCH}L${x(TAIL_INNER_X)} ${TAIL_BOTTOM}Z`;
}

const STAR = starPath(STAR_POINTS, STAR_OUTER_R, STAR_INNER_R);
const ROSETTE = starPath(ROSETTE_POINTS, ROSETTE_OUTER_R, ROSETTE_INNER_R);
const TAILS = [tailPath(-1), tailPath(1)] as const;

/** R3013002: the stamp of `level`, or nothing for a level no stamp has. */
export function ProficiencyStamp({ level }: { level: number }): ReactElement | null {
  if (!isStampLevel(level)) return null;
  const label = t("Level {level} stamp", { level: String(level) });
  return (
    <span className="cf-stamp" data-stamp={level} role="img" aria-label={label} title={label}>
      <svg viewBox={`0 0 ${VIEW} ${VIEW}`} aria-hidden="true" focusable="false">
        {level >= TAILS_FROM && TAILS.map((tail) => <path key={tail} className="cf-stamp-tail" d={tail} />)}
        {level >= ROSETTE_FROM && <path className="cf-stamp-rim" d={ROSETTE} />}
        <circle className="cf-stamp-rim" cx={CENTRE} cy={CENTRE} r={RING_R} strokeWidth={INK_WIDTH} />
        {level >= INNER_RING_FROM && <circle className="cf-stamp-inner" cx={CENTRE} cy={CENTRE} r={INNER_RING_R} strokeWidth={INNER_INK_WIDTH} strokeDasharray={DASH} />}
        {level >= STAR_FROM && <path className="cf-stamp-star" d={STAR} />}
      </svg>
    </span>
  );
}
