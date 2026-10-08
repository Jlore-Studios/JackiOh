// The banner that tops a menu while the player has a game to go back to (R765): a practice game
// they saved with Save and leave, on the practice menu (issue #476), and their live online game, on
// the main menu and the practice menu (issue #478). One component, so the two read as one thing:
// a notice in the tavern's gold hairline, what the game is, and the one way back into it.
//
// Nothing here is a rule (CLAUDE.md rule 7). The practice banner shows what the device kept
// (`practice/resume.ts`); the online one shows what `GET /api/auth/me` said (`net/liveGame.ts`).

import { useId, type ReactElement, type ReactNode } from "react";

import { DISCONNECT_GRACE_SECONDS } from "@jackioh/server-config";
import { MATCH_FOUND_STATUS, type LiveGame } from "../net/liveGame.ts";
import { followInApp } from "./nav.tsx";
import "./game-banner.css";

/** Chrome this component invented; `e2e/support/testids.ts` mirrors the strings. */
export const gameBannerTestid = {
  /** R765: the practice game saved with Save and leave, on the practice menu (`data-difficulty`). */
  practice: "practice-resume-banner",
  /** In the practice banner: back into that game. */
  practiceResume: "practice-resume",
  /** R765: the player's live online game (`data-kind="match|series"`), on the main and practice menus. */
  live: "live-game-banner",
  /** In the live banner: a link back to the board, or to the series screen between games. */
  rejoin: "live-game-rejoin",
  /** R765: "Match found!", on any screen but `/play`, while a pairing takes the player to the game. */
  queueFound: "queue-found",
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
          {title}
        </h2>
        <p className="game-banner__body">{children}</p>
      </div>
      {"href" in action ? (
        <a
          className="game-banner__action"
          href={action.href}
          data-testid={action.testId}
          onClick={followInApp(action.href)}
        >
          {action.label}
        </a>
      ) : (
        <button type="button" className="game-banner__action" data-testid={action.testId} onClick={action.onPress}>
          {action.label}
        </button>
      )}
    </section>
  );
}

/** What the live banner says, by the kind of game. */
const LIVE_WORDS: Readonly<Record<LiveGame["kind"], { title: string; body: string }>> = {
  match: {
    title: "You're in a game",
    body: `Your online match is still on. If you stay away from it for ${String(DISCONNECT_GRACE_SECONDS)} seconds, you lose it.`,
  },
  series: {
    title: "You're in a Conquest series",
    body: "Your series is still on, and its next game is waiting for you.",
  },
};

/** R765: the player's live online game and the way back to it, or nothing when there is none. */
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
      {words.body}
    </GameBanner>
  );
}

/** R765: a pairing on its way, announced on whatever screen the player is on, in `/play`'s words. */
export function QueueFound({ game }: { game: LiveGame | null }): ReactElement | null {
  if (game === null) return null;
  return (
    <div className="queue-found tavern">
      <p className="notice queue-found__notice" data-testid={gameBannerTestid.queueFound} data-kind={game.kind} role="status">
        {MATCH_FOUND_STATUS}
      </p>
    </div>
  );
}
