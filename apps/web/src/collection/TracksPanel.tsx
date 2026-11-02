// The set tracks panel (#1307, SPEC §14.7): each shipped set's level, XP bar and next rewards, and
// how a locked track unlocks, all `GET /api/tracks`' (CLAUDE.md rule 7: the client knows no curve).
// #1664: the bar is the economy's `FillBar` at rest, a native `<progress>` of `levelXp` holding
// `levelXp − xpToNext`.

import type { ReactElement } from "react";

import { SET_TRACK_LEVELS } from "@jackioh/server-config";

import FillBar from "../economy/FillBar.tsx";
import { formatNumber, t } from "../i18n/i18n.ts";
import type { TrackView } from "../net/api.ts";
import { collectionTestid, lockWords, rewardWords } from "./collection.ts";

export default function TracksPanel({ tracks }: { tracks: readonly TrackView[] }): ReactElement {
  return (
    <ul className="collection-list">
      {tracks.map(({ setId, level, levelXp, xpToNext, nextRewards, unlocked }) => (
        <li
          key={setId}
          className="collection-track"
          data-testid={collectionTestid.track(setId)}
          data-unlocked={String(unlocked)}
        >
          <strong>{t("{set} track", { set: t(setId) })}</strong>
          {unlocked ? null : <span data-testid={collectionTestid.trackLock(setId)}>{lockWords()}</span>}
          <span data-testid={collectionTestid.trackLevel(setId)}>
            {t("Level {level} of {levels}", { level: formatNumber(level), levels: formatNumber(SET_TRACK_LEVELS) })}
          </span>
          {levelXp !== null && xpToNext !== null ? (
            <>
              <FillBar label={t("{set} track XP", { set: t(setId) })} size={levelXp} value={levelXp - xpToNext} />
              <span data-testid={collectionTestid.trackXp(setId)}>
                {t("{xp} XP to the next level", { xp: formatNumber(xpToNext) })}
              </span>
            </>
          ) : (
            <span data-testid={collectionTestid.trackXp(setId)}>{t("Last level reached.")}</span>
          )}
          <span data-testid={collectionTestid.trackRewards(setId)}>
            {nextRewards === null
              ? t("All rewards claimed.")
              : t("Next rewards: {rewards}", { rewards: rewardWords(nextRewards, setId) })}
          </span>
        </li>
      ))}
    </ul>
  );
}
