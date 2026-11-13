// The inbox's bell (R3729002), in the shell's top bar and the landing's corner: one press opens everything waiting
// on the player (`InboxPanel.tsx`). Its badge counts what needs them (`order.ts`): the live challenges, the lobby
// invites, the daily games on their move, the daily challenges, the friend requests, and the shared replays
// not yet seen; until the daily list has been read, the social read's `dailyWaiting` stands for its games.
//
// The bell adds no server state: it reads the social snapshot the shell already polls (`together/socialPoll.ts`)
// and, while shown, the daily list, the friends list and the shared replays (`store.ts`). It is hidden when no
// session is held, the server refused the session, or the first social read has not answered, as the Friends
// button is, and the panel is portalled to the page as the Friends panel is, so the bar's layout cannot clip it.
//
// CLAUDE.md rule 7: nothing here is a rule; the badge is a count of what the server said is waiting.

import { useCallback, useId, useRef, useState, type ReactElement } from "react";
import { createPortal } from "react-dom";

import { useNow } from "../daily/DailyClockPill.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import { readSession } from "../net/session.ts";
import { useSocialReads } from "../together/socialPoll.ts";
import { INBOX_TICK_MS } from "./config.ts";
import { useInvites } from "./invites.ts";
import { InboxPanel } from "./InboxPanel.tsx";
import { inboxBadge, inboxItems } from "./order.ts";
import { useInboxPoll, useInboxReads } from "./store.ts";
import { inboxTestid } from "./testids.ts";

import "../auth/tavern.css";
import "../routes/lobby.css";
import "../together/together.css";
import "../routes/top-bar.css";
import "./inbox.css";

/** A bell on a 24-unit square, filled in `currentColor` so every skin's button color applies. */
export function InboxIcon(): ReactElement {
  return (
    <svg viewBox="0 0 24 24" width="20" height="20" focusable="false">
      <path d="M12 2.5a1.5 1.5 0 0 1 1.5 1.5v.6c2.7.7 4.5 3.1 4.5 6v3.3l1.8 2.7a1 1 0 0 1-.8 1.6H5a1 1 0 0 1-.8-1.6L6 13.9v-3.3c0-2.9 1.8-5.3 4.5-6V4A1.5 1.5 0 0 1 12 2.5z" fill="currentColor" />
      <path d="M9.5 19.5h5a2.5 2.5 0 0 1-5 0z" fill="currentColor" />
    </svg>
  );
}

export function InboxBell(): ReactElement | null {
  useLocale();
  const reads = useSocialReads();
  const inbox = useInboxReads();
  const invites = useInvites();
  const now = useNow(INBOX_TICK_MS);
  const panelId = useId();
  const button = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);

  // Signed out, a session the server refuses, or no read of the challenges yet: nothing to show.
  const active = readSession() !== null && !reads.refused && reads.challengeReads > 0;
  useInboxPoll(active, reads.challengeReads);

  /** Gives the focus back to the bell, as a modal should leave it (WAI-ARIA APG; `FriendsButton.tsx`). */
  const closePanel = useCallback((): void => {
    setOpen(false);
    const element = button.current;
    if (element !== null && element.isConnected) element.focus({ preventScroll: true });
  }, []);

  if (!active) return null;

  const items = inboxItems(
    {
      challenges: reads.incoming,
      invites,
      daily: inbox.daily,
      requests: Array.isArray(inbox.friends?.incoming) ? inbox.friends.incoming : [],
      shared: inbox.shared ?? [],
    },
    now,
  );
  // Until the daily list has been read, the social read's count stands for the games on the player's move.
  const count = inboxBadge(items, inbox.seen) + (inbox.daily === null ? reads.dailyWaiting : 0);

  return (
    <>
      <button
        ref={button}
        type="button"
        className="inbox-bell top-bar-button top-bar-button--round"
        data-testid={inboxTestid.bell}
        aria-label={count > 0 ? t("Inbox, {count} waiting for you", { count }) : t("Inbox")}
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-controls={open ? panelId : undefined}
        title={t("Inbox")}
        onClick={() => {
          setOpen(true);
        }}
      >
        <span aria-hidden="true" className="inbox-bell__icon">
          <InboxIcon />
        </span>
        {count > 0 ? (
          <span className="together-badge inbox-bell__badge" data-testid={inboxTestid.badge} aria-hidden="true">
            {count}
          </span>
        ) : null}
      </button>
      {open
        ? createPortal(
            <div className="tavern lobby together-screen inbox-layer">
              <InboxPanel id={panelId} items={items} now={now} onClose={closePanel} />
            </div>,
            document.body,
          )
        : null}
    </>
  );
}
