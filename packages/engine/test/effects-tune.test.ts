// Degrade and Upgrade (docs/classic-sets.md B3.4; R386, R440, R442) and KY's Constant's number set
// outright (Classic+ #41): every row of the menu with its bounds, the draw (the row, then the item),
// Immutable, "N times", random picks over hidden piles and their cues, R242's order, what the views
// show and hide (R177, R311), and how the changes ride the card — through leaving the field (R78),
// onto a copy (R57), into a Fuse (R102), and not through a Transform.

import type { GameEvent, PlayerId, TuningChange } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { TUNE_COST_CAP } from "../src/config";
import {
  applicableChanges,
  degrade,
  setNumber,
  summonCopy,
  shuffleCopiesOfSelf,
  shuffleInto,
  transform,
  tuneOnce,
  upgrade,
} from "../src/effects";
import { draw as drawCards } from "../src/draw";
import { printedEcho } from "../src/echo";
import { statsWithBuffs, unitView } from "../src/layers";
import { effectiveCost } from "../src/mana";
import { numbersOn } from "../src/numbers";
import { paramValue } from "../src/params";
import { makeContext, type HookOptions } from "../src/resolve";
import type { Effect } from "../src/script";
import { newInstance, type CardInstance, type GameState } from "../src/state";
import { fuse } from "../src/subsystems/fuse";
import { tunedCount, xOf } from "../src/tuning";
import { HIDDEN_ID, viewFor } from "../src/viewFor";
import { moveToZone, placeOnField } from "../src/zones";
import { plain } from "./fixtures/combat";
import { eventsOfType, inHand, put, setLibrary, sinkFor, slot } from "./fixtures/harness";
import {
  activator,
  billy,
  body,
  brittleUnit,
  constant,
  dearBody,
  echoBolt,
  freeBody,
  instanceGame,
  numbered,
  numberedBody,
  shackled,
  stoic,
  tributer,
  xBolt,
} from "./fixtures/instanceData";

function game(seed = "tune"): GameState {
  const state = instanceGame(seed);
  state.turn = 3;
  state.active = "p1";
  state.phase = "main";
  return state;
}

/** Apply one effect as `resolve.ts` does and hand back its events; the rng cursor is kept. */
function run(state: GameState, effect: Effect, options: HookOptions & { self?: CardInstance | null } = {}): GameEvent[] {
  const { self = null, ...hook } = options;
  const sink = sinkFor(state);
  effect.apply(makeContext(sink, self, { controller: "p1", ...hook }));
  state.rngCursor = sink.rng.cursor;
  return sink.events;
}

/** One application at a time, as `tuneOnce` makes it. */
function once(state: GameState, card: CardInstance, direction: "degrade" | "upgrade"): GameEvent[] {
  const sink = sinkFor(state);
  tuneOnce(makeContext(sink, null, { controller: "p1" }), card, direction);
  state.rngCursor = sink.rng.cursor;
  return sink.events;
}

function changes(events: readonly GameEvent[]): TuningChange[] {
  return events.flatMap((event) => (event.type === "degraded" || event.type === "upgraded" ? [event.change] : []));
}

function card(state: GameState, defId: string, player: PlayerId = "p1"): CardInstance {
  const [made] = inHand(state, defId, player);
  if (made === undefined) throw new Error("no card");
  return made;
}

describe("B3.4 the menu, row by row (R386)", () => {
  it("R386 cost: a Degrade adds (1) up to (4), an Upgrade takes (1) down to (0), each by costMod", () => {
    const state = game();
    const spell = card(state, constant.id);
    expect(applicableChanges(state, spell, "degrade")).toEqual(["cost"]);
    const degraded = run(state, degrade({ instanceId: spell.id, times: 4 }));
    // 1 → 2 → 3 → 4, and at (4) nothing is left to change: the fourth application is cued `none`
    // because the card is in a hand the other player may not read (R440).
    expect(changes(degraded)).toEqual([
      { kind: "cost", delta: 1 },
      { kind: "cost", delta: 1 },
      { kind: "cost", delta: 1 },
      { kind: "none" },
    ]);
    expect(spell.costMod).toBe(3);
    expect(effectiveCost(state, spell)).toBe(TUNE_COST_CAP);
    expect(applicableChanges(state, spell, "degrade")).toEqual([]);

    const upgraded = run(state, upgrade({ instanceId: spell.id, times: 5 }));
    expect(changes(upgraded).map((change) => change.kind)).toEqual(["cost", "cost", "cost", "cost", "none"]);
    expect(effectiveCost(state, spell)).toBe(0);
  });

  it("R386 cost is never an X-cost card's: its row is its X instead", () => {
    const state = game();
    const bolt = card(state, xBolt.id);
    expect(applicableChanges(state, bolt, "degrade")).toEqual(["x"]);
    expect(applicableChanges(state, bolt, "upgrade")).toEqual(["x"]);
  });

  it("R386 stats: a Degrade's −4 split floors attack at 0 and current health at 1, the floors' share lost", () => {
    for (let seed = 1; seed <= 25; seed += 1) {
      const state = game(`stats-${seed}`);
      const unit = put(state, dearBody.id, slot("p1", "units", 1));
      unit.damage = 1;
      // (4) already: the stats are the only row, so each application is one draw, the split.
      expect(applicableChanges(state, unit, "degrade")).toEqual(["stats"]);
      const before = unitView(state, unit);
      const [change] = changes(once(state, unit, "degrade"));
      if (change?.kind !== "stats") throw new Error("expected a stats change");
      const after = unitView(state, unit);
      expect(after.attack).toBe(before.attack + change.attack);
      expect(after.health).toBe(before.health + change.health);
      expect(after.attack).toBeGreaterThanOrEqual(0);
      expect(after.health).toBeGreaterThanOrEqual(1);
      // A 2/1 (2/2 with 1 damage) has 2 attack and 0 health above the floors to give.
      expect(-change.attack).toBeLessThanOrEqual(2);
      expect(change.health).toBe(0);
      expect(unit.damage).toBe(1);
    }
  });

  it("R386 stats: an Upgrade's +4 split moves max health on the field and the face a hand card will enter with", () => {
    for (let seed = 1; seed <= 10; seed += 1) {
      const state = game(`up-${seed}`);
      const unit = put(state, dearBody.id, slot("p1", "units", 1));
      unit.damage = 1;
      const [change] = changes(once(state, unit, "upgrade"));
      if (change?.kind === "stats") {
        expect(change.attack + change.health).toBe(4);
        expect(unitView(state, unit).maxHealth).toBe(2 + change.health);
        expect(unitView(state, unit).health).toBe(1 + change.health);
      }
      const held = card(state, dearBody.id);
      const [inHandChange] = changes(once(state, held, "upgrade"));
      if (inHandChange?.kind === "stats") {
        expect(statsWithBuffs(state, held)).toEqual({ attack: 2 + inHandChange.attack, maxHealth: 2 + inHandChange.health });
      }
    }
  });

  it("R386 keyword: a Degrade takes one the card has of its own, never a harmful one, nor one its position lends", () => {
    for (let seed = 1; seed <= 25; seed += 1) {
      const state = game(`kw-${seed}`);
      const unit = put(state, shackled.id, slot("p1", "units", 1));
      unit.position = "DEF";
      unit.costMod = TUNE_COST_CAP - 1; // no cost row
      unit.damage = 1;
      unit.tuning = { attack: -2 }; // 0/1: no stats row either
      expect(applicableChanges(state, unit, "degrade")).toEqual(["keyword"]);
      const [change] = changes(once(state, unit, "degrade"));
      expect(change).toEqual({ kind: "keyword", keyword: { kind: "Rush" }, added: false });
      const kinds = unitView(state, unit).keywords.map((keyword) => keyword.kind);
      expect(kinds).toContain("Can't attack");
      expect(kinds).toContain("Taunt");
      expect(kinds).not.toContain("Rush");
      // Nothing left of its own but the harmful one: the row is gone.
      expect(applicableChanges(state, unit, "degrade")).toEqual([]);
    }
  });

  it("R386 keyword: a removed keyword goes whether it was printed, added by an Upgrade or granted", () => {
    const state = game();
    const unit = put(state, shackled.id, slot("p1", "units", 1));
    unit.grantedKeywords = [{ kind: "Rush" }, { kind: "Pierce" }];
    unit.tuning = { addKeywords: [{ kind: "Rush" }] };
    const sink = sinkFor(state);
    const ctx = makeContext(sink, null, { controller: "p1" });
    // Draw until the keyword row takes Rush, then read what is left.
    for (let i = 0; i < 40 && unitView(state, unit).keywords.some((k) => k.kind === "Rush"); i += 1) {
      tuneOnce(ctx, unit, "degrade");
    }
    expect(unitView(state, unit).keywords.some((k) => k.kind === "Rush")).toBe(false);
    expect(unit.tuning?.removeKeywords).toContain("Rush");
    expect(unit.tuning?.addKeywords ?? []).not.toContainEqual({ kind: "Rush" });
    expect(unit.grantedKeywords).not.toContainEqual({ kind: "Rush" });
  });

  it("R386 keyword: an Upgrade adds one from R21's pool that a Unit lacks, never to a Spell or a Vanilla unit", () => {
    for (let seed = 1; seed <= 15; seed += 1) {
      const state = game(`add-${seed}`);
      const unit = put(state, freeBody.id, slot("p1", "units", 1));
      expect(applicableChanges(state, unit, "upgrade")).toEqual(["stats", "keyword"]);
      const change = changes(once(state, unit, "upgrade"))[0];
      if (change?.kind === "keyword") {
        expect(change.added).toBe(true);
        expect(unit.tuning?.addKeywords).toEqual([change.keyword]);
        expect(unitView(state, unit).keywords).toContainEqual(change.keyword);
      }
    }
    const state = game();
    expect(applicableChanges(state, card(state, constant.id), "upgrade")).toEqual(["cost"]);
    const silenced = put(state, freeBody.id, slot("p1", "units", 2));
    silenced.vanilla = true;
    expect(applicableChanges(state, silenced, "upgrade")).toEqual(["stats"]);
  });

  it("R386 X: printed Armor, Lucky and Spell Damage move by 1, more being better, never below 1", () => {
    const state = game();
    const unit = card(state, numberedBody.id);
    unit.costMod = TUNE_COST_CAP - 1;
    expect(applicableChanges(state, unit, "degrade")).toEqual(["stats", "keyword", "x"]);
    const numbers = (): Record<string, number> =>
      Object.fromEntries(numbersOn(state, unit).filter((n) => n.ref.kind === "keyword").map((n) => [n.label, n.value]));
    expect(numbers()).toEqual({ Armor: 2, Lucky: 1, "Spell Damage": 1 });
    // The X row alone, by hand: Armor is the only one a Degrade can move.
    unit.tuning = { x: { Armor: -1 } };
    expect(numbers()).toEqual({ Armor: 1, Lucky: 1, "Spell Damage": 1 });
    expect(statsOfKeywords(state, unit)).toContainEqual({ kind: "Armor", n: 1 });
    unit.tuning = { x: { Lucky: 2, "Spell Damage": 1 } };
    expect(numbers()).toEqual({ Armor: 2, Lucky: 3, "Spell Damage": 2 });
  });

  it("R386 X: Echo, Activate and Tribute read through their tuning; Tribute is less-is-better", () => {
    const state = game();
    const echo = card(state, echoBolt.id);
    const active = card(state, activator.id);
    const tribute = card(state, tributer.id);
    const valueOf = (c: CardInstance, key: string): number | undefined => numbersOn(state, c).find((n) => n.label === key)?.value;
    expect([valueOf(echo, "Echo"), valueOf(active, "Activate"), valueOf(tribute, "Tribute")]).toEqual([1, 2, 2]);

    // Upgrade's X row: one better each — Echo 2, Activate 3, Tribute 1.
    for (const [c, key] of [[echo, "Echo"], [active, "Activate"], [tribute, "Tribute"]] as const) {
      for (let i = 0; i < 60 && (c.tuning?.x?.[key] ?? 0) === 0; i += 1) once(state, c, "upgrade");
    }
    expect([valueOf(echo, "Echo"), valueOf(active, "Activate"), valueOf(tribute, "Tribute")]).toEqual([2, 3, 1]);
    expect(printedEcho(echo)).toBe(2);
    expect(tunedCount(tribute, "Tribute", 2)).toBe(1);
    // Tribute 1 is as good as it gets: no Upgrade X item is left on it, and a Degrade raises it.
    expect(tribute.tuning?.x?.Tribute).toBe(-1);
  });

  it("R386 X: an X-cost card's X counts 1 more or less when it resolves, never below 1", () => {
    const state = game();
    const spell = card(state, xBolt.id);
    run(state, degrade({ instanceId: spell.id, times: 2 }));
    expect(spell.tuning?.x).toEqual({ X: -2 });
    spell.x = 3;
    expect(xOf(spell)).toBe(1);
    spell.x = 5;
    expect(xOf(spell)).toBe(3);
    // The context a resolution runs in reads it (`resolve.makeContext`).
    expect(makeContext(sinkFor(state), spell).x).toBe(3);
  });

  it("R386 X: a Buff Billy on the field is its X in stats, and an Upgrade of its X grows it (B2.7)", () => {
    const state = game();
    const unit = newInstance(state, billy.id, "p1", { z: "hand", player: "p1" });
    unit.x = 2;
    placeOnField(state, unit, slot("p1", "units", 1));
    expect(unitView(state, unit)).toMatchObject({ attack: 6, maxHealth: 6 });
    unit.tuning = { x: { X: 1 } };
    expect(unitView(state, unit)).toMatchObject({ attack: 9, maxHealth: 9 });
    // On the field its X is known, so a Degrade of it needs room above 1.
    unit.x = 1;
    unit.tuning = undefined;
    expect(applicableChanges(state, unit, "degrade")).not.toContain("x");
  });

  it("R386 X: a Brittle count in force moves itself (never below 1); a printed one not yet started moves the number it starts at", () => {
    const state = game();
    const unit = put(state, brittleUnit.id, slot("p1", "units", 1));
    unit.costMod = TUNE_COST_CAP - 1;
    expect(unit.brittle?.count).toBe(2);
    for (let i = 0; i < 60 && unit.brittle?.count === 2; i += 1) once(state, unit, "degrade");
    expect(unit.brittle?.count).toBe(1);
    const held = card(state, brittleUnit.id);
    for (let i = 0; i < 60 && held.tuning?.x?.Brittle === undefined; i += 1) once(state, held, "upgrade");
    expect(held.tuning?.x?.Brittle).toBe(1);
    expect(placeOnField(state, held, slot("p1", "units", 2))).toBe(true);
    expect(held.brittle?.count).toBe(3);
  });

  it("R386 number: a declared number steps its own way — up or down, by its step — within its bounds", () => {
    const state = game();
    const spell = card(state, numbered.id);
    spell.costMod = -spell.costMod - 2; // a (0): no cost row for an Upgrade
    const value = (key: string): number => paramValue(state, spell, key);
    expect(applicableChanges(state, spell, "upgrade")).toEqual(["number"]);
    const seen = new Set<string>();
    for (let i = 0; i < 40; i += 1) {
      const change = changes(once(state, spell, "upgrade"))[0];
      if (change?.kind === "number") seen.add(change.key);
    }
    expect([...seen].sort()).toEqual(["big", "damage", "huge", "threshold"]);
    // Less is better for the threshold, and it never goes below its min (2).
    expect(value("threshold")).toBe(2);
    // The default steps: 2 for 8 (6–12), a quarter for 20 (5); `huge` stops at its max.
    expect((value("big") - 8) % 2).toBe(0);
    expect((value("huge") - 20) % 5 === 0 || value("huge") === 44).toBe(true);
    expect(value("huge")).toBeLessThanOrEqual(44);
  });

  it("R386 number: a Degrade never takes an amount below 1", () => {
    const state = game();
    const spell = card(state, numbered.id);
    run(state, degrade({ instanceId: spell.id, times: 60 }));
    expect(paramValue(state, spell, "damage")).toBe(1);
    expect(paramValue(state, spell, "threshold")).toBeGreaterThanOrEqual(3);
  });
});

describe("B3.4 rules 1 and 2: the draw, Immutable, N times (R386, R442)", () => {
  it("R386 an Immutable card is never changed and nothing is drawn for it", () => {
    const state = game();
    const unit = put(state, stoic.id, slot("p1", "units", 1));
    const cursor = state.rngCursor;
    const events = run(state, degrade({ instanceId: unit.id, times: 3 }));
    expect(events).toEqual([]);
    expect(state.rngCursor).toBe(cursor);
    expect(unit.tuning).toBeUndefined();
    expect(unit.costMod).toBe(0);
  });

  it("R440 an Immutable card in a hand is still cued, once per application, with the change `none`", () => {
    const state = game();
    const held = card(state, stoic.id);
    const events = run(state, upgrade({ instanceId: held.id, times: 2 }));
    expect(events).toEqual([
      { type: "upgraded", instanceId: held.id, defId: stoic.id, change: { kind: "none" }, hiddenFrom: ["p2"] },
      { type: "upgraded", instanceId: held.id, defId: stoic.id, change: { kind: "none" }, hiddenFrom: ["p2"] },
    ]);
  });

  it("R442 one row, one item: nothing is drawn; two rows: one draw picks the row, then the row draws its own", () => {
    const state = game();
    const spell = card(state, constant.id);
    const cursor = state.rngCursor;
    once(state, spell, "degrade");
    expect(state.rngCursor).toBe(cursor);

    const unit = put(state, freeBody.id, slot("p1", "units", 1));
    expect(applicableChanges(state, unit, "upgrade")).toEqual(["stats", "keyword"]);
    const before = state.rngCursor;
    once(state, unit, "upgrade");
    // The row, then the split k or the keyword out of the pool: two draws either way.
    expect(state.rngCursor).toBe(before + 2);
  });

  it("R386 N times is N applications, each reported", () => {
    const state = game();
    const unit = put(state, body.id, slot("p1", "units", 1));
    const events = run(state, upgrade({ instanceId: unit.id, times: 5 }));
    expect(eventsOfType(events, "upgraded")).toHaveLength(5);
    expect(eventsOfType(events, "upgraded").every((e) => e.hiddenFrom === undefined && e.change.kind !== "none")).toBe(true);
  });

  it("R386 a named target reaches a card on the field, in a hand or in a deck; one that has ceased to exist is not changed", () => {
    const state = game();
    const [top] = setLibrary(state, "p1", [constant.id]);
    if (top === undefined) throw new Error("no card");
    const events = run(state, degrade({ target: { of: "chosen" } }), {
      targets: [{ pick: "instance", instanceId: top.id }],
    });
    expect(top.costMod).toBe(1);
    expect(eventsOfType(events, "degraded")[0]?.hiddenFrom).toEqual(["p1", "p2"]);
    top.zone = { z: "gone", player: "p1" };
    state.players.p1.library = [];
    expect(run(state, degrade({ target: { of: "chosen" } }), { targets: [{ pick: "instance", instanceId: top.id }] })).toEqual([]);
  });
});

describe("B3.4 scopes and random picks (R60, R129, R242, R440)", () => {
  it("R440 a random pick over a deck picks N different cards, cues each for both players, in the deck's order", () => {
    const state = game();
    const deck = setLibrary(state, "p2", Array.from({ length: 10 }, () => constant.id));
    const events = eventsOfType(run(state, degrade({ scope: { side: "enemy", zones: ["library"] }, random: 4 })), "degraded");
    expect(events).toHaveLength(4);
    expect(new Set(events.map((e) => e.instanceId)).size).toBe(4);
    expect(events.every((e) => JSON.stringify(e.hiddenFrom) === JSON.stringify(["p1", "p2"]))).toBe(true);
    const order = events.map((e) => deck.findIndex((c) => c.id === e.instanceId));
    expect(order).toEqual([...order].sort((a, b) => a - b));
  });

  it("R129 a pick of at least as many cards as there are takes them all and draws nothing for the pick", () => {
    const state = game();
    setLibrary(state, "p2", [constant.id, constant.id]);
    const cursor = state.rngCursor;
    const events = eventsOfType(run(state, degrade({ scope: { side: "enemy", zones: ["library"] }, random: 4 })), "degraded");
    expect(events).toHaveLength(2);
    // Each card had one row and one item, so no draw at all.
    expect(state.rngCursor).toBe(cursor);
  });

  it("R440 a scope's filters decide which hidden cards change, and the rest are cued `none`, so the cues number the pile", () => {
    const state = game();
    const units = inHand(state, freeBody.id, "p1", 2);
    const spells = inHand(state, constant.id, "p1", 2);
    const events = eventsOfType(run(state, upgrade({ scope: { zones: ["hand"], types: ["Unit"] } })), "upgraded");
    expect(events).toHaveLength(4);
    for (const spell of spells) {
      expect(events.find((e) => e.instanceId === spell.id)?.change).toEqual({ kind: "none" });
      expect(spell.costMod).toBe(0);
    }
    for (const unit of units) expect(events.find((e) => e.instanceId === unit.id)?.change.kind).not.toBe("none");
  });

  it("R242 the events go out group by group: public cards, then the owner's hidden ones, then the deck's", () => {
    const state = game();
    const [deckCard] = setLibrary(state, "p1", [constant.id]);
    const [handCard] = inHand(state, constant.id, "p1");
    const unit = put(state, body.id, slot("p1", "units", 5));
    const events = eventsOfType(run(state, degrade({ scope: { zones: ["library", "hand", "field"] } })), "degraded");
    expect(events.map((e) => e.instanceId)).toEqual([unit.id, handCard?.id, deckCard?.id]);
  });

  it("R60 a random pick over a hand and a side of the field takes public and hidden cards alike, by their counts alone", () => {
    const counts = { field: 0, hand: 0 };
    for (let seed = 1; seed <= 40; seed += 1) {
      const state = game(`mix-${seed}`);
      const unit = put(state, body.id, slot("p2", "units", 1));
      inHand(state, stoic.id, "p2");
      const events = run(state, degrade({ scope: { side: "enemy", zones: ["hand", "field"] }, random: 1 }));
      const [event] = eventsOfType(events, "degraded");
      if (event?.instanceId === unit.id) counts.field += 1;
      else counts.hand += 1;
    }
    // The Immutable hand card is picked (and cued) as often as the unit, though it can never change.
    expect(counts.field).toBeGreaterThan(5);
    expect(counts.hand).toBeGreaterThan(5);
  });
});

describe("B3.4 rule 7: what the views show (R177, R311, R386)", () => {
  it("R386 a change to a hand card reads in full to its owner and as a bare cue to the other player", () => {
    const state = game();
    const held = card(state, constant.id);
    run(state, degrade({ instanceId: held.id }));
    state.applied = [{ nonce: "t", events: run(state, degrade({ instanceId: held.id })) }];
    const own = viewFor(state, "p1").events.find((e) => e.type === "degraded");
    const theirs = viewFor(state, "p2").events.find((e) => e.type === "degraded");
    expect(own).toEqual({ type: "degraded", instanceId: held.id, defId: constant.id, change: { kind: "cost", delta: 1 } });
    expect(theirs).toEqual({ type: "degraded", instanceId: HIDDEN_ID, defId: HIDDEN_ID, change: { kind: "number", key: HIDDEN_ID, delta: 0 } });
    const hand = viewFor(state, "p1").you.hand;
    if (!Array.isArray(hand)) throw new Error("own hand is a list");
    expect(hand[0]?.cost).toBe(3);
    expect(JSON.stringify(viewFor(state, "p2"))).not.toContain(held.id);
  });

  it("R311 a change made inside a deck stays unread by both players for good, and the owner's list shows the card as it went in", () => {
    const state = game();
    const [top] = setLibrary(state, "p1", [numbered.id]);
    if (top === undefined) throw new Error("no card");
    const listBefore = viewFor(state, "p1").you.ownLibrary;
    state.applied = [{ nonce: "t", events: run(state, upgrade({ instanceId: top.id, times: 2 })) }];
    expect(viewFor(state, "p1").you.ownLibrary).toEqual(listBefore);
    // Drawn, the card reads with its changes — and the events made inside the deck still do not.
    drawCards(sinkFor(state), "p1", 1);
    const view = viewFor(state, "p1");
    const hand = view.you.hand;
    if (!Array.isArray(hand)) throw new Error("own hand is a list");
    expect(hand.find((c) => c.instanceId === top.id)?.tuning).toEqual(top.tuning);
    for (const event of view.events.filter((e) => e.type === "upgraded")) {
      expect(event).toMatchObject({ instanceId: HIDDEN_ID, defId: HIDDEN_ID });
    }
  });

  it("R386 a changed card's view carries its tuning and its declared numbers where its viewer may read it", () => {
    const state = game();
    const unit = put(state, body.id, slot("p1", "units", 1));
    unit.tuning = { attack: 2, addKeywords: [{ kind: "Rush" }] };
    const held = card(state, numbered.id);
    held.tuning = { numbers: { damage: 2 } };
    const mine = viewFor(state, "p1");
    const theirs = viewFor(state, "p2");
    expect(mine.you.units[0]?.tuning).toEqual({ attack: 2, addKeywords: [{ kind: "Rush" }] });
    expect(theirs.opponent.units[0]?.tuning).toEqual({ attack: 2, addKeywords: [{ kind: "Rush" }] });
    expect(theirs.opponent.units[0]?.attack).toBe(5);
    const hand = mine.you.hand;
    if (!Array.isArray(hand)) throw new Error("own hand is a list");
    expect(hand.find((c) => c.instanceId === held.id)?.params).toEqual({ damage: 4, threshold: 3, big: 8, huge: 20 });
    expect(theirs.opponent.hand).toEqual({ count: 1 });
  });
});

describe("B3.4 rule 4: the changes are the card's (R57, R78, R102, R386, R443)", () => {
  it("R386 tuning is kept through leaving the field and in every zone (R78), a spent Brittle count aside", () => {
    const state = game();
    const unit = put(state, body.id, slot("p1", "units", 1));
    unit.tuning = { attack: -1, removeKeywords: ["Rush"], numbers: { damage: 1 } };
    unit.buffs = { attack: 3, health: 0 };
    moveToZone(state, unit, "hand");
    expect(unit.tuning).toEqual({ attack: -1, removeKeywords: ["Rush"], numbers: { damage: 1 } });
    expect(unit.buffs).toEqual({ attack: 0, health: 0 });
    moveToZone(state, unit, "graveyard");
    moveToZone(state, unit, "exile");
    expect(unit.tuning).toEqual({ attack: -1, removeKeywords: ["Rush"], numbers: { damage: 1 } });
  });

  it("R57 a copy on the field keeps the source's tuning and never its Brittle count", () => {
    const state = game();
    const unit = put(state, brittleUnit.id, slot("p1", "units", 1));
    unit.tuning = { health: 2 };
    run(state, summonCopy({ of: { of: "instance", instanceId: unit.id } }));
    const copy = state.players.p1.units[1]?.[0];
    expect(copy?.tuning).toEqual({ health: 2 });
    // It entered the field, so its own printed Brittle starts; the source's count is not copied.
    unit.brittle = { count: 1, since: 1 };
    run(state, summonCopy({ of: { of: "instance", instanceId: unit.id } }));
    const second = state.players.p1.units[2]?.[0];
    expect(second?.brittle).toEqual({ count: 2, since: state.turn, printed: true });
  });

  it("R57 a copy shuffled into a deck carries the source's tuning", () => {
    const state = game();
    const spell = newInstance(state, numbered.id, "p1", { z: "resolving", player: "p1" });
    state.players.p1.resolving.push(spell);
    spell.tuning = { numbers: { damage: 1 } };
    const copies = (): CardInstance[] => state.players.p1.library.filter((c) => c.defId === numbered.id);
    run(state, shuffleCopiesOfSelf({ count: 2 }), { self: spell });
    expect(copies().map((c) => c.tuning)).toEqual([{ numbers: { damage: 1 } }, { numbers: { damage: 1 } }]);
    run(state, shuffleInto({ defId: numbered.id, count: 1, copyOf: spell.id }));
    expect(copies().filter((c) => c.tuning !== undefined)).toHaveLength(3);
    // A card the text names rather than copies is a fresh card with none.
    run(state, shuffleInto({ defId: numbered.id, count: 1 }));
    expect(copies().filter((c) => c.tuning === undefined)).toHaveLength(1);
  });

  it("R386 a Transform makes a card without the old one's tuning", () => {
    const state = game();
    const unit = put(state, body.id, slot("p1", "units", 1));
    unit.tuning = { attack: 3 };
    run(state, transform({ instanceId: unit.id, defId: plain.id }));
    const replacement = state.players.p1.units[0]?.[0];
    expect(replacement?.defId).toBe(plain.id);
    expect(replacement?.tuning).toBeUndefined();
  });

  it("R102 a Fuse sums its ingredients' tuning onto the card it keeps", () => {
    const state = game();
    const kept = put(state, body.id, slot("p1", "units", 1));
    kept.tuning = { attack: 1, x: { Armor: 1 } };
    const other = put(state, plain.id, slot("p1", "units", 2));
    other.tuning = { attack: 2, health: -1, x: { Armor: 1 }, addKeywords: [{ kind: "Rush" }] };
    const fused = fuse(sinkFor(state), { ingredients: [other], target: kept });
    expect(fused?.id).toBe(kept.id);
    expect(kept.tuning).toEqual({ attack: 3, health: -1, x: { Armor: 2 }, addKeywords: [{ kind: "Rush" }] });
  });
});

describe("Classic+ #41 KY's Constant: a number set outright (R386)", () => {
  it("R386 numbersOn lists the cost (never an X), attack, health, numbered keywords and declared numbers; none on an Immutable card", () => {
    const state = game();
    const unit = put(state, numberedBody.id, slot("p1", "units", 1));
    unit.damage = 1;
    expect(numbersOn(state, unit).map((n) => [n.id, n.value])).toEqual([
      ["cost", 1],
      ["attack", 2],
      ["health", 1],
      ["keyword:Armor", 2],
      ["keyword:Lucky", 1],
      ["keyword:Spell Damage", 1],
    ]);
    expect(numbersOn(state, card(state, xBolt.id))).toEqual([]);
    expect(numbersOn(state, card(state, numbered.id)).map((n) => n.id)).toEqual([
      "cost",
      "param:damage",
      "param:threshold",
      "param:big",
      "param:huge",
    ]);
    expect(numbersOn(state, card(state, stoic.id))).toEqual([]);
  });

  it("R386 each number is set to the value, and stays set as tuning; a later step counts from it", () => {
    const state = game();
    const unit = put(state, numberedBody.id, slot("p1", "units", 1));
    unit.damage = 1;
    for (const which of ["cost", "attack", "health", "keyword:Armor", "keyword:Lucky"]) {
      run(state, setNumber({ instanceId: unit.id, which, value: 3 }));
    }
    expect(numbersOn(state, unit).map((n) => n.value)).toEqual([3, 3, 3, 3, 3, 1]);
    expect(unitView(state, unit)).toMatchObject({ attack: 3, health: 3, maxHealth: 4 });
    unit.tuning = { ...unit.tuning, x: { Armor: 1 } };
    expect(numbersOn(state, unit).find((n) => n.id === "keyword:Armor")?.value).toBe(4);

    const spell = card(state, numbered.id);
    const events = run(state, setNumber({ instanceId: spell.id, which: "param:huge", value: 3 }));
    expect(paramValue(state, spell, "huge")).toBe(3);
    expect(events).toEqual([{ type: "numberChanged", instanceId: spell.id, defId: numbered.id, key: "huge", value: 3, hiddenFrom: ["p2"] }]);
  });

  it("R386 `random` picks among the numbers that are not the value already, and does nothing with none left", () => {
    const state = game();
    const spell = card(state, numbered.id);
    run(state, setNumber({ instanceId: spell.id, which: "random", value: 3 }));
    expect(numbersOn(state, spell).filter((n) => n.value === 3).length).toBeGreaterThanOrEqual(2);
    for (let i = 0; i < 6; i += 1) run(state, setNumber({ instanceId: spell.id, which: "random", value: 3 }));
    expect(numbersOn(state, spell).every((n) => n.value === 3)).toBe(true);
    const cursor = state.rngCursor;
    expect(run(state, setNumber({ instanceId: spell.id, which: "random", value: 3 }))).toEqual([]);
    expect(state.rngCursor).toBe(cursor);
  });

  it("R386 a number the card does not have, or an Immutable card, is left alone", () => {
    const state = game();
    const spell = card(state, constant.id);
    expect(run(state, setNumber({ instanceId: spell.id, which: "attack", value: 3 }))).toEqual([]);
    const wall = card(state, stoic.id);
    expect(run(state, setNumber({ instanceId: wall.id, which: "cost", value: 3 }))).toEqual([]);
  });

  it("R386 the number set on a hand card is the card's to keep: the other player sees a bare cue", () => {
    const state = game();
    const spell = card(state, numbered.id);
    state.applied = [{ nonce: "t", events: run(state, setNumber({ instanceId: spell.id, which: "param:damage", value: 3 })) }];
    expect(viewFor(state, "p2").events.find((e) => e.type === "numberChanged")).toEqual({
      type: "numberChanged",
      instanceId: HIDDEN_ID,
      defId: HIDDEN_ID,
      key: HIDDEN_ID,
      value: 0,
    });
    expect(viewFor(state, "p1").events.find((e) => e.type === "numberChanged")).toMatchObject({ key: "damage", value: 3 });
  });
});

/** The keywords a hand card will carry onto the field, for the X row test. */
function statsOfKeywords(state: GameState, c: CardInstance): unknown[] {
  return numbersOn(state, c).length > 0 ? unitViewKeywords(state, c) : [];
}

function unitViewKeywords(state: GameState, c: CardInstance): unknown[] {
  const zone = c.zone;
  if (zone.z === "field") return unitView(state, c).keywords;
  const hand = viewFor(state, c.owner).you.hand;
  if (!Array.isArray(hand)) return [];
  return hand.find((view) => view.instanceId === c.id)?.keywords ?? [];
}
