// T-glitch Glitch (SPEC §7, issue #170; R658–R664). BUILD M9's row: "hidden Spell token, cost 0, blank;
// only R658's roll makes one, after a … in the System play, at n/10000 per generated card; always
// playable on its owner's turn; resolves as one of four outcomes drawn by the match rng".
//
// The engine's own test (packages/engine/test/glitch.test.ts) proves the odds' arithmetic and each
// outcome on a fixture; this file proves them again on the real catalog, through `scenario()`.

import { describe, expect, it } from "vitest";
import { GLITCH_DEF_ID, GLITCH_ODDS_DENOMINATOR, GLITCH_OUTCOMES, query, seatPlayedBy } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { base, def, radiant } from "../../src/scripts/classic/t-glitch-glitch";
import { scenario, type Scenario } from "../_harness";

type Outcome = (typeof GLITCH_OUTCOMES)[number];

const OTHER_BOARD = [{ defId: "core-008", radiant: true }]; // #8 Mr. Vanilla, Radiant

/** A game at p1's turn with Glitch in hand and no mana to pay for anything else. */
function withGlitch(seed: string): Scenario {
  return scenario({
    seed,
    p1: { hand: [GLITCH_DEF_ID], field: ["core-001"], mana: 0 },
    p2: { field: ["core-002"] },
    glitchBoards: [OTHER_BOARD, []],
  });
}

function outcomeOf(events: readonly GameEvent[]): Outcome | undefined {
  const event = events.find((e): e is Extract<GameEvent, { type: "glitched" }> => e.type === "glitched");
  return event?.outcome;
}

/** The first seed whose Glitch draws `outcome`, played. */
function played(outcome: Outcome): Scenario {
  for (let n = 0; n < 200; n += 1) {
    const g = withGlitch(`t-glitch-${outcome}-${n}`).play(GLITCH_DEF_ID);
    if (outcomeOf(g.lastEvents) === outcome) return g;
  }
  throw new Error(`no seed draws ${outcome}`);
}

describe("T-glitch Glitch", () => {
  it("is a (0) Spell token, blank on both faces, whose one hook is the same on both", () => {
    expect([def.cost, def.type, def.token, def.rarity, def.base.text, def.radiant.text]).toEqual([0, "Spell", true, "Token", "", ""]);
    expect(base.cry).toBeTypeOf("function");
    expect(radiant).toBe(base);
  });

  it("R658 after a … in the System play, a generated card may be Glitch; at 10000 in 10000 every one is", () => {
    const g = scenario({ seed: "t-glitch-odds", p1: { hand: ["classic-025", "core-057"], mana: 4 } });
    g.play("classic-025");
    expect(g.state.systemPlays).toBe(1);
    g.state.systemPlays = GLITCH_ODDS_DENOMINATOR;
    g.play("core-057");
    expect(g.hand("p1").map((card) => card.defId)).toEqual([GLITCH_DEF_ID, GLITCH_DEF_ID, GLITCH_DEF_ID]);
  });

  it("R658 with no System play the same generation makes no Glitch and draws as it always did", () => {
    const g = scenario({ seed: "t-glitch-none", p1: { hand: ["core-057"], mana: 4 } });
    g.play("core-057");
    expect(g.state.systemPlays).toBeUndefined();
    expect(g.hand("p1").map((card) => card.defId)).not.toContain(GLITCH_DEF_ID);
  });

  it("R659 is in no pool, Classic+ #23 Dropshipping's every-token pool included", () => {
    expect(query({ withTokens: true }).map((card) => card.id)).not.toContain(GLITCH_DEF_ID);
    expect(query({ set: "Classic", token: true }).map((card) => card.id)).not.toContain(GLITCH_DEF_ID);
  });

  it("R660 is played for nothing on its owner's turn, a cost increase and no mana notwithstanding", () => {
    const g = withGlitch("t-glitch-free");
    g.card(GLITCH_DEF_ID).costMod = 3;
    g.play(GLITCH_DEF_ID);
    expect(outcomeOf(g.lastEvents)).toBeDefined();
  });

  it("R661 each of the four outcomes comes from the match rng", () => {
    const seen = new Set(GLITCH_OUTCOMES.map((outcome) => outcomeOf(played(outcome).events)));
    expect([...seen].sort()).toEqual([...GLITCH_OUTCOMES].sort());
  });

  it("R661 reset: the match starts again at its mulligans", () => {
    const g = played("reset");
    expect(g.state.phase).toBe("mulligan");
    expect(g.state.mulligan).toBeDefined();
    expect(g.unit("p1", 1)).toBeNull();
  });

  it("R662 swap: each account now plays the other seat", () => {
    const g = played("swap");
    expect(seatPlayedBy(g.state, "p1")).toBe("p2");
    expect(g.unit("p1", 1)?.defId).toBe("core-001");
  });

  it("R663 boards: both fields become the frozen other games' boards", () => {
    const g = played("boards");
    expect([g.unit("p1", 1)?.defId, g.unit("p1", 1)?.radiant]).toEqual(["core-008", true]);
    expect(g.unit("p2", 1)).toBeNull();
  });

  it("R664 void: the game ends with no winner, reason voided", () => {
    expect(played("void").state.result).toEqual({ winner: "draw", reason: "voided" });
  });
});
