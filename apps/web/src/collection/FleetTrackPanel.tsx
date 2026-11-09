// R3335001: the Fleet track panel: its level out of FLEET_TRACK_LEVELS, the economy's `FillBar` at rest of the
// level's XP (`levelXp − xpToNext` of `levelXp`, as TracksPanel's), the XP paid today out of
// FLEET_TRACK_XP_DAILY_MAX, and the next levels' rewards, each drawn as its look (`CosmeticLook`: a Starship reward as
// a Starship). Everything is `GET /api/tracks`' `fleet` (CLAUDE.md rule 7: the client knows no curve and pays nothing).

import type { ReactElement } from "react";

import { FLEET_TRACK_LEVELS, FLEET_TRACK_XP_DAILY_MAX } from "@jackioh/server-config";

import FillBar from "../economy/FillBar.tsx";
import { formatNumber, t } from "../i18n/i18n.ts";
import type { FleetTrackView } from "../wire/generated/FleetTrackView.ts";
import { COSMETICS } from "../wire/generated/cosmetics.ts";
import { collectionTestid as T, earnWords } from "./collection.ts";
import { CosmeticLook } from "./CosmeticLook.tsx";

/** One of the next rewards: its look, its name and the level that pays it. An id this client does not know shows nothing. */
function FleetReward({ id }: { id: string }): ReactElement | null {
  const cosmetic = COSMETICS.find((entry) => entry.id === id);
  if (cosmetic === undefined) return null;
  const earned = earnWords(cosmetic);
  return (
    <li className="collection-fleet__reward" data-testid={T.fleetReward(id)} data-rarity={cosmetic.rarity}>
      <CosmeticLook id={id} cardId={null} />
      <span className="collection-tile__name">{t(cosmetic.name)}</span>
      {earned === null ? null : <span className="collection-tile__rarity">{earned}</span>}
    </li>
  );
}

export default function FleetTrackPanel({ fleet }: { fleet: FleetTrackView }): ReactElement {
  const { level, levelXp, xpToNext, xpToday, nextRewards } = fleet;
  return (
    <div className="collection-track collection-fleet" data-testid={T.fleet}>
      <span data-testid={T.fleetLevel}>
        {t("Level {level} of {levels}", { level: formatNumber(level), levels: formatNumber(FLEET_TRACK_LEVELS) })}
      </span>
      {levelXp !== null && xpToNext !== null ? (
        <>
          <FillBar label={t("Fleet track XP")} size={levelXp} value={levelXp - xpToNext} />
          <span data-testid={T.fleetXp}>{t("{xp} XP to the next level", { xp: formatNumber(xpToNext) })}</span>
        </>
      ) : (
        <span data-testid={T.fleetXp}>{t("Last level reached.")}</span>
      )}
      <span data-testid={T.fleetToday}>
        {t("Fleet XP today: {xp} of {max}", { xp: formatNumber(xpToday), max: formatNumber(FLEET_TRACK_XP_DAILY_MAX) })}
      </span>
      {nextRewards.length === 0 ? (
        <span data-testid={T.fleetRewards}>{t("All rewards claimed.")}</span>
      ) : (
        <ul className="collection-fleet__rewards" data-testid={T.fleetRewards} aria-label={t("Next rewards")}>
          {nextRewards.map((id) => (
            <FleetReward key={id} id={id} />
          ))}
        </ul>
      )}
    </div>
  );
}
