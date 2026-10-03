// C+ #44 Simplicity Audit and #45 Complexity Audit's sweep (`subsystems/audit.ts`, SPEC §8.7 rows 44
// and 45, E36), through test-only definitions carrying their own `loc`. The real cards are proved in
// packages/cards (test/classic-plus/044-simplicity-audit.test.ts, 045-complexity-audit.test.ts).

import type { CardDef } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { newInstance, type GameState } from "../src/state";
import { auditTargets, linesOfCode } from "../src/subsystems/audit";
import { fuse } from "../src/subsystems/fuse";
import { placeOnField } from "../src/zones";
import { newGame, put, sinkFor, slot } from "./fixtures/harness";

function unit(id: string, loc: number | undefined, extra: Partial<CardDef> = {}): CardDef {
  return {
    id,
    index: id,
    name: id,
    set: "Core",
    type: "Unit",
    tags: [],
    rarity: "Common",
    token: false,
    cost: 1,
    ...(loc === undefined ? {} : { loc }),
    base: { attack: 1, health: 1, keywords: [], text: id },
    radiant: { attack: 2, health: 2, keywords: [], text: id },
    ...extra,
  };
}

const small = unit("au-3", 3);
const even = unit("au-10", 10);
const big = unit("au-20", 20);
const none = unit("au-none", undefined);
const stacker = unit("au-stack", 40, {
  base: { attack: 1, health: 1, keywords: [{ kind: "Stack" }], text: "stack" },
  radiant: { attack: 2, health: 2, keywords: [{ kind: "Stack" }], text: "stack" },
});
const trap = unit("au-trap", 4, { type: "Trap", base: { keywords: [], text: "trap" }, radiant: { keywords: [], text: "trap" } });

function board(): GameState {
  const state = newGame("audit");
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries([small, even, big, none, stacker, trap].map((d) => [d.id, d])) });
  state.active = "p1";
  return state;
}

const ids = (cards: { defId: string }[]): string[] => cards.map((card) => card.defId);

describe("the Audits' sweep (E36)", () => {
  it("E36 fewer: every permanent below the Audit's loc on both sides, the active side first; an equal one stays", () => {
    const state = board();
    put(state, small.id, slot("p2", "units", 1));
    put(state, even.id, slot("p1", "units", 1));
    put(state, small.id, slot("p1", "units", 2));
    put(state, big.id, slot("p2", "units", 2));
    expect(ids(auditTargets(state, { controller: "p2", active: "p1", loc: 10, more: false, enemyOnly: false }))).toEqual([small.id, small.id]);
    const targets = auditTargets(state, { controller: "p2", active: "p1", loc: 10, more: false, enemyOnly: false });
    expect(targets.map((card) => card.controller)).toEqual(["p1", "p2"]);
  });

  it("E36 more: every permanent above it; an equal one stays", () => {
    const state = board();
    put(state, small.id, slot("p1", "units", 1));
    put(state, even.id, slot("p2", "units", 1));
    put(state, big.id, slot("p2", "units", 2));
    expect(ids(auditTargets(state, { controller: "p1", active: "p1", loc: 10, more: true, enemyOnly: false }))).toEqual([big.id]);
  });

  it("E36 only the opponent's: the controller's own permanents are left out", () => {
    const state = board();
    put(state, small.id, slot("p1", "units", 1));
    put(state, small.id, slot("p2", "units", 1));
    const targets = auditTargets(state, { controller: "p1", active: "p1", loc: 10, more: false, enemyOnly: true });
    expect(targets.map((card) => card.controller)).toEqual(["p2"]);
  });

  it("§3.2 face-down backrow cards count; a card dormant under a Stack is not on the field", () => {
    const state = board();
    put(state, trap.id, slot("p2", "backrow", 1));
    put(state, small.id, slot("p2", "units", 1));
    const top = newInstance(state, stacker.id, "p2", { z: "hand", player: "p2" });
    expect(placeOnField(state, top, slot("p2", "units", 1), { stack: true })).toBe(true);
    expect(ids(auditTargets(state, { controller: "p1", active: "p1", loc: 10, more: false, enemyOnly: false }))).toEqual([trap.id]);
    expect(ids(auditTargets(state, { controller: "p1", active: "p1", loc: 10, more: true, enemyOnly: false }))).toEqual([stacker.id]);
  });

  it("E36 a card with no loc reads 0; a fused card's loc is its ingredients' sum (R77)", () => {
    const state = board();
    expect(linesOfCode(state, none.id)).toBe(0);
    const kept = put(state, small.id, slot("p1", "units", 1));
    const other = newInstance(state, big.id, "p1", { z: "gone", player: "p1" });
    const fused = fuse(sinkFor(state), { ingredients: [kept, other], target: kept });
    expect(linesOfCode(state, fused?.defId ?? "")).toBe(23);
    expect(ids(auditTargets(state, { controller: "p2", active: "p1", loc: 23, more: false, enemyOnly: false }))).toEqual([]);
    expect(auditTargets(state, { controller: "p2", active: "p1", loc: 24, more: false, enemyOnly: false })).toHaveLength(1);
  });
});
