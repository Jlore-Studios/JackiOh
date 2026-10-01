// C #78 Mutate Spell — SPEC §8.6 row 78, BUILD M9 Classic row C 78: "A Field Spell (R402); Activate ♾️
// (R384), the target carried in the `activate` action: a permanent with a Plague Token, either side,
// face-down included (the option carries only its id, R177); remove one token, then: an enemy permanent is
// exiled; your backrow card, itself included, draws 2; your Unit makes one forced attack (R53) on a random
// enemy it may attack, hero or Unit, spending no exertion; with no tokened permanent it can't activate; at
// most `ACTIVATE_UNLIMITED_CAP` uses per turn; not a play; radiant: an enemy permanent is fused (R77, R102)
// onto a card of yours of its type, picked in a prompt over your field, hand and deck, your Immutable
// cards never offered and the deck's cards shown to you only (§10.8), never named in the opponent's view;
// the enemy card ceases to exist; with no card of its type it is exiled; your backrow card draws 4; your
// Unit attacks a random enemy twice, the second only if it survived the first; its tuned numbers (draw,
// attacks) read through `param()` (R386)".

import {
  ACTIVATE_UNLIMITED_CAP,
  hashState,
  legalActions,
  reduce,
  stepParam,
  subsystems,
  type CardInstance,
  type GameState,
} from "@jackioh/engine";
import type { ActionBody, GameEvent, PlayerId, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/078-mutate-spell";

const MUTATE = "classic-078";
const CRAWLER = "classic-053"; // (1) Unit: whenever Plague Tokens are placed on this, draw 1.
const VANILLA = "core-008"; // (1) Unit 4/4.
const MENACE = "core-019"; // (3) Unit 9/9 Taunt; Radiant adds Immutable.
const TIMMY = "core-011"; // (1) Unit 3/3 Rush, First Strike.
const PAWN = "core-096"; // (1) Trap: answers only an attack that would be lethal.
const SHEEPISH = "core-041"; // (1) Trap.
const MANA_WELL = "core-006"; // (3) Field Spell.
const FILLER = "core-005"; // (1) Spell (§2.5).
const X = "core-020"; // library filler.

type Activate = Extract<ActionBody, { type: "activate" }>;

function lib(n: number): string[] {
  return Array.from({ length: n }, () => X);
}

function at(card: CardInstance): Selection[] {
  return [{ pick: "instance", instanceId: card.id }];
}

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`missing: ${what}`);
  return value;
}

function drawsBy(events: readonly GameEvent[], player: PlayerId): number {
  return events.filter((event) => event.type === "drawn" && event.player === player).length;
}

function attacksBy(events: readonly GameEvent[], attacker: CardInstance): string[] {
  return events.flatMap((event) => (event.type === "attackDeclared" && event.attackerId === attacker.id ? [event.targetId] : []));
}

/** The targets legalActions offers for Mutate Spell's activation, each once. */
function offered(s: Scenario): string[] {
  const ids = legalActions(s.state, "p1")
    .filter((action): action is Activate => action.type === "activate" && action.instanceId === s.card(MUTATE).id)
    .flatMap((action) => (action.targets ?? []).flatMap((target) => (target.pick === "instance" ? [target.instanceId] : [])));
  return [...new Set(ids)];
}

describe("C #78 Mutate Spell", () => {
  it("R402 is a Field Spell with one Activate ♾️ ability, its target a tokened permanent, and two numbers", () => {
    expect(def.id).toBe(MUTATE);
    expect(def.type).toBe("Field Spell");
    expect(def.params).toEqual([
      { key: "draw", base: 2, radiant: 4, better: "up", step: 1, min: 1 },
      { key: "attacks", base: 1, radiant: 2, better: "up", step: 1, min: 1 },
    ]);
    for (const script of [base, radiant]) {
      expect(script.activations).toHaveLength(1);
      expect(script.activations?.[0]?.uses).toBe("unlimited");
      expect(script.activations?.[0]?.targets).toEqual([
        { kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "backrow"], plague: true } },
      ]);
    }
  });

  describe("base", () => {
    it("R402 an enemy permanent: its token is removed, then it is exiled", () => {
      const s = scenario({ p1: { hand: [FILLER], backrow: [MUTATE] }, p2: { hand: [FILLER], field: [{ def: VANILLA, counters: { plague: 2 } }] } });
      const vanilla = s.card(VANILLA);

      s.activate(MUTATE, { targets: at(vanilla) });

      s.expectInZone(vanilla, "exile");
      const kinds = s.lastEvents.flatMap((event) => (event.type === "activated" || event.type === "counterChanged" || event.type === "exiled" ? [event.type] : []));
      expect(kinds).toEqual(["activated", "counterChanged", "exiled"]);
      s.expectInZone(MUTATE, "field");
    });

    it("R177 a face-down enemy trap with a token is offered by its id alone, and is exiled", () => {
      const s = scenario({ p1: { hand: [FILLER], backrow: [MUTATE] }, p2: { hand: [FILLER], backrow: [{ def: PAWN, faceUp: false, counters: { plague: 1 } }] } });
      const pawn = s.card(PAWN);

      expect(offered(s)).toEqual([pawn.id]);
      expect(JSON.stringify(s.view("p1"))).not.toContain(PAWN);
      s.activate(MUTATE, { targets: at(pawn) });

      s.expectInZone(pawn, "exile");
    });

    it("R402 your backrow card draws 2: another Field Spell, or Mutate Spell itself", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [{ def: MUTATE, counters: { plague: 1 } }, { def: MANA_WELL, lane: 2, counters: { plague: 1 } }], library: lib(5) },
        p2: { hand: [FILLER] },
      });

      s.activate(MUTATE, { targets: at(s.card(MANA_WELL)) });
      expect(drawsBy(s.lastEvents, "p1")).toBe(2);
      expect(s.card(MANA_WELL).counters.plague).toBeUndefined();
      s.expectInZone(MANA_WELL, "field");

      s.activate(MUTATE, { targets: at(s.card(MUTATE)) });
      expect(drawsBy(s.lastEvents, "p1")).toBe(2);
    });

    it("R53 your Unit makes one forced attack on a random enemy, spending no exertion", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: VANILLA, counters: { plague: 1 } }], backrow: [MUTATE] },
        p2: { hand: [FILLER], health: 30 },
      });
      const vanilla = s.card(VANILLA);

      s.activate(MUTATE, { targets: at(vanilla) });

      expect(attacksBy(s.lastEvents, vanilla)).toEqual(["hero-p2"]);
      s.expectHealth("p2", 26);
      // No exertion spent: it may still attack this turn.
      s.attack(vanilla, "hero");
      s.expectHealth("p2", 22);
    });

    it("R53 the random enemy is a hero or a Unit it may attack, Taunt and position ignored", () => {
      const seen = new Set<string>();
      for (let seed = 0; seed < 12 && seen.size < 2; seed += 1) {
        const s = scenario({
          seed: `mutate-random-${seed}`,
          p1: { hand: [FILLER], field: [{ def: MENACE, counters: { plague: 1 } }], backrow: [MUTATE] },
          p2: { hand: [FILLER], field: [{ def: VANILLA, position: "DEF" }], health: 30 },
        });
        const menace = s.card(MENACE);
        s.activate(MUTATE, { targets: at(menace) });
        const targets = attacksBy(s.lastEvents, menace);
        expect(targets).toHaveLength(1);
        seen.add(targets[0] === "hero-p2" ? "hero" : "unit");
      }
      expect(seen).toEqual(new Set(["hero", "unit"]));
    });

    it("a removal is no placement: a Plague Crawler targeted draws nothing", () => {
      const s = scenario({ p1: { hand: [FILLER], field: [{ def: CRAWLER, counters: { plague: 1 } }], backrow: [MUTATE], library: lib(3) }, p2: { hand: [FILLER] } });

      s.activate(MUTATE, { targets: at(s.card(CRAWLER)) });

      expect(drawsBy(s.lastEvents, "p1")).toBe(0);
      expect(s.lastEvents.filter((event) => event.type === "counterChanged")).toEqual([
        { type: "counterChanged", instanceId: s.card(CRAWLER).id, counter: "plague", value: 0 },
      ]);
    });

    it("R384 with no tokened permanent it can't activate, and a clean target is refused", () => {
      const s = scenario({ p1: { hand: [FILLER], field: [VANILLA], backrow: [MUTATE] }, p2: { hand: [FILLER], field: [MENACE] } });

      expect(legalActions(s.state, "p1").some((action) => action.type === "activate")).toBe(false);
      expect(() => s.activate(MUTATE, { targets: at(s.card(MENACE)) })).toThrow();
      const view = s.view("p1").you.backrow[0];
      expect(JSON.stringify(view)).toContain('"usable":false');

      const t = scenario({ p1: { hand: [FILLER], field: [{ def: VANILLA, counters: { plague: 1 } }], backrow: [MUTATE] }, p2: { hand: [FILLER], field: [MENACE] } });
      expect(offered(t)).toEqual([t.card(VANILLA).id]);
      expect(() => t.activate(MUTATE, { targets: at(t.card(MENACE)) })).toThrow();
    });

    it("§3.2 R13 a card dormant under a Stack pile is not offered, even with tokens", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [MUTATE] },
        p2: { hand: [FILLER], field: [{ def: VANILLA, counters: { plague: 2 } }, { def: "core-092", stack: true }] },
      });

      expect(offered(s)).toEqual([]);
      expect(() => s.activate(MUTATE, { targets: at(s.card(VANILLA)) })).toThrow();
    });

    it("R384 ♾️: again and again in one turn while tokens remain, and not a play", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [MUTATE] },
        p2: { hand: [FILLER], field: [{ def: VANILLA, counters: { plague: 1 } }, { def: MENACE, lane: 2, counters: { plague: 1 } }, { def: TIMMY, lane: 3, counters: { plague: 1 } }] },
      });
      const played = s.state.players.p1.turnLog.cardsPlayed;

      for (const card of [s.card(VANILLA), s.card(MENACE), s.card(TIMMY)]) s.activate(MUTATE, { targets: at(card) });

      expect(s.pile("p2", "exile")).toHaveLength(3);
      expect(s.events.filter((event) => event.type === "activated")).toHaveLength(3);
      expect(s.events.some((event) => event.type === "cardPlayed")).toBe(false);
      expect(s.state.players.p1.turnLog.cardsPlayed).toBe(played);
    });

    it("R384 at most ACTIVATE_UNLIMITED_CAP uses per turn", () => {
      const s = scenario({ p1: { hand: [FILLER], backrow: [MUTATE] }, p2: { hand: [FILLER], field: [{ def: VANILLA, counters: { plague: 1 } }] } });
      s.card(MUTATE).memory[subsystems.ACTIVATIONS_MEMORY_KEY] = { turn: s.state.turn, count: ACTIVATE_UNLIMITED_CAP };

      expect(legalActions(s.state, "p1").some((action) => action.type === "activate")).toBe(false);
      expect(() => s.activate(MUTATE, { targets: at(s.card(VANILLA)) })).toThrow(/100 times/);
    });

    it("R384 only its controller, in their own main phase", () => {
      const s = scenario({ active: "p2", p1: { hand: [FILLER], backrow: [MUTATE] }, p2: { hand: [FILLER], field: [{ def: VANILLA, counters: { plague: 1 } }] } });

      expect(() => s.activate(MUTATE, { targets: at(s.card(VANILLA)) })).toThrow(/not your turn/);
    });

    it("§2.4 a full hand burns the draws", () => {
      const s = scenario({
        p1: { hand: Array.from({ length: 10 }, () => FILLER), backrow: [{ def: MUTATE, counters: { plague: 1 } }], library: lib(3) },
        p2: { hand: [FILLER] },
      });

      s.activate(MUTATE, { targets: at(s.card(MUTATE)) });

      expect(s.lastEvents.filter((event) => event.type === "burned")).toHaveLength(2);
    });

    it("R386 an Upgrade draws 3 and attacks twice", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: VANILLA, counters: { plague: 1 } }], backrow: [{ def: MUTATE, counters: { plague: 1 } }], library: lib(4) },
        p2: { hand: [FILLER], health: 30 },
      });
      stepParam(s.card(MUTATE), "draw", 1);
      stepParam(s.card(MUTATE), "attacks", 1);

      s.activate(MUTATE, { targets: at(s.card(MUTATE)) });
      expect(drawsBy(s.lastEvents, "p1")).toBe(3);
      s.activate(MUTATE, { targets: at(s.card(VANILLA)) });
      expect(attacksBy(s.lastEvents, s.card(VANILLA))).toEqual(["hero-p2", "hero-p2"]);
      s.expectHealth("p2", 22);
    });
  });

  describe("radiant", () => {
    function fusing(extra: { hand?: readonly string[]; field?: readonly (string | { def: string; radiant?: boolean })[]; library?: readonly string[] } = {}): Scenario {
      return scenario({
        p1: {
          hand: [FILLER, ...(extra.hand ?? [])],
          field: [...(extra.field ?? [])],
          backrow: [{ def: MUTATE, radiant: true }],
          library: [...(extra.library ?? [])],
        },
        p2: { hand: [FILLER], field: [{ def: VANILLA, counters: { plague: 1 } }], backrow: [{ def: SHEEPISH, faceUp: false, counters: { plague: 1 } }] },
      });
    }

    it("R77 an enemy Unit is fused onto a Unit of yours you pick from your field, hand and deck; it ceases to exist", () => {
      const s = fusing({ field: [TIMMY], hand: [MENACE], library: ["core-012", X] });
      const enemy = s.card(VANILLA);
      const timmy = s.card(TIMMY);

      s.activate(MUTATE, { targets: at(enemy) });

      const prompt = must(s.state.pending, "the fuse prompt");
      expect(prompt.playerId).toBe("p1");
      const options = prompt.options.flatMap((option) => (option.selection.pick === "instance" ? [option.selection.instanceId] : []));
      const library = s.pile("p1", "library").filter((card) => card.defId !== FILLER);
      expect(new Set(options)).toEqual(new Set([timmy.id, s.card(MENACE).id, ...library.map((card) => card.id)]));
      s.answer(timmy.id);

      s.expectInZone(enemy, "gone");
      const fused = must(s.unit("p1", 1), "the fused card");
      expect(fused.id).toBe(timmy.id);
      expect(fused.defId).not.toBe(TIMMY);
      expect(s.lastEvents.some((event) => event.type === "fused")).toBe(true);
    });

    it("R23 your Immutable cards are never offered", () => {
      const s = fusing({ field: [{ def: MENACE, radiant: true }, TIMMY] });

      s.activate(MUTATE, { targets: at(s.card(VANILLA)) });

      const options = must(s.state.pending, "the fuse prompt").options.flatMap((option) => (option.selection.pick === "instance" ? [option.selection.instanceId] : []));
      expect(options).toEqual([s.card(TIMMY).id]);
    });

    it("§10.8 the deck's cards are shown to you only: the opponent's view names none of them", () => {
      const s = fusing({ library: ["core-012", "core-015"] });
      const enemy = s.card(VANILLA);

      s.activate(MUTATE, { targets: at(enemy) });

      const mine = must(s.view("p1").pending, "p1's prompt");
      if (!mine.forYou) throw new Error("the prompt is p1's");
      expect(mine.options.map((option) => option.defId).sort()).toEqual(["core-012", "core-015"]);
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
      const theirs = JSON.stringify(s.view("p2"));
      expect(theirs).not.toContain("core-012");
      expect(theirs).not.toContain("core-015");

      const deckCard = s.pile("p1", "library").find((card) => card.defId === "core-015") as CardInstance;
      s.answer(deckCard.id);

      expect(JSON.stringify(s.view("p2"))).not.toContain("core-015");
      s.expectInZone(enemy, "gone");
    });

    it("a face-down enemy Trap is fused onto a Trap of yours in hand (Trap and Field Trap are one type)", () => {
      const s = fusing({ hand: [PAWN] });
      const sheepish = s.card(SHEEPISH);

      s.activate(MUTATE, { targets: at(sheepish) });
      const pawn = s.card(PAWN);
      expect(must(s.state.pending, "the fuse prompt").options.map((option) => option.key)).toEqual([`instance:${pawn.id}`]);
      s.answer(pawn.id);

      s.expectInZone(sheepish, "gone");
      expect(s.hand("p1").some((card) => card.id === pawn.id && card.defId !== PAWN)).toBe(true);
    });

    it("with no card of its type, it is exiled with no prompt", () => {
      const s = fusing({ hand: [MANA_WELL] });

      s.activate(MUTATE, { targets: at(s.card(VANILLA)) });

      expect(s.state.pending).toBeNull();
      s.expectInZone(VANILLA, "exile");
    });

    it("§9.3 the fuse prompt survives a JSON round trip and finishes as the live one does", () => {
      const s = fusing({ field: [TIMMY], hand: [MENACE] });
      s.activate(MUTATE, { targets: at(s.card(VANILLA)) });

      const revived = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(revived).toEqual(s.state);
      const choice = must(revived.pending, "the fuse prompt");
      const result = reduce(revived, { type: "answer", playerId: "p1", choiceId: choice.id, selection: at(s.card(MENACE)), nonce: "mutate-json" });
      expect(result.error).toBeUndefined();
      s.answer(s.card(MENACE).id);

      expect(hashState(result.state)).toBe(hashState(s.state));
    });

    it("your backrow card draws 4", () => {
      const s = scenario({ p1: { hand: [FILLER], backrow: [{ def: MUTATE, radiant: true, counters: { plague: 1 } }], library: lib(5) }, p2: { hand: [FILLER] } });

      s.activate(MUTATE, { targets: at(s.card(MUTATE)) });

      expect(drawsBy(s.lastEvents, "p1")).toBe(4);
    });

    it("R53 your Unit attacks a random enemy twice when it survives the first", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: VANILLA, counters: { plague: 1 } }], backrow: [{ def: MUTATE, radiant: true }] },
        p2: { hand: [FILLER], health: 30 },
      });
      const vanilla = s.card(VANILLA);

      s.activate(MUTATE, { targets: at(vanilla) });

      expect(attacksBy(s.lastEvents, vanilla)).toEqual(["hero-p2", "hero-p2"]);
      s.expectHealth("p2", 22);
    });

    it("R96 the second attack happens only if it survived the first", () => {
      let diedFirst = 0;
      for (let seed = 0; seed < 16; seed += 1) {
        const s = scenario({
          seed: `mutate-twice-${seed}`,
          p1: { hand: [FILLER], field: [{ def: TIMMY, counters: { plague: 1 } }], backrow: [{ def: MUTATE, radiant: true }] },
          p2: { hand: [FILLER], field: [MENACE], health: 30 },
        });
        const timmy = s.card(TIMMY);
        s.activate(MUTATE, { targets: at(timmy) });
        const targets = attacksBy(s.lastEvents, timmy);
        if (targets[0] !== "hero-p2") {
          diedFirst += 1;
          expect(targets, `seed ${seed}`).toHaveLength(1);
          s.expectInZone(timmy, "graveyard");
        } else {
          expect(targets.length, `seed ${seed}`).toBeGreaterThanOrEqual(1);
          expect(targets.length, `seed ${seed}`).toBeLessThanOrEqual(2);
        }
      }
      expect(diedFirst).toBeGreaterThan(0);
    });

    it("R386 a Degrade draws 3 and attacks once", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: VANILLA, counters: { plague: 1 } }], backrow: [{ def: MUTATE, radiant: true, counters: { plague: 1 } }], library: lib(4) },
        p2: { hand: [FILLER], health: 30 },
      });
      stepParam(s.card(MUTATE), "draw", -1);
      stepParam(s.card(MUTATE), "attacks", -1);

      s.activate(MUTATE, { targets: at(s.card(MUTATE)) });
      expect(drawsBy(s.lastEvents, "p1")).toBe(3);
      s.activate(MUTATE, { targets: at(s.card(VANILLA)) });
      expect(attacksBy(s.lastEvents, s.card(VANILLA))).toEqual(["hero-p2"]);
    });
  });
});
