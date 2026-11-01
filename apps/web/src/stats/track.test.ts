// From views to a game's log (SPEC R639): only what the viewer was shown is counted.

import type { GameEvent, GameOverReason } from "@jackioh/shared";
import { beforeEach, describe, expect, it } from "vitest";

import { ALLURE_MARK } from "../cards/marks.ts";
import { HIDDEN_ID } from "../game/animations.ts";
import { baseView, card, emptySide, faceDownBackrow, faceUpBackrow, resetIds, unit } from "../test/fixtures.ts";
import { EMPTY_LOG, logWith } from "./model.ts";
import { countAllured, countEvent, countHealing, countHits, countWin, observe, outcomeOfView, seenIn } from "./track.ts";

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
    const event: GameEvent = { type: "healthLost", player: "p1", amount: 3 };
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

describe("R1743000 the damage, attacks, healing, draws, Radiant changes and turns", () => {
  const hit = (sourceId: string | null, targetId: string, amount = 3): GameEvent => ({
    type: "damage",
    sourceId,
    targetId,
    amount,
    combat: true,
  });
  const attack = (attackerId: string, targetId = "hero-p2"): GameEvent => ({
    type: "attackDeclared",
    attackerId,
    targetId,
    forced: false,
  });
  const healed = (amount: number): GameEvent => ({ type: "healed", targetId: "hero-p1", amount });
  const activated = (player: "p1" | "p2", defId: string): GameEvent => ({
    type: "activated",
    player,
    instanceId: "i1",
    defId,
    ability: "a",
  });
  const sprung = (controller: "p1" | "p2", defId: string): GameEvent => ({
    type: "trapFired",
    instanceId: "t1",
    defId,
    controller,
    row: "backrow",
    lane: 0,
  });
  const drew = (player: "p1" | "p2", defId: string): GameEvent => ({ type: "drawn", player, instanceId: "d1", defId });
  const shone = (player: "p1" | "p2", defId: string): GameEvent => ({
    type: "radiantSet",
    instanceId: "r1",
    defId,
    zone: { z: "hand", player },
  });
  /** The viewer's Unit "mine" and a spell in their graveyard; the opponent's Unit "enemy". */
  const view = baseView({
    you: emptySide("p1", {
      units: [unit("p1", { instanceId: "mine", defId: "core-004" }), null, null, null, null],
      graveyard: [card({ instanceId: "bolt", defId: "core-030" })],
    }),
    opponent: emptySide("p2", {
      units: [unit("p2", { instanceId: "enemy", defId: "core-006", health: 4 }), null, null, null, null],
    }),
  });

  it("R1743000 the viewer's hits add their damage and keep the biggest; a hidden, null, zero or opposing source counts nothing", () => {
    const fresh = [
      hit("mine", "hero-p2", 3),
      hit("mine", "enemy", 5),
      hit("bolt", "u99", 2),
      hit(null, "hero-p2", 9),
      hit(HIDDEN_ID, "hero-p2", 9),
      hit("enemy", "hero-p1", 7),
      hit("mine", "hero-p2", 0),
      hit("nobody", "hero-p2", 8),
    ];
    const log = countHits(EMPTY_LOG, view, fresh, null);
    expect(log.damageDealt).toEqual({ "core-004": 8, "core-030": 2 });
    expect(log.biggestHit).toEqual({ "core-004": 5, "core-030": 2 });
    expect(countHits(EMPTY_LOG, view, [hit(null, "hero-p2"), hit("enemy", "hero-p1")], null)).toBe(EMPTY_LOG);
  });

  it("R1743000 overkill reads the target's health in the previous view less the hits before it in the batch, and none without one", () => {
    const before = baseView({
      opponent: emptySide("p2", {
        hero: { health: 5, armor: 0, powers: [], power: null },
        units: [unit("p2", { instanceId: "enemy", health: 4 }), null, null, null, null],
        carried: [unit("p2", { instanceId: "tucked", health: 1 })],
      }),
      you: emptySide("p1", { hero: { health: 2, armor: 0, powers: [], power: null } }),
    });
    const fresh = [
      hit("mine", "enemy", 3), // 4 left: not past it, 1 left
      hit("mine", "enemy", 4), // 1 left: 3 past it
      hit("mine", "hero-p2", 9), // 5 left: 4 past it
      hit("mine", "tucked", 7), // 1 left: 6 past it, a carried unit counts
      hit("mine", "u99", 50), // a target the view does not show: no overkill
      hit("mine", HIDDEN_ID, 50),
      hit("mine", "hero-p1", 2), // exactly its health: not past it
    ];
    expect(countHits(EMPTY_LOG, view, fresh, before).overkill).toEqual({ "core-004": 6 });
    expect(countHits(EMPTY_LOG, view, fresh.slice(0, 3), before).overkill).toEqual({ "core-004": 4 });
    // With no view before: damage and the biggest hit, but no overkill.
    const blind = countHits(EMPTY_LOG, view, fresh, null);
    expect(blind.overkill).toEqual({});
    expect(blind.biggestHit).toEqual({ "core-004": 50 });
    // A hit that did nothing (zero) leaves what its target had.
    expect(countHits(EMPTY_LOG, view, [hit("mine", "enemy", 0), hit("mine", "enemy", 5)], before).overkill).toEqual({ "core-004": 1 });
    // A hit of the opponent's takes health off a target too, but counts for no card of the viewer's.
    expect(countHits(EMPTY_LOG, view, [hit("enemy", "hero-p2", 4), hit("mine", "hero-p2", 4)], before).overkill).toEqual({ "core-004": 3 });
  });

  it("R1743000 only the viewer's readable attacks count", () => {
    const log = countHits(EMPTY_LOG, view, [attack("mine"), attack("mine", "enemy"), attack("enemy", "hero-p1"), attack(HIDDEN_ID), attack("gone")], null);
    expect(log.attacks).toEqual({ "core-004": 2 });
    expect(countHits(EMPTY_LOG, view, [attack("enemy", "hero-p1")], null)).toBe(EMPTY_LOG);
  });

  it("R1743000 a heal counts toward the viewer's last play, activation or trap before it; none, or the opponent's, counts nothing", () => {
    const count = (fresh: GameEvent[]): Record<string, number> => ({ ...countHealing(EMPTY_LOG, view, fresh).healing });
    expect(count([played("p1", "core-040"), healed(3), healed(2)])).toEqual({ "core-040": 5 });
    expect(count([played("p1", "core-040"), healed(3), activated("p1", "core-041"), healed(2)])).toEqual({ "core-040": 3, "core-041": 2 });
    expect(count([activated("p1", "core-041"), healed(1), sprung("p1", "core-020"), healed(4)])).toEqual({ "core-041": 1, "core-020": 4 });
    expect(count([healed(3)])).toEqual({});
    expect(count([healed(9), played("p1", "core-040")])).toEqual({});
    expect(count([played("p1", "core-040"), healed(0)])).toEqual({});
    // The opponent's (or a hidden) card is the source: the viewer's earlier one no longer is.
    expect(count([played("p1", "core-040"), played("p2", "core-050"), healed(3)])).toEqual({});
    expect(count([played("p1", "core-040"), activated("p2", "core-050"), healed(3)])).toEqual({});
    expect(count([played("p1", "core-040"), sprung("p2", HIDDEN_ID), healed(3)])).toEqual({});
    expect(count([played("p2", HIDDEN_ID), healed(3)])).toEqual({});
    expect(countHealing(EMPTY_LOG, view, [healed(3)])).toBe(EMPTY_LOG);
  });

  it("R1743000 only the viewer's draws that show the card and the viewer's Radiant changes count", () => {
    let log = countEvent(EMPTY_LOG, drew("p1", "core-001"), "p1");
    log = countEvent(log, drew("p1", "core-001"), "p1");
    log = countEvent(log, drew("p2", HIDDEN_ID), "p1");
    log = countEvent(log, drew("p2", "core-002"), "p1");
    expect(log.drawn).toEqual({ "core-001": 2 });
    expect(countEvent(EMPTY_LOG, drew("p1", HIDDEN_ID), "p1")).toBe(EMPTY_LOG);
    expect(countEvent(EMPTY_LOG, drew("p1", "core-001"), "p2")).toBe(EMPTY_LOG);

    let shining = countEvent(EMPTY_LOG, shone("p1", "core-003"), "p1");
    shining = countEvent(shining, shone("p2", "core-004"), "p1");
    shining = countEvent(shining, shone("p1", HIDDEN_ID), "p1");
    expect(shining.radiant).toEqual({ "core-003": 1 });
    expect(countEvent(EMPTY_LOG, shone("p2", HIDDEN_ID), "p1")).toBe(EMPTY_LOG);
    expect(countEvent(EMPTY_LOG, shone("p1", "core-003"), "p2")).toBe(EMPTY_LOG);
  });

  it("R1743000 a game's turns are the highest turn a turnStarted named", () => {
    const started = (turn: number): GameEvent => ({ type: "turnStarted", player: "p1", turn });
    const seven = countEvent(EMPTY_LOG, started(7), "p1");
    expect(seven.turns).toBe(7);
    expect(countEvent(seven, started(5), "p1")).toBe(seven);
    expect(countEvent(seven, started(7), "p1")).toBe(seven);
    expect(countEvent(seven, started(8), "p2").turns).toBe(8);
  });

  it("R1743000 observe counts them all from a view, its new events and the view before", () => {
    const before = baseView({
      opponent: emptySide("p2", { units: [unit("p2", { instanceId: "enemy", health: 2 }), null, null, null, null] }),
    });
    const log = observe(
      EMPTY_LOG,
      view,
      [
        { type: "turnStarted", player: "p1", turn: 9 },
        drew("p1", "core-001"),
        played("p1", "core-030"),
        attack("mine", "enemy"),
        hit("mine", "enemy", 5),
        healed(2),
        shone("p1", "core-004"),
      ],
      before,
    );
    expect(log).toMatchObject({
      turns: 9,
      drawn: { "core-001": 1 },
      attacks: { "core-004": 1 },
      damageDealt: { "core-004": 5 },
      biggestHit: { "core-004": 5 },
      overkill: { "core-004": 3 },
      healing: { "core-030": 2 },
      radiant: { "core-004": 1 },
    });
    // Without the view before, as for a seat's first view, the rest still counts.
    expect(observe(EMPTY_LOG, view, [hit("mine", "enemy", 5)]).overkill).toEqual({});
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
