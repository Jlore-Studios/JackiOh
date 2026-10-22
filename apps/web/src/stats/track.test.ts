// From views to a game's log (SPEC R639): only what the viewer was shown is counted.

import type { GameEvent, GameOverReason } from "@jackioh/shared";
import { beforeEach, describe, expect, it } from "vitest";

import { ALLURE_MARK } from "../cards/marks.ts";
import { HIDDEN_ID } from "../game/animations.ts";
import { baseView, card, emptySide, faceDownBackrow, faceUpBackrow, resetIds, unit } from "../test/fixtures.ts";
import { EMPTY_LOG, logWith } from "./model.ts";
import { countAllured, countEvent, countWin, observe, outcomeOfView, seenIn } from "./track.ts";

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

describe("R1582000 the new counters", () => {
  const lifted = (id: string, over: { mark?: string; added?: boolean } = {}): GameEvent => ({
    type: "marked",
    instanceId: id,
    mark: over.mark ?? ALLURE_MARK,
    color: "pink",
    added: over.added ?? false,
  });
  const stolen = (controller: "p1" | "p2", id: string): GameEvent => ({
    type: "controlChanged",
    instanceId: id,
    controller,
    row: "units",
    lane: 0,
    how: "steal",
  });
  const fired = (controller: "p1" | "p2", defId: string): GameEvent => ({
    type: "trapFired",
    instanceId: "t1",
    defId,
    controller,
    row: "backrow",
    lane: 0,
  });
  const hit = (sourceId: string | null, targetId: string, amount = 3): GameEvent => ({
    type: "damage",
    sourceId,
    targetId,
    amount,
    combat: true,
  });
  const over = (winner: "p1" | "p2" | "draw", reason: GameOverReason = "hero-death"): GameEvent => ({
    type: "gameOver",
    winner,
    reason,
  });

  it("R1582000 a card of the viewer's the opponent's Allure took counts once, named where the view shows it now", () => {
    const view = baseView({
      opponent: emptySide("p2", {
        units: [unit("p1", { instanceId: "taken", defId: "core-050", controller: "p2" }), null, null, null, null],
      }),
    });
    expect(countAllured(EMPTY_LOG, view, [stolen("p2", "taken"), lifted("taken")]).allured).toEqual({ "core-050": 1 });
    expect(countAllured(EMPTY_LOG, view, [lifted("taken"), stolen("p2", "taken")]).allured).toEqual({ "core-050": 1 });
  });

  it("R1582000 a steal no Allure mark left, the viewer's own Allure, an added mark and a face-down Trap taken count nothing", () => {
    const mine = baseView({
      you: emptySide("p1", {
        units: [unit("p2", { instanceId: "mine", defId: "core-050" }), null, null, null, null],
      }),
    });
    const view = baseView();
    expect(countAllured(EMPTY_LOG, view, [stolen("p2", "taken")])).toBe(EMPTY_LOG);
    expect(countAllured(EMPTY_LOG, view, [stolen("p2", "taken"), lifted("taken", { mark: "steal" })])).toBe(
      EMPTY_LOG,
    );
    expect(countAllured(EMPTY_LOG, view, [stolen("p2", "taken"), lifted("taken", { added: true })])).toBe(
      EMPTY_LOG,
    );
    expect(countAllured(EMPTY_LOG, mine, [stolen("p1", "mine"), lifted("mine")])).toBe(EMPTY_LOG);
    expect(countAllured(EMPTY_LOG, view, [stolen("p2", HIDDEN_ID), lifted(HIDDEN_ID)])).toBe(EMPTY_LOG);
  });

  it("R1582000 a Trap the viewer controlled counts each time it fires; one they did not control counts nothing", () => {
    const fromP1 = baseView();
    let log = countEvent(EMPTY_LOG, fired("p1", "core-020"), "p1");
    log = countEvent(log, fired("p1", "core-020"), "p1");
    expect(log.sprung).toEqual({ "core-020": 2 });
    expect(countEvent(EMPTY_LOG, fired("p2", HIDDEN_ID), "p1")).toBe(EMPTY_LOG);
    expect(countEvent(EMPTY_LOG, fired("p2", "core-021"), "p1")).toBe(EMPTY_LOG);
    expect(countEvent(EMPTY_LOG, fired("p2", "core-021"), "p2").sprung).toEqual({ "core-021": 1 });
    expect(fromP1.viewer).toBe("p1");
  });

  it("R1582000 a won game's end counts each Unit on the viewer's side as surviving, a copy each, and none of the opponent's", () => {
    const view = baseView({
      you: emptySide("p1", {
        units: [
          unit("p1", { defId: "core-004" }),
          unit("p1", { defId: "core-004" }),
          unit("p1", { defId: "core-005" }),
          null,
          null,
        ],
      }),
      opponent: emptySide("p2", {
        units: [unit("p2", { defId: "core-006" }), null, null, null, null],
      }),
    });
    expect(countWin(EMPTY_LOG, view, [over("p1")]).survived).toEqual({ "core-004": 2, "core-005": 1 });
  });

  it("R1582000 the card on the viewer's side that last damaged the opponent's hero before the end dealt the lethal blow", () => {
    const view = baseView({
      you: emptySide("p1", {
        units: [unit("p1", { instanceId: "attacker", defId: "core-004" }), null, null, null, null],
        graveyard: [card({ instanceId: "bolt", defId: "core-030" })],
      }),
    });
    const fresh: GameEvent[] = [
      hit("attacker", "hero-p2"),
      hit("bolt", "hero-p2", 2),
      hit("attacker", "hero-p2", 0),
      hit("attacker", "u99"),
      over("p1"),
    ];
    expect(countWin(EMPTY_LOG, view, fresh).lethal).toEqual({ "core-030": 1 });
  });

  it("R1582000 a loss, a draw, a concession, fatigue, a hidden source, an opponent's source or a finished view without the end event names no lethal blow", () => {
    const view = baseView({
      you: emptySide("p1", {
        units: [unit("p1", { instanceId: "attacker", defId: "core-004" }), null, null, null, null],
      }),
      opponent: emptySide("p2", {
        units: [unit("p2", { instanceId: "enemy", defId: "core-006" }), null, null, null, null],
      }),
    });
    expect(countWin(EMPTY_LOG, view, [hit("attacker", "hero-p2"), over("p2")])).toBe(EMPTY_LOG);
    expect(countWin(EMPTY_LOG, view, [over("draw", "both-heroes-dead")])).toBe(EMPTY_LOG);
    expect(countWin(EMPTY_LOG, view, [hit("attacker", "hero-p2"), over("p1", "concede")]).lethal).toEqual({});
    expect(countWin(EMPTY_LOG, view, [hit(null, "hero-p2"), over("p1")]).lethal).toEqual({});
    expect(countWin(EMPTY_LOG, view, [hit(HIDDEN_ID, "hero-p2"), over("p1")]).lethal).toEqual({});
    expect(countWin(EMPTY_LOG, view, [hit("enemy", "hero-p2"), over("p1")]).lethal).toEqual({});
    expect(countWin(EMPTY_LOG, baseView({ result: { winner: "p1", reason: "hero-death" } }), [])).toBe(EMPTY_LOG);
  });

  it("R1582000 observe counts all four from a view and its new events", () => {
    const view = baseView({
      you: emptySide("p1", {
        units: [unit("p1", { instanceId: "attacker", defId: "core-004" }), null, null, null, null],
      }),
      opponent: emptySide("p2", {
        units: [unit("p1", { instanceId: "taken", defId: "core-050", controller: "p2" }), null, null, null, null],
      }),
    });
    const log = observe(
      EMPTY_LOG,
      view,
      [fired("p1", "core-020"), stolen("p2", "taken"), lifted("taken"), hit("attacker", "hero-p2"), over("p1")],
    );
    expect(log.sprung).toEqual({ "core-020": 1 });
    expect(log.allured).toEqual({ "core-050": 1 });
    expect(log.lethal).toEqual({ "core-004": 1 });
    expect(log.survived).toEqual({ "core-004": 1 });
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
