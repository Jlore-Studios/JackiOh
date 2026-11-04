// useSideline: the React half of the sideline session (R2990000, R2990002, R2990004), as `useEmotes` is
// the seats': re-rendering, expiry and sound. A watcher's send outside its dealt hand is dropped before
// the gate, as the server drops it (R2990001); an admitted one shows and sounds at once and goes out
// through `emit`. A relayed one shows and sounds unless the session is muted, by its Mute or the
// device's `muteSpectatorEmotes`. Every show stands EMOTE_EMOJI_MS, an emoji sticker's whole show.

import { useCallback, useEffect, useMemo, useReducer, useRef } from "react";

import type { EmojiEmoteId, EmoteGate, EmoteId } from "@jackioh/shared";

import type { SoundSink } from "../audio/types.ts";
import { EMOTE_EMOJI_MS } from "../emotes/config.ts";
import { EMOJI_SFX } from "../emotes/play.ts";
import { SELF, createSidelineSession, type SidelineSession, type SidelineShow } from "./session.ts";

/** What a board draws in its sideline strip (`SidelineStrip.tsx`). */
export type GameSideline = {
  shows: readonly SidelineShow[];
  /** R2990001: the watcher's dealt hand, which its menu offers; empty for a player. */
  hand: readonly EmojiEmoteId[];
  /** The watcher's pick: false when it is outside the hand or the gate holds it (R2990002). */
  send: (emote: EmoteId) => boolean;
  gate: () => EmoteGate;
  mute: () => void;
  muted: boolean;
  /** Whether anything has been shown this session. */
  heard: boolean;
};

export type SidelineApi = GameSideline & {
  /** A watcher's emote the server relayed (R2990000); false when muted. */
  receive: (watcher: string, emote: EmojiEmoteId) => boolean;
};

const NO_HAND: readonly EmojiEmoteId[] = [];

export function useSideline(
  opts: {
    /** R2990001: the watcher's hand from its `spectatorHand` frame; none for a player. */
    hand?: readonly EmojiEmoteId[];
    /** Where an admitted pick goes: the watch socket. */
    emit?: (emote: EmojiEmoteId) => void;
    /** The audio engine; null silences every show (they still show). */
    engine?: SoundSink | null;
    /** R2990004: the device's `muteSpectatorEmotes`: mutes the session, live. */
    globalMute?: boolean;
  } = {},
): SidelineApi {
  const { hand = NO_HAND, emit, engine = null, globalMute = false } = opts;
  const sessionRef = useRef<SidelineSession | null>(null);
  if (sessionRef.current === null) sessionRef.current = createSidelineSession();
  const session = sessionRef.current;
  const timers = useRef(new Map<string, number>());
  const [, bump] = useReducer((tick: number) => tick + 1, 0);

  useEffect(() => session.subscribe(bump), [session]);
  useEffect(() => {
    const held = timers.current;
    return () => {
      for (const timer of held.values()) window.clearTimeout(timer);
      held.clear();
    };
  }, []);

  // One expiry per watcher, keyed to its show: a replaced show's timer is cleared, and `expire`
  // checks the key besides.
  const showAndPlay = useCallback(
    (watcher: string, emote: EmojiEmoteId): void => {
      const show = session.shows().find((standing) => standing.watcher === watcher);
      if (show === undefined) return;
      const existing = timers.current.get(watcher);
      if (existing !== undefined) window.clearTimeout(existing);
      timers.current.set(
        watcher,
        window.setTimeout(() => {
          session.expire(watcher, show.key);
        }, Math.max(0, show.until - Date.now())),
      );
      engine?.playSfx(EMOJI_SFX[emote]);
    },
    [engine, session],
  );

  const send = useCallback(
    (emote: EmoteId): boolean => {
      // R2990001: before the gate, so a refused pick spends none of the limit.
      const picked = hand.find((held) => held === emote);
      if (picked === undefined || !session.send(picked, EMOTE_EMOJI_MS)) return false;
      showAndPlay(SELF, picked);
      emit?.(picked);
      return true;
    },
    [emit, hand, session, showAndPlay],
  );

  const receive = useCallback(
    (watcher: string, emote: EmojiEmoteId): boolean => {
      if (!session.receive(watcher, emote, EMOTE_EMOJI_MS)) return false;
      showAndPlay(watcher, emote);
      return true;
    },
    [session, showAndPlay],
  );

  const mute = useCallback((): void => {
    session.mute();
    for (const [watcher, timer] of timers.current) {
      if (watcher === SELF) continue;
      window.clearTimeout(timer);
      timers.current.delete(watcher);
    }
  }, [session]);

  // R2990004: the device setting mutes the session, a standing show included. Like a Mute pressed, it
  // lasts the session: turning the setting off mid-match unmutes nothing.
  useEffect(() => {
    if (globalMute) mute();
  }, [globalMute, mute]);

  const gate = useCallback(() => session.gate(), [session]);

  // Read as each notify re-renders: `shows()` is a new list on every change the session makes.
  const shows = session.shows();
  const muted = session.muted();
  const heard = session.heard();
  return useMemo(
    () => ({ shows, hand, send, receive, gate, mute, muted, heard }),
    [shows, hand, send, receive, gate, mute, muted, heard],
  );
}
