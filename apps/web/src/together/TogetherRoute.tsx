// `/together` — Play together (#936, SPEC S8, B54–B59): friends with their status, requests both
// ways, the players this account blocked, the challenges it sent, the replays friends shared with it,
// chat with any friend, and Duos co-op (`duos/CoopPanel.tsx`, which gets the friends list).
//
// Behind the gate like `/play` (B54). Every list is the server's answer (`net/social.ts`), read again
// without a reload: the friends list (statuses, requests, blocks) every `FRIENDS_POLL_MS`, the pace a
// status changes at, and the challenges and unread counts every `SOCIAL_POLL_MS` (`socialPoll.ts`, the
// one read of them every screen shares); and each list again at once after each of this page's own
// changes. Nothing here is a rule (CLAUDE.md rule 7): which friend can be challenged or watched is what the server says of them
// (`status`, `watchable`), and every refusal is shown in the server's words.
//
// A friend row offers (B56): Challenge while the friend is online and not in a game; Watch while the
// server says the game is `watchable`; Chat always; Remove and Block, each asking once.

import { useEffect, useRef, useState, type ReactElement } from "react";

import { AccountError } from "../auth/AccountError.tsx";
import Username from "../auth/Username.tsx";
import { CoopPanel } from "../duos/CoopPanel.tsx";
import { CatalogContext } from "../game/catalog.ts";
import { formatDateTime, t, useLocale } from "../i18n/i18n.ts";
import { withSession } from "../net/gate.ts";
import {
  acceptFriendRequest,
  blockPlayer,
  cancelChallenge,
  cancelFriendRequest,
  declineFriendRequest,
  getFriends,
  getSharedReplays,
  removeFriend,
  unblockPlayer,
  type Friend,
  type PlayerRef,
  type SharedReplay,
} from "../net/social.ts";
import OpenedReplay from "../replay/OpenedReplay.tsx";
import { openOnlineMatchReplay } from "../replay/sources.ts";
import { BackLink } from "../routes/nav.tsx";
import { SHARED_ANCHOR } from "./anchors.ts";
import { useCatalogLookup } from "./catalog.ts";
import { ChallengeDialog } from "./ChallengeDialog.tsx";
import { ChatPanel } from "./ChatPanel.tsx";
import { FRIENDS_POLL_MS } from "./config.ts";
import { AddFriend, FriendRow, RequestRow } from "./FriendRows.tsx";
import { attempt, messageOf, usePolled } from "./session.ts";
import { noteFriends, reloadSocial, useSocialPoll } from "./socialPoll.ts";
import { SpectatorSwitch } from "./SpectatorSwitch.tsx";
import { togetherTestid } from "./testids.ts";
import { MODE_WORDS, Named } from "./words.tsx";

import "../auth/tavern.css";
import "../routes/lobby.css";
import "./together.css";

const NO_FRIENDS: readonly Friend[] = [];

/** An ISO time as a date and time in the reader's language, or the text itself when it is not one. */
function when(iso: string): string {
  const at = Date.parse(iso);
  return Number.isFinite(at) ? formatDateTime(at, { dateStyle: "medium", timeStyle: "short" }) : iso;
}

export function TogetherRoute(): ReactElement {
  useLocale();
  const [friends, reloadFriends] = usePolled(getFriends, FRIENDS_POLL_MS);
  // The challenges and unread counts are the tab's one read of them (`socialPoll.ts`), shared with the shell's toast.
  const social = useSocialPoll();
  const [shared, reloadShared] = usePolled(getSharedReplays, null);
  const lookup = useCatalogLookup();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [chatWith, setChatWith] = useState<PlayerRef | null>(null);
  const [challenging, setChallenging] = useState<PlayerRef | null>(null);
  const [watching, setWatching] = useState<SharedReplay | null>(null);
  const challengeOpener = useRef<HTMLElement | null>(null);

  // The Friends button's badge counts the requests this page last read.
  useEffect(() => {
    if (friends.kind === "ready") noteFriends(friends.value);
  }, [friends]);

  // "Shared with me →" on `/replays` opens this page at its list.
  useEffect(() => {
    if (window.location.hash !== `#${SHARED_ANCHOR}`) return;
    const list = document.getElementById(SHARED_ANCHOR);
    // jsdom has no scrolling.
    if (list !== null && typeof list.scrollIntoView === "function") list.scrollIntoView();
  }, [shared.kind]);

  /** One of this page's own changes, then the lists it touched read again. */
  const act = (work: () => Promise<unknown>, after: () => void = reloadFriends): void => {
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
        after();
      });
  };

  /**
   * Closes the challenge dialog and gives the focus back to the Challenge button that opened it, as a
   * dialog should leave it (WAI-ARIA APG; `Game.tsx`'s `refocusConcede`). The button is kept when it is
   * pressed, not read off the focus, which a click on a button does not move in every browser.
   */
  const closeChallenge = (): void => {
    setChallenging(null);
    const opener = challengeOpener.current;
    challengeOpener.current = null;
    if (opener !== null && opener.isConnected) opener.focus({ preventScroll: true });
  };

  if (watching !== null) {
    return (
      <CatalogContext.Provider value={lookup}>
        <OpenedReplay
          key={watching.matchId}
          load={() => withSession((token) => openOnlineMatchReplay(token, watching.matchId))}
          onLeave={() => {
            setWatching(null);
          }}
        />
      </CatalogContext.Provider>
    );
  }

  const list = friends.kind === "ready" ? friends.value : null;
  const counts = social.unread;
  const outgoingChallenges = social.outgoing;

  return (
    <div className="app-shell tavern lobby together-screen one-screen" data-testid={togetherTestid.root}>
      <BackLink />

      <header className="together-hero">
        <div className="brand">
          <h1>JackiOh</h1>
        </div>
        <p className="together-hero__title">{t("Play together")}</p>
        <p className="together-hero__lead">
          {t("Add friends by their username, see who is on, challenge them, watch their games, share replays and chat.")}
        </p>
      </header>

      {error === null ? null : (
        <p className="notice" data-testid={togetherTestid.error} role="alert">
          {t(error)}
        </p>
      )}

      <div className="together-layout one-screen__body">
        <section className="lobby-card together-panel" aria-labelledby="together-friends-heading">
          <h2 id="together-friends-heading" className="together-panel__heading">
            {t("Friends")}
          </h2>
          <AddFriend
            onAdded={() => {
              reloadFriends();
            }}
          />
          {friends.kind === "loading" ? (
            <p className="lobby-note" role="status">
              {t("Loading your friends…")}
            </p>
          ) : null}
          {friends.kind === "failed" ? (
            <AccountError
              message={friends.message}
              retryAfterMs={friends.retryAfterMs}
              onRetry={reloadFriends}
              testId={togetherTestid.friendsError}
              retryTestId={togetherTestid.friendsRetry}
            />
          ) : null}
          {list !== null && list.friends.length === 0 ? (
            <p className="lobby-note" data-testid={togetherTestid.noFriends}>
              {t("No friends yet. Ask a friend for their username, with its #tag if it has one, and add them above.")}
            </p>
          ) : null}
          {list !== null && list.friends.length > 0 ? (
            <ul className="together-list">
              {list.friends.map((friend) => (
                <FriendRow
                  key={friend.profileId}
                  friend={friend}
                  unread={counts[friend.profileId] ?? 0}
                  busy={busy}
                  onChallenge={(opener) => {
                    challengeOpener.current = opener;
                    setChallenging({ profileId: friend.profileId, username: friend.username });
                  }}
                  onChat={() => {
                    setChatWith({ profileId: friend.profileId, username: friend.username });
                  }}
                  onRemove={() => {
                    act(() => removeFriend(friend.profileId));
                  }}
                  onBlock={() => {
                    act(() => blockPlayer(friend.profileId));
                  }}
                />
              ))}
            </ul>
          ) : null}
        </section>

        <div className="together-side">
          {list !== null && (list.incoming.length > 0 || list.outgoing.length > 0) ? (
            <section className="lobby-card together-panel" aria-labelledby="together-requests-heading">
              <h2 id="together-requests-heading" className="together-panel__heading">
                {t("Friend requests")}
              </h2>
              <ul className="together-list">
                {list.incoming.map((request) => (
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
                    onCancel={() => undefined}
                    onBlock={() => {
                      act(() => blockPlayer(request.from.profileId));
                    }}
                  />
                ))}
                {list.outgoing.map((request) => (
                  <RequestRow
                    key={request.requestId}
                    request={request}
                    direction="outgoing"
                    busy={busy}
                    onAccept={() => undefined}
                    onDecline={() => undefined}
                    onCancel={() => {
                      act(() => cancelFriendRequest(request.requestId));
                    }}
                    onBlock={() => undefined}
                  />
                ))}
              </ul>
            </section>
          ) : null}

          {outgoingChallenges.length === 0 ? null : (
            <section className="lobby-card together-panel" aria-labelledby="together-challenges-heading">
              <h2 id="together-challenges-heading" className="together-panel__heading">
                {t("Your challenges")}
              </h2>
              <p className="lobby-note">{t("You will be taken to the game as soon as your friend joins.")}</p>
              <ul className="together-list">
                {outgoingChallenges.map((challenge) => (
                  <li
                    key={challenge.challengeId}
                    className="together-row"
                    data-testid={togetherTestid.outgoingChallenge}
                    data-challenge-id={challenge.challengeId}
                    data-room-code={challenge.roomCode}
                  >
                    <span className="together-row__who">
                      <Named
                        text={t("{mode} against {name}, room {code}", { mode: t(MODE_WORDS[challenge.mode]), code: challenge.roomCode })}
                        name={challenge.to.username}
                      />
                    </span>
                    <span className="together-row__actions">
                      <button
                        type="button"
                        data-testid={togetherTestid.cancelChallenge}
                        disabled={busy}
                        onClick={() => {
                          act(() => cancelChallenge(challenge.challengeId), reloadSocial);
                        }}
                      >
                        {t("Cancel")}
                      </button>
                    </span>
                  </li>
                ))}
              </ul>
            </section>
          )}

          <section className="lobby-card together-panel" id={SHARED_ANCHOR} aria-labelledby="together-shared-heading">
            <h2 id="together-shared-heading" className="together-panel__heading">
              {t("Shared with me")}
            </h2>
            {shared.kind === "loading" ? (
              <p className="lobby-note" role="status">
                {t("Loading…")}
              </p>
            ) : null}
            {shared.kind === "failed" ? (
              <>
                <p className="notice" role="alert">
                  {t(shared.message)}
                </p>
                <div className="row">
                  <button
                    type="button"
                    data-testid={togetherTestid.sharedRetry}
                    onClick={() => {
                      reloadShared();
                    }}
                  >
                    {t("Try again")}
                  </button>
                </div>
              </>
            ) : null}
            {shared.kind === "ready" && shared.value.shared.length === 0 ? (
              <p className="lobby-note" data-testid={togetherTestid.noShared}>
                {t("When a friend shares a replay with you, it is listed here.")}
              </p>
            ) : null}
            {shared.kind === "ready" && shared.value.shared.length > 0 ? (
              <ul className="together-list">
                {shared.value.shared.map((replay) => (
                  <li key={`${replay.matchId}-${replay.sharer.profileId}`} className="together-row" data-testid={togetherTestid.sharedRow} data-match-id={replay.matchId}>
                    <span className="together-row__who">
                      <Named text={t("Shared by {name}")} name={replay.sharer.username} />
                      {replay.endedAt === null ? null : <span className="together-row__status">{when(replay.endedAt)}</span>}
                    </span>
                    <span className="together-row__actions">
                      <button
                        type="button"
                        data-testid={togetherTestid.sharedOpen}
                        onClick={() => {
                          setWatching(replay);
                        }}
                      >
                        {t("Watch")}
                      </button>
                    </span>
                  </li>
                ))}
              </ul>
            ) : null}
          </section>

          <CoopPanel friends={list?.friends ?? NO_FRIENDS} />

          {list !== null && list.blocked.length > 0 ? (
            <section className="lobby-card together-panel" aria-labelledby="together-blocked-heading">
              <h2 id="together-blocked-heading" className="together-panel__heading">
                {t("Blocked")}
              </h2>
              <ul className="together-list">
                {list.blocked.map((player) => (
                  <li key={player.profileId} className="together-row" data-testid={togetherTestid.blockedRow} data-profile-id={player.profileId}>
                    <span className="together-row__who">
                      <Username name={player.username} />
                    </span>
                    <span className="together-row__actions">
                      <button
                        type="button"
                        data-testid={togetherTestid.unblock}
                        disabled={busy}
                        onClick={() => {
                          act(() => unblockPlayer(player.profileId));
                        }}
                      >
                        {t("Unblock")}
                      </button>
                    </span>
                  </li>
                ))}
              </ul>
            </section>
          ) : null}

          <section className="lobby-card together-panel" aria-labelledby="together-privacy-heading">
            <h2 id="together-privacy-heading" className="together-panel__heading">
              {t("Watching")}
            </h2>
            <SpectatorSwitch />
          </section>
        </div>
      </div>

      {chatWith === null ? null : (
        <ChatPanel
          key={chatWith.profileId}
          friend={chatWith}
          onClose={() => {
            setChatWith(null);
            reloadSocial();
          }}
        />
      )}
      {challenging === null ? null : (
        <ChallengeDialog
          friend={challenging}
          onClose={closeChallenge}
          onSent={() => {
            closeChallenge();
            reloadSocial();
          }}
        />
      )}
    </div>
  );
}
