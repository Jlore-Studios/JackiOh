// Web push for daily games (#3727): this device's subscription, made from the server's public key
// (`GET /api/daily/settings`'s `vapidPublicKey`; null, the server sends no push) and kept at the server
// (`POST /api/daily/push`), which pushes `{title, body, url}` when a game waits on the player.
// `public/sw.js` shows it and opens the game when it is clicked.
//
// The subscription's address is also kept in this browser (`DAILY_PUSH_ENDPOINT_KEY`), so sign-out can
// drop it at the server with the session's own token before the token goes (`forgetPushOnSignOut`).
// CLAUDE.md rule 7: when to push, and to which device, is the server's.

import { deletePushSubscription, deletePushSubscriptionAs, addPushSubscription } from "../net/daily.ts";
import { getStored, removeStored, setStored } from "../net/storage.ts";
import { DAILY_PUSH_ENDPOINT_KEY, DAILY_SERVICE_WORKER_PATH } from "./config.ts";

/** A refusal of the browser's, in a player's words: shown as it is. */
export class PushRefused extends Error {}

/** The browser can subscribe to push here (a service worker, the Push API and notifications). */
export function pushSupported(): boolean {
  return (
    typeof navigator !== "undefined" &&
    "serviceWorker" in navigator &&
    typeof window !== "undefined" &&
    "PushManager" in window &&
    "Notification" in window
  );
}

/** An iPhone's or an iPad's browser; iPadOS names itself a Mac with a touch screen. */
function appleMobile(): boolean {
  if (typeof navigator === "undefined") return false;
  if (/\b(iPhone|iPad|iPod)\b/u.test(navigator.userAgent)) return true;
  return /\bMacintosh\b/u.test(navigator.userAgent) && navigator.maxTouchPoints > 1;
}

/** Opened from the Home Screen (or as an installed app). */
function standalone(): boolean {
  if ((navigator as Navigator & { standalone?: boolean }).standalone === true) return true;
  return typeof window.matchMedia === "function" && window.matchMedia("(display-mode: standalone)").matches;
}

/** iOS and iPadOS send web push only to a site added to the Home Screen and opened from there. */
export function needsHomeScreen(): boolean {
  return appleMobile() && !standalone();
}

/** The server's VAPID key, base64url, as `applicationServerKey` takes it. */
export function keyBytes(base64url: string): Uint8Array<ArrayBuffer> {
  const padded = base64url.replace(/-/gu, "+").replace(/_/gu, "/").padEnd(Math.ceil(base64url.length / 4) * 4, "=");
  const raw = atob(padded);
  const bytes = new Uint8Array(new ArrayBuffer(raw.length));
  for (let index = 0; index < raw.length; index += 1) bytes[index] = raw.charCodeAt(index);
  return bytes;
}

async function registration(): Promise<ServiceWorkerRegistration> {
  await navigator.serviceWorker.register(DAILY_SERVICE_WORKER_PATH);
  return navigator.serviceWorker.ready;
}

/** This device's subscription as the browser holds it, or null (none, or no push here). */
export async function currentSubscription(): Promise<PushSubscription | null> {
  if (!pushSupported()) return null;
  const found = await navigator.serviceWorker.getRegistration(DAILY_SERVICE_WORKER_PATH);
  return (await found?.pushManager.getSubscription()) ?? null;
}

/** Whether this device has push on: the browser holds a subscription this browser sent the server. */
export async function pushOnHere(): Promise<boolean> {
  const subscription = await currentSubscription();
  return subscription !== null && subscription.endpoint === getStored(DAILY_PUSH_ENDPOINT_KEY);
}

/** Asks for notifications, subscribes with the server's key and sends the subscription. */
export async function enablePush(vapidPublicKey: string): Promise<void> {
  const permission = await Notification.requestPermission();
  if (permission !== "granted") {
    throw new PushRefused("Notifications are blocked for this site. Allow them in your browser's settings, then try again.");
  }
  const worker = await registration();
  const subscription =
    (await worker.pushManager.getSubscription()) ??
    (await worker.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: keyBytes(vapidPublicKey) }));
  const { endpoint, keys } = subscription.toJSON();
  const p256dh = keys?.["p256dh"];
  const auth = keys?.["auth"];
  if (endpoint === undefined || p256dh === undefined || auth === undefined) {
    throw new PushRefused("This browser didn't give us a way to reach it. Try another browser.");
  }
  await addPushSubscription({ endpoint, keys: { p256dh, auth } });
  setStored(DAILY_PUSH_ENDPOINT_KEY, endpoint);
}

/** Drops this device's subscription at the server and in the browser. */
export async function disablePush(): Promise<void> {
  const subscription = await currentSubscription();
  const endpoint = subscription?.endpoint ?? getStored(DAILY_PUSH_ENDPOINT_KEY);
  if (endpoint !== null) await deletePushSubscription(endpoint);
  removeStored(DAILY_PUSH_ENDPOINT_KEY);
  await subscription?.unsubscribe();
}

/**
 * At sign-out, before the session goes: the server forgets this device's subscription (sent with the
 * session's token, outliving the page), and the browser drops it. A failure changes nothing: the server
 * also drops a subscription its push service says is gone.
 */
export function forgetPushOnSignOut(token: string): void {
  const endpoint = getStored(DAILY_PUSH_ENDPOINT_KEY);
  if (endpoint === null) return;
  removeStored(DAILY_PUSH_ENDPOINT_KEY);
  void deletePushSubscriptionAs(token, endpoint).catch(() => undefined);
  void currentSubscription()
    .then((subscription) => subscription?.unsubscribe())
    .catch(() => undefined);
}
