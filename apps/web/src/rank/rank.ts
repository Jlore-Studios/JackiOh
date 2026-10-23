// A visible rank in a player's words (SPEC §9.12, R612). The hidden rating never reaches the
// client, so these words are all there is: a tier, a division in numerals, pips, or a Jlorious
// position. Shared by the leaderboard, the account screen, the match screen and the Play screen so
// they cannot disagree about what a rank says. `RankCrest.tsx` draws these ranks as crests (#1650).

import { t } from "../i18n/i18n.ts";
import type { GrapeTier, PeakBadge, VisibleRank } from "../net/api.ts";

/** Every tier a rank can name, in a player's words. */
export function tierName(tier: VisibleRank["tier"] | GrapeTier): string {
  switch (tier) {
    case "raisin":
      return t("Raisin");
    case "rotten":
      return t("Rotten Grape");
    case "normal":
      return t("Normal Grape");
    case "large":
      return t("Large Grape");
    case "golden":
      return t("Golden Grape");
    case "mythic":
      return t("Mythic Grape");
    case "jlorious":
      return t("Jlorious");
  }
}

/** Every tier a rank can name, shortened for a phone's rank line (no "Grape"). */
export function tierShortName(tier: VisibleRank["tier"] | GrapeTier): string {
  switch (tier) {
    case "raisin":
      return t("Raisin");
    case "rotten":
      return t("Rotten");
    case "normal":
      return t("Normal");
    case "large":
      return t("Large");
    case "golden":
      return t("Golden");
    case "mythic":
      return t("Mythic");
    case "jlorious":
      return t("Jlorious");
  }
}

/** A Grape division (3 at the bottom, 1 at the top) as the numeral the client prints. */
export function divisionNumeral(division: number): string {
  switch (division) {
    case 3:
      return "III";
    case 2:
      return "II";
    case 1:
      return "I";
    default:
      return String(division);
  }
}

/** What a player is shown as: placements, a tier with its division and pips, or a position. */
export function rankWords(rank: VisibleRank): string {
  if (rank.tier === "raisin") {
    return t("Raisin · {played}/{games} placements", {
      played: String(rank.placementsPlayed),
      games: String(rank.placementGames),
    });
  }
  if (rank.tier === "jlorious") return t("Jlorious #{position}", { position: String(rank.position) });
  return t("{tier} {division} · {pips}/{total} pips", {
    tier: tierName(rank.tier),
    division: divisionNumeral(rank.division),
    pips: String(rank.pips),
    total: String(rank.pipsPerDivision),
  });
}

/** The words beside a crest (#1650): the tier and its division numeral, or the tier alone for Raisin and Jlorious. The crest draws the pips, placements and position. */
export function rankCaption(rank: VisibleRank | PeakBadge): string {
  if (rank.tier === "raisin" || rank.tier === "jlorious") return tierName(rank.tier);
  return `${tierName(rank.tier)} ${divisionNumeral(rank.division)}`;
}

/** `rankCaption` without "Grape", for the phone's rank line beside the crest. */
export function rankShortWords(rank: VisibleRank): string {
  if (rank.tier === "raisin" || rank.tier === "jlorious") return tierShortName(rank.tier);
  return `${tierShortName(rank.tier)} ${divisionNumeral(rank.division)}`;
}

/** A season's best, as the profile's badge shows it (R607). */
export function badgeWords(badge: PeakBadge): string {
  if (badge.tier === "jlorious") return `${t("Jlorious #{position}", { position: String(badge.position) })} · ${badge.seasonId}`;
  return `${tierName(badge.tier)} ${divisionNumeral(badge.division)} · ${badge.seasonId}`;
}
