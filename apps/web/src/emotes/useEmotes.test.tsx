// useEmotes: the React half of the emote session (R643, R644). The pure half is proved in
// session.test.ts; what this file proves is what only the hook does — a send shows and sounds
// locally without waiting for the relay, a muted or device-muted emote never reaches the audio
// engine, a newer emote's expiry timer is the only one that lands, and `globalMute` spares the
// seat `you` names.
//
// The engine seam is SoundSink's two methods, so it is a pair of spies. The expiry timers are
// `window.setTimeout`, which `vi.useFakeTimers()` owns — `useEmotes` injects no clock of its own,
// so the faked `Date.now` drives both the session's `until` and the timeout's delay.

import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { EMOTE_COOLDOWN_MS, type EmoteId, type PortraitId } from "@jackioh/shared";

import { VOICE_PRIORITY } from "../audio/constants.ts";
import type { SoundSink } from "../audio/types.ts";
import { EMOTE_EMOJI_MS } from "./config.ts";
import { useEmotes } from "./useEmotes.ts";

type Sink = SoundSink & {
  playSfx: ReturnType<typeof vi.fn>;
  playVoice: ReturnType<typeof vi.fn>;
  playEffect: ReturnType<typeof vi.fn>;
};

function engineSpy(): Sink {
  return { playSfx: vi.fn(() => true), playVoice: vi.fn(() => true), playEffect: vi.fn(() => true) };
}

type Options = {
  portraits?: { p1: PortraitId; p2: PortraitId } | null;
  emit?: (emote: EmoteId) => void;
  engine?: SoundSink | null;
  globalMute?: boolean;
  you?: "p1" | "p2";
};

function hook(initial: Options = {}) {
  return renderHook((opts: Options) => useEmotes(opts), { initialProps: initial });
}

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

describe("R642 the dealt portraits", () => {
  it("R642 portraitOf reads the server's frame, and vanilla before one arrives", () => {
    const dealt = hook({ portraits: { p1: "gary", p2: "shredder" } });
    expect(dealt.result.current.portraitOf("p1")).toBe("gary");
    expect(dealt.result.current.portraitOf("p2")).toBe("shredder");

    const waiting = hook();
    expect(waiting.result.current.portraitOf("p1")).toBe("vanilla");
    expect(waiting.result.current.portraitOf("p2")).toBe("vanilla");
    waiting.unmount();
  });
});

describe("R643 a local send", () => {
  it("R643 an admitted send shows and sounds at once, then is emitted — no relay waited on", () => {
    const engine = engineSpy();
    const emit = vi.fn();
    const { result } = hook({
      portraits: { p1: "gary", p2: "shredder" },
      emit,
      engine,
      you: "p1",
    });

    let sent = false;
    act(() => {
      sent = result.current.send("p1", "greetings");
    });

    expect(sent).toBe(true);
    // The bubble is up already: the sender's own line text, for that portrait.
    expect(result.current.visible("p1")).toMatchObject({
      emote: "greetings",
      text: "Hey there, friend! Feeling lucky?",
    });
    // The voice channel got the portrait's own defId before the wire ever could have relayed.
    expect(engine.playVoice).toHaveBeenCalledWith("emote-gary", "greetings", 0, VOICE_PRIORITY.react);
    expect(engine.playSfx).not.toHaveBeenCalled();
    expect(emit).toHaveBeenCalledTimes(1);
    expect(emit).toHaveBeenCalledWith("greetings");
  });

  it("R643 a send the gate drops emits nothing, plays nothing and leaves the menu's reading true", () => {
    const engine = engineSpy();
    const emit = vi.fn();
    const { result } = hook({ emit, engine, you: "p1" });

    act(() => {
      expect(result.current.send("p1", "laugh")).toBe(true);
    });
    expect(engine.playSfx).toHaveBeenCalledWith("emoteLaugh");

    let again = true;
    act(() => {
      again = result.current.send("p1", "laugh");
    });

    expect(again).toBe(false);
    const gate = result.current.gate("p1");
    expect(gate.ok).toBe(false);
    if (gate.ok === false) expect(gate.retryAfterMs).toBeGreaterThan(0);
    expect(result.current.visible("p1")).toMatchObject({ emote: "laugh" });
    expect(emit).toHaveBeenCalledTimes(1);
    expect(engine.playSfx).toHaveBeenCalledTimes(1);
  });

  it("R643 the gate the menu reads opens again once the cooldown has passed", () => {
    vi.useFakeTimers();
    const emit = vi.fn();
    const { result } = hook({ emit, engine: null });

    act(() => {
      expect(result.current.send("p1", "thanks")).toBe(true);
      expect(result.current.send("p1", "thanks")).toBe(false);
    });
    expect(result.current.gate("p1").ok).toBe(false);

    act(() => {
      vi.advanceTimersByTime(EMOTE_COOLDOWN_MS);
    });

    expect(result.current.gate("p1").ok).toBe(true);
    act(() => {
      expect(result.current.send("p1", "thanks")).toBe(true);
    });
    expect(emit).toHaveBeenCalledTimes(2);
  });
});

describe("R643 an opponent's emote", () => {
  it("R643 a received emote shows its sticker or bubble and plays its sound", () => {
    const engine = engineSpy();
    const { result } = hook({ portraits: { p1: "vanilla", p2: "shredder" }, engine, you: "p1" });

    act(() => {
      expect(result.current.receive("p2", "threaten")).toBe(true);
    });

    expect(result.current.visible("p2")).toMatchObject({
      emote: "threaten",
      text: "TARGET ACQUIRED. SHREDDING.",
    });
    expect(engine.playVoice).toHaveBeenCalledWith("emote-shredder", "threaten", 0, VOICE_PRIORITY.react);

    act(() => {
      expect(result.current.receive("p2", "sob")).toBe(true);
    });
    expect(result.current.visible("p2")).toMatchObject({ emote: "sob", text: null });
    expect(engine.playSfx).toHaveBeenCalledWith("emoteSob");
  });

  it("R643 a muted player's emote shows nothing and plays no sound", () => {
    const engine = engineSpy();
    const { result } = hook({ engine });

    act(() => {
      result.current.mute("p2");
    });
    expect(result.current.muted("p2")).toBe(true);

    let landed = true;
    act(() => {
      landed = result.current.receive("p2", "greetings");
    });

    expect(landed).toBe(false);
    expect(result.current.visible("p2")).toBeNull();
    expect(engine.playVoice).not.toHaveBeenCalled();
    expect(engine.playSfx).not.toHaveBeenCalled();
  });

  it("R643 mute takes the muted player's standing bubble down with it", () => {
    const { result } = hook({});

    act(() => {
      result.current.receive("p2", "laugh");
    });
    expect(result.current.visible("p2")).not.toBeNull();

    act(() => {
      result.current.mute("p2");
    });

    expect(result.current.visible("p2")).toBeNull();
    expect(result.current.muted("p2")).toBe(true);
  });

  it("R643 the device's muteOpponentEmotes setting silences every seat but you, live", () => {
    const engine = engineSpy();
    const { result, rerender } = hook({ engine, globalMute: false, you: "p1" });

    act(() => {
      result.current.receive("p2", "wellPlayed");
    });
    expect(result.current.visible("p2")).not.toBeNull();

    // The setting flips mid-match: the opponent's standing bubble leaves and they are muted.
    rerender({ engine, globalMute: true, you: "p1" });

    expect(result.current.muted("p2")).toBe(true);
    expect(result.current.muted("p1")).toBe(false);
    expect(result.current.visible("p2")).toBeNull();

    act(() => {
      expect(result.current.receive("p2", "angry")).toBe(false);
      expect(result.current.send("p1", "greetings")).toBe(true);
    });
    expect(result.current.visible("p1")?.emote).toBe("greetings");
    expect(engine.playVoice).toHaveBeenCalledTimes(2); // p2's own line, then yours — the muted one never played
  });

  it("R643 in a hotseat the seat on move is the one the setting spares", () => {
    // Hotseat hands `you` the seat whose turn it is; globalMute mutes only the other.
    const { result, rerender } = hook({ globalMute: true, you: "p1" });
    expect(result.current.muted("p2")).toBe(true);
    expect(result.current.muted("p1")).toBe(false);

    rerender({ globalMute: true, you: "p2" });

    expect(result.current.muted("p1")).toBe(true);
  });
});

describe("R643 one emote per player, expiring on its own span", () => {
  it("R643 a newer emote replaces the old, and the old show's timer cannot take it down", () => {
    vi.useFakeTimers();
    const { result } = hook({});

    act(() => {
      result.current.receive("p2", "sob");
    });
    act(() => {
      vi.advanceTimersByTime(EMOTE_EMOJI_MS / 2);
      result.current.receive("p2", "laugh");
    });
    const shown = result.current.visible("p2");
    expect(shown?.emote).toBe("laugh");

    // The first sticker's timer fires inside the second's span and must not expire it.
    act(() => {
      vi.advanceTimersByTime(EMOTE_EMOJI_MS / 2);
    });
    expect(result.current.visible("p2")).toBe(shown);

    act(() => {
      vi.advanceTimersByTime(EMOTE_EMOJI_MS / 2);
    });
    expect(result.current.visible("p2")).toBeNull();
  });

  it("R643 the bubble leaves when its span is up", () => {
    vi.useFakeTimers();
    const { result } = hook({});

    act(() => {
      result.current.receive("p2", "yawn");
    });
    expect(result.current.visible("p2")).not.toBeNull();

    act(() => {
      vi.advanceTimersByTime(EMOTE_EMOJI_MS - 1);
    });
    expect(result.current.visible("p2")).not.toBeNull();

    act(() => {
      vi.advanceTimersByTime(1);
    });
    expect(result.current.visible("p2")).toBeNull();
  });

  it("R643 a nulled engine still shows the emote — sound is cosmetic, the bubble is not", () => {
    const { result } = hook({ engine: null });

    act(() => {
      expect(result.current.receive("p2", "thanks")).toBe(true);
    });

    expect(result.current.visible("p2")).toMatchObject({ emote: "thanks", text: "Thanks. Simple as that." });
  });
});
