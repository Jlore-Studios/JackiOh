// Leaderboard: every row of a ladder, people and bots alike, as the server orders them (highest rating
// first). A bot's strength and persona are shown beside its row, read-only; a hand-set rating is marked.

import { useId, useState, type ReactElement } from "react";

import { shown, useRead, type SectionProps } from "./section.ts";
import { testids } from "./testids.ts";
import { LADDERS, type AdminLeaderboardRow, type Ladder, type VisibleRank } from "./types.ts";

const LADDER_NAMES: Readonly<Record<Ladder, string>> = { constructed: "Constructed", random: "Random" };

const capitalized = (word: string): string => word.charAt(0).toUpperCase() + word.slice(1);

/** A rank as the server sent it, every part it carries and nothing worked out. */
export function rankText(rank: VisibleRank): string {
  const parts = [capitalized(rank.tier)];
  if (rank.division !== undefined) parts.push(`division ${String(rank.division)}`);
  if (rank.pips !== undefined) {
    parts.push(`${String(rank.pips)}${rank.pipsPerDivision === undefined ? "" : `/${String(rank.pipsPerDivision)}`} pips`);
  }
  if (rank.position !== undefined) parts.push(`#${String(rank.position)}`);
  if (rank.placementsPlayed !== undefined) {
    parts.push(
      `placements ${String(rank.placementsPlayed)}${rank.placementGames === undefined ? "" : `/${String(rank.placementGames)}`}`,
    );
  }
  return parts.join(" · ");
}

function botText(row: AdminLeaderboardRow): string {
  if (row.bot === null) return "";
  return `strength ${shown(row.bot.strength)}, ${row.bot.persona}${row.bot.retired ? ", retired" : ""}`;
}

export function Leaderboard({ api, revision }: SectionProps): ReactElement {
  const ids = useId();
  const [ladder, setLadder] = useState<Ladder>("constructed");
  const read = useRead(() => api.leaderboard(ladder), [api, ladder, revision]);
  // The last answer is another ladder's until this one's arrives.
  const board = read.value !== null && read.value.ladder === ladder ? read.value : null;

  return (
    <section data-testid={testids.leaderboard} aria-labelledby={`${ids}-heading`}>
      <h2 id={`${ids}-heading`}>Leaderboard</h2>
      <div className="row" role="group" aria-label="Ladder">
        {LADDERS.map((name) => (
          <button
            key={name}
            type="button"
            data-testid={testids.ladder(name)}
            aria-pressed={ladder === name}
            onClick={() => {
              setLadder(name);
            }}
          >
            {LADDER_NAMES[name]}
          </button>
        ))}
      </div>
      {read.error === null ? null : (
        <p className="error" data-testid={testids.loadError}>
          {read.error}
        </p>
      )}
      {board === null ? (
        <p>…</p>
      ) : (
        <>
          <p data-testid={testids.leaderboardSeason}>{`Season ${board.season}, ${LADDER_NAMES[ladder]}: ${String(board.rows.length)} rows`}</p>
          <div className="scroll">
            <table>
              <thead>
                <tr>
                  <th scope="col">Player</th>
                  <th scope="col">Kind</th>
                  <th scope="col">Rating</th>
                  <th scope="col">Games</th>
                  <th scope="col">W / L / D</th>
                  <th scope="col">Rank</th>
                  <th scope="col">Hand-set</th>
                  <th scope="col">Bot</th>
                </tr>
              </thead>
              <tbody>
                {board.rows.map((row) => (
                  <tr
                    key={row.profileId}
                    className={row.you ? "you" : undefined}
                    data-testid={testids.leaderboardRow}
                    data-profile-id={row.profileId}
                    data-kind={row.kind}
                    data-hand-set={String(row.handSet)}
                    data-you={String(row.you)}
                  >
                    <td>{row.you ? `${row.username} (you)` : row.username}</td>
                    <td>
                      <span className={`tag ${row.kind}`}>{row.kind === "bot" ? "Bot" : "Person"}</span>
                    </td>
                    <td>{shown(row.rating)}</td>
                    <td>{String(row.games)}</td>
                    <td>{`${String(row.wins)} / ${String(row.losses)} / ${String(row.draws)}`}</td>
                    <td>{rankText(row.rank)}</td>
                    <td>{row.handSet ? "Hand-set" : ""}</td>
                    <td>{botText(row)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </>
      )}
    </section>
  );
}
