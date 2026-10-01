// R502: a cast on draw is read off the order of the redacted events (castOnDraw.ts), and the
// planner's memory keeps what it needs across entries: the last events, and the plays still resolving
// (memory.ts).

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { castOnDrawAt } from "./castOnDraw.ts";
import { FX_MEMORY_RECENT, FX_MEMORY_RESOLVING } from "./constants.ts";
import { createFxMemory } from "./memory.ts";

const drawn = (player: "p1" | "p2", instanceId: string, defId = "core-021"): GameEvent => ({ type: "drawn", player, instanceId, defId });
const played = (player: "p1" | "p2", instanceId: string, defId = "core-021", extra: Partial<GameEvent> = {}): GameEvent =>
  ({ type: "cardPlayed", player, instanceId, defId, costPaid: 0, ...extra }) as GameEvent;
const resolved = (player: "p1" | "p2", instanceId: string, defId = "core-021"): GameEvent => ({
  type: "cardResolved",
  player,
  instanceId,
  defId,
  permanent: false,
  costPaid: 0,
  radiant: false,
});
const added = (player: "p1" | "p2", instanceId: string): GameEvent => ({ type: "addedToHand", player, instanceId, defId: "core-005" });
const MANA: GameEvent = { type: "manaChanged", player: "p2", current: 2, max: 2 };

describe("R502 which cardPlayed is a cast on draw", () => {
  it("R502 a cardPlayed right after its own drawn is one; on the viewer's seat and the other alike", () => {
    expect(castOnDrawAt([MANA, drawn("p2", "c21"), played("p2", "c21")], 2)).toBe(true);
    expect(castOnDrawAt([drawn("p1", "c9", "core-027"), played("p1", "c9", "core-027")], 1)).toBe(true);
  });

  it("R502 R97 a hidden draw and a hidden play of the same seat still read as one, naming nothing", () => {
    expect(castOnDrawAt([drawn("p2", "hidden", "hidden"), played("p2", "hidden", "hidden")], 1)).toBe(true);
    expect(castOnDrawAt([drawn("p2", "hidden", "hidden"), played("p2", "c30", "core-030")], 1)).toBe(true);
  });

  it("R502 R227 a card set face down as it is cast is matched by its formerId", () => {
    expect(castOnDrawAt([drawn("p1", "c4"), played("p1", "c40", "core-060", { formerId: "c4" })], 1)).toBe(true);
  });

  it("R502 an announce of the same card between the two (B5 E1) does not break it", () => {
    const announced: GameEvent = { type: "cardAnnounced", player: "p2", instanceId: "c21", defId: "core-021", cardType: "Spell", costPaid: 0, targets: [] };
    expect(castOnDrawAt([drawn("p2", "c21"), announced, played("p2", "c21")], 2)).toBe(true);
  });

  it("R502 the prompts the cast asks its caster (#21 Hinder's discard) do not break it, another seat's do", () => {
    const asked: GameEvent = { type: "promptOpened", player: "p2", choiceId: "q1", kind: "hand" };
    const answered: GameEvent = { type: "promptAnswered", player: "p2", choiceId: "q1" };
    expect(castOnDrawAt([drawn("p2", "c1"), asked, answered, played("p2", "c1")], 3)).toBe(true);
    const theirs: GameEvent = { type: "promptOpened", player: "p1", choiceId: "q2", kind: "target" };
    expect(castOnDrawAt([drawn("p2", "c1"), theirs, played("p2", "c1")], 2)).toBe(false);
  });

  it("R502 anything else is a play: a card from hand, a draw that reached the hand first, another seat's draw, another card", () => {
    expect(castOnDrawAt([MANA, played("p2", "c21")], 1)).toBe(false);
    expect(castOnDrawAt([drawn("p2", "c21"), added("p2", "c21"), played("p2", "c21")], 2)).toBe(false);
    expect(castOnDrawAt([drawn("p1", "c21"), played("p2", "c21")], 1)).toBe(false);
    expect(castOnDrawAt([drawn("p2", "c20"), played("p2", "c21")], 1)).toBe(false);
    expect(castOnDrawAt([played("p2", "c21")], 0)).toBe(false);
    expect(castOnDrawAt([drawn("p2", "c21")], 0)).toBe(false);
    expect(castOnDrawAt([drawn("p2", "c21"), played("p2", "c21")], 5)).toBe(false);
  });
});

describe("R502 the planner's memory across entries", () => {
  it("R502 remembers each cardPlayed of a cast on draw even though its drawn came in an earlier entry", () => {
    const memory = createFxMemory();
    const draw = drawn("p2", "c21");
    const cast = played("p2", "c21");
    const hand = played("p2", "c22", "core-005");
    memory.remember([MANA]);
    memory.remember([draw]);
    memory.remember([cast]);
    expect(memory.castOnDraw(cast)).toBe(true);
    memory.remember([hand]);
    expect(memory.castOnDraw(hand)).toBe(false);
    // The object itself is what is remembered: an equal copy was never seen.
    expect(memory.castOnDraw({ ...cast } as GameEvent)).toBe(false);
  });

  it("R502 keeps the plays still resolving, innermost last, each with its step count, and closes them on cardResolved or countered", () => {
    const memory = createFxMemory();
    memory.remember([drawn("p2", "c27", "core-027"), played("p2", "c27", "core-027")]);
    expect(memory.resolving()).toEqual({ player: "p2", instanceId: "c27", defId: "core-027", castOnDraw: true, step: 0 });
    memory.remember([{ type: "healthLost", player: "p2", amount: 5 }]);
    expect(memory.resolving()?.step).toBe(1);
    memory.remember([played("p2", "c95", "core-095")]);
    expect(memory.resolving()?.defId).toBe("core-095");
    memory.remember([resolved("p2", "c95", "core-095")]);
    expect(memory.resolving()?.defId).toBe("core-027");
    memory.remember([{ type: "countered", player: "p2", instanceId: "c27", defId: "core-027", byInstanceId: null, to: "graveyard" }]);
    expect(memory.resolving()).toBeUndefined();
  });

  it("R202 a hidden play is kept as a hidden play and closed by a hidden resolution of its seat", () => {
    const memory = createFxMemory();
    memory.remember([played("p2", "hidden", "hidden")]);
    expect(memory.resolving()).toMatchObject({ player: "p2", instanceId: "hidden", defId: "hidden" });
    expect(memory.casterOf("hidden")).toBeUndefined();
    memory.remember([resolved("p1", "hidden", "hidden")]);
    expect(memory.resolving()).toBeDefined();
    memory.remember([resolved("p2", "hidden", "hidden")]);
    expect(memory.resolving()).toBeUndefined();
  });

  it("R502 keeps at most FX_MEMORY_RECENT events and FX_MEMORY_RESOLVING plays, and clear forgets both", () => {
    const memory = createFxMemory();
    memory.remember([drawn("p2", "c21")]);
    memory.remember(Array.from({ length: FX_MEMORY_RECENT }, () => MANA));
    const late = played("p2", "c21");
    memory.remember([late]);
    expect(memory.castOnDraw(late), "the drawn has fallen out of the window").toBe(false);

    for (let i = 0; i < FX_MEMORY_RESOLVING + 3; i += 1) memory.remember([played("p1", `c${String(i)}`, "core-005")]);
    for (let i = 0; i < FX_MEMORY_RESOLVING; i += 1) memory.remember([resolved("p1", `c${String(FX_MEMORY_RESOLVING + 2 - i)}`, "core-005")]);
    expect(memory.resolving()).toBeUndefined();

    const draw = drawn("p1", "c50");
    memory.remember([draw, played("p1", "c50")]);
    memory.clear();
    const after = played("p1", "c50");
    memory.remember([after]);
    expect(memory.castOnDraw(after)).toBe(false);
    expect(memory.resolving()?.castOnDraw).toBe(false);
  });
});
