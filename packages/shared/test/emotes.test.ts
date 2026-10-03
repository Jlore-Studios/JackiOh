// Emotes and hero portraits (SPEC §9.4 D5, §9.5, §10.10, R635–R637).
//
// `packages/shared/src/emotes.ts` is the one place both ends of the wire read the emote ids, the
// portrait roster and the rate limit from, so what is proved here holds for the client's greyed
// menu items and the server's drop alike (R637), and for the deck save's D5 check, which reads the
// same roster through the caller's `isPortrait` — proved end to end in the validator's own
// drafts.test.ts ("R635 …").

import { describe, expect, it } from "vitest";

import {
  DEFAULT_PORTRAIT,
  EMOJI_EMOTE_IDS,
  EMOTE_COOLDOWN_MS,
  EMOTE_IDS,
  EMOTE_WINDOW_MAX,
  EMOTE_WINDOW_MS,
  PORTRAITS,
  PORTRAIT_IDS,
  VOICE_EMOTE_IDS,
  emoteGate,
  isEmoteId,
  isPortraitId,
  isVoiceEmote,
  pickPortrait,
  pickPortraitFromSeed,
  portraitOrDefault,
} from "../src/index";

describe("R637 — the ten emote ids the wire spells", () => {
  it("R637 admits exactly the ten ids: the five voice lines, then the five emoji", () => {
    expect(VOICE_EMOTE_IDS).toEqual(["greetings", "wellPlayed", "oops", "thanks", "threaten"]);
    expect(EMOJI_EMOTE_IDS).toEqual(["sob", "yawn", "laugh", "angry", "wahWah"]);
    expect(EMOTE_IDS).toEqual([...VOICE_EMOTE_IDS, ...EMOJI_EMOTE_IDS]);
    for (const id of EMOTE_IDS) expect(isEmoteId(id), id).toBe(true);
  });

  it("R637 refuses an id the wire does not spell, and anything that is not a string", () => {
    for (const bad of ["", "GREETINGS", "greetings ", " emote", "sobbing", "wahWah!"]) {
      expect(isEmoteId(bad), JSON.stringify(bad)).toBe(false);
    }
    for (const notString of [undefined, null, 0, 5, true, {}, [], ["greetings"]]) {
      expect(isEmoteId(notString), JSON.stringify(notString)).toBe(false);
    }
  });

  it("R637 sorts every emote into a voice line or an emoji, never both, never neither", () => {
    for (const id of VOICE_EMOTE_IDS) expect(isVoiceEmote(id), id).toBe(true);
    for (const id of EMOJI_EMOTE_IDS) expect(isVoiceEmote(id), id).toBe(false);
    expect(EMOTE_IDS.filter(isVoiceEmote)).toEqual(VOICE_EMOTE_IDS);
  });
});

describe("R635 — the launch portrait roster a deck may carry", () => {
  it("R635 holds exactly the six issue portraits, each with a card name and a flavour", () => {
    expect(PORTRAIT_IDS).toEqual(["vanilla", "gary", "timmy", "dfender", "felinors", "shredder"]);
    expect(Object.keys(PORTRAITS)).toEqual([...PORTRAIT_IDS]);
    for (const id of PORTRAIT_IDS) {
      const entry = PORTRAITS[id];
      expect(entry.cardName.length, id).toBeGreaterThan(0);
      expect(entry.flavour.length, id).toBeGreaterThan(0);
    }
  });

  it("R635 takes vanilla as the default and reads null, undefined and unknown ids as it", () => {
    expect(DEFAULT_PORTRAIT).toBe("vanilla");
    expect(PORTRAIT_IDS[0]).toBe(DEFAULT_PORTRAIT);
    for (const id of PORTRAIT_IDS) {
      expect(isPortraitId(id), id).toBe(true);
      expect(portraitOrDefault(id), id).toBe(id);
    }
    expect(portraitOrDefault(null)).toBe(DEFAULT_PORTRAIT);
    expect(portraitOrDefault(undefined)).toBe(DEFAULT_PORTRAIT);
    expect(portraitOrDefault("bogus")).toBe(DEFAULT_PORTRAIT);
    expect(portraitOrDefault("")).toBe(DEFAULT_PORTRAIT);
  });

  it("R635 knows the six and nothing else: no other string, nothing that is not a string", () => {
    for (const bad of ["", "Vanilla", "vanilla ", "gary1", "portrait", "null"]) {
      expect(isPortraitId(bad), JSON.stringify(bad)).toBe(false);
    }
    for (const notString of [undefined, null, 0, true, {}, []]) {
      expect(isPortraitId(notString)).toBe(false);
    }
  });
});

describe("R636 — a match's portrait picks land on the roster", () => {
  it("R636 pickPortrait maps the bounds of rng onto the roster's ends", () => {
    expect(pickPortrait(() => 0)).toBe("vanilla");
    expect(pickPortrait(() => 0.999999)).toBe(PORTRAIT_IDS.at(-1));
    // A draw of exactly 1, which Math.random never returns but a foreign rng could, clamps on-roster.
    expect(pickPortrait(() => 1)).toBe(PORTRAIT_IDS.at(-1));
  });

  it("R636 pickPortrait deals every roster member over a sweep of draws, and nothing else", () => {
    const dealt = new Set<string>();
    for (let i = 0; i < 1000; i += 1) {
      const pick = pickPortrait(() => i / 1000);
      expect(isPortraitId(pick), `draw ${String(i / 1000)}`).toBe(true);
      dealt.add(pick);
    }
    expect(dealt.size).toBe(PORTRAIT_IDS.length);
  });

  it("R636 pickPortraitFromSeed is deterministic and deals each seat a roster id", () => {
    expect(pickPortraitFromSeed("match-42:portrait:p1")).toBe(
      pickPortraitFromSeed("match-42:portrait:p1"),
    );
    let differ = 0;
    for (let i = 0; i < 60; i += 1) {
      const p1 = pickPortraitFromSeed(`match-${i}:portrait:p1`);
      const p2 = pickPortraitFromSeed(`match-${i}:portrait:p2`);
      expect(isPortraitId(p1), `match-${i} p1`).toBe(true);
      expect(isPortraitId(p2), `match-${i} p2`).toBe(true);
      if (p1 !== p2) differ += 1;
    }
    // Dealt independently per seat (R636): the two draws disagree on at least some matches.
    expect(differ).toBeGreaterThan(0);
  });
});

describe("R637 — the one rate limit both ends enforce", () => {
  it("R637 fixes 1.5 s between a player's emotes and at most five in a rolling 20 s", () => {
    expect(EMOTE_COOLDOWN_MS).toBe(1500);
    expect(EMOTE_WINDOW_MS).toBe(20_000);
    expect(EMOTE_WINDOW_MAX).toBe(5);
  });

  it("R637 admits the first emote a player sends", () => {
    expect(emoteGate([], 1_000)).toEqual({ ok: true, sentAt: [] });
  });

  it("R637 rejects an emote inside the cooldown with the time left to wait", () => {
    expect(emoteGate([10_000], 10_500)).toEqual({
      ok: false,
      retryAfterMs: EMOTE_COOLDOWN_MS - 500,
      sentAt: [10_000],
    });
  });

  it("R637 admits an emote exactly at the cooldown boundary", () => {
    expect(emoteGate([10_000], 10_000 + EMOTE_COOLDOWN_MS)).toEqual({
      ok: true,
      sentAt: [10_000],
    });
  });

  it("R637 admits five spaced emotes inside a window and refuses the sixth", () => {
    let sentAt: readonly number[] = [];
    // Two seconds apart: past the cooldown each time, all five inside the rolling window.
    for (const now of [0, 2_000, 4_000, 6_000, 8_000]) {
      const gate = emoteGate(sentAt, now);
      expect(gate.ok, `emote at ${String(now)}`).toBe(true);
      sentAt = [...gate.sentAt, now];
    }
    expect(sentAt).toHaveLength(EMOTE_WINDOW_MAX);

    const sixth = emoteGate(sentAt, 10_000);
    expect(sixth.ok).toBe(false);
    if (!sixth.ok) {
      // The wait is the time until the window's oldest emote ages out.
      expect(sixth.retryAfterMs).toBe(EMOTE_WINDOW_MS - (10_000 - (sentAt[0] ?? 0)));
      expect(sixth.retryAfterMs).toBeGreaterThan(0);
    }
    // A refused emote does not join the rolling list.
    expect(sixth.sentAt).toEqual(sentAt);
  });

  it("R637 prunes emotes older than the window from the returned rolling list", () => {
    // At t = EMOTE_WINDOW_MS the t = 0 emote is exactly a window old: the boundary drops it.
    expect(emoteGate([0, 18_000], 20_000)).toEqual({ ok: true, sentAt: [18_000] });
  });

  it("R637 does not count emotes outside the window toward the cap", () => {
    // Five recorded emotes, but the oldest has aged out: four in the window, the send is admitted.
    const gate = emoteGate([1_000, 16_000, 18_000, 20_000, 22_000], 24_000);
    expect(gate).toEqual({ ok: true, sentAt: [16_000, 18_000, 20_000, 22_000] });
  });
});
