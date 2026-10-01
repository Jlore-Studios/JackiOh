// C #57 Echo — SPEC §8.6 row 57, R399, BUILD M9 Classic row C 57: "Reads the game-wide last Spell played
// by either player (`lastSpell = { defId, radiant }`), casts included, a countered Spell never counted
// (it was never played) and a Field Spell never a Spell; in its owner's hand its view carries that
// Spell's face under Echo's name (as R243 carries a fused card's), hidden from the opponent like any hand
// card; played, it declares and resolves that Spell's choices and script on the face recorded; a played
// Echo records the Spell it copied, never Echo, so two Echoes never loop (R399); with no Spell played
// yet it has no text and resolves to nothing; radiant: also Echo 1, one more resolution with fresh
// prompts (§6.2); its name is a rules word, so "Echo 1" in #51 and #79 is no reference to it (R381); its
// tuned number (radiant Echo) reads through `param()` (R386)" — a numbered keyword SPEC's row says
// Degrade and Upgrade move as an X, not a `params` entry, so its R386 proof steps that X.
//
// The text is B5 E14 (`engine/src/subsystems/copiedText.ts`, its engine tests in
// `engine/test/copied-text.test.ts`), with this workstream's rulings R545 (an X-cost text), R546 (the
// copy fixed as the play begins; "this" is Echo) and R547 (its static text: Cast on draw yes, the
// end-of-turn return no).

import { addStep, lastSpellPlayed, legalActions, playedThisGameWithTag, reduce, subsystems, tuningOf, type GameState } from "@jackioh/engine";
import type { Action, GameEvent, PlayerId, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { cardDef } from "../../src/catalog-data";
import { base, def, radiant } from "../../src/scripts/classic/057-echo";
import { scenario, type Scenario } from "../_harness";

const ECHO = "classic-057";
const WILDFIRE = "classic-055"; // (1) Spell: Deal {damage} damage (4, Radiant 8), a declared target.
const GRAND = "classic-072"; // Trap: counters the opponent's non-Unit plays.
const STOCKPILE = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
const RAPID = "core-010"; // (0) Spell: Combo 3: Draw 3.
const ARMOR = "core-073"; // (2) Field Spell
const ADAPTIVE = "core-074"; // (X) Spell: Deal X to a target; heal X; draw X; an X/X Ghoul.
const REMINISCE = "core-072"; // (1) Spell: Discover a card from your GY. It costs (1) less. Exile this.
const DREAM = "core-023"; // (1) Spell: … End of turn: Return this to your hand.
const CN_VIRUS = "core-090-1"; // (1) Spell, Cast on draw: take 1 damage.
const MENACE = "core-019"; // (3) Unit 9/9
const VANILLA = "core-008"; // (1) Unit 4/4
const PALANTIR = "classic-004"; // Base: when your opponent plays a Book, you may Tribute this to steal it.

const AT_P2: Selection[] = [{ pick: "hero", player: "p2" }];

function hits(events: readonly GameEvent[], target: string): number[] {
  return events.flatMap((event) => (event.type === "damage" && event.targetId === target ? [event.amount] : []));
}

function echoPlays(s: Scenario, player: PlayerId = "p1"): Extract<ReturnType<typeof legalActions>[number], { type: "play" }>[] {
  const id = s.card(ECHO).id;
  return legalActions(s.state, player).filter(
    (action): action is Extract<ReturnType<typeof legalActions>[number], { type: "play" }> =>
      action.type === "play" && action.instanceId === id,
  );
}

function ownView(s: Scenario, player: PlayerId = "p1") {
  const hand = s.view(player).you.hand;
  return (Array.isArray(hand) ? hand : []).find((card) => card.defId === ECHO);
}

describe("C #57 Echo", () => {
  it("is a (1) Spell that copies the last Spell's text; its Radiant face adds Echo 1, a numbered keyword", () => {
    expect(def.type).toBe("Spell");
    expect(def.cost).toBe(1);
    expect(def.params).toBeUndefined();
    expect(base.staticFlags).toEqual({ copiesLastSpell: true });
    expect(radiant.staticFlags).toEqual({ copiesLastSpell: true, echo: 1 });
    expect(base.cry).toBeUndefined();
  });

  it("R381 its name is a rules word: \"Echo 1\" in #51 and #79 is no reference to it", () => {
    expect(cardDef("core-051").radiant.text).toContain("Echo 1");
    expect(cardDef("core-051").refs ?? []).not.toContain(ECHO);
    expect(cardDef("core-079").refs ?? []).not.toContain(ECHO);
  });

  describe("base", () => {
    it("R399 with no Spell played yet it has no text: no choices, it resolves to nothing, and records nothing", () => {
      const s = scenario({ p1: { hand: [ECHO, VANILLA] } });
      expect(ownView(s)).not.toHaveProperty("copies");
      expect(echoPlays(s).map((play) => play.targets)).toEqual([undefined]);
      s.play(ECHO);
      expect(s.lastEvents.filter((event) => event.type === "damage" || event.type === "drawn")).toEqual([]);
      s.expectInZone(ECHO, "graveyard").expectMana("p1", 3);
      expect(lastSpellPlayed(s.state)).toBeNull();
    });

    it("R399 it has the text of the last Spell either player played: the opponent's Spell", () => {
      const s = scenario({
        p1: { hand: [ECHO, VANILLA], library: [MENACE, MENACE, MENACE, MENACE] },
        p2: { hand: [STOCKPILE, VANILLA], library: [VANILLA, VANILLA] },
        active: "p2",
      });
      s.play(STOCKPILE).endTurn();
      expect(subsystems.copiedTextOf(s.state, s.card(ECHO))).toEqual({ defId: STOCKPILE, radiant: false });
      s.play(ECHO);
      // Stockpile's text: draw 2, heal your hero 2 — for Echo's player.
      expect(s.lastEvents.filter((event) => event.type === "drawn" && event.player === "p1")).toHaveLength(2);
    });

    it("R399 R70 a cast is a play: a Spell cast on draw is the last Spell", () => {
      const s = scenario({ p1: { hand: [STOCKPILE, ECHO], library: [CN_VIRUS, VANILLA, VANILLA] } });
      s.play(STOCKPILE);
      expect(lastSpellPlayed(s.state)).toEqual({ defId: CN_VIRUS, radiant: false });
      expect(ownView(s)?.copies).toEqual({ defId: CN_VIRUS, radiant: false });
    });

    it("R399 a countered Spell was never played and is never the last Spell", () => {
      const s = scenario({
        p1: { hand: [WILDFIRE, ECHO], backrow: [GRAND] },
        p2: { hand: [STOCKPILE, VANILLA] },
      });
      s.play(WILDFIRE, { targets: AT_P2 }).endTurn();
      s.play(STOCKPILE);
      expect(s.lastEvents.some((event) => event.type === "countered")).toBe(true);
      expect(lastSpellPlayed(s.state)).toEqual({ defId: WILDFIRE, radiant: false });
    });

    it("R399 a Field Spell is never a Spell: playing one leaves the record as it was", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, ARMOR, ECHO], library: [VANILLA], mana: 9 } });
      s.play(WILDFIRE, { targets: AT_P2 }).play(ARMOR);
      expect(lastSpellPlayed(s.state)).toEqual({ defId: WILDFIRE, radiant: false });
    });

    it("R399 R243 in its owner's hand its view carries that Spell's face and numbers under Echo's name and cost", () => {
      const s = scenario({ p1: { hand: [{ def: WILDFIRE, radiant: true }, ECHO] } });
      s.play(WILDFIRE, { targets: AT_P2 });
      const card = ownView(s);
      expect(card).toMatchObject({ defId: ECHO, cost: 1, copies: { defId: WILDFIRE, radiant: true, params: { damage: 8 } } });
    });

    it("R97 hidden from the opponent like any hand card: their view never names it nor what it copies", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, ECHO] } });
      s.play(WILDFIRE, { targets: AT_P2 });
      const theirs = s.view("p2");
      expect(theirs.opponent.hand).toEqual({ count: 1 });
      expect(JSON.stringify(theirs)).not.toContain(ECHO);
      expect(JSON.stringify(theirs)).not.toContain("copies");
    });

    it("R399 played, it declares the copied Spell's choices: legalActions offers exactly the copied Spell's targets", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, ECHO, WILDFIRE] }, p2: { field: [MENACE] } });
      const [first, second] = s.hand("p1").filter((card) => card.defId === WILDFIRE);
      if (first === undefined || second === undefined) throw new Error("setup");
      s.play(first, { targets: AT_P2 });
      const wildfireTargets = legalActions(s.state, "p1")
        .flatMap((action) => (action.type === "play" && action.instanceId === second.id ? [action.targets] : []));
      expect(echoPlays(s).map((play) => play.targets)).toEqual(wildfireTargets);
      expect(() => s.play(ECHO)).toThrow(/target/);
    });

    it("R399 R386 it resolves the copied script on the face recorded, with that face's declared number", () => {
      const s = scenario({ p1: { hand: [{ def: WILDFIRE, radiant: true }, ECHO] }, p2: { field: [MENACE] } });
      s.play(WILDFIRE, { targets: AT_P2 });
      s.play(ECHO, { targets: [{ pick: "instance", instanceId: s.card(MENACE).id }] });
      s.expectStats(MENACE, { health: 1 });
      s.expectInZone(ECHO, "graveyard").expectMana("p1", 2);
    });

    it("R399 a played Echo records the Spell it copied, never itself, so a second Echo copies the same Spell", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, ECHO, ECHO, VANILLA], mana: 9 }, p2: { hand: [VANILLA] } });
      const [one, two] = s.hand("p1").filter((card) => card.defId === ECHO);
      if (one === undefined || two === undefined) throw new Error("setup");
      s.play(WILDFIRE, { targets: AT_P2 }).play(one, { targets: AT_P2 });
      expect(lastSpellPlayed(s.state)).toEqual({ defId: WILDFIRE, radiant: false });
      s.play(two, { targets: AT_P2 });
      expect(hits(s.events, "hero-p2")).toEqual([4, 4, 4]);
      expect(lastSpellPlayed(s.state)).toEqual({ defId: WILDFIRE, radiant: false });
    });

    it("R546 it keeps its own name, cost, type and tags: copying a Book makes no Book play, and C #4 Palantir does not ask", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, ECHO, VANILLA] }, p2: { backrow: [PALANTIR], hand: [VANILLA] } });
      s.play(WILDFIRE, { targets: AT_P2 });
      expect(s.state.pending?.playerId).toBe("p2");
      s.answer("pass");
      expect(ownView(s)).toMatchObject({ defId: ECHO, cost: 1, copies: { defId: WILDFIRE } });
      s.play(ECHO, { targets: AT_P2 });
      expect(s.state.pending).toBeNull();
      expect(s.lastEvents.find((event) => event.type === "cardPlayed")).toMatchObject({ defId: ECHO, costPaid: 1 });
      expect(hits(s.events, "hero-p2")).toEqual([4, 4]);
      expect(playedThisGameWithTag(s.state, "p1", "Book")).toBe(1);
      s.expectInZone(PALANTIR, "field");
    });

    it("R545 an X-cost text: X is chosen with the play, from 1 up to the mana left once Echo's (1) is paid", () => {
      const s = scenario({ p1: { hand: [ADAPTIVE, ECHO], library: [VANILLA, VANILLA, VANILLA], mana: 9 } });
      s.play(ADAPTIVE, { x: 1, targets: AT_P2 });
      s.state.players.p1.mana.current = 4;
      expect([...new Set(echoPlays(s).map((play) => play.x))]).toEqual([1, 2, 3]);
      expect(() => s.play(ECHO, { x: 4, targets: AT_P2 })).toThrow(/X is above/);
      s.play(ECHO, { x: 3, targets: AT_P2 });
      expect(hits(s.lastEvents, "hero-p2")).toEqual([3]);
      s.expectMana("p1", 3);
      // The first Adaptive UI's 1/1 Ghoul holds lane 1; Echo's 3/3 lands beside it (R64).
      const ghost = s.unit("p1", 2);
      expect(ghost === null ? null : s.stats(ghost)).toMatchObject({ attack: 3, health: 3 });
    });

    it("R545 with nothing left once Echo's (1) is paid, an X-cost text cannot be played: absent and refused", () => {
      const s = scenario({ p1: { hand: [ADAPTIVE, ECHO], library: [VANILLA, VANILLA, VANILLA], mana: 9 } });
      s.play(ADAPTIVE, { x: 1, targets: AT_P2 });
      s.state.players.p1.mana.current = 1;
      expect(echoPlays(s)).toEqual([]);
      expect(() => s.play(ECHO, { x: 1, targets: AT_P2 })).toThrow(/X is above/);
      s.expectInZone(ECHO, "hand");
    });

    it("R546 a prompt in the copied text continues the copied script; \"this\" is Echo, exiled where Reminisce says so", () => {
      const s = scenario({ p1: { hand: [REMINISCE, ECHO], graveyard: [MENACE, VANILLA] } });
      s.play(REMINISCE).answer(MENACE);
      s.play(ECHO);
      const pending = s.state.pending;
      expect(pending?.resume.defId).toBe(REMINISCE);
      const round = JSON.parse(JSON.stringify(s.state)) as GameState;
      const vanilla = s.pile("p1", "graveyard").find((card) => card.defId === VANILLA);
      const option = pending?.options.find((entry) => entry.selection.pick === "instance" && entry.selection.instanceId === vanilla?.id);
      const action: Action = {
        type: "answer",
        playerId: "p1",
        nonce: "c57-round-trip",
        choiceId: pending?.id ?? "",
        selection: option === undefined ? [] : [option.selection],
      };
      const live = reduce(s.state, action);
      const revived = reduce(round, action);
      expect(live.error).toBeUndefined();
      expect(revived.state).toEqual(live.state);
      const hand = live.state.players.p1.hand;
      expect(hand.find((card) => card.id === vanilla?.id)?.costMod).toBe(-1);
      // "Exile this": the Echo, not a Reminisce, goes to exile.
      expect(live.state.players.p1.exile.map((card) => card.defId)).toEqual([REMINISCE, ECHO]);
    });

    it("R547 an Echo drawn while the last Spell casts on draw is cast, and the draw repeats", () => {
      const s = scenario({ p1: { hand: [STOCKPILE, VANILLA], library: [CN_VIRUS, ECHO, MENACE, VANILLA] } });
      s.play(STOCKPILE);
      // CN-Virus is cast (1 damage), the draw repeats into Echo, which copies it and is cast too (1).
      expect(hits(s.lastEvents, "hero-p1")).toEqual([1, 1]);
      s.expectInZone(ECHO, "graveyard");
      expect(s.lastEvents.some((event) => event.type === "cardPlayed" && event.defId === ECHO)).toBe(true);
    });

    it("R547 the copied end-of-turn return is not had: an Echo copying Reoccurring Dream stays in the graveyard", () => {
      const s = scenario({ p1: { hand: [DREAM, ECHO, VANILLA] }, p2: { hand: [VANILLA] } });
      s.play(DREAM).play(ECHO).endTurn();
      s.expectInZone(DREAM, "hand");
      s.expectInZone(ECHO, "graveyard");
    });

    it("R547 the copied face's yellow glow answers for Echo in hand, and its Combo resolves for Echo's play", () => {
      const s = scenario({ p1: { hand: [RAPID, RAPID, RAPID, ECHO], library: [MENACE, MENACE, MENACE, MENACE] } });
      for (const card of s.hand("p1").filter((held) => held.defId === RAPID)) s.play(card);
      expect(ownView(s)?.conditionActive).toBe(true);
      s.play(ECHO);
      expect(s.lastEvents.filter((event) => event.type === "drawn")).toHaveLength(3);
    });
  });

  describe("radiant", () => {
    it("§6.2 Echo 1: the copied text resolves once more, asking a fresh pick for the repeat", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, { def: ECHO, radiant: true }, VANILLA] }, p2: { field: [MENACE], hand: [VANILLA] } });
      s.play(WILDFIRE, { targets: AT_P2 });
      s.play(ECHO, { targets: AT_P2 });
      expect(s.state.pending?.prompt).toMatch(/Echo/);
      s.answer([{ pick: "instance", instanceId: s.card(MENACE).id }]);
      expect(hits(s.events, "hero-p2")).toEqual([4, 4]);
      s.expectStats(MENACE, { health: 5 });
    });

    it("R399 a Radiant Echo after a played Echo copies the Spell that Echo copied, never Echo: Echo 1 repeats that text", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, ECHO, { def: ECHO, radiant: true }, VANILLA], mana: 9 }, p2: { hand: [VANILLA] } });
      const plain = s.hand("p1").find((card) => card.defId === ECHO && !card.radiant);
      const shining = s.hand("p1").find((card) => card.defId === ECHO && card.radiant);
      if (plain === undefined || shining === undefined) throw new Error("setup");
      s.play(WILDFIRE, { targets: AT_P2 }).play(plain, { targets: AT_P2 });
      expect(subsystems.copiedTextOf(s.state, s.card(shining))).toEqual({ defId: WILDFIRE, radiant: false });
      s.play(shining, { targets: AT_P2 }).answer(AT_P2);
      expect(hits(s.events, "hero-p2")).toEqual([4, 4, 4, 4]);
    });

    it("R386 its Echo is a numbered keyword Upgrade moves as an X: one step more is one more repeat", () => {
      const s = scenario({ p1: { hand: [WILDFIRE, { def: ECHO, radiant: true }, VANILLA] }, p2: { hand: [VANILLA] } });
      const echo = s.card(ECHO);
      const tuning = tuningOf(echo);
      tuning.x = addStep(tuning.x, "Echo", 1);
      s.play(WILDFIRE, { targets: AT_P2 });
      s.play(ECHO, { targets: AT_P2 }).answer(AT_P2).answer(AT_P2);
      expect(hits(s.events, "hero-p2")).toEqual([4, 4, 4, 4]);
    });

    it("R399 with nothing to copy it does nothing, and its Echo repeats nothing", () => {
      const s = scenario({ p1: { hand: [{ def: ECHO, radiant: true }, VANILLA] } });
      s.play(ECHO);
      expect(s.state.pending).toBeNull();
      expect(s.lastEvents.some((event) => event.type === "damage")).toBe(false);
      s.expectInZone(ECHO, "graveyard");
    });
  });
});
