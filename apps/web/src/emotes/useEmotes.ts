// useEmotes: the React half of the emote session (R637, R638). session.ts holds the pure state —
// what shows, who is muted, when the shared gate admits a send — and this hook adds the three
// things a component tree needs that must not live in the pure half:
//
//   1. Re-rendering. The session's `subscribe` bumps a counter; `visible`/`gate`/`muted` read it
//      live, so a show lands in the same render that asked for it.
//   2. Expiry. Each show schedules the timeout that calls `expire`; a newer show's different key
//      makes an old timer a no-op, which is what "a new emote replaces the current one
//      immediately" (issue §5) costs in code.
//   3. Sound. `playEmote` (play.ts) runs on a shown emote: the voice channel for a voice line,
//      the effects channel for an emoji. A muted or gate-dropped emote returns before this.
//
// Every route uses it the same way: `send` for the local seat (admit → show → play → emit), the
// peer's relay through `receive`, `mute` from the opponent portrait's menu, and `portraitOf` from
// the server's portraits frame (or the practice/hotseat route's own deal).

import { useCallback, useEffect, useMemo, useReducer, useRef } from "react";

import type { EmoteGate, EmoteId, PlayerId, PortraitId } from "@jackioh/shared";
import { DEFAULT_PORTRAIT } from "@jackioh/shared";

import type { SoundSink } from "../audio/types.ts";
import { createEmoteSession, type EmoteSession, type EmoteShow } from "./session.ts";
import { emoteShowInfo, playEmote } from "./play.ts";

export type EmotesApi = {
  /** What `player`'s portrait shows right now, or null. */
  visible: (player: PlayerId) => EmoteShow | null;
  /**
   * The local seat's emote: shared-gate it, show + play on admit, then `emit` it (the wire, or the
   * practice AI's reply hook). Returns whether it was admitted — false is the silent drop.
   */
  send: (player: PlayerId, emote: EmoteId) => boolean;
  /** An emote arrived from `player`: shows + plays unless that player is muted. */
  receive: (player: PlayerId, emote: EmoteId) => boolean;
  /** Mute `player`'s emotes for the rest of the match (their current show leaves too). */
  mute: (player: PlayerId) => void;
  muted: (player: PlayerId) => boolean;
  /** The live limiter reading for `player` — the menu's grey-out. */
  gate: (player: PlayerId) => EmoteGate;
  /** `player`'s portrait id — the server's frame when it arrived, the default otherwise. */
  portraitOf: (player: PlayerId) => PortraitId;
  /** Raw session, for tests and for drivers that schedule their own shows. */
  session: EmoteSession;
};

export function useEmotes(opts: {
  /** The portraits frame the match sent ({ p1, p2 }), or the route's own deal; null → vanilla. */
  portraits?: { p1: PortraitId; p2: PortraitId } | null;
  /** Where an admitted local emote goes — `MatchClient.sendEmote`, the AI hook, or nothing. */
  emit?: (emote: EmoteId) => void;
  /** The audio engine; null silences every emote (they still show). */
  engine?: SoundSink | null;
  /** The device setting (settings store): mutes every player that isn't `you`, live. */
  globalMute?: boolean;
  /** The seat `globalMute` spares: the local player. Hotseat passes the seat on move. */
  you?: PlayerId;
}): EmotesApi {
  const { portraits = null, emit, engine = null, globalMute = false, you = "p1" } = opts;
  const sessionRef = useRef<EmoteSession | null>(null);
  if (sessionRef.current === null) sessionRef.current = createEmoteSession();
  const session = sessionRef.current;
  const timers = useRef(new Map<PlayerId, number>());
  const [, bump] = useReducer((tick: number) => tick + 1, 0);

  const portraitOf = useCallback(
    (player: PlayerId): PortraitId => portraits?.[player] ?? DEFAULT_PORTRAIT,
    [portraits],
  );

  // Expiry: one timeout per player, keyed to its show. Replaced shows leave stale timers that
  // expire checks the key on, so nothing needs cancelling except on unmount.
  const armExpiry = useCallback(
    (player: PlayerId, show: EmoteShow): void => {
      const existing = timers.current.get(player);
      if (existing !== undefined) window.clearTimeout(existing);
      const holdMs = Math.max(0, show.until - Date.now());
      timers.current.set(
        player,
        window.setTimeout(() => {
          session.expire(player, show.key);
        }, holdMs),
      );
    },
    [session],
  );
  useEffect(() => session.subscribe(bump), [session]);
  useEffect(() => {
    const held = timers.current;
    return () => {
      for (const timer of held.values()) window.clearTimeout(timer);
      held.clear();
    };
  }, []);

  const showAndPlay = useCallback(
    (player: PlayerId, emote: EmoteId): void => {
      const portrait = portraitOf(player);
      const show = session.visible(player);
      if (show !== null) armExpiry(player, { ...show, until: show.until });
      if (engine !== null) playEmote(engine, portrait, emote);
    },
    [armExpiry, engine, portraitOf, session],
  );

  // `send`/`receive` wrap the session: it decides admit/show; the hook then reads the show back,
  // arms its expiry and plays its sound.
  const send = useCallback(
    (player: PlayerId, emote: EmoteId): boolean => {
      const info = emoteShowInfo(portraitOf(player), emote);
      if (!session.send(player, emote, info.text, info.holdMs)) return false;
      showAndPlay(player, emote);
      emit?.(emote);
      return true;
    },
    [emit, portraitOf, session, showAndPlay],
  );

  const receive = useCallback(
    (player: PlayerId, emote: EmoteId): boolean => {
      const info = emoteShowInfo(portraitOf(player), emote);
      if (!session.receive(player, emote, info.text, info.holdMs)) return false;
      showAndPlay(player, emote);
      return true;
    },
    [portraitOf, session, showAndPlay],
  );

  const mute = useCallback(
    (player: PlayerId): void => {
      session.mute(player);
      const timer = timers.current.get(player);
      if (timer !== undefined) window.clearTimeout(timer);
      timers.current.delete(player);
    },
    [session],
  );

  // The device setting (issue §5): turning it on mid-match silences everyone but the local seat,
  // including a bubble already on screen. Turning it off only stops muting NEW emotes — a player
  // muted through the menu stays muted for the match (the session owns that).
  useEffect(() => {
    if (!globalMute) return;
    for (const player of ["p1", "p2"] as const) {
      if (player !== you) session.mute(player);
    }
  }, [globalMute, session, you]);

  const gate = useCallback(
    (player: PlayerId) => session.gate(player),
    [session],
  );

  return useMemo(
    () => ({
      visible: (player: PlayerId) => session.visible(player),
      send,
      receive,
      mute,
      muted: (player: PlayerId) => session.muted(player),
      gate,
      portraitOf,
      session,
    }),
    [session, send, receive, mute, gate, portraitOf],
  );
}
