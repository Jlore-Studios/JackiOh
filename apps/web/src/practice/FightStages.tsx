// The screens a fight against the AI shows around its board, shared by a dungeon run's fight
// (`dungeon/fight/FightScreen.tsx`) and an Infinite Assault fight (`assault/fight/FightScreen.tsx`):
// the shell, the no-setup, stopped and shuffling stages, and the board's Menu. Each screen passes its
// own `data-testid` vocabulary (`fightTestid`, `assaultFightTestid`) and imports the stylesheets
// (`auth/tavern.css`, `practice.css`), so the order they load in stays the screen's. The stages are
// functions that return elements, not components: every screen a fight shows then roots in the one
// `FightShell`, and React keeps its nodes from one stage to the next, as when each screen drew them inline.

import type { ReactElement, ReactNode } from "react";

import { t } from "../i18n/i18n.ts";
import { PageTitle } from "../routes/PageTitle.tsx";

/** The test ids of the stages below, as each fight screen names them. */
export type FightStageTestids = Readonly<Record<"screen" | "loading" | "failed" | "retry" | "noSetup" | "menu", string>>;

/** The setup, loading and failure screens are practice's reading column; the fight is its full-width board. */
export function FightShell({
  variant,
  testids,
  children,
}: {
  variant: "lobby" | "game";
  testids: FightStageTestids;
  children: ReactNode;
}): ReactElement {
  const shell = variant === "game" ? "app-shell app-shell--wide" : "app-shell tavern one-screen";
  return (
    <div className={`${shell} practice practice--${variant}`} data-testid={variant === "game" ? testids.screen : undefined}>
      {children}
    </div>
  );
}

/** The fight could not be set up from what its screen was given. */
export function fightNoSetup(testids: FightStageTestids): ReactElement {
  return (
    <FightShell variant="lobby" testids={testids}>
      <div className="practice-stage">
        <p className="notice" data-testid={testids.noSetup} role="alert">
          {t("This fight could not be set up.")}
        </p>
      </div>
    </FightShell>
  );
}

/** The game stopped (the worker refused it or failed): why, and Try again, which deals it fresh. */
export function fightStopped(testids: FightStageTestids, failure: string | null, onRetry: () => void): ReactElement {
  return (
    <FightShell variant="lobby" testids={testids}>
      <div className="practice-stage">
        <PageTitle>{t("The fight stopped")}</PageTitle>
        <p className="notice" data-testid={testids.failed} role="alert">
          {t(failure ?? "Something went wrong while the game was running.")}
        </p>
        <button type="button" className="practice-play button-primary" data-testid={testids.retry} onClick={onRetry}>
          {t("Try again")}
        </button>
      </div>
    </FightShell>
  );
}

/** The deal on its way. */
export function fightShuffling(testids: FightStageTestids): ReactElement {
  return (
    <FightShell variant="lobby" testids={testids}>
      <div className="practice-stage">
        <div className="practice-shuffle" aria-hidden="true">
          <span />
          <span />
          <span />
        </div>
        <p className="practice-stage__status" data-testid={testids.loading} role="status">
          {t("Shuffling the decks…")}
        </p>
      </div>
    </FightShell>
  );
}

/** The board's Menu: mid-fight its screen asks first (a dialog), and once the fight is `over` it leaves at once. */
export function FightMenuButton({
  testids,
  over,
  onClick,
}: {
  testids: FightStageTestids;
  over: boolean;
  onClick: () => void;
}): ReactElement {
  return (
    <button
      type="button"
      className="control control-practice-menu"
      data-testid={testids.menu}
      aria-haspopup={!over ? "dialog" : undefined}
      aria-label={t("Main menu")}
      title={t("Main menu")}
      onClick={onClick}
    >
      {t("Menu")}
    </button>
  );
}
