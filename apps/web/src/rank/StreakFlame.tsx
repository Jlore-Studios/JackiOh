// R2101000: a ranked win streak as a flame with its count, on the profile and a ranked match's
// line only. Presentation only (CLAUDE.md rule 7): it draws the server's count, from
// `RANK_STREAK_FLAME_LENGTH` on.

import type { ReactElement } from "react";

import { RANK_STREAK_FLAME_LENGTH } from "@jackioh/server-config";
import { t } from "../i18n/i18n.ts";
import { CREST_SIZE_PX, type RankCrestSize } from "./RankCrest.tsx";

import "./streakFlame.css";

const VIEW_BOX = "0 0 16 16";
const FLAME = "M8 1 C9 4 13 6 13 10 C13 13 10.8 15 8 15 C5.2 15 3 13 3 10 C3 7.5 4.5 6 5.5 4.5 C5.7 6 6.3 7 7 7.5 C7 5 7.5 3 8 1 Z";
const CORE = "M8 8 C9 9.5 10.5 10.5 10.5 12 C10.5 13.4 9.4 14.2 8 14.2 C6.6 14.2 5.5 13.4 5.5 12 C5.5 10.8 6.5 10 7 9.2 C7.3 9.8 7.6 10.1 8 10.3 C8 9.5 8 8.8 8 8 Z";
const FLAME_COLOR = "#ff7a1a";
const CORE_COLOR = "#ffd23f";

/** The streak a flame shows, or null below `RANK_STREAK_FLAME_LENGTH` (a loss sets it to 0). */
export function flameStreak(streak: number): number | null {
  return streak >= RANK_STREAK_FLAME_LENGTH ? streak : null;
}

/** What the flame is named and titled with. */
export function streakWords(count: number): string {
  return t("{count} ranked wins in a row", { count });
}

/** The flame after a space, or nothing (and no stray space) below the threshold. */
export default function StreakFlame({ streak, size }: { streak: number; size: RankCrestSize }): ReactElement | null {
  const count = flameStreak(streak);
  if (count === null) return null;
  const words = streakWords(count);
  const edge = CREST_SIZE_PX[size];
  return (
    <>
      {" "}
      <span className="streak-flame" role="img" aria-label={words} title={words} data-size={size}>
        <svg className="streak-flame__art" viewBox={VIEW_BOX} width={edge} height={edge} aria-hidden="true" focusable="false">
          <path d={FLAME} fill={FLAME_COLOR} />
          <path d={CORE} fill={CORE_COLOR} />
        </svg>
        <span className="streak-flame__count" aria-hidden="true">
          {count}
        </span>
      </span>
    </>
  );
}
