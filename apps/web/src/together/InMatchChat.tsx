// Chat from the board (#936, B58): a toggle in the match screen's board rail, with the unread count
// beside it, that lists the friends who are on (online or in a game) and opens the chat panel with
// the one picked. The friends are read only while the list is open; the unread count every
// `SOCIAL_POLL_MS`.

import { useState, type ReactElement } from "react";

import Username from "../auth/Username.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import { getFriends, getUnread, type PlayerRef } from "../net/social.ts";
import { ChatPanel } from "./ChatPanel.tsx";
import { SOCIAL_POLL_MS } from "./config.ts";
import { unreadCountsOf, usePolled } from "./session.ts";
import { chatTestid } from "./testids.ts";
import { STATUS_WORDS } from "./words.tsx";

import "./together.css";

/** The friends who are on, read while the list is open. */
function OnlineFriends({
  unread,
  onPick,
}: {
  unread: Readonly<Record<string, number>>;
  onPick: (friend: PlayerRef) => void;
}): ReactElement {
  const [friends] = usePolled(getFriends, SOCIAL_POLL_MS);
  if (friends.kind === "loading") {
    return (
      <p className="in-match-chat__note" role="status">
        {t("Loading your friends…")}
      </p>
    );
  }
  if (friends.kind === "failed") {
    return (
      <p className="in-match-chat__note" role="alert">
        {t(friends.message)}
      </p>
    );
  }
  // A list whatever answered: the rail sits on every match screen.
  const all = Array.isArray(friends.value.friends) ? friends.value.friends : [];
  const on = all.filter((friend) => friend.status !== "offline");
  if (on.length === 0) return <p className="in-match-chat__note">{t("None of your friends are online.")}</p>;
  return (
    <ul className="in-match-chat__friends">
      {on.map((friend) => {
        const count = unread[friend.profileId] ?? 0;
        return (
          <li key={friend.profileId}>
            <button
              type="button"
              className="in-match-chat__friend"
              data-testid={chatTestid.inMatchFriend}
              data-profile-id={friend.profileId}
              onClick={() => {
                onPick({ profileId: friend.profileId, username: friend.username });
              }}
            >
              <span className="together-dot" data-status={friend.status} aria-hidden="true" />
              <Username name={friend.username} />
              <span className="in-match-chat__status">{t(STATUS_WORDS[friend.status])}</span>
              {count > 0 ? <span className="together-badge">{count}</span> : null}
            </button>
          </li>
        );
      })}
    </ul>
  );
}

export function InMatchChat(): ReactElement {
  useLocale();
  const [open, setOpen] = useState(false);
  const [chatWith, setChatWith] = useState<PlayerRef | null>(null);
  const [unread, reloadUnread] = usePolled(getUnread, SOCIAL_POLL_MS);
  const counts: Readonly<Record<string, number>> = unread.kind === "ready" ? unreadCountsOf(unread.value) : {};
  const total = Object.values(counts).reduce((sum, count) => sum + count, 0);

  return (
    <div className="in-match-chat">
      <button
        type="button"
        className="link-button in-match-chat__toggle"
        data-testid={chatTestid.inMatchToggle}
        aria-expanded={open}
        onClick={() => {
          setOpen((shown) => !shown);
        }}
      >
        {t("Chat")}
        {total > 0 ? (
          <span className="together-badge" data-testid={chatTestid.inMatchUnread} aria-label={t("{count} unread", { count: total })}>
            {total}
          </span>
        ) : null}
      </button>
      {open ? (
        <div className="in-match-chat__list" data-testid={chatTestid.inMatchList}>
          <OnlineFriends
            unread={counts}
            onPick={(friend) => {
              setChatWith(friend);
              setOpen(false);
            }}
          />
        </div>
      ) : null}
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
    </div>
  );
}
