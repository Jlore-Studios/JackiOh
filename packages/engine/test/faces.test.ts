// Faces that change more than text (docs/classic-sets.md B2.7, B5 E40): a face with its own type
// (Classic+ #22 Blood Moon's Radiant face is a Field Trap) and X in the stats (Classic+ #69 Buff
// Billy's "[3X/3X]"). Every rule that asks what an instance *is* reads the running face's type
// (`faces.cardTypeOf`); a definition read on its own — a pool, a catalog filter — reads `def.type`.

import type { Action, ActionInput } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { query } from "../src/catalog";
import { cardsInCardScope } from "../src/effects";
import { summon } from "../src/effects/summon";
import { cardTypeOf, runningFace } from "../src/faces";
import { unitView } from "../src/layers";
import { rowForCard } from "../src/playChoices";
import { makeContext } from "../src/resolve";
import { beginGame, legalActions, reduce } from "../src/reduce";
import { stateCheck } from "../src/stateCheck";
import type { GameState } from "../src/state";
import { consumeTrap, isFieldTrap, isTrapType } from "../src/traps";
import { viewFor } from "../src/viewFor";
import { landsFaceDown } from "../src/zones";
import { inHand, put, sinkFor, slot } from "./fixtures/harness";
import { billy, bloodMoon, instanceGame } from "./fixtures/instanceData";

function game(): GameState {
  const state = instanceGame("faces");
  state.turn = 3;
  state.active = "p1";
  state.phase = "main";
  return state;
}

describe("B2.7 a face with its own type", () => {
  it("B2.7 the card's type is its running face's: a Radiant Blood Moon is a Field Trap, the base one a Trap", () => {
    const state = game();
    const [base, radiant] = inHand(state, bloodMoon.id, "p1", 2);
    if (base === undefined || radiant === undefined) throw new Error("no card");
    radiant.radiant = true;
    expect(runningFace(state, radiant).type).toBe("Field Trap");
    expect(cardTypeOf(state, base)).toBe("Trap");
    expect(cardTypeOf(state, radiant)).toBe("Field Trap");
    expect([isTrapType(state, base), isTrapType(state, radiant)]).toEqual([true, true]);
    expect([isFieldTrap(state, base), isFieldTrap(state, radiant)]).toEqual([false, true]);
    expect([landsFaceDown(state, base, "backrow"), landsFaceDown(state, radiant, "backrow")]).toEqual([true, true]);
    expect(rowForCard(state, radiant)).toBe("backrow");
  });

  it("B2.7 a Radiant Blood Moon stays on the field after it fires, as a Field Trap does; the base one goes", () => {
    const state = game();
    const base = put(state, bloodMoon.id, slot("p1", "backrow", 1));
    const radiant = put(state, bloodMoon.id, slot("p1", "backrow", 2), { radiant: true });
    const sink = sinkFor(state);
    consumeTrap(sink, base);
    consumeTrap(sink, radiant);
    expect(base.zone.z).toBe("graveyard");
    expect(radiant.zone).toEqual({ z: "field", player: "p1", row: "backrow", lane: 2 });
    expect(radiant.faceUp).toBe(true);
  });

  it("B2.7 filters read the card's type now; a pool of definitions reads the definition's", () => {
    const state = game();
    const [radiant] = inHand(state, bloodMoon.id, "p1");
    if (radiant === undefined) throw new Error("no card");
    radiant.radiant = true;
    const ctx = makeContext(sinkFor(state), null, { controller: "p1" });
    expect(cardsInCardScope(ctx, { zones: ["hand"], types: ["Field Trap"] }).map((e) => e.card.id)).toEqual([radiant.id]);
    expect(cardsInCardScope(ctx, { zones: ["hand"], types: ["Trap"] })).toEqual([]);
    expect(query({ type: "Trap" }).map((def) => def.id)).toContain(bloodMoon.id);
  });

  it("B2.7 the view names the type only where it differs from the definition's", () => {
    const state = game();
    put(state, bloodMoon.id, slot("p1", "backrow", 1));
    put(state, bloodMoon.id, slot("p1", "backrow", 2), { radiant: true });
    const own = viewFor(state, "p1").you.backrow;
    expect(own[0]).toMatchObject({ faceDown: false, type: "Trap" });
    expect(own[1]).toMatchObject({ faceDown: false, type: "Field Trap" });
    const hand = viewFor(state, "p1").you.hand;
    if (!Array.isArray(hand)) throw new Error("own hand is a list");
    for (const card of hand) expect(card.type).toBeUndefined();
    const [held] = inHand(state, bloodMoon.id, "p1");
    if (held === undefined) throw new Error("no card");
    held.radiant = true;
    const after = viewFor(state, "p1").you.hand;
    if (!Array.isArray(after)) throw new Error("own hand is a list");
    expect(after.find((card) => card.instanceId === held.id)?.type).toBe("Field Trap");
    // The other player reads the face-down cards as zones only (R351).
    expect(viewFor(state, "p2").opponent.backrow.slice(0, 2)).toEqual([
      { faceDown: true, cost: 1 },
      { faceDown: true, cost: 1 },
    ]);
  });
});

let nonce = 0;
function act(state: GameState, body: ActionInput): GameState {
  nonce += 1;
  const result = reduce(state, { ...body, nonce: `fc${nonce}` } as Action);
  if (result.error !== undefined) throw new Error(result.error);
  return result.state;
}

describe("B2.7 X in the stats (Buff Billy)", () => {
  it("B2.7 an X-stat Unit played for X is xStats × X, and its Cry's Upgrades land on that body", () => {
    let state = beginGame(instanceGame("billy")).state;
    state = act(state, { type: "mulligan", keep: state.players.p1.hand.map((c) => c.id), playerId: "p1" });
    state = act(state, { type: "mulligan", keep: state.players.p2.hand.map((c) => c.id), playerId: "p2" });
    const [card] = inHand(state, billy.id, "p1");
    if (card === undefined) throw new Error("no card");
    state.players.p1.mana = { ...state.players.p1.mana, current: 3, max: 3 };
    const play = legalActions(state, "p1").find(
      (action) => action.type === "play" && action.instanceId === card.id && action.x === 2,
    );
    if (play === undefined) throw new Error("expected a play for X = 2");
    state = act(state, { ...play, playerId: "p1" } as ActionInput);
    const unit = state.players.p1.units.flatMap((pile) => pile ?? []).find((c) => c.id === card.id);
    if (unit === undefined) throw new Error("expected Billy on the field");
    expect(unit.x).toBe(2);
    // 6/6 from the X, and two Upgrades from its Cry on top.
    expect(unit.tuning === undefined ? 0 : Object.keys(unit.tuning).length).toBeGreaterThan(0);
    const view = unitView(state, unit);
    expect(view.attack + view.maxHealth).toBeGreaterThanOrEqual(12);
  });

  it("B2.7 the Radiant face multiplies the same X by its own numbers (7X/7X)", () => {
    const state = instanceGame("billy-radiant");
    const [card] = inHand(state, billy.id, "p1");
    if (card === undefined) throw new Error("no card");
    card.radiant = true;
    card.x = 2;
    expect(unitView(state, card)).toMatchObject({ attack: 14, maxHealth: 14 });
  });

  it("B2.7 with no X — a summon outside a play — it arrives 0/0 and the state check collects it", () => {
    const state = instanceGame("billy-summon");
    state.turn = 3;
    state.active = "p1";
    const sink = sinkFor(state);
    summon({ defId: billy.id }).apply(makeContext(sink, null, { controller: "p1" }));
    const unit = state.players.p1.units[0]?.[0];
    if (unit === undefined) throw new Error("expected a summon");
    expect(unitView(state, unit)).toMatchObject({ attack: 0, maxHealth: 0 });
    stateCheck(sink);
    expect(unit.zone.z).toBe("graveyard");
  });
});
