// The emote pool and the dealt hand on the client (R1340–R1343). The deal is the engine's
// (`crates/engine/src/wire/emotes.rs`), reached over WASM; what this file proves is that the
// client's hand-kept copy of the pool and the hand's shape (`src/wire/emotes.ts`) agree with the
// Rust ones, that the WASM deal is the server's, and how a frame's hand is read.

import { describe, expect, it } from "vitest";

import type { EmoteId } from "@jackioh/shared";
import { EMOJI_EMOTE_IDS, EMOTE_HAND_SIZE, EMOTE_HAND_VOICE, EMOTE_IDS, VOICE_EMOTE_IDS, isVoiceEmote } from "@jackioh/shared";
import { dealEmoteHand } from "@jackioh/engine";

import type { EmoteId as GeneratedEmoteId } from "../wire/generated/EmoteId.ts";
import { dealEmoteHands } from "./deal.ts";
import { DEFAULT_EMOTE_HAND, handFor, isEmoteHand } from "./hand.ts";

// R1340: the hand-kept TypeScript union and the one ts-rs writes from Rust's `EmoteId` are the same
// set of strings, or this file does not typecheck.
type Same<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;
const POOL_TYPES_AGREE: Same<EmoteId, GeneratedEmoteId> = true;

const SEEDS = Array.from({ length: 300 }, (_, i) => `seed-${i.toString(16)}`);

describe("R1340 the pool the client spells", () => {
  it("R1340 twenty-four ids: the five voice lines, then the nineteen emoji, the same as the engine's", () => {
    expect(POOL_TYPES_AGREE).toBe(true);
    expect(EMOTE_IDS).toHaveLength(24);
    expect(VOICE_EMOTE_IDS).toHaveLength(5);
    expect(EMOJI_EMOTE_IDS).toHaveLength(19);
    expect(EMOTE_IDS).toEqual([...VOICE_EMOTE_IDS, ...EMOJI_EMOTE_IDS]);
    // Every id the engine deals over a sweep of seeds is one the client knows, and it deals them all.
    const dealt = new Set<string>();
    for (const seed of SEEDS) {
      for (const id of [...dealEmoteHand(seed, "p1"), ...dealEmoteHand(seed, "p2")]) dealt.add(id);
    }
    expect([...dealt].sort()).toEqual([...EMOTE_IDS].sort());
  });
});

describe("R1341 the hand the engine deals, over WASM", () => {
  it("R1341 is the server's: the same eight the Rust deal pins for seed-actor's p1", () => {
    expect(dealEmoteHand("seed-actor", "p1")).toEqual(["greetings", "thanks", "threaten", "laugh", "wahWah", "wave", "thumbsUp", "party"]);
  });

  it("R1341 holds EMOTE_HAND_VOICE voice lines and the rest emoji, EMOTE_HAND_SIZE distinct, in the pool's order", () => {
    expect(EMOTE_HAND_SIZE).toBe(8);
    expect(EMOTE_HAND_VOICE).toBe(3);
    for (const seed of SEEDS) {
      for (const seat of ["p1", "p2"] as const) {
        const hand = dealEmoteHand(seed, seat);
        expect(hand, `${seed} ${seat}`).toHaveLength(EMOTE_HAND_SIZE);
        expect(new Set(hand).size).toBe(EMOTE_HAND_SIZE);
        expect(hand.filter((id) => isVoiceEmote(id))).toHaveLength(EMOTE_HAND_VOICE);
        const positions = hand.map((id) => EMOTE_IDS.indexOf(id));
        expect(positions).toEqual([...positions].sort((a, b) => a - b));
        expect(isEmoteHand(hand)).toBe(true);
      }
    }
  });

  it("R1341 a game on this device deals both seats from its seed, the same on every call (a reload, a resumed game)", () => {
    const hands = dealEmoteHands("practice-7f3a");
    expect(hands).toEqual({ p1: dealEmoteHand("practice-7f3a", "p1"), p2: dealEmoteHand("practice-7f3a", "p2") });
    expect(dealEmoteHands("practice-7f3a")).toEqual(hands);
    expect(hands.p1).not.toEqual(hands.p2);
  });

  it("R1341 an unknown seat is the engine's refusal, not a hand", () => {
    expect(() => dealEmoteHand("seed", "p3" as "p1")).toThrow();
  });
});

describe("R1342 R1343 reading a hand off the wire, and the default before one arrives", () => {
  it("R1342 a hand is exactly EMOTE_HAND_SIZE distinct pool ids", () => {
    const hand = dealEmoteHand("seed-wire", "p2");
    expect(isEmoteHand(hand)).toBe(true);
    expect(isEmoteHand(hand.slice(1))).toBe(false);
    expect(isEmoteHand([...hand, "sob"])).toBe(false);
    expect(isEmoteHand([...hand.slice(1), hand[1]])).toBe(false);
    expect(isEmoteHand([...hand.slice(1), "poke"])).toBe(false);
    expect(isEmoteHand(null)).toBe(false);
    expect(isEmoteHand("greetings")).toBe(false);
  });

  it("R1343 the default hand has a dealt hand's shape and only R643's ten ids", () => {
    expect(isEmoteHand([...DEFAULT_EMOTE_HAND])).toBe(true);
    expect(DEFAULT_EMOTE_HAND.filter((id) => isVoiceEmote(id))).toHaveLength(EMOTE_HAND_VOICE);
    expect(DEFAULT_EMOTE_HAND.every((id) => EMOTE_IDS.indexOf(id) < 10)).toBe(true);
    expect(handFor(null, "p1")).toEqual(DEFAULT_EMOTE_HAND);
    expect(handFor({ p2: dealEmoteHand("s", "p2") }, "p1")).toEqual(DEFAULT_EMOTE_HAND);
    expect(handFor({ p2: dealEmoteHand("s", "p2") }, "p2")).toEqual(dealEmoteHand("s", "p2"));
  });
});
