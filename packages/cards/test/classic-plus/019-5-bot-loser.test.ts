// C+ #19.5 Bot Loser — SPEC §8.7 row 19.5, BUILD M9 Classic+ row C+ 19.5: "Rush, First Strike; whenever
// it destroys a Unit (R42), a forced attack's kill included, it gets +5 Attack permanently; while
// Berserk (set by Jungle Loser, lost on leaving the field, R78) it makes a forced attack (R53) on its
// own controller's hero at the start and at the end of that player's turn, the hero never striking back
// and that hero's Armor and caps applying; not Berserk, nothing; `conditionMet` on the field answers
// whether it is Berserk (R195); the gain reads through `param()`; radiant Charge, First Strike, +10
// Attack, and it can't go Berserk (R412): Jungle Loser's base face never sets the flag".
//
// R195's proofs for this card are in its own file (the `describe("R195 conditionMet …")` block).

import { applyEffects, createRng, makeContext, setParam, stepParam } from "@jackioh/engine";
import { goBerserk } from "@jackioh/engine/effects";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/019-5-bot-loser";

const BOT = "classicplus-019-5";
const JUNGLE = "classicplus-019-2";
const VANILLA = "core-008"; // 4/4
const MOTHS = "core-009"; // 1/14; start of turn: every enemy Unit attacks this
const ANTI_ONESHOT = "core-073"; // hits on the hero capped at 5
const HIT_JOB = "core-016"; // destroy a target Unit
const FILLER = "core-005";
const DECK = [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER];

function bot(s: Scenario, player: "p1" | "p2" = "p1"): ReturnType<Scenario["card"]> {
  for (let lane = 1; lane <= 5; lane += 1) {
    const unit = s.unit(player, lane);
    if (unit?.defId === BOT) return unit;
  }
  throw new Error("no Bot Loser");
}

function sendBerserk(s: Scenario, card: ReturnType<Scenario["card"]>): void {
  const sink = { state: s.state, events: [] as GameEvent[], rng: createRng(s.state.seed, s.state.rngCursor) };
  applyEffects([goBerserk({ target: { of: "instance", instanceId: card.id } })], makeContext(sink, null, { controller: card.controller }));
}

function withBot(p1: SideSetup = {}, radiantFace = false, p2: SideSetup = {}): Scenario {
  return scenario({
    p1: { hand: [FILLER], library: DECK, ...p1, field: [{ def: BOT, lane: 3, radiant: radiantFace }, ...(p1.field ?? [])] },
    p2: { hand: [FILLER], library: DECK, ...p2 },
  });
}

function glows(s: Scenario): boolean {
  return s.view("p1").you.units[2]?.conditionActive === true;
}

describe("C+ #19.5 Bot Loser", () => {
  it("is a (2) 5/5 Rush, First Strike token, printed Legendary; the Radiant face is Charge and never Berserk", () => {
    expect(def.base.keywords).toEqual([{ kind: "Rush" }, { kind: "First Strike" }]);
    expect(def.radiant.keywords).toEqual([{ kind: "Charge" }, { kind: "First Strike" }]);
    expect(radiant.staticFlags?.neverBerserk).toBe(true);
    expect(radiant.startOfTurn).toBeUndefined();
    expect(base.startOfTurn).toBeDefined();
  });

  describe("base", () => {
    it("R42 whenever it destroys a Unit it gets +5 Attack permanently", () => {
      const s = withBot({}, false, { field: [{ def: VANILLA, lane: 3 }] });
      s.attack(bot(s), s.unit("p2", 3) ?? "");
      s.expectStats(bot(s), { attack: 10, health: 5 });
      s.endTurn().endTurn();
      s.expectStats(bot(s), { attack: 10 });
    });

    it("R42 a kill in a forced attack counts: Moths to the Flame's compulsion", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [FILLER], field: [{ def: BOT, lane: 2 }], library: DECK },
        p2: { hand: [FILLER], field: [{ def: MOTHS, lane: 1, damage: 10 }], library: DECK },
      });
      // p2's start of turn: every enemy Unit attacks Moths.
      s.startTurn();
      expect(s.unit("p2", 1)).toBeNull();
      s.expectStats(bot(s), { attack: 10 });
    });

    it("a kill it does not make gives it nothing", () => {
      const s = withBot({ field: [{ def: VANILLA, lane: 1 }] }, false, { field: [{ def: VANILLA, lane: 1, damage: 2 }] });
      s.attack(s.unit("p1", 1) ?? "", s.unit("p2", 1) ?? "");
      s.expectStats(bot(s), { attack: 5 });
    });

    it("R53 while Berserk it attacks its own hero at the start and at the end of your turn; the hero never strikes back", () => {
      const s = withBot();
      sendBerserk(s, bot(s));
      s.endTurn();
      // The end of p1's turn: 5 to p1's own hero.
      s.expectHealth("p1", 25);
      s.expectStats(bot(s), { health: 5 });
      s.endTurn();
      // The start of p1's next turn: 5 more.
      s.expectHealth("p1", 20);
      const own = s.events.filter((event) => event.type === "attackDeclared" && event.targetId === "hero-p1");
      expect(own).toHaveLength(2);
      for (const event of own) expect(event).toMatchObject({ forced: true });
    });

    it("§4.4 the hit on its own hero takes that hero's Armor and caps", () => {
      const s = withBot({ armor: 2, backrow: [{ def: ANTI_ONESHOT, lane: 1 }] });
      stepParam(bot(s), "attackGain", 0);
      bot(s).buffs.attack += 5;
      sendBerserk(s, bot(s));
      s.endTurn();
      // 10 attack, 2 Armor, then the cap of 5.
      s.expectHealth("p1", 25);
    });

    it("not Berserk, it attacks nothing at the start or end of a turn", () => {
      const s = withBot();
      s.endTurn().endTurn();
      s.expectHealth("p1", 30);
    });

    it("R78 Berserk is lost when it leaves the field: back by Reborn, it is not Berserk and attacks nothing", () => {
      const s = withBot({ hand: [HIT_JOB, FILLER] });
      const first = bot(s);
      s.card(first).grantedKeywords.push({ kind: "Reborn" });
      sendBerserk(s, first);
      expect(s.card(first).berserk).toBe(true);
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: first.id }] });
      expect(s.lastEvents.some((event) => event.type === "destroyed" && event.instanceId === first.id)).toBe(true);
      const back = bot(s);
      expect(back.berserk).toBeUndefined();
      s.endTurn();
      s.expectHealth("p1", 30);
    });

    it("R412 Jungle Loser's base face sends it Berserk when it kills the Unit across from it", () => {
      const s = withBot({ field: [{ def: JUNGLE, lane: 1 }] }, false, { field: [{ def: VANILLA, lane: 3 }] });
      setParam(s.unit("p1", 1) ?? bot(s), "chance", 100);
      s.endTurn();
      expect(bot(s).berserk).toBe(true);
      expect(s.view("p2").opponent.units[2]?.berserk).toBe(true);
    });

    it("R386 the gain reads through param: an Upgrade makes it +6", () => {
      const s = withBot({}, false, { field: [{ def: VANILLA, lane: 3 }] });
      stepParam(bot(s), "attackGain", 1);
      s.attack(bot(s), s.unit("p2", 3) ?? "");
      s.expectStats(bot(s), { attack: 11 });
    });
  });

  describe("radiant", () => {
    it("10/10 Charge, First Strike: it gets +10 Attack per kill", () => {
      const s = withBot({}, true, { field: [{ def: VANILLA, lane: 3 }] });
      s.attack(bot(s), s.unit("p2", 3) ?? "");
      s.expectStats(bot(s), { attack: 20, health: 10 });
    });

    it("R412 it can't go Berserk: Jungle Loser's base face never sets the flag", () => {
      const s = withBot({ field: [{ def: JUNGLE, lane: 1 }] }, true, { field: [{ def: VANILLA, lane: 3 }] });
      setParam(s.unit("p1", 1) ?? bot(s), "chance", 100);
      s.endTurn();
      expect(s.unit("p2", 3)).toBeNull();
      expect(bot(s).berserk).toBeUndefined();
      s.endTurn();
      s.expectHealth("p1", 30);
    });

    it("R412 a Berserk Bot Loser made Radiant stops attacking its hero", () => {
      const s = withBot();
      sendBerserk(s, bot(s));
      bot(s).radiant = true;
      s.endTurn().endTurn();
      s.expectHealth("p1", 30);
    });
  });

  describe("R195 conditionMet on the field: whether it is Berserk", () => {
    it("lit while Berserk, and its end of turn attacks the hero", () => {
      const s = withBot();
      expect(glows(s)).toBe(false);
      sendBerserk(s, bot(s));
      expect(glows(s)).toBe(true);
      s.endTurn();
      s.expectHealth("p1", 25);
    });

    it("unlit when not Berserk, and its end of turn attacks nothing", () => {
      const s = withBot();
      expect(glows(s)).toBe(false);
      s.endTurn();
      s.expectHealth("p1", 30);
    });

    it("never on the opponent's view", () => {
      const s = withBot();
      sendBerserk(s, bot(s));
      expect(s.view("p2").opponent.units[2]?.conditionActive).toBeUndefined();
    });
  });
});
