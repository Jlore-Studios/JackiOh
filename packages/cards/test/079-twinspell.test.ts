// #79 Twinspell — SPEC §8.3, R30, §2.2, §6.2 Echo, §10.5 step 6.
//
// BUILD M4-T4: "Next spell echoes once (radiant twice); consumed to GY on use (R30); survives
// cleanup".
//
// The echo count is read through The Coin (§7), whose "Gain 1 mana this turn." makes each resolution
// a number: one extra resolution is +2 instead of +1, two extra is +3. (It was read through #78
// /fullsend's "gain 4 mana" until patch v0.1.1 made that a Refresh, which stops at max.)
// The "fresh prompts per repeat" half of §10.5 step 6 is read through #80 Zao Gao, the one Core card whose resolution opens a prompt
// (§10.6), which a declared play-time pick (#26) would not reopen.

import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "./_harness";

const TWINSPELL = "core-079";
const COIN = "core-t-coin";
const ZAO_GAO = "core-080";

const LIBRARY = ["core-008", "core-008", "core-008", "core-008"] as const;
const DISCARDABLE = ["core-001", "core-002", "core-003", "core-004"] as const;

/** Harness gap (reported): no `mods()` accessor, so the test reads `state` — B1.7 covers `src` only. */
function echoRiders(s: Scenario): unknown[] {
  return s.state.players.p1.mods.filter((mod) => mod.kind === "echoNextSpell");
}

/** Mana 4, so Twinspell (2) and The Coin (0) both fit on one turn. */
function withCoin(radiant: boolean): Scenario {
  return scenario({
    p1: { hand: [{ def: TWINSPELL, radiant }, COIN], mana: 4, library: [...LIBRARY] },
    p2: { hand: ["core-005"], field: ["core-019"], library: [...LIBRARY] },
  });
}

/** Mana 4: Twinspell (2) and Zao Gao (2), with four cards left over to discard. */
function withZaoGao(radiant: boolean): Scenario {
  return scenario({
    p1: { hand: [{ def: TWINSPELL, radiant }, ZAO_GAO, ...DISCARDABLE], library: [...LIBRARY] },
    p2: { hand: ["core-005"], field: ["core-019"], library: [...LIBRARY] },
  });
}

describe("#79 Twinspell — base", () => {
  it("R30 installs an until-used Echo +1 rider naming its own instance", () => {
    const s = withCoin(false);
    const twin = s.card(TWINSPELL);

    s.play(TWINSPELL);

    // A Field Spell with a Cry is ordinary (#73): it enters the backrow and the Cry fires there.
    s.expectInZone(twin, "field");
    expect(echoRiders(s)).toHaveLength(1);
    expect(s.state.players.p1.mods.find((mod) => mod.kind === "echoNextSpell")).toMatchObject({
      kind: "echoNextSpell",
      amount: 1,
      // §2.2: not turn-scoped, so it waits for a spell rather than for cleanup.
      expiry: { until: "used" },
      // R30: the engine needs to know which card to bury when the rider is consumed.
      sourceId: twin.id,
    });
  });

  it("§2.2 the pending Echo survives end-of-turn cleanup", () => {
    const s = withCoin(false);
    s.play(TWINSPELL);

    s.endTurn();

    // §2.2 says so in as many words: "Twinspell's pending Echo is not turn-scoped and survives
    // cleanup." BUILD M4-T2's acceptance row says the same of a "this turn" discount's opposite.
    expect(echoRiders(s)).toHaveLength(1);
    s.expectInZone(TWINSPELL, "field");
  });

  it("§10.5 step 6 the next spell resolves one extra time", () => {
    const s = withCoin(false);
    s.play(TWINSPELL); // 4 − 2 = 2

    s.play(COIN); // 0, then 1 mana per resolution

    // Two resolutions of "gain 1 mana": 2 + 2 = 4 rather than 2 + 1 = 3.
    s.expectMana("p1", 4);
  });

  it("R30 Twinspell is consumed to the graveyard when it applies, and the rider is gone", () => {
    const s = withCoin(false);
    const twin = s.card(TWINSPELL);
    s.play(TWINSPELL);

    s.play(COIN);

    s.expectInZone(twin, "graveyard");
    expect(echoRiders(s)).toHaveLength(0);
  });

  it("R30 only the NEXT spell echoes: a second spell resolves once", () => {
    const s = scenario({
      p1: { hand: [TWINSPELL, COIN, COIN], mana: 4, library: [...LIBRARY] },
      p2: { hand: ["core-005"], field: ["core-019"], library: [...LIBRARY] },
    });
    s.play(TWINSPELL); // 4 − 2 = 2
    const [first, second] = s.hand("p1").filter((card) => card.defId === COIN);
    if (first === undefined || second === undefined) throw new Error("setup: two Coins in hand");
    s.play(first); // +1 per resolution, twice: 4
    s.expectMana("p1", 4);

    s.play(second); // +1 once: 5

    s.expectMana("p1", 5);
    // The same fact without the arithmetic: the rider was spent on the first spell, so this one
    // carries no Echo and has exactly one resolution to show for itself (§10.5 step 6).
    expect(s.lastEvents.filter((event) => event.type === "cardResolved")).toHaveLength(1);
    expect(echoRiders(s)).toHaveLength(0);
  });

  it("§10.5 step 6 an echoed prompting spell reopens its prompt on the repeat", () => {
    const s = withZaoGao(false);
    s.play(TWINSPELL);

    s.play(ZAO_GAO);

    // First resolution's hand prompt (§10.6: Zao Gao's chosen discard).
    expect(s.state.pending?.kind).toBe("hand");
    s.answer([DISCARDABLE[0], DISCARDABLE[1]]);
    // The repeat asks again, with fresh options: a prompt, not a play-time declaration.
    expect(s.state.pending?.kind).toBe("hand");
    s.answer([DISCARDABLE[2], DISCARDABLE[3]]);

    expect(s.state.pending).toBeNull();
    // Two resolutions × two Rush Tokens.
    expect(s.state.players.p1.units.filter((pile) => pile !== null)).toHaveLength(4);
    expect(s.pile("p1", "graveyard").map((card) => card.defId)).toContain(DISCARDABLE[3]);
  });
});

describe("#79 Twinspell — radiant", () => {
  it("R30 radiant installs Echo +2", () => {
    const s = withCoin(true);
    const twin = s.card(TWINSPELL);

    s.play(TWINSPELL);

    expect(s.state.players.p1.mods.find((mod) => mod.kind === "echoNextSpell")).toMatchObject({
      kind: "echoNextSpell",
      amount: 2,
      expiry: { until: "used" },
      sourceId: twin.id,
    });
  });

  it("§10.5 step 6 radiant makes the next spell resolve twice more", () => {
    const s = withCoin(true);
    s.play(TWINSPELL); // 4 − 2 = 2

    s.play(COIN); // three resolutions of +1

    s.expectMana("p1", 5);
  });

  it("R30 radiant is consumed to the graveyard on use and survives cleanup until then", () => {
    const s = withCoin(true);
    const twin = s.card(TWINSPELL);
    s.play(TWINSPELL);

    s.endTurn();
    expect(echoRiders(s)).toHaveLength(1);
    s.expectInZone(twin, "field");

    // Back around to p1's turn, where the waiting rider is spent.
    s.endTurn();
    s.play(COIN);

    s.expectInZone(twin, "graveyard");
    expect(echoRiders(s)).toHaveLength(0);
  });

  it("§10.5 step 6 radiant reopens a prompting spell's prompt on both repeats", () => {
    const s = scenario({
      p1: {
        hand: [
          { def: TWINSPELL, radiant: true },
          ZAO_GAO,
          ...DISCARDABLE,
          "core-005",
          "core-006",
        ],
        library: [...LIBRARY],
      },
      p2: { hand: ["core-007"], field: ["core-019"], library: [...LIBRARY] },
    });
    s.play(TWINSPELL);

    s.play(ZAO_GAO);

    expect(s.state.pending?.kind).toBe("hand");
    s.answer([DISCARDABLE[0], DISCARDABLE[1]]);
    expect(s.state.pending?.kind).toBe("hand");
    s.answer([DISCARDABLE[2], DISCARDABLE[3]]);
    expect(s.state.pending?.kind).toBe("hand");
    s.answer(["core-005", "core-006"]);

    expect(s.state.pending).toBeNull();
    // R64: three resolutions want six tokens and the unit row holds five.
    expect(s.state.players.p1.units.filter((pile) => pile !== null)).toHaveLength(5);
  });
});
