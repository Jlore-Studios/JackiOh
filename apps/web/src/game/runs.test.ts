// The animation runner's reads off a burst (game/runs.ts): which plays are casts, which runs hit a
// whole pile, and which runs of hits sweep a whole side.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { baseView, card, emptySide, unit } from "../test/fixtures.ts";
import { createPlayTracker, pileOf, pileRunAt, sweepAt } from "./runs.ts";

function viewWith(you: Partial<ReturnType<typeof emptySide>> = {}, opponent: Partial<ReturnType<typeof emptySide>> = {}): PlayerView {
  return baseView({ you: emptySide("p1", you), opponent: emptySide("p2", { hand: { count: 4 }, ...opponent }) });
}

function played(player: PlayerId, instanceId: string, defId: string, costPaid: number): GameEvent {
  return { type: "cardPlayed", player, instanceId, defId, costPaid };
}

function resolved(player: PlayerId, instanceId: string): GameEvent {
  return { type: "cardResolved", player, instanceId, defId: "core-001", permanent: false, costPaid: 2 };
}

describe("createPlayTracker", () => {
  it("reads a play inside another play that paid nothing as a cast", () => {
    const plays = createPlayTracker();
    expect(plays.see(played("p1", "box", "classicplus-047", 8))).toBe(null);
    const first = plays.see(played("p1", "cast-1", "core-010", 0));
    expect(first).toEqual({ by: { player: "p1", instanceId: "box", defId: "classicplus-047", casts: 1 }, ordinal: 1 });
    // A cast resolves before the next begins, so the next is the box's second cast, not the cast's.
    plays.see(resolved("p1", "cast-1"));
    const second = plays.see(played("p1", "cast-2", "core-011", 0));
    expect(second?.ordinal).toBe(2);
    expect(second?.by.casts).toBe(2);
  });

  it("does not read a free play with nothing open as a cast", () => {
    const plays = createPlayTracker();
    expect(plays.see(played("p1", "free", "core-010", 0))).toBe(null);
  });

  it("closes a play when it resolves, so a later play is not its cast", () => {
    const plays = createPlayTracker();
    plays.see(played("p1", "box", "classicplus-047", 8));
    plays.see(resolved("p1", "box"));
    expect(plays.see(played("p1", "free", "core-010", 0))).toBe(null);
  });

  it("closes the sentinel's innermost play of that player", () => {
    const plays = createPlayTracker();
    plays.see(played("p2", "box", "classicplus-047", 8));
    plays.see({ type: "cardResolved", player: "p2", instanceId: "hidden", defId: "hidden", permanent: false, costPaid: 0 });
    expect(plays.current()).toBe(undefined);
  });

  it("clears whatever a lost event left open on a new turn", () => {
    const plays = createPlayTracker();
    plays.see(played("p1", "box", "classicplus-047", 8));
    plays.see({ type: "turnStarted", player: "p2", turn: 4 });
    expect(plays.current()).toBe(undefined);
  });
});

describe("pileOf", () => {
  it("reads a readable card's pile off the view", () => {
    const grave = card({ instanceId: "g1", defId: "core-003" });
    const view = viewWith({ graveyard: [grave] });
    expect(pileOf({ type: "degraded", instanceId: "g1", defId: "core-003", change: { kind: "cost", delta: -1 } }, view, undefined)).toEqual({
      side: "you",
      pile: "graveyard",
    });
  });

  it("reads a hidden change off the resolving card's public text", () => {
    const view = viewWith({ libraryCount: 12 });
    const play = { player: "p1" as PlayerId, instanceId: "storm", defId: "classicplus-008", casts: 0 };
    expect(pileOf({ type: "degraded", instanceId: "hidden", defId: "hidden", change: { kind: "cost", delta: -1 } }, view, play)).toEqual({
      side: "opponent",
      pile: "library",
    });
  });

  it("maps no pile for a hidden change whose card names none", () => {
    const view = viewWith();
    const play = { player: "p1" as PlayerId, instanceId: "x", defId: "core-001", casts: 0 };
    expect(pileOf({ type: "degraded", instanceId: "hidden", defId: "hidden", change: { kind: "cost", delta: -1 } }, view, play)).toBe(null);
  });

  it("never puts a crumble in a pile: it happens on the field", () => {
    const view = viewWith();
    expect(pileOf({ type: "crumbled", instanceId: "u1", defId: "core-004", owner: "p1", zone: "field" }, view, undefined)).toBe(null);
  });

  it("lands every shuffle in its player's library, hidden or not", () => {
    const view = viewWith({ libraryCount: 5 });
    const hidden = pileOf({ type: "shuffledIn", player: "p1", instanceId: "hidden", defId: "hidden", position: 0 }, view, undefined);
    expect(hidden).toEqual({ side: "you", pile: "library" });
    expect(hidden?.inferred).toBe(undefined);
  });

  it("infers the viewer's own unshown exile as their library, and maps no pile for the opponent's", () => {
    const view = viewWith();
    const own = pileOf({ type: "exiled", instanceId: "gone", defId: "hidden", owner: "p1" }, view, undefined);
    expect(own).toEqual({ side: "you", pile: "library", inferred: true });
    // An opponent's hidden hand and backrow make the same guess unsound: a random hand exile must
    // never read as their library.
    expect(pileOf({ type: "exiled", instanceId: "gone", defId: "hidden", owner: "p2" }, view, undefined)).toBe(null);
  });

  it("reads an exiled graveyard card as its graveyard", () => {
    const grave = card({ instanceId: "g1", defId: "core-003" });
    const view = viewWith({ graveyard: [grave] });
    expect(pileOf({ type: "exiled", instanceId: "g1", defId: "core-003", owner: "p1" }, view, undefined)).toEqual({
      side: "you",
      pile: "graveyard",
    });
  });
});

describe("pileRunAt", () => {
  function degraded(id: string): GameEvent {
    return { type: "degraded", instanceId: id, defId: "core-003", change: { kind: "cost", delta: -1 } };
  }

  it("plays a run that reaches every card of a pile once", () => {
    const graves = [card({ instanceId: "g1" }), card({ instanceId: "g2" })];
    const view = viewWith({ graveyard: graves });
    const run = pileRunAt([degraded("g1"), degraded("g2")], 0, view, undefined);
    expect(run).toMatchObject({ pile: { side: "you", pile: "graveyard" }, length: 2, whole: true, count: 2 });
  });

  it("still plays one entry per card when the card names a number of them", () => {
    const graves = [card({ instanceId: "g1" }), card({ instanceId: "g2" }), card({ instanceId: "g3" }), card({ instanceId: "g4" })];
    const view = viewWith({ graveyard: graves });
    const run = pileRunAt([degraded("g1"), degraded("g2")], 0, view, undefined);
    expect(run?.whole).toBe(false);
    expect(run?.length).toBe(2);
  });

  it("counts an inferred pile whole only at exactly its size", () => {
    const view = viewWith({ libraryCount: 3 });
    const exiled = (id: string): GameEvent => ({ type: "exiled", instanceId: id, defId: "hidden", owner: "p1" });
    expect(pileRunAt([exiled("a"), exiled("b")], 0, view, undefined)?.whole).toBe(false);
    expect(pileRunAt([exiled("a"), exiled("b"), exiled("c")], 0, view, undefined)?.whole).toBe(true);
  });
});

describe("sweepAt", () => {
  function board(): { view: PlayerView; you: string[]; foe: string[] } {
    const you = [unit("p1", { instanceId: "y1" }), unit("p1", { instanceId: "y2" })];
    const foe = [unit("p2", { instanceId: "e1" }), unit("p2", { instanceId: "e2" })];
    const view = viewWith(
      { units: [you[0] ?? null, you[1] ?? null, null, null, null] },
      { units: [foe[0] ?? null, foe[1] ?? null, null, null, null] },
    );
    return { view, you: ["y1", "y2"], foe: ["e1", "e2"] };
  }

  function hit(targetId: string, sourceId: string | null = "spell"): GameEvent {
    return { type: "damage", sourceId, targetId, amount: 2, combat: false };
  }

  it("sweeps non-combat hits from one source over every unit of a side", () => {
    const { view, foe } = board();
    const events = foe.map((id) => hit(id));
    const found = sweepAt(events, 0, view);
    expect(found).toEqual({ sweep: { tone: "damage", sides: ["opponent"], sourceId: "spell" }, length: 2 });
  });

  it("sweeps heals over allies", () => {
    const { view, you } = board();
    const events: GameEvent[] = you.map((id) => ({ type: "healed", targetId: id, amount: 2 }));
    expect(sweepAt(events, 0, view)?.sweep).toEqual({ tone: "heal", sides: ["you"], sourceId: null });
  });

  it("ignores combat damage and single hits", () => {
    const { view, foe } = board();
    expect(sweepAt([{ type: "damage", sourceId: null, targetId: foe[0] ?? "", amount: 2, combat: true }], 0, view)).toBe(null);
    expect(sweepAt([hit(foe[0] ?? "")], 0, view)).toBe(null);
  });

  it("is no sweep when a unit of the side is missed", () => {
    const { view, foe } = board();
    expect(sweepAt([hit(foe[0] ?? "")], 0, view)).toBe(null);
  });

  it("starts a new sweep when a target is hit again", () => {
    const { view, foe } = board();
    const first = foe[0] ?? "";
    const second = foe[1] ?? "";
    const events = [hit(first), hit(second), hit(first), hit(second)];
    expect(sweepAt(events, 0, view)?.length).toBe(2);
    expect(sweepAt(events, 2, view)).toEqual({ sweep: { tone: "damage", sides: ["opponent"], sourceId: "spell" }, length: 2 });
  });

  it("breaks the run when the source changes", () => {
    const { view, foe } = board();
    expect(sweepAt([hit(foe[0] ?? "", "a"), hit(foe[1] ?? "", "b")], 0, view)).toBe(null);
  });
});
