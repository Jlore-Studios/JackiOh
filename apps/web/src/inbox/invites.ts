// The lobby and co-op invites this tab was sent (R3730005, R3729002), kept for the inbox. The social socket's
// `lobbyInvite` frame is the only place the server says one, and it says it once, so the tab holds it: in
// memory, for `INBOX_INVITE_KEEP_MS` (`order.ts` drops an older one), and gone with the tab. Nothing is stored
// on the server or the device. The toast (`lobby/LobbyInviteToast.tsx`) hides on its own flag, so Dismiss
// leaves the invite here. CLAUDE.md rule 7: an invite only names a lobby; the lobby's screen enters it, and the
// server decides.

import { useSyncExternalStore } from "react";

import type { LobbyInvite } from "../together/socialPoll.ts";

/** An invite as the inbox holds it: the frame's, the time it came and whether its toast is out of the way. */
export type InboxInvite = LobbyInvite & {
  /** Epoch ms the tab heard of it. */
  receivedAt: number;
  /** The toast was answered or dismissed; the inbox still lists the invite. */
  toastHidden: boolean;
};

const NONE: readonly InboxInvite[] = Object.freeze([]);

let invites: readonly InboxInvite[] = NONE;
const listeners = new Set<() => void>();

function publish(next: readonly InboxInvite[]): void {
  invites = next;
  for (const listener of listeners) listener();
}

/** An invite arrived: it replaces one already held for the same lobby, and shows its toast again. */
export function noteInvite(invite: LobbyInvite, now: number = Date.now()): void {
  const kept = invites.filter((each) => each.code !== invite.code);
  publish([...kept, { code: invite.code, format: invite.format, from: invite.from, receivedAt: now, toastHidden: false }]);
}

/** The toast for `code` goes; the invite stays in the inbox. */
export function hideInviteToast(code: string): void {
  if (!invites.some((each) => each.code === code && !each.toastHidden)) return;
  publish(invites.map((each) => (each.code === code ? { ...each, toastHidden: true } : each)));
}

/** The invite for `code` is answered or dismissed: gone from the toast and the inbox. */
export function dropInvite(code: string): void {
  if (!invites.some((each) => each.code === code)) return;
  publish(invites.filter((each) => each.code !== code));
}

/** Every invite goes (a sign-out, a refused session): they were another account's. */
export function forgetInvites(): void {
  if (invites.length > 0) publish(NONE);
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function snapshot(): readonly InboxInvite[] {
  return invites;
}

/** The invites this tab holds, oldest first; the inbox's order is `order.ts`'s. */
export function useInvites(): readonly InboxInvite[] {
  return useSyncExternalStore(subscribe, snapshot, snapshot);
}
