// T-Glitch Glitch — SPEC §7, BUILD M9 Classic row C T-Glitch: "A hidden (0) Spell token in no pool, the
// Almanac or the Deck Builder (R662); after n System cards (C #18, C #25) have been played in the match,
// by either player, a card an effect generates from a random pool is Glitch instead n times in 10000
// (R661); it costs (0) whatever would change that and no ban refuses it, on its owner's turn only
// (R663); when played, one draw of the match rng picks one of four outcomes, none built yet (R664);
// radiant: draws 1 first".

import { describe, expect, it } from "vitest";
import {
  GLITCH_DEF_ID,
  GLITCH_ODDS_DENOMINATOR,
  SYSTEM_CARD_DEF_IDS,
  createRng,
  findDef,
  legalActions,
  query,
} from "@jackioh/engine";
import { glitchOutcome, rollGlitchOutcome } from "@jackioh/engine/effects";
import type { PlayerId } from "@jackioh/shared";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/t-glitch-glitch";

const GLITCH = GLITCH_DEF_ID;
const GLITCH_IN_THE_SYSTEM = "classic-018"; // (3) Spell: "Choose a number. Exile every card … that costs that much."
const LAG_IN_THE_SYSTEM = "classic-025"; // (0) Spell: "Exile every card … that costs (1) or less."
const CONJURE = "core-057"; // (2) Spell: "Add 3 random KY cards to your hand."
const COIN = "core-t-coin"; // (0) Spell token
const TAX = "classicplus-t-ai-07"; // (1) Spell token: "Your opponent's cards cost (1) more during their next turn."
const MENACE = "core-019"; // (3) Unit 9/9
const VANILLA = "core-008"; // (1) Unit 4/4
const COUNTERSPELL = "classic-017"; // (2) Trap: counters the opponent's Spell
const ECHO = "classic-057"; // (1) Spell: "This has the text of the last Spell either player played."
const TRANSMOGULATE = "core-083"; // (2) Spell: "Replace every card in your hand, deck, board, GY and exile with a random Legendary."
const SEVEN = "core-025"; // (4) Unit 7/7
const CONJURED = 3;
/** A cost no card here prints, so C #18 exiles nothing. */
const NOTHING_COSTS = "9";
const SURCHARGE = 4;

function playable(s: Scenario, player: PlayerId, defId: string): boolean {
  const ids = s.hand(player).filter((card) => card.defId === defId).map((card) => card.id);
  return legalActions(s.state, player).some((action) => action.type === "play" && ids.includes(action.instanceId));
}

describe("T-Glitch Glitch", () => {
  it("is a hidden (0) Spell token; both faces are always playable, and the Radiant face draws 1 first (R275)", () => {
    expect([def.cost, def.type, def.set, def.index, def.rarity]).toEqual([0, "Spell", "Classic", "T-Glitch", "Token"]);
    expect(def.token).toBe(true);
    expect(def.tags).toEqual(["Token"]);
    expect(def.hidden).toBe(true);
    expect(def.params).toBeUndefined();
    expect(base.staticFlags).toEqual({ alwaysPlayable: true });
    expect(radiant.staticFlags).toEqual({ alwaysPlayable: true });
    expect(SYSTEM_CARD_DEF_IDS).toEqual([GLITCH_IN_THE_SYSTEM, LAG_IN_THE_SYSTEM]);
  });

  describe("R661 how a game makes one", () => {
    it("R661 C #18 and C #25 count whoever plays them; no other card does", () => {
      const s = scenario({
        p1: { hand: [GLITCH_IN_THE_SYSTEM, CONJURE], library: [MENACE, MENACE], mana: 8 },
        p2: { hand: [LAG_IN_THE_SYSTEM], library: [MENACE, MENACE] },
      });
      expect(s.state.systemPlays).toBeUndefined();
      s.play(CONJURE);
      expect(s.state.systemPlays).toBeUndefined();
      s.play(GLITCH_IN_THE_SYSTEM, { modes: [NOTHING_COSTS] });
      expect(s.state.systemPlays).toBe(1);
      s.endTurn();
      s.play(LAG_IN_THE_SYSTEM);
      expect(s.state.systemPlays).toBe(2);
    });

    it("R661 a countered System card was never played and counts for nothing (R448)", () => {
      const s = scenario({
        p1: { hand: [LAG_IN_THE_SYSTEM, COIN], library: [MENACE] },
        p2: { backrow: [{ def: COUNTERSPELL, faceUp: false }], library: [MENACE] },
      });
      const lag = s.card(LAG_IN_THE_SYSTEM);
      s.play(lag);
      s.expectInZone(lag, "graveyard");
      expect(s.events.some((event) => event.type === "countered")).toBe(true);
      expect(s.state.systemPlays).toBeUndefined();
    });

    it("R661 C #57 Echo taking a System card's text is not a System card, and does not count", () => {
      const s = scenario({ p1: { hand: [GLITCH_IN_THE_SYSTEM, ECHO, COIN], library: [MENACE], mana: 8 } });
      s.play(GLITCH_IN_THE_SYSTEM, { modes: [NOTHING_COSTS] });
      expect(s.state.systemPlays).toBe(1);
      const echo = s.card(ECHO);
      s.play(echo, { modes: [NOTHING_COSTS] });
      s.expectInZone(echo, "graveyard");
      expect(s.state.systemPlays).toBe(1);
    });

    it("R661 Core #83's replacements in hand and deck may be Glitch; its board's never are", () => {
      const s = scenario({ seed: "glitch-transmogulate", p1: { hand: [TRANSMOGULATE, COIN], field: [VANILLA], library: [SEVEN] } });
      s.state.systemPlays = GLITCH_ODDS_DENOMINATOR;
      s.play(TRANSMOGULATE);
      expect(s.hand("p1").map((card) => card.defId)).toEqual([GLITCH]);
      expect(s.pile("p1", "library").map((card) => card.defId)).toEqual([GLITCH]);
      const unit = s.unit("p1", 1);
      expect(unit).not.toBeNull();
      expect(unit?.defId).not.toBe(GLITCH);
      expect(findDef(null, unit?.defId ?? "")?.rarity).toBe("Legendary");
    });

    it("R661 with no System card played, a pool's cards are drawn as before and none is Glitch", () => {
      const s = scenario({ seed: "glitch-none", p1: { hand: [CONJURE] } });
      const cursor = s.state.rngCursor;
      s.play(CONJURE);
      const added = s.hand("p1").map((card) => card.defId);
      expect(added).toHaveLength(CONJURED);
      expect(added).not.toContain(GLITCH);
      // One draw per card, as before R661: the Glitch check draws nothing while no System card was played.
      expect(s.state.rngCursor).toBe(cursor + CONJURED);
    });

    it("R661 once System cards have been played, each generated card may be Glitch, one more draw apiece", () => {
      // As many System plays as the odds' denominator: every check comes up Glitch.
      const s = scenario({ seed: "glitch-always", p1: { hand: [CONJURE] } });
      s.state.systemPlays = GLITCH_ODDS_DENOMINATOR;
      const cursor = s.state.rngCursor;
      s.play(CONJURE);
      expect(s.hand("p1").map((card) => card.defId)).toEqual([GLITCH, GLITCH, GLITCH]);
      expect(s.state.rngCursor).toBe(cursor + 2 * CONJURED);
    });
  });

  describe("R662 hidden", () => {
    it("R662 no pool holds it, not even C+ #23 Dropshipping's, which takes every other token", () => {
      expect(query({ withTokens: true }).some((card) => card.id === GLITCH)).toBe(false);
      expect(query({ token: true }).some((card) => card.id === GLITCH)).toBe(false);
      expect(query({ type: "Spell", tags: ["Token"] }).some((card) => card.id === GLITCH)).toBe(false);
      expect(query({ withTokens: true }).some((card) => card.id === COIN)).toBe(true);
    });
  });

  describe("R663 always playable on your turn", () => {
    it("R663 costs (0) under a price rule and a cost change that put a (0) Coin out of reach", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [{ def: GLITCH, costMod: SURCHARGE }, { def: COIN, costMod: SURCHARGE }], library: [SEVEN, SEVEN] },
        // The Coin keeps p2's turn from ending by itself once the Tax is played.
        p2: { hand: [TAX, COIN], library: [SEVEN, SEVEN] },
      });
      s.play(TAX).endTurn();
      expect(s.state.active).toBe("p1");
      // The Coin costs (0) + 4 + the tax's (1), more than p1's 4 mana.
      expect(playable(s, "p1", COIN)).toBe(false);
      expect(playable(s, "p1", GLITCH)).toBe(true);
      const mana = s.state.players.p1.mana.current;
      const glitch = s.card(GLITCH);
      s.play(GLITCH);
      s.expectInZone(glitch, "graveyard");
      s.expectMana("p1", mana);
    });

    it("R663 a copier that takes Glitch's text (C #57 Echo) runs its roll but keeps its own price (R545)", () => {
      const taxed = scenario({ p1: { hand: [GLITCH, { def: ECHO, costMod: SURCHARGE }, COIN], library: [SEVEN] } });
      taxed.play(GLITCH);
      // Echo has Glitch's text now, and its own (1) Cost with the change on it: more than 4 mana.
      expect(playable(taxed, "p1", ECHO)).toBe(false);

      const s = scenario({ seed: "glitch-echo", p1: { hand: [GLITCH, ECHO, COIN], library: [SEVEN] } });
      s.play(GLITCH);
      const mana = s.state.players.p1.mana.current;
      const cursor = s.state.rngCursor;
      s.play(ECHO);
      s.expectMana("p1", mana - 1);
      expect(s.state.rngCursor).toBe(cursor + 1);
    });

    it("R663 is not playable on the opponent's turn", () => {
      const s = scenario({ p2: { hand: [GLITCH] } });
      expect(s.state.active).toBe("p1");
      expect(playable(s, "p2", GLITCH)).toBe(false);
    });
  });

  describe("R664 when played", () => {
    it("R664 base: one draw of the match rng picks the outcome, and until the outcomes are built nothing else happens", () => {
      // The Coin keeps p1's turn from ending by itself once Glitch is played.
      const s = scenario({ seed: "glitch-base", p1: { hand: [GLITCH, COIN], library: [SEVEN, SEVEN] }, p2: { library: [SEVEN] } });
      const cursor = s.state.rngCursor;
      const outcome = rollGlitchOutcome(createRng(s.state.seed, cursor));
      expect(["reset", "swapSeats", "foreignBoards", "void"]).toContain(outcome);
      const glitch = s.card(GLITCH);
      s.play(GLITCH);
      expect(s.state.rngCursor).toBe(cursor + 1);
      s.expectInZone(glitch, "graveyard").expectHealth("p1", 30).expectHealth("p2", 30);
      expect(s.hand("p1").map((card) => card.defId)).toEqual([COIN]);
      expect(s.state.active).toBe("p1");
      expect(s.state.result).toBeNull();
      expect(base.cry?.({} as never).map((effect) => effect.kind)).toEqual([glitchOutcome().kind]);
    });

    it("R664 radiant: draws 1, then the same roll", () => {
      const s = scenario({
        seed: "glitch-radiant",
        p1: { hand: [{ def: GLITCH, radiant: true }, COIN], library: [SEVEN, MENACE] },
        p2: { library: [SEVEN] },
      });
      const cursor = s.state.rngCursor;
      s.play(GLITCH);
      expect(s.hand("p1").map((card) => card.defId)).toEqual([COIN, SEVEN]);
      expect(s.state.rngCursor).toBe(cursor + 1);
      expect(radiant.cry?.({} as never).map((effect) => effect.kind)).toEqual(["draw", glitchOutcome().kind]);
    });
  });
});
