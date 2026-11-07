// `/leaderboard` — the global ranked ladder (SPEC §9.12, R608, R612): Jlorious #1–#100, then each
// Grape tier. Read-only and decides nothing (CLAUDE.md rule 7): it renders `GET /api/leaderboard`
// through `rank/RankCrest.tsx` (#1650), and the hidden rating never reaches it. A row is keyed by its
// profile id and shows the player's username, never the id (R1436).
// R2778000: a Constructed / All Random switch picks the season's ladder; the address carries All
// Random as `?ladder=random`.

import { useEffect, useState, type ReactElement } from "react";

import Username from "../auth/Username.tsx";
import { t } from "../i18n/i18n.ts";
import { getLeaderboard, keptPaths, ladderPath, type Ladder, type LeaderboardResponse } from "../net/api.ts";
import { cachedRead } from "../net/readCache.ts";
import { divisionNumeral, rankCaption, tierName } from "../rank/rank.ts";
import RankCrest from "../rank/RankCrest.tsx";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { BackLink } from "./nav.tsx";
import { Wordmark } from "./Wordmark.tsx";
import { messageOf } from "../net/errors.ts";
import { Segmented } from "../tabs/index.ts";

import "../auth/tavern.css";
import "./leaderboard.css";

export const leaderboardTestid = {
  screen: "leaderboard-screen",
  loading: "leaderboard-loading",
  error: "leaderboard-error",
  /** #1568: a failed read offers Try again beside the alert. */
  retry: "leaderboard-retry",
  season: "leaderboard-season",
  you: "leaderboard-you",
  jlorious: "leaderboard-jlorious",
  tier: "leaderboard-tier",
  raisins: "leaderboard-raisins",
  /** R2778000: the Constructed / All Random switch. */
  ladder: "leaderboard-ladder",
} as const;

/** R2778000: the switch's two ladders, Constructed first, each with its label in English for `t()`. */
const LADDERS: readonly { ladder: Ladder; label: string }[] = [
  { ladder: "constructed", label: "Constructed" },
  { ladder: "random", label: "All Random" },
];

/** R2778000: the ladder the address asks for: All Random by `?ladder=random`, else Constructed. */
function ladderOfAddress(): Ladder {
  return new URLSearchParams(window.location.search).get("ladder") === "random" ? "random" : "constructed";
}

export type LeaderboardRouteProps = {
  token: string;
};

export default function LeaderboardRoute({ token }: LeaderboardRouteProps): ReactElement {
  const [ladder, setLadder] = useState<Ladder>(ladderOfAddress);
  // R2125000: a revisit starts on the ladder the last visit kept and reads it again behind it.
  const [board, setBoard] = useState<LeaderboardResponse | null>(
    () => cachedRead<LeaderboardResponse>(token, ladderPath(keptPaths.leaderboard, ladderOfAddress())) ?? null,
  );
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);

  useEffect(() => {
    let canceled = false;
    const path = ladderPath(keptPaths.leaderboard, ladder);
    getLeaderboard(token, ladder)
      .then((next) => {
        if (!canceled) setBoard(next);
      })
      .catch((cause: unknown) => {
        if (canceled) return;
        // R2125000: a read that fails behind a kept ladder leaves it and says nothing; only a first visit says why.
        const kept = cachedRead<LeaderboardResponse>(token, path);
        if (kept === undefined) setError(messageOf(cause));
        else setBoard(kept);
      });
    return () => {
      canceled = true;
    };
  }, [token, attempt, ladder]);

  /** R2778000: shows the other ladder, from what it kept if anything, and puts it in the address. */
  const pick = (next: Ladder): void => {
    if (next === ladder) return;
    setLadder(next);
    setError(null);
    setBoard(cachedRead<LeaderboardResponse>(token, ladderPath(keptPaths.leaderboard, next)) ?? null);
    const params = new URLSearchParams(window.location.search);
    if (next === "random") params.set("ladder", "random");
    else params.delete("ladder");
    const query = params.toString();
    window.history.replaceState(window.history.state, "", `${window.location.pathname}${query === "" ? "" : `?${query}`}`);
  };

  return (
    <div className="app-shell tavern leaderboard-screen one-screen" data-testid={leaderboardTestid.screen}>
      <BackLink title={t("Leaderboard")} />
      <section className="panel panel--auth one-screen__frame">
        <Wordmark />
        <Segmented
          label={t("Ladder")}
          testId={leaderboardTestid.ladder}
          className="row"
          value={ladder}
          onChange={pick}
          options={LADDERS.map((entry) => ({
            value: entry.ladder,
            label: t(entry.label),
          }))}
        />
        <div className="one-screen__body one-screen__stack" tabIndex={0}>
          {error !== null ? (
            <>
              <p className="notice" data-testid={leaderboardTestid.error} role="alert">
                {t(error)}
              </p>
              <div className="row">
                <button
                  type="button"
                  className="button-primary"
                  data-testid={leaderboardTestid.retry}
                  onClick={() => {
                    setError(null);
                    setAttempt((before) => before + 1);
                  }}
                >
                  {t("Try again")}
                </button>
              </div>
            </>
          ) : null}

          {board === null && error === null ? (
            <Skeleton variant="rows" label={t("Loading the ladder…")} testId={leaderboardTestid.loading} />
          ) : null}

          {board !== null ? (
            <>
              <p data-testid={leaderboardTestid.season}>{t("Season {season}", { season: board.season })}</p>
              <p data-testid={leaderboardTestid.you}>
                <RankCrest rank={board.you} size="large" />{" "}
                {board.jlorious.some((row) => row.you) || board.tiers.some((tier) => tier.players.some((row) => row.you))
                  ? t("You are {rank}", { rank: rankCaption(board.you) })
                  : t("You are {rank} — play rated games to climb", { rank: rankCaption(board.you) })}
              </p>

              <h2 className="section-heading leaderboard-heading">{t("Jlorious")}</h2>
              {board.jlorious.length === 0 ? (
                <p>{t("No Jlorious players yet this season.")}</p>
              ) : (
                <ol data-testid={leaderboardTestid.jlorious}>
                  {board.jlorious.map((row) => (
                    <li key={row.profileId} data-you={row.you ? "you" : undefined}>
                      <RankCrest rank={{ tier: "jlorious", position: row.position }} size="small" /> <Username name={row.username} />
                      {row.you ? ` ${t("(you)")}` : ""}
                    </li>
                  ))}
                </ol>
              )}

              {board.tiers.map((tier) => (
                <section key={tier.tier} data-testid={leaderboardTestid.tier} data-tier={tier.tier}>
                  <h2 className="section-heading leaderboard-heading">
                    <RankCrest tier={tier.tier} size="small" /> {tierName(tier.tier)} ({tier.count})
                  </h2>
                  {tier.players.length === 0 ? (
                    <p>{t("Nobody here yet.")}</p>
                  ) : (
                    <ul>
                      {tier.players.map((row) => (
                        <li key={row.profileId} data-you={row.you ? "you" : undefined}>
                          <Username name={row.username} /> · <RankCrest tier={tier.tier} division={row.division} size="small" />{" "}
                          {t("Division {division}", { division: divisionNumeral(row.division) })} ·{" "}
                          {row.pips === 1 ? t("{n} pip", { n: row.pips }) : t("{n} pips", { n: row.pips })}
                          {row.you ? ` ${t("(you)")}` : ""}
                        </li>
                      ))}
                    </ul>
                  )}
                </section>
              ))}

              <p data-testid={leaderboardTestid.raisins}>
                {board.raisins === 0
                  ? t("Nobody is playing placements right now.")
                  : board.raisins === 1
                    ? t("{n} player is still playing placements.", { n: board.raisins })
                    : t("{n} players are still playing placements.", { n: board.raisins })}
              </p>
            </>
          ) : null}
        </div>
      </section>
    </div>
  );
}
