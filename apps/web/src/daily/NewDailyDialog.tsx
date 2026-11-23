// New daily game (#3727): challenge any friend, online or not, or find an opponent in the daily queue, with
// the mode, its deck or lean, and the time per move (`DailyFields.tsx`). The queue answers a game at once
// when someone waits with the same mode and time, else a seek that waits on the Daily screen. A finished
// game's Rematch opens it as a new challenge to that opponent at the game's mode and time per move, so only
// the deck or lean is picked again (§9.13).
//
// CLAUDE.md rule 7: who may be challenged, how many games a player may keep and every refusal are the
// server's, shown in its words. R3221000: an alertdialog on the one Dialog, Cancel first and the send last.

import { useEffect, useId, useState, type ReactElement } from "react";

import { t, useLocale } from "../i18n/i18n.ts";
import { createDailyChallenge, createDailySeek } from "../net/daily.ts";
import { attempt, messageOf } from "../net/errors.ts";
import { paths } from "../net/navigate.ts";
import type { DailyMode } from "../net/daily.ts";
import { getFriends, type PlayerRef } from "../net/social.ts";
import { Dialog } from "../game/Dialog.tsx";
import { followInApp } from "../routes/nav.tsx";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { Tabs } from "../tabs/index.ts";
import { DailyFields, dailyChoiceOf, useDailyPick } from "./DailyFields.tsx";
import { dailyKindTestid, dailyTestid } from "./testids.ts";

import "./daily.css";

export type NewDailyKind = "friend" | "queue";

/** A finished game's rematch: its opponent, its mode and its time per move. */
export type DailyRematch = { opponent: PlayerRef; mode: DailyMode; perMoveDays: number };

export function NewDailyDialog({
  onClose,
  onDone,
  initialKind = "friend",
  rematch,
}: {
  onClose: () => void;
  /** The game made at once (the queue paired), or null: a challenge or a seek now waits. */
  onDone: (matchId: string | null) => void;
  initialKind?: NewDailyKind;
  /** A rematch: the challenge goes to its opponent at its mode and time; the server says if they are no friend. */
  rematch?: DailyRematch;
}): ReactElement {
  useLocale();
  const idPrefix = useId();
  const [kind, setKind] = useState<NewDailyKind>(rematch === undefined ? initialKind : "friend");
  const pick = useDailyPick(rematch?.mode, true, rematch?.perMoveDays ?? null);
  const [friends, setFriends] = useState<readonly PlayerRef[] | null>(null);
  const [friendsError, setFriendsError] = useState<string | null>(null);
  const [friendId, setFriendId] = useState<string | null>(rematch?.opponent.profileId ?? null);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // A rematch names its friend: no list to pick from.
    if (rematch !== undefined) return undefined;
    let canceled = false;
    attempt(() => getFriends()).then(
      (answer) => {
        if (canceled) return;
        const all = answer.friends.map(({ profileId, username }) => ({ profileId, username }));
        setFriends(all);
        setFriendId((before) => before ?? all[0]?.profileId ?? null);
      },
      (cause: unknown) => {
        if (!canceled) setFriendsError(messageOf(cause));
      },
    );
    return () => {
      canceled = true;
    };
  }, [rematch]);

  const choice = dailyChoiceOf(pick);
  const ready = choice !== null && (kind === "queue" || friendId !== null);

  const send = (): void => {
    if (choice === null || sending || !ready) return;
    setSending(true);
    setError(null);
    const made =
      kind === "friend" && friendId !== null
        ? attempt(() => createDailyChallenge(friendId, choice)).then(() => null)
        : attempt(() => createDailySeek(choice)).then((answer) => answer.matchId ?? null);
    made
      .then(
        (matchId) => {
          onDone(matchId);
        },
        (cause: unknown) => {
          setError(messageOf(cause));
        },
      )
      .finally(() => {
        setSending(false);
      });
  };

  const friendPanel = (
    <>
      {friends === null && friendsError === null ? <Skeleton variant="rows" label={t("Loading your friends…")} /> : null}
      {friendsError === null ? null : (
        <p className="notice" role="alert">
          {t(friendsError)}
        </p>
      )}
      {friends !== null && friends.length === 0 ? (
        <p className="lobby-note" data-testid={dailyTestid.noFriends}>
          {t("No friends yet.")}{" "}
          <a href={paths.together} onClick={followInApp(paths.together)}>
            {t("Add a friend in Play together")}
          </a>
        </p>
      ) : null}
      {friends !== null && friends.length > 0 ? (
        <label className="daily-field">
          <span className="daily-field__label">{t("Friend")}</span>
          <select
            className="lobby-select"
            data-testid={dailyTestid.friend}
            value={friendId ?? ""}
            disabled={sending}
            onChange={(event) => {
              setFriendId(event.target.value);
            }}
          >
            {friends.map((friend) => (
              <option key={friend.profileId} value={friend.profileId}>
                {friend.username}
              </option>
            ))}
          </select>
        </label>
      ) : null}
    </>
  );

  if (rematch !== undefined) {
    return (
      <Dialog
        role="alertdialog"
        title={t("Rematch")}
        description={t("A new daily challenge to {name}, at the same mode and time per move.", { name: rematch.opponent.username })}
        onClose={onClose}
        cancel={{ label: t("Cancel"), kind: "secondary", testid: dailyTestid.close }}
        actions={[
          {
            label: t("Send challenge"),
            kind: "primary",
            onClick: send,
            testid: dailyTestid.send,
            disabled: !ready || sending,
            busy: sending,
          },
        ]}
        testid={dailyTestid.dialog}
        className="daily-dialog"
      >
        <DailyFields pick={pick} fixedMode withTime={false} disabled={sending} />
        {error === null ? null : (
          <p className="notice" data-testid={dailyTestid.dialogError} role="alert">
            {t(error)}
          </p>
        )}
      </Dialog>
    );
  }

  return (
    <Dialog
      role="alertdialog"
      title={t("New daily game")}
      description={t("A move every few days, on your own time. We'll tell you when it's your move.")}
      onClose={onClose}
      cancel={{ label: t("Cancel"), kind: "secondary", testid: dailyTestid.close }}
      actions={[
        {
          label: kind === "friend" ? t("Send challenge") : t("Find an opponent"),
          kind: "primary",
          onClick: send,
          testid: dailyTestid.send,
          disabled: !ready || sending,
          busy: sending,
        },
      ]}
      testid={dailyTestid.dialog}
      className="daily-dialog"
    >
      <Tabs
        label={t("Who you play")}
        idPrefix={idPrefix}
        testId={dailyTestid.dialogKind}
        value={kind}
        onChange={setKind}
        items={[
          { id: "friend", label: t("Challenge a friend"), testId: dailyKindTestid("friend"), panel: friendPanel },
          {
            id: "queue",
            label: t("Find an opponent"),
            testId: dailyKindTestid("queue"),
            panel: <p className="lobby-note">{t("We'll pair you with the next player who picks the same mode and time.")}</p>,
          },
        ]}
      />
      <DailyFields pick={pick} disabled={sending} />
      {error === null ? null : (
        <p className="notice" data-testid={dailyTestid.dialogError} role="alert">
          {t(error)}
        </p>
      )}
    </Dialog>
  );
}
