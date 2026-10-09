// The emote session (R643, R644): what is on screen, who is muted, and when the next send is admitted,
// as pure data plus a notify, so the unit tests run the same object the routes drive.
//
// `send` is the local player's emote: the shared `emoteGate`, the one limiter and the function the actor
// runs (R643), judges it, and an admitted emote shows at once, without the relay (issue §7). `receive` is
// the peer's (the actor's relay or the practice AI's persona), shown unless that player is muted, by
// `mute` or the `globalMute` setting; a muted emote is never kept and its sound never scheduled (issue
// §5). One emote per player: `show` replaces it. No timers here (tests inject `now`): `useEmotes` owns
// expiry and sound. Players are keyed by PlayerId, so a hotseat hand-over does not move a bubble.

import type { EmoteId, PlayerId } from "@jackioh/shared";
import { emoteGate, type EmoteGate } from "@jackioh/shared";

/** What the board draws at one player's portrait until `until`. */
export type EmoteShow = {
  /** Bumped on every show, so the same emote twice still re-mounts the bubble. */
  key: number;
  emote: EmoteId;
  /** The voice line's text; null for an emoji emote (the sticker carries no words). */
  text: string | null;
  /** `now` at which the bubble leaves. */
  until: number;
};

export type EmoteSession = {
  /**
   * `player` asked for `emote`. Admitted: it shows and returns true, and the caller transmits it.
   * Dropped by the gate (R643): false, nothing shown or sent.
   */
  send: (player: PlayerId, emote: EmoteId, text: string | null, holdMs: number) => boolean;
  /** An emote arrived from `player`: shown unless muted; returns whether it was shown. */
  receive: (player: PlayerId, emote: EmoteId, text: string | null, holdMs: number) => boolean;
  /** What `player`'s portrait displays, or null. */
  visible: (player: PlayerId) => EmoteShow | null;
  /** The gate state for `player` at `now`: the menu's grey-out data. */
  gate: (player: PlayerId, now?: number) => EmoteGate;
  /** Mute `player` for the rest of the match: their current emote leaves. */
  mute: (player: PlayerId) => void;
  muted: (player: PlayerId) => boolean;
  /** Forget `player`'s emote, but only if it is still `key` (a newer show keeps standing). */
  expire: (player: PlayerId, key: number) => void;
  subscribe: (listener: () => void) => () => void;
};

export function createEmoteSession(opts?: { now?: () => number }): EmoteSession {
  const now = opts?.now ?? Date.now;
  const sentAt: Record<PlayerId, number[]> = { p1: [], p2: [] };
  const shown = new Map<PlayerId, EmoteShow>();
  const mutedPlayers = new Set<PlayerId>();
  const listeners = new Set<() => void>();
  let seq = 0;

  function emit(): void {
    for (const listener of [...listeners]) listener();
  }

  function show(player: PlayerId, emote: EmoteId, text: string | null, holdMs: number): EmoteShow {
    seq += 1;
    const entry: EmoteShow = { key: seq, emote, text, until: now() + holdMs };
    shown.set(player, entry);
    emit();
    return entry;
  }

  return {
    send(player, emote, text, holdMs) {
      const gate = emoteGate(sentAt[player], now());
      sentAt[player] = [...gate.sentAt];
      if (!gate.ok) return false;
      sentAt[player].push(now());
      show(player, emote, text, holdMs);
      return true;
    },
    receive(player, emote, text, holdMs) {
      if (mutedPlayers.has(player)) return false;
      show(player, emote, text, holdMs);
      return true;
    },
    visible: (player) => shown.get(player) ?? null,
    gate: (player, at) => emoteGate(sentAt[player], at ?? now()),
    mute(player) {
      mutedPlayers.add(player);
      if (shown.delete(player)) emit();
    },
    muted: (player) => mutedPlayers.has(player),
    expire(player, key) {
      if (shown.get(player)?.key === key) {
        shown.delete(player);
        emit();
      }
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}
