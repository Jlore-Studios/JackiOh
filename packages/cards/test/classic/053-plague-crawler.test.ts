// C #53 Plague Crawler — SPEC §8.6 row 53, BUILD M9 Classic row C 53: "Cry: one Plague Token on another
// permanent (a declared target, either side, face-down included; with none it enters anyway); whenever
// Plague Tokens are placed on it, by either player, draw 1, once per placement however many tokens
// (C #27's doubling included); a face-down option carries only its id (R177) and the placement on it
// never names it to you; radiant 4/4: 2 tokens, draw 2; its tuned numbers (tokens, draw) read through
// `param()` (R386)".
//
// The C #27 Pestilent Slime case fuses a Crawler onto a Slime (R77), so the Slime's multiplier and the
// Crawler's trigger sit on one card; it needs C #27's script (cards-classic-a) registered.

import { createRng, legalActions, stepParam, subsystems, type CardInstance } from "@jackioh/engine";
import type { ActionBody, GameEvent, PlayerId, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/053-plague-crawler";

const CRAWLER = "classic-053";
const SLIME = "classic-027"; // (0) Unit 1/1: Plague Tokens placed on this are doubled.
const VANILLA = "core-008"; // (1) Unit 4/4.
const MENACE = "core-019"; // (3) Unit 9/9 Taunt; Radiant adds Immutable.
const MANA_WELL = "core-006"; // (3) Field Spell.
const UNLICENSED = "core-085"; // (2) Trap, answers only a permanent of a type its controller controls.
const FIENDER = "core-092"; // (2) Unit 5/7 Stack.
const FLOOD = "core-017"; // (4) Spell: Bounce all Units.
const FILLER = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
const ANCHOR = "core-010"; // (0) Spell, a card to keep a hand from auto-ending the turn (§2.5).
const X = "core-020"; // library filler.

type Play = Extract<ActionBody, { type: "play" }>;

function at(card: CardInstance): Selection[] {
  return [{ pick: "instance", instanceId: card.id }];
}

function lib(n: number): string[] {
  return Array.from({ length: n }, () => X);
}

/** Every placement so far: one entry per `counterChanged` that carries `placed`. */
function placements(s: Scenario): { id: string; value: number; placed: number }[] {
  return s.events.flatMap((event) =>
    event.type === "counterChanged" && event.counter === "plague" && event.placed !== undefined
      ? [{ id: event.instanceId, value: event.value, placed: event.placed }]
      : [],
  );
}

function drawsBy(events: readonly GameEvent[], player: PlayerId): number {
  return events.filter((event) => event.type === "drawn" && event.player === player).length;
}

function playsOf(s: Scenario, card: CardInstance, player: PlayerId = "p1"): Play[] {
  return legalActions(s.state, player).filter((action): action is Play => action.type === "play" && action.instanceId === card.id);
}

/** The cards legalActions offers as the play's target, each once (a play is listed once per zone). */
function offeredTargets(s: Scenario, card: CardInstance, player: PlayerId = "p1"): string[] {
  const ids = playsOf(s, card, player).flatMap((play) =>
    (play.targets ?? []).flatMap((target) => (target.pick === "instance" ? [target.instanceId] : [])),
  );
  return [...new Set(ids)];
}

function crawlers(s: Scenario, player: PlayerId): CardInstance[] {
  return [...s.hand(player)].filter((card) => card.defId === CRAWLER);
}

describe("C #53 Plague Crawler", () => {
  it("declares one target, another permanent on either side, its two numbers, and one script on both faces", () => {
    expect(def.id).toBe(CRAWLER);
    expect(base.targets).toEqual([
      { kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "backrow"], excludeSelf: true } },
    ]);
    expect(def.params).toEqual([
      { key: "tokens", base: 1, radiant: 2, better: "up", step: 1, min: 1 },
      { key: "draw", base: 1, radiant: 2, better: "up", step: 1, min: 1 },
    ]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("is a 2/2, and its Cry places 1 Plague Token on an enemy Unit you choose, as one placement", () => {
      const s = scenario({ p1: { hand: [CRAWLER, ANCHOR] }, p2: { hand: [ANCHOR], field: [VANILLA] } });
      const vanilla = s.card(VANILLA);

      s.play(CRAWLER, { targets: at(vanilla) });

      s.expectStats(CRAWLER, { attack: 2, health: 2 });
      expect(s.card(vanilla).counters.plague).toBe(1);
      expect(placements(s)).toEqual([{ id: vanilla.id, value: 1, placed: 1 }]);
    });

    it("R81 either side: your own Unit, or a Field Spell in your backrow, takes the token", () => {
      const s = scenario({ p1: { hand: [CRAWLER, CRAWLER, ANCHOR], field: [VANILLA], backrow: [MANA_WELL], library: lib(2) }, p2: { hand: [ANCHOR] } });
      const [first, second] = crawlers(s, "p1");
      if (first === undefined || second === undefined) throw new Error("two Crawlers in hand");

      s.play(first, { targets: at(s.card(VANILLA)) });
      s.play(second, { targets: at(s.card(MANA_WELL)) });

      expect(s.card(VANILLA).counters.plague).toBe(1);
      expect(s.card(MANA_WELL).counters.plague).toBe(1);
    });

    it("an Immutable permanent takes tokens too: a token is a counter, not a change of text", () => {
      const s = scenario({ p1: { hand: [CRAWLER, ANCHOR] }, p2: { hand: [ANCHOR], field: [{ def: MENACE, radiant: true }] } });

      s.play(CRAWLER, { targets: at(s.card(MENACE)) });

      expect(s.stats(MENACE).keywords.map((keyword) => keyword.kind)).toContain("Immutable");
      expect(s.card(MENACE).counters.plague).toBe(1);
    });

    it("'another': it never offers itself, and legalActions offers every other permanent", () => {
      const s = scenario({ p1: { hand: [CRAWLER, ANCHOR], field: [VANILLA] }, p2: { hand: [ANCHOR], field: [MENACE], backrow: [MANA_WELL] } });
      const crawler = s.card(CRAWLER);

      const offered = offeredTargets(s, crawler);

      expect(offered).not.toContain(crawler.id);
      expect(new Set(offered)).toEqual(new Set([s.card(VANILLA).id, s.card(MENACE).id, s.card(MANA_WELL).id]));
      expect(() => s.play(crawler, { targets: at(crawler) })).toThrow();
    });

    it("§3.2 R13 a card dormant under a Stack pile is not offered; the top of the pile is", () => {
      const s = scenario({ p1: { hand: [CRAWLER, ANCHOR] }, p2: { hand: [ANCHOR], field: [VANILLA, { def: FIENDER, stack: true }] } });
      const dormant = s.card(VANILLA);
      const top = s.card(FIENDER);

      expect(offeredTargets(s, s.card(CRAWLER))).toEqual([top.id]);
      expect(() => s.play(CRAWLER, { targets: at(dormant) })).toThrow();

      s.play(CRAWLER, { targets: at(top) });
      expect(s.card(top).counters.plague).toBe(1);
      expect(s.card(dormant).counters.plague).toBeUndefined();
    });

    it("with no other permanent on the field it enters anyway and places nothing", () => {
      const s = scenario({ p1: { hand: [CRAWLER, ANCHOR] }, p2: { hand: [ANCHOR] } });
      const crawler = s.card(CRAWLER);
      expect(playsOf(s, crawler).length).toBeGreaterThan(0);
      expect(offeredTargets(s, crawler)).toEqual([]);

      s.play(crawler);

      s.expectInZone(crawler, "field");
      expect(placements(s)).toEqual([]);
      expect(drawsBy(s.events, "p1")).toBe(0);
    });

    it("R177 a face-down enemy trap is offered by its id alone, and the placement on it never names it to you", () => {
      const s = scenario({ p1: { hand: [CRAWLER, ANCHOR] }, p2: { hand: [ANCHOR], backrow: [{ def: UNLICENSED, faceUp: false }] } });
      const trap = s.card(UNLICENSED);

      expect(offeredTargets(s, s.card(CRAWLER))).toEqual([trap.id]);
      expect(JSON.stringify(s.view("p1"))).not.toContain(UNLICENSED);

      s.play(CRAWLER, { targets: at(trap) });

      expect(s.card(trap).counters.plague).toBe(1);
      expect(s.card(trap).faceUp).not.toBe(true);
      expect(JSON.stringify(s.view("p1"))).not.toContain(UNLICENSED);
      // Its controller still reads it, tokens and all.
      expect(JSON.stringify(s.view("p2").you.backrow)).toContain(UNLICENSED);
    });

    it("whenever Plague Tokens are placed on it, it draws 1: a second Crawler's Cry on it", () => {
      const s = scenario({ p1: { hand: [CRAWLER, CRAWLER, ANCHOR], library: lib(3) }, p2: { hand: [ANCHOR] } });
      const [first, second] = crawlers(s, "p1");
      if (first === undefined || second === undefined) throw new Error("two Crawlers in hand");
      s.play(first);
      expect(drawsBy(s.events, "p1")).toBe(0);

      s.play(second, { targets: at(first) });

      expect(s.card(first).counters.plague).toBe(1);
      expect(drawsBy(s.lastEvents, "p1")).toBe(1);
    });

    it("by either player: the opponent's Crawler placing on yours draws you 1, and them nothing", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [ANCHOR], field: [CRAWLER], library: lib(3) },
        p2: { hand: [CRAWLER, ANCHOR], library: lib(3) },
      });
      const mine = s.unit("p1", 1);
      if (mine === null) throw new Error("p1's Crawler is on the field");

      s.play(crawlers(s, "p2")[0] as CardInstance, { targets: at(mine) });

      expect(drawsBy(s.lastEvents, "p1")).toBe(1);
      expect(drawsBy(s.lastEvents, "p2")).toBe(0);
    });

    it("once per placement however many tokens: a Radiant Crawler's placement of 2 on it draws 1", () => {
      const s = scenario({ p1: { hand: [{ def: CRAWLER, radiant: true }, ANCHOR], field: [CRAWLER], library: lib(3) }, p2: { hand: [ANCHOR] } });
      const onField = s.unit("p1", 1) as CardInstance;
      const radiantOne = s.hand("p1").find((card) => card.defId === CRAWLER) as CardInstance;

      s.play(radiantOne, { targets: at(onField) });

      expect(s.card(onField).counters.plague).toBe(2);
      expect(placements(s)).toEqual([{ id: onField.id, value: 2, placed: 2 }]);
      expect(drawsBy(s.lastEvents, "p1")).toBe(1);
    });

    it("C #27 a Crawler fused onto a Pestilent Slime takes 2 from a placement of 1, and still draws once", () => {
      const s = scenario({ p1: { hand: [CRAWLER, ANCHOR], field: [SLIME], library: lib(3) }, p2: { hand: [CRAWLER, ANCHOR] } });
      const slime = s.card(SLIME);
      const sink = { state: s.state, events: [], rng: createRng(s.state.seed, s.state.rngCursor) };
      subsystems.fuse(sink, { ingredients: [s.hand("p1").find((card) => card.defId === CRAWLER) as CardInstance], target: slime });
      s.endTurn();

      s.play(crawlers(s, "p2")[0] as CardInstance, { targets: at(slime) });

      expect(s.card(slime).counters.plague).toBe(2);
      expect(placements(s).filter((entry) => entry.id === slime.id)).toEqual([{ id: slime.id, value: 2, placed: 2 }]);
      expect(drawsBy(s.lastEvents, "p1")).toBe(1);
    });

    it("its own Cry draws it nothing: the placement is on another permanent", () => {
      const s = scenario({ p1: { hand: [CRAWLER, ANCHOR], field: [VANILLA], library: lib(3) }, p2: { hand: [ANCHOR] } });

      s.play(CRAWLER, { targets: at(s.card(VANILLA)) });

      expect(drawsBy(s.events, "p1")).toBe(0);
    });

    it("§2.4 a full hand burns the draw", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: Array.from({ length: 10 }, () => FILLER), field: [CRAWLER], library: lib(3) },
        p2: { hand: [CRAWLER, ANCHOR] },
      });
      const mine = s.unit("p1", 1) as CardInstance;

      s.play(crawlers(s, "p2")[0] as CardInstance, { targets: at(mine) });

      expect(s.hand("p1")).toHaveLength(10);
      expect(s.lastEvents.filter((event) => event.type === "burned")).toHaveLength(1);
    });

    it("§2.4 with an empty deck the draw is fatigue", () => {
      const s = scenario({ p1: { hand: [CRAWLER, ANCHOR], field: [CRAWLER], health: 20 }, p2: { hand: [ANCHOR] } });
      const onField = s.unit("p1", 1) as CardInstance;
      const inHand = s.hand("p1").find((card) => card.defId === CRAWLER) as CardInstance;

      s.play(inHand, { targets: at(onField) });

      expect(s.lastEvents.filter((event) => event.type === "fatigue" && event.player === "p1")).toHaveLength(1);
      expect(s.state.players.p1.hero.health).toBeLessThan(20);
    });

    it("R78 its tokens go when it leaves the field: bounced, it comes back with none", () => {
      const s = scenario({
        p1: { hand: [CRAWLER, ANCHOR], field: [{ def: CRAWLER, counters: { plague: 3 } }], library: lib(3) },
        p2: { hand: [FLOOD, ANCHOR] },
        active: "p2",
      });
      const onField = s.unit("p1", 1) as CardInstance;

      s.play(FLOOD);

      s.expectInZone(onField, "hand");
      expect(s.card(onField).counters.plague).toBeUndefined();
    });

    it("R386 an Upgrade of its tokens places 2 in one placement; of its draw draws 2", () => {
      const s = scenario({ p1: { hand: [CRAWLER, CRAWLER, ANCHOR], library: lib(4) }, p2: { hand: [ANCHOR], field: [VANILLA] } });
      const [first, second] = crawlers(s, "p1");
      if (first === undefined || second === undefined) throw new Error("two Crawlers in hand");
      stepParam(first, "tokens", 1);
      stepParam(first, "draw", 1);

      s.play(first, { targets: at(s.card(VANILLA)) });
      expect(s.card(VANILLA).counters.plague).toBe(2);
      expect(placements(s)).toEqual([{ id: s.card(VANILLA).id, value: 2, placed: 2 }]);

      s.play(second, { targets: at(first) });
      expect(drawsBy(s.lastEvents, "p1")).toBe(2);
    });
  });

  describe("radiant", () => {
    it("is a 4/4, and its Cry places 2 Plague Tokens on another permanent as one placement", () => {
      const s = scenario({ p1: { hand: [{ def: CRAWLER, radiant: true }, ANCHOR] }, p2: { hand: [ANCHOR], field: [VANILLA] } });
      const vanilla = s.card(VANILLA);

      s.play(CRAWLER, { targets: at(vanilla) });

      s.expectStats(CRAWLER, { attack: 4, health: 4 });
      expect(placements(s)).toEqual([{ id: vanilla.id, value: 2, placed: 2 }]);
    });

    it("whenever Plague Tokens are placed on it, it draws 2", () => {
      const s = scenario({ p1: { hand: [CRAWLER, ANCHOR], field: [{ def: CRAWLER, radiant: true }], library: lib(3) }, p2: { hand: [ANCHOR] } });
      const onField = s.unit("p1", 1) as CardInstance;

      s.play(s.hand("p1").find((card) => card.defId === CRAWLER) as CardInstance, { targets: at(onField) });

      expect(drawsBy(s.lastEvents, "p1")).toBe(2);
    });

    it("R177 a face-down enemy trap takes its 2 tokens without being named to you", () => {
      const s = scenario({ p1: { hand: [{ def: CRAWLER, radiant: true }, ANCHOR] }, p2: { hand: [ANCHOR], backrow: [{ def: UNLICENSED, faceUp: false }] } });
      const trap = s.card(UNLICENSED);

      s.play(CRAWLER, { targets: at(trap) });

      expect(s.card(trap).counters.plague).toBe(2);
      expect(JSON.stringify(s.view("p1"))).not.toContain(UNLICENSED);
    });

    it("R386 a Degrade of its draw draws 1; of its tokens places 1", () => {
      const s = scenario({ p1: { hand: [CRAWLER, { def: CRAWLER, radiant: true }, ANCHOR], library: lib(4) }, p2: { hand: [ANCHOR], field: [VANILLA] } });
      const radiantOne = s.hand("p1").find((card) => card.defId === CRAWLER && card.radiant) as CardInstance;
      const baseOne = s.hand("p1").find((card) => card.defId === CRAWLER && !card.radiant) as CardInstance;
      stepParam(radiantOne, "tokens", -1);
      stepParam(radiantOne, "draw", -1);

      s.play(radiantOne, { targets: at(s.card(VANILLA)) });
      expect(placements(s)).toEqual([{ id: s.card(VANILLA).id, value: 1, placed: 1 }]);

      s.play(baseOne, { targets: at(radiantOne) });
      expect(drawsBy(s.lastEvents, "p1")).toBe(1);
    });
  });
});
