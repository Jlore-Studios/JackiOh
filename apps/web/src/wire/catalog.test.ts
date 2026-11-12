// The catalog vocabulary's runtime half (`./catalog.ts`, SPEC §5): which sets ship, the
// newest one, the lean picks, and the pure text helpers. One `it` per rule, reading the
// engine's own set table through `./engineConfig.ts` (CLAUDE.md rule 9).

import { describe, expect, it } from "vitest";

import { SHIPPED_SETS } from "./engineConfig.ts";
import {
  LEAN_NEWEST,
  fillParams,
  hasKeyword,
  isLeanPick,
  keywordKey,
  leanPicks,
  leanSetOf,
  newestShippedSet,
  opponentOf,
  paramPlaceholders,
  setShips,
} from "./catalog.ts";

describe("opponentOf", () => {
  it("flips the seat", () => {
    expect(opponentOf("p1")).toBe("p2");
    expect(opponentOf("p2")).toBe("p1");
  });
});

describe("setShips", () => {
  it("ships every set the engine lists and no other", () => {
    for (const set of SHIPPED_SETS) expect(setShips(set)).toBe(true);
    expect(setShips("No Such Set" as never)).toBe(false);
  });
});

describe("newestShippedSet", () => {
  it("is the last entry of the engine's set table", () => {
    expect(newestShippedSet()).toBe(SHIPPED_SETS[SHIPPED_SETS.length - 1]);
  });
});

describe("lean picks", () => {
  it("offers newest first, then every set that ships", () => {
    expect(leanPicks()).toEqual([LEAN_NEWEST, ...SHIPPED_SETS]);
  });

  it("takes newest and every shipped set, nothing else", () => {
    expect(isLeanPick(LEAN_NEWEST)).toBe(true);
    for (const set of SHIPPED_SETS) expect(isLeanPick(set)).toBe(true);
    expect(isLeanPick("No Such Set")).toBe(false);
    expect(isLeanPick(null)).toBe(false);
    expect(isLeanPick(undefined)).toBe(false);
  });

  it("leans newest on the newest set that ships", () => {
    expect(leanSetOf(LEAN_NEWEST)).toBe(newestShippedSet());
    const shipped = SHIPPED_SETS[0]!;
    expect(leanSetOf(shipped)).toBe(shipped);
  });
});

describe("keywords", () => {
  it("keys an Armor keyword with its number and a plain one bare", () => {
    expect(keywordKey({ kind: "Taunt" })).toBe("Taunt");
    expect(keywordKey({ kind: "Armor", n: 2 })).toBe("Armor 2");
  });

  it("finds a keyword by kind and nothing else", () => {
    expect(hasKeyword([{ kind: "Taunt" }], "Taunt")).toBe(true);
    expect(hasKeyword([{ kind: "Taunt" }], "Rush")).toBe(false);
    expect(hasKeyword([], "Taunt")).toBe(false);
  });
});

describe("params", () => {
  it("lists every placeholder in order", () => {
    expect(paramPlaceholders("Deal {damage} damage. Draw {draw|card|cards}.")).toEqual([
      { key: "damage" },
      { key: "draw", one: "card", many: "cards" },
    ]);
    expect(paramPlaceholders("No tunable here.")).toEqual([]);
  });

  it("fills a number and the agreeing wording from its printed values", () => {
    const def = {
      params: [
        { key: "damage", base: 3, radiant: 5, better: "up" as const },
        { key: "draw", base: 1, radiant: 2, better: "up" as const },
      ],
      base: { keywords: [], text: "Deal {damage} damage. Draw {draw|card|cards}." },
      radiant: { keywords: [], text: "Deal {damage} damage. Draw {draw|card|cards}." },
    };
    expect(fillParams(def, "base")).toBe("Deal 3 damage. Draw 1 card.");
    expect(fillParams(def, "radiant")).toBe("Deal 5 damage. Draw 2 cards.");
    expect(fillParams(def, "base", { damage: 2, draw: 3 })).toBe("Deal 2 damage. Draw 3 cards.");
  });

  it("leaves a text with no params alone and an unknown key as written", () => {
    const plain = {
      base: { keywords: [], text: "Deal 3 damage." },
      radiant: { keywords: [], text: "Deal 3 damage." },
    };
    expect(fillParams(plain, "base")).toBe("Deal 3 damage.");
    const def = {
      params: [{ key: "damage", base: 3, radiant: 5, better: "up" as const }],
      base: { keywords: [], text: "Deal {unknown} damage." },
      radiant: { keywords: [], text: "Deal {unknown} damage." },
    };
    expect(fillParams(def, "base")).toBe("Deal {unknown} damage.");
  });
});
