// The practice sandbox's card search and the cards its tools can name (practice/sandbox.ts, #923).

import { describe, expect, it } from "vitest";

import { registeredCatalog } from "@jackioh/engine";
import { setShips } from "@jackioh/shared";

import { baseView, card, emptySide, faceDownBackrow, faceUpBackrow, fullBoardView, unit } from "../test/fixtures.ts";
import { SANDBOX_RESULTS_MAX } from "./config.ts";
import { sandboxAction, searchCards, spawnableDefs, visibleCards } from "./sandbox.ts";

describe("R923001 the sandbox's card search", () => {
  const defs = registeredCatalog();
  const pool = spawnableDefs(defs);

  it("R923001 offers every card and token of a set that ships, in catalog order, and nothing else", () => {
    const shipped = Object.values(defs).filter((def) => setShips(def.set));
    expect(pool.map((def) => def.id)).toEqual(shipped.map((def) => def.id));
    expect(pool.some((def) => def.token === true), "tokens are spawnable").toBe(true);
    expect(pool.every((def) => setShips(def.set))).toBe(true);
  });

  it("R923001 an empty search finds the first SANDBOX_RESULTS_MAX cards; a limit caps any search", () => {
    expect(searchCards(pool, "")).toEqual(pool.slice(0, SANDBOX_RESULTS_MAX));
    expect(searchCards(pool, "   ", 3)).toEqual(pool.slice(0, 3));
  });

  it("R923001 a word matches part of the name, either text, the id, the set, the type or the rarity, ignoring case", () => {
    const bigot = pool.find((def) => def.name === "Bigot");
    expect(bigot, "the catalog has Bigot").toBeDefined();
    if (bigot === undefined) return;
    expect(searchCards(pool, "BIGO", pool.length)).toContain(bigot);
    expect(searchCards(pool, bigot.id, pool.length)).toEqual([bigot]);

    const spells = searchCards(pool, "spell", pool.length);
    expect(spells.length).toBeGreaterThan(0);
    for (const def of searchCards(pool, "legendary", pool.length)) {
      expect(`${def.rarity} ${def.printedRarity ?? ""} ${def.name} ${def.base.text} ${def.radiant.text}`.toLowerCase()).toContain("legendary");
    }
    const tokens = searchCards(pool, "token", pool.length);
    expect(tokens.some((def) => def.token === true)).toBe(true);
  });

  it("R923001 a number matches the cost, and every word must match", () => {
    const threes = searchCards(pool, "3", pool.length);
    expect(threes.length).toBeGreaterThan(0);
    for (const def of threes) {
      expect(typeof def.cost === "number" ? def.cost : typeof def.cost === "object" ? def.cost.base : null).toBe(3);
    }
    const unitThrees = searchCards(pool, "3 unit", pool.length);
    expect(unitThrees.length).toBeGreaterThan(0);
    // Every 3-cost Unit, and only 3-cost cards ("unit" also matches a text that names one).
    expect(unitThrees.every((def) => threes.includes(def))).toBe(true);
    expect(threes.filter((def) => def.type === "Unit").every((def) => unitThrees.includes(def))).toBe(true);
    expect(searchCards(pool, "no-card-is-called-this", pool.length)).toEqual([]);
  });
});

describe("R923003 the cards the sandbox's tools can name", () => {
  it("R923003 both fields' face-up cards, the human's hand and both graveyards; never a face-down enemy card or the AI's hand", () => {
    const view = fullBoardView();
    const named = visibleCards(view);
    const ids = new Set(named.map((entry) => entry.instanceId));

    for (const slot of [...view.you.units, ...view.opponent.units]) {
      if (slot !== null) expect(ids.has(slot.instanceId)).toBe(true);
    }
    const hand = Array.isArray(view.you.hand) ? view.you.hand : [];
    expect(hand.length).toBeGreaterThan(0);
    for (const held of hand) expect(named).toContainEqual({ instanceId: held.instanceId, defId: held.defId, side: "you", where: "hand" });
    for (const dead of [...view.you.graveyard, ...view.opponent.graveyard]) expect(ids.has(dead.instanceId)).toBe(true);
    expect(named.filter((entry) => entry.side === "opponent" && entry.where === "hand")).toEqual([]);
    // Exile is neither destroyed nor removed again.
    for (const gone of view.you.exile) expect(ids.has(gone.instanceId)).toBe(false);
  });

  it("R923003 a face-up backrow card is named, a face-down marker is not", () => {
    const shown = faceUpBackrow("p2", { defId: "core-020", instanceId: "shown-1" });
    const view = baseView({
      you: emptySide("p1", { units: [unit("p1"), null, null, null, null], hand: [card()] }),
      opponent: emptySide("p2", { hand: { count: 3 }, backrow: [shown, faceDownBackrow, null, null, null] }),
    });
    const named = visibleCards(view);
    expect(named.filter((entry) => entry.side === "opponent")).toEqual([
      { instanceId: "shown-1", defId: "core-020", side: "opponent", where: "field" },
    ]);
    expect(named.filter((entry) => entry.side === "you").map((entry) => entry.where)).toEqual(["field", "hand"]);
  });

  it("R923000 a tool is sent as the engine's `sandbox` action", () => {
    expect(sandboxAction({ kind: "draw", player: "p1", count: 2 })).toEqual({ type: "sandbox", op: { kind: "draw", player: "p1", count: 2 } });
  });
});
