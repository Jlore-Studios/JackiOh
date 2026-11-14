// The rows the Friends page and the friends panel share (#936, B55, B56; #1831; R3732002): the add form,
// one friend with the one thing to do with them now and a ⋯ menu for the rest, and one friend request.
// Nothing here is a rule (CLAUDE.md rule 7): which friend can be challenged, watched or joined is what the
// server says of them (`status`, `watchable`, `lobbyCode`), and every refusal is shown in the server's words.

import { useId, useState, type FormEvent, type ReactElement } from "react";

import Username from "../auth/Username.tsx";
import { t } from "../i18n/i18n.ts";
import { navigate, paths } from "../net/navigate.ts";
import { sendFriendRequest, type Friend, type FriendRequest, type PlayerRef } from "../net/social.ts";
import type { ChallengeKind } from "./ChallengeDialog.tsx";
import { FriendMenu } from "./FriendMenu.tsx";
import { menuItemsOf, primaryActionOf, statusLineOf } from "./friendGroups.ts";
import { togetherTestid } from "./testids.ts";
import { watchLink } from "./watchLink.ts";
import { Named } from "./words.tsx";
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

/**
 * Block's inline question, a friend row's and an incoming request's (B56, #2684): not a modal, it asks once
 * before it blocks. R3221000: the safe Keep comes first and the act last, as in every dialog.
 */
function BlockQuestion({
  question,
  name,
  busy,
  onBlock,
  onKeep,
}: {
  question: string;
  name: string;
  busy: boolean;
  onBlock: () => void;
  onKeep: () => void;
}): ReactElement {
  const questionId = useId();
  return (
    <span className="together-row__actions" role="alertdialog" aria-labelledby={questionId}>
      <span className="together-row__question" id={questionId}>
        <Named text={question} name={name} />
      </span>
      <button type="button" className="link-button" data-testid={togetherTestid.keep} onClick={onKeep}>
        {t("Keep")}
      </button>
      <button type="button" data-testid={togetherTestid.confirm} disabled={busy} onClick={onBlock}>
        {t("Block")}
      </button>
    </span>
  );
}

/** #3727: a live game needs the friend online and free; a daily game is any friend's (untranslated). */
export const LIVE_NEEDS_ONLINE = "A live game needs your friend online and free. A daily game can wait for them.";

/** Why a live game can't be played with `friend` now (the player's own refusal first), or null. */
export function liveRefusalOf(friend: Friend, challengeRefusal: string | null): string | null {
  if (challengeRefusal !== null) return challengeRefusal;
  return friend.status === "online" ? null : LIVE_NEEDS_ONLINE;
}

/**
 * The friend a challenge dialog is open for, and why Live is out (`liveRefusalOf`). `kind` is the kind it
 * opens on (R3732002: a row's Daily opens on Daily); none, Live where it can be played.
 */
export type Challenging = PlayerRef & { liveRefusal: string | null; kind?: ChallengeKind };

export type FriendRowProps = {
  friend: Friend;
  unread: number;
  busy: boolean;
  /** Epoch ms the status line counts "last seen" from. */
  now: number;
  /**
   * The Challenge or Daily pressed, as the primary button or the menu's Daily challenge; `opener` has the
   * focus back once the dialog closes.
   */
  onChallenge: (opener: HTMLElement, kind: ChallengeKind) => void;
  onChat: () => void;
  /** A screen that leaves these out offers no Remove or Block. Remove goes at once; the screen offers Undo (R2119000). */
  onRemove?: () => void;
  onBlock?: () => void;
  /** Runs before Watch or Join takes the player to the game or the lobby (the friends panel closes). */
  onLeave?: () => void;
  /**
   * Why the player cannot play anyone live now (the server's own sentence). #3727: Challenge stays, for a
   * daily game, and its dialog says it of Live.
   */
  challengeRefusal?: string | null;
  /** R3730005: a screen that passes it offers Invite to lobby: a 1v1 lobby made and the friend invited into it. */
  onLobby?: () => void;
};

/**
 * One friend (R3732002): the dot, the name, what they are doing in words, the one button their state asks
 * for (Watch, Join, Challenge or Daily) and the ⋯ menu with the rest.
 */
export function FriendRow({
  friend,
  unread,
  busy,
  now,
  onChallenge,
  onChat,
  onRemove,
  onBlock,
  onLeave,
  challengeRefusal = null,
  onLobby,
}: FriendRowProps): ReactElement {
  const [asking, setAsking] = useState(false);
  const primary = primaryActionOf(friend);
  // #3727: any friend can be challenged to a daily game; a live one needs them online and both free.
  const live = liveRefusalOf(friend, challengeRefusal) === null;
  const line = statusLineOf(friend, now);
  const watchRoute = friend.matchId === null ? null : watchLink(friend.matchId, friend.profileId, friend.duos === true);
  const watch = (): void => {
    if (watchRoute === null) return;
    onLeave?.();
    navigate(watchRoute);
  };
  const items = menuItemsOf(friend, primary, { lobby: onLobby !== undefined, remove: onRemove !== undefined, block: onBlock !== undefined });
  return (
    <li
      className="together-row"
      data-testid={togetherTestid.friendRow}
      data-profile-id={friend.profileId}
      data-status={friend.status}
      data-watchable={friend.watchable ? "true" : "false"}
      data-primary={primary}
    >
      <span className="together-row__who">
        <span className="together-dot" data-status={friend.status} aria-hidden="true" />
        <span className="together-row__name">
          <Username name={friend.username} />
          {line === "" ? null : <span className="together-row__status">{line}</span>}
        </span>
      </span>
      {!asking ? (
        <span className="together-row__actions">
          {primary === "watch" ? (
            <button type="button" data-testid={togetherTestid.watch} data-route={watchRoute ?? undefined} onClick={watch}>
              {t("Watch")}
            </button>
          ) : null}
          {primary === "join" && friend.lobbyCode !== undefined ? (
            <button
              type="button"
              data-testid={togetherTestid.join}
              data-route={paths.lobby(friend.lobbyCode)}
              onClick={() => {
                if (friend.lobbyCode === undefined) return;
                onLeave?.();
                navigate(paths.lobby(friend.lobbyCode));
              }}
            >
              {t("Join")}
            </button>
          ) : null}
          {primary === "challenge" || primary === "daily" ? (
            <button
              type="button"
              data-testid={togetherTestid.challenge}
              data-action={primary}
              data-live={live && primary === "challenge" ? "true" : "false"}
              disabled={busy}
              onClick={(event) => {
                onChallenge(event.currentTarget, primary === "daily" ? "daily" : "live");
              }}
            >
              {primary === "daily" ? t("Daily") : t("Challenge")}
            </button>
          ) : null}
          <FriendMenu
            name={friend.username}
            items={items}
            unread={unread}
            busy={busy}
            onItem={(item, opener) => {
              switch (item) {
                case "chat":
                  onChat();
                  return;
                case "daily":
                  onChallenge(opener, "daily");
                  return;
                case "lobby":
                  onLobby?.();
                  return;
                case "watch":
                  watch();
                  return;
                case "remove":
                  onRemove?.();
                  return;
                case "block":
                  setAsking(true);
                  return;
              }
            }}
          />
        </span>
      ) : (
        <BlockQuestion
          question={t("Block {name}? You stop being friends, and neither of you can reach the other.")}
          name={friend.username}
          busy={busy}
          onBlock={() => {
            setAsking(false);
            onBlock?.();
          }}
          onKeep={() => {
            setAsking(false);
          }}
        />
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
        // As a friend row's: a block lifted later restores no request (§9.4, #2684).
        <BlockQuestion
          question={t("Block {name}? Their request goes, and neither of you can reach the other.")}
          name={other.username}
          busy={busy}
          onBlock={() => {
            setAsking(false);
            onBlock?.();
          }}
          onKeep={() => {
            setAsking(false);
          }}
        />
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
