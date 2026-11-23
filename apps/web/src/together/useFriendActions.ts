// What the friends drawer and the Friends page do to a friend or a request, shared (R3732002): one change at
// a time behind `busy`, the server's refusal in `error`, and the friends list read again after each.
// R2119000: Remove asks nothing: the friend leaves the list at once and the unfriend waits behind "Friend
// removed · Undo" (`undo/hold.ts`), so Undo sends nothing. R3730005, R3874000: Invite to lobby makes a 1v1
// lobby, invites the friend and opens it. Nothing here is a rule (CLAUDE.md rule 7): the server decides each.

import { useEffect, useMemo, useState } from "react";

import { t } from "../i18n/i18n.ts";
import { createLobby, inviteToLobby } from "../lobby/api.ts";
import { holdInviteRefusal } from "../lobby/inviteRefusal.ts";
import { attempt, messageOf } from "../net/errors.ts";
import { navigate, paths } from "../net/navigate.ts";
import { blockPlayer, removeFriend, type Friend, type FriendsList, type PlayerRef } from "../net/social.ts";
import { useUndoHold, type HeldDeletion, type UndoTimer } from "../undo/hold.ts";

export type FriendActions = {
  /**
   * The friends the list holds minus the ones a held or sent Remove took off it; null while the list is
   * unread, and none for an answer with no list of friends.
   */
  shown: readonly Friend[] | null;
  busy: boolean;
  error: string | null;
  /** One change of the screen's own, then `after` (the friends list read again unless the screen says another). */
  act: (work: () => Promise<unknown>, after?: () => void) => void;
  remove: (friend: PlayerRef) => void;
  block: (friend: PlayerRef) => void;
  /** `opened` runs once the lobby is made and opened (the friends drawer closes then). */
  inviteToLobby: (friend: PlayerRef, opened?: () => void) => void;
  /** The held Remove the undo toast offers to undo, if any. */
  held: HeldDeletion | null;
  undo: () => void;
  /** Sends the held Remove now, before the screen leaves. */
  sendNow: () => void;
};

export function useFriendActions({
  friends,
  reloadFriends,
  undoTimer,
}: {
  friends: FriendsList | null;
  reloadFriends: () => void;
  /** The undo window's timers (R2119000); a test hands a manual one. */
  undoTimer?: UndoTimer;
}): FriendActions {
  const { held, hold, undo, sendNow } = useUndoHold(undoTimer);
  /** R2119000: friends a held or sent Remove took off the list, until the list no longer names them. */
  const [hiddenFriends, setHiddenFriends] = useState<ReadonlySet<string>>(() => new Set());
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const hideFriend = (id: string): void => {
    setHiddenFriends((before) => new Set(before).add(id));
  };
  const showFriend = (id: string): void => {
    setHiddenFriends((before) => {
      if (!before.has(id)) return before;
      const after = new Set(before);
      after.delete(id);
      return after;
    });
  };

  // A list whatever answered: a server's older answer must not take a screen down.
  const listed = useMemo(() => (friends === null ? null : Array.isArray(friends.friends) ? friends.friends : []), [friends]);

  // R2119000: a removed friend the list no longer names needs hiding no more, so a friend made again shows.
  useEffect(() => {
    if (listed === null) return;
    const named = new Set(listed.map((friend) => friend.profileId));
    setHiddenFriends((before) => {
      const kept = [...before].filter((id) => named.has(id));
      return kept.length === before.size ? before : new Set(kept);
    });
  }, [listed]);

  const shown = useMemo(() => (listed === null ? null : listed.filter((friend) => !hiddenFriends.has(friend.profileId))), [listed, hiddenFriends]);

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

  const remove = (friend: PlayerRef): void => {
    const id = friend.profileId;
    // `hold` first, since it sends any Remove held before this one. R3059000: it refuses a double click's
    // second press, which lands on the friend that moved up, and nothing then changes.
    if (
      !hold({
        words: () => t("Friend removed"),
        send: (keepalive) => {
          void attempt(() => (keepalive ? removeFriend(id, { keepalive: true }) : removeFriend(id))).then(
            () => {
              reloadFriends();
            },
            (cause: unknown) => {
              showFriend(id);
              setError(messageOf(cause));
            },
          );
        },
        restore: () => {
          showFriend(id);
        },
      })
    )
      return;
    setError(null);
    hideFriend(id);
  };

  return {
    shown,
    busy,
    error,
    act,
    remove,
    block: (friend) => {
      act(() => blockPlayer(friend.profileId));
    },
    inviteToLobby: (friend, opened) => {
      // R3730005, R3874000: a 1v1 lobby of the friend's and the player's own, the friend invited into it.
      // Through `act`, so the screen's controls wait and a second press sends nothing; a refused invite
      // still opens the lobby made, whose screen says the refusal.
      act(async () => {
        const made = await createLobby({ format: "duel", mode: "bo1" });
        try {
          await inviteToLobby(made.code, friend.profileId);
        } catch (cause) {
          holdInviteRefusal(made.code, messageOf(cause));
        }
        navigate(paths.lobby(made.code));
        opened?.();
      });
    },
    held,
    undo,
    sendNow,
  };
}
