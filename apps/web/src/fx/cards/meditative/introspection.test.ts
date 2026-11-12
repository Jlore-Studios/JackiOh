// R3405001's play-order helpers (fx/cards/meditative/introspection.ts): where a Field Spell's beat
// stands, and whether an event is the first of its type its play or hook has seen. Neither helper is
// named by any test today; the entrance itself is pinned in the signatures suites.

import { describe, expect, it } from "vitest";

import type { GameEvent } from "@jackioh/shared";

import { testid } from "../../../game/contract.ts";
import { baseView, emptySide, faceUpBackrow, unit } from "../../../test/fixtures.ts";
import type { FxMemory, FxPlay } from "../../types.ts";
import type { CardFxPlan } from "../recipe.ts";
import type { AnimationEntry } from "../../../game/animations.ts";
import { fieldTile, firstOfPlay } from "./introspection.ts";

function play(over: Partial<FxPlay> = {}): FxPlay {
  return { player: "p1", instanceId: "f1", defId: "meditative-007", castOnDraw: false, step: 1, seen: {}, ...over };
}

function plan(events: readonly GameEvent[], resolving?: FxPlay): CardFxPlan {
  const memory = { resolving: () => resolving } as FxMemory;
  return {
    entry: { events } as AnimationEntry,
    view: baseView(),
    env: { intensity: 1, card: () => undefined, memory },
    D: 1000,
  };
}

function healed(id: string): GameEvent {
  return { type: "healed", targetId: id, amount: 2 };
}

describe("R3405001 the first of its type the play has seen", () => {
  it("is the entry's own first event of the type, when the play has seen nothing", () => {
    const first = healed("u1");
    const second = healed("u2");
    const p = plan([first, second]);
    expect(firstOfPlay(first, p)).toBe(true);
    expect(firstOfPlay(second, p)).toBe(false);
  });

  it("is false for everything once the play saw the type before this entry", () => {
    const first = healed("u1");
    const p = plan([first], play({ seen: { healed: 2 } }));
    expect(firstOfPlay(first, p)).toBe(false);
  });

  it("counts each type on its own: the seen counts include the entry's own events (R202)", () => {
    const heal = healed("u1");
    const damage: GameEvent = { type: "damage", sourceId: null, targetId: "u1", amount: 1, combat: true };
    const p = plan([heal, damage], play({ seen: { healed: 1, damage: 1 } }));
    expect(firstOfPlay(heal, p)).toBe(true);
    expect(firstOfPlay(damage, p)).toBe(true);
  });
});

describe("R3405001 where a Field Spell's beat stands", () => {
  it("is its card while the board shows it face up", () => {
    const view = baseView({ you: emptySide("p1", { units: [unit("p1", { instanceId: "u1" }), null, null, null, null] }) });
    expect(fieldTile(view, play({ instanceId: "u1" }))).toEqual({ kind: "testid", testid: testid.card("u1") });
  });

  it("is else the first face-up backrow card of its definition on its side", () => {
    const back = faceUpBackrow("p1", { instanceId: "b9", defId: "meditative-007" });
    const view = baseView({ you: emptySide("p1", { backrow: [back, null, null, null, null] }) });
    expect(fieldTile(view, play({ instanceId: "gone" }))).toEqual({
      kind: "testid",
      testid: testid.zone("you", "backrow", 1),
    });
  });

  it("is else its hero: a hidden card, or a backrow of other definitions, names nothing", () => {
    const view = baseView({
      opponent: emptySide("p2", { backrow: [faceUpBackrow("p2", { defId: "core-020" }), null, null, null, null] }),
    });
    expect(fieldTile(view, play({ player: "p2", instanceId: "gone" }))).toEqual({
      kind: "testid",
      testid: testid.hero("opponent"),
    });
  });
});
