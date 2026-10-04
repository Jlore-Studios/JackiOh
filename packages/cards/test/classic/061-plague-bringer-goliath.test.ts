// C #61 Plague Bringer Goliath — SPEC §8.6 row 61, BUILD M9 Classic row C 61: "Tribute 1, Rush, Trample:
// can't be played without a Unit to Tribute, and may take the tributed Unit's zone on a full board
// (R391); Cry: place 3 Plague Tokens as three placements, all on the one permanent a single prompt
// names (R661), on any permanent either side, face-down ones included (a face-down option carries only
// its id, R177), then draw 1; Trample's excess hits the hero (R63); radiant 14/14: draw 3; its tuned
// numbers (tokens, draw) read through `param()` (R386)".

import { hashState, legalActions, reduce, stepParam, type CardInstance, type GameState } from "@jackioh/engine";
import type { ActionBody, GameEvent, PlayerId, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/061-plague-bringer-goliath";

const GOLIATH = "classic-061";
const CRAWLER = "classic-053"; // (1) Unit: whenever Plague Tokens are placed on this, draw 1.
const VANILLA = "core-008"; // (1) Unit 4/4.
const MENACE = "core-019"; // (3) Unit 9/9 Taunt.
const PAWN = "core-096"; // (1) Trap: answers only an attack that would be lethal.
const MANA_WELL = "core-006"; // (3) Field Spell.
const ANCHOR = "core-010"; // (0) Spell (§2.5).
const X = "core-020"; // library filler.

type Play = Extract<ActionBody, { type: "play" }>;

function lib(n: number): string[] {
  return Array.from({ length: n }, () => X);
}

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`missing: ${what}`);
  return value;
}

function optionIds(s: Scenario): string[] {
  return must(s.state.pending, "an open prompt").options.flatMap((option) =>
    option.selection.pick === "instance" ? [option.selection.instanceId] : [],
  );
}

function drawsBy(events: readonly GameEvent[], player: PlayerId): number {
  return events.filter((event) => event.type === "drawn" && event.player === player).length;
}

function pick(card: CardInstance): Selection[] {
  return [{ pick: "instance", instanceId: card.id }];
}

/** Goliath in hand with a Vanilla to tribute, and a board to place on. */
function ready(radiantFace = false, extra: { p2Backrow?: readonly (string | { def: string; faceUp?: boolean })[]; library?: number } = {}): Scenario {
  return scenario({
    p1: { hand: [{ def: GOLIATH, radiant: radiantFace }, ANCHOR], field: [VANILLA], library: lib(extra.library ?? 5) },
    p2: { hand: [ANCHOR], field: [MENACE], backrow: [...(extra.p2Backrow ?? [])] },
  });
}

function playGoliath(s: Scenario): CardInstance {
  const vanilla = s.card(VANILLA);
  s.play(GOLIATH, { tributes: [vanilla.id] });
  return s.card(GOLIATH);
}

describe("C #61 Plague Bringer Goliath", () => {
  it("declares Tribute 1, its two numbers, and one script on both faces; Rush and Trample are printed", () => {
    expect(def.id).toBe(GOLIATH);
    expect(base.staticFlags).toEqual({ tribute: 1 });
    expect(def.params).toEqual([
      { key: "tokens", base: 3, radiant: 3, better: "up", step: 1, min: 1 },
      { key: "draw", base: 1, radiant: 3, better: "up", step: 1, min: 1 },
    ]);
    expect(def.base.keywords).toEqual([{ kind: "Rush" }, { kind: "Trample" }]);
    expect(def.radiant.keywords).toEqual([{ kind: "Rush" }, { kind: "Trample" }]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("R101 can't be played without a Unit to Tribute: legalActions offers no play, and the play is refused", () => {
      const s = scenario({ p1: { hand: [GOLIATH, ANCHOR] }, p2: { hand: [ANCHOR], field: [MENACE] } });
      const goliath = s.card(GOLIATH);

      expect(legalActions(s.state, "p1").some((action) => action.type === "play" && action.instanceId === goliath.id)).toBe(false);
      expect(() => s.play(goliath)).toThrow(/Tribute/);
    });

    it("§6.3 the Tribute is paid with one of your Units, a death; the Goliath enters a 7/7 with Rush and Trample", () => {
      const s = ready();
      const vanilla = s.card(VANILLA);

      const goliath = playGoliath(s);

      s.expectInZone(vanilla, "graveyard");
      s.expectStats(goliath, { attack: 7, health: 7 });
      expect(s.stats(goliath).keywords.map((keyword) => keyword.kind)).toEqual(["Rush", "Trample"]);
      s.expectEvents("destroyed", "cardPlayed");
    });

    it("R391 on a full unit row it may take the zone its own Tribute empties", () => {
      const s = scenario({
        p1: { hand: [GOLIATH, ANCHOR], field: [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA], library: lib(3) },
        p2: { hand: [ANCHOR] },
      });
      const third = must(s.unit("p1", 3), "lane 3");
      const plays = legalActions(s.state, "p1").filter((action): action is Play => action.type === "play" && action.instanceId === s.card(GOLIATH).id);
      expect(plays.some((play) => play.zone?.lane === 3 && play.tributes?.includes(third.id) === true)).toBe(true);

      s.play(GOLIATH, { zone: 3, tributes: [third.id] });

      expect(s.unit("p1", 3)?.defId).toBe(GOLIATH);
    });

    it("Cry: three placements on the one permanent a single prompt names, over every permanent on either side, itself included", () => {
      const s = ready(false, { p2Backrow: [MANA_WELL] });
      const goliath = playGoliath(s);
      const menace = s.card(MENACE);
      const well = s.card(MANA_WELL);

      expect(s.state.pending?.playerId).toBe("p1");
      expect(s.state.pending?.kind).toBe("target");
      expect(new Set(optionIds(s))).toEqual(new Set([goliath.id, menace.id, well.id]));
      // One answer puts all three on the pick: no second prompt opens.
      s.answer(menace.id);

      expect(s.state.pending).toBeNull();
      expect(s.card(menace).counters.plague).toBe(3);
      expect(s.card(well).counters.plague ?? 0).toBe(0);
      expect(s.card(goliath).counters.plague ?? 0).toBe(0);
      const placed = s.events.filter((event) => event.type === "counterChanged" && event.placed !== undefined);
      expect(placed).toHaveLength(3);
    });

    it("R661 no spreading: one answer puts all three on the pick, three placements of 1", () => {
      const s = ready();
      playGoliath(s);
      const menace = s.card(MENACE);

      s.answer(menace.id);

      expect(s.state.pending).toBeNull();
      expect(s.card(menace).counters.plague).toBe(3);
      expect(
        s.events.flatMap((event) => (event.type === "counterChanged" && event.placed !== undefined ? [event.placed] : [])),
      ).toEqual([1, 1, 1]);
    });

    it("R113 then draw 1: the draw waits for the one answer", () => {
      const s = ready();
      playGoliath(s);
      const menace = s.card(MENACE);

      s.answer(menace.id);

      expect(s.state.pending).toBeNull();
      expect(drawsBy(s.lastEvents, "p1")).toBe(1);
      const kinds = s.lastEvents.flatMap((event) => (event.type === "counterChanged" || event.type === "drawn" ? [event.type] : []));
      expect(kinds).toEqual(["counterChanged", "counterChanged", "counterChanged", "drawn"]);
    });

    it("R177 a face-down enemy trap is an option by its id alone, and placing on it never names it to you", () => {
      const s = ready(false, { p2Backrow: [{ def: PAWN, faceUp: false }] });
      playGoliath(s);
      const trap = s.card(PAWN);

      expect(optionIds(s)).toContain(trap.id);
      const mine = must(s.view("p1").pending, "p1's view of the prompt");
      if (!mine.forYou) throw new Error("the prompt is p1's");
      const option = must(mine.options.find((entry) => entry.instanceId === trap.id || entry.key.includes(trap.id)), "the trap's option");
      expect(option.defId).toBeUndefined();
      expect(JSON.stringify(s.view("p1"))).not.toContain(PAWN);
      expect(JSON.stringify(s.view("p1"))).not.toContain("My Pawn");

      s.answer(trap.id);

      expect(s.state.pending).toBeNull();
      expect(s.card(trap).counters.plague).toBe(3);
      expect(JSON.stringify(s.view("p1"))).not.toContain(PAWN);
    });

    it("§10.6 the opponent sees only that a prompt is open", () => {
      const s = ready();
      playGoliath(s);

      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
    });

    it("each placement is its own: a C #53 Plague Crawler that takes all three draws three times", () => {
      const s = scenario({
        p1: { hand: [GOLIATH, ANCHOR], field: [VANILLA, CRAWLER], library: lib(6) },
        p2: { hand: [ANCHOR] },
      });
      playGoliath(s);
      const crawler = s.card(CRAWLER);

      s.answer(crawler.id);

      // Three Crawler draws and the Goliath's own.
      expect(drawsBy(s.events, "p1")).toBe(4);
    });

    it("§9.3 the open prompt survives a JSON round trip and finishes as the live one does", () => {
      const s = ready();
      playGoliath(s);
      const menace = s.card(MENACE);

      const revived = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(revived).toEqual(s.state);
      const choice = must(revived.pending, "the placement prompt");
      const result = reduce(revived, { type: "answer", playerId: "p1", choiceId: choice.id, selection: pick(menace), nonce: "goliath-json-0" });
      expect(result.error).toBeUndefined();
      s.answer(menace.id);

      expect(result.state.pending).toBeNull();
      expect(hashState(result.state)).toBe(hashState(s.state));
    });

    it("R63 Trample: with Rush it attacks a Unit the turn it enters, and the excess hits the hero", () => {
      const s = scenario({
        p1: { hand: [GOLIATH, ANCHOR], field: [VANILLA], library: lib(3) },
        p2: { hand: [ANCHOR], field: [{ def: VANILLA, lane: 2 }], health: 20 },
      });
      const theirs = must(s.unit("p2", 2), "p2's Vanilla");
      const goliath = playGoliath(s);
      s.answer(goliath.id);

      s.attack(goliath, theirs);

      s.expectInZone(theirs, "graveyard");
      s.expectHealth("p2", 17);
    });

    it("§2.4 a full hand burns the draw; an empty deck makes it fatigue", () => {
      const full = scenario({
        p1: { hand: [GOLIATH, ...Array.from({ length: 9 }, () => ANCHOR)], field: [VANILLA, CRAWLER], library: lib(6) },
        p2: { hand: [ANCHOR] },
      });
      playGoliath(full);
      const crawler = full.card(CRAWLER);
      full.answer(crawler.id);
      // 9 in hand after the play, and four draws (three of the Crawler's, one of its own): one fills it, three burn.
      expect(full.hand("p1")).toHaveLength(10);
      expect(full.events.filter((event) => event.type === "burned")).toHaveLength(3);

      const empty = ready(false, { library: 0 });
      const goliath = playGoliath(empty);
      empty.answer(goliath.id);
      expect(empty.lastEvents.filter((event) => event.type === "fatigue" && event.player === "p1")).toHaveLength(1);
    });

    it("R386 an Upgrade of its tokens places four; of its draw draws 2", () => {
      const s = ready();
      stepParam(s.card(GOLIATH), "tokens", 1);
      stepParam(s.card(GOLIATH), "draw", 1);
      playGoliath(s);
      const menace = s.card(MENACE);

      s.answer(menace.id);

      expect(s.state.pending).toBeNull();
      expect(s.card(menace).counters.plague).toBe(4);
      expect(drawsBy(s.lastEvents, "p1")).toBe(2);
    });
  });

  describe("radiant", () => {
    it("is a 14/14 with Rush and Trample, the same Tribute, three placements on the one pick, then draw 3", () => {
      const s = ready(true);
      expect(() => scenario({ p1: { hand: [{ def: GOLIATH, radiant: true }, ANCHOR] } }).play(GOLIATH)).toThrow(/Tribute/);
      const goliath = playGoliath(s);
      const menace = s.card(MENACE);

      s.expectStats(goliath, { attack: 14, health: 14 });
      s.answer(menace.id);

      expect(s.state.pending).toBeNull();
      expect(s.card(menace).counters.plague).toBe(3);
      expect(drawsBy(s.lastEvents, "p1")).toBe(3);
    });

    it("R386 a Degrade of its draw draws 2; of its tokens places two", () => {
      const s = ready(true);
      stepParam(s.card(GOLIATH), "draw", -1);
      stepParam(s.card(GOLIATH), "tokens", -1);
      playGoliath(s);
      const menace = s.card(MENACE);

      s.answer(menace.id);

      expect(s.state.pending).toBeNull();
      expect(s.card(menace).counters.plague).toBe(2);
      expect(drawsBy(s.lastEvents, "p1")).toBe(2);
    });
  });
});
