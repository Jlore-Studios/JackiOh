// The service worker of daily games' web push (#3727; apps/web/src/daily/push.ts registers it at the
// site's root). It caches nothing and serves no request: it shows the push the server sends as a
// notification, `{title, body, url}` in JSON, and focuses a tab already at `url` when the notification is
// clicked, else opens one: it never moves another tab of the site, which may be in a live match. A push
// it cannot read still shows the site's name, as browsers require a notification for every push. The
// worker's globals are read off `globalThis` (its `self`).

const worker = globalThis;

const FALLBACK_TITLE = "JackiOh";
const FALLBACK_URL = "/daily";
const ICON = "/apple-touch-icon.png";

/** Only a path on this site is opened. */
function sitePath(value) {
  return typeof value === "string" && value.startsWith("/") && !value.startsWith("//") ? value : FALLBACK_URL;
}

/** The push's fields, or the fallbacks. */
function readPush(data) {
  let parsed;
  try {
    parsed = data ? data.json() : null;
  } catch {
    parsed = null;
  }
  const fields = typeof parsed === "object" && parsed !== null ? parsed : {};
  const title = typeof fields.title === "string" && fields.title !== "" ? fields.title : FALLBACK_TITLE;
  const body = typeof fields.body === "string" ? fields.body : "";
  return { title, body, url: sitePath(fields.url) };
}

worker.addEventListener("push", (event) => {
  const { title, body, url } = readPush(event.data);
  event.waitUntil(worker.registration.showNotification(title, { body, icon: ICON, badge: ICON, tag: url, data: { url } }));
});

worker.addEventListener("notificationclick", (event) => {
  event.notification.close();
  const data = event.notification.data;
  const target = worker.location.origin + sitePath(data ? data.url : null);
  event.waitUntil(
    worker.clients.matchAll({ type: "window", includeUncontrolled: true }).then((windows) => {
      const open = windows.find((client) => client.url === target);
      if (open !== undefined) return open.focus();
      return worker.clients.openWindow(target);
    }),
  );
});
