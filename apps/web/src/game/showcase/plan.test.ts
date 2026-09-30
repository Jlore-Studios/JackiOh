// The showcase's trigger rules, from the redacted event stream alone (plan.ts; CLAUDE.md rule 7,
// R97, R227): the opponent's plays are held up, the viewer's own never are, and a card the view
// hides is a back that names nothing.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { baseView, card, withEvents } from "../../test/fixtures.ts";
import { capQueue, chaosRollsIn, eventsSince, opponentPlays, sameOccurrence, showcasePlays } from "./plan.ts";

const TURN: GameEvent = { type: "turnStarted", player: "p2", turn: 4 };
const MANA: GameEvent = { type: "manaChanged", player: "p2", current: 3, max: 3 };

function played(player: "p1" | "p2", instanceId: string, defId: string, extra: Partial<GameEvent> = {}): GameEvent {
  return { type: "cardPlayed", player, instanceId, defId, costPaid: 1, ...extra } as GameEvent;
}

function summoned(player: "p1" | "p2", instanceId: string, defId: string, row: "units" | "backrow", lane = 2): GameEvent {
  return { type: "summoned", player, instanceId, defId, row, lane };
}

/** The viewer is p1 (`baseView`), so p2 is the opponent. */
const view = baseView();

describe("which plays are held up", () => {
  it("holds up the opponent's play, by its definition, its instance and what it cost", () => {
    const plays = opponentPlays([MANA, played("p2", "c7", "core-032"), summoned("p2", "c7", "core-032", "units")], view);
    // The instance and the price paid let the showcase draw the card in play (SPEC §10.10).
    expect(plays).toEqual([
      { player: "p2", defId: "core-032", instanceId: "c7", costPaid: 1, radiant: false, set: false },
    ]);
  });

  it("never holds up the viewer's own play", () => {
    expect(opponentPlays([played("p1", "c3", "core-011"), summoned("p1", "c3", "core-011", "units")], view)).toEqual([]);
  });

  it("holds up every opponent play in order, a cast (R70) included", () => {
    const plays = opponentPlays([played("p2", "c7", "core-005"), played("p2", "c8", "core-010", { costPaid: 0 })], view);
    expect(plays.map((play) => play.defId)).toEqual(["core-005", "core-010"]);
  });

  it("R97 / R227 a card set face down is a back that says it was set, with no definition", () => {
    const plays = opponentPlays([played("p2", "hidden", "hidden"), summoned("p2", "hidden", "hidden", "backrow", 3)], view);
    expect(plays).toEqual([{ player: "p2", defId: null, radiant: false, set: true }]);
    expect(JSON.stringify(plays)).not.toMatch(/core-\d+/);
  });

  it("R370 a card set face down carries the cost its back shows where it landed, while it stands there", () => {
    const events = [played("p2", "hidden", "hidden"), summoned("p2", "hidden", "hidden", "backrow", 3)];
    const withCost = baseView({
      opponent: { ...baseView().opponent, backrow: [null, null, { faceDown: true, cost: 2 }, null, null] },
    });
    expect(opponentPlays(events, withCost)).toEqual([{ player: "p2", defId: null, radiant: false, set: true, cost: 2 }]);
    // The zone is empty again (the trap fired), or its back gives no cost: no cost is claimed.
    for (const entry of [null, { faceDown: true as const }]) {
      const gone = baseView({ opponent: { ...baseView().opponent, backrow: [null, null, entry, null, null] } });
      expect(opponentPlays(events, gone)).toEqual([{ player: "p2", defId: null, radiant: false, set: true }]);
    }
  });

  it("R97 a hidden play that set nothing is a back that says a card was played", () => {
    expect(opponentPlays([played("p2", "hidden", "hidden")], view)).toEqual([
      { player: "p2", defId: null, radiant: false, set: false },
    ]);
  });

  it("takes the face that resolved from cardResolved, else the one the view shows", () => {
    const resolved: GameEvent = {
      type: "cardResolved",
      player: "p2",
      instanceId: "c7",
      defId: "core-032",
      permanent: true,
      costPaid: 1,
      radiant: true,
    };
    expect(opponentPlays([played("p2", "c7", "core-032"), resolved], view)[0]?.radiant).toBe(true);

    const onBoard = baseView({
      opponent: { ...baseView().opponent, graveyard: [card({ instanceId: "c9", defId: "core-005", radiant: true })] },
    });
    expect(opponentPlays([played("p2", "c9", "core-005")], onBoard)[0]?.radiant).toBe(true);
  });

  it("a hotseat viewer seated p2 reads p1 as the opponent", () => {
    const asP2 = baseView({ viewer: "p2", you: baseView().opponent, opponent: baseView().you });
    expect(opponentPlays([played("p1", "c3", "core-011")], asP2).map((play) => play.defId)).toEqual(["core-011"]);
    expect(opponentPlays([played("p2", "c4", "core-011")], asP2)).toEqual([]);
  });
});

describe("which events are new", () => {
  it("finds the events after the overlap of two windows", () => {
    const a: GameEvent = { type: "drawn", player: "p2", instanceId: "c1", defId: "core-001" };
    const prev = [TURN, MANA, a];
    const next = [MANA, a, played("p2", "c1", "core-001")];
    expect(eventsSince(prev, next)).toEqual([played("p2", "c1", "core-001")]);
  });

  it("R97 a draw that read as the sentinel and now names the card played is the same event", () => {
    const hiddenDraw: GameEvent = { type: "drawn", player: "p2", instanceId: "hidden", defId: "hidden" };
    const openDraw: GameEvent = { type: "drawn", player: "p2", instanceId: "c1", defId: "core-032" };
    const prev = [TURN, hiddenDraw, MANA];
    const next = [TURN, openDraw, MANA, played("p2", "c1", "core-032")];
    // A plain comparison finds no overlap and would call the whole window new, the old plays too.
    expect(eventsSince(prev, next)).toEqual([played("p2", "c1", "core-032")]);
  });

  it("R97 a play hidden before and readable now is not played again", () => {
    const hiddenPlay = played("p2", "hidden", "hidden");
    const openPlay = played("p2", "c5", "core-041");
    const prev = [TURN, hiddenPlay];
    const next = [TURN, openPlay, MANA];
    expect(eventsSince(prev, next)).toEqual([MANA]);
  });

  it("two windows with nothing in common make the whole newer one new", () => {
    expect(eventsSince([MANA], [TURN])).toEqual([TURN]);
    expect(eventsSince([], [TURN])).toEqual([TURN]);
  });

  it("the same occurrence: identical events, or a sentinel against a reading about the same seats", () => {
    expect(sameOccurrence(MANA, { ...MANA })).toBe(true);
    expect(sameOccurrence(MANA, { ...MANA, current: 2 } as GameEvent)).toBe(false);
    expect(sameOccurrence(played("p2", "hidden", "hidden"), played("p2", "c5", "core-041"))).toBe(true);
    expect(sameOccurrence(played("p2", "hidden", "hidden"), played("p1", "c5", "core-041"))).toBe(false);
    expect(sameOccurrence(played("p2", "c4", "core-001"), played("p2", "c5", "core-041"))).toBe(false);
    expect(sameOccurrence(MANA, TURN)).toBe(false);
    expect(sameOccurrence(undefined, MANA)).toBe(false);
  });
});

describe("the queue", () => {
  it("keeps the newest plays", () => {
    expect(capQueue([1, 2, 3, 4, 5], 3)).toEqual([3, 4, 5]);
    expect(capQueue([1, 2], 3)).toEqual([1, 2]);
  });
});

describe("R502 a cast on draw is held up on both seats", () => {
  const drawn = (player: "p1" | "p2", instanceId: string, defId: string): GameEvent => ({ type: "drawn", player, instanceId, defId });
  const cast = (player: "p1" | "p2", instanceId: string, defId: string): GameEvent => played(player, instanceId, defId, { costPaid: 0 });

  it("R502 the opponent's cast on draw is held up, marked as one", () => {
    const events = [TURN, drawn("p2", "c21", "core-021"), cast("p2", "c21", "core-021")];
    expect(showcasePlays(events, withEvents(view, events)).map((item) => item.play)).toEqual([
      { player: "p2", defId: "core-021", instanceId: "c21", costPaid: 0, radiant: false, set: false, castOnDraw: true },
    ]);
  });

  it("R502 the viewer's own cast on draw is held up too, though its other plays never are", () => {
    const events = [drawn("p1", "c27", "core-027"), cast("p1", "c27", "core-027"), played("p1", "c3", "core-011")];
    const items = showcasePlays(events, withEvents(view, events));
    expect(items.map((item) => [item.play.defId, item.play.castOnDraw])).toEqual([["core-027", true]]);
    // The item carries the very event the view holds, so the runner's entry can be matched to it.
    expect(items[0]?.event).toBe(events[1]);
  });

  it("R502 R97 a hidden cast on draw is a back that names nothing", () => {
    const events = [drawn("p2", "hidden", "hidden"), cast("p2", "hidden", "hidden")];
    const items = showcasePlays(events, withEvents(view, events));
    expect(items.map((item) => item.play)).toEqual([{ player: "p2", defId: null, radiant: false, set: false, castOnDraw: true }]);
    expect(JSON.stringify(items.map((item) => item.play))).not.toMatch(/core-\d+/);
  });

  it("R502 a drawn that came in an earlier view still makes the cast one: it is read off the whole window", () => {
    const window = [TURN, drawn("p2", "c21", "core-021"), cast("p2", "c21", "core-021")];
    const fresh = window.slice(2);
    expect(showcasePlays(fresh, withEvents(view, window))[0]?.play.castOnDraw).toBe(true);
    // Fresh events that are not the window's tail are read on their own.
    expect(showcasePlays([cast("p2", "c21", "core-021")], withEvents(view, [TURN]))[0]?.play.castOnDraw).toBeUndefined();
  });

  it("R502 the opponent's ordinary plays are held up as before, with no cast mark", () => {
    const events = [MANA, played("p2", "c7", "core-032")];
    expect(showcasePlays(events, withEvents(view, events)).map((item) => item.play)).toEqual(opponentPlays(events, view));
    expect(opponentPlays(events, view)[0]?.castOnDraw).toBeUndefined();
  });
});

describe("R436 the rolls a Call to Chaos names", () => {
  it("R436 finds each roll among the fresh events, with its event, and skips an empty one", () => {
    const roll: GameEvent = { type: "chaosRolled", player: "p2", instanceId: "c95", defId: "core-095", effects: ["heal", "units"] };
    const empty: GameEvent = { type: "chaosRolled", player: "p1", instanceId: "c96", defId: "core-095", effects: [] };
    expect(chaosRollsIn([TURN, roll, empty])).toEqual([{ roll: { player: "p2", instanceId: "c95", defId: "core-095", effects: ["heal", "units"] }, event: roll }]);
  });
});
