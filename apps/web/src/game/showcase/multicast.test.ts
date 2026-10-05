// The showcase's issue-#124 trigger rule (showcase/plan.ts): every card another card casts is held
// up on both seats, with the card that cast it and which of its casts it is.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { baseView, withEvents } from "../../test/fixtures.ts";
import { createPlayTracker } from "../runs.ts";
import { showcasePlays } from "./plan.ts";

const view = baseView();

function boxPlayed(): GameEvent {
  return { type: "cardPlayed", player: "p1", instanceId: "box", defId: "classicplus-047", costPaid: 8 } satisfies GameEvent;
}

function castPlayed(instanceId: string): GameEvent {
  return { type: "cardPlayed", player: "p1", instanceId, defId: "core-010", costPaid: 0 } satisfies GameEvent;
}

function castResolved(instanceId: string): GameEvent {
  return { type: "cardResolved", player: "p1", instanceId, defId: "core-010", permanent: false, costPaid: 0 } satisfies GameEvent;
}

describe("a card another card cast", () => {
  it("is held up with its caster and its ordinal, on the caster's own seat too", () => {
    const events = [boxPlayed(), castPlayed("c1")];
    const items = showcasePlays(events, withEvents(view, events));
    expect(items.map((item) => item.play.castBy)).toEqual([{ defId: "classicplus-047", ordinal: 1 }]);
  });

  it("numbers each cast of the burst in order", () => {
    const events = [boxPlayed(), castPlayed("c1"), castResolved("c1"), castPlayed("c2")];
    const items = showcasePlays(events, withEvents(view, events));
    expect(items.map((item) => item.play.castBy)).toEqual([
      { defId: "classicplus-047", ordinal: 1 },
      { defId: "classicplus-047", ordinal: 2 },
    ]);
  });

  it("names no caster the view hides", () => {
    const hidden: GameEvent = { type: "cardPlayed", player: "p2", instanceId: "box", defId: "hidden", costPaid: 8 };
    const events: GameEvent[] = [hidden, { type: "cardPlayed", player: "p2", instanceId: "c1", defId: "core-010", costPaid: 0 }];
    const items = showcasePlays(events, withEvents(view, events));
    // The hidden box is held up as the opponent's ordinary play; its cast names no caster.
    expect(items.map((item) => item.play.castBy)).toEqual([undefined, { defId: null, ordinal: 1 }]);
    expect(JSON.stringify(items.map((item) => item.play))).not.toMatch(/classicplus/);
  });

  it("still never holds up the viewer's own ordinary plays", () => {
    const own: GameEvent = { type: "cardPlayed", player: "p1", instanceId: "c3", defId: "core-011", costPaid: 1 };
    expect(showcasePlays([own], withEvents(view, [own]))).toEqual([]);
  });

  it("finds a cast whose caster opened in an earlier view through the kept tracker", () => {
    const plays = createPlayTracker();
    const earlier: GameEvent[] = [boxPlayed()];
    for (const event of earlier) plays.see(event);
    const fresh = [castPlayed("c1")];
    const items = showcasePlays(fresh, withEvents(view, [...earlier, ...fresh]), plays);
    expect(items.map((item) => item.play.castBy)).toEqual([{ defId: "classicplus-047", ordinal: 1 }]);
  });
});
