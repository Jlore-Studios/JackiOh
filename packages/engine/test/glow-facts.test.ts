// R662: the board facts the yellow glow reads for the cards R195 left out (`src/query.ts`), and the
// granted half of `conditionActive` (`src/condition.ts` rule 5): a hand card glows while a condition
// another card grants it holds. The real cards (#38, #64, #78, #96, #85) prove the same again in
// their own test files; here the granting cards are test-only definitions carrying the same static
// flags and modifiers the pipeline reads, registered on top of the fixture catalog and put back in
// `afterAll`.

import type { CardDef } from "@jackioh/shared";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { conditionActive } from "../src/condition";
import { fusablePermanentsOf, giftedWouldMakeRadiant, grantedComboLive, lethalAttackersOf } from "../src/query";
import type { CardScripts, Script } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import type { GameState, PlayerModifier } from "../src/state";
import { viewFor } from "../src/viewFor";
import { immutable } from "./fixtures/callToChaosPlus";
import { plain, trampler } from "./fixtures/combat";
import { inHand, newGame, put, slot } from "./fixtures/harness";

let nextIndex = 1980;
function fieldSpell(name: string): CardDef {
  nextIndex += 1;
  return {
    id: `gf-${name}`,
    index: String(nextIndex),
    name,
    set: "Core",
    type: "Field Spell",
    tags: [],
    rarity: "Common",
    token: false,
    cost: 1,
    base: { keywords: [], text: name },
    radiant: { keywords: [], text: name },
  };
}

/** #38's flag and #64's threshold, on Field Spells with no other text. */
const striker = fieldSpell("striker");
const gifted = fieldSpell("gifted");
const twoCost: CardDef = { ...fieldSpell("two-cost"), cost: 2 };

function both(script: Script): CardScripts {
  return { base: script, radiant: script };
}

const DEFS = [striker, gifted, twoCost, immutable];
const SCRIPTS: Record<string, CardScripts> = {
  [striker.id]: both({ staticFlags: { quickstriker: true } }),
  [gifted.id]: both({ staticFlags: { giftedProgram: 1 } }),
};

let savedCatalog: ReturnType<typeof registeredCatalog>;
let savedScripts: ReturnType<typeof registeredScripts>;

beforeAll(() => {
  savedCatalog = registeredCatalog();
  savedScripts = registeredScripts();
});

afterAll(() => {
  registerCatalog(savedCatalog);
  registerScripts(savedScripts);
});

function board(seed: string): GameState {
  const state = newGame(seed);
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries(DEFS.map((def) => [def.id, def])) });
  registerScripts({ ...registeredScripts(), ...SCRIPTS });
  state.turn = 4;
  state.active = "p1";
  state.phase = "main";
  state.pending = null;
  state.players.p1.mana.current = 5;
  return state;
}

/** As §10.5 step 4 counts a play this turn. */
function playedOne(state: GameState, paid = 3): void {
  const log = state.players.p1.turnLog;
  log.cardsPlayed += 1;
  log.playedIds.push(`played-${log.cardsPlayed}`);
  log.costsPaid = [...(log.costsPaid ?? []), paid];
}

type Rider = { kind: "comboDraw"; amount: number } | { kind: "quickstrikerDamage" };

function mod(state: GameState, body: Rider): void {
  const rider: PlayerModifier = {
    id: `gf-mod-${state.players.p1.mods.length}`,
    expiry: { until: "thisTurn", turn: state.turn },
    ...body,
  };
  state.players.p1.mods.push(rider);
}

describe("R662 grantedComboLive: a granted Combo answers the next play", () => {
  it("R662 a Quickstriker's flag on the field is live once a card has been played this turn", () => {
    const state = board("gf-striker");
    put(state, striker.id, slot("p1", "backrow", 1));
    expect(grantedComboLive(state, "p1")).toBe(false);
    playedOne(state);
    expect(grantedComboLive(state, "p1")).toBe(true);
    // The other seat's plays and permanents are its own.
    expect(grantedComboLive(state, "p2")).toBe(false);
  });

  it("R662 a comboDraw or quickstrikerDamage rider counts; a spent turn's rider, or a draw of 0, does not", () => {
    const state = board("gf-riders");
    playedOne(state);
    expect(grantedComboLive(state, "p1")).toBe(false);
    mod(state, { kind: "comboDraw", amount: 0 });
    expect(grantedComboLive(state, "p1")).toBe(false);
    mod(state, { kind: "comboDraw", amount: 1 });
    expect(grantedComboLive(state, "p1")).toBe(true);

    const later = board("gf-riders-2");
    playedOne(later);
    later.players.p1.mods.push({ id: "old", kind: "quickstrikerDamage", expiry: { until: "thisTurn", turn: 1 } });
    expect(grantedComboLive(later, "p1")).toBe(false);
    mod(later, { kind: "quickstrikerDamage" });
    expect(grantedComboLive(later, "p1")).toBe(true);
  });
});

describe("R662 giftedWouldMakeRadiant: step 3's question, asked of a hand card now", () => {
  it("R662 the first card costing the threshold or less this turn, and not one already Radiant", () => {
    const state = board("gf-gifted");
    put(state, gifted.id, slot("p1", "backrow", 1));
    const [cheap] = inHand(state, plain.id, "p1");
    const [dear] = inHand(state, twoCost.id, "p1");
    expect(giftedWouldMakeRadiant(state, "p1", cheap!)).toBe(true);
    expect(giftedWouldMakeRadiant(state, "p1", dear!)).toBe(false);

    cheap!.radiant = true;
    expect(giftedWouldMakeRadiant(state, "p1", cheap!)).toBe(false);
    cheap!.radiant = false;

    // R213: a cheap card played earlier this turn was the first.
    playedOne(state, 1);
    expect(giftedWouldMakeRadiant(state, "p1", cheap!)).toBe(false);
  });

  it("R662 without a Gifted Program on the player's side nothing qualifies", () => {
    const state = board("gf-gifted-none");
    put(state, gifted.id, slot("p2", "backrow", 1));
    const [cheap] = inHand(state, plain.id, "p1");
    expect(giftedWouldMakeRadiant(state, "p1", cheap!)).toBe(false);
  });
});

describe("R662 lethalAttackersOf: R44's projection over the enemy units acting now", () => {
  it("R662 names the enemy units whose attack on the hero would be lethal, and only those", () => {
    const state = board("gf-lethal");
    const big = put(state, trampler.id, slot("p2", "units", 1)); // 6/4
    const small = put(state, plain.id, slot("p2", "units", 2)); // 3/3
    const mine = put(state, plain.id, slot("p1", "units", 1));
    expect(lethalAttackersOf(state, "p1")).toEqual([]);

    state.players.p1.hero.health = 6;
    expect(lethalAttackersOf(state, "p1").map((unit) => unit.id)).toEqual([big.id]);
    state.players.p1.hero.health = 3;
    expect(lethalAttackersOf(state, "p1").map((unit) => unit.id)).toEqual([big.id, small.id]);
    // Armor counts (§4.4 step 2): neither blow gets through 10 Armor.
    state.players.p1.hero.armor = 10;
    expect(lethalAttackersOf(state, "p1")).toEqual([]);
    // A player's own units are never a threat to their own hero: p2's list is p1's unit alone.
    state.players.p2.hero.health = 1;
    expect(lethalAttackersOf(state, "p2").map((unit) => unit.id)).toEqual([mine.id]);
  });
});

describe("R662 fusablePermanentsOf: where #85's Fuse could land", () => {
  it("R662 every permanent of the player's but the one excepted, and never an Immutable one", () => {
    const state = board("gf-fusable");
    const trap = put(state, striker.id, slot("p1", "backrow", 1));
    expect(fusablePermanentsOf(state, "p1", trap.id)).toEqual([]);
    put(state, immutable.id, slot("p1", "units", 1));
    expect(fusablePermanentsOf(state, "p1", trap.id)).toEqual([]);
    const body = put(state, plain.id, slot("p1", "units", 2));
    expect(fusablePermanentsOf(state, "p1", trap.id).map((card) => card.id)).toEqual([body.id]);
    expect(fusablePermanentsOf(state, "p1", null).map((card) => card.id).sort()).toEqual([body.id, trap.id].sort());
  });
});

describe("R662 conditionActive rule 5: a hand card glows for a condition another card grants", () => {
  it("R662 a Quickstriker on the field and a card played: every hand card glows, in the owner's view only", () => {
    const state = board("gf-glow-striker");
    put(state, striker.id, slot("p1", "backrow", 1));
    const [card] = inHand(state, plain.id, "p1");
    expect(conditionActive(state, card!, "p1", "hand")).toBe(false);
    playedOne(state);
    expect(conditionActive(state, card!, "p1", "hand")).toBe(true);
    const hand = viewFor(state, "p1").you.hand;
    expect(Array.isArray(hand) && hand.every((view) => view.conditionActive === true)).toBe(true);
    // Rule 3 still stands first: not in the other seat's turn.
    state.active = "p2";
    expect(conditionActive(state, card!, "p1", "hand")).toBe(false);
  });

  it("R662 a Gifted Program lights the hand cards it would make Radiant, and not the rest", () => {
    const state = board("gf-glow-gifted");
    put(state, gifted.id, slot("p1", "backrow", 1));
    const [cheap] = inHand(state, plain.id, "p1");
    const [dear] = inHand(state, twoCost.id, "p1");
    expect(conditionActive(state, cheap!, "p1", "hand")).toBe(true);
    expect(conditionActive(state, dear!, "p1", "hand")).toBe(false);
  });

  it("R662 a granted condition never lights a card on the field", () => {
    const state = board("gf-glow-field");
    put(state, striker.id, slot("p1", "backrow", 1));
    const unit = put(state, plain.id, slot("p1", "units", 1));
    playedOne(state);
    expect(conditionActive(state, unit, "p1", "field")).toBe(false);
  });
});
