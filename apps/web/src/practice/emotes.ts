// Practice's emote driver (R642–R645, R1341, R1344): the AI's persona, both portraits and both
// hands, all on the page side, since the persona is presentation, not part of the AI's decisions
// (issue §6: "a separate RNG outside the engine and AI search"). `useEmotes` owns what every route
// shares (the session, the sounds, the shared gate, the mutes); this hook owns what only practice has.
// The hands (R1341) are each seat's eight, dealt from the game's seed as a match deals them.

import { useCallback, useEffect, useRef, useState } from "react";

import type { ActionBody, EmoteId, PlayerId, PlayerView, PortraitId } from "@jackioh/shared";
import { opponentOf, pickPortrait, portraitOrDefault } from "@jackioh/shared";
import { createEmotePersona, pickPersona, type AiEmote, type EmotePersona } from "@jackioh/ai";
import { AI_EMOTE } from "@jackioh/ai/config";

import { getAudioEngine } from "../audio/index.ts";
import { newEventsSince } from "../game/animations.ts";
import { dealEmoteHands } from "../emotes/deal.ts";
import { useEmotes, type EmotesApi } from "../emotes/useEmotes.ts";
import { useSetting } from "../settings/index.ts";
import type { PracticeSnapshot, PracticeStartConfig } from "./protocol.ts";

/** The portraits one game deals (R642), before seats are assigned: `[human, ai]`. */
function dealPortraits(config: PracticeStartConfig): { human: PortraitId; ai: PortraitId } {
  // The tutorial's rule is both `vanilla` (issue §1), whatever deck the lesson hands it.
  if (config.lesson !== undefined) return { human: "vanilla", ai: "vanilla" };
  const human =
    config.deck.kind === "saved"
      ? portraitOrDefault(config.deck.portrait)
      : config.deck.kind === "random"
        ? pickPortrait(Math.random)
        : "vanilla";
  return { human, ai: pickPortrait(Math.random) };
}

/** R645: the tutorial is always Silent (issue §6); the persona chooses within the AI seat's own hand (R1344). */
function dealPersona(config: PracticeStartConfig, seat: PlayerId, hand: readonly EmoteId[]): EmotePersona {
  const persona = config.lesson === undefined ? pickPersona(Math.random) : "silent";
  return createEmotePersona({ persona, seat, rng: Math.random, hand });
}

/**
 * One practice game's emote half, keyed to `config` (a new game is a new config object — the
 * controller hands `state.config` back verbatim).
 */
/**
 * MD-D29, R1127: the page's side of the `Emote` action, the player's sends. The persona's emotes stay
 * cosmetic (R643): they are no AI decision, so they never reach the game.
 */
export type PracticeEmotePort = {
  act(action: ActionBody): void;
};

export function usePracticeEmotes(
  config: PracticeStartConfig | null,
  snapshot: PracticeSnapshot | null,
  aiSeat: PlayerId | null,
  port?: PracticeEmotePort | null,
): EmotesApi {
  const globalMuteEmotes = useSetting("muteOpponentEmotes");
  const view = snapshot?.view ?? null;

  // The game's deal, made once when the AI seat is first known (the render-adjust pattern `concedeFor`
  // uses), so the first painted board already wears the dealt portraits; a ref too, for the closures.
  const [deal, setDeal] = useState<{
    config: PracticeStartConfig;
    persona: EmotePersona;
    portraits: { p1: PortraitId; p2: PortraitId };
    hands: { p1: readonly EmoteId[]; p2: readonly EmoteId[] };
  } | null>(null);
  if (config !== null && aiSeat !== null && deal?.config !== config) {
    const { human, ai } = dealPortraits(config);
    const hands = dealEmoteHands(config.seed);
    setDeal({
      config,
      persona: dealPersona(config, aiSeat, hands[aiSeat]),
      portraits: config.humanSeat === "p1" ? { p1: human, p2: ai } : { p1: ai, p2: human },
      hands,
    });
  }
  const dealRef = useRef(deal);
  dealRef.current = deal;

  // Delivery: an intent shows after its delay as a `receive`, where the AI's mute is honored like
  // anywhere else. A new game clears whatever timers the old one left pending.
  const timers = useRef<number[]>([]);
  const emotesRef = useRef<EmotesApi | null>(null);
  const portRef = useRef(port);
  portRef.current = port;
  const schedule = useCallback((intent: AiEmote, seat: PlayerId) => {
    const timer = window.setTimeout(() => {
      timers.current = timers.current.filter((held) => held !== timer);
      emotesRef.current?.receive(seat, intent.emote);
    }, intent.delayMs);
    timers.current.push(timer);
  }, []);
  useEffect(() => {
    const held = timers.current;
    return () => {
      for (const timer of held) window.clearTimeout(timer);
      held.length = 0;
    };
  }, [deal]);

  // The player's own send feeds the reply table before it would leave the board — §6's replies
  // answer the player, and "the AI never replies to a reply" is the module's own shape.
  const onPlayerEmote = useCallback(
    (emote: EmoteId) => {
      // MD-D29, R1127: the player's emote becomes an action only while the view hears emotes.
      if (view?.emotesHeard === true) portRef.current?.act({ type: "emote", emote });
      const current = dealRef.current;
      if (current === null || current.config !== config) return;
      const seat = opponentOf(config?.humanSeat ?? "p1");
      for (const intent of current.persona.onPlayerEmote(emote, Date.now())) {
        schedule(intent, seat);
      }
    },
    [config, schedule, view],
  );

  const emotes = useEmotes({
    portraits: deal?.portraits ?? null,
    hands: deal?.hands ?? null,
    emit: onPlayerEmote,
    engine: getAudioEngine(),
    globalMute: globalMuteEmotes,
    you: view?.viewer ?? config?.humanSeat ?? "p1",
  });
  emotesRef.current = emotes;

  // Every new snapshot's events run the table once, by the animation runner's own delta
  // (`newEventsSince`); practice has no seat hand-over, so the views always compare.
  const fed = useRef<{ config: PracticeStartConfig | null; prev: PlayerView | null }>({
    config: null,
    prev: null,
  });
  useEffect(() => {
    const current = dealRef.current;
    if (view === null || current === null || current.config !== config) {
      fed.current = { config, prev: null };
      return;
    }
    if (fed.current.prev === view) return;
    const prev = fed.current.config === config ? fed.current.prev : null;
    const fresh = prev === null ? [] : newEventsSince(prev.events, view.events);
    fed.current = { config, prev: view };
    const seat = opponentOf(view.viewer);
    for (const intent of current.persona.onEvents(fresh, prev, view, Date.now())) {
      schedule(intent, seat);
    }
  }, [view, config, schedule]);

  // §6's long-turn trigger: armed when the player's turn starts, fired once it has run
  // AI_EMOTE.longTurnMs — the persona's own turn-number check makes a second arm harmless.
  const longTurn = useRef<{ config: PracticeStartConfig | null; turn: number; timer: number | null }>({
    config: null,
    turn: -1,
    timer: null,
  });
  const latestView = useRef(view);
  latestView.current = view;
  useEffect(() => {
    const armed = longTurn.current;
    if (armed.timer !== null) {
      window.clearTimeout(armed.timer);
      armed.timer = null;
    }
    if (view === null || config === null || view.result !== null || view.active === undefined) {
      longTurn.current = { config, turn: -1, timer: null };
      return;
    }
    if (view.active !== view.viewer) {
      longTurn.current = { config, turn: view.turn, timer: null };
      return;
    }
    const timer = window.setTimeout(() => {
      const current = dealRef.current;
      const now = latestView.current;
      if (current === null || now === null) return;
      const seat = opponentOf(now.viewer);
      for (const intent of current.persona.onPlayerTurnLong(now, Date.now())) {
        schedule(intent, seat);
      }
    }, AI_EMOTE.longTurnMs);
    longTurn.current = { config, turn: view.turn, timer };
    return () => window.clearTimeout(timer);
  }, [view, config, schedule]);

  return emotes;
}
