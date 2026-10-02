// #83 Transmogulate (SPEC §8.4 row 83; R11, R23, R35, R365).
//
// BUILD M4-T4's must-pass row: "Zone counts preserved; board cards replaced by same-type
// Legendaries in place; pool is exactly #52, #85, #87, #92, #93, #95, and a Field Trap becomes
// Unlicensed Experimentation (R35); the hand is replaced too (R365); radiant gives radiant cards" —
// and since patch v0.2.0 (R380) the pool is every set's non-token Legendaries but #83.
//
// The board fixture covers every permanent type at once: a Unit, an Immutable Unit (R23), a Field
// Spell, a Trap and a Field Trap. The pool is read from the catalog and checked to hold the six ids
// R35 names, so if `query`, the rarities or the exclusion of #83 ever drift, these tests say so.
//
// One count to keep in mind: Transmogulate is a Spell, so it reaches its owner's graveyard AFTER
// its own script has run (§10.5). The graveyard therefore ends one card larger than it started —
// the replacement for what was there, plus Transmogulate itself, which was in `resolving` while the
// replacements happened and so was never replaced by one of them.

import { describe, expect, it } from "vitest";
import { scenario } from "./_harness";
import { catalog } from "../src/query";

const TRANSMOGULATE = "core-083";

/**
 * R35's pool: "every non-token Legendary except #83" — of every set since patch v0.2.0 (R380). In
 * Core that is the six R35 names, #52, #85, #87, #92, #93, #95.
 */
const POOL = catalog.pool(TRANSMOGULATE, { rarity: "Legendary" }).map((def) => def.id);
const CORE_SIX = ["core-052", "core-085", "core-087", "core-092", "core-093", "core-095"];
/** The Legendary Units in the pool, every set's (Core #52 Silly Silas and #92 Felinor Fiender among them). */
const LEGENDARY_UNITS = catalog.pool(TRANSMOGULATE, { rarity: "Legendary", type: "Unit" }).map((def) => def.id);
/** The Legendary Field Spells: Core #93 Combo-Index, Classic #4, #7, Classic+ #78. */
const LEGENDARY_FIELD_SPELLS = catalog.pool(TRANSMOGULATE, { rarity: "Legendary", type: "Field Spell" }).map((def) => def.id);
/** The only Legendary Trap of any set: #85 Unlicensed Experimentation — and "Field Trap counts as Trap". */
const LEGENDARY_TRAP = "core-085";

/** Board fixtures: #11 Tempo Timmy (Unit), a Radiant #19 Midrange Menace (Immutable Unit), #73
 *  (Field Spell), #41 Sheepish (Trap), #18 Bread and Butter (Field Trap). */
const IMMUTABLE = "core-019";
function board(radiantFace = false) {
  const s = scenario({
    seed: "transmogulate",
    p1: {
      hand: [TRANSMOGULATE, "core-056"],
      field: ["core-011", { def: IMMUTABLE, radiant: true }],
      backrow: [
        { def: "core-073", lane: 1 },
        { def: "core-041", lane: 2 },
        { def: "core-018", lane: 3 },
      ],
      library: ["core-020", "core-025"],
      graveyard: ["core-056"],
      exile: ["core-011"],
    },
    p2: { field: ["core-011"], library: ["core-020"] },
  });
  // Stands in for a missing `{ def, radiant }` form on SideSetup.hand (harness request).
  if (radiantFace) s.card(TRANSMOGULATE).radiant = true;
  return s;
}

describe("#83 Transmogulate — base", () => {
  it("R35 board cards are replaced in place by a Legendary of the same type", () => {
    const s = board().play(TRANSMOGULATE);

    // A Unit becomes a Legendary Unit, in its own lane.
    expect(LEGENDARY_UNITS).toContain(s.unit("p1", 1)?.defId);
    // A Field Spell becomes the Legendary Field Spell; a Trap becomes the Legendary Trap.
    expect(LEGENDARY_FIELD_SPELLS).toContain(s.backrow("p1", 1)?.defId);
    expect(s.backrow("p1", 2)?.defId).toBe(LEGENDARY_TRAP);
    s.expectEvents("cardPlayed", "transformed");
  });

  it("R35 'Field Trap counts as Trap': a Field Trap becomes Unlicensed Experimentation", () => {
    const s = board().play(TRANSMOGULATE);

    expect(s.backrow("p1", 3)?.defId).toBe(LEGENDARY_TRAP);
  });

  it("R23 an Immutable board card stays, since this is a Transform", () => {
    const s = board();
    const immutable = s.card(IMMUTABLE).id;
    s.play(TRANSMOGULATE);

    expect(s.unit("p1", 2)?.id).toBe(immutable);
    expect(s.unit("p1", 2)?.defId).toBe(IMMUTABLE);
  });

  it("R35 R365 same counts per zone, in hand, library, graveyard and exile", () => {
    const s = board();
    const before = {
      hand: s.hand("p1").length - 1,
      library: s.pile("p1", "library").length,
      graveyard: s.pile("p1", "graveyard").length,
      exile: s.pile("p1", "exile").length,
    };
    s.play(TRANSMOGULATE);

    expect(s.hand("p1")).toHaveLength(before.hand);
    expect(s.pile("p1", "library")).toHaveLength(before.library);
    expect(s.pile("p1", "exile")).toHaveLength(before.exile);
    // Plus Transmogulate itself, which was resolving while the replacements happened.
    expect(s.pile("p1", "graveyard")).toHaveLength(before.graveyard + 1);
    expect(s.pile("p1", "graveyard").map((card) => card.defId)).toContain(TRANSMOGULATE);
  });

  it("R35 R380 the pool is every set's non-token Legendaries — Core's #52, #85, #87, #92, #93, #95 among them — never Transmogulate itself", () => {
    for (const id of CORE_SIX) expect(POOL).toContain(id);
    expect(POOL).not.toContain(TRANSMOGULATE);
    expect(POOL.some((id) => id.startsWith("classic"))).toBe(true);
    const s = board();
    const spell = s.card(TRANSMOGULATE).id;
    s.play(TRANSMOGULATE);

    const replaced = [
      ...s.hand("p1"),
      ...s.pile("p1", "library"),
      ...s.pile("p1", "exile"),
      ...s.pile("p1", "graveyard").filter((card) => card.id !== spell),
      ...[1, 2, 3, 4, 5].flatMap((lane) => {
        const unit = s.unit("p1", lane);
        const back = s.backrow("p1", lane);
        return [...(unit === null ? [] : [unit]), ...(back === null ? [] : [back])];
      }),
    ];

    for (const card of replaced) {
      // The Immutable Menace is the one card R23 left alone.
      if (card.defId === IMMUTABLE) continue;
      expect(POOL, `${card.defId} is not in R35's pool`).toContain(card.defId);
    }
  });

  it("R35 replaced cards cease to exist: they are in no pile at all", () => {
    const s = board();
    const oldUnit = s.card("core-011").id;
    const oldLibrary = s.pile("p1", "library").map((card) => card.id);
    const oldGraveyard = s.pile("p1", "graveyard").map((card) => card.id);
    s.play(TRANSMOGULATE);

    s.expectInZone(oldUnit, "gone");
    for (const id of [...oldLibrary, ...oldGraveyard]) s.expectInZone(id, "gone");
  });

  it("R365 your hand is replaced too: the card in it ceases to exist and a pool card takes its place", () => {
    const s = board();
    const spare = s.hand("p1").find((card) => card.defId === "core-056")?.id ?? "";
    s.play(TRANSMOGULATE);

    s.expectInZone(spare, "gone");
    expect(s.hand("p1")).toHaveLength(1);
    expect(POOL).toContain(s.hand("p1")[0]?.defId);
    expect(s.hand("p1")[0]?.radiant).toBe(false);
  });

  it("R365 the opponent never learns what the hand became (R177)", () => {
    const s = board();
    s.play(TRANSMOGULATE);

    expect(s.view("p2").opponent.hand).toEqual({ count: 1 });
    const seen = JSON.stringify(s.view("p2"));
    expect(seen).not.toContain(s.hand("p1")[0]?.id ?? "no-card");
  });

  it("§8 'your' library and board: the opponent keeps everything", () => {
    const s = board().play(TRANSMOGULATE);

    expect(s.unit("p2", 1)?.defId).toBe("core-011");
    expect(s.pile("p2", "library").map((card) => card.defId)).toEqual(["core-020"]);
  });

  it("base gives non-Radiant cards", () => {
    const s = board().play(TRANSMOGULATE);

    expect(s.pile("p1", "library").map((card) => card.radiant)).toEqual([false, false]);
    expect(s.unit("p1", 1)?.radiant).toBe(false);
    expect(s.backrow("p1", 1)?.radiant).toBe(false);
  });

  it("R13 a card dormant under a Stack is not on your board: only the top of the pile is replaced", () => {
    // #92 Felinor Fiender (Stack) on top of #43 Big Felinor in lane 1; the Big Felinor is dormant.
    const s = scenario({
      seed: "transmogulate-stack",
      p1: { hand: [TRANSMOGULATE], field: ["core-043", { def: "core-092", stack: true }], library: ["core-020"] },
      p2: { field: ["core-011"], library: ["core-020"] },
    });
    const buried = s.card("core-043");
    const top = s.card("core-092");
    expect(s.unit("p1", 1)?.id, "the Fiender is on top of lane 1").toBe(top.id);

    s.play(TRANSMOGULATE);

    // The acting top card was replaced in place by a Legendary Unit.
    const transformed = s.events.filter((event) => event.type === "transformed");
    expect(transformed.some((event) => event.instanceId === top.id)).toBe(true);
    expect(LEGENDARY_UNITS).toContain(s.unit("p1", 1)?.defId);
    // The dormant card is untouched: still on the field beneath it, still a Big Felinor.
    expect(transformed.some((event) => event.instanceId === buried.id)).toBe(false);
    s.expectInZone(buried, "field");
    expect(s.card(buried).defId).toBe("core-043");
  });
});

describe("#83 Transmogulate — radiant", () => {
  it("'Random Radiant Legendaries': every replacement is Radiant, in every zone", () => {
    const s = board(true);
    const spell = s.card(TRANSMOGULATE).id;
    s.play(TRANSMOGULATE);

    expect(s.unit("p1", 1)?.radiant).toBe(true);
    expect(s.backrow("p1", 1)?.radiant).toBe(true);
    expect(s.backrow("p1", 2)?.radiant).toBe(true);
    expect(s.backrow("p1", 3)?.radiant).toBe(true);
    expect(s.pile("p1", "library").map((card) => card.radiant)).toEqual([true, true]);
    expect(s.pile("p1", "exile").map((card) => card.radiant)).toEqual([true]);
    expect(s.hand("p1").map((card) => card.radiant)).toEqual([true]);
    expect(
      s.pile("p1", "graveyard").filter((card) => card.id !== spell).map((card) => card.radiant),
    ).toEqual([true]);
  });

  it("the radiant face keeps R35's pool and its same-type board rule", () => {
    const s = board(true).play(TRANSMOGULATE);

    expect(LEGENDARY_UNITS).toContain(s.unit("p1", 1)?.defId);
    expect(LEGENDARY_FIELD_SPELLS).toContain(s.backrow("p1", 1)?.defId);
    expect(s.backrow("p1", 3)?.defId).toBe(LEGENDARY_TRAP);
    for (const card of s.pile("p1", "library")) expect(POOL).toContain(card.defId);
  });

  it("R23 an Immutable board card still stays on the radiant face", () => {
    const s = board(true).play(TRANSMOGULATE);

    const immutable = s.unit("p1", 2);
    expect(immutable?.defId).toBe(IMMUTABLE);
    // A refused Transform changes nothing about the card: it is the Menace it was.
    expect(immutable?.radiant).toBe(true);
  });
});

describe("#83 Transmogulate — R312 the owner's library list", () => {
  it("R312 every library replacement is a card its owner was never shown, so the list counts them unknown", () => {
    const s = board();
    // Before: p1's own two cards, by printed cost (R310): Pointmaster (2), then the 7/7 (4).
    expect(s.view("p1").you.ownLibrary).toEqual({
      cards: [
        { defId: "core-020", radiant: false, count: 1 },
        { defId: "core-025", radiant: false, count: 1 },
      ],
      unknown: 0,
    });

    s.play(TRANSMOGULATE);

    // The same count, none of it named: the Legendaries it rolled stay unread, even by p1.
    expect(s.view("p1").you.ownLibrary).toEqual({ cards: [], unknown: 2 });
    const mine = JSON.stringify(s.view("p1"));
    for (const card of s.pile("p1", "library")) expect(mine).not.toContain(`"${card.id}"`);
    // p2's library is untouched and still fully known to p2.
    expect(s.view("p2").you.ownLibrary).toEqual({ cards: [{ defId: "core-020", radiant: false, count: 1 }], unknown: 0 });
  });
});
