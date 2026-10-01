// C+ #69 Buff Billy — SPEC §8.7 row 69, BUILD M9 row C+ 69: X at least 1 (R348), chosen with the play;
// a 3X/3X from its `xStats`; its Cry Upgrades it X times, each its own draw from what fits it now
// (R386) — the stats split or an R21 keyword it lacks, never cost or X (it is an X-cost card, and on the
// field it costs its X, R396); the upgrades are `tuning`, kept by a copy (R57); summoned outside a play
// (a Recruit) it has X 0, arrives 0/0 and dies at the state check; radiant 7X/7X and 2X Upgrades.

import { describe, expect, it } from "vitest";
import {
  applyEffects,
  costNow,
  createRng,
  legalActions,
  makeContext,
  stateCheck,
  type CardInstance,
  type EngineSink,
} from "@jackioh/engine";
import type { Effect } from "@jackioh/engine";
import { recruit, summonCopy } from "@jackioh/engine/effects";
import type { GameEvent } from "@jackioh/shared";
import { base, def, radiant } from "../../src/scripts/classic-plus/069-buff-billy";
import { scenario, type Scenario } from "../_harness";

const BILLY = "classicplus-069";
const FILLER = "core-005";

function run(s: Scenario, effects: Effect[]): void {
  const state = s.state;
  const sink: EngineSink = { state, events: [], rng: createRng(state.seed, state.rngCursor) };
  applyEffects(effects, makeContext(sink, null, { controller: "p1" }));
  stateCheck(sink);
  state.rngCursor = sink.rng.cursor;
}

function upgrades(events: readonly GameEvent[]): Extract<GameEvent, { type: "upgraded" }>[] {
  return events.filter((event): event is Extract<GameEvent, { type: "upgraded" }> => event.type === "upgraded");
}

function billy(x: number, opts: { radiant?: boolean; seed?: string } = {}): { s: Scenario; unit: CardInstance } {
  const s = scenario({
    ...(opts.seed === undefined ? {} : { seed: opts.seed }),
    p1: { hand: [{ def: BILLY, radiant: opts.radiant === true }, FILLER], mana: 10 },
    p2: { hand: [FILLER] },
  });
  s.play(BILLY, { zone: 1, x });
  const unit = s.unit("p1", 1);
  if (unit === null) throw new Error("Buff Billy on the field");
  return { s, unit };
}

/** The stats the Upgrades added, summed from the events. */
function statsAdded(events: readonly GameEvent[]): { attack: number; health: number } {
  return upgrades(events).reduce(
    (sum, event) =>
      event.change.kind === "stats"
        ? { attack: sum.attack + event.change.attack, health: sum.health + event.change.health }
        : sum,
    { attack: 0, health: 0 },
  );
}

describe("C+ #69 Buff Billy", () => {
  it("prints an (X) Unit, Human, 3X/3X and Radiant 7X/7X", () => {
    expect(def).toMatchObject({ cost: "X", type: "Unit", tags: ["Human"] });
    expect(def.base.xStats).toEqual({ attack: 3, health: 3 });
    expect(def.radiant.xStats).toEqual({ attack: 7, health: 7 });
    expect(base.cry).toBeDefined();
    expect(radiant.cry).toBeDefined();
  });

  describe("base", () => {
    it("R348 X is chosen with the play, 1 up to your mana, and X = 0 is refused", () => {
      const s = scenario({ p1: { hand: [BILLY, FILLER] }, p2: { hand: [FILLER] } });
      const card = s.card(BILLY);
      const xs = legalActions(s.state, "p1").flatMap((action) =>
        action.type === "play" && action.instanceId === card.id && action.zone?.lane === 1 ? [action.x] : [],
      );
      expect(xs.sort()).toEqual([1, 2, 3, 4]);
      expect(() => s.play(BILLY, { zone: 1, x: 0 })).toThrow(/X must be at least 1/);
    });

    it("R386 played for X = 2 it is a 6/6 and its Cry Upgrades it twice, each its own draw, landing on its stats", () => {
      const { s, unit } = billy(2);
      const events = upgrades(s.events);
      expect(events).toHaveLength(2);
      expect(events.every((event) => event.instanceId === unit.id)).toBe(true);
      const added = statsAdded(s.events);
      s.expectStats(unit, { attack: 6 + added.attack, maxHealth: 6 + added.health });
      expect(s.card(unit).x).toBe(2);
    });

    it("R386 R396 every Upgrade is the stats split or a keyword it lacks, never cost or X; on the field it costs its X", () => {
      for (let n = 0; n < 12; n += 1) {
        const { s, unit } = billy(4, { seed: `billy-${n}` });
        const changes = upgrades(s.events).map((event) => event.change);
        expect(changes).toHaveLength(4);
        for (const change of changes) {
          expect(["stats", "keyword"]).toContain(change.kind);
          if (change.kind === "stats") expect(change.attack + change.health).toBe(4);
          if (change.kind === "keyword") expect(change.added).toBe(true);
        }
        const live = s.card(unit);
        expect(live.tuning?.x).toBeUndefined();
        expect(live.costMod).toBe(0);
        expect(costNow(s.state, live)).toBe(4);
      }
    });

    it("R21 a keyword it gains is one it lacks, from R21's pool, and it keeps it", () => {
      let checked = false;
      for (let n = 0; n < 20 && !checked; n += 1) {
        const { s, unit } = billy(3, { seed: `billy-kw-${n}` });
        const gained = upgrades(s.events).flatMap((event) => (event.change.kind === "keyword" ? [event.change.keyword.kind] : []));
        if (gained.length === 0) continue;
        expect(new Set(gained).size).toBe(gained.length);
        expect(s.stats(unit).keywords.map((keyword) => keyword.kind)).toEqual(expect.arrayContaining(gained));
        checked = true;
      }
      expect(checked).toBe(true);
    });

    it("R57 the Upgrades are its tuning, and a copy keeps them", () => {
      const { s, unit } = billy(3);
      const tuning = s.card(unit).tuning;
      expect(tuning).toBeDefined();
      run(s, [summonCopy({ of: { of: "instance", instanceId: unit.id }, lane: 2 })]);
      const copy = s.unit("p1", 2) ?? s.pile("p1", "graveyard").find((card) => card.defId === BILLY);
      expect(copy?.defId).toBe(BILLY);
      expect(copy?.tuning).toEqual(tuning);
    });

    it("B2.7 a Recruit has no X: it arrives 0/0 and dies at the state check", () => {
      const s = scenario({ p1: { hand: [FILLER], library: [BILLY] }, p2: { hand: [FILLER] } });
      run(s, [recruit({ filter: { defId: BILLY } })]);
      s.expectInZone(BILLY, "graveyard");
      expect(s.unit("p1", 1)).toBeNull();
    });

    it("R396 a Buff Billy played for 1 is a 3/3 with one Upgrade, costing (1) on the field", () => {
      const { s, unit } = billy(1);
      expect(upgrades(s.events)).toHaveLength(1);
      const added = statsAdded(s.events);
      s.expectStats(unit, { attack: 3 + added.attack, maxHealth: 3 + added.health });
      expect(costNow(s.state, s.card(unit))).toBe(1);
    });
  });

  describe("radiant", () => {
    it("played for X = 2 it is a 14/14 and its Cry Upgrades it 2X = 4 times", () => {
      const { s, unit } = billy(2, { radiant: true });
      expect(upgrades(s.events)).toHaveLength(4);
      const added = statsAdded(s.events);
      s.expectStats(unit, { attack: 14 + added.attack, maxHealth: 14 + added.health });
    });

    it("R386 its Upgrades are the stats split or a keyword, never cost or X", () => {
      const { s, unit } = billy(1, { radiant: true, seed: "billy-radiant" });
      const changes = upgrades(s.events).map((event) => event.change.kind);
      expect(changes).toHaveLength(2);
      expect(changes.every((kind) => kind === "stats" || kind === "keyword")).toBe(true);
      expect(s.card(unit).tuning?.x).toBeUndefined();
    });
  });
});
