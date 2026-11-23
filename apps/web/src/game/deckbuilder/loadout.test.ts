// The collection and the pool's shelf. Nothing here asserts a rule: the validator owns those
// (workshop.test.ts checks the builder hands decks to it faithfully).

import { describe, expect, it } from "vitest";

import { TOKEN_ID, fixtureCardId, fixtureCatalog, fixtureCollection } from "./fixtures.ts";
import { collectionFrom, poolFrom } from "./loadout.ts";

describe("the pool", () => {
  it("offers what the profile owns, in catalog order, and never a Token (R251)", () => {
    const pool = poolFrom(fixtureCatalog(), fixtureCollection());
    expect(pool).not.toContain(TOKEN_ID);
    expect(pool[0]).toBe(fixtureCardId(1));
    expect(pool[1]).toBe(fixtureCardId(2));
  });

  it("leaves out a card the profile owns none of", () => {
    const collection = { ...fixtureCollection() };
    delete collection[fixtureCardId(3)];
    expect(poolFrom(fixtureCatalog(), collection)).not.toContain(fixtureCardId(3));
  });

  it("falls back to the whole non-token catalog when the collection is unreadable", () => {
    const pool = poolFrom(fixtureCatalog(), null);
    expect(pool).toContain(fixtureCardId(3));
    expect(pool).not.toContain(TOKEN_ID);
  });

  it("collectionFrom projects the ledger's entries to quantities", () => {
    expect(collectionFrom([{ cardId: "core-001", quantity: 1 }])).toEqual({ "core-001": 1 });
  });

  it("#4015 collectionFrom drops prototype-pollution card ids", () => {
    const owned = collectionFrom([
      { cardId: "core-001", quantity: 1 },
      { cardId: "__proto__", quantity: 2 },
      { cardId: "constructor", quantity: 2 },
    ]);
    expect(owned).toEqual({ "core-001": 1 });
    expect(Object.hasOwn(owned, "constructor")).toBe(false);
    expect(owned.constructor).toBe(Object);
    expect(({} as Record<string, unknown>)["polluted"]).toBeUndefined();
  });

  it("#4015 collectionFrom keeps an object quantity off the result's prototype", () => {
    const owned = collectionFrom([{ cardId: "__proto__", quantity: { polluted: true } as unknown as number }]);
    expect(Object.getPrototypeOf(owned)).toBe(Object.prototype);
    expect((owned as Record<string, unknown>)["polluted"]).toBeUndefined();
    expect(({} as Record<string, unknown>)["polluted"]).toBeUndefined();
  });
});
