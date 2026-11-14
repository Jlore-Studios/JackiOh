// The banner that tops a menu while the player has a game to go back to (R765): a saved practice
// game (#476) or a live online one, co-op included (R1566000, #478), one component so both read alike.
// Nothing here is a rule (CLAUDE.md rule 7): the practice banner shows what the device kept
// (`practice/resume.ts`), the online one what `GET /api/auth/me` said (`net/liveGame.ts`).
//
// R3729001: a pairing's reveal (`QueueFound`) names the opponent and their rank, as `GET /api/matches/:id/ranks`
// answered (`OpponentLine`); the read never holds the player back, and the plain line stands without it.

import { useEffect, useId, useState, type ReactElement, type ReactNode } from "react";

import { DISCONNECT_GRACE_SECONDS } from "@jackioh/server-config";
import { t } from "../i18n/i18n.ts";
import { getMatchRanks, type VisibleRank } from "../net/api.ts";
import { withSession } from "../net/gate.ts";
import { MATCH_FOUND_STATUS, type LiveGame } from "../net/liveGame.ts";
import { rankCaption } from "../rank/rank.ts";
import { Named } from "../together/words.tsx";
import { followInApp } from "./nav.tsx";
import "./game-banner.css";

/** Chrome this component invented; `e2e/support/testids.ts` mirrors the strings. */
export const gameBannerTestid = {
  /** The practice game saved with Save and leave, on the practice menu (`data-difficulty`). */
  practice: "practice-resume-banner",
  /** In the practice banner: back into that game. */
  practiceResume: "practice-resume",
  /** The live online game (`data-kind="match|series|duos"`), on the main and practice menus. */
  live: "live-game-banner",
  /** In the live banner: a link back to the board, or to the series screen between games. */
  rejoin: "live-game-rejoin",
  /** R765: "Match found!", on any screen but `/play`, while a pairing takes the player to the game. */
  queueFound: "queue-found",
  /** R3729001: in the "Match found!" notice, the opponent's name and rank, once the server has answered. */
  queueFoundOpponent: "queue-found-opponent",
} as const;

type BannerAction = { label: string; testId: string } & ({ href: string } | { onPress: () => void });

export type GameBannerProps = {
  testId: string;
  title: string;
  /** One line under the title: what the game is, and what to know before going back. */
  children: ReactNode;
  action: BannerAction;
  /** `data-*` attributes for a test to read the banner by. */
  data?: Readonly<Record<string, string>>;
};

export function GameBanner({ testId, title, children, action, data = {} }: GameBannerProps): ReactElement {
  const titleId = useId();
  const attributes = Object.fromEntries(Object.entries(data).map(([key, value]) => [`data-${key}`, value]));
  return (
    <section className="notice game-banner" data-testid={testId} aria-labelledby={titleId} {...attributes}>
      <div className="game-banner__text">
        <h2 className="game-banner__title" id={titleId}>
          {t(title)}
        </h2>
        <p className="game-banner__body">{children}</p>
      </div>
      {"href" in action ? (
        <a
          className="game-banner__action button-primary"
          href={action.href}
          data-testid={action.testId}
          onClick={followInApp(action.href)}
        >
          {t(action.label)}
        </a>
      ) : (
        <button type="button" className="game-banner__action button-primary" data-testid={action.testId} onClick={action.onPress}>
          {t(action.label)}
        </button>
      )}
    </section>
  );
}

/** What the live banner says, by the kind of game; `{seconds}` is the disconnect grace. */
const LIVE_WORDS: Readonly<Record<LiveGame["kind"], { title: string; body: string }>> = {
  match: {
    title: "You're in a game",
    body: "Your online match is still on. If you stay away from it for {seconds} seconds, you lose it.",
  },
  series: {
    title: "You're in a Conquest series",
    body: "Your series is still on, and its next game is waiting for you.",
  },
  duos: {
    title: "You're in a co-op game",
    body: "Your co-op game is still on. If you stay away from it for {seconds} seconds, the AI plays your seat until you come back.",
  },
};

/** R765: the player's live online game and the way back to it, if any. */
export function LiveGameBanner({ game }: { game: LiveGame | null }): ReactElement | null {
  if (game === null) return null;
  const words = LIVE_WORDS[game.kind];
  return (
    <GameBanner
      testId={gameBannerTestid.live}
      title={words.title}
      action={{ label: "Rejoin", testId: gameBannerTestid.rejoin, href: game.path }}
      data={{ kind: game.kind }}
    >
      {t(words.body, { seconds: String(DISCONNECT_GRACE_SECONDS) })}
    </GameBanner>
  );
}

/** The other seat of a match: its name, and its rank when the match is ranked. */
type Opponent = { name: string; rank: VisibleRank | null };

/**
 * The seat across from the player in a `GET /api/matches/:id/ranks` answer, never the player's own (`you`), or
 * null when the answer is no ranks body or the seat has no name. An unranked match (a room's) has no rank to say.
 */
function opponentOf(answer: unknown): Opponent | null {
  if (typeof answer !== "object" || answer === null) return null;
  const { ranked, seats } = answer as { ranked?: unknown; seats?: unknown };
  if (typeof seats !== "object" || seats === null) return null;
  const { p1, p2 } = seats as { p1?: unknown; p2?: unknown };
  for (const seat of [p1, p2]) {
    if (typeof seat !== "object" || seat === null) continue;
    const { you, username, rank } = seat as { you?: unknown; username?: unknown; rank?: unknown };
    if (you !== false) continue;
    if (typeof username !== "string" || username === "") return null;
    const ranks = ranked === true && typeof rank === "object" && rank !== null && typeof (rank as { tier?: unknown }).tier === "string";
    return { name: username, rank: ranks ? (rank as VisibleRank) : null };
  }
  return null;
}

/**
 * R3729001: "You face Max, Normal Grape II." for the match `matchId`, an inline span for the sentence it
 * stands in (the found notice). It is nothing until the read answers, and stays nothing when the read fails or
 * names no opponent, so it never holds the screen: whoever draws it goes on to the game regardless.
 */
export function OpponentLine({ matchId }: { matchId: string }): ReactElement | null {
  const [read, setRead] = useState<{ matchId: string; opponent: Opponent | null } | null>(null);
  useEffect(() => {
    let canceled = false;
    withSession((token) => getMatchRanks(token, matchId)).then(
      (answer) => {
        if (!canceled) setRead({ matchId, opponent: opponentOf(answer) });
      },
      () => undefined,
    );
    return () => {
      canceled = true;
    };
  }, [matchId]);
  const opponent = read !== null && read.matchId === matchId ? read.opponent : null;
  if (opponent === null) return null;
  // `{name}` stays in the sentence for `Named` to draw the username in its place (R1436).
  const sentence = opponent.rank === null ? t("You face {name}.") : t("You face {name}, {rank}.", { rank: rankCaption(opponent.rank) });
  return (
    <span data-testid={gameBannerTestid.queueFoundOpponent}>
      <Named text={sentence} name={opponent.name} />
    </span>
  );
}

/** R765: a pairing on its way, announced on whatever screen the player is on, in `/play`'s words; R3729001: with the opponent, once known. */
export function QueueFound({ game }: { game: LiveGame | null }): ReactElement | null {
  if (game === null) return null;
  return (
    <div className="queue-found tavern">
      <p className="notice queue-found__notice" data-testid={gameBannerTestid.queueFound} data-kind={game.kind} role="status">
        {t(MATCH_FOUND_STATUS)}
        {game.kind === "match" ? (
          <>
            {" "}
            <OpponentLine matchId={game.id} />
          </>
        ) : null}
      </p>
    </div>
  );
}
