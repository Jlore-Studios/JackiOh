// C #74 Corpse Plantation — SPEC §8.6 row 74, BUILD M9 Classic row C 74: "Cry: one placement of 2 Plague
// Tokens on itself; while it has tokens, `legalActions` offers `play` for Units in your graveyard, the
// action carrying how many tokens pay, at least 1 and at most the tokens on it and the price, each paying
// (1) and the rest paid in mana; it is a play (the Unit's Cry fires and it counts as played), not a free
// cast; a (0) Cost Unit can't use it; with no tokens left it offers nothing; tokens others place add to
// it, and leaving the field resets them (R78); radiant: 4 tokens; its tuned number (tokens) reads through
// `param()` (R386)".
//
// The harness's `play` takes a card from a hand, so a graveyard play is sent to `reduce` as
// `legalActions` offers it and read back off its result (as C #28 Second Wind's test does).

import { findInstance, legalActions, reduce, stepParam, type CardInstance, type GameState } from "@jackioh/engine";
import type { ActionBody, GameEvent, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/074-corpse-plantation";

const PLANTATION = "classic-074";
const CRAWLER = "classic-053"; // (1) Unit: Cry: place a Plague Counter on another permanent.
const SLIME = "classic-027"; // (0) Unit 1/1.
const MENACE = "core-019"; // (3) Unit 9/9 Taunt.
const MR_TOKEN = "core-015"; // (1) Unit 1/1: Cry: Summon a Rush Token.
const VANILLA = "core-008"; // (1) Unit 4/4.
const STOCKPILE = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
const COLLATERAL = "core-034"; // (4) Spell: Exile target permanent and a random card from your opponent's deck.
const ANCHOR = "core-010"; // (0) Spell (§2.5).
const RUSH_TOKEN = "core-t-rush";

type Play = Extract<ActionBody, { type: "play" }>;
type Step = { state: GameState; events: GameEvent[] };

function graveyardPlays(state: GameState, card: CardInstance): Play[] {
  return legalActions(state, "p1").filter((action): action is Play => action.type === "play" && action.instanceId === card.id);
}

/** The token counts legalActions offers for a play of `card`, each once. */
function tokenOptions(state: GameState, card: CardInstance): (number | null)[] {
  return [...new Set(graveyardPlays(state, card).map((play) => play.plague?.tokens ?? null))].sort();
}

let nonce = 0;
function send(state: GameState, body: Omit<Play, "playerId" | "type">): Step & { error?: string } {
  nonce += 1;
  const result = reduce(state, { type: "play", playerId: "p1", ...body, nonce: `plantation-${nonce}` });
  return { state: result.state, events: result.events, ...(result.error === undefined ? {} : { error: result.error }) };
}

function playFromGraveyard(state: GameState, card: CardInstance, plague: { from: CardInstance; tokens: number }, targets?: Selection[]): Step {
  const result = send(state, {
    instanceId: card.id,
    plague: { from: plague.from.id, tokens: plague.tokens },
    ...(targets === undefined ? {} : { targets }),
  });
  if (result.error !== undefined) throw new Error(result.error);
  return result;
}

function tokensOn(state: GameState, card: CardInstance): number {
  return findInstance(state, card.id)?.counters.plague ?? 0;
}

/** The Plantation already standing with `tokens` on it, and a graveyard of Units and a Spell. */
function standing(tokens: number, graveyard: readonly string[] = [MENACE, MR_TOKEN, STOCKPILE, SLIME], mana?: number): Scenario {
  return scenario({
    p1: { hand: [ANCHOR], backrow: [{ def: PLANTATION, counters: tokens > 0 ? { plague: tokens } : {} }], graveyard: [...graveyard], ...(mana === undefined ? {} : { mana }) },
    p2: { hand: [ANCHOR] },
  });
}

describe("C #74 Corpse Plantation", () => {
  it("declares its one number and its permission (Units, paid with its tokens), one script on both faces", () => {
    expect(def.id).toBe(PLANTATION);
    expect(def.params).toEqual([{ key: "tokens", base: 2, radiant: 4, better: "up", step: 1, min: 1 }]);
    const s = standing(0);
    expect(base.graveyardPlay?.({ state: s.state, self: s.card(PLANTATION), radiant: false })).toEqual([{ units: true, plague: true }]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("Cry: one placement of 2 Plague Counters on itself", () => {
      const s = scenario({ p1: { hand: [PLANTATION, ANCHOR] }, p2: { hand: [ANCHOR] } });

      s.play(PLANTATION);

      const plantation = s.card(PLANTATION);
      expect(plantation.counters.plague).toBe(2);
      expect(s.events.filter((event) => event.type === "counterChanged")).toEqual([
        { type: "counterChanged", instanceId: plantation.id, counter: "plague", value: 2, placed: 2 },
      ]);
    });

    it("while it has tokens, legalActions offers each graveyard Unit with 1 up to min(tokens, price) tokens paying, and no Spell", () => {
      const s = standing(2);
      const [menace, mrToken, stockpile] = [s.card(MENACE), s.card(MR_TOKEN), s.card(STOCKPILE)];

      expect(tokenOptions(s.state, menace)).toEqual([1, 2]);
      expect(tokenOptions(s.state, mrToken)).toEqual([1]);
      expect(graveyardPlays(s.state, stockpile)).toEqual([]);
      // Every offer spends tokens: the permission pays in tokens and mana, never in mana alone.
      expect(graveyardPlays(s.state, menace).every((play) => play.plague?.from === s.card(PLANTATION).id)).toBe(true);
    });

    it("a play: each token pays (1), the rest in mana, and the spent tokens come off it", () => {
      const s = standing(2);
      const plantation = s.card(PLANTATION);
      const menace = s.card(MENACE);

      const after = playFromGraveyard(s.state, menace, { from: plantation, tokens: 2 });

      expect(findInstance(after.state, menace.id)?.zone.z).toBe("field");
      expect(after.state.players.p1.mana.current).toBe(4 - 1);
      expect(tokensOn(after.state, plantation)).toBe(0);
      expect(after.events.filter((event) => event.type === "counterChanged")).toEqual([
        { type: "counterChanged", instanceId: plantation.id, counter: "plague", value: 0 },
      ]);
    });

    it("it is a play, not a free cast: the Unit's Cry fires and it counts as played", () => {
      const s = standing(2);
      const mrToken = s.card(MR_TOKEN);

      const after = playFromGraveyard(s.state, mrToken, { from: s.card(PLANTATION), tokens: 1 });

      expect(after.events.some((event) => event.type === "cardPlayed" && event.instanceId === mrToken.id)).toBe(true);
      expect(after.events.some((event) => event.type === "summoned" && event.defId === RUSH_TOKEN)).toBe(true);
      expect(after.state.players.p1.turnLog.playedIds).toContain(mrToken.id);
      expect(after.state.players.p1.mana.current).toBe(4);
    });

    it("R81 its choices are as from hand: a Plague Crawler from the graveyard declares its target and places on it", () => {
      const s = scenario({
        p1: { hand: [ANCHOR], backrow: [{ def: PLANTATION, counters: { plague: 1 } }], graveyard: [CRAWLER] },
        p2: { hand: [ANCHOR], field: [VANILLA] },
      });
      const vanilla = s.card(VANILLA);

      const after = playFromGraveyard(s.state, s.card(CRAWLER), { from: s.card(PLANTATION), tokens: 1 }, [{ pick: "instance", instanceId: vanilla.id }]);

      expect(tokensOn(after.state, vanilla)).toBe(1);
    });

    it("a (0) Cost Unit can't use it", () => {
      const s = standing(2);
      const slime = s.card(SLIME);

      expect(graveyardPlays(s.state, slime)).toEqual([]);
      expect(send(s.state, { instanceId: slime.id, plague: { from: s.card(PLANTATION).id, tokens: 1 } }).error).toBeDefined();
    });

    it("the refusals agree with legalActions: no tokens, too many, more than the price, or mana alone", () => {
      const s = standing(2);
      const plantation = s.card(PLANTATION).id;
      const mrToken = s.card(MR_TOKEN).id;
      const menace = s.card(MENACE).id;

      expect(send(s.state, { instanceId: menace }).error).toMatch(/Plague Counters/);
      expect(send(s.state, { instanceId: menace, plague: { from: plantation, tokens: 0 } }).error).toBeDefined();
      expect(send(s.state, { instanceId: menace, plague: { from: plantation, tokens: 3 } }).error).toBeDefined();
      expect(send(s.state, { instanceId: mrToken, plague: { from: plantation, tokens: 2 } }).error).toBeDefined();
    });

    it("the rest of the price must be in mana: with none, only a full token payment is offered", () => {
      const s = standing(2, [MENACE, MR_TOKEN], 0);

      expect(tokenOptions(s.state, s.card(MENACE))).toEqual([]);
      expect(tokenOptions(s.state, s.card(MR_TOKEN))).toEqual([1]);
    });

    it("with no tokens left it offers nothing", () => {
      const s = standing(1, [MR_TOKEN, VANILLA]);
      const after = playFromGraveyard(s.state, s.card(MR_TOKEN), { from: s.card(PLANTATION), tokens: 1 });

      expect(tokensOn(after.state, s.card(PLANTATION))).toBe(0);
      expect(graveyardPlays(after.state, s.card(VANILLA))).toEqual([]);
      const zero = standing(0, [VANILLA]);
      expect(graveyardPlays(zero.state, zero.card(VANILLA))).toEqual([]);
    });

    it("tokens others place add to it: a Plague Crawler's placement makes 3 to pay with", () => {
      const s = scenario({
        p1: { hand: [CRAWLER, ANCHOR], backrow: [{ def: PLANTATION, counters: { plague: 2 } }], graveyard: [MENACE] },
        p2: { hand: [ANCHOR] },
      });
      const plantation = s.card(PLANTATION);

      s.play(CRAWLER, { targets: [{ pick: "instance", instanceId: plantation.id }] });

      expect(s.card(plantation).counters.plague).toBe(3);
      expect(tokenOptions(s.state, s.card(MENACE))).toEqual([1, 2, 3]);
    });

    it("R78 leaving the field ends it and resets its tokens", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [ANCHOR], backrow: [{ def: PLANTATION, counters: { plague: 2 } }], graveyard: [MENACE], library: [VANILLA] },
        p2: { hand: [COLLATERAL, ANCHOR], library: [VANILLA] },
      });
      const plantation = s.card(PLANTATION);

      s.play(COLLATERAL, { targets: [{ pick: "instance", instanceId: plantation.id }] });
      s.endTurn();

      s.expectInZone(plantation, "exile");
      expect(s.card(plantation).counters.plague).toBeUndefined();
      expect(graveyardPlays(s.state, s.card(MENACE))).toEqual([]);
    });

    it("§3 with your unit row full nothing is offered, and the play is refused", () => {
      const s = scenario({
        p1: { hand: [ANCHOR], field: [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA], backrow: [{ def: PLANTATION, counters: { plague: 2 } }], graveyard: [MR_TOKEN] },
        p2: { hand: [ANCHOR] },
      });
      const mrToken = s.card(MR_TOKEN);

      expect(graveyardPlays(s.state, mrToken)).toEqual([]);
      expect(send(s.state, { instanceId: mrToken.id, plague: { from: s.card(PLANTATION).id, tokens: 1 } }).error).toBeDefined();
    });

    it("only your own graveyard: the opponent's Units are not offered", () => {
      const s = scenario({
        p1: { hand: [ANCHOR], backrow: [{ def: PLANTATION, counters: { plague: 2 } }] },
        p2: { hand: [ANCHOR], graveyard: [MENACE] },
      });

      expect(legalActions(s.state, "p1").some((action) => action.type === "play" && action.instanceId === s.card(MENACE).id)).toBe(false);
    });

    it("R386 an Upgrade places 3 on itself", () => {
      const s = scenario({ p1: { hand: [PLANTATION, ANCHOR] }, p2: { hand: [ANCHOR] } });
      stepParam(s.card(PLANTATION), "tokens", 1);

      s.play(PLANTATION);

      expect(s.card(PLANTATION).counters.plague).toBe(3);
    });
  });

  describe("radiant", () => {
    it("Cry: one placement of 4 on itself, and a 3 Cost Unit may be paid wholly in tokens", () => {
      const s = scenario({ p1: { hand: [{ def: PLANTATION, radiant: true }, ANCHOR], graveyard: [MENACE], mana: 2 }, p2: { hand: [ANCHOR] } });

      s.play(PLANTATION);

      const plantation = s.card(PLANTATION);
      expect(plantation.counters.plague).toBe(4);
      // 0 mana left: only the full payment in tokens is offered.
      expect(tokenOptions(s.state, s.card(MENACE))).toEqual([3]);
      const after = playFromGraveyard(s.state, s.card(MENACE), { from: plantation, tokens: 3 });
      expect(tokensOn(after.state, plantation)).toBe(1);
      expect(after.state.players.p1.mana.current).toBe(0);
    });

    it("R386 a Degrade places 3", () => {
      const s = scenario({ p1: { hand: [{ def: PLANTATION, radiant: true }, ANCHOR] }, p2: { hand: [ANCHOR] } });
      stepParam(s.card(PLANTATION), "tokens", -1);

      s.play(PLANTATION);

      expect(s.card(PLANTATION).counters.plague).toBe(3);
    });
  });
});
