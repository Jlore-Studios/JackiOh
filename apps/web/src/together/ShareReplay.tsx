// "Share with a friend" (#936, B52, B59): a finished online match's replay, shared with a friend, who
// then finds it under "Shared with me" on Play together and watches it from the sharer's seat. Offered
// on `/replays`' online games and in the replay player of an online match.
//
// Who may share what is the server's (a seat holder, a finished match, a friend; CLAUDE.md rule 7),
// and its refusal is shown as it says it. A share made here can be taken back here ("Stop sharing");
// unfriending or blocking ends one too.

import { useEffect, useState, type ReactElement } from "react";

import Username from "../auth/Username.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import { paths } from "../net/navigate.ts";
import { getFriends, shareReplay, unshareReplay, type Friend } from "../net/social.ts";
import { followInApp } from "../routes/nav.tsx";
import { attempt, messageOf } from "./session.ts";
import { shareTestid } from "./testids.ts";

import "./together.css";

export function ShareWithFriend({ matchId }: { matchId: string }): ReactElement {
  useLocale();
  const [open, setOpen] = useState(false);
  const [friends, setFriends] = useState<readonly Friend[] | null>(null);
  /** The friends this replay has been shared with from here. */
  const [shared, setShared] = useState<ReadonlySet<string>>(() => new Set());
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open || friends !== null) return;
    let cancelled = false;
    attempt(getFriends).then(
      (answer) => {
        if (!cancelled) setFriends(answer.friends);
      },
      (cause: unknown) => {
        if (!cancelled) setError(messageOf(cause));
      },
    );
    return () => {
      cancelled = true;
    };
  }, [open, friends]);

  const toggle = (friend: Friend): void => {
    if (busy !== null) return;
    const sharing = !shared.has(friend.profileId);
    setBusy(friend.profileId);
    setError(null);
    attempt(() => (sharing ? shareReplay(matchId, friend.profileId) : unshareReplay(matchId, friend.profileId)))
      .then(
        () => {
          setShared((ids) => {
            const next = new Set(ids);
            if (sharing) next.add(friend.profileId);
            else next.delete(friend.profileId);
            return next;
          });
        },
        (cause: unknown) => {
          setError(messageOf(cause));
        },
      )
      .finally(() => {
        setBusy(null);
      });
  };

  return (
    <div className="share-friend">
      <button
        type="button"
        data-testid={shareTestid.open}
        aria-expanded={open}
        onClick={() => {
          setOpen((shown) => !shown);
        }}
      >
        {t("Share with a friend")}
      </button>
      {open ? (
        <div className="share-friend__list">
          {friends === null && error === null ? <p role="status">{t("Loading your friends…")}</p> : null}
          {friends !== null && friends.length === 0 ? (
            <p data-testid={shareTestid.none}>
              {t("You have no friends to share with yet.")}{" "}
              <a href={paths.together} onClick={followInApp(paths.together)}>
                {t("Add one on Play together")}
              </a>
            </p>
          ) : null}
          {friends !== null && friends.length > 0 ? (
            <ul>
              {friends.map((friend) => {
                const isShared = shared.has(friend.profileId);
                return (
                  <li key={friend.profileId} className="share-friend__row">
                    <Username name={friend.username} />
                    <button
                      type="button"
                      data-testid={isShared ? shareTestid.unshare : shareTestid.option}
                      data-profile-id={friend.profileId}
                      data-shared={isShared ? "true" : "false"}
                      disabled={busy !== null}
                      aria-busy={busy === friend.profileId}
                      onClick={() => {
                        toggle(friend);
                      }}
                    >
                      {isShared ? t("Shared ✓ · Stop sharing") : t("Share")}
                    </button>
                  </li>
                );
              })}
            </ul>
          ) : null}
          {error === null ? null : (
            <p className="notice" data-testid={shareTestid.error} role="alert">
              {t(error)}
            </p>
          )}
        </div>
      ) : null}
    </div>
  );
}
