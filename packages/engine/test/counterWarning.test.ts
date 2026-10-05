// R659: the counter warning a hand card's view carries (`counterWarning.ts`, `viewFor`), driven with
// fixture scripts so the engine is proved without `packages/cards`; Classic #87 Plague Chalice's own
// test proves the real card again (CLAUDE.md, "The engine doesn't depend on packages/cards").
//
// Fixtures are prefixed `cw-` and indexed above 1480 so they cannot collide (BUILD §0).

import type { Action, ActionInput, CardDef, CardView, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { counteredHandCards } from "../src/counterWarning";
import { beginGame, reduce } from "../src/reduce";
import type { CardScripts, Script, TriggerDef } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import type { GameState } from "../src/state";
import { viewFor } from "../src/viewFor";
import { inHand, newGame, put, slot } from "./fixtures/harness";

let nextIndex = 1480;

function def(name: string, type: CardDef["type"], cost: CardDef["cost"]): CardDef {
  nextIndex += 1;
  return {
    id: `cw-${name}`,
    index: String(nextIndex),
    name: `${name} (counterWarning)`,
    set: "Core",
    type,
    tags: [],
    rarity: "Common",
    token: false,
    cost,
    base: { keywords: [], text: name },
    radiant: { keywords: [], text: name },
  };
}

/** Counters every play at cost 2, whoever makes it: a Plague Chalice standing at 2. */
const counterField = def("counter-field", "Field Spell", 1);
/** The same counter as a Trap, which answers through `traps.ts` and so warns of nothing. */
const counterTrap = def("counter-trap", "Trap", 1);
/** Counters only its controller's opponent at cost 2: a Radiant Chalice. */
const opponentsOnly = def("opponents-only", "Field Spell", 1);
const two = def("two", "Spell", 2);
const one = def("one", "Spell", 1);

const DEFS = [counterField, counterTrap, opponentsOnly, two, one];

/** A counter trigger on `cardAnnounced` that never fires: the warning reads its zone, not its run. */
const announced: TriggerDef = { id: "cw-counter", on: ["cardAnnounced"], when: () => false, run: () => [] };

function counter(opponentOnly: boolean): Script {
  return {
    triggers: [announced],
    wouldCounter: ({ controller, player, costPaid }) => costPaid === 2 && !(opponentOnly && player === controller),
  };
}

const both = (script: Script): CardScripts => ({ base: script, radiant: script });

const SCRIPTS: Record<string, CardScripts> = {
  [counterField.id]: both(counter(false)),
  [counterTrap.id]: both(counter(false)),
  [opponentsOnly.id]: both(counter(true)),
  [two.id]: both({ cry: () => [] }),
  [one.id]: both({ cry: () => [] }),
};

let nonce = 0;

function act(state: GameState, body: ActionInput): GameState {
  nonce += 1;
  return reduce(state, { ...body, nonce: `cw${nonce}` } as Action).state;
}

/** Past the mulligans, in p1's main phase, each hand holding a (2) and a (1) Spell. */
function playing(seed: string): GameState {
  let state = beginGame(newGame(seed)).state;
  state = act(state, { type: "mulligan", keep: state.players.p1.hand.map((c) => c.id), playerId: "p1" });
  state = act(state, { type: "mulligan", keep: state.players.p2.hand.map((c) => c.id), playerId: "p2" });
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries(DEFS.map((d) => [d.id, d])) });
  registerScripts({ ...registeredScripts(), ...SCRIPTS });
  for (const player of ["p1", "p2"] as const) {
    inHand(state, two.id, player);
    inHand(state, one.id, player);
  }
  return state;
}

/** The definitions `player`'s own view marks `counteredOnPlay`. */
function warned(view: PlayerView): string[] {
  const hand = view.you.hand;
  return Array.isArray(hand) ? hand.filter((card: CardView) => card.counteredOnPlay === true).map((card) => card.defId) : [];
}

describe("R659 the counter warning on the viewer's hand", () => {
  it("R659 marks each seat's own hand cards that a field card would counter at every price, and no others", () => {
    const state = playing("r658-field");
    put(state, counterField.id, slot("p2", "backrow", 1));

    expect(warned(viewFor(state, "p1"))).toEqual([two.id]);
    expect(warned(viewFor(state, "p2"))).toEqual([two.id]);
    // Never `false`: a card it would not counter carries no key at all.
    const hand = viewFor(state, "p1").you.hand as CardView[];
    expect(hand.filter((card) => card.defId !== two.id).every((card) => !("counteredOnPlay" in card))).toBe(true);
    // R97: the opponent's hand is a count, with nothing on it.
    expect(viewFor(state, "p1").opponent.hand).toEqual({ count: state.players.p2.hand.length });
  });

  it("R659 a counter that covers only its controller's opponent warns only them", () => {
    const state = playing("r658-opponent");
    put(state, opponentsOnly.id, slot("p1", "backrow", 1));

    expect(warned(viewFor(state, "p1"))).toEqual([]);
    expect(warned(viewFor(state, "p2"))).toEqual([two.id]);
  });

  it("R659 nothing warns while no card on the field would counter: none at all, one in a hand, or a Trap", () => {
    const none = playing("r658-none");
    expect(counteredHandCards(none, "p1").size).toBe(0);

    const held = playing("r658-held");
    inHand(held, counterField.id, "p2");
    expect(warned(viewFor(held, "p1"))).toEqual([]);

    // A Trap answers through `traps.ts`, never the trigger dispatch, and face-down it is unreadable (R97).
    const trapped = playing("r658-trap");
    put(trapped, counterTrap.id, slot("p2", "backrow", 1));
    expect(warned(viewFor(trapped, "p1"))).toEqual([]);
  });

  it("R659 the warning is the viewer's own: it is asked for one hand and lists that hand's ids", () => {
    const state = playing("r658-ids");
    put(state, counterField.id, slot("p1", "backrow", 2));
    const ids = (player: PlayerId): string[] =>
      state.players[player].hand.filter((card) => card.defId === two.id).map((card) => card.id);

    expect([...counteredHandCards(state, "p1")]).toEqual(ids("p1"));
    expect([...counteredHandCards(state, "p2")]).toEqual(ids("p2"));
  });
});
