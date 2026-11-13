// The Friends page's tabs but its first (R3732002): Requests, in and out with the challenges the player sent;
// Recently played (R3732000), the opponents of the player's recent unranked games, each with Add friend;
// Blocked, with Unblock; and Shared with me, the replays friends have shared. Each reads what it shows and
// draws the server's answer; nothing here is a rule (CLAUDE.md rule 7), and every refusal is in the server's
// words.

import { useState, type ReactElement } from "react";

import { AccountError } from "../auth/AccountError.tsx";
import Username from "../auth/Username.tsx";
import { EmptyState } from "../empty/EmptyState.tsx";
import { formatDateTime, t, useLocale } from "../i18n/i18n.ts";
import { attempt, messageOf } from "../net/errors.ts";
import {
  acceptFriendRequest,
  cancelChallenge,
  cancelFriendRequest,
  declineFriendRequest,
  getRecentOpponents,
  getSharedReplays,
  sendFriendRequest,
  unblockPlayer,
  type Challenge,
  type FriendsList,
  type RecentOpponent,
  type SharedReplay,
} from "../net/social.ts";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { SHARED_ANCHOR } from "./anchors.ts";
import { RequestRow } from "./FriendRows.tsx";
import { usePolled } from "./session.ts";
import { reloadSocial } from "./socialPoll.ts";
import { togetherTestid } from "./testids.ts";
import type { FriendActions } from "./useFriendActions.ts";
import { MODE_WORDS, Named } from "./words.tsx";

function when(iso: string): string {
  const at = Date.parse(iso);
  return Number.isFinite(at) ? formatDateTime(at, { dateStyle: "medium", timeStyle: "short" }) : iso;
}

/** The requests in, with Accept, Decline and Block, and out, with Cancel, and the challenges the player sent. */
export function RequestsTab({
  list,
  challenges,
  actions,
}: {
  list: FriendsList | null;
  challenges: readonly Challenge[];
  actions: FriendActions;
}): ReactElement {
  useLocale();
  const { busy, act } = actions;
  const incoming = Array.isArray(list?.incoming) ? list.incoming : [];
  const outgoing = Array.isArray(list?.outgoing) ? list.outgoing : [];
  return (
    <section className="lobby-card together-panel" aria-label={t("Friend requests")}>
      {list !== null && incoming.length === 0 && outgoing.length === 0 && challenges.length === 0 ? (
        <EmptyState text={t("No requests. Friend requests and the challenges you send are listed here.")} testId={togetherTestid.noRequests} />
      ) : null}
      {incoming.length === 0 && outgoing.length === 0 ? null : (
        <ul className="together-list">
          {incoming.map((request) => (
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
                actions.block(request.from);
              }}
            />
          ))}
          {outgoing.map((request) => (
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
      )}
      {challenges.length === 0 ? null : (
        <>
          <h2 className="together-group__heading">{t("Your challenges")}</h2>
          <p className="lobby-note">{t("You will be taken to the game as soon as your friend joins.")}</p>
          <ul className="together-list">
            {challenges.map((challenge) => (
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
        </>
      )}
    </section>
  );
}

/** One opponent of Recently played: Add friend sends the request the add form does, by `name#tag`. */
function RecentRow({ opponent, onAdded }: { opponent: RecentOpponent; onAdded: () => void }): ReactElement {
  const [state, setState] = useState<"idle" | "sending" | "sent">("idle");
  const [error, setError] = useState<string | null>(null);
  const add = (): void => {
    if (state !== "idle") return;
    setState("sending");
    setError(null);
    attempt(() => sendFriendRequest(opponent.username)).then(
      () => {
        setState("sent");
        onAdded();
      },
      (cause: unknown) => {
        setState("idle");
        setError(messageOf(cause));
      },
    );
  };
  return (
    <li className="together-row" data-testid={togetherTestid.recentRow} data-profile-id={opponent.profileId}>
      <span className="together-row__who">
        <span className="together-row__name">
          <Username name={opponent.username} />
          <span className="together-row__status">{when(opponent.playedAt)}</span>
        </span>
      </span>
      <span className="together-row__actions">
        <button type="button" data-testid={togetherTestid.recentAdd} disabled={state !== "idle"} aria-busy={state === "sending"} onClick={add}>
          {state === "sent" ? t("Request sent") : t("Add friend")}
        </button>
      </span>
      {error === null ? null : (
        <p className="notice together-row__question" role="alert">
          {t(error)}
        </p>
      )}
    </li>
  );
}

/** R3732000: who the player lately played an unranked game with, never one the queue paired. */
export function RecentTab({ onAdded }: { onAdded: () => void }): ReactElement {
  useLocale();
  const [recent, reload] = usePolled(getRecentOpponents, null);
  return (
    <section className="lobby-card together-panel" aria-label={t("Recently played")}>
      <p className="lobby-note">{t("People you played in a room, a lobby, a challenge or a daily game.")}</p>
      {recent.kind === "loading" ? <Skeleton variant="rows" label={t("Loading…")} /> : null}
      {recent.kind === "failed" ? (
        <AccountError
          message={recent.message}
          retryAfterMs={recent.retryAfterMs}
          onRetry={reload}
          testId={togetherTestid.recentError}
          retryTestId={togetherTestid.recentRetry}
        />
      ) : null}
      {recent.kind === "ready" && recent.value.recent.length === 0 ? (
        <EmptyState text={t("Nobody yet. The people you play with outside the ladder are listed here.")} testId={togetherTestid.noRecent} />
      ) : null}
      {recent.kind === "ready" && recent.value.recent.length > 0 ? (
        <ul className="together-list">
          {recent.value.recent.map((opponent) => (
            <RecentRow key={opponent.profileId} opponent={opponent} onAdded={onAdded} />
          ))}
        </ul>
      ) : null}
    </section>
  );
}

/** The players the caller blocks, each with Unblock. */
export function BlockedTab({ list, actions }: { list: FriendsList | null; actions: FriendActions }): ReactElement {
  useLocale();
  const blocked = Array.isArray(list?.blocked) ? list.blocked : [];
  return (
    <section className="lobby-card together-panel" aria-label={t("Blocked")}>
      {list !== null && blocked.length === 0 ? (
        <EmptyState text={t("Nobody is blocked. Players you block are listed here, and you can unblock them.")} testId={togetherTestid.noBlocked} />
      ) : null}
      {blocked.length === 0 ? null : (
        <ul className="together-list">
          {blocked.map((player) => (
            <li key={player.profileId} className="together-row" data-testid={togetherTestid.blockedRow} data-profile-id={player.profileId}>
              <span className="together-row__who">
                <Username name={player.username} />
              </span>
              <span className="together-row__actions">
                <button
                  type="button"
                  data-testid={togetherTestid.unblock}
                  disabled={actions.busy}
                  onClick={() => {
                    actions.act(() => unblockPlayer(player.profileId));
                  }}
                >
                  {t("Unblock")}
                </button>
              </span>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

/** "Shared with me": the replays friends have shared; `onOpen` takes the player to one. */
export function SharedTab({ onOpen }: { onOpen: (replay: SharedReplay) => void }): ReactElement {
  useLocale();
  const [shared, reloadShared] = usePolled(getSharedReplays, null);
  return (
    <section className="lobby-card together-panel" id={SHARED_ANCHOR} aria-label={t("Shared with me")}>
      {shared.kind === "loading" ? <Skeleton variant="rows" label={t("Loading…")} /> : null}
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
        <EmptyState text={t("When a friend shares a replay with you, it is listed here.")} testId={togetherTestid.noShared} />
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
                    onOpen(replay);
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
  );
}
