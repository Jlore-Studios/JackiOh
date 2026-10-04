// From views to a game's log (SPEC R639): only what the viewer was shown is counted.

import type { GameEvent } from "@jackioh/shared";
import { beforeEach, describe, expect, it } from "vitest";

import { HIDDEN_ID } from "../game/animations.ts";
import { baseView, card, emptySide, faceDownBackrow, faceUpBackrow, resetIds, unit } from "../test/fixtures.ts";
import { EMPTY_LOG, logWith } from "./model.ts";
import { countEvent, observe, outcomeOfView, seenIn } from "./track.ts";

beforeEach(() => {
  resetIds();
});

const played = (player: "p1" | "p2", defId: string): GameEvent => ({
  type: "cardPlayed",
  player,
  instanceId: "i1",
  defId,
  costPaid: 1,
});

const destroyed = (owner: "p1" | "p2", defId: string): GameEvent => ({
  type: "destroyed",
  instanceId: "i2",
  defId,
  owner,
  controller: owner,
  attack: 1,
  maxHealth: 1,
  killerId: null,
});

describe("R639 what the viewer was shown", () => {
  it("R639 a view's cards are the viewer's hand, both boards, both graveyards, exile and what is resolving", () => {
    const view = baseView({
      you: emptySide("p1", {
        hand: [card({ defId: "core-001" })],
        graveyard: [card({ defId: "core-002" })],
        exile: [card({ defId: "core-003" })],
        resolving: [card({ defId: "core-004" })],
        units: [unit("p1", { defId: "core-005" }), null, null, null, null],
        backrow: [faceUpBackrow("p1", { defId: "core-006" }), null, null, null, null],
      }),
      opponent: emptySide("p2", {
        hand: { count: 5 },
        graveyard: [card({ defId: "core-007" })],
        units: [null, unit("p2", { defId: "core-008" }), null, null, null],
        backrow: [faceDownBackrow, faceUpBackrow("p2", { defId: "core-009" }), null, null, null],
      }),
    });
    expect(seenIn(view).sort()).toEqual(
      ["core-001", "core-002", "core-003", "core-004", "core-005", "core-006", "core-007", "core-008", "core-009"].sort(),
    );
  });

  it("R639 a card the view hides is never seen: the sentinel, a face-down trap and the opponent's hand name nothing", () => {
    const view = baseView({
      you: emptySide("p1", { hand: [card({ defId: HIDDEN_ID })] }),
      opponent: emptySide("p2", {
        hand: { count: 3 },
        backrow: [faceDownBackrow, null, null, null, null],
        graveyard: [card({ defId: HIDDEN_ID })],
      }),
    });
    expect(seenIn(view)).toEqual([]);
  });

  it("R639 a card in the viewer's own library is not in front of them, so it is not seen", () => {
    const view = baseView({
      you: emptySide("p1", { ownLibrary: { cards: [{ defId: "core-001", radiant: false, count: 3 }], unknown: 0 } }),
    });
    expect(seenIn(view)).toEqual([]);
  });
});

describe("R639 what the events count", () => {
  it("R639 a play is the viewer's own, or one they watched the opponent make", () => {
    let log = countEvent(EMPTY_LOG, played("p1", "core-001"), "p1");
    log = countEvent(log, played("p2", "core-002"), "p1");
    expect(log.played).toEqual({ "core-001": 1 });
    expect(log.playedAgainst).toEqual({ "core-002": 1 });
  });

  it("R639 a face-down play the sentinel hides counts for nobody", () => {
    expect(countEvent(EMPTY_LOG, played("p2", HIDDEN_ID), "p1")).toBe(EMPTY_LOG);
    expect(countEvent(EMPTY_LOG, destroyed("p2", HIDDEN_ID), "p1")).toBe(EMPTY_LOG);
  });

  it("R639 a destroyed card is the viewer's loss when it was theirs, and a takedown when it was the opponent's", () => {
    let log = countEvent(EMPTY_LOG, destroyed("p1", "core-001"), "p1");
    log = countEvent(log, destroyed("p2", "core-002"), "p1");
    expect(log.destroyed).toEqual({ "core-001": 1 });
    expect(log.defeated).toEqual({ "core-002": 1 });
  });

  it("R639 the same events read from the other seat swap the sides", () => {
    const log = countEvent(EMPTY_LOG, played("p1", "core-001"), "p2");
    expect(log.playedAgainst).toEqual({ "core-001": 1 });
    expect(log.played).toEqual({});
  });

  it("R639 other events change nothing", () => {
    const event: GameEvent = { type: "turnStarted", player: "p1", turn: 3 };
    expect(countEvent(EMPTY_LOG, event, "p1")).toBe(EMPTY_LOG);
  });
});

describe("R639 a view and its new events", () => {
  it("R639 observe adds what the view shows and what the new events say", () => {
    const view = baseView({ you: emptySide("p1", { hand: [card({ defId: "core-001" })] }) });
    const log = observe(logWith(EMPTY_LOG, "played", "core-009"), view, [played("p1", "core-001")]);
    expect(log.seen).toEqual(["core-001"]);
    expect(log.played).toEqual({ "core-009": 1, "core-001": 1 });
  });

  it("R639 the outcome reads from the viewer's seat, and is null while the game is on", () => {
    expect(outcomeOfView(baseView())).toBeNull();
    expect(outcomeOfView(baseView({ result: { winner: "p1", reason: "hero-death" } }))).toBe("win");
    expect(outcomeOfView(baseView({ result: { winner: "p2", reason: "concede" } }))).toBe("loss");
    expect(outcomeOfView(baseView({ result: { winner: "draw", reason: "turn-cap" } }))).toBe("draw");
  });
});
