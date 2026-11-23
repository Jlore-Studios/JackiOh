// R4039001: desktop notifications while JackiOh is open in a hidden tab. The player turns them on in the
// notification settings (`daily/NotificationSettings.tsx`), which asks the browser's permission; the choice is
// this browser's (`DESKTOP_NOTIFY_KEY`), never the server's. A row of the feed that the social socket pushes
// (`NotificationToast.tsx`) shows one when the tab is hidden, for the kinds that ask something of the player:
// their daily move, a daily challenge or its acceptance, a lobby invite and a friend request. A click focuses the tab and opens
// the row's place. R3576000 closes a hidden tab's socket after `HIDDEN_QUIET_MS`, so a tab hidden longer hears
// no push, and shows none; web push (`daily/push.ts`) is what reaches a closed tab.

import { SITE_NAME } from "../net/head.ts";
import { navigate } from "../net/navigate.ts";
import type { FeedKind, FeedNotification } from "../net/notifications.ts";
import { getStored, removeStored, setStored } from "../net/storage.ts";
import { DESKTOP_NOTIFY_KEY } from "./config.ts";
import { feedLine, feedPath } from "./feed.ts";

/** What the key holds while desktop notifications are on. */
const ON = "on";

/** The kinds a desktop notification is shown for: each asks something of the player. */
const DESKTOP_KINDS: readonly FeedKind[] = ["dailyMove", "dailyChallenge", "lobbyInvite", "friendRequest"];

/** The row asks something of the player: one of `DESKTOP_KINDS`, or a daily challenge of theirs accepted, whose
 * game now waits on them (R4039000 writes the accept as `dailyAnswered`, not as a `dailyMove`). */
function asksSomething(row: FeedNotification): boolean {
  return DESKTOP_KINDS.includes(row.kind) || (row.kind === "dailyAnswered" && row.payload.answer === "accepted");
}

/** The browser can show a notification from the page. */
export function desktopSupported(): boolean {
  return typeof window !== "undefined" && "Notification" in window;
}

/** The player turned them on here, and the browser still allows them. */
export function desktopOn(): boolean {
  return desktopSupported() && getStored(DESKTOP_NOTIFY_KEY) === ON && Notification.permission === "granted";
}

/** Asks the browser's permission and turns them on if it is given; answers whether they are on. */
export async function enableDesktop(): Promise<boolean> {
  if (!desktopSupported()) return false;
  const permission = await Notification.requestPermission();
  if (permission !== "granted") {
    removeStored(DESKTOP_NOTIFY_KEY);
    return false;
  }
  setStored(DESKTOP_NOTIFY_KEY, ON);
  return true;
}

export function disableDesktop(): void {
  removeStored(DESKTOP_NOTIFY_KEY);
}

/** Shows `row` as a desktop notification, when the tab is hidden, they are on and its kind asks something of the player. */
export function notifyDesktop(row: FeedNotification): void {
  if (document.visibilityState !== "hidden" || !desktopOn() || !asksSomething(row)) return;
  let shown: Notification;
  try {
    shown = new Notification(SITE_NAME, { body: feedLine(row), tag: `jackioh-${String(row.id)}` });
  } catch {
    // A browser that shows notifications only from a service worker (Chrome on Android) refuses the page's.
    return;
  }
  shown.onclick = () => {
    window.focus();
    const path = feedPath(row);
    if (path !== null) navigate(path);
    shown.close();
  };
}
