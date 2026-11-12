// The practice AI's client door (`./ai.ts`, SPEC §9.9): the Rust AI through WebAssembly.
// Budgets and the shadow ban are the module's own numbers (CLAUDE.md rule 9), read once it
// loads; `decide`, `aiToAct` and `buildAiDeck` only forward. The games that prove the AI
// are `practice/driver.test.ts`; here the door's own values are pinned.

import { describe, expect, it } from "vitest";

import { DECK_SIZE } from "./engineConfig.ts";
import { createRng } from "./rng.ts";
import {
  AI_BUDGET,
  AI_DECK,
  AI_GATE_BUDGET,
  FIXTURE_BANNED_IDS,
  SHADOW_BAN_IDS,
  aiToAct,
  buildAiDeck,
  lethalHint,
} from "./ai.ts";
import { beginGame, createGame } from "./engine.ts";

describe("budgets and bans", () => {
  it("reads the practice budget from the module", () => {
    expect(AI_BUDGET).toMatchObject({ nodes: 1200, lethalNodes: 300 });
    expect(AI_GATE_BUDGET).toEqual(AI_BUDGET);
  });

  it("reads the shadow bans sorted, the fixture pool frozen", () => {
    expect([...SHADOW_BAN_IDS].sort()).toEqual(SHADOW_BAN_IDS);
    expect([...FIXTURE_BANNED_IDS].sort()).toEqual(FIXTURE_BANNED_IDS);
  });

  it("reads the deck builder's numbers from the module", () => {
    expect(AI_DECK.leanMinShare).toBe(0.5);
  });
});

describe("buildAiDeck", () => {
  it("deals the asked size, deterministically from the seed", () => {
    const a = buildAiDeck(createRng("ai-deck-seed"), DECK_SIZE);
    const b = buildAiDeck(createRng("ai-deck-seed"), DECK_SIZE);
    expect(a).toEqual(b);
    expect(a).toHaveLength(DECK_SIZE);
  });

  it("deals a different deck from another seed", () => {
    const a = buildAiDeck(createRng("ai-deck-a"), DECK_SIZE);
    const b = buildAiDeck(createRng("ai-deck-b"), DECK_SIZE);
    expect(a).not.toEqual(b);
  });
});

describe("aiToAct and lethalHint", () => {
  it("answers whether a seat owes the AI an action, without throwing", () => {
    const d1 = buildAiDeck(createRng("ai-game-1"), DECK_SIZE);
    const d2 = buildAiDeck(createRng("ai-game-2"), DECK_SIZE);
    const state = beginGame(createGame({ seed: "ai-act-seed", decks: [d1, d2] })).state;
    expect(typeof aiToAct(state, "p1")).toBe("boolean");
    expect(typeof lethalHint(state, "p1")).toBe("boolean");
  });
});
