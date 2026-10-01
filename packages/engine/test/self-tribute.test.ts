// Two card-specific additions of Classic #88 Siphon Squad (R403), proved through fixture scripts:
//   - §10.4 layer 5's "an aura that sets attack to a value applies after every other layer"
//     (`StatMod.setAttack`, the Radiant face's "Enemy Units have 0 Attack");
//   - "When …, Tribute this", a condition every state check reads, the one right after the card arrives
//     included (`Script.tributeWhen`, `selfTribute.ts`), which reaches a face-down card too.

import type { CardDef } from "@jackioh/shared";
import { opponentOf } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { unclampedAttack, unitView } from "../src/layers";
import type { CardScripts } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import { stateCheck } from "../src/stateCheck";
import { fuse } from "../src/subsystems/fuse";
import { activeUnitsOf } from "../src/zones";
import { unitDef } from "./fixtures/catalog";
import { eventsOfType, inHand, newGame, put, sinkFor, slot } from "./fixtures/harness";

function def(id: string, type: CardDef["type"], index: number): CardDef {
  return {
    id,
    index: String(index),
    name: id,
    set: "Core",
    type,
    tags: [],
    rarity: "Common",
    token: false,
    cost: 2,
    base: { keywords: [], text: id },
    radiant: { keywords: [], text: id },
  };
}

/** Radiant Siphon Squad's shape: enemy Units have 0 Attack; Tribute this when they have none. */
const zeroer = def("st-zeroer", "Field Trap", 9601);
/** A +3 attack aura on its controller's Units, to show the set beats every other layer. */
const booster = def("st-booster", "Field Spell", 9602);
const body = unitDef(9603, { attack: 4, health: 4 });
/** A Field Trap with no text, to fuse onto the zeroer (R77: a fusion keeps every ingredient's text). */
const blank = def("st-blank", "Field Trap", 9604);

const SCRIPTS: Record<string, CardScripts> = {
  [zeroer.id]: {
    base: {
      aura: ({ self }) => [{ applies: (unit) => unit.controller !== self.controller, mod: { setAttack: 0 } }],
      tributeWhen: ({ state, self }) => activeUnitsOf(state, opponentOf(self.controller)).length === 0,
    },
    radiant: {},
  },
  [booster.id]: {
    base: { aura: ({ self }) => [{ applies: (unit) => unit.controller === self.controller, mod: { attack: 3 } }] },
    radiant: {},
  },
};

function game(seed: string): ReturnType<typeof newGame> {
  const state = newGame(`self-tribute-${seed}`);
  registerCatalog({ ...registeredCatalog(), [zeroer.id]: zeroer, [booster.id]: booster, [body.id]: body, [blank.id]: blank });
  registerScripts({ ...registeredScripts(), ...SCRIPTS });
  state.phase = "main";
  return state;
}

describe("§10.4 an aura that sets attack applies after every other layer (C #88, R403)", () => {
  it("R403 a buff and a +3 aura do not lift it: the unit reads 0, raw and floored", () => {
    const state = game("set");
    put(state, zeroer.id, slot("p1", "backrow", 1));
    put(state, booster.id, slot("p2", "backrow", 1));
    const enemy = put(state, body.id, slot("p2", "units", 1));
    enemy.buffs.attack += 5;

    expect(unitView(state, enemy).attack).toBe(0);
    expect(unclampedAttack(state, enemy)).toBe(0);
    // Its own side is untouched by it.
    const mine = put(state, body.id, slot("p1", "units", 1));
    expect(unitView(state, mine).attack).toBe(4);
  });
});

describe("'When …, Tribute this' is read at every state check (C #88, R403)", () => {
  it("R403 while the condition holds the card is sacrificed, face-down, at the next check", () => {
    const state = game("tribute");
    const trap = put(state, zeroer.id, slot("p1", "backrow", 1));
    trap.faceUp = false;
    const sink = sinkFor(state);

    stateCheck(sink);

    expect(trap.zone.z).toBe("graveyard");
    expect(eventsOfType(sink.events, "destroyed").map((event) => event.instanceId)).toEqual([trap.id]);
  });

  it("R403 while the opponent has a Unit it stays; once it has none, the next check takes it", () => {
    const state = game("stays");
    const trap = put(state, zeroer.id, slot("p1", "backrow", 1));
    const enemy = put(state, body.id, slot("p2", "units", 1));
    const sink = sinkFor(state);

    stateCheck(sink);
    expect(trap.zone.z).toBe("field");

    enemy.markedDestroyed = true;
    stateCheck(sink);

    expect(enemy.zone.z).toBe("graveyard");
    expect(trap.zone.z).toBe("graveyard");
  });
});

describe("a fusion keeps both additions of its ingredients (R77, R102, R403)", () => {
  it("R403 a Field Trap fused onto the zeroer still zeroes enemy attack, and is tributed when they have no Unit", () => {
    const state = game("fused");
    const target = put(state, zeroer.id, slot("p1", "backrow", 1));
    const enemy = put(state, body.id, slot("p2", "units", 1));
    const sink = sinkFor(state);
    const [ingredient] = inHand(state, blank.id, "p1");
    if (ingredient === undefined) throw new Error("no ingredient");

    const fused = fuse(sink, { ingredients: [ingredient], target });

    expect(fused?.id).toBe(target.id);
    expect(unitView(state, enemy).attack).toBe(0);
    stateCheck(sink);
    expect(target.zone.z).toBe("field");

    enemy.markedDestroyed = true;
    stateCheck(sink);
    expect(target.zone.z).toBe("graveyard");
  });
});
