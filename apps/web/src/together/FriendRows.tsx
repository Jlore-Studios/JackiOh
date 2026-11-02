// The rows Play together and the friends panel share (#936, B55, B56; #1831): the add form, one friend
// with what can be done with them now, and one friend request. Nothing here is a rule (CLAUDE.md
// rule 7): which friend can be challenged or watched is what the server says of them (`status`,
// `watchable`), and every refusal is shown in the server's words.

import { useId, useState, type FormEvent, type ReactElement } from "react";

import Username from "../auth/Username.tsx";
import { t } from "../i18n/i18n.ts";
import { navigate } from "../net/navigate.ts";
import { sendFriendRequest, type Friend, type FriendRequest } from "../net/social.ts";
import { togetherTestid } from "./testids.ts";
import { watchLink } from "./watchLink.ts";
import { Named, STATUS_WORDS } from "./words.tsx";
import { attempt, messageOf } from "../net/errors.ts";

import "./together.css";

export function AddFriend({ onAdded }: { onAdded: () => void }): ReactElement {
  const inputId = useId();
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
      <label htmlFor={inputId}>{t("Add a friend by their username")}</label>
      <div className="together-add__row">
        <input
          id={inputId}
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

export type FriendRowProps = {
  friend: Friend;
  unread: number;
  busy: boolean;
  /** Gets the Challenge button pressed, which has the focus back once the dialog closes. */
  onChallenge: (opener: HTMLElement) => void;
  onChat: () => void;
  /** A screen that leaves these out offers no Remove or Block. Remove goes at once; the screen offers Undo (R2119000). */
  onRemove?: () => void;
  onBlock?: () => void;
  /** Runs before Watch takes the player to the game (the friends panel closes). */
  onWatch?: () => void;
  /** Why the player cannot challenge anyone now (the server's own sentence); Challenge is then disabled and says it. */
  challengeRefusal?: string | null;
};

export function FriendRow({
  friend,
  unread,
  busy,
  onChallenge,
  onChat,
  onRemove,
  onBlock,
  onWatch,
  challengeRefusal = null,
}: FriendRowProps): ReactElement {
  const [asking, setAsking] = useState(false);
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
      {!asking ? (
        <span className="together-row__actions">
          {canChallenge ? (
            <button
              type="button"
              className="button-primary"
              data-testid={togetherTestid.challenge}
              disabled={busy || challengeRefusal !== null}
              title={challengeRefusal === null ? undefined : t(challengeRefusal)}
              onClick={(event) => {
                onChallenge(event.currentTarget);
              }}
            >
              {t("Challenge")}
            </button>
          ) : null}
          {canWatch && friend.matchId !== null ? (
            <button
              type="button"
              data-testid={togetherTestid.watch}
              data-route={watchLink(friend.matchId, friend.profileId)}
              onClick={() => {
                if (friend.matchId === null) return;
                onWatch?.();
                navigate(watchLink(friend.matchId, friend.profileId));
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
          {onRemove === undefined ? null : (
            <button
              type="button"
              className="link-button"
              data-testid={togetherTestid.remove}
              disabled={busy}
              onClick={onRemove}
            >
              {t("Remove")}
            </button>
          )}
          {onBlock === undefined ? null : (
            <button
              type="button"
              className="link-button"
              data-testid={togetherTestid.block}
              disabled={busy}
              onClick={() => {
                setAsking(true);
              }}
            >
              {t("Block")}
            </button>
          )}
        </span>
      ) : (
        // An inline question, not a modal: the row asks once before it blocks (B56).
        <span className="together-row__actions" role="alertdialog" aria-labelledby={questionId}>
          <span className="together-row__question" id={questionId}>
            <Named text={t("Block {name}? You stop being friends, and neither of you can reach the other.")} name={friend.username} />
          </span>
          <button
            type="button"
            data-testid={togetherTestid.confirm}
            disabled={busy}
            onClick={() => {
              setAsking(false);
              onBlock?.();
            }}
          >
            {t("Block")}
          </button>
          <button
            type="button"
            className="link-button"
            data-testid={togetherTestid.keep}
            onClick={() => {
              setAsking(false);
            }}
          >
            {t("Keep")}
          </button>
        </span>
      )}
    </li>
  );
}

export function RequestRow({
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
  onCancel?: () => void;
  onBlock?: () => void;
}): ReactElement {
  const other = direction === "incoming" ? request.from : request.to;
  /** Block's inline question is open (an incoming request's only, #2684). */
  const [asking, setAsking] = useState(false);
  const questionId = useId();
  return (
    <li className="together-row" data-testid={togetherTestid.requestRow} data-direction={direction} data-request-id={request.requestId} data-profile-id={other.profileId}>
      <span className="together-row__who">
        {direction === "incoming" ? (
          <Named text={t("{name} wants to be friends")} name={other.username} />
        ) : (
          <Named text={t("Waiting for {name}")} name={other.username} />
        )}
      </span>
      {asking ? (
        // An inline question, not a modal, as a friend row's: a block lifted later restores no request (§9.4, #2684).
        <span className="together-row__actions" role="alertdialog" aria-labelledby={questionId}>
          <span className="together-row__question" id={questionId}>
            <Named text={t("Block {name}? Their request goes, and neither of you can reach the other.")} name={other.username} />
          </span>
          <button
            type="button"
            data-testid={togetherTestid.confirm}
            disabled={busy}
            onClick={() => {
              setAsking(false);
              onBlock?.();
            }}
          >
            {t("Block")}
          </button>
          <button
            type="button"
            className="link-button"
            data-testid={togetherTestid.keep}
            onClick={() => {
              setAsking(false);
            }}
          >
            {t("Keep")}
          </button>
        </span>
      ) : (
        <span className="together-row__actions">
          {direction === "incoming" ? (
            <>
              <button type="button" className="button-primary" data-testid={togetherTestid.requestAccept} disabled={busy} onClick={onAccept}>
                {t("Accept")}
              </button>
              <button type="button" data-testid={togetherTestid.requestDecline} disabled={busy} onClick={onDecline}>
                {t("Decline")}
              </button>
              {onBlock === undefined ? null : (
                <button
                  type="button"
                  className="link-button"
                  data-testid={togetherTestid.requestBlock}
                  disabled={busy}
                  onClick={() => {
                    setAsking(true);
                  }}
                >
                  {t("Block")}
                </button>
              )}
            </>
          ) : onCancel === undefined ? null : (
            <button type="button" data-testid={togetherTestid.requestCancel} disabled={busy} onClick={onCancel}>
              {t("Cancel")}
            </button>
          )}
        </span>
      )}
    </li>
  );
}
