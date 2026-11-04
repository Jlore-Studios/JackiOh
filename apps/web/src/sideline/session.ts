// The sideline's session (R2990002, R2990004), as `emotes/session.ts` is the seats': pure data plus a
// notify, so the unit tests run the object `useSideline` drives. A watcher's own send is judged by the
// shared `emoteGate` at the server's numbers for a watcher, which the server judges it by too, and
// shows at once under `SELF`, since the server never echoes it back (R2990000). A relayed watcher's
// emote shows once per watcher, the newest last. Mute lasts the session and keeps the sender's own.
// No timers here (tests inject `now`): `useSideline` owns expiry and sound.

import { SPECTATOR_EMOTE_COOLDOWN_MS, SPECTATOR_EMOTE_WINDOW_MAX, SPECTATOR_EMOTE_WINDOW_MS } from "@jackioh/server-config";
import { emoteGate, type EmojiEmoteId, type EmoteGate, type EmoteLimits } from "@jackioh/shared";

/** R2990002: a watcher's limit, the server's three numbers. */
export const SPECTATOR_EMOTE_LIMITS: EmoteLimits = {
  cooldownMs: SPECTATOR_EMOTE_COOLDOWN_MS,
  windowMs: SPECTATOR_EMOTE_WINDOW_MS,
  windowMax: SPECTATOR_EMOTE_WINDOW_MAX,
};

/** The key of the sender's own shows: no relayed watcher's name is empty (`spectatorEmoteOf`). */
export const SELF = "";

/** One emote standing in the strip until `until`. */
export type SidelineShow = {
  /** Bumped on every show, so the same emote twice still re-mounts. */
  key: number;
  /** The watcher's name as the server relayed it; `SELF` for the sender's own. */
  watcher: string;
  emote: EmojiEmoteId;
  self: boolean;
  until: number;
};

export type SidelineSession = {
  /** Admitted (R2990002): shows under `SELF` and returns true, and the caller sends it; dropped: false. */
  send: (emote: EmojiEmoteId, holdMs: number) => boolean;
  /** A watcher's relayed emote: shown, replacing that watcher's last, unless muted. */
  receive: (watcher: string, emote: EmojiEmoteId, holdMs: number) => boolean;
  /** What stands now, oldest first. */
  shows: () => readonly SidelineShow[];
  /** The gate at `at` (now by default): the menu's grey-out. */
  gate: (at?: number) => EmoteGate;
  /** Forget `watcher`'s show, but only if it is still `key` (a newer show keeps standing). */
  expire: (watcher: string, key: number) => void;
  /** R2990004: every other watcher's show leaves, and none lands again this session. */
  mute: () => void;
  muted: () => boolean;
  /** Whether anything has been shown this session. */
  heard: () => boolean;
  subscribe: (listener: () => void) => () => void;
};

export function createSidelineSession(opts?: { now?: () => number }): SidelineSession {
  const now = opts?.now ?? Date.now;
  let sentAt: readonly number[] = [];
  const shown = new Map<string, SidelineShow>();
  let list: readonly SidelineShow[] = [];
  let isMuted = false;
  let wasHeard = false;
  const listeners = new Set<() => void>();
  let seq = 0;

  function emit(): void {
    list = [...shown.values()];
    for (const listener of [...listeners]) listener();
  }

  function show(watcher: string, emote: EmojiEmoteId, holdMs: number): void {
    seq += 1;
    // Deleted first, so the newest show is the map's last.
    shown.delete(watcher);
    shown.set(watcher, { key: seq, watcher, emote, self: watcher === SELF, until: now() + holdMs });
    wasHeard = true;
    emit();
  }

  return {
    send(emote, holdMs) {
      const gate = emoteGate(sentAt, now(), SPECTATOR_EMOTE_LIMITS);
      sentAt = gate.sentAt;
      if (!gate.ok) return false;
      sentAt = [...sentAt, now()];
      show(SELF, emote, holdMs);
      return true;
    },
    receive(watcher, emote, holdMs) {
      if (isMuted) return false;
      show(watcher, emote, holdMs);
      return true;
    },
    shows: () => list,
    gate: (at) => emoteGate(sentAt, at ?? now(), SPECTATOR_EMOTE_LIMITS),
    expire(watcher, key) {
      if (shown.get(watcher)?.key !== key) return;
      shown.delete(watcher);
      emit();
    },
    mute() {
      if (isMuted) return;
      isMuted = true;
      for (const watcher of [...shown.keys()]) if (watcher !== SELF) shown.delete(watcher);
      emit();
    },
    muted: () => isMuted,
    heard: () => wasHeard,
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}
