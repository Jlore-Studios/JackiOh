// Card scopes (docs/classic-sets.md B3.3, B3.4, B5 E38, E39): the cards a verb reaches on the field,
// in hands and in decks, in R242's order — public cards, then the owner's hidden ones, then the
// decks' — with its filters, and every card of a hidden pile kept for a verb that must cue the whole
// pile (R440); and who may read a card where it sits (R97, R177).

import { describe, expect, it } from "vitest";
import { cardsInCardScope, readersOf, unreadableBy } from "../src/effects";
import { makeContext } from "../src/resolve";
import type { GameState } from "../src/state";
import { placeOnField } from "../src/zones";
import { newInstance } from "../src/state";
import { plain, stacker } from "./fixtures/combat";
import { inHand, put, setLibrary, sinkFor, slot } from "./fixtures/harness";
import { bloodMoon, echoBolt, instanceGame, tesla } from "./fixtures/instanceData";

function game(active: "p1" | "p2" = "p1"): GameState {
  const state = instanceGame("card-scope");
  state.turn = 3;
  state.active = active;
  return state;
}

describe("card scopes (R242, R440)", () => {
  it("R242 walks public cards first, then hands and face-down traps, then decks; the active side first in each group", () => {
    const state = game("p2");
    const deck1 = setLibrary(state, "p1", [plain.id]);
    const deck2 = setLibrary(state, "p2", [plain.id]);
    const [hand1] = inHand(state, plain.id, "p1");
    const [hand2] = inHand(state, plain.id, "p2");
    const unit1 = put(state, plain.id, slot("p1", "units", 3));
    const trap1 = put(state, bloodMoon.id, slot("p1", "backrow", 1));
    const field2 = put(state, tesla.id, slot("p2", "backrow", 2));
    field2.faceUp = true;
    const ctx = makeContext(sinkFor(state), null, { controller: "p1" });
    const walk = cardsInCardScope(ctx, { side: "any", zones: ["field", "hand", "library"] });
    expect(walk.map((entry) => entry.card.id)).toEqual([
      field2.id,
      unit1.id,
      hand2?.id,
      trap1.id,
      hand1?.id,
      deck2[0]?.id,
      deck1[0]?.id,
    ]);
    expect(walk.map((entry) => entry.readers)).toEqual(["everyone", "everyone", "owner", "owner", "owner", "nobody", "nobody"]);
  });

  it("R440 filters keep the matching cards; `wholeHiddenPiles` keeps every hidden card, the rest marked", () => {
    const state = game();
    const [unit] = inHand(state, plain.id, "p1");
    const [spell] = inHand(state, echoBolt.id, "p1");
    const fieldSpell = put(state, plain.id, slot("p1", "units", 1));
    const trap = put(state, bloodMoon.id, slot("p1", "backrow", 1));
    const ctx = makeContext(sinkFor(state), null, { controller: "p1" });
    const scope = { zones: ["field", "hand"] as ("field" | "hand")[], types: ["Spell" as const] };
    expect(cardsInCardScope(ctx, scope).map((e) => e.card.id)).toEqual([spell?.id]);
    const whole = cardsInCardScope(ctx, scope, { wholeHiddenPiles: true });
    // The unit on the field is public and left out; the face-down trap and the hand unit are kept, unmatched.
    expect(whole.map((e) => [e.card.id, e.matches])).toEqual([
      [trap.id, false],
      [unit?.id, false],
      [spell?.id, true],
    ]);
    expect(whole.some((e) => e.card.id === fieldSpell.id)).toBe(false);
  });

  it("§3.2 a card dormant under a Stack is not on the field for a scope; the side and rows narrow it", () => {
    const state = game();
    const under = put(state, plain.id, slot("p1", "units", 1));
    const top = newInstance(state, stacker.id, "p1", { z: "hand", player: "p1" });
    placeOnField(state, top, slot("p1", "units", 1), { stack: true });
    const back = put(state, tesla.id, slot("p1", "backrow", 2));
    const theirs = put(state, plain.id, slot("p2", "units", 1));
    const ctx = makeContext(sinkFor(state), top, { controller: "p1" });
    const ids = (side: "self" | "enemy" | "any", rows?: ("units" | "backrow")[], excludeSelf?: boolean): string[] =>
      cardsInCardScope(ctx, { side, zones: ["field"], ...(rows === undefined ? {} : { rows }), ...(excludeSelf === true ? { excludeSelf } : {}) }).map(
        (e) => e.card.id,
      );
    expect(ids("self")).toEqual([top.id, back.id]);
    expect(ids("self")).not.toContain(under.id);
    expect(ids("self", ["units"])).toEqual([top.id]);
    expect(ids("self", undefined, true)).toEqual([back.id]);
    expect(ids("enemy")).toEqual([theirs.id]);
  });

  it("R177 who may not read a card where it sits: both for a deck card, the other player for a hand card or a face-down trap", () => {
    const state = game();
    const [deck] = setLibrary(state, "p1", [plain.id]);
    const [held] = inHand(state, plain.id, "p2");
    const trap = put(state, bloodMoon.id, slot("p1", "backrow", 1));
    const unit = put(state, plain.id, slot("p1", "units", 1));
    if (deck === undefined || held === undefined) throw new Error("no card");
    expect(unreadableBy(state, deck)).toEqual(["p1", "p2"]);
    expect(unreadableBy(state, held)).toEqual(["p1"]);
    expect(unreadableBy(state, trap)).toEqual(["p2"]);
    expect(unreadableBy(state, unit)).toEqual([]);
    trap.faceUp = true;
    expect(readersOf(state, trap)).toBe("everyone");
  });
});
