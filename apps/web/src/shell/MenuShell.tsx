// The menus' shell (#3728, docs/polish/10-menus.md): one frame around every menu screen, so a player always
// knows where they are, where everything else is, and what is waiting on them.
//
// - R3728000: every path in `paths` but a board's (`isBoardPath`), the series screen (a game's flow) and the dev
//   screens renders inside it (`menuShellOn`, `main.tsx`). Its navigation is Home, Play, Collection, Shop and
//   Friends: a rail with labels over `tablet` (1025 px and wider), the same rail with icons alone from 601 to
//   1024 px, and a tab bar at the foot of a phone (600 px and under): the width decides, never the device,
//   since an iPad's window can be any width. A board drawn on a menu path (a replay, a dungeon fight) takes the
//   navigation away while it is up, and the screen stays mounted: the screen's column comes first, so a board
//   appearing changes nothing before it, and Tab reaches the screen's Back before the navigation.
// - R3728001: Friends opens the friends panel as a drawer down the right edge (`FriendsButton`'s `rail`).
// - R3728002: the status row (`context.ts`): the live game to go back to, else this tab's queue search with
//   Leave the queue, which the top bar draws under itself (`routes/nav.tsx`) and Home under its corner.
//
// CLAUDE.md rule 7: nothing here is a rule. The live game is what `GET /api/auth/me` said (`net/liveGame.ts`),
// the queue what this tab remembers, and the server still refuses whatever it would.

import { useState, type ReactElement, type ReactNode } from "react";

import { useBoardShown } from "../game/boardShown.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { dequeue, type QueueMode } from "../net/api.ts";
import { useAccount } from "../net/gate.ts";
import { MODE_LABEL, forgetQueued, useLiveGame, useQueued, type LiveGame } from "../net/liveGame.ts";
import { isBoardPath, paths, seriesIdOf } from "../net/navigate.ts";
import { GameBanner, LiveGameBanner } from "../routes/GameBanner.tsx";
import { followInApp } from "../routes/nav.tsx";
import { FriendsButton, FriendsIcon } from "../together/FriendsButton.tsx";
import { attempt } from "../net/errors.ts";
import { MenuShellContext } from "./context.ts";
import { menuShellTestid } from "./testids.ts";

import "./menu-shell.css";

export { menuShellTestid };

/** The dev screens: a hotseat board, the four-seat co-op board and the motif gallery's fixture board. */
const DEV_SCREENS: readonly string[] = [paths.hotseat, paths.duosDev, paths.fxGallery];

/** R3728000: whether the screen at `path` is a menu, drawn inside the shell. */
export function menuShellOn(path: string): boolean {
  return !isBoardPath(path) && seriesIdOf(path) === null && !DEV_SCREENS.includes(path);
}

/** One destination, a real link that moves in place on a plain click (`followInApp`). */
function ShellLink({
  to,
  path,
  label,
  testId,
  icon,
}: {
  to: string;
  path: string;
  label: string;
  testId: string;
  icon: ReactElement;
}): ReactElement {
  return (
    <a
      className="menu-shell__item"
      href={to}
      data-testid={testId}
      title={label}
      aria-current={path === to ? "page" : undefined}
      onClick={followInApp(to)}
    >
      <span className="menu-shell__icon" aria-hidden="true">
        {icon}
      </span>
      <span className="menu-shell__label">{label}</span>
    </a>
  );
}

/** The icons, on a 24-unit square in `currentColor`, as the Friends button's figures are. */
function Icon({ children }: { children: ReactNode }): ReactElement {
  return (
    <svg viewBox="0 0 24 24" width="24" height="24" focusable="false">
      {children}
    </svg>
  );
}

const HOME_ICON = (
  <Icon>
    <path d="M12 3 2.5 11.2l1.3 1.5L5 11.7V21h5.5v-6h3v6H19v-9.3l1.2 1 1.3-1.5z" fill="currentColor" />
  </Icon>
);

/** Two swords, crossed. */
const PLAY_ICON = (
  <Icon>
    <g fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
      <path d="m4 4 10 10M11.5 16.5l5-5M15 15l4 4M20 4 10 14M12.5 16.5l-5-5M9 15l-4 4" />
    </g>
    <circle cx="19.8" cy="19.8" r="1.4" fill="currentColor" />
    <circle cx="4.2" cy="19.8" r="1.4" fill="currentColor" />
  </Icon>
);

/** A hand of three cards. */
const COLLECTION_ICON = (
  <Icon>
    <rect x="3.2" y="5.6" width="9" height="13" rx="1.5" transform="rotate(-12 7.7 12.1)" fill="currentColor" opacity="0.55" />
    <rect x="11.8" y="5.6" width="9" height="13" rx="1.5" transform="rotate(12 16.3 12.1)" fill="currentColor" opacity="0.55" />
    <rect x="7.5" y="4" width="9" height="14" rx="1.5" fill="currentColor" />
  </Icon>
);

/** A shopping bag. */
const SHOP_ICON = (
  <Icon>
    <path d="M8 8V7a4 4 0 0 1 8 0v1" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
    <path d="M4.5 8h15l-1.2 12.2a1 1 0 0 1-1 .8H6.7a1 1 0 0 1-1-.8z" fill="currentColor" />
  </Icon>
);

/** R3728000: the rail, or a phone's tab bar. Friends opens the drawer once the friends' reads answer, and is a link to Play together before. */
function ShellNav({ path }: { path: string }): ReactElement {
  const friends = (
    <ShellLink to={paths.together} path={path} label={t("Friends")} testId={menuShellTestid.friends} icon={<FriendsIcon />} />
  );
  return (
    <nav className="menu-shell__nav" aria-label={t("Main menu")} data-testid={menuShellTestid.nav}>
      <ShellLink to={paths.landing} path={path} label={t("Home")} testId={menuShellTestid.home} icon={HOME_ICON} />
      <ShellLink to={paths.play} path={path} label={t("Play")} testId={menuShellTestid.play} icon={PLAY_ICON} />
      <ShellLink to={paths.collection} path={path} label={t("Collection")} testId={menuShellTestid.collection} icon={COLLECTION_ICON} />
      <ShellLink to={paths.shop} path={path} label={t("Shop")} testId={menuShellTestid.shop} icon={SHOP_ICON} />
      <FriendsButton placement="rail" fallback={friends} />
    </nav>
  );
}

/** R3728002: this tab's queue search, off `/play`, and the way out of it, as `/play`'s Leave the queue. */
function QueueBanner({ mode, token }: { mode: QueueMode; token: string }): ReactElement {
  const [leaving, setLeaving] = useState(false);
  const onPress = (): void => {
    if (leaving) return;
    setLeaving(true);
    // A refusal keeps the search on screen, its button pressable again; `/play` says why.
    attempt(() => dequeue(token))
      .then(forgetQueued, () => undefined)
      .finally(() => {
        setLeaving(false);
      });
  };
  return (
    <GameBanner
      testId={menuShellTestid.queue}
      title="You're in the queue"
      action={{ label: "Leave the queue", testId: menuShellTestid.queueLeave, onPress }}
      data={{ mode }}
    >
      {t("Looking for a {mode} opponent…", { mode: t(MODE_LABEL[mode]) })}
    </GameBanner>
  );
}

/** R3728002: the live game, unless this is its own screen; else the queue search, unless this is `/play`, which shows its own. */
function ShellStatus({
  game,
  queued,
  token,
  path,
}: {
  game: LiveGame | null;
  queued: QueueMode | null;
  token: string | null;
  path: string;
}): ReactElement | null {
  if (game !== null && game.path !== path) return <LiveGameBanner game={game} />;
  if (queued !== null && token !== null && path !== paths.play) return <QueueBanner mode={queued} token={token} />;
  return null;
}

export function MenuShell({ path, children }: { path: string; children: ReactNode }): ReactElement {
  useLocale();
  // R3206000: the screen's own gate shares this read while it is out, and opens on its answer after.
  const account = useAccount();
  const game = useLiveGame(account);
  const queued = useQueued();
  const board = useBoardShown();
  const token = account.kind === "ready" ? account.token : null;
  // A board on screen owns it: its rail's bar (the replay player's BackLink) gets no status row.
  const status = board ? null : <ShellStatus game={game} queued={queued} token={token} path={path} />;

  return (
    <MenuShellContext.Provider value={{ status }}>
      <div className="menu-shell" data-testid={menuShellTestid.root} {...(board ? { "data-board": "" } : {})}>
        <div className="menu-shell__main">{children}</div>
        {board ? null : <ShellNav path={path} />}
      </div>
    </MenuShellContext.Provider>
  );
}
