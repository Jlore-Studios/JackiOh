// C+ #26 Tommy Tempo — SPEC §8.7 row 26, BUILD M9 Classic+ row C+ 26: "Taunt; cast on draw (R70: free,
// counts as played) into your leftmost open unit zone, then, once the rest of the drawing effect's
// list resolves (a "Draw 2" still draws its second card), your turn ends as if you had pressed End
// turn, every end-of-turn step running; drawn at the start of your turn it ends that turn before your
// main phase; drawn on the opponent's turn only the summon happens; with no open zone it goes to your
// hand uncast as R58's cap sends one (burned when the hand is full); played from a hand it is a plain
// Unit and ends nothing; radiant you may take one more main-phase action (a play, an attack, a
// position switch, an activation, or ending the turn yourself) and the turn ends once it resolves
// (R415); the action count reads through `param()`".

import { hashState, reduce, stepParam, type GameState } from "@jackioh/engine";
import type { Action, GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/026-tommy-tempo";

const TOMMY = "classicplus-026";
const STOCKPILE = "core-005"; // Draw 2. Heal your hero 2.
const MENACE = "core-019"; // 9/9 Taunt; end of turn: heal this to full.
const FILLER = "core-008"; // Mr. Vanilla, a (1) 4/4: a plain card to play or hold
const PANTHER = "core-032"; // Prem Panther: after it attacks and survives, draw 2 for each Unit it destroyed
const MOTHS = "core-009"; // Moths to the Flame: start of turn, every enemy Unit attacks this
const NOSE = "classic-015"; // Nose Hunter: "Activate: Discard a random card. …"
const SCARAB = "core-007"; // Jewelosco Scarab: Cry: Discover a (2) Cost card (a prompt)
const DECK = [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER];

/** p2 is active; p1's library has Tommy on top, so p2 ending the turn makes p1 draw it at its start. */
function drawnAtStart(radiantFace = false, p1: SideSetup = {}): Scenario {
  return scenario({
    active: "p2",
    p1: { hand: [FILLER], library: [{ def: TOMMY, radiant: radiantFace }, ...DECK], ...p1 },
    p2: { hand: [FILLER], library: DECK },
  });
}

function tommyOnField(s: Scenario): ReturnType<Scenario["card"]> | null {
  for (let lane = 1; lane <= 5; lane += 1) {
    const unit = s.unit("p1", lane);
    if (unit?.defId === TOMMY) return unit;
  }
  return null;
}

describe("C+ #26 Tommy Tempo", () => {
  it("is a (3) 9/9 Taunt Human Unit that casts itself on draw on both faces", () => {
    expect(def.cost).toBe(3);
    expect(def.base.keywords).toEqual([{ kind: "Taunt" }]);
    expect(base.staticFlags?.castOnDraw).toBe(true);
    expect(radiant.staticFlags?.castOnDraw).toBe(true);
  });

  describe("base", () => {
    it("R70 drawn at the start of your turn it is cast for free into the leftmost open zone, and that turn ends before its main phase", () => {
      const s = drawnAtStart(false, { field: [{ def: MENACE, lane: 1 }] });
      s.endTurn();
      const tommy = tommyOnField(s);
      expect(tommy).not.toBeNull();
      expect(s.unit("p1", 2)?.id).toBe(tommy?.id);
      const played = s.events.find((event) => event.type === "cardPlayed" && event.instanceId === tommy?.id);
      expect(played).toMatchObject({ costPaid: 0 });
      const cut = s.events.find((event) => event.type === "turnCutShort");
      expect(cut).toMatchObject({ player: "p1", byInstanceId: tommy?.id });
      // p1 never reached a main phase: the turn ended and p2's began.
      expect(s.state.active).toBe("p2");
      expect(s.stats(tommy ?? "").keywords.some((keyword) => keyword.kind === "Taunt")).toBe(true);
    });

    it("§6.3 End the turn: a Draw 2 draws its second card and heals, then the turn ends with every end-of-turn step", () => {
      const s = scenario({
        p1: { hand: [STOCKPILE, FILLER], library: [TOMMY, FILLER, ...DECK], field: [{ def: MENACE, lane: 1, damage: 4 }], health: 20 },
        p2: { hand: [FILLER], library: DECK },
      });
      const menace = s.unit("p1", 1);
      if (menace === null) throw new Error("setup");
      const second = s.state.players.p1.library[1];
      s.play(STOCKPILE);
      const log = s.lastEvents;
      const at = (match: (event: GameEvent) => boolean): number => log.findIndex(match);
      const cut = at((event) => event.type === "turnCutShort");
      // The second draw and the heal resolve before the turn is cut short.
      expect(s.card(second ?? "").zone.z).toBe("hand");
      expect(cut).toBeGreaterThan(at((event) => event.type === "drawn" && event.instanceId === second?.id));
      expect(cut).toBeGreaterThan(at((event) => event.type === "healed" && event.targetId === "hero-p1"));
      // Every end-of-turn step ran: the Menace healed to full at the end of p1's turn.
      expect(s.card(menace).damage).toBe(0);
      expect(s.state.active).toBe("p2");
      s.expectHealth("p1", 22);
    });

    it("R415 drawn on the opponent's turn only the summon happens", () => {
      // p2's Moths to the Flame makes p1's Prem Panther attack it at p2's start of turn; the Panther kills
      // it and survives, so p1 draws 2 on p2's turn — Tommy Tempo first.
      const s = scenario({
        p1: { hand: [FILLER], library: [TOMMY, ...DECK], field: [{ def: PANTHER, lane: 1 }] },
        p2: { hand: [FILLER, FILLER], library: DECK, field: [{ def: MOTHS, lane: 3, damage: 10 }] },
      });
      s.endTurn();
      const tommy = tommyOnField(s);
      expect(tommy).not.toBeNull();
      expect(s.events.find((event) => event.type === "cardPlayed" && event.instanceId === tommy?.id)).toMatchObject({ costPaid: 0 });
      // No rider on anybody, nothing cut short, and p2's turn goes on: p2 still plays.
      expect(s.events.some((event) => event.type === "turnCutShort")).toBe(false);
      expect(s.state.players.p1.mods.some((mod) => mod.kind === "turnEnds")).toBe(false);
      expect(s.state.players.p2.mods.some((mod) => mod.kind === "turnEnds")).toBe(false);
      expect(s.state.active).toBe("p2");
      s.play(s.hand("p2")[0] ?? FILLER);
      expect(s.state.active).toBe("p2");
    });

    it("R560 with no open unit zone it goes to the hand uncast and ends nothing", () => {
      const s = drawnAtStart(false, { field: [MENACE, MENACE, MENACE, MENACE, MENACE] });
      s.endTurn();
      const tommy = s.hand("p1").find((card) => card.defId === TOMMY);
      expect(tommy).toBeDefined();
      expect(s.events.some((event) => event.type === "cardPlayed" && event.instanceId === tommy?.id)).toBe(false);
      expect(s.events.some((event) => event.type === "turnCutShort")).toBe(false);
      expect(s.state.active).toBe("p1");
      expect(s.state.phase).toBe("main");
    });

    it("R560 R317 with no open unit zone and a full hand it burns", () => {
      const s = drawnAtStart(false, {
        field: [MENACE, MENACE, MENACE, MENACE, MENACE],
        hand: Array.from({ length: 10 }, () => FILLER),
      });
      s.endTurn();
      const burned = s.events.find((event) => event.type === "burned");
      expect(burned).toMatchObject({ defId: TOMMY });
      expect(s.events.some((event) => event.type === "turnCutShort")).toBe(false);
    });

    it("played from a hand it is a plain Unit and ends nothing", () => {
      const s = scenario({ p1: { hand: [TOMMY, FILLER], library: DECK }, p2: { hand: [FILLER], library: DECK } });
      s.play(TOMMY);
      expect(tommyOnField(s)).not.toBeNull();
      expect(s.events.some((event) => event.type === "turnCutShort")).toBe(false);
      expect(s.state.players.p1.mods.some((mod) => mod.kind === "turnEnds")).toBe(false);
      s.play(FILLER);
      expect(s.state.active).toBe("p1");
    });

    it("§9.3 the draw that casts it and the turn it cuts short replay from a JSON copy to the same hash", () => {
      const s = drawnAtStart();
      const action = { type: "endTurn", playerId: "p2", nonce: "tommy-replay" } as Action;
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      const live = reduce(s.state, action);
      expect(live.error).toBeUndefined();
      expect(live.events.some((event) => event.type === "turnCutShort")).toBe(true);
      expect(live.state.active).toBe("p2");
      expect(hashState(reduce(thawed, action).state)).toBe(hashState(live.state));
    });
  });

  describe("radiant", () => {
    it("R415 drawn at the start of your turn, you take one more action, and the turn ends once it resolves", () => {
      const s = drawnAtStart(true);
      s.endTurn();
      expect(s.state.active).toBe("p1");
      const tommy = tommyOnField(s);
      s.expectStats(tommy ?? "", { attack: 18, health: 18 });
      s.play(FILLER);
      expect(s.events.some((event) => event.type === "turnCutShort")).toBe(true);
      expect(s.state.active).toBe("p2");
    });

    it("R415 drawn on the opponent's turn only the summon happens: no action is counted", () => {
      const s = scenario({
        p1: { hand: [FILLER], library: [{ def: TOMMY, radiant: true }, ...DECK], field: [{ def: PANTHER, lane: 1 }] },
        p2: { hand: [FILLER, FILLER], library: DECK, field: [{ def: MOTHS, lane: 3, damage: 10 }] },
      });
      s.endTurn();
      s.expectStats(tommyOnField(s) ?? "", { attack: 18, health: 18 });
      expect(s.state.players.p1.mods.some((mod) => mod.kind === "turnEnds")).toBe(false);
      expect(s.state.players.p2.mods.some((mod) => mod.kind === "turnEnds")).toBe(false);
      s.play(s.hand("p2")[0] ?? FILLER);
      expect(s.state.active).toBe("p2");
    });

    it("R113 R415 the one action asks a question: the turn ends once it is answered, after a JSON round trip", () => {
      const s = drawnAtStart(true, { hand: [SCARAB, FILLER] });
      s.endTurn();
      s.play(SCARAB);
      const pending = s.state.pending;
      expect(pending?.playerId).toBe("p1");
      expect(s.state.active).toBe("p1");
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(thawed).toEqual(s.state);
      const choice = pending?.options[0]?.selection;
      if (pending === null || choice === undefined) throw new Error("no Discover");
      const action = { type: "answer", playerId: "p1", choiceId: pending.id, selection: [choice], nonce: "tommy-answer" } as Action;
      const live = reduce(s.state, action);
      const frozen = reduce(thawed, action);
      expect(live.error).toBeUndefined();
      expect(live.state.active).toBe("p2");
      expect(live.events.some((event) => event.type === "turnCutShort")).toBe(true);
      expect(hashState(frozen.state)).toBe(hashState(live.state));
    });

    it("R415 an attack is the action", () => {
      const s = drawnAtStart(true, { field: [{ def: MENACE, lane: 1 }] });
      s.endTurn();
      s.attack(s.unit("p1", 1) ?? "", "hero");
      expect(s.state.active).toBe("p2");
    });

    it("R415 a position switch is the action", () => {
      const s = drawnAtStart(true, { field: [{ def: MENACE, lane: 1 }] });
      s.endTurn();
      s.switchPosition(s.unit("p1", 1) ?? "");
      expect(s.state.active).toBe("p2");
    });

    it("R415 an activation is the action", () => {
      const s = drawnAtStart(true, { field: [{ def: NOSE, lane: 1 }], hand: [FILLER, FILLER] });
      s.endTurn();
      s.activate(s.unit("p1", 1) ?? "");
      expect(s.events.some((event) => event.type === "activated")).toBe(true);
      expect(s.state.active).toBe("p2");
    });

    it("R415 ending the turn yourself uses it", () => {
      const s = drawnAtStart(true);
      s.endTurn();
      s.endTurn();
      expect(s.state.active).toBe("p2");
      expect(s.events.filter((event) => event.type === "turnCutShort")).toHaveLength(0);
    });

    it("R386 the action count reads through param: an Upgrade leaves two actions", () => {
      const s = drawnAtStart(true, { hand: [FILLER, FILLER] });
      const tommy = s.state.players.p1.library[0];
      if (tommy === undefined) throw new Error("setup");
      stepParam(tommy, "actions", 1);
      s.endTurn();
      s.play(FILLER);
      expect(s.state.active).toBe("p1");
      s.play(FILLER);
      expect(s.state.active).toBe("p2");
    });

    it("R113 the rider is state: the turn waiting on its last action survives JSON", () => {
      const s = drawnAtStart(true);
      s.endTurn();
      expect(s.state.players.p1.mods.find((mod) => mod.kind === "turnEnds")).toMatchObject({ actionsLeft: 1 });
      expect(JSON.parse(JSON.stringify(s.state))).toEqual(s.state);
    });
  });
});
