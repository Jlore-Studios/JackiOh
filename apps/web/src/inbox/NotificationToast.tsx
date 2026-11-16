// R4039001: one toast for the newest row the server writes to the player's notification feed, mounted once beside
// the challenge and lobby invite toasts (`main.tsx`). Each row the social socket pushes replaces the toast
// before it (the inbox's Recent section keeps them all); a `lobbyInvite` has its own toast
// (`lobby/LobbyInviteToast.tsx`) and shows none here. While a board is up the toast keeps out of the way and
// shows once it goes. Each push is also a desktop notification when the tab is hidden (`desktop.ts`).
// CLAUDE.md rule 7: Open goes where the row names; the screen there reads its own state.

import { useEffect, useState, type ReactElement } from "react";

import { useBoardShown } from "../game/boardShown.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { navigate } from "../net/navigate.ts";
import type { FeedNotification } from "../net/notifications.ts";
import { onNotificationPush } from "../together/socialPoll.ts";
import { Named } from "../together/words.tsx";
import { notifyDesktop } from "./desktop.ts";
import { feedName, feedPath, feedSentence } from "./feed.ts";
import { inboxTestid } from "./testids.ts";

import "../together/together.css";
import "./inbox.css";

export function NotificationToast(): ReactElement {
  useLocale();
  const board = useBoardShown();
  const [pending, setPending] = useState<FeedNotification | null>(null);

  useEffect(
    () =>
      onNotificationPush((row) => {
        notifyDesktop(row);
        if (row.kind !== "lobbyInvite") setPending(row);
      }),
    [],
  );

  const path = pending === null ? null : feedPath(pending);
  return (
    <div className="notification-toasts" aria-live="polite">
      {pending === null || board ? null : (
        <section className="notice challenge-toast" data-testid={inboxTestid.toast} data-kind={pending.kind} aria-label={t("Notification")}>
          <p className="challenge-toast__text">
            <Named text={feedSentence(pending)} name={feedName(pending)} />
          </p>
          <div className="challenge-toast__actions">
            {path === null ? null : (
              <button
                type="button"
                className="button-primary"
                data-testid={inboxTestid.toastOpen}
                onClick={() => {
                  if (navigate(path)) setPending(null);
                }}
              >
                {t("Open")}
              </button>
            )}
            <button
              type="button"
              className="button-secondary"
              data-testid={inboxTestid.toastDismiss}
              onClick={() => {
                setPending(null);
              }}
            >
              {t("Dismiss")}
            </button>
          </div>
        </section>
      )}
    </div>
  );
}
