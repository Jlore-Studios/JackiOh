// C #65 Ace in the Hole — SPEC §8.6 row 65, BUILD M9 Classic row C 65: "Face-down (R33); at the end of
// its controller's turn, in the end-of-turn trap window (R62), flip a coin from the match rng: heads
// fires it (to the graveyard) and Recruits the first permanent from the top of your deck (none, or its
// row full, → nothing); tails does nothing and it stays set; the opponent's turns flip nothing;
// radiant: tails Recruits 1 without firing, the trap staying face-down and never named in the
// opponent's view, and heads fires it and Recruits 3; a recruited trap lands face-down (R33); its tuned
// number (recruits) reads through `param()` (R386)".
//
// The coin is the match rng's next draw as its controller's turn ends (the trap's end-of-turn hook,
// which R62 runs before the window the trap fires in), so each case picks a seed whose next coin is the
// face it needs (`withCoin`) and then checks the trap did what that coin says — which is what "one
// seeded coin each time" means. The deck holds Core cards: Mr. Vanilla and Gary the Gambler (Units),
// Sheepish (a Trap), Lunar Eclipse (a Spell, which a Recruit passes by).

import { createRng, reduce, stepParam, type GameState } from "@jackioh/engine";
import type { Action } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/065-ace-in-the-hole";

const ACE = "classic-065";
const VANILLA = "core-008";
const GARY = "core-004";
const SHEEPISH = "core-041";
const LUNAR = "core-035";
const FILLER = "core-005";

type Coin = "heads" | "tails";

/** The coin the match rng gives next: the one the trap flips as the active player's turn ends. */
function nextCoin(state: GameState): Coin {
  return createRng(state.seed, state.rngCursor).coin() ? "heads" : "tails";
}

/** A board with an Ace in the Hole set in p1's backrow, on the first seed whose next coin is `coin`. */
function withCoin(coin: Coin, opts: { radiant?: boolean; p1?: Partial<SideSetup>; active?: "p1" | "p2" } = {}): Scenario {
  for (let attempt = 0; attempt < 64; attempt += 1) {
    const s = scenario({
      seed: `ace-in-the-hole-${coin}-${attempt}`,
      p1: {
        hand: [FILLER],
        backrow: [{ def: ACE, faceUp: false, ...(opts.radiant === true ? { radiant: true } : {}) }],
        library: [LUNAR, VANILLA, GARY, VANILLA],
        ...opts.p1,
      },
      p2: { hand: [FILLER], library: [FILLER, FILLER] },
      ...(opts.active === undefined ? {} : { active: opts.active }),
    });
    if (nextCoin(s.state) === coin) return s;
  }
  throw new Error(`no seed gave ${coin}`);
}

function fired(s: Scenario): number {
  return s.events.filter((event) => event.type === "trapFired").length;
}

function units(s: Scenario): (string | null)[] {
  return [1, 2, 3, 4, 5].map((lane) => s.unit("p1", lane)?.defId ?? null);
}

describe("C #65 Ace in the Hole", () => {
  it("is a Trap that flips at the end of a turn and fires in the window; its number is `recruits`", () => {
    expect(def.id).toBe(ACE);
    expect(def.type).toBe("Trap");
    expect(def.params).toEqual([{ key: "recruits", base: 1, radiant: 3, better: "up", step: 1, min: 1 }]);
    for (const face of [base, radiant]) {
      expect(face.endOfTurn).toBeTypeOf("function");
      expect(face.triggers?.map((trigger) => trigger.on)).toEqual([["turnEnded"]]);
    }
  });

  describe("base", () => {
    it("R33 it is set face-down and never named in the opponent's view", () => {
      const s = scenario({ p1: { hand: [ACE, FILLER] }, p2: { hand: [FILLER] } });
      s.play(ACE);
      expect(s.card(ACE).faceUp).not.toBe(true);
      expect(s.view("p2").opponent.backrow[0]).toMatchObject({ faceDown: true });
      expect(JSON.stringify(s.view("p2"))).not.toContain(ACE);
    });

    it("R62 heads: at the end of your turn it fires — face-up, to the graveyard — and Recruits the first permanent from the top of your deck", () => {
      const s = withCoin("heads");
      const ace = s.card(ACE);
      const cursor = s.state.rngCursor;
      s.endTurn();
      expect(s.state.rngCursor).toBeGreaterThan(cursor);
      s.expectEvents("turnEnded", "trapFired", "summoned");
      expect(units(s)).toEqual([VANILLA, null, null, null, null]);
      s.expectInZone(ace, "graveyard");
      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([LUNAR, GARY, VANILLA]);
    });

    it("tails: nothing happens and it stays set, face-down, never named to the opponent", () => {
      const s = withCoin("tails");
      const ace = s.card(ACE);
      s.endTurn();
      expect(fired(s)).toBe(0);
      expect(s.events.some((event) => event.type === "summoned")).toBe(false);
      s.expectInZone(ace, "field");
      expect(s.card(ace).faceUp).not.toBe(true);
      expect(JSON.stringify(s.view("p2"))).not.toContain(ACE);
    });

    it("R177 tails leaves the opponent nothing to read: their events and the shared counter match a face-down Sheepish's", () => {
      const s = withCoin("tails");
      const plain = scenario({ seed: s.state.seed, p1: { hand: [FILLER], backrow: [{ def: SHEEPISH, faceUp: false }], library: [LUNAR, VANILLA, GARY, VANILLA] }, p2: { hand: [FILLER], library: [FILLER, FILLER] } });
      s.endTurn();
      plain.endTurn();
      expect(s.state.nextSeq).toBe(plain.state.nextSeq);
      expect(s.view("p2").events).toEqual(plain.view("p2").events);
    });

    it("a trap left set flips again at your next end of turn, one coin each time", () => {
      const s = withCoin("tails");
      s.endTurn(); // p1's end: tails.
      s.endTurn(); // p2's end: no coin.
      const second = nextCoin(s.state);
      s.endTurn(); // p1's end again: a fresh coin.
      expect(fired(s)).toBe(second === "heads" ? 1 : 0);
      s.expectInZone(ACE, second === "heads" ? "graveyard" : "field");
    });

    it("the opponent's turns flip nothing: their end of turn draws no coin and leaves it set", () => {
      const s = withCoin("heads", { active: "p2" });
      const cursor = s.state.rngCursor;
      s.endTurn();
      expect(s.state.rngCursor).toBe(cursor);
      expect(fired(s)).toBe(0);
      s.expectInZone(ACE, "field");
    });

    it("heads with no permanent in the deck: it fires and recruits nothing", () => {
      const s = withCoin("heads", { p1: { library: [LUNAR, FILLER] } });
      s.endTurn();
      expect(fired(s)).toBe(1);
      expect(s.events.some((event) => event.type === "summoned")).toBe(false);
      s.expectInZone(ACE, "graveyard");
    });

    it("heads with the unit row full: it fires and the Unit stays on top of the deck", () => {
      const s = withCoin("heads", { p1: { field: [GARY, GARY, GARY, GARY, GARY], library: [VANILLA, LUNAR] } });
      s.endTurn();
      expect(fired(s)).toBe(1);
      expect(s.events.some((event) => event.type === "summoned")).toBe(false);
      expect(s.pile("p1", "library")[0]?.defId).toBe(VANILLA);
    });

    it("R33 a recruited Trap lands face-down, unnamed in the opponent's view", () => {
      const s = withCoin("heads", { p1: { library: [SHEEPISH, VANILLA] } });
      s.endTurn();
      const sheepish = s.card(SHEEPISH);
      expect(sheepish.zone.z).toBe("field");
      expect(sheepish.faceUp).not.toBe(true);
      expect(JSON.stringify(s.view("p2"))).not.toContain(SHEEPISH);
    });

    it("R386 an Upgrade recruits 2 on heads", () => {
      const s = withCoin("heads");
      stepParam(s.card(ACE), "recruits", 1);
      s.endTurn();
      expect(units(s)).toEqual([VANILLA, GARY, null, null, null]);
    });

    it("the coin is seeded: a round-tripped state flips the same coin and replays the same turn end", () => {
      const s = withCoin("heads");
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      const end = { type: "endTurn", playerId: "p1", nonce: "ace-roundtrip" } as Action;
      const live = reduce(s.state, end);
      const frozen = reduce(thawed, end);
      expect(live.error).toBeUndefined();
      expect(frozen.state).toEqual(live.state);
      expect(frozen.events).toEqual(live.events);
      expect(live.events.some((event) => event.type === "trapFired")).toBe(true);
    });
  });

  describe("radiant", () => {
    it("heads: it fires and Recruits 3", () => {
      const s = withCoin("heads", { radiant: true });
      s.endTurn();
      expect(fired(s)).toBe(1);
      expect(units(s)).toEqual([VANILLA, GARY, VANILLA, null, null]);
      s.expectInZone(ACE, "graveyard");
    });

    it("R665 tails: it Recruits 1 without firing, and stays set Revealed — readable, still armed", () => {
      const s = withCoin("tails", { radiant: true });
      const ace = s.card(ACE);
      s.endTurn();
      expect(fired(s)).toBe(0);
      expect(units(s)).toEqual([VANILLA, null, null, null, null]);
      s.expectInZone(ace, "field");
      expect(s.card(ace).faceUp).not.toBe(true);
      // R665: Revealed regardless of the coin flip — the opponent reads its face, but it never fired.
      expect(s.card(ace).revealed).toBe(true);
      const theirs = s.view("p2");
      expect(JSON.stringify(theirs)).toContain(ACE);
      expect(theirs.opponent.backrow[0]).toMatchObject({ faceDown: false });
    });

    it("tails: a recruited Trap lands face-down (R33), and the set trap flips again at your next end of turn", () => {
      const s = withCoin("tails", { radiant: true, p1: { library: [SHEEPISH, VANILLA, GARY, VANILLA] } });
      s.endTurn(); // p1's end: tails, Sheepish recruited.
      expect(s.card(SHEEPISH).zone.z).toBe("field");
      expect(s.card(SHEEPISH).faceUp).not.toBe(true);
      expect(JSON.stringify(s.view("p2"))).not.toContain(SHEEPISH);
      s.endTurn(); // p2's end: no coin.
      const second = nextCoin(s.state);
      s.endTurn(); // p1's end again: a fresh coin — heads recruits 3, tails 1.
      expect(fired(s)).toBe(second === "heads" ? 1 : 0);
      expect(units(s).filter((unit) => unit !== null)).toHaveLength(second === "heads" ? 3 : 1);
      s.expectInZone(ACE, second === "heads" ? "graveyard" : "field");
    });

    it("tails replays the same from a round-tripped state: one coin, one recruit, no firing", () => {
      const s = withCoin("tails", { radiant: true });
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      const end = { type: "endTurn", playerId: "p1", nonce: "ace-radiant-roundtrip" } as Action;
      const live = reduce(s.state, end);
      const frozen = reduce(thawed, end);
      expect(live.error).toBeUndefined();
      expect(frozen.state).toEqual(live.state);
      expect(frozen.events).toEqual(live.events);
      expect(live.state.rngCursor).toBe(s.state.rngCursor + 1);
      expect(live.events.filter((event) => event.type === "summoned")).toHaveLength(1);
      expect(live.events.some((event) => event.type === "trapFired")).toBe(false);
    });

    it("tails with nothing to recruit: nothing happens and it stays set", () => {
      const s = withCoin("tails", { radiant: true, p1: { library: [LUNAR] } });
      s.endTurn();
      expect(fired(s)).toBe(0);
      expect(s.events.some((event) => event.type === "summoned")).toBe(false);
      s.expectInZone(ACE, "field");
    });

    it("the opponent's turns flip nothing and recruit nothing", () => {
      const s = withCoin("tails", { radiant: true, active: "p2" });
      const cursor = s.state.rngCursor;
      s.endTurn();
      expect(s.state.rngCursor).toBe(cursor);
      expect(s.events.some((event) => event.type === "summoned")).toBe(false);
    });

    it("R33 a recruited Trap lands face-down", () => {
      const s = withCoin("heads", { radiant: true, p1: { library: [SHEEPISH] } });
      s.endTurn();
      expect(s.card(SHEEPISH).faceUp).not.toBe(true);
      expect(JSON.stringify(s.view("p2"))).not.toContain(SHEEPISH);
    });

    it("R386 a Degrade recruits 2 on heads", () => {
      const s = withCoin("heads", { radiant: true });
      stepParam(s.card(ACE), "recruits", -1);
      s.endTurn();
      expect(units(s)).toEqual([VANILLA, GARY, null, null, null]);
    });
  });
});
