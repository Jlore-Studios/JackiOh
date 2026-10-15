// `/together` — Play together (#936, SPEC S8, B54–B59): friends with their status, requests both
// ways, the players this account blocked, the challenges it sent, the replays friends shared with it,
// chat with any friend, and Duos co-op (`duos/CoopPanel.tsx`, which gets the friends list).
//
// Behind the gate like `/play` (B54). Every list is the server's answer (`net/social.ts`), read again
// every `SOCIAL_POLL_MS` so a friend coming online, a request arriving or a challenge being taken
// shows without a reload, and again at once after each of this page's own changes. Nothing here is a
// rule (CLAUDE.md rule 7): which friend can be challenged or watched is what the server says of them
// (`status`, `watchable`), and every refusal is shown in the server's words.
//
// A friend row offers (B56): Challenge while the friend is online and not in a game; Watch while the
// server says the game is `watchable`; Chat always; Remove and Block, each asking once.

import { useEffect, useId, useState, type FormEvent, type ReactElement } from "react";

import Username from "../auth/Username.tsx";
import { CoopPanel } from "../duos/CoopPanel.tsx";
import { CatalogContext } from "../game/catalog.ts";
import { formatDateTime, t, useLocale } from "../i18n/i18n.ts";
import { withSession } from "../net/gate.ts";
import { navigate } from "../net/navigate.ts";
import {
  acceptFriendRequest,
  blockPlayer,
  cancelChallenge,
  cancelFriendRequest,
  declineFriendRequest,
  getChallenges,
  getFriends,
  getSharedReplays,
  getUnread,
  removeFriend,
  sendFriendRequest,
  unblockPlayer,
  type Friend,
  type FriendRequest,
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
import { SOCIAL_POLL_MS } from "./config.ts";
import { attempt, messageOf, unreadCountsOf, usePolled } from "./session.ts";
import { SpectatorSwitch } from "./SpectatorSwitch.tsx";
import { togetherTestid } from "./testids.ts";
import { watchLink } from "./watchLink.ts";
import { MODE_WORDS, Named, STATUS_WORDS } from "./words.tsx";

import "../auth/tavern.css";
import "../routes/lobby.css";
import "./together.css";

const NO_FRIENDS: readonly Friend[] = [];

/** An ISO time as a date and time in the reader's language, or the text itself when it is not one. */
function when(iso: string): string {
  const at = Date.parse(iso);
  return Number.isFinite(at) ? formatDateTime(at, { dateStyle: "medium", timeStyle: "short" }) : iso;
}

/** The add form (B55): a `name#tag`, and the server's answer to it. */
function AddFriend({ onAdded }: { onAdded: () => void }): ReactElement {
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [sent, setSent] = useState<string | null>(null);

  const submit = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    const username = name.trim();
    if (username.length === 0 || busy) return;
    setBusy(true);
    setError(null);
    setSent(null);
    attempt(() => sendFriendRequest(username))
      .then(
        () => {
          setSent(username);
          setName("");
          onAdded();
        },
        (cause: unknown) => {
          setError(messageOf(cause));
        },
      )
      .finally(() => {
        setBusy(false);
      });
  };

  return (
    <form className="together-add" onSubmit={submit}>
      <label htmlFor="together-add-input">{t("Add a friend by their username")}</label>
      <div className="together-add__row">
        <input
          id="together-add-input"
          data-testid={togetherTestid.addInput}
          value={name}
          autoComplete="off"
          spellCheck={false}
          placeholder={t("name#tag")}
          onChange={(event) => {
            setName(event.target.value);
          }}
        />
        <button type="submit" className="button-primary" data-testid={togetherTestid.addSubmit} disabled={busy || name.trim().length === 0} aria-busy={busy}>
          {t("Add")}
        </button>
      </div>
      {error === null ? null : (
        <p className="notice" data-testid={togetherTestid.addError} role="alert">
          {t(error)}
        </p>
      )}
      {sent === null ? null : (
        <p className="lobby-note" data-testid={togetherTestid.addSent} role="status">
          <Named text={t("Friend request sent to {name}.")} name={sent} />
        </p>
      )}
    </form>
  );
}

type FriendRowProps = {
  friend: Friend;
  unread: number;
  busy: boolean;
  onChallenge: () => void;
  onChat: () => void;
  onRemove: () => void;
  onBlock: () => void;
};

/** One friend (B56): status, and what can be done with them now. */
function FriendRow({ friend, unread, busy, onChallenge, onChat, onRemove, onBlock }: FriendRowProps): ReactElement {
  const [asking, setAsking] = useState<"remove" | "block" | null>(null);
  const questionId = useId();
  const canChallenge = friend.status === "online";
  const canWatch = friend.watchable && friend.matchId !== null;
  return (
    <li
      className="together-row"
      data-testid={togetherTestid.friendRow}
      data-profile-id={friend.profileId}
      data-status={friend.status}
      data-watchable={friend.watchable ? "true" : "false"}
    >
      <span className="together-row__who">
        <span className="together-dot" data-status={friend.status} aria-hidden="true" />
        <Username name={friend.username} />
        <span className="together-row__status">{t(STATUS_WORDS[friend.status])}</span>
      </span>
      {asking === null ? (
        <span className="together-row__actions">
          {canChallenge ? (
            <button type="button" className="button-primary" data-testid={togetherTestid.challenge} disabled={busy} onClick={onChallenge}>
              {t("Challenge")}
            </button>
          ) : null}
          {canWatch && friend.matchId !== null ? (
            <button
              type="button"
              data-testid={togetherTestid.watch}
              onClick={() => {
                if (friend.matchId !== null) navigate(watchLink(friend.matchId, friend.profileId));
              }}
            >
              {t("Watch")}
            </button>
          ) : null}
          <button type="button" data-testid={togetherTestid.chatOpen} onClick={onChat}>
            {t("Chat")}
            {unread > 0 ? (
              <span className="together-badge" data-testid={togetherTestid.unread} aria-label={t("{count} unread", { count: unread })}>
                {unread}
              </span>
            ) : null}
          </button>
          <button
            type="button"
            className="link-button"
            data-testid={togetherTestid.remove}
            disabled={busy}
            onClick={() => {
              setAsking("remove");
            }}
          >
            {t("Remove")}
          </button>
          <button
            type="button"
            className="link-button"
            data-testid={togetherTestid.block}
            disabled={busy}
            onClick={() => {
              setAsking("block");
            }}
          >
            {t("Block")}
          </button>
        </span>
      ) : (
        // An inline question, not a modal: the row asks once before it removes or blocks (B56).
        <span className="together-row__actions" role="alertdialog" aria-labelledby={questionId}>
          <span className="together-row__question" id={questionId}>
            {asking === "remove" ? (
              <Named text={t("Remove {name} from your friends?")} name={friend.username} />
            ) : (
              <Named text={t("Block {name}? You stop being friends, and neither of you can reach the other.")} name={friend.username} />
            )}
          </span>
          <button
            type="button"
            data-testid={togetherTestid.confirm}
            disabled={busy}
            onClick={() => {
              const action = asking;
              setAsking(null);
              if (action === "remove") onRemove();
              else onBlock();
            }}
          >
            {asking === "remove" ? t("Remove") : t("Block")}
          </button>
          <button
            type="button"
            className="link-button"
            data-testid={togetherTestid.keep}
            onClick={() => {
              setAsking(null);
            }}
          >
            {t("Keep")}
          </button>
        </span>
      )}
    </li>
  );
}

/** One request, either way (B55). */
function RequestRow({
  request,
  direction,
  busy,
  onAccept,
  onDecline,
  onCancel,
  onBlock,
}: {
  request: FriendRequest;
  direction: "incoming" | "outgoing";
  busy: boolean;
  onAccept: () => void;
  onDecline: () => void;
  onCancel: () => void;
  onBlock: () => void;
}): ReactElement {
  const other = direction === "incoming" ? request.from : request.to;
  return (
    <li className="together-row" data-testid={togetherTestid.requestRow} data-direction={direction} data-request-id={request.requestId} data-profile-id={other.profileId}>
      <span className="together-row__who">
        {direction === "incoming" ? (
          <Named text={t("{name} wants to be friends")} name={other.username} />
        ) : (
          <Named text={t("Waiting for {name}")} name={other.username} />
        )}
      </span>
      <span className="together-row__actions">
        {direction === "incoming" ? (
          <>
            <button type="button" className="button-primary" data-testid={togetherTestid.requestAccept} disabled={busy} onClick={onAccept}>
              {t("Accept")}
            </button>
            <button type="button" data-testid={togetherTestid.requestDecline} disabled={busy} onClick={onDecline}>
              {t("Decline")}
            </button>
            <button type="button" className="link-button" data-testid={togetherTestid.requestBlock} disabled={busy} onClick={onBlock}>
              {t("Block")}
            </button>
          </>
        ) : (
          <button type="button" data-testid={togetherTestid.requestCancel} disabled={busy} onClick={onCancel}>
            {t("Cancel")}
          </button>
        )}
      </span>
    </li>
  );
}

export function TogetherRoute(): ReactElement {
  useLocale();
  const [friends, reloadFriends] = usePolled(getFriends, SOCIAL_POLL_MS);
  const [challenges, reloadChallenges] = usePolled(getChallenges, SOCIAL_POLL_MS);
  const [unread, reloadUnread] = usePolled(getUnread, SOCIAL_POLL_MS);
  const [shared] = usePolled(getSharedReplays, null);
  const lookup = useCatalogLookup();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [chatWith, setChatWith] = useState<PlayerRef | null>(null);
  const [challenging, setChallenging] = useState<PlayerRef | null>(null);
  const [watching, setWatching] = useState<SharedReplay | null>(null);

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
  const counts: Readonly<Record<string, number>> = unread.kind === "ready" ? unreadCountsOf(unread.value) : {};
  const outgoingChallenges = challenges.kind === "ready" ? challenges.value.outgoing : [];

  return (
    <div className="app-shell tavern lobby together-screen" data-testid={togetherTestid.root}>
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

      <div className="together-layout">
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
            <p className="notice" role="alert">
              {t(friends.message)}
            </p>
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
                  onChallenge={() => {
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
                          act(() => cancelChallenge(challenge.challengeId), reloadChallenges);
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
              <p className="notice" role="alert">
                {t(shared.message)}
              </p>
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
            reloadUnread();
          }}
        />
      )}
      {challenging === null ? null : (
        <ChallengeDialog
          friend={challenging}
          onClose={() => {
            setChallenging(null);
          }}
          onSent={() => {
            setChallenging(null);
            reloadChallenges();
          }}
        />
      )}
    </div>
  );
}
