// #80 Zao Gao — SPEC §8.3, R11, R16, R21, R64, R215, R275, R276, R354, §5.2, §7, §9.3.
//
// BUILD M4-T4: "Discards 2 random cards; two Rush Tokens each with two distinct pool keywords;
// radiant the tokens are Radiant 6/6 Rush, Cleave, roll three keywords, and roll no keyword they
// have".
//
// Patch v0.1.1 (issue #27, R354): the discard is random, not the player's choice, so no prompt opens;
// the Radiant face's Radiant Rush Tokens each roll a third keyword; and the card is tagged CN.
// Radiant: "Discard 2 random cards. Summon 2 Radiant Rush Tokens, each with 3 random keywords." Each
// token is summoned on its Radiant face (§7: 6/6, Rush, Cleave) and then rolls its three keywords,
// which never repeat one it has (R21) — so neither Rush nor Cleave is ever one of them.

import { describe, expect, it } from "vitest";
import { RANDOM_KEYWORD_POOL } from "@jackioh/engine/config";
import { scenario, type Scenario } from "./_harness";
import { base, def, radiant } from "../src/scripts/080-zao-gao";

const ZAO_GAO = "core-080";
const RUSH_TOKEN = "core-t-rush";

const DISCARDABLE = ["core-001", "core-002", "core-003"] as const;
const LIBRARY = ["core-008", "core-008", "core-008", "core-008"] as const;

/** R21's pool as keyword KINDS; "Armor 1" is its one numbered entry (§6.1). */
const POOL_KINDS: ReadonlySet<string> = new Set<string>(
  RANDOM_KEYWORD_POOL.map((entry) => (entry === "Armor 1" ? "Armor" : entry)),
);

/** The live keyword list of the unit in a lane, read through `viewFor`'s §10.4 layers. */
function keywordKinds(s: Scenario, lane: number): string[] {
  const view = s.view("p1").you.units[lane - 1];
  if (view === undefined || view === null) throw new Error(`p1 has no unit in lane ${lane}`);
  return view.keywords.map((keyword) => keyword.kind);
}

/** §7: the Rush Token's printed keywords on each face. */
const BASE_PRINTED = ["Rush"] as const;
const RADIANT_PRINTED = ["Rush", "Cleave"] as const;

/**
 * R21 on one token: its printed keywords (§7) plus exactly `count` more, all distinct, all from the
 * pool, and none of them one it already printed.
 */
function expectPoolKeywords(s: Scenario, lane: number, count: number, printed: readonly string[]): void {
  const kinds = keywordKinds(s, lane);
  for (const kind of printed) expect(kinds).toContain(kind);
  // R21: "no repeats on one unit".
  expect(new Set(kinds).size).toBe(kinds.length);
  const granted = kinds.filter((kind) => !printed.includes(kind));
  expect(granted).toHaveLength(count);
  for (const kind of granted) expect(POOL_KINDS.has(kind)).toBe(true);

  // The `keywordGranted` events for this token are the roll itself, never a printed keyword.
  const tokenId = s.unit("p1", lane)!.id;
  const rolled = s.events.flatMap((event) =>
    event.type === "keywordGranted" && event.instanceId === tokenId ? [event.keyword.kind] : [],
  );
  expect(rolled).toHaveLength(count);
  for (const kind of printed) expect(rolled).not.toContain(kind);
}

function occupiedLanes(s: Scenario): number[] {
  return s.state.players.p1.units.flatMap((pile, index) => (pile === null ? [] : [index + 1]));
}

function board(radiantFace: boolean, hand: readonly string[], seed?: string): Scenario {
  return scenario({
    ...(seed === undefined ? {} : { seed }),
    p1: {
      hand: [{ def: ZAO_GAO, radiant: radiantFace }, ...hand],
      library: [...LIBRARY],
    },
    // R82: the opponent keeps something to do, so nothing auto-ends under the assertions.
    p2: { hand: ["core-005"], field: ["core-019"], library: [...LIBRARY] },
  });
}

/** The def ids of p1's graveyard, sorted. */
function graveyard(s: Scenario): string[] {
  return s.pile("p1", "graveyard").map((card) => card.defId).sort();
}

describe("#80 Zao Gao — card data (R354)", () => {
  it("R354 is tagged CN and prints the patch's random discard on both faces", () => {
    expect(def.tags).toEqual(["CN"]);
    expect(def.base.text).toBe("Discard 2 random cards. Summon 2 Rush Tokens, each with 2 random keywords.");
    expect(def.radiant.text).toBe(
      "Discard 2 random cards. Summon 2 Radiant Rush Tokens, each with 3 random keywords.",
    );
  });
});

describe("#80 Zao Gao — base", () => {
  it("R354 R16 the discard is random: no prompt opens, and two of the hand go to the graveyard", () => {
    const s = board(false, DISCARDABLE);

    s.play(ZAO_GAO);

    expect(s.state.pending).toBeNull();
    expect(s.events.some((event) => event.type === "promptOpened")).toBe(false);
    // Two different cards of the three, and Zao Gao itself (§10.5 step 7).
    const discarded = s.pile("p1", "graveyard").filter((card) => card.defId !== ZAO_GAO);
    expect(discarded).toHaveLength(2);
    expect(new Set(discarded.map((card) => card.id)).size).toBe(2);
    for (const card of discarded) expect(DISCARDABLE).toContain(card.defId);
    expect(s.pile("p1", "hand")).toHaveLength(1);
    s.expectEvents("cardPlayed", "discarded", "discarded", "summoned", "summoned");
  });

  it("R354 the picks come off the match rng: the same seed discards the same two cards, and a seed moves them", () => {
    const picked = (seed: string): string[] => {
      const s = board(false, DISCARDABLE, seed);
      s.play(ZAO_GAO);
      return s.pile("p1", "hand").map((card) => card.defId);
    };
    expect(picked("core-080-rng-a")).toEqual(picked("core-080-rng-a"));
    const kept = new Set(Array.from({ length: 12 }, (_, n) => picked(`core-080-rng-${n}`)[0]));
    // Over a dozen seeds, more than one card is the one left in hand: it is not a fixed choice.
    expect(kept.size).toBeGreaterThan(1);
  });

  it("R64 two Rush Tokens stand in the leftmost free zones", () => {
    const s = board(false, DISCARDABLE);

    s.play(ZAO_GAO);

    expect(occupiedLanes(s)).toEqual([1, 2]);
    expect(s.unit("p1", 1)?.defId).toBe(RUSH_TOKEN);
    expect(s.unit("p1", 2)?.defId).toBe(RUSH_TOKEN);
    expect(s.state.work).toHaveLength(0);
  });

  it("§7 the base face's tokens are base Rush Tokens: 3/3, not Radiant", () => {
    const s = board(false, DISCARDABLE);

    s.play(ZAO_GAO);

    for (const lane of [1, 2]) {
      const token = s.unit("p1", lane)!;
      expect(token.radiant).toBe(false);
      s.expectStats(token, { attack: 3, health: 3, maxHealth: 3 });
    }
  });

  it("R21 each Rush Token carries two distinct keywords from the pool, rolled independently", () => {
    const s = board(false, DISCARDABLE);

    s.play(ZAO_GAO);

    expectPoolKeywords(s, 1, 2, BASE_PRINTED);
    expectPoolKeywords(s, 2, 2, BASE_PRINTED);
  });

  it("§8.3 fewer than 2 in hand: what there is is discarded", () => {
    const s = board(false, [DISCARDABLE[0]]);

    s.play(ZAO_GAO);

    expect(graveyard(s)).toEqual([DISCARDABLE[0], ZAO_GAO].sort());
    expect(s.pile("p1", "hand")).toHaveLength(0);
    expect(occupiedLanes(s)).toEqual([1, 2]);
  });

  it("an empty hand discards nothing and the two tokens are summoned all the same", () => {
    const s = board(false, []);

    s.play(ZAO_GAO);

    expect(s.state.pending).toBeNull();
    expect(graveyard(s)).toEqual([ZAO_GAO]);
    expect(occupiedLanes(s)).toEqual([1, 2]);
  });

  it("R11 a unit-token card discarded from the hand ceases to exist instead of reaching the graveyard", () => {
    const s = board(false, [RUSH_TOKEN, RUSH_TOKEN]);

    s.play(ZAO_GAO);

    expect(s.pile("p1", "hand")).toHaveLength(0);
    expect(graveyard(s)).toEqual([ZAO_GAO]);
  });

  it("R64 a nearly full board gets fewer tokens and the extra summon fizzles", () => {
    const s = scenario({
      p1: {
        hand: [ZAO_GAO, ...DISCARDABLE],
        // Four of the five unit zones are taken, so only one token fits.
        field: ["core-019", "core-019", "core-019", "core-019"],
        library: [...LIBRARY],
      },
      p2: { hand: ["core-005"], field: ["core-019"], library: [...LIBRARY] },
    });

    s.play(ZAO_GAO);

    expect(occupiedLanes(s)).toEqual([1, 2, 3, 4, 5]);
    expect(s.unit("p1", 5)?.defId).toBe(RUSH_TOKEN);
    // The discard happened either way: it is not conditional on the summons.
    expect(s.pile("p1", "graveyard")).toHaveLength(3); // two discards plus Zao Gao itself
  });

  it("R64 a full board summons nothing and the discard still happens", () => {
    const s = scenario({
      p1: {
        hand: [ZAO_GAO, ...DISCARDABLE],
        field: ["core-019", "core-019", "core-019", "core-019", "core-019"],
        library: [...LIBRARY],
      },
      p2: { hand: ["core-005"], field: ["core-019"], library: [...LIBRARY] },
    });

    s.play(ZAO_GAO);

    expect(s.state.players.p1.units.filter((pile) => pile !== null)).toHaveLength(5);
    expect(s.unit("p1", 5)?.defId).not.toBe(RUSH_TOKEN);
    expect(s.pile("p1", "hand")).toHaveLength(1);
  });
});

describe("#80 Zao Gao — radiant", () => {
  it("R276 the radiant face is its own Script, not the base object", () => {
    expect(radiant).not.toBe(base);
  });

  it("R354 a Radiant Zao Gao discards two random cards and summons two Radiant Rush Tokens", () => {
    const s = board(true, DISCARDABLE);

    s.play(ZAO_GAO);

    expect(s.state.pending).toBeNull();
    expect(s.pile("p1", "graveyard").filter((card) => card.defId !== ZAO_GAO)).toHaveLength(2);
    expect(occupiedLanes(s)).toEqual([1, 2]);
    for (const lane of [1, 2]) {
      const token = s.unit("p1", lane)!;
      expect(token.defId).toBe(RUSH_TOKEN);
      // §7: the Radiant Rush Token is 6/6 with Rush and Cleave.
      expect(token.radiant).toBe(true);
      s.expectStats(token, { attack: 6, health: 6, maxHealth: 6 });
    }
  });

  it("R354 R21 a Radiant Zao Gao's tokens roll three pool keywords each, never Rush or Cleave", () => {
    const s = board(true, DISCARDABLE);

    s.play(ZAO_GAO);

    expectPoolKeywords(s, 1, 3, RADIANT_PRINTED);
    expectPoolKeywords(s, 2, 3, RADIANT_PRINTED);
  });

  it("R21 the roll reads the Radiant face on every seed: Cleave is never rolled onto a token that prints it", () => {
    // Each token's three draws come from the ten pool keywords a Radiant Rush Token lacks; were the
    // roll made before the flag set (or off the base face), Cleave would be offered and, over
    // enough seeds, rolled.
    const SEEDS = 40;
    for (let n = 0; n < SEEDS; n += 1) {
      const s = board(true, DISCARDABLE, `core-080-radiant-roll-${n}`);
      s.play(ZAO_GAO);
      expectPoolKeywords(s, 1, 3, RADIANT_PRINTED);
      expectPoolKeywords(s, 2, 3, RADIANT_PRINTED);
    }
  });

  it("R215 the spent Spell goes to the graveyard Radiant", () => {
    const s = board(true, DISCARDABLE);
    const self = s.card(ZAO_GAO);
    expect(self.radiant).toBe(true);

    s.play(ZAO_GAO);

    s.expectInZone(self, "graveyard");
    expect(s.card(self).radiant).toBe(true);
  });

  it("an empty hand on the radiant face discards nothing, and its tokens are Radiant", () => {
    const s = board(true, []);

    s.play(ZAO_GAO);

    expect(s.state.pending).toBeNull();
    expect(occupiedLanes(s)).toEqual([1, 2]);
    expect(s.unit("p1", 1)?.radiant).toBe(true);
    expect(s.unit("p1", 2)?.radiant).toBe(true);
  });

  it("R64 a nearly full board gets one Radiant token and the extra summon fizzles", () => {
    const s = scenario({
      p1: {
        hand: [{ def: ZAO_GAO, radiant: true }, ...DISCARDABLE],
        field: ["core-019", "core-019", "core-019", "core-019"],
        library: [...LIBRARY],
      },
      p2: { hand: ["core-005"], field: ["core-019"], library: [...LIBRARY] },
    });

    s.play(ZAO_GAO);

    expect(occupiedLanes(s)).toEqual([1, 2, 3, 4, 5]);
    const token = s.unit("p1", 5)!;
    expect(token.defId).toBe(RUSH_TOKEN);
    expect(token.radiant).toBe(true);
    expectPoolKeywords(s, 5, 3, RADIANT_PRINTED);
  });
});
