// Which practice games ask the server for a seed, and what the result dialog says of a claim
// (SPEC §14.8, R1303000).

import { describe, expect, it } from "vitest";

import { CATALOG_VERSION } from "@jackioh/cards";
import { LEAN_NEWEST } from "@jackioh/shared";

import { earningText, seedRequestOf } from "./earning.ts";
import { PRACTICE_PRESETS } from "./decks.ts";
import type { PracticeStartConfig } from "./protocol.ts";

function config(over: Partial<PracticeStartConfig> = {}): PracticeStartConfig {
  return { seed: "earning-test", difficulty: "medium", humanSeat: "p2", deck: { kind: "random" }, ...over };
}

describe("R1303000 which practice games ask for a server seed", () => {
  it("R1303000 a lesson, a sandbox, custom cards, Try it and aiBanned ask for no seed", () => {
    expect(seedRequestOf(config({ lesson: "basics" }))).toBeNull();
    expect(seedRequestOf(config({ sandbox: true }))).toBeNull();
    expect(
      seedRequestOf(
        config({
          deck: { kind: "saved", index: 1, cards: ["custom-0123"] },
          customCards: [{ id: "custom-0123" } as NonNullable<PracticeStartConfig["customCards"]>[number]],
        }),
      ),
    ).toBeNull();
    expect(seedRequestOf(config({ deck: { kind: "try", card: "custom-0123" } }))).toBeNull();
    expect(seedRequestOf(config({ aiBanned: ["core-001"] }))).toBeNull();
    expect(seedRequestOf(config({ deck: { kind: "preset", id: "no-such-preset" } }))).toBeNull();
  });

  it("a random deck asks with the set it leans on, R1373's switch read as newest", () => {
    const base = { difficulty: "medium", humanSeat: "p2", catalog: CATALOG_VERSION };
    expect(seedRequestOf(config())).toEqual({ ...base, deck: { kind: "random", lean: null } });
    expect(seedRequestOf(config({ deck: { kind: "random", leanSet: "Classic+" } }))).toEqual({
      ...base,
      deck: { kind: "random", lean: "Classic+" },
    });
    expect(seedRequestOf(config({ deck: { kind: "random", leanNewest: true } }))).toEqual({
      ...base,
      deck: { kind: "random", lean: LEAN_NEWEST },
    });
  });

  it("a preset and a saved deck ask with their cards", () => {
    const preset = PRACTICE_PRESETS[0];
    if (preset === undefined) throw new Error("practice has presets");
    expect(seedRequestOf(config({ humanSeat: "p1", deck: { kind: "preset", id: preset.id } }))).toEqual({
      difficulty: "medium",
      humanSeat: "p1",
      deck: { kind: "cards", cards: [...preset.cards] },
      catalog: CATALOG_VERSION,
    });
    const cards = ["core-001", "core-002"];
    expect(seedRequestOf(config({ deck: { kind: "saved", index: 2, cards, portrait: null } }))?.deck).toEqual({
      kind: "cards",
      cards,
    });
  });
});

describe("what the result dialog says of a claim", () => {
  it("earningText words each answer", () => {
    expect(earningText({ kind: "pending" })).toBe("Checking this game on the server…");
    expect(earningText({ kind: "paid", jlore: 10, xp: 42, jloreCapped: false })).toBe("+10 Jlore, +42 XP");
    expect(earningText({ kind: "refused", reason: "ai_mismatch" })).toBe(
      "This game earned nothing (the AI chose another move on the server).",
    );
    expect(earningText({ kind: "refused", reason: "a_reason_from_a_newer_server" })).toBe(
      "This game earned nothing (a_reason_from_a_newer_server).",
    );
    expect(earningText({ kind: "unreachable" })).toBe("The server could not check this game.");
  });

  it("R1483000 reports the daily cap without hiding the actual payout", () => {
    // The server's `jlore` is already what the cap left: a game that crossed it, and one after it.
    expect(earningText({ kind: "paid", jlore: 3, xp: 42, jloreCapped: true })).toBe(
      "+3 Jlore, +42 XP · Daily Jlore reached",
    );
    expect(earningText({ kind: "paid", jlore: 0, xp: 42, jloreCapped: true })).toBe(
      "+0 Jlore, +42 XP · Daily Jlore reached",
    );
    expect(earningText({ kind: "paid", jlore: 5, xp: 42, jloreCapped: false })).not.toContain("Daily Jlore reached");
  });

  it("every refusal the server names (`economy::practice::ClaimRefusal`) is worded", () => {
    const reasons = [
      "economy_off",
      "unknown_seed",
      "already_claimed",
      "expired",
      "does_not_fold",
      "ai_mismatch",
      "not_finished",
      "voided",
      "result_mismatch",
    ];
    for (const reason of reasons) {
      expect(earningText({ kind: "refused", reason }), reason).not.toBe(`This game earned nothing (${reason}).`);
    }
  });
});
