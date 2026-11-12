// The shared emote gate and ids (`./emotes.ts`, R643, R1340, R1341, R641): the client's copy of the
// engine's `SEAT_EMOTE_LIMITS` (CLAUDE.md rule 9) and the pure predicates over it. One `it` per rule,
// with the engine config's own numbers.

import { describe, expect, it } from "vitest";

import {
  EMOTE_COOLDOWN_MS,
  EMOTE_HAND_SIZE,
  EMOTE_IDS,
  EMOTE_WINDOW_MAX,
  EMOTE_WINDOW_MS,
  PORTRAIT_IDS,
  SEAT_EMOTE_LIMITS,
  DEFAULT_PORTRAIT,
  emoteGate,
  handHolds,
  isEmoteId,
  isPortraitId,
  isVoiceEmote,
  pickPortrait,
  portraitOrDefault,
  type EmoteId,
} from "./emotes.ts";

describe("emoteGate", () => {
  it("lets the first emote through with an empty history", () => {
    expect(emoteGate([], 10_000)).toEqual({ ok: true, sentAt: [] });
  });

  it("prunes a send at the window's edge while keeping one inside it", () => {
    const now = 100_000;
    expect(emoteGate([now - EMOTE_WINDOW_MS, now - EMOTE_WINDOW_MS + 1], now)).toEqual({
      ok: true,
      sentAt: [now - EMOTE_WINDOW_MS + 1],
    });
  });

  it("refuses inside the cooldown with the wait that is left", () => {
    const now = 50_000;
    const last = now - 500;
    expect(emoteGate([last], now)).toEqual({
      ok: false,
      retryAfterMs: EMOTE_COOLDOWN_MS - 500,
      sentAt: [last],
    });
  });

  it("lets one through exactly when the cooldown has passed", () => {
    const now = 50_000;
    const last = now - EMOTE_COOLDOWN_MS;
    expect(emoteGate([last], now)).toEqual({ ok: true, sentAt: [last] });
  });

  it("refuses a full window with the wait until its oldest send falls out", () => {
    const now = 80_000;
    const first = now - 1_000;
    const sentAt = [first, now - 900, now - 800, now - 700, now - EMOTE_COOLDOWN_MS];
    expect(sentAt).toHaveLength(EMOTE_WINDOW_MAX);
    expect(emoteGate(sentAt, now)).toEqual({
      ok: false,
      retryAfterMs: EMOTE_WINDOW_MS - (now - first),
      sentAt,
    });
  });

  it("lets one through with one fewer than the window's most in it", () => {
    const now = 80_000;
    const sentAt = [now - 900, now - 800, now - 700, now - EMOTE_COOLDOWN_MS];
    expect(sentAt).toHaveLength(EMOTE_WINDOW_MAX - 1);
    expect(emoteGate(sentAt, now)).toEqual({ ok: true, sentAt });
  });
});

describe("emote ids", () => {
  it("knows every id it lists and nothing else", () => {
    for (const id of EMOTE_IDS) expect(isEmoteId(id)).toBe(true);
    expect(isEmoteId("greetings")).toBe(true);
    expect(isEmoteId("party")).toBe(true);
    expect(isEmoteId("not-an-emote")).toBe(false);
    expect(isEmoteId(null)).toBe(false);
    expect(isEmoteId(42)).toBe(false);
  });

  it("calls only the five portrait lines voice", () => {
    expect(isVoiceEmote("greetings")).toBe(true);
    expect(isVoiceEmote("threaten")).toBe(true);
    expect(isVoiceEmote("fire")).toBe(false);
  });

  it("reads a hand by membership", () => {
    const hand: EmoteId[] = ["greetings", "fire"];
    expect(handHolds(hand, "fire")).toBe(true);
    expect(handHolds(hand, "heart")).toBe(false);
    expect(hand).toHaveLength(2);
    expect(EMOTE_HAND_SIZE).toBeGreaterThan(2);
  });
});

describe("portraits", () => {
  it("knows every portrait it lists and nothing else", () => {
    for (const id of PORTRAIT_IDS) expect(isPortraitId(id)).toBe(true);
    expect(isPortraitId("vanilla")).toBe(true);
    expect(isPortraitId("gandalf")).toBe(false);
    expect(isPortraitId(null)).toBe(false);
  });

  it("reads a saved null as the default portrait", () => {
    expect(portraitOrDefault(null)).toBe(DEFAULT_PORTRAIT);
    expect(portraitOrDefault(undefined)).toBe(DEFAULT_PORTRAIT);
    expect(portraitOrDefault("no-such-portrait")).toBe(DEFAULT_PORTRAIT);
    expect(portraitOrDefault("gary")).toBe("gary");
  });

  it("picks the first portrait on a zero roll and clamps a one roll to the last", () => {
    expect(pickPortrait(() => 0)).toBe(PORTRAIT_IDS[0]);
    expect(pickPortrait(() => 1)).toBe(PORTRAIT_IDS[PORTRAIT_IDS.length - 1]);
  });
});

describe("seat limits", () => {
  it("is the engine config's three numbers", () => {
    expect(SEAT_EMOTE_LIMITS).toEqual({
      cooldownMs: EMOTE_COOLDOWN_MS,
      windowMs: EMOTE_WINDOW_MS,
      windowMax: EMOTE_WINDOW_MAX,
    });
  });
});
