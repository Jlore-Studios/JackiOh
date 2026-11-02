// The Friends button (#1831): on every screen a signed-in account has, a match included
// (`placement`: `routes/nav.tsx`'s top bar, a board's control bar, a phone's match menu, #2645), so
// friends are one press away. Its badge reads what the shell's one poll published (`socialPoll.ts`),
// so the button adds no request of its own.
//
// The panel it opens reads the friends list only while it is open, every `FRIENDS_POLL_MS`. Chat and
// Challenge close it and open their own modal over the screen, which is left as it was. A modal like
// the client's others (`ChallengeDialog.tsx`): the focus stays inside and returns to this button.
//
// CLAUDE.md rule 7: nothing here is a rule. Which friend can be challenged or watched is the server's
// status; Challenge is off while the player is in a game only to say what the server would, and the
// server still refuses it. Every refusal is shown in the server's words.

import { useCallback, useEffect, useId, useRef, useState, type ReactElement } from "react";
import { createPortal } from "react-dom";

import { AccountError } from "../auth/AccountError.tsx";
import Username from "../auth/Username.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import { getMe } from "../net/api.ts";
import { withSession } from "../net/gate.ts";
import { bannerGameOf, type LiveGame } from "../net/liveGame.ts";
import { paths } from "../net/navigate.ts";
import { readSession } from "../net/session.ts";
import {
  acceptFriendRequest,
  cachedFriends,
  declineFriendRequest,
  getFriends,
  type Friend,
  type PlayerRef,
} from "../net/social.ts";
import { followInApp } from "../routes/nav.tsx";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { ChallengeDialog } from "./ChallengeDialog.tsx";
import { ChatPanel } from "./ChatPanel.tsx";
import { FRIENDS_POLL_MS } from "./config.ts";
import { AddFriend, FriendRow, RequestRow } from "./FriendRows.tsx";
import { tabStops, useModalKeys } from "./modalKeys.ts";
import { usePolled } from "./session.ts";
import { friendsBadgeCount, noteFriends, reloadSocial, useSocialReads } from "./socialPoll.ts";
import { friendsTestid, togetherTestid } from "./testids.ts";
import { attempt, messageOf } from "../net/errors.ts";

import "../auth/tavern.css";
import "../routes/lobby.css";
import "./together.css";

/** What the server says to a challenge from a player in a game (`in_game`, `crates/server/src/api/decks.rs`), by kind of game. */
const IN_GAME_WORDS: Readonly<Record<LiveGame["kind"], string>> = {
  match: "finish your current match first",
  series: "Finish your Conquest series first.",
  duos: "Finish your co-op match first.",
};

/** The order the panel lists friends in: online first, then in a game, then offline. */
const STATUS_ORDER: readonly Friend["status"][] = ["online", "inMatch", "offline"];

/** The friends in the panel's order; the server's order holds within a status. */
function onlineFirst(friends: readonly Friend[]): Friend[] {
  return [...friends].sort((a, b) => STATUS_ORDER.indexOf(a.status) - STATUS_ORDER.indexOf(b.status));
}

function FriendsPanel({
  id,
  unread,
  onClose,
  onChat,
  onChallenge,
}: {
  id: string;
  unread: Readonly<Record<string, number>>;
  onClose: () => void;
  onChat: (friend: PlayerRef) => void;
  onChallenge: (friend: PlayerRef) => void;
}): ReactElement {
  useLocale();
  const titleId = useId();
  const panel = useRef<HTMLDivElement>(null);
  const [friends, reloadFriends] = usePolled(getFriends, FRIENDS_POLL_MS, cachedFriends);
  const [me] = usePolled(() => withSession(getMe), null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useModalKeys(panel, onClose);

  // Keyboard and screen-reader users start inside the panel.
  useEffect(() => {
    const element = panel.current;
    if (element !== null) tabStops(element)[0]?.focus({ preventScroll: true });
  }, []);

  // The badge counts the requests this read held.
  useEffect(() => {
    if (friends.kind === "ready") noteFriends(friends.value);
  }, [friends]);

  /** One of the panel's own changes, then the list read again. */
  const act = (work: () => Promise<unknown>): void => {
    if (busy) return;
    setBusy(true);
    setError(null);
    attempt(work)
      .then(
        () => undefined,
        (cause: unknown) => {
          setError(messageOf(cause));
        },
      )
      .finally(() => {
        setBusy(false);
        reloadFriends();
      });
  };

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

  // A list whatever answered: the panel sits on every screen, a server's older answer must not take one down.
  const list = friends.kind === "ready" ? friends.value : null;
  const all = Array.isArray(list?.friends) ? list.friends : [];
  const requests = Array.isArray(list?.incoming) ? list.incoming : [];

  return (
    <div
      className="friends-panel-scrim"
      onClick={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div ref={panel} id={id} className="friends-panel" data-testid={friendsTestid.panel} role="dialog" aria-modal="true" aria-labelledby={titleId}>
        <div className="friends-panel__head">
          <h2 className="friends-panel__title" id={titleId}>
            {t("Friends")}
          </h2>
          <button type="button" className="link-button" data-testid={friendsTestid.close} aria-label={t("Close")} onClick={onClose}>
            ×
          </button>
        </div>

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

        {error === null ? null : (
          <p className="notice" data-testid={friendsTestid.error} role="alert">
            {t(error)}
          </p>
        )}

        {requests.length === 0 ? null : (
          <ul className="together-list">
            {requests.map((request) => (
              <RequestRow
                key={request.requestId}
                request={request}
                direction="incoming"
                busy={busy}
                onAccept={() => {
                  act(() => acceptFriendRequest(request.requestId));
                }}
                onDecline={() => {
                  act(() => declineFriendRequest(request.requestId));
                }}
              />
            ))}
          </ul>
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
        {list !== null && all.length === 0 ? (
          <p className="lobby-note" data-testid={togetherTestid.noFriends}>
            {t("No friends yet. Ask a friend for their username, with its #tag if it has one, and add them above.")}
          </p>
        ) : null}
        {all.length > 0 ? (
          <ul className="together-list">
            {onlineFirst(all).map((friend) => (
              <FriendRow
                key={friend.profileId}
                friend={friend}
                unread={unread[friend.profileId] ?? 0}
                busy={busy}
                challengeRefusal={challengeRefusal}
                onChallenge={() => {
                  onChallenge({ profileId: friend.profileId, username: friend.username });
                }}
                onChat={() => {
                  onChat({ profileId: friend.profileId, username: friend.username });
                }}
                onWatch={onClose}
              />
            ))}
          </ul>
        ) : null}

        <a
          href={paths.together}
          className="friends-panel__all"
          data-testid={friendsTestid.all}
          onClick={(event) => {
            onClose();
            followInApp(paths.together)(event);
          }}
        >
          {t("All of Play together")}
        </a>
      </div>
    </div>
  );
}

/** Two figures, side by side, on a 24-unit square, filled in `currentColor` so every skin's button color applies. */
function FriendsIcon(): ReactElement {
  return (
    <svg viewBox="0 0 24 24" width="20" height="20" focusable="false">
      <circle cx="9" cy="8" r="3.2" fill="currentColor" />
      <path d="M2.5 19.5c0-3.6 2.9-6 6.5-6s6.5 2.4 6.5 6z" fill="currentColor" />
      <circle cx="17" cy="9" r="2.6" fill="currentColor" />
      <path d="M16.6 13.6c3 0 5.4 1.9 5.4 4.9h-4.6c0-1.9-.5-3.5-1.5-4.6z" fill="currentColor" />
    </svg>
  );
}

export function FriendsButton({ placement }: { placement: "game" | "nav" | "menu" }): ReactElement | null {
  useLocale();
  const reads = useSocialReads();
  const panelId = useId();
  const button = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);
  const [chatWith, setChatWith] = useState<PlayerRef | null>(null);
  const [challenging, setChallenging] = useState<PlayerRef | null>(null);

  /** Gives the focus back to the button, as a modal should leave it (WAI-ARIA APG; `SettingsButton.tsx`). */
  const refocus = useCallback((): void => {
    const element = button.current;
    if (element === null || !element.isConnected) return;
    // #2645: in a phone's match menu, closed by now (the press that opened the panel closed it): the
    // control that opens that menu is the one to land on.
    const closed = element.closest<HTMLElement>("[hidden]");
    const opener = closed === null || closed.id === "" ? null : document.querySelector<HTMLElement>(`[aria-controls="${closed.id}"]`);
    (opener ?? element).focus({ preventScroll: true });
  }, []);

  const closePanel = useCallback((): void => {
    setOpen(false);
    refocus();
  }, [refocus]);

  const closeChat = useCallback((): void => {
    setChatWith(null);
    reloadSocial();
    refocus();
  }, [refocus]);

  const closeChallenge = useCallback((): void => {
    setChallenging(null);
    refocus();
  }, [refocus]);

  const sentChallenge = useCallback((): void => {
    setChallenging(null);
    reloadSocial();
    refocus();
  }, [refocus]);

  // Signed out, an account the server refuses, or no read of the challenges yet: no friends to show.
  if (readSession() === null || reads.challengeReads === 0 || reads.refused) return null;

  const count = friendsBadgeCount(reads);

  return (
    <>
      <button
        ref={button}
        type="button"
        className={`friends-button friends-button--${placement}`}
        data-testid={friendsTestid.button}
        aria-label={count > 0 ? t("Friends, {count} waiting for you", { count }) : t("Friends")}
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-controls={open ? panelId : undefined}
        title={t("Friends")}
        onClick={() => {
          setOpen(true);
        }}
      >
        <span aria-hidden="true" className="friends-button__icon">
          <FriendsIcon />
        </span>
        {placement === "menu" ? <span className="friends-button__label">{t("Friends")}</span> : null}
        {count > 0 ? (
          <span className="together-badge friends-button__badge" data-testid={friendsTestid.badge} aria-hidden="true">
            {count}
          </span>
        ) : null}
      </button>
      {open || chatWith !== null || challenging !== null
        ? createPortal(
            <div className="tavern lobby together-screen friends-layer">
              {open ? (
                <FriendsPanel
                  id={panelId}
                  unread={reads.unread}
                  onClose={closePanel}
                  onChat={(friend) => {
                    setOpen(false);
                    setChatWith(friend);
                  }}
                  onChallenge={(friend) => {
                    setOpen(false);
                    setChallenging(friend);
                  }}
                />
              ) : null}
              {chatWith === null ? null : <ChatPanel key={chatWith.profileId} friend={chatWith} onClose={closeChat} />}
              {challenging === null ? null : <ChallengeDialog friend={challenging} onClose={closeChallenge} onSent={sentChallenge} />}
            </div>,
            document.body,
          )
        : null}
    </>
  );
}
