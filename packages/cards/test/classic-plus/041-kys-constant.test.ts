// C+ #41 KY's Constant — SPEC §8.7 row 41, B3.4, R60, R81, R97, R129, R386, BUILD M9 row C+ 41.
// The setting itself is the engine's `setNumber` / `discoverNumber` (packages/engine/test/effects-tune.test.ts).

import { effectiveCost, hashState, legalActions, numbersOn, reduce, type CardInstance, type GameState } from "@jackioh/engine";
import type { Action } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { base, def, radiant } from "../../src/scripts/classic-plus/041-kys-constant";
import { scenario, type Scenario } from "../_harness";

const CONSTANT = "classicplus-041";
const TIMMY = "core-011"; // (1) 3/3: its cost is its one number that is not 3
const VANILLA = "core-008"; // (1) 4/4
const MENACE = "core-019"; // (3) 9/9 Taunt
const ARMORED = "core-025"; // (4) 7/7 Armor 7
const DUPLICATING = "core-012"; // (2) 3/4, Cry: summon a copy of this
const HIT_JOB = "core-016"; // (3) Spell: no number other than 3
const DIVIDEND = "core-024"; // (X) Spell: an X is never a number
const GIFT = "classicplus-042-1"; // (4) Field Spell with mana 1, discards 1, heal 5
const FILLER = "core-005";

function constant(hand: readonly string[], radiantFace = false, seed?: string): Scenario {
  return scenario({ ...(seed === undefined ? {} : { seed }), p1: { hand: [{ def: CONSTANT, radiant: radiantFace }, ...hand] }, p2: { hand: [FILLER] } });
}

function numbers(s: Scenario, card: string | CardInstance): Record<string, number> {
  return Object.fromEntries(numbersOn(s.state, s.card(card)).map((entry) => [entry.id, entry.value]));
}

function pick(s: Scenario, card: string): { pick: "instance"; instanceId: string }[] {
  return [{ pick: "instance", instanceId: s.card(card).id }];
}

/** The hand cards `legalActions` offers as the Constant's pick. */
function offered(s: Scenario): string[] {
  const constantId = s.card(CONSTANT).id;
  return legalActions(s.state, "p1").flatMap((action) =>
    action.type === "play" && action.instanceId === constantId
      ? (action.targets ?? []).flatMap((target) => (target.pick === "instance" ? [s.card(target.instanceId).defId] : []))
      : [],
  );
}

describe("C+ #41 KY's Constant", () => {
  it("R81 both faces declare one hand pick, never itself", () => {
    expect(def.id).toBe(CONSTANT);
    expect(base.targets).toEqual(radiant.targets);
    expect(base.targets).toEqual([{ kind: "hand", min: 1, max: 1, filter: { of: ["hand"], excludeSelf: true, check: "number" } }]);
  });

  describe("the pick (R81, R386)", () => {
    it("R386 offers only hand cards with a number other than 3: never one whose numbers are all 3, nor an X", () => {
      const s = constant([TIMMY, HIT_JOB, DIVIDEND, VANILLA]);
      expect(offered(s).sort()).toEqual([TIMMY, VANILLA].sort());
    });

    it("R386 an Immutable card has no number to change, so it is never offered", () => {
      const s = constant([VANILLA, TIMMY]);
      // As E38's grantKeywordCards leaves a hand card it makes Immutable.
      s.card(VANILLA).grantedKeywords.push({ kind: "Immutable" });
      expect(numbersOn(s.state, s.card(VANILLA))).toEqual([]);
      expect(offered(s)).toEqual([TIMMY]);
    });

    it("with no such hand card the Spell fizzles and still counts as played", () => {
      const s = constant([HIT_JOB]);
      s.play(CONSTANT);
      s.expectInZone(CONSTANT, "graveyard").expectEvents("cardPlayed", "cardResolved");
      expect(s.events.some((event) => event.type === "numberChanged")).toBe(false);
      expect(numbers(s, HIT_JOB)).toEqual({ cost: 3 });
    });
  });

  describe("base", () => {
    it("R129 one number not 3: Tempo Timmy's cost becomes (3), raised through costMod, with no random draw", () => {
      const s = constant([TIMMY]);
      const cursor = s.state.rngCursor;
      s.play(CONSTANT, { targets: pick(s, TIMMY) });
      expect(numbers(s, TIMMY)).toEqual({ cost: 3, attack: 3, health: 3 });
      expect(s.card(TIMMY).costMod).toBe(2);
      expect(effectiveCost(s.state, s.card(TIMMY))).toBe(3);
      expect(s.state.rngCursor).toBe(cursor);
    });

    it("R60 one random number of several becomes 3, any of them over seeds; the others stay", () => {
      const changed = new Set<string>();
      for (let n = 0; n < 20; n += 1) {
        const s = constant([VANILLA], false, `constant-${n}`);
        s.play(CONSTANT, { targets: pick(s, VANILLA) });
        const now = numbers(s, VANILLA);
        const moved = Object.entries({ cost: 1, attack: 4, health: 4 }).filter(([key, was]) => now[key] !== was);
        expect(moved).toHaveLength(1);
        expect(now[moved[0]?.[0] ?? ""]).toBe(3);
        changed.add(moved[0]?.[0] ?? "");
      }
      expect([...changed].sort()).toEqual(["attack", "cost", "health"]);
    });

    it("R386 it lowers as well: Midrange Menace's attack or health becomes 3", () => {
      const s = constant([MENACE]);
      s.play(CONSTANT, { targets: pick(s, MENACE) });
      const now = numbers(s, MENACE);
      expect([now.attack, now.health].sort()).toEqual([3, 9]);
    });

    it("R386 a numbered keyword and a declared number are numbers too", () => {
      const seen = new Set<string>();
      for (let n = 0; n < 24; n += 1) {
        const s = constant([ARMORED, GIFT], false, `constant-kinds-${n}`);
        const target = n % 2 === 0 ? ARMORED : GIFT;
        const before = numbers(s, target);
        s.play(CONSTANT, { targets: pick(s, target) });
        const now = numbers(s, target);
        for (const key of Object.keys(before)) if (now[key] !== before[key]) seen.add(key);
      }
      expect(seen.has("keyword:Armor")).toBe(true);
      expect([...seen].some((key) => key.startsWith("param:"))).toBe(true);
    });

    it("R386 the change is tuning: it stays as the card is played, and a copy keeps it", () => {
      for (let n = 0; n < 20; n += 1) {
        const s = constant([DUPLICATING, FILLER], false, `constant-copy-${n}`);
        s.play(CONSTANT, { targets: pick(s, DUPLICATING) });
        if (numbers(s, DUPLICATING).health !== 3) continue;
        s.play(DUPLICATING, { zone: 1 });
        // Cry: summon a copy of this — both the card and its copy are 3/3.
        s.expectStats(s.unit("p1", 1) ?? DUPLICATING, { attack: 3, health: 3 });
        s.expectStats(s.unit("p1", 2) ?? DUPLICATING, { attack: 3, health: 3 });
        return;
      }
      throw new Error("no seed changed the health");
    });

    it("R97 the opponent's view names neither the card nor the number", () => {
      const s = constant([TIMMY]);
      s.play(CONSTANT, { targets: pick(s, TIMMY) });
      const mine = s.view("p1").events.find((event) => event.type === "numberChanged");
      const theirs = s.view("p2").events.find((event) => event.type === "numberChanged");
      expect(mine?.type === "numberChanged" ? [mine.defId, mine.key, mine.value] : []).toEqual([TIMMY, "cost", 3]);
      expect(theirs?.type === "numberChanged" ? [theirs.instanceId, theirs.defId, theirs.key, theirs.value] : []).toEqual([
        "hidden",
        "hidden",
        "hidden",
        0,
      ]);
      expect(JSON.stringify(s.view("p2"))).not.toContain(TIMMY);
    });
  });

  describe("radiant", () => {
    it("Discover: up to 3 different numbers on the card, shown to you only; the chosen one becomes 3", () => {
      const s = constant([ARMORED], true);
      s.play(CONSTANT, { targets: pick(s, ARMORED) });
      const pending = s.state.pending;
      expect(pending?.kind).toBe("discover");
      expect(pending?.playerId).toBe("p1");
      expect(pending?.options).toHaveLength(3);
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
      const option = pending?.options.find((entry) => entry.key === "mode:health") ?? pending?.options[0];
      const key = option?.selection.pick === "mode" ? option.selection.option : "";
      s.answer(option?.key ?? "");
      expect(numbers(s, ARMORED)[key]).toBe(3);
    });

    it("R129 with no more than 3 numbers not 3, all are offered and nothing is drawn", () => {
      const s = constant([VANILLA], true);
      const cursor = s.state.rngCursor;
      s.play(CONSTANT, { targets: pick(s, VANILLA) });
      expect(s.state.pending?.options.map((option) => option.key).sort()).toEqual(["mode:attack", "mode:cost", "mode:health"]);
      expect(s.state.rngCursor).toBe(cursor);
      s.answer("mode:cost");
      expect(numbers(s, VANILLA)).toEqual({ cost: 3, attack: 4, health: 4 });
    });

    it("§9.3 paused on the Discover, the state survives JSON and answers to the same hash", () => {
      const s = constant([ARMORED], true);
      s.play(CONSTANT, { targets: pick(s, ARMORED) });
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      const action = {
        type: "answer",
        playerId: "p1",
        choiceId: s.state.pending?.id ?? "",
        selection: [s.state.pending?.options[0]?.selection],
        nonce: "constant-json",
      } as Action;
      const live = reduce(s.state, action);
      expect(live.error).toBeUndefined();
      expect(live.state.pending).toBeNull();
      const armored = live.state.players.p1.hand.find((card) => card.defId === ARMORED);
      expect(numbersOn(live.state, armored ?? s.card(ARMORED)).filter((entry) => entry.value === 3)).toHaveLength(1);
      expect(hashState(reduce(thawed, action).state)).toBe(hashState(live.state));
    });

    it("with no such hand card it fizzles, asking nothing", () => {
      const s = constant([HIT_JOB], true);
      s.play(CONSTANT);
      expect(s.state.pending).toBeNull();
      s.expectInZone(CONSTANT, "graveyard");
    });
  });
});
