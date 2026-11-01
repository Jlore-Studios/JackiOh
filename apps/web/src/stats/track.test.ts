// From views to a game's log (SPEC R639): only what the viewer was shown is counted.

import type { GameEvent, GameOverReason, PlayerView } from "@jackioh/shared";
import { beforeEach, describe, expect, it } from "vitest";

import { ALLURE_MARK, FROZEN_MARK } from "../cards/marks.ts";
import { HIDDEN_ID } from "../game/animations.ts";
import { baseView, card, emptySide, faceDownBackrow, faceUpBackrow, resetIds, unit } from "../test/fixtures.ts";
import { EMPTY_LOG, logWith, type GameLog } from "./model.ts";
import {
  countAllured,
  countCreated,
  countDeaths,
  countDoubleAttacks,
  countEvent,
  countFengShuiAndCounters,
  countFreezes,
  countHealing,
  countHits,
  countKills,
  countReborn,
  countSacrificed,
  countTaken,
  countTemporary,
  countTrades,
  countWin,
  observe,
  outcomeOfView,
  seenIn,
} from "./track.ts";

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

const destroyed = (owner: "p1" | "p2", defId: string, killerId: string | null = null, instanceId = "i2"): GameEvent => ({
  type: "destroyed",
  instanceId,
  defId,
  owner,
  controller: owner,
  attack: 1,
  maxHealth: 1,
  killerId,
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
    expect(countDeaths(EMPTY_LOG, baseView(), [played("p1", "core-001"), destroyed("p2", HIDDEN_ID)], null)).toBe(EMPTY_LOG);
  });

  it("R639 R2648000 a destroyed card the other seat caused is the viewer's loss when it was theirs, and a takedown when it was the opponent's", () => {
    let log = countDeaths(EMPTY_LOG, baseView(), [played("p2", "core-009"), destroyed("p1", "core-001")], null);
    log = countDeaths(log, baseView(), [played("p1", "core-009"), destroyed("p2", "core-002")], null);
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
    // The same hits with no view before count their damage and their biggest, and no overkill.
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
    // No card before it in the batch, or a zero heal: nothing. A play after it is not its source.
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
    // An older or equal turn is the very same log; a later one raises it.
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
      healing: { "core-004": 2 },
      radiant: { "core-004": 1 },
    });
    // Without the view before, as for a seat's first view, the rest still counts.
    expect(observe(EMPTY_LOG, view, [hit("mine", "enemy", 5)]).overkill).toEqual({});
  });
});

/** R2648000's board: the viewer's Unit "mine" and a spell "bolt" in their graveyard; the opponent's Unit "enemy". */
const board = baseView({
  you: emptySide("p1", {
    units: [unit("p1", { instanceId: "mine", defId: "core-004" }), null, null, null, null],
    graveyard: [card({ instanceId: "bolt", defId: "core-030" })],
  }),
  opponent: emptySide("p2", {
    units: [unit("p2", { instanceId: "enemy", defId: "core-007", health: 4 }), null, null, null, null],
  }),
});

const attacked = (attackerId: string, targetId: string): GameEvent => ({ type: "attackDeclared", attackerId, targetId, forced: false });

describe("R2648000 a death counts only when the other seat caused it", () => {
  const crumbled = (owner: "p1" | "p2", instanceId = "i2"): GameEvent => ({
    type: "crumbled",
    instanceId,
    defId: "core-001",
    owner,
    zone: "field",
  });
  const activated = (player: "p1" | "p2"): GameEvent => ({ type: "activated", player, instanceId: "i1", defId: "core-041", ability: "a" });
  const fired = (controller: "p1" | "p2"): GameEvent => ({
    type: "trapFired",
    instanceId: "t1",
    defId: "core-060",
    controller,
    row: "backrow",
    lane: 0,
  });
  const deaths = (fresh: GameEvent[], before: PlayerView | null = null, view: PlayerView = board): GameLog =>
    countDeaths(EMPTY_LOG, view, fresh, before);

  it("R2648000 a Tribute's and a Sacrifice's deaths, the owner's own play with no killer, count neither destroyed nor defeated", () => {
    // R119: a Tribute's death comes before its play.
    expect(deaths([destroyed("p1", "core-001"), played("p1", "core-021")])).toBe(EMPTY_LOG);
    expect(deaths([played("p1", "core-021"), destroyed("p1", "core-001")])).toBe(EMPTY_LOG);
    expect(deaths([activated("p1"), destroyed("p1", "core-001")])).toBe(EMPTY_LOG);
    expect(deaths([destroyed("p2", "core-007"), played("p2", "core-021")])).toBe(EMPTY_LOG);
    expect(deaths([played("p2", "core-021"), destroyed("p2", "core-007")])).toBe(EMPTY_LOG);
    expect(deaths([activated("p2"), destroyed("p2", "core-007")])).toBe(EMPTY_LOG);
    // A killer on the owner's own side is no cause from the other.
    expect(deaths([played("p1", "core-021"), destroyed("p1", "core-001", "bolt")])).toBe(EMPTY_LOG);
    // No actor and no killer at all: nobody caused it.
    expect(deaths([destroyed("p1", "core-001")])).toBe(EMPTY_LOG);
  });

  it("R2648000 a Brittle crumble counts neither", () => {
    expect(deaths([played("p2", "core-021"), crumbled("p1"), destroyed("p1", "core-001")])).toBe(EMPTY_LOG);
    expect(deaths([played("p1", "core-021"), crumbled("p2"), destroyed("p2", "core-007")])).toBe(EMPTY_LOG);
    // A crumble of another card leaves this death to count.
    expect(deaths([played("p2", "core-021"), crumbled("p1", "other"), destroyed("p1", "core-001")]).destroyed).toEqual({ "core-001": 1 });
  });

  it("R2648000 an enemy attacker's kill and an enemy destroy Spell's count destroyed", () => {
    expect(deaths([attacked("enemy", "mine"), destroyed("p1", "core-004", "enemy", "mine")]).destroyed).toEqual({ "core-004": 1 });
    // A destroy Spell names no killer (R89): the opponent's last play, ability or Trap is the cause.
    expect(deaths([played("p2", "core-021"), destroyed("p1", "core-001")]).destroyed).toEqual({ "core-001": 1 });
    expect(deaths([activated("p2"), destroyed("p1", "core-001")]).destroyed).toEqual({ "core-001": 1 });
    expect(deaths([fired("p2"), destroyed("p1", "core-001")]).destroyed).toEqual({ "core-001": 1 });
    // A trade: each killer is looked up in the view before the batch, though both died in it.
    const trade = deaths(
      [attacked("mine", "enemy"), destroyed("p1", "core-004", "enemy", "mine"), destroyed("p2", "core-007", "mine", "enemy")],
      board,
      baseView(),
    );
    expect(trade.destroyed).toEqual({ "core-004": 1 });
    expect(trade.defeated).toEqual({ "core-007": 1 });
    expect(observe(EMPTY_LOG, board, [played("p2", "core-021"), destroyed("p1", "core-001")]).destroyed).toEqual({ "core-001": 1 });
  });

  it("R2648000 the mirror cases count defeated", () => {
    expect(deaths([attacked("mine", "enemy"), destroyed("p2", "core-007", "mine", "enemy")]).defeated).toEqual({ "core-007": 1 });
    expect(deaths([played("p1", "core-021"), destroyed("p2", "core-007")]).defeated).toEqual({ "core-007": 1 });
    expect(deaths([activated("p1"), destroyed("p2", "core-007")]).defeated).toEqual({ "core-007": 1 });
    expect(deaths([fired("p1"), destroyed("p2", "core-007")]).defeated).toEqual({ "core-007": 1 });
    expect(deaths([destroyed("p2", "core-007", "bolt")]).defeated).toEqual({ "core-007": 1 });
    // A hidden card names nothing, and a killer or attacker the views do not show is no cause.
    expect(deaths([played("p1", "core-021"), destroyed("p2", HIDDEN_ID)])).toBe(EMPTY_LOG);
    expect(deaths([destroyed("p2", "core-007", "gone")])).toBe(EMPTY_LOG);
    expect(deaths([attacked("gone", "enemy"), destroyed("p2", "core-007")])).toBe(EMPTY_LOG);
  });
});

describe("R2648000 a Lifesteal hit's heal", () => {
  const hit = (sourceId: string | null, targetId: string, amount: number): GameEvent => ({
    type: "damage",
    sourceId,
    targetId,
    amount,
    combat: true,
  });
  const healed = (amount: number): GameEvent => ({ type: "healed", targetId: "hero-p1", amount });
  const count = (fresh: GameEvent[]): Record<string, number> => ({ ...countHealing(EMPTY_LOG, board, fresh).healing });

  it("R2648000 a Lifesteal Unit's attack credits its healed to the attacker, and the opponent's attack credits nothing", () => {
    expect(count([attacked("mine", "enemy"), hit("mine", "enemy", 3), healed(3)])).toEqual({ "core-004": 3 });
    // It replaces the play before it as the source.
    expect(count([played("p1", "core-040"), attacked("mine", "enemy"), hit("mine", "enemy", 3), healed(3)])).toEqual({ "core-004": 3 });
    // A hit alone names its source, the hero it struck included.
    expect(count([hit("mine", "hero-p2", 4), healed(4)])).toEqual({ "core-004": 4 });
    // The opponent's attack clears the viewer's earlier play as the source.
    expect(count([played("p1", "core-040"), attacked("enemy", "hero-p1"), hit("enemy", "hero-p1", 3), healed(3)])).toEqual({});
    // An enemy hit after the viewer's own takes the source, so its heal is not the viewer's card's.
    expect(count([attacked("mine", "enemy"), hit("mine", "enemy", 2), healed(2), hit("enemy", "mine", 3), healed(3)])).toEqual({
      "core-004": 2,
    });
  });
});

describe("R2649000 the game's own mechanics", () => {
  const playedAs = (player: "p1" | "p2", defId: string, instanceId: string, over: Partial<Extract<GameEvent, { type: "cardPlayed" }>> = {}): GameEvent => ({
    type: "cardPlayed",
    player,
    instanceId,
    defId,
    costPaid: 1,
    ...over,
  });
  const killed = (owner: "p1" | "p2", defId: string, killerId: string | null, instanceId: string, maxHealth = 4): GameEvent => ({
    type: "destroyed",
    instanceId,
    defId,
    owner,
    controller: owner,
    attack: 1,
    maxHealth,
    killerId,
  });
  const hit = (sourceId: string | null, targetId: string, amount: number, absorbed?: number): GameEvent => ({
    type: "damage",
    sourceId,
    targetId,
    amount,
    combat: true,
    ...(absorbed === undefined ? {} : { absorbed }),
  });
  const healed = (amount: number): GameEvent => ({ type: "healed", targetId: "hero-p1", amount });
  const ended = (player: "p1" | "p2"): GameEvent => ({ type: "turnEnded", player, turn: 3, unspentMana: 0 });
  const started = (player: "p1" | "p2", turn = 4): GameEvent => ({ type: "turnStarted", player, turn });
  const discarded = (owner: "p1" | "p2", instanceId: string, defId: string): GameEvent => ({ type: "discarded", instanceId, defId, owner });
  const frost = (instanceId: string, added = true): GameEvent => ({ type: "marked", instanceId, mark: FROZEN_MARK, color: "cyan", added });
  const crumbledOf = (owner: "p1" | "p2", instanceId: string, defId = "core-001"): GameEvent => ({ type: "crumbled", instanceId, defId, owner, zone: "field" });
  const traded = (player: "p1" | "p2"): GameEvent => ({ type: "traded", player });
  const shuffled = (player: "p1" | "p2"): GameEvent => ({ type: "shuffledIn", player, instanceId: HIDDEN_ID, defId: HIDDEN_ID, position: 0 });
  const drew = (player: "p1" | "p2", defId: string): GameEvent => ({ type: "drawn", player, instanceId: "d1", defId });
  const summoned = (player: "p1" | "p2", instanceId: string, defId: string): GameEvent => ({ type: "summoned", player, instanceId, defId, row: "units", lane: 0 });
  const countered = (player: "p1" | "p2", defId: string, byInstanceId: string | null): GameEvent => ({
    type: "countered",
    player,
    instanceId: "played",
    defId,
    byInstanceId,
    to: "graveyard",
  });
  const activated = (player: "p1" | "p2", defId = "core-041"): GameEvent => ({ type: "activated", player, instanceId: "i1", defId, ability: "a" });
  /** The viewer's hand in the view before: a granted Temporary, a printed one (R243: no `keywords`), and a plain card. */
  const handBefore = baseView({
    you: emptySide("p1", {
      hand: [
        card({ instanceId: "temp", defId: "core-005", keywords: [{ kind: "Temporary" }] }),
        card({ instanceId: "printed", defId: "meditative-022-1" }),
        card({ instanceId: "plain", defId: "core-001" }),
      ],
    }),
  });

  it("R2649000 Use It or Lose It: a Temporary card discarded at the viewer's own turn end counts; a plain one, another time's, the opponent's, a hidden one and no view before count nothing", () => {
    const count = (fresh: GameEvent[], before: PlayerView | null = handBefore, view: PlayerView = baseView()): GameLog =>
      countTemporary(EMPTY_LOG, view, fresh, before);
    expect(count([ended("p1"), discarded("p1", "temp", "core-005"), discarded("p1", "printed", "meditative-022-1")]).tempExpired).toEqual({
      "core-005": 1,
      "meditative-022-1": 1,
    });
    // A non-Temporary discard, one before the turn's end, one after the next turn starts, or at the opponent's turn end.
    expect(count([ended("p1"), discarded("p1", "plain", "core-001")])).toBe(EMPTY_LOG);
    expect(count([discarded("p1", "temp", "core-005"), ended("p1")])).toBe(EMPTY_LOG);
    expect(count([ended("p1"), started("p2"), discarded("p1", "temp", "core-005")])).toBe(EMPTY_LOG);
    expect(count([ended("p2"), discarded("p1", "temp", "core-005")])).toBe(EMPTY_LOG);
    // The opponent's mirror case, the sentinel and no view before.
    expect(count([ended("p2"), discarded("p2", "temp", "core-005")])).toBe(EMPTY_LOG);
    expect(count([ended("p1"), discarded("p1", "temp", HIDDEN_ID)])).toBe(EMPTY_LOG);
    expect(count([ended("p1"), discarded("p1", "temp", "core-005")], null)).toBe(EMPTY_LOG);
  });

  it("R2649000 Just in Time: the viewer's play of a card Temporary in their hand in the view before counts; a plain one, the opponent's and no view before count nothing", () => {
    const count = (fresh: GameEvent[], before: PlayerView | null = handBefore): GameLog => countTemporary(EMPTY_LOG, baseView(), fresh, before);
    expect(count([playedAs("p1", "core-005", "temp"), playedAs("p1", "meditative-022-1", "printed")]).tempPlayed).toEqual({
      "core-005": 1,
      "meditative-022-1": 1,
    });
    expect(count([playedAs("p1", "core-001", "plain")])).toBe(EMPTY_LOG);
    expect(count([playedAs("p2", "core-005", "temp")])).toBe(EMPTY_LOG);
    expect(count([playedAs("p1", HIDDEN_ID, "temp")])).toBe(EMPTY_LOG);
    expect(count([playedAs("p1", "core-005", "temp")], null)).toBe(EMPTY_LOG);
    // R227: a Temporary Trap set face-down takes a fresh id; its play carries the hand id as `formerId`.
    expect(count([playedAs("p1", "core-005", "set1", { formerId: "temp" })]).tempPlayed).toEqual({ "core-005": 1 });
    expect(count([playedAs("p1", "core-001", "set2", { formerId: "plain" })])).toBe(EMPTY_LOG);
    // A hand the view before does not show (an opponent's count) holds nothing Temporary.
    expect(count([playedAs("p1", "core-005", "temp")], baseView({ you: emptySide("p1", { hand: { count: 3 } }) }))).toBe(EMPTY_LOG);
  });

  it("R2649000 Sand Castle and Outlasted: a crumble is the viewer's card's or the opponent's, from the viewer's seat, and the sentinel names nothing", () => {
    let log = countEvent(EMPTY_LOG, crumbledOf("p1", "a", "classicplus-061"), "p1");
    log = countEvent(log, crumbledOf("p2", "b", "stinkyscape-009"), "p1");
    expect(log.crumbled).toEqual({ "classicplus-061": 1 });
    expect(log.crumbledAgainst).toEqual({ "stinkyscape-009": 1 });
    const mirror = countEvent(EMPTY_LOG, crumbledOf("p1", "a", "classicplus-061"), "p2");
    expect([mirror.crumbled, mirror.crumbledAgainst]).toEqual([{}, { "classicplus-061": 1 }]);
    expect(countEvent(EMPTY_LOG, crumbledOf("p2", HIDDEN_ID, HIDDEN_ID), "p1")).toBe(EMPTY_LOG);
  });

  it("R2649000 Good and Bad Feng Shui: a judgement of the viewer's play counts for its card where the view shows it; the opponent's and a hidden card count nothing", () => {
    const judged = (player: "p1" | "p2", instanceId: string, outcome: "positive" | "negative"): GameEvent => ({
      type: "fengShui",
      instanceId,
      sourceId: "judge",
      player,
      outcome,
    });
    const count = (fresh: GameEvent[], before: PlayerView | null = null): GameLog => countFengShuiAndCounters(EMPTY_LOG, board, fresh, before);
    const log = count([judged("p1", "mine", "positive"), judged("p1", "bolt", "negative"), judged("p1", "bolt", "negative")]);
    expect(log.fengShuiGood).toEqual({ "core-004": 1 });
    expect(log.fengShuiBad).toEqual({ "core-030": 2 });
    // A card gone from this view is named where the view before showed it.
    const before = baseView({ you: emptySide("p1", { units: [unit("p1", { instanceId: "gone", defId: "core-002" }), null, null, null, null] }) });
    expect(count([judged("p1", "gone", "negative")], before).fengShuiBad).toEqual({ "core-002": 1 });
    expect(count([judged("p2", "enemy", "positive")])).toBe(EMPTY_LOG);
    expect(count([judged("p1", HIDDEN_ID, "positive")])).toBe(EMPTY_LOG);
  });

  it("R2649000 Hot Potato: each viewer trade takes one Tradable card of the hand before that the view shows nowhere; the opponent's trade, a card still shown and no view before count nothing", () => {
    const before = baseView({
      you: emptySide("p1", {
        hand: [
          card({ instanceId: "h1", defId: "meditative-053", tradable: true }),
          card({ instanceId: "h2", defId: "stinkyscape-006", tradable: true }),
          card({ instanceId: "h3", defId: "core-002" }),
        ],
      }),
    });
    const after = (...ids: string[]): PlayerView =>
      baseView({ you: emptySide("p1", { hand: [...ids.map((id) => card({ instanceId: id, defId: "core-001" })), card({ instanceId: "new", defId: "core-009" })] }) });
    // The shuffle-in is the sentinel to the trader too (R1838001), so the card is the one the hand lost.
    expect(countTrades(EMPTY_LOG, after("h2", "h3"), [traded("p1"), shuffled("p1"), drew("p1", "core-009")], before).traded).toEqual({ "meditative-053": 1 });
    // Two trades, two cards gone.
    const twice = countTrades(EMPTY_LOG, after("h3"), [traded("p1"), shuffled("p1"), drew("p1", "core-009"), traded("p1"), shuffled("p1"), drew("p1", "core-008")], before);
    expect(twice.traded).toEqual({ "meditative-053": 1, "stinkyscape-006": 1 });
    // No more cards than trades: one trade with both gone takes the first in hand order.
    expect(countTrades(EMPTY_LOG, after("h3"), [traded("p1")], before).traded).toEqual({ "meditative-053": 1 });
    // A card the view still shows (played into a pile, say) was not traded; a plain card never is.
    expect(countTrades(EMPTY_LOG, after("h1", "h2"), [traded("p1")], before)).toBe(EMPTY_LOG);
    // The opponent's trade, no trade, the sentinel and no view before.
    expect(countTrades(EMPTY_LOG, after("h3"), [traded("p2"), shuffled("p2")], before)).toBe(EMPTY_LOG);
    expect(countTrades(EMPTY_LOG, after("h3"), [], before)).toBe(EMPTY_LOG);
    const hidden = baseView({ you: emptySide("p1", { hand: [card({ instanceId: "h9", defId: HIDDEN_ID, tradable: true })] }) });
    expect(countTrades(EMPTY_LOG, after(), [traded("p1")], hidden)).toBe(EMPTY_LOG);
    expect(countTrades(EMPTY_LOG, after("h3"), [traded("p1")], null)).toBe(EMPTY_LOG);
  });

  it("R2649000 Trade-In Value: the viewer's first draw after each of their trades counts; a draw before it, a second one, the opponent's and a hidden one count nothing", () => {
    const count = (fresh: GameEvent[]): GameLog => countTrades(EMPTY_LOG, baseView(), fresh, null);
    expect(count([traded("p1"), shuffled("p1"), drew("p1", "core-009"), drew("p1", "core-008")]).tradeDrawn).toEqual({ "core-009": 1 });
    expect(count([traded("p1"), drew("p1", "core-009"), traded("p1"), drew("p1", "core-009")]).tradeDrawn).toEqual({ "core-009": 2 });
    expect(count([drew("p1", "core-009"), traded("p1")])).toBe(EMPTY_LOG);
    expect(count([traded("p2"), drew("p2", "core-009")])).toBe(EMPTY_LOG);
    expect(count([traded("p1"), drew("p2", HIDDEN_ID)])).toBe(EMPTY_LOG);
    expect(count([traded("p1"), drew("p1", HIDDEN_ID)])).toBe(EMPTY_LOG);
  });

  it("R2649000 Mass Produced: a Created card on the viewer's side counts once however many views show it or zones it passes, two of one card count 2, and a dealt card, the opponent's and a hidden one count nothing", () => {
    const made = (zone: "hand" | "graveyard" | "units", instanceId: string, defId = "core-002", created = true): PlayerView =>
      baseView({
        you: emptySide("p1", {
          hand: zone === "hand" ? [card({ instanceId, defId, ...(created ? { created: true as const } : {}) })] : [],
          graveyard: zone === "graveyard" ? [card({ instanceId, defId, ...(created ? { created: true as const } : {}) })] : [],
          units: zone === "units" ? [unit("p1", { instanceId, defId, ...(created ? { created: true as const } : {}) }), null, null, null, null] : [null, null, null, null, null],
        }),
      });
    let log = countCreated(EMPTY_LOG, made("hand", "m1"));
    log = countCreated(log, made("hand", "m1"));
    log = countCreated(log, made("units", "m1"));
    log = countCreated(log, made("graveyard", "m1"));
    expect(log.created).toEqual({ "core-002": 1 });
    expect(countCreated(log, made("units", "m2")).created).toEqual({ "core-002": 2 });
    // A dealt card, the opponent's Created card, a face-down one the viewer cannot read and the sentinel.
    expect(countCreated(EMPTY_LOG, made("hand", "d1", "core-002", false))).toBe(EMPTY_LOG);
    const theirs = baseView({
      opponent: emptySide("p2", {
        units: [unit("p2", { instanceId: "o1", defId: "core-002", created: true }), null, null, null, null],
        backrow: [faceDownBackrow, null, null, null, null],
      }),
    });
    expect(countCreated(EMPTY_LOG, theirs)).toBe(EMPTY_LOG);
    expect(countCreated(EMPTY_LOG, made("hand", "h1", HIDDEN_ID))).toBe(EMPTY_LOG);
    // A face-up backrow card of the viewer's counts, a token too (it is never shown, R1743000).
    const backrow = baseView({ you: emptySide("p1", { backrow: [faceUpBackrow("p1", { instanceId: "b9", defId: "core-t-coin", created: true }), null, null, null, null] }) });
    expect(countCreated(EMPTY_LOG, backrow).created).toEqual({ "core-t-coin": 1 });
    // R227: a Created card set face-down, then shown again under its fresh id, is the same card and
    // counts once; the fresh id of a card never counted proves nothing.
    const setFaceDown: GameEvent = { type: "cardPlayed", player: "p1", instanceId: "s1", defId: "core-002", costPaid: 0, formerId: "m1" };
    const seen = countCreated(EMPTY_LOG, made("hand", "m1"));
    const set = countCreated(seen, baseView(), [setFaceDown]);
    expect(set.createdIds).toContain("s1");
    expect(countCreated(set, made("graveyard", "s1")).created).toEqual({ "core-002": 1 });
    const summonedFaceDown: GameEvent = { type: "summoned", player: "p1", instanceId: "s2", defId: "core-002", row: "units", lane: 0, formerId: "m1" };
    expect(countCreated(seen, made("units", "s2"), [summonedFaceDown]).created).toEqual({ "core-002": 1 });
    expect(countCreated(EMPTY_LOG, made("graveyard", "s1"), [setFaceDown]).created).toEqual({ "core-002": 1 });
  });

  it("R2649000 Executioner and Giant Slayer: an opponent's card the viewer's card killed counts a kill and its biggest maxHealth; the opponent's kill, a hidden killer, a crumble and no killer count nothing", () => {
    const count = (fresh: GameEvent[], before: PlayerView | null = null, view: PlayerView = board): GameLog => countKills(EMPTY_LOG, view, fresh, before);
    const log = count([attacked("mine", "enemy"), killed("p2", "core-007", "mine", "enemy", 4), killed("p2", "core-002", "bolt", "e2", 7), killed("p2", "core-009", "mine", "e3", 2)]);
    expect(log.kills).toEqual({ "core-004": 2, "core-030": 1 });
    expect(log.biggestKill).toEqual({ "core-004": 4, "core-030": 7 });
    // The killer died in the same combat: it is named where the view before showed it.
    expect(count([killed("p2", "core-007", "mine", "enemy")], board, baseView()).kills).toEqual({ "core-004": 1 });
    expect(count([killed("p2", "core-007", "mine", "enemy")], null, baseView())).toBe(EMPTY_LOG);
    // The opponent's mirror, a hidden killer, a killer the views do not show, a crumble and a destroy with no killer.
    expect(count([attacked("enemy", "mine"), killed("p1", "core-004", "enemy", "mine")])).toBe(EMPTY_LOG);
    expect(count([killed("p2", "core-007", HIDDEN_ID, "enemy")])).toBe(EMPTY_LOG);
    expect(count([killed("p2", HIDDEN_ID, "mine", HIDDEN_ID)])).toBe(EMPTY_LOG);
    expect(count([killed("p2", "core-007", "gone", "enemy")])).toBe(EMPTY_LOG);
    expect(count([crumbledOf("p2", "enemy"), killed("p2", "core-007", "mine", "enemy")])).toBe(EMPTY_LOG);
    expect(count([playedAs("p1", "core-021", "i1"), killed("p2", "core-007", null, "enemy")])).toBe(EMPTY_LOG);
  });

  it("R2649000 Shatter: an opponent's Unit Frozen in the view before that the viewer's card killed, or with no killer that the viewer's last actor did; a thawed one, the opponent's killer and no view before count nothing", () => {
    const frozenBefore = baseView({
      you: board.you,
      opponent: emptySide("p2", { units: [unit("p2", { instanceId: "enemy", defId: "core-007", frozen: true }), null, null, null, null] }),
    });
    const thawedBefore = baseView({ you: board.you, opponent: board.opponent });
    const count = (fresh: GameEvent[], before: PlayerView | null = frozenBefore): GameLog => countKills(EMPTY_LOG, board, fresh, before);
    expect(count([attacked("mine", "enemy"), killed("p2", "core-007", "mine", "enemy")]).shattered).toEqual({ "core-004": 1 });
    expect(count([playedAs("p1", "core-021", "i1"), killed("p2", "core-007", null, "enemy")]).shattered).toEqual({ "core-021": 1 });
    expect(count([activated("p1", "core-041"), killed("p2", "core-007", null, "enemy")]).shattered).toEqual({ "core-041": 1 });
    // Not Frozen before, the opponent's own killer or actor, a hidden actor, and no view before.
    expect(count([attacked("mine", "enemy"), killed("p2", "core-007", "mine", "enemy")], thawedBefore).shattered).toEqual({});
    expect(count([killed("p2", "core-007", "enemy", "enemy")])).toBe(EMPTY_LOG);
    expect(count([playedAs("p1", "core-021", "i1"), playedAs("p2", "core-022", "i2"), killed("p2", "core-007", null, "enemy")])).toBe(EMPTY_LOG);
    expect(count([playedAs("p1", HIDDEN_ID, "i1"), killed("p2", "core-007", null, "enemy")])).toBe(EMPTY_LOG);
    expect(count([attacked("mine", "enemy"), killed("p2", "core-007", "mine", "enemy")], null).shattered).toEqual({});
  });

  it("R2649000 Second Wind: the viewer's Unit with Reborn in the view before that died and was summoned again counts; one without Reborn, another card's summon, the opponent's and no view before count nothing", () => {
    const before = baseView({
      you: emptySide("p1", {
        units: [
          unit("p1", { instanceId: "phoenix", defId: "core-003", keywords: [{ kind: "Reborn" }] }),
          unit("p1", { instanceId: "plain", defId: "core-001" }),
          null,
          null,
          null,
        ],
      }),
      opponent: emptySide("p2", { units: [unit("p2", { instanceId: "theirs", defId: "core-003", keywords: [{ kind: "Reborn" }] }), null, null, null, null] }),
    });
    const count = (fresh: GameEvent[], from: PlayerView | null = before): GameLog => countReborn(EMPTY_LOG, baseView(), fresh, from);
    expect(count([killed("p1", "core-003", "enemy", "phoenix"), summoned("p1", "phoenix-2", "core-003")]).reborn).toEqual({ "core-003": 1 });
    expect(count([killed("p1", "core-003", "enemy", "phoenix"), summoned("p1", "x", "core-003"), summoned("p1", "y", "core-003")]).reborn).toEqual({ "core-003": 1 });
    expect(count([summoned("p1", "phoenix-2", "core-003"), killed("p1", "core-003", "enemy", "phoenix")])).toBe(EMPTY_LOG);
    expect(count([killed("p1", "core-001", "enemy", "plain"), summoned("p1", "plain-2", "core-001")])).toBe(EMPTY_LOG);
    expect(count([killed("p1", "core-003", "enemy", "phoenix"), summoned("p1", "other", "core-002")])).toBe(EMPTY_LOG);
    expect(count([killed("p2", "core-003", "mine", "theirs"), summoned("p2", "theirs-2", "core-003")])).toBe(EMPTY_LOG);
    expect(count([killed("p1", HIDDEN_ID, "enemy", HIDDEN_ID), summoned("p1", HIDDEN_ID, HIDDEN_ID)])).toBe(EMPTY_LOG);
    expect(count([killed("p1", "core-003", "enemy", "phoenix"), summoned("p1", "phoenix-2", "core-003")], null)).toBe(EMPTY_LOG);
  });

  it("R2649000 Bubble Wrap: a shield the viewer's Unit lost counts; the opponent's and the sentinel count nothing", () => {
    const lost = (instanceId: string): GameEvent => ({ type: "divineShieldLost", instanceId });
    expect(countTaken(EMPTY_LOG, board, [lost("mine"), lost("mine")], null).shieldsLost).toEqual({ "core-004": 2 });
    expect(countTaken(EMPTY_LOG, board, [lost("enemy"), lost(HIDDEN_ID), lost("gone")], null)).toBe(EMPTY_LOG);
  });

  it("R2649000 Iron Wall: the Armor of the viewer's Unit, a hit's `absorbed` and a hit absorbed whole; the opponent's and the sentinel count nothing", () => {
    const absorbed = (targetId: string, amount: number): GameEvent => ({ type: "damageAbsorbed", sourceId: "enemy", targetId, absorbed: amount, combat: true });
    expect(countTaken(EMPTY_LOG, board, [hit("enemy", "mine", 1, 2), absorbed("mine", 4), hit("enemy", "mine", 3)], null).armorAbsorbed).toEqual({ "core-004": 6 });
    expect(countTaken(EMPTY_LOG, board, [hit("mine", "enemy", 1, 2), absorbed("enemy", 4), absorbed(HIDDEN_ID, 4), absorbed("hero-p1", 4)], null).armorAbsorbed).toEqual({});
  });

  it("R2649000 Punching Bag: the damage the viewer's Unit took, named in the view before when it died; the opponent's, a zero hit, the hero and the sentinel count nothing", () => {
    expect(countTaken(EMPTY_LOG, board, [hit("enemy", "mine", 3), hit(null, "mine", 2), hit("enemy", "mine", 0)], null).damageTaken).toEqual({ "core-004": 5 });
    expect(countTaken(EMPTY_LOG, baseView(), [hit("enemy", "mine", 3)], board).damageTaken).toEqual({ "core-004": 3 });
    expect(countTaken(EMPTY_LOG, board, [hit("mine", "enemy", 3), hit("enemy", "hero-p1", 3), hit("enemy", HIDDEN_ID, 3)], null)).toBe(EMPTY_LOG);
  });

  it("R2649000 Double Tap: a viewer Unit's second attack in one turn counts once; one attack a turn, a turn between, the opponent's and the sentinel count nothing", () => {
    const count = (log: GameLog, fresh: GameEvent[]): GameLog => countDoubleAttacks(log, board, fresh, null);
    expect(count(EMPTY_LOG, [attacked("mine", "enemy"), attacked("mine", "hero-p2"), attacked("mine", "hero-p2")]).doubleAttacks).toEqual({ "core-004": 1 });
    // The second attack in a later batch of the same turn counts: the log keeps this turn's attackers.
    expect(count(count(EMPTY_LOG, [attacked("mine", "enemy")]), [attacked("mine", "hero-p2")]).doubleAttacks).toEqual({ "core-004": 1 });
    const once = count(EMPTY_LOG, [attacked("mine", "enemy")]);
    expect(once.doubleAttacks).toEqual({});
    expect(count(once, [started("p2"), started("p1"), attacked("mine", "enemy")]).doubleAttacks).toEqual({});
    expect(count(EMPTY_LOG, [attacked("mine", "enemy"), started("p2"), attacked("mine", "enemy")]).doubleAttacks).toEqual({});
    expect(count(EMPTY_LOG, [attacked("enemy", "mine"), attacked("enemy", "mine"), attacked(HIDDEN_ID, "mine"), attacked(HIDDEN_ID, "mine")])).toBe(EMPTY_LOG);
  });

  it("R2649000 Vampire: the heal after a hit by the viewer's card that had Lifesteal in the view before is drained; one without, the opponent's and no view before drain nothing", () => {
    const before = baseView({
      you: emptySide("p1", {
        units: [unit("p1", { instanceId: "mine", defId: "core-004", keywords: [{ kind: "Lifesteal" }] }), null, null, null, null],
        hand: [card({ instanceId: "book", defId: "classic-012" })],
      }),
      opponent: emptySide("p2", { units: [unit("p2", { instanceId: "enemy", defId: "core-007", keywords: [{ kind: "Lifesteal" }] }), null, null, null, null] }),
    });
    const view = baseView({
      you: emptySide("p1", {
        units: [unit("p1", { instanceId: "mine", defId: "core-004", keywords: [{ kind: "Lifesteal" }] }), null, null, null, null],
        graveyard: [card({ instanceId: "book", defId: "classic-012" })],
      }),
      opponent: before.opponent,
    });
    const count = (fresh: GameEvent[], from: PlayerView | null = before, after: PlayerView = view): GameLog => countHealing(EMPTY_LOG, after, fresh, from);
    const log = count([attacked("mine", "enemy"), hit("mine", "enemy", 3), healed(3), healed(2)]);
    expect(log.lifesteal).toEqual({ "core-004": 3 });
    expect(log.healing).toEqual({ "core-004": 5 });
    // A Spell that prints Lifesteal (Book of Blood), read off its catalog face in the hand before.
    expect(count([playedAs("p1", "classic-012", "book"), hit("book", "enemy", 2), healed(2)]).lifesteal).toEqual({ "classic-012": 2 });
    // A Unit without Lifesteal in the view before, the opponent's Lifesteal, another source between, and no view before.
    expect(count([hit("mine", "enemy", 3), healed(3)], board).lifesteal).toEqual({});
    expect(count([hit("enemy", "mine", 3), healed(3)])).toBe(EMPTY_LOG);
    expect(count([hit("mine", "enemy", 3), hit(null, "mine", 1), healed(3)]).lifesteal).toEqual({});
    expect(count([hit("mine", "enemy", 3), healed(3)], null).lifesteal).toEqual({});
    expect(count([hit(HIDDEN_ID, "enemy", 3), healed(3)])).toBe(EMPTY_LOG);
  });

  it("R2649000 Martyr: the viewer's own Tribute, Sacrifice or effect counts its death; a crumble, the opponent's cause, no actor at all, the opponent's own and the sentinel count nothing", () => {
    const count = (fresh: GameEvent[]): GameLog => countSacrificed(EMPTY_LOG, board, fresh, null);
    // R119: a Tribute's death comes before its play, so it waits for the batch's first actor.
    expect(count([destroyed("p1", "core-001"), played("p1", "core-021")]).sacrificed).toEqual({ "core-001": 1 });
    expect(count([played("p1", "core-021"), destroyed("p1", "core-001")]).sacrificed).toEqual({ "core-001": 1 });
    expect(count([activated("p1"), destroyed("p1", "core-001")]).sacrificed).toEqual({ "core-001": 1 });
    expect(count([destroyed("p1", "core-001", "bolt")]).sacrificed).toEqual({ "core-001": 1 });
    // A crumble, the opponent's cause, nobody acting, a first actor of the opponent's, the opponent's own Tribute and the sentinel.
    expect(count([played("p1", "core-021"), crumbledOf("p1", "i2"), destroyed("p1", "core-001")])).toBe(EMPTY_LOG);
    expect(count([played("p2", "core-021"), destroyed("p1", "core-001")])).toBe(EMPTY_LOG);
    expect(count([played("p1", "core-021"), destroyed("p1", "core-004", "enemy", "mine")])).toBe(EMPTY_LOG);
    expect(count([destroyed("p1", "core-001")])).toBe(EMPTY_LOG);
    expect(count([destroyed("p1", "core-001"), played("p2", "core-021")])).toBe(EMPTY_LOG);
    expect(count([destroyed("p2", "core-007"), played("p2", "core-021")])).toBe(EMPTY_LOG);
    expect(count([played("p1", "core-021"), destroyed("p1", HIDDEN_ID)])).toBe(EMPTY_LOG);
    // R2648000 still counts these for neither destroyed nor defeated.
    expect(countDeaths(EMPTY_LOG, board, [destroyed("p1", "core-001"), played("p1", "core-021")], null)).toBe(EMPTY_LOG);
  });

  it("R2649000 Cold Snap: a freeze on an opponent's Unit counts for the viewer's card that is the source at that point; none, the opponent's, a thaw and another mark count nothing", () => {
    const count = (fresh: GameEvent[], before: PlayerView | null = null): GameLog => countFreezes(EMPTY_LOG, board, fresh, before);
    expect(count([attacked("mine", "enemy"), hit("mine", "enemy", 1), frost("enemy")]).froze).toEqual({ "core-004": 1 });
    expect(count([played("p1", "core-030"), frost("enemy")]).froze).toEqual({ "core-030": 1 });
    // The Unit died since: the view before shows it on the opponent's side.
    expect(count([played("p1", "core-030"), frost("e9")], baseView({ opponent: emptySide("p2", { units: [unit("p2", { instanceId: "e9" }), null, null, null, null] }) })).froze).toEqual({
      "core-030": 1,
    });
    expect(count([frost("enemy")])).toBe(EMPTY_LOG);
    expect(count([played("p1", "core-030"), played("p2", "core-031"), frost("enemy")])).toBe(EMPTY_LOG);
    expect(count([played("p1", HIDDEN_ID), frost("enemy")])).toBe(EMPTY_LOG);
    expect(count([played("p1", "core-030"), frost("enemy", false)])).toBe(EMPTY_LOG);
    expect(count([played("p1", "core-030"), { ...frost("enemy"), mark: "steal" } as GameEvent])).toBe(EMPTY_LOG);
    expect(count([played("p1", "core-030"), frost(HIDDEN_ID)])).toBe(EMPTY_LOG);
  });

  it("R2649000 Brain Freeze: a freeze on the viewer's Unit counts for it; the opponent's Unit and the sentinel count nothing", () => {
    expect(countFreezes(EMPTY_LOG, board, [played("p2", "core-030"), frost("mine"), frost("mine")], null).frozen).toEqual({ "core-004": 2 });
    expect(countFreezes(EMPTY_LOG, board, [frost("enemy"), frost(HIDDEN_ID), frost("gone")], null)).toBe(EMPTY_LOG);
  });

  it("R2649000 Spring Thaw: a thaw of the viewer's Unit still on the field counts; a frozen Unit that died, the opponent's and the sentinel count nothing", () => {
    expect(countFreezes(EMPTY_LOG, board, [frost("mine", false)], null).thawed).toEqual({ "core-004": 1 });
    // It died: the view after no longer shows it on the field, though the view before did.
    const dead = baseView({ you: emptySide("p1", { graveyard: [card({ instanceId: "mine", defId: "core-004" })] }) });
    expect(countFreezes(EMPTY_LOG, dead, [frost("mine", false)], board)).toBe(EMPTY_LOG);
    expect(countFreezes(EMPTY_LOG, board, [frost("enemy", false), frost(HIDDEN_ID, false)], null)).toBe(EMPTY_LOG);
  });

  it("R2649000 Encore: the viewer's play from the graveyard counts; the opponent's and the sentinel count nothing", () => {
    const flash = (player: "p1" | "p2", defId: string): GameEvent => playedAs(player, defId, "i1", { from: "graveyard" });
    expect(countEvent(countEvent(EMPTY_LOG, flash("p1", "stinkyscape-021"), "p1"), playedAs("p1", "stinkyscape-021", "i2"), "p1").flashback).toEqual({ "stinkyscape-021": 1 });
    expect(countEvent(EMPTY_LOG, flash("p2", "stinkyscape-021"), "p1").flashback).toEqual({});
    expect(countEvent(EMPTY_LOG, flash("p1", HIDDEN_ID), "p1")).toBe(EMPTY_LOG);
  });

  it("R2649000 Bargain Hunter: the printed cost less the price paid, never below 0; an X, an embiggen, a cast on draw, the opponent's and an unknown card save nothing", () => {
    // Big D-fender (core-001) prints 2.
    const at = (costPaid: number, over: Partial<Extract<GameEvent, { type: "cardPlayed" }>> = {}): GameEvent => playedAs("p1", "core-001", "i1", { costPaid, ...over });
    let log = countEvent(EMPTY_LOG, at(0), "p1");
    log = countEvent(log, at(1), "p1");
    log = countEvent(log, at(2), "p1");
    log = countEvent(log, at(5), "p1");
    expect(log.manaSaved).toEqual({ "core-001": 3 });
    for (const over of [{ x: 0 }, { embiggened: true }, { onDraw: true as const }]) expect(countEvent(EMPTY_LOG, at(0, over), "p1").manaSaved).toEqual({});
    expect(countEvent(EMPTY_LOG, playedAs("p2", "core-001", "i1", { costPaid: 0 }), "p1").manaSaved).toEqual({});
    expect(countEvent(EMPTY_LOG, playedAs("p1", "gone-001", "i1", { costPaid: 0 }), "p1").manaSaved).toEqual({});
    expect(countEvent(EMPTY_LOG, playedAs("p1", HIDDEN_ID, "i1", { costPaid: 0 }), "p1")).toBe(EMPTY_LOG);
  });

  it("R2649000 Denied: the viewer's play countered counts; the opponent's and the sentinel count nothing", () => {
    expect(countEvent(EMPTY_LOG, countered("p1", "core-005", "trap"), "p1").countered).toEqual({ "core-005": 1 });
    expect(countEvent(EMPTY_LOG, countered("p2", "core-005", "trap"), "p1")).toBe(EMPTY_LOG);
    expect(countEvent(EMPTY_LOG, countered("p1", HIDDEN_ID, "trap"), "p1")).toBe(EMPTY_LOG);
  });

  it("R2649000 Nope: the opponent's play countered by the viewer's card counts for it; no counterer, the opponent's own, the viewer's own play and the sentinel count nothing", () => {
    const view = baseView({
      you: emptySide("p1", { graveyard: [card({ instanceId: "trap", defId: "core-041" })] }),
      opponent: emptySide("p2", { graveyard: [card({ instanceId: "theirs", defId: "core-060" })] }),
    });
    const count = (fresh: GameEvent[]): GameLog => countFengShuiAndCounters(EMPTY_LOG, view, fresh, null);
    expect(count([countered("p2", "core-005", "trap"), countered("p2", HIDDEN_ID, "trap")]).counterspells).toEqual({ "core-041": 2 });
    expect(count([countered("p2", "core-005", null), countered("p2", "core-005", "theirs"), countered("p1", "core-005", "trap"), countered("p2", "core-005", HIDDEN_ID)])).toBe(EMPTY_LOG);
  });

  it("R2649000 Burnt Toast and Frequent Flyer: the viewer's card burned from a full hand or bounced counts; the opponent's and the sentinel count nothing", () => {
    for (const type of ["burned", "bounced"] as const) {
      const event = (owner: "p1" | "p2", defId: string): GameEvent => ({ type, instanceId: "c1", defId, owner });
      const log = countEvent(countEvent(EMPTY_LOG, event("p1", "core-002"), "p1"), event("p2", "core-009"), "p1");
      expect(log[type]).toEqual({ "core-002": 1 });
      expect(countEvent(EMPTY_LOG, event("p1", "core-002"), "p2")).toBe(EMPTY_LOG);
      expect(countEvent(EMPTY_LOG, event("p1", HIDDEN_ID), "p1")).toBe(EMPTY_LOG);
    }
  });

  it("R2649000 Liftoff: each piece of the viewer's launched Starship counts; the opponent's launch and a hidden piece count nothing", () => {
    const launched = (player: "p1" | "p2", pieces: string[]): GameEvent => ({
      type: "starshipLaunched",
      player,
      instanceId: "ship",
      defId: "stinkyscape-t-shield-layer",
      name: "Drone Battleship",
      pieces,
    });
    expect(countEvent(EMPTY_LOG, launched("p1", ["stinkyscape-038", "stinkyscape-040", "stinkyscape-038", HIDDEN_ID]), "p1").launched).toEqual({
      "stinkyscape-038": 2,
      "stinkyscape-040": 1,
    });
    expect(countEvent(EMPTY_LOG, launched("p2", ["stinkyscape-038"]), "p1")).toBe(EMPTY_LOG);
    expect(countEvent(EMPTY_LOG, launched("p1", [HIDDEN_ID]), "p1")).toBe(EMPTY_LOG);
  });

  it("R2649000 observe counts every new counter from its views and their events", () => {
    const before = baseView({
      you: emptySide("p1", {
        hand: [
          card({ instanceId: "temp", defId: "core-005", keywords: [{ kind: "Temporary" }] }),
          card({ instanceId: "late", defId: "core-006", keywords: [{ kind: "Temporary" }] }),
          card({ instanceId: "swap", defId: "meditative-053", tradable: true }),
        ],
        units: [
          unit("p1", { instanceId: "mine", defId: "core-004", keywords: [{ kind: "Lifesteal" }] }),
          unit("p1", { instanceId: "phoenix", defId: "core-003", keywords: [{ kind: "Reborn" }] }),
          null,
          null,
          null,
        ],
      }),
      opponent: emptySide("p2", { units: [unit("p2", { instanceId: "enemy", defId: "core-007", frozen: true }), null, null, null, null] }),
    });
    const view = baseView({
      you: emptySide("p1", {
        hand: [card({ instanceId: "made", defId: "core-002", created: true })],
        graveyard: [
          card({ instanceId: "temp", defId: "core-005" }),
          card({ instanceId: "late", defId: "core-006" }),
          card({ instanceId: "trap", defId: "core-041" }),
          card({ instanceId: "fb", defId: "stinkyscape-021" }),
        ],
        units: [unit("p1", { instanceId: "mine", defId: "core-004" }), unit("p1", { instanceId: "phoenix-2", defId: "core-003" }), null, null, null],
      }),
    });
    const fresh: GameEvent[] = [
      playedAs("p1", "core-005", "temp", { costPaid: 0 }),
      playedAs("p1", "stinkyscape-021", "fb", { from: "graveyard" }),
      crumbledOf("p1", "c1", "classicplus-061"),
      crumbledOf("p2", "c2", "stinkyscape-009"),
      { type: "fengShui", instanceId: "mine", sourceId: "judge", player: "p1", outcome: "positive" },
      { type: "fengShui", instanceId: "mine", sourceId: "judge", player: "p1", outcome: "negative" },
      traded("p1"),
      shuffled("p1"),
      drew("p1", "core-009"),
      attacked("mine", "enemy"),
      hit("mine", "enemy", 3, 1),
      healed(3),
      killed("p2", "core-007", "mine", "enemy", 6),
      attacked("mine", "hero-p2"),
      frost("e2"),
      { type: "divineShieldLost", instanceId: "mine" },
      hit("enemy", "mine", 2),
      { type: "damageAbsorbed", sourceId: "enemy", targetId: "mine", absorbed: 4, combat: true },
      frost("mine"),
      frost("mine", false),
      killed("p1", "core-003", "enemy", "phoenix"),
      summoned("p1", "phoenix-2", "core-003"),
      activated("p1", "core-041"),
      killed("p1", "core-001", null, "fodder"),
      countered("p2", "core-060", "trap"),
      countered("p1", "core-030", null),
      { type: "burned", instanceId: "b1", defId: "core-008", owner: "p1" },
      { type: "bounced", instanceId: "b2", defId: "core-006", owner: "p1" },
      { type: "starshipLaunched", player: "p1", instanceId: "ship", defId: "stinkyscape-t-shield-layer", name: "Drone Battleship", pieces: ["stinkyscape-038"] },
      ended("p1"),
      discarded("p1", "late", "core-006"),
    ];
    const withFoe = { ...before, opponent: emptySide("p2", { units: [unit("p2", { instanceId: "enemy", defId: "core-007", frozen: true }), unit("p2", { instanceId: "e2" }), null, null, null] }) };
    const log = observe(EMPTY_LOG, view, fresh, withFoe);
    expect(log).toMatchObject({
      tempPlayed: { "core-005": 1 },
      tempExpired: { "core-006": 1 },
      crumbled: { "classicplus-061": 1 },
      crumbledAgainst: { "stinkyscape-009": 1 },
      fengShuiGood: { "core-004": 1 },
      fengShuiBad: { "core-004": 1 },
      traded: { "meditative-053": 1 },
      tradeDrawn: { "core-009": 1 },
      created: { "core-002": 1 },
      kills: { "core-004": 1 },
      biggestKill: { "core-004": 6 },
      reborn: { "core-003": 1 },
      shieldsLost: { "core-004": 1 },
      armorAbsorbed: { "core-004": 4 },
      damageTaken: { "core-004": 2 },
      doubleAttacks: { "core-004": 1 },
      lifesteal: { "core-004": 3 },
      sacrificed: { "core-001": 1 },
      froze: { "core-004": 1 },
      frozen: { "core-004": 1 },
      thawed: { "core-004": 1 },
      shattered: { "core-004": 1 },
      flashback: { "stinkyscape-021": 1 },
      manaSaved: { "core-005": 1 },
      countered: { "core-030": 1 },
      counterspells: { "core-041": 1 },
      burned: { "core-008": 1 },
      bounced: { "core-006": 1 },
      launched: { "stinkyscape-038": 1 },
    });
    expect(log.createdIds).toEqual(["made"]);
    expect(log.attackers).toEqual({ mine: 2 });
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
