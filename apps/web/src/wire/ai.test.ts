// The practice AI's client door (`./ai.ts`, SPEC §9.9): the Rust AI through WebAssembly.
// Budgets and the shadow ban are the module's own numbers (CLAUDE.md rule 9), read once it
// loads; `decide`, `aiToAct` and `buildAiDeck` only forward. The games that prove the AI
// are `practice/driver.test.ts`; here the door's own values are pinned.

import { afterEach, describe, expect, it, vi } from "vitest";

import { aiConstants, type AiDeckRequest, type DecideRequest, type DecideResult } from "../wasm/index.ts";
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
  type Decision,
} from "./ai.ts";
import { beginGame, createGame } from "./engine.ts";
import type { GameState } from "./generated/GameState.ts";
import type { PlayerId } from "./index.ts";

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

// What the door hands the module, against a stand-in for it (`vi.doMock`, then `./ai.ts` imported
// afresh): each entry point's arguments as they cross, and the caller's stream drawn forward to the
// cursor the module stopped at, so `rng.cursor` reads as the module left it (the practice save keeps
// it, R668).
describe("what the door hands the module", () => {
  const WASM = "../wasm/index.ts";
  const state = { seed: "door" } as unknown as GameState;

  /** `./ai.ts` imported afresh over a module that answers with `bindings`. */
  async function door(bindings: Record<string, unknown>): Promise<typeof import("./ai.ts")> {
    const constants = aiConstants();
    vi.resetModules();
    vi.doMock(WASM, () => ({ aiConstants: () => constants, whenWasmLoaded: (run: () => void) => run(), ...bindings }));
    return await import("./ai.ts");
  }

  afterEach(() => {
    vi.doUnmock(WASM);
    vi.resetModules();
  });

  it("lethalHint passes its deadline on, and 0, no clock, without one", async () => {
    const lethalHintBinding = vi.fn((_state: GameState, _seat: PlayerId, _deadlineMs: number) => true);
    const ai = await door({ lethalHint: lethalHintBinding });
    expect(ai.lethalHint(state, "p1", 12_345)).toBe(true);
    expect(ai.lethalHint(state, "p2")).toBe(true);
    expect(lethalHintBinding.mock.calls).toStrictEqual([
      [state, "p1", 12_345],
      [state, "p2", 0],
    ]);
  });

  it("decide hands the module the caller's stream, and a budget only when the caller gives one", async () => {
    const decision = { reason: "search" } as unknown as Decision;
    const decideBinding = vi.fn(
      (_state: GameState, _seat: PlayerId, _request: DecideRequest, _deadlineMs: number): DecideResult => ({ decision, rngCursor: 7 }),
    );
    const ai = await door({ decide: decideBinding });
    const rng = createRng("door-ai", 2);
    expect(ai.decide(state, "p2", { rng, budget: AI_BUDGET, deadlineMs: 99 })).toBe(decision);
    expect(rng.cursor).toBe(7);
    expect(ai.decide(state, "p2", { rng })).toBe(decision);
    expect(decideBinding.mock.calls).toStrictEqual([
      [state, "p2", { rngSeed: "door-ai", rngCursor: 2, budget: AI_BUDGET }, 99],
      [state, "p2", { rngSeed: "door-ai", rngCursor: 7 }, 0],
    ]);
  });

  it("buildAiDeck hands the module the stream, the size and the options, and draws the stream to where it stopped", async () => {
    const buildAiDeckBinding = vi.fn((_request: AiDeckRequest) => ({ deck: ["c001"], rngCursor: 5 }));
    const ai = await door({ buildAiDeck: buildAiDeckBinding });
    const rng = createRng("door-deck");
    expect(ai.buildAiDeck(rng, 1, { banned: [] })).toEqual(["c001"]);
    expect(rng.cursor).toBe(5);
    expect(buildAiDeckBinding.mock.calls).toStrictEqual([[{ rngSeed: "door-deck", rngCursor: 0, size: 1, banned: [] }]]);
  });
});
