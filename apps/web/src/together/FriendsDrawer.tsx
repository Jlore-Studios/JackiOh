// #4444: the Friends drawer content, split from the rail button so the first load carries only
// the button, its badge and its polling. The drawer (the friends panel, the chat panel and the
// challenge dialog) arrives lazily through `friendsDrawerChunk.ts` the first time Friends opens.
// Data fetching is untouched: the panel reads the friends list only while it is open, every
// `FRIENDS_POLL_MS`, and again when the tab's social socket says it changed (R3214000). Chat and
// Challenge close the panel and open their own modal over the screen. R3221000: the drawer is the
// one Dialog's sheet or drawer (R3728001); the list it holds is `FriendList.tsx` (R3732002).
//
// CLAUDE.md rule 7: nothing here is a rule. Which friend can be challenged or watched is the server's
// status; Challenge is off while the player is in a game only to say what the server would, and the
// server still refuses it. Every refusal is shown in the server's words.

import { useEffect, useState, type ReactElement } from "react";

import { AccountError } from "../auth/AccountError.tsx";
import Username from "../auth/Username.tsx";
import { Dialog } from "../game/Dialog.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import { getMe, type MeResponse } from "../net/api.ts";
import { keptAccount, withSession } from "../net/gate.ts";
import { bannerGameOf, type LiveGame } from "../net/liveGame.ts";
import { paths } from "../net/navigate.ts";
import { cachedFriends, getFriends, type PlayerRef } from "../net/social.ts";
import { followInApp } from "../routes/nav.tsx";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { ChallengeDialog } from "./ChallengeDialog.tsx";
import { ChatPanel } from "./ChatPanel.tsx";
import { FRIENDS_POLL_MS } from "./config.ts";
import { FriendList } from "./FriendList.tsx";
import { AddFriend, type Challenging } from "./FriendRows.tsx";
import { usePolled } from "./session.ts";
import { noteFriends, onFriendsPush } from "./socialPoll.ts";
import { friendsTestid } from "./testids.ts";
import { useFriendActions } from "./useFriendActions.ts";
import { attempt } from "../net/errors.ts";
import { UndoToast } from "../undo/UndoToast.tsx";

/** What the server says to a challenge from a player in a game (`in_game`, `crates/server/src/api/decks.rs`), by kind of game. */
const IN_GAME_WORDS: Readonly<Record<LiveGame["kind"], string>> = {
  match: "finish your current match first",
  series: "Finish your Conquest series first.",
  duos: "Finish your co-op match first.",
};

/**
 * R3207003: the account the page already read stands for the panel; one that names a game is read again,
 * since that game may have ended and Challenge must not stay off. The server still refuses a challenge
 * from a player in a game (CLAUDE.md rule 7).
 */
function readAccount(): Promise<MeResponse> {
  const kept = keptAccount();
  return kept !== undefined && bannerGameOf(kept) === null ? Promise.resolve(kept) : withSession(getMe);
}

function FriendsPanel({
  id,
  drawer,
  unread,
  onClose,
  onChat,
  onChallenge,
}: {
  id: string;
  /** R3728001: down the right edge, opened from the shell's navigation; a sheet otherwise. */
  drawer: boolean;
  unread: Readonly<Record<string, number>>;
  onClose: () => void;
  onChat: (friend: PlayerRef) => void;
  onChallenge: (friend: Challenging) => void;
}): ReactElement {
  useLocale();
  const [friends, reloadFriends] = usePolled(getFriends, FRIENDS_POLL_MS, cachedFriends);
  const [me] = usePolled(readAccount, null);
  const [copied, setCopied] = useState(false);

  // The badge counts the requests this read held.
  useEffect(() => {
    if (friends.kind === "ready") noteFriends(friends.value);
  }, [friends]);

  // R3214000: the server said the friends list changed (a request, a friend made or lost, a block).
  useEffect(() => onFriendsPush(reloadFriends), [reloadFriends]);

  // A list whatever answered: the panel sits on every screen, a server's older answer must not take one down.
  const list = friends.kind === "ready" ? friends.value : null;
  const requests = Array.isArray(list?.incoming) ? list.incoming : [];
  // R3732002: the same actions as the Friends page's, since the row's menu holds Remove, Block and Invite to lobby.
  const actions = useFriendActions({ friends: list, reloadFriends });

  const name = me.kind === "ready" ? me.value.username?.name : undefined;
  const game = me.kind === "ready" ? bannerGameOf(me.value) : null;
  const challengeRefusal = game === null ? null : IN_GAME_WORDS[game.kind];

  const copy = (): void => {
    if (name === undefined) return;
    // Copy failures stay silent because the name stays on screen.
    attempt(() => navigator.clipboard.writeText(name)).then(
      () => {
        setCopied(true);
      },
      () => undefined,
    );
  };

  // Keyboard and screen-reader users start inside the panel, on its ✕ (Dialog's).
  return (
    <Dialog
      role="dialog"
      sheet={!drawer}
      drawer={drawer}
      id={id}
      title={t("Friends")}
      onClose={onClose}
      actions={[]}
      testid={friendsTestid.panel}
      closeTestid={friendsTestid.close}
      className="friends-panel"
    >
      {name === undefined ? null : (
        <div className="friends-panel__me">
          <span className="friends-panel__label">{t("Username")}</span>
          <Username name={name} />
          <button type="button" data-testid={friendsTestid.copy} onClick={copy}>
            {copied ? t("Copied") : t("Copy")}
          </button>
        </div>
      )}

      <AddFriend onAdded={reloadFriends} />

      {actions.error === null ? null : (
        <p className="notice" data-testid={friendsTestid.error} role="alert">
          {t(actions.error)}
        </p>
      )}

      {friends.kind === "loading" ? <Skeleton variant="rows" label={t("Loading your friends…")} /> : null}
      {friends.kind === "failed" ? (
        <AccountError
          message={friends.message}
          retryAfterMs={friends.retryAfterMs}
          onRetry={reloadFriends}
          testId={friendsTestid.listError}
          retryTestId={friendsTestid.listRetry}
        />
      ) : null}
      <FriendList
        friends={actions.shown}
        requests={requests}
        unread={unread}
        actions={actions}
        challengeRefusal={challengeRefusal}
        onChat={onChat}
        onChallenge={onChallenge}
        onLeave={onClose}
      />

      <a
        href={paths.together}
        className="friends-panel__all"
        data-testid={friendsTestid.all}
        onClick={(event) => {
          onClose();
          followInApp(paths.together)(event);
        }}
      >
        {t("See all friends")}
      </a>
      {/* Inside the dialog, so Undo is a stop of its Tab order (the toast sits at the window's foot, fixed). */}
      <UndoToast held={actions.held} onUndo={actions.undo} />
    </Dialog>
  );
}

export type FriendsDrawerProps = {
  id: string;
  /** R3728001: down the right edge, opened from the shell's navigation; a sheet otherwise. */
  drawer: boolean;
  unread: Readonly<Record<string, number>>;
  open: boolean;
  chatWith: PlayerRef | null;
  challenging: Challenging | null;
  onClosePanel: () => void;
  onChat: (friend: PlayerRef) => void;
  onChallenge: (friend: Challenging) => void;
  onCloseChat: () => void;
  onCloseChallenge: () => void;
  onSentChallenge: () => void;
};

/** #4444: what `FriendsButton`'s portal drew, now behind its own chunk: the panel, the chat, the challenge. */
export function FriendsDrawer({
  id,
  drawer,
  unread,
  open,
  chatWith,
  challenging,
  onClosePanel,
  onChat,
  onChallenge,
  onCloseChat,
  onCloseChallenge,
  onSentChallenge,
}: FriendsDrawerProps): ReactElement {
  return (
    <div className="tavern lobby together-screen friends-layer">
      {open ? (
        <FriendsPanel id={id} drawer={drawer} unread={unread} onClose={onClosePanel} onChat={onChat} onChallenge={onChallenge} />
      ) : null}
      {chatWith === null ? null : <ChatPanel key={chatWith.profileId} friend={chatWith} drawer={drawer} onClose={onCloseChat} />}
      {challenging === null ? null : (
        <ChallengeDialog
          friend={challenging}
          liveRefusal={challenging.liveRefusal}
          {...(challenging.kind === undefined ? {} : { initialKind: challenging.kind })}
          onClose={onCloseChallenge}
          onSent={onSentChallenge}
        />
      )}
    </div>
  );
}

export default FriendsDrawer;
