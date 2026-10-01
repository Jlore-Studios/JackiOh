// C #54 Rewind — SPEC §8.6 row 54, BUILD M9 Classic row C 54: "A declared target: one of your Units on
// the field (the top of a pile) or a Unit card in your graveyard that has a Cry; its Cry runs with that
// Unit as `self` under your control, its choices yours as prompts (R70's caster picks); a Cry that acts
// on "this" finds nothing when the Unit is in a graveyard; running a Cry is not a play of that Unit; no
// Unit with a Cry → it may still be played and fizzles, counting as played (§8's conventions, R90);
// radiant: any Unit with a Cry on the field or in either graveyard, its Cry run twice, each run with
// its own choices; its tuned number (repeats) reads through `param()` (R386)".
//
// The Cries are Core cards with their own tests: Gary the Gambler (flips coins to buff itself), Me and
// Mr Token (summons a Rush Token), Duplicating Felinors (summons a copy of itself), Bigot (destroys a
// target enemy non-Human Unit) and Twisted Sorcerer (deals 4 damage to a target). Mr. Vanilla has no
// Cry; Felinor Fiender's Stack buries a card beneath it.

import { cardsPlayedThisTurn, legalActions, reduce, stepParam, type GameState } from "@jackioh/engine";
import type { Action, PlayerId, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/054-rewind";

const REWIND = "classic-054";
const GARY = "core-004"; // 1/1, Cry: flip 5 coins, +1 attack per heads, +1 health per tails.
const MR_TOKEN = "core-015"; // 1/1, Cry: summon a Rush Token.
const DUPLICATING = "core-012"; // 3/4, Cry: summon a copy of this.
const BIGOT = "core-002"; // 6/1 Human, Cry: destroy target enemy non-Human Unit.
const SORCERER = "core-068"; // 5/5, Cry: deal 4 damage to a target (8 under 10 health).
const VANILLA = "core-008"; // 4/4, no text.
const MENACE = "core-019"; // 9/9 Taunt.
const FIENDER = "core-092"; // Stack.
const RUSH_TOKEN = "core-t-rush";
const LUNAR = "core-035";
const FILLER = "core-005";

function pick(id: string): Selection[] {
  return [{ pick: "instance", instanceId: id }];
}

/** The instance ids `legalActions` offers as Rewind's target. */
function offered(s: Scenario, player: PlayerId = "p1"): string[] {
  const rewind = s.hand(player).find((card) => card.defId === REWIND);
  if (rewind === undefined) throw new Error("Rewind should be in hand");
  return legalActions(s.state, player).flatMap((action) =>
    action.type === "play" && action.instanceId === rewind.id
      ? (action.targets ?? []).flatMap((selection) => (selection.pick === "instance" ? [selection.instanceId] : []))
      : [],
  );
}

function tokensOf(s: Scenario, player: PlayerId): number {
  return [1, 2, 3, 4, 5].filter((lane) => s.unit(player, lane)?.defId === RUSH_TOKEN).length;
}

describe("C #54 Rewind", () => {
  it("declares one target, a Unit with a Cry on the field or in a graveyard: your side on the base face, either on the Radiant", () => {
    expect(def.id).toBe(REWIND);
    expect(def.params).toEqual([{ key: "repeats", base: 1, radiant: 2, better: "up", step: 1, min: 1 }]);
    expect(base.targets).toEqual([
      { kind: "target", min: 1, max: 1, filter: { side: "ally", of: ["unit", "graveyard"], check: "hasCry" } },
    ]);
    expect(radiant.targets).toEqual([
      { kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "graveyard"], check: "hasCry" } },
    ]);
  });

  describe("base", () => {
    it("on the field: the Cry runs with that Unit as itself — Gary flips its coins and buffs itself", () => {
      const s = scenario({ p1: { hand: [REWIND, FILLER], field: [GARY] }, p2: { hand: [FILLER] } });
      const gary = s.card(GARY);
      s.play(REWIND, { targets: pick(gary.id) });
      const stats = s.stats(gary);
      // Five coins, one point each, split between attack and health.
      expect(stats.attack + stats.maxHealth).toBe(1 + 1 + 5);
      s.expectEvents("cardPlayed", "buffed");
    });

    it("in your graveyard: the Cry runs under your control — Me and Mr Token summons a Rush Token for you", () => {
      const s = scenario({ p1: { hand: [REWIND, FILLER], graveyard: [MR_TOKEN] }, p2: { hand: [FILLER] } });
      s.play(REWIND, { targets: pick(s.card(MR_TOKEN).id) });
      expect(tokensOf(s, "p1")).toBe(1);
      s.expectInZone(MR_TOKEN, "graveyard");
    });

    it("a Cry that acts on \"this\" finds nothing in a graveyard: Gary buffs nothing, Duplicating Felinors copies nothing", () => {
      const s = scenario({ p1: { hand: [REWIND, REWIND, FILLER], graveyard: [GARY, DUPLICATING] }, p2: { hand: [FILLER] } });
      const [first, second] = s.hand("p1");
      if (first === undefined || second === undefined) throw new Error("two Rewinds should be in hand");
      s.play(first, { targets: pick(s.card(GARY).id) });
      s.play(second, { targets: pick(s.card(DUPLICATING).id) });
      expect(s.events.some((event) => event.type === "buffed" || event.type === "summoned")).toBe(false);
      expect(units(s, "p1")).toEqual([null, null, null, null, null]);
    });

    it("R70 its choices are yours, as prompts: Bigot asks you which enemy non-Human Unit to destroy", () => {
      const s = scenario({
        p1: { hand: [REWIND, FILLER], field: [BIGOT] },
        p2: { hand: [FILLER], field: [VANILLA, DUPLICATING, MENACE] },
      });
      s.play(REWIND, { targets: pick(s.card(BIGOT).id) });
      const pending = s.state.pending;
      expect(pending?.playerId).toBe("p1");
      expect(pending?.options.map((option) => option.selection)).toEqual([
        { pick: "instance", instanceId: s.card(DUPLICATING).id },
        { pick: "instance", instanceId: s.card(MENACE).id },
      ]);
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });
      s.answer(s.card(MENACE).id);
      s.expectInZone(MENACE, "graveyard").expectInZone(DUPLICATING, "field");
    });

    it("running a Cry is not a play of that Unit: only Rewind counts as played", () => {
      const s = scenario({ p1: { hand: [REWIND, FILLER], field: [MR_TOKEN] }, p2: { hand: [FILLER] } });
      const mrToken = s.card(MR_TOKEN);
      s.play(REWIND, { targets: pick(mrToken.id) });
      expect(cardsPlayedThisTurn(s.state, "p1")).toBe(1);
      const played = s.events.flatMap((event) => (event.type === "cardPlayed" ? [event.defId] : []));
      expect(played).toEqual([REWIND]);
      expect(tokensOf(s, "p1")).toBe(1);
    });

    it("R90 with no Unit that has a Cry it is still played, fizzles, and counts as played", () => {
      const s = scenario({ p1: { hand: [REWIND, FILLER], field: [VANILLA], graveyard: [LUNAR] }, p2: { hand: [FILLER], field: [GARY] } });
      const rewind = s.card(REWIND);
      expect(legalActions(s.state, "p1").some((action) => action.type === "play" && action.instanceId === rewind.id)).toBe(true);
      expect(offered(s)).toEqual([]);
      s.play(rewind);
      s.expectInZone(rewind, "graveyard").expectEvents("cardPlayed", "cardResolved");
      expect(cardsPlayedThisTurn(s.state, "p1")).toBe(1);
      expect(s.events.some((event) => event.type === "buffed")).toBe(false);
    });

    it("R90 only your Units are offered: an enemy Unit and the enemy graveyard are refused, and legalActions agrees", () => {
      const s = scenario({
        p1: { hand: [REWIND, FILLER], field: [GARY], graveyard: [MR_TOKEN] },
        p2: { hand: [FILLER], field: [SORCERER], graveyard: [BIGOT] },
      });
      expect(new Set(offered(s))).toEqual(new Set([s.card(GARY).id, s.card(MR_TOKEN).id]));
      expect(() => s.play(REWIND, { targets: pick(s.card(SORCERER).id) })).toThrow();
      expect(() => s.play(REWIND, { targets: pick(s.card(BIGOT).id) })).toThrow();
      s.expectInZone(REWIND, "hand");
    });

    it("a Unit with no Cry, a Spell in the graveyard and a card dormant under a Stack pile are never offered (R13)", () => {
      const s = scenario({
        p1: { hand: [REWIND, FILLER], field: [VANILLA, GARY, { def: FIENDER, stack: true }], graveyard: [LUNAR] },
        p2: { hand: [FILLER] },
      });
      expect(offered(s)).toEqual([]);
      expect(() => s.play(REWIND, { targets: pick(s.card(GARY).id) })).toThrow();
    });

    it("a paused triggered Cry survives a round trip: the frozen state answers to the same game", () => {
      const s = scenario({ p1: { hand: [REWIND, FILLER], field: [BIGOT] }, p2: { hand: [FILLER], field: [MENACE] } });
      s.play(REWIND, { targets: pick(s.card(BIGOT).id) });
      const pending = s.state.pending;
      expect(pending).not.toBeNull();
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(thawed).toEqual(s.state);
      const answer = {
        type: "answer",
        choiceId: pending?.id ?? "",
        selection: pick(s.card(MENACE).id),
        playerId: "p1",
        nonce: "rewind-roundtrip",
      } as Action;
      const live = reduce(s.state, answer);
      const frozen = reduce(thawed, answer);
      expect(live.error).toBeUndefined();
      expect(frozen.state).toEqual(live.state);
      expect(frozen.events).toEqual(live.events);
      expect(live.state.players.p2.graveyard.map((card) => card.defId)).toEqual([MENACE]);
    });

    it("R386 an Upgrade triggers the Cry twice: Me and Mr Token summons two Rush Tokens", () => {
      const s = scenario({ p1: { hand: [REWIND, FILLER], graveyard: [MR_TOKEN] }, p2: { hand: [FILLER] } });
      stepParam(s.card(REWIND), "repeats", 1);
      s.play(REWIND, { targets: pick(s.card(MR_TOKEN).id) });
      expect(tokensOf(s, "p1")).toBe(2);
    });
  });

  describe("radiant", () => {
    it("reaches an enemy Unit on the field: its Cry runs under your control — the Rush Tokens are yours, twice", () => {
      const s = scenario({ p1: { hand: [{ def: REWIND, radiant: true }, FILLER] }, p2: { hand: [FILLER], field: [MR_TOKEN] } });
      s.play(REWIND, { targets: pick(s.card(MR_TOKEN).id) });
      expect(tokensOf(s, "p1")).toBe(2);
      expect(tokensOf(s, "p2")).toBe(0);
    });

    it("reaches either graveyard", () => {
      const s = scenario({
        p1: { hand: [{ def: REWIND, radiant: true }, FILLER], graveyard: [GARY] },
        p2: { hand: [FILLER], graveyard: [MR_TOKEN] },
      });
      expect(new Set(offered(s))).toEqual(new Set([s.card(GARY).id, s.card(MR_TOKEN).id]));
      s.play(REWIND, { targets: pick(s.card(MR_TOKEN).id) });
      expect(tokensOf(s, "p1")).toBe(2);
    });

    it("runs the Cry twice, each run with its own choices, asked of you one after the other", () => {
      const s = scenario({
        p1: { hand: [{ def: REWIND, radiant: true }, FILLER], field: [SORCERER] },
        p2: { hand: [FILLER], field: [MENACE] },
      });
      s.play(REWIND, { targets: pick(s.card(SORCERER).id) });
      expect(s.state.pending?.playerId).toBe("p1");
      s.answer(s.card(MENACE).id);
      expect(s.state.pending?.playerId).toBe("p1");
      s.answer("hero:p2");
      expect(s.state.pending).toBeNull();
      s.expectStats(MENACE, { health: 5 }).expectHealth("p2", 26);
    });

    it("a pause between the two runs survives a round trip", () => {
      const s = scenario({
        p1: { hand: [{ def: REWIND, radiant: true }, FILLER], field: [SORCERER] },
        p2: { hand: [FILLER], field: [MENACE] },
      });
      s.play(REWIND, { targets: pick(s.card(SORCERER).id) });
      s.answer(s.card(MENACE).id);
      const pending = s.state.pending;
      expect(pending).not.toBeNull();
      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      const answer = {
        type: "answer",
        choiceId: pending?.id ?? "",
        selection: [{ pick: "hero", player: "p2" }],
        playerId: "p1",
        nonce: "rewind-radiant-roundtrip",
      } as Action;
      const live = reduce(s.state, answer);
      const frozen = reduce(thawed, answer);
      expect(live.error).toBeUndefined();
      expect(frozen.state).toEqual(live.state);
      expect(live.state.players.p2.hero.health).toBe(26);
    });

    it("a Unit with no Cry is never offered", () => {
      const s = scenario({ p1: { hand: [{ def: REWIND, radiant: true }, FILLER], field: [VANILLA] }, p2: { hand: [FILLER], field: [VANILLA] } });
      expect(offered(s)).toEqual([]);
    });

    it("R386 a Degrade triggers the Cry once", () => {
      const s = scenario({ p1: { hand: [{ def: REWIND, radiant: true }, FILLER], graveyard: [MR_TOKEN] }, p2: { hand: [FILLER] } });
      stepParam(s.card(REWIND), "repeats", -1);
      s.play(REWIND, { targets: pick(s.card(MR_TOKEN).id) });
      expect(tokensOf(s, "p1")).toBe(1);
    });
  });
});

function units(s: Scenario, player: PlayerId): (string | null)[] {
  return [1, 2, 3, 4, 5].map((lane) => s.unit(player, lane)?.defId ?? null);
}
