// R97, R385: `newEventsSince` hands the runner only the events its view has not shown yet, and finds them
// by matching the end of the old window against the start of the new one with `sameOccurrence`. R97
// judges a card by where it sits NOW, so an older event can read differently in the next view: the
// engine's `redactEvent` rewrites some fields together with a hidden card's identity (viewFor.ts), and
// `REWRITTEN_WITH_IDENTITY` has to name each of them or two copies of one event stop matching. When
// they stop matching, no overlap is found and the whole 32-event window is animated again.
//
// Found by the R318 sweep in animations.window.test.ts (game 57 of the overflow games, once the pool
// held one more card): a `counterChanged` for a Brittle count on a card that was hidden in the old view
// and readable in the new one. Each pair below is the engine's redacted copy beside the open one.

import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { HIDDEN_ID, newEventsSince, sameOccurrence } from "./animations.ts";

const filler = (n: number): GameEvent => ({ type: "manaChanged", player: "p1", current: n, max: n });

/** [what the engine rewrites, the copy a viewer who may not read the card has, the copy once it may]. */
const PAIRS: readonly (readonly [string, GameEvent, GameEvent])[] = [
  [
    "R385 a Brittle count goes with the card's identity",
    { type: "counterChanged", instanceId: HIDDEN_ID, counter: "brittle", value: -1 },
    { type: "counterChanged", instanceId: "c84", counter: "brittle", value: 2 },
  ],
  [
    "R227 a stolen card's former id goes with its identity",
    { type: "controlChanged", instanceId: HIDDEN_ID, controller: "p1", row: "units", lane: 2 },
    { type: "controlChanged", instanceId: "c84", controller: "p1", row: "units", lane: 2, formerId: "c12" },
  ],
  [
    "R386 a hidden Degrade shows that a card changed, never how",
    { type: "degraded", instanceId: HIDDEN_ID, defId: HIDDEN_ID, change: { kind: "number", key: HIDDEN_ID, delta: 0 } },
    { type: "degraded", instanceId: "c84", defId: "core-004", change: { kind: "stats", attack: -1, health: -1 } },
  ],
  [
    "R386 a hidden Upgrade likewise",
    { type: "upgraded", instanceId: HIDDEN_ID, defId: HIDDEN_ID, change: { kind: "number", key: HIDDEN_ID, delta: 0 } },
    { type: "upgraded", instanceId: "c84", defId: "core-004", change: { kind: "cost", delta: -1 } },
  ],
  [
    "Classic+ #41 a number set outright goes with the card",
    { type: "numberChanged", instanceId: HIDDEN_ID, defId: HIDDEN_ID, key: HIDDEN_ID, value: 0 },
    { type: "numberChanged", instanceId: "c84", defId: "classicplus-041", key: "damage", value: 7 },
  ],
  [
    "R448 a card set face-down announces as a Trap whatever type it is",
    { type: "cardAnnounced", player: "p2", instanceId: HIDDEN_ID, defId: HIDDEN_ID, cardType: "Trap", costPaid: 1, targets: [], faceDown: true },
    { type: "cardAnnounced", player: "p2", instanceId: "c84", defId: "core-018", cardType: "Field Trap", costPaid: 1, targets: [], faceDown: true },
  ],
];

describe("R97 sameOccurrence matches an event across the view that hid its card and the view that shows it", () => {
  for (const [name, hidden, open] of PAIRS) {
    it(`${name}: in either order`, () => {
      expect(sameOccurrence(hidden, open)).toBe(true);
      expect(sameOccurrence(open, hidden)).toBe(true);
    });

    it(`${name}: newEventsSince still hands the runner only what is new`, () => {
      const fresh = [filler(7), filler(8)];
      const before = [filler(1), hidden, filler(2)];
      const after = [filler(1), open, filler(2), ...fresh];

      expect(newEventsSince(before, after)).toEqual(fresh);
    });
  }

  it("two different events on a readable card are still not the same occurrence", () => {
    const a: GameEvent = { type: "counterChanged", instanceId: "c84", counter: "brittle", value: 2 };
    const b: GameEvent = { type: "counterChanged", instanceId: "c84", counter: "brittle", value: 1 };

    expect(sameOccurrence(a, b)).toBe(false);
  });

  it("a rewritten field is skipped only where the card was hidden: a Brittle count still has to agree between two open copies", () => {
    const open: GameEvent = { type: "counterChanged", instanceId: "c84", counter: "brittle", value: 2 };
    const hiddenOther: GameEvent = { type: "counterChanged", instanceId: HIDDEN_ID, counter: "plague", value: 3 };

    expect(sameOccurrence(open, { ...open })).toBe(true);
    expect(sameOccurrence(open, hiddenOther)).toBe(false);
  });
});
