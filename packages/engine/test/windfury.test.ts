// Windfury (SPEC §6.1, R636): a Unit may attack twice each turn. The second attack is a full one, the
// first spends the exertion a switch needs, and `legalActions` and the reducer's refusal agree on it
// because both ask `combat.hasExertion` (BUILD M2-T1).

import type { Action, ActionInput, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { WINDFURY_ATTACKS } from "../src/config";
import { attacksPerTurn, hasExertion, switchPosition } from "../src/combat";
import { beginGame, legalActions, reduce } from "../src/reduce";
import type { CardInstance, GameState } from "../src/state";
import { deftDuelist, plain, windfurier } from "./fixtures/combat";
import { newGame, put, sinkFor, slot } from "./fixtures/harness";

let nonce = 0;
function attempt(state: GameState, body: ActionInput): ReturnType<typeof reduce> {
  nonce += 1;
  return reduce(state, { ...body, nonce: `wf${nonce}` } as Action);
}

function act(state: GameState, body: ActionInput): GameState {
  const result = attempt(state, body);
  if (result.error !== undefined) throw new Error(result.error);
  return result.state;
}

/** Past the mulligans, in the main phase of turn 1. */
function playing(seed: string): GameState {
  let state = beginGame(newGame(seed)).state;
  state = act(state, { type: "mulligan", keep: state.players.p1.hand.map((c) => c.id), playerId: "p1" });
  state = act(state, { type: "mulligan", keep: state.players.p2.hand.map((c) => c.id), playerId: "p2" });
  return state;
}

function unitAt(state: GameState, player: PlayerId, lane: number): CardInstance {
  const unit = state.players[player].units[lane - 1]?.[0];
  if (unit === undefined) throw new Error(`no unit in ${player} lane ${lane}`);
  return unit;
}

function attackHero(state: GameState, lane: number): GameState {
  return act(state, { type: "attack", attackerId: unitAt(state, "p1", lane).id, targetId: "hero-p2", playerId: "p1" });
}

/** Whether `legalActions` offers an attack by this unit (and so the reducer must accept it). */
function offersAttack(state: GameState, unit: CardInstance): boolean {
  return legalActions(state, "p1").some((a) => a.type === "attack" && a.attackerId === unit.id);
}

describe("Windfury (R636)", () => {
  it("R636 a Unit with Windfury attacks twice in a turn, each a full attack, and not a third time", () => {
    expect(WINDFURY_ATTACKS).toBe(2);
    let state = playing("windfury-twice");
    put(state, windfurier.id, slot("p1", "units", 1));
    expect(attacksPerTurn(state, unitAt(state, "p1", 1))).toBe(2);

    state = attackHero(state, 1);
    expect(state.players.p2.hero.health).toBe(27);
    // The first attack leaves the unit able to declare another, in `legalActions` and in the reducer.
    expect(offersAttack(state, unitAt(state, "p1", 1))).toBe(true);

    state = attackHero(state, 1);
    expect(state.players.p2.hero.health).toBe(24);
    expect(unitAt(state, "p1", 1).exertion).toEqual({ attacked: true, switched: false, attacks: 2 });

    // The third is refused the way a second attack by any unit is, and `legalActions` no longer lists it.
    expect(offersAttack(state, unitAt(state, "p1", 1))).toBe(false);
    expect(
      attempt(state, { type: "attack", attackerId: unitAt(state, "p1", 1).id, targetId: "hero-p2", playerId: "p1" }).error,
    ).toMatch(/already acted/);
    expect(state.players.p2.hero.health).toBe(24);
  });

  it("R636 a Unit without Windfury still attacks once, and the first Windfury attack spends the switch (R6)", () => {
    let state = playing("windfury-switch");
    put(state, plain.id, slot("p1", "units", 1));
    put(state, windfurier.id, slot("p1", "units", 2));

    state = attackHero(state, 1);
    expect(offersAttack(state, unitAt(state, "p1", 1))).toBe(false);

    state = attackHero(state, 2);
    const wind = unitAt(state, "p1", 2);
    // Attacked once, with an attack left: it cannot also switch, since a switch takes the exertion an attack spent.
    expect(hasExertion(state, wind, "attack")).toBe(true);
    expect(hasExertion(state, wind, "switch")).toBe(false);
    expect(legalActions(state, "p1").some((a) => a.type === "switchPosition" && a.instanceId === wind.id)).toBe(false);
    expect(attempt(state, { type: "switchPosition", instanceId: wind.id, playerId: "p1" }).error).toMatch(/already acted/);
  });

  it("R636 a Unit that switched cannot attack, Windfury or not (R6)", () => {
    let state = playing("windfury-switched");
    put(state, windfurier.id, slot("p1", "units", 1));
    state = act(state, { type: "switchPosition", instanceId: unitAt(state, "p1", 1).id, playerId: "p1" });
    // An effect flips it back to Attack Position (R20, no exertion): the switch it made already spent the turn's.
    expect(switchPosition(sinkFor(state), unitAt(state, "p1", 1), { spendExertion: false, to: "ATK" }).error).toBeUndefined();
    const wind = unitAt(state, "p1", 1);
    expect(hasExertion(state, wind, "attack")).toBe(false);
    expect(offersAttack(state, wind)).toBe(false);
    expect(
      attempt(state, { type: "attack", attackerId: wind.id, targetId: "hero-p2", playerId: "p1" }).error,
    ).toMatch(/already acted/);
  });

  it("R636 the second attack comes back next turn: both attacks are available again at its controller's start", () => {
    let state = playing("windfury-reset");
    put(state, windfurier.id, slot("p1", "units", 1));
    state = attackHero(state, 1);
    state = attackHero(state, 1);
    state = act(state, { type: "endTurn", playerId: "p1" });
    state = act(state, { type: "endTurn", playerId: "p2" });
    expect(state.active).toBe("p1");
    expect(unitAt(state, "p1", 1).exertion).toEqual({ attacked: false, switched: false });
    state = attackHero(state, 1);
    state = attackHero(state, 1);
    expect(state.players.p2.hero.health).toBe(18);
  });

  it("R636 the count is read when the second attack is declared: Windfury gained after one attack gives another, lost it takes it", () => {
    let state = playing("windfury-gain-lose");
    put(state, plain.id, slot("p1", "units", 1));
    put(state, windfurier.id, slot("p1", "units", 2));

    state = attackHero(state, 1);
    state = attackHero(state, 2);
    expect(offersAttack(state, unitAt(state, "p1", 1))).toBe(false);

    unitAt(state, "p1", 1).grantedKeywords.push({ kind: "Windfury" });
    expect(offersAttack(state, unitAt(state, "p1", 1))).toBe(true);
    state = attackHero(state, 1);
    expect(offersAttack(state, unitAt(state, "p1", 1))).toBe(false);

    // The other unit loses its Windfury (here: it is Vanilla'd, which takes the printed keyword) after one attack.
    unitAt(state, "p1", 2).vanilla = true;
    expect(attacksPerTurn(state, unitAt(state, "p1", 2))).toBe(1);
    expect(offersAttack(state, unitAt(state, "p1", 2))).toBe(false);
    expect(
      attempt(state, { type: "attack", attackerId: unitAt(state, "p1", 2).id, targetId: "hero-p2", playerId: "p1" }).error,
    ).toMatch(/already acted/);
  });

  it("R636 Deft Duelist keeps its independent switch beside Windfury", () => {
    const state = playing("windfury-duelist");
    const duelist = put(state, deftDuelist.id, slot("p1", "units", 1));
    duelist.grantedKeywords.push({ kind: "Windfury" });
    const next = attackHero(state, 1);
    const after = unitAt(next, "p1", 1);
    // One attack made, one left, and the duelist's own switch still unspent.
    expect(hasExertion(next, after, "attack")).toBe(true);
    expect(hasExertion(next, after, "switch")).toBe(true);
  });
});
