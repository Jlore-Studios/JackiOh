// C #52 Final Gambit — SPEC §8.6 row 52, BUILD M9 Classic row C 52: "Face-down (R33); fires at §4.4
// step 4a when one hit would leave your hero at 0 or less, judged after Armor, multipliers and caps,
// this hit alone (R44's reading); fatigue counts, while losing health (R18) and set health (C #29)
// never open it; a hit that isn't lethal leaves it set; the hit is re-aimed at the enemy hero as a new
// instance from the same source (`redirected`), through their Armor, multipliers and caps; then heal
// your hero 10 and draw 3; a redirected hit that kills the opponent ends the game at the state check,
// a draw if both heroes are at 0 (§2.5); radiant: heal 20 and draw your whole deck (R58), most of it
// burning (R317); its tuned numbers (heal, draw) read through `param()` (R386)".
//
// The hits come from Core cards with their own tests: attacks by Mr. Vanilla (4/4), Pointmaster (7/1)
// and Midrange Menace (9/9), Lunar Eclipse's 3, and fatigue on an empty deck. Going Long (Armor 2),
// C #75 Argusland (halved) and Anti-oneshot Armor (a cap of 5) stand on either hero's side. Blood
// Ridden Glowy Jelly Bean loses health on draw (R18), C #29 Book of Vital Kill sets a hero's health to
// 13, and Hinder, drawn by the follow-up, asks its caster to discard (R158) — the pause the round trip
// is taken across.
//
// "A draw if both heroes are at 0" (§2.5) needs a second hit inside the same effect, after the trap is
// spent: Prem Panther's "draw 2" into an empty deck, whose first fatigue the trap re-aims at a 1-health
// opponent and whose second then lands on you.

import { reduce, stepParam, type GameState } from "@jackioh/engine";
import type { Action, GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/052-final-gambit";

const GAMBIT = "classic-052";
const ARGUSLAND = "classic-075";
const VITAL_KILL = "classic-029"; // C #29 Book of Vital Kill: set a hero's health to 13.
const VANILLA = "core-008"; // 4/4
const POINTMASTER = "core-020"; // 7/1 First Strike
const MENACE = "core-019"; // 9/9 Taunt
const PANTHER = "core-032"; // 5/4 Rush: after this attacks and survives, draw 2 for each Unit that attack destroyed.
const GARY = "core-004"; // 1/1
const LUNAR = "core-035"; // (1) Spell: deal 3 damage to a target.
const STOCKPILE = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
const BLOOD_BEAN = "core-027"; // Cast on draw: make a random hand card Radiant. Lose 5 health.
const HINDER = "core-021"; // Cast on draw: their next refresh −1. Discard 1 (a hand prompt, R158).
const GOING_LONG = "core-084"; // Field Spell: your hero has Armor 2.
const ANTI_ONESHOT = "core-073"; // Field Spell: your hero can't take more than 5 damage at once.
const FILLER = "core-005";
const DECK = [FILLER, FILLER, FILLER, FILLER, FILLER] as const;
const FORWARD = "classicplus-074"; // (2) Field Trap: every 2 cards the opponent plays, the last is fused into this.
const RAPID = "core-010"; // (0) Spell: Combo 3: draw 3.
const TRUE_STRIKE = "core-044"; // (1) Spell: Pierce. Deal 4 damage. Exile this.

type Gambit = string | { def: string; radiant?: boolean };

function heroHits(s: Scenario, player: PlayerId): number[] {
  return s.events.flatMap((event: GameEvent) =>
    event.type === "damage" && event.targetId === `hero-${player}` ? [event.amount] : [],
  );
}

function fired(s: Scenario): GameEvent[] {
  return s.events.filter((event) => event.type === "trapFired");
}

/**
 * p2 attacks p1's hero with `attacker`. p1 has `health`, a Final Gambit set face-down beside the rest
 * of its backrow, a card in hand and a deck to draw from; p2 has whatever `p2` adds.
 */
function lethalAttack(opts: {
  attacker: string;
  health: number;
  gambit?: Gambit;
  p1Backrow?: readonly Gambit[];
  p1?: Partial<SideSetup>;
  p2?: Partial<SideSetup>;
}): Scenario {
  const s = scenario({
    p1: {
      hand: [FILLER],
      backrow: [{ ...asEntry(opts.gambit ?? GAMBIT), faceUp: false }, ...(opts.p1Backrow ?? [])],
      library: DECK,
      health: opts.health,
      ...opts.p1,
    },
    p2: { hand: [FILLER], field: [opts.attacker], library: DECK, ...opts.p2 },
    active: "p2",
  });
  s.attack(opts.attacker, "hero");
  return s;
}

function asEntry(entry: Gambit): { def: string; radiant?: boolean } {
  return typeof entry === "string" ? { def: entry } : entry;
}

describe("C #52 Final Gambit", () => {
  it("declares one replacement at the lethal-hit window, re-aiming the hit at the enemy hero, then its follow-up", () => {
    expect(def.id).toBe(GAMBIT);
    expect(def.type).toBe("Trap");
    for (const script of [base, radiant]) {
      expect(script.replacements).toEqual([
        { id: "final-gambit", on: "lethalHit", instead: { redirect: "enemyHero" }, then: "afterRedirect" },
      ]);
      expect(script.resume?.afterRedirect).toBeDefined();
    }
  });

  describe("base", () => {
    it("R33 it is set face-down: the opponent's view shows its back and never names it", () => {
      const s = scenario({ p1: { hand: [GAMBIT, FILLER] }, p2: { hand: [FILLER] } });
      s.play(GAMBIT);
      const card = s.card(GAMBIT);
      expect(card.faceUp).not.toBe(true);
      const theirs = s.view("p2");
      expect(theirs.opponent.backrow[0]).toMatchObject({ faceDown: true });
      expect(JSON.stringify(theirs)).not.toContain(GAMBIT);
      expect(JSON.stringify(s.view("p1"))).toContain(GAMBIT);
    });

    it("a hit that is not lethal leaves it set, face-down", () => {
      const s = lethalAttack({ attacker: VANILLA, health: 5 });
      expect(fired(s)).toEqual([]);
      s.expectHealth("p1", 1).expectInZone(GAMBIT, "field");
      expect(s.card(GAMBIT).faceUp).not.toBe(true);
    });

    it("a lethal hit fires it on the opponent's turn: re-aimed at the enemy hero as a hit from the same source, then heal 10 and draw 3", () => {
      const s = lethalAttack({ attacker: VANILLA, health: 4 });
      const gambit = s.card(GAMBIT);
      const vanilla = s.card(VANILLA);

      s.expectEvents("attackDeclared", "trapFired", "redirected", "damage", "healed");
      expect(s.events.find((event) => event.type === "redirected")).toEqual({
        type: "redirected",
        what: "damage",
        fromId: "hero-p1",
        toId: "hero-p2",
        byInstanceId: gambit.id,
      });
      const hit = s.events.find((event) => event.type === "damage");
      expect(hit).toMatchObject({ sourceId: vanilla.id, targetId: "hero-p2", amount: 4 });
      expect(heroHits(s, "p1")).toEqual([]);
      s.expectHealth("p2", 26).expectHealth("p1", 14);
      expect(s.hand("p1")).toHaveLength(4);
      expect(s.pile("p1", "library")).toHaveLength(2);
      s.expectInZone(gambit, "graveyard");
    });

    it("a hit that leaves the hero at exactly 0 is lethal", () => {
      const s = lethalAttack({ attacker: POINTMASTER, health: 7 });
      expect(fired(s)).toHaveLength(1);
      s.expectHealth("p2", 23).expectHealth("p1", 17);
    });

    it("R44 lethal is this hit alone: a 4 at 5 health leaves it set, and the 3 that follows at 1 fires it", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [{ def: GAMBIT, faceUp: false }], library: DECK, health: 5 },
        p2: { hand: [LUNAR, FILLER], field: [VANILLA], library: DECK },
        active: "p2",
      });
      s.attack(VANILLA, "hero");
      expect(fired(s)).toEqual([]);
      s.expectHealth("p1", 1);

      s.play(LUNAR, { targets: [{ pick: "hero", player: "p1" }] });
      expect(fired(s)).toHaveLength(1);
      s.expectHealth("p2", 27).expectHealth("p1", 11);
    });

    it("§4.4 it is judged after Armor: Going Long's 2 off a 7 is a 5 — lethal at 5, not at 6", () => {
      const at5 = lethalAttack({ attacker: POINTMASTER, health: 5, p1Backrow: [GOING_LONG] });
      expect(fired(at5)).toHaveLength(1);
      const at6 = lethalAttack({ attacker: POINTMASTER, health: 6, p1Backrow: [GOING_LONG] });
      expect(fired(at6)).toEqual([]);
      at6.expectHealth("p1", 1);
    });

    it("§4.4 it is judged after the hero damage multipliers: Argusland halves a 7 to 4 — lethal at 4, not at 5", () => {
      const at4 = lethalAttack({ attacker: POINTMASTER, health: 4, p1Backrow: [ARGUSLAND] });
      expect(fired(at4)).toHaveLength(1);
      const at5 = lethalAttack({ attacker: POINTMASTER, health: 5, p1Backrow: [ARGUSLAND] });
      expect(fired(at5)).toEqual([]);
      at5.expectHealth("p1", 1);
    });

    it("§4.4 it is judged after the hit caps: Anti-oneshot Armor caps a 9 at 5 — lethal at 5, not at 6", () => {
      const at5 = lethalAttack({ attacker: MENACE, health: 5, p1Backrow: [ANTI_ONESHOT] });
      expect(fired(at5)).toHaveLength(1);
      const at6 = lethalAttack({ attacker: MENACE, health: 6, p1Backrow: [ANTI_ONESHOT] });
      expect(fired(at6)).toEqual([]);
      at6.expectHealth("p1", 1);
    });

    it("§6.3 Redirect the re-aimed hit meets the enemy hero's own Armor and multipliers: 7 − 2 = 5, halved to 3", () => {
      const s = lethalAttack({ attacker: POINTMASTER, health: 4, p2: { backrow: [GOING_LONG, ARGUSLAND] } });
      expect(heroHits(s, "p2")).toEqual([3]);
      s.expectHealth("p2", 27);
    });

    it("§6.3 Redirect ... and the enemy hero's caps: Anti-oneshot Armor caps the re-aimed 9 at 5", () => {
      const s = lethalAttack({ attacker: MENACE, health: 4, p2: { backrow: [ANTI_ONESHOT] } });
      expect(heroHits(s, "p2")).toEqual([5]);
      s.expectHealth("p2", 25);
    });

    it("R125 fatigue is damage and counts, and it fires on its controller's own turn", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [{ def: GAMBIT, faceUp: false }], health: 1 },
        p2: { hand: [FILLER], library: DECK },
      });
      s.startTurn();
      expect(fired(s)).toHaveLength(1);
      // The 1st fatigue (1) went to p2's hero; then heal 10 and draw 3 into the empty deck: fatigue 2, 3, 4.
      expect(heroHits(s, "p2")).toEqual([1]);
      expect(heroHits(s, "p1")).toEqual([2, 3, 4]);
      s.expectHealth("p2", 29).expectHealth("p1", 2);
    });

    it("R18 losing health never opens it: the Jelly Bean's 5 at 5 health ends the game with the trap still set", () => {
      const s = scenario({
        p1: {
          hand: [STOCKPILE, FILLER],
          backrow: [{ def: GAMBIT, faceUp: false }],
          library: [BLOOD_BEAN, FILLER, FILLER],
          health: 5,
        },
        p2: { hand: [FILLER] },
      });
      s.play(STOCKPILE);
      expect(fired(s)).toEqual([]);
      expect(s.state.result).toMatchObject({ winner: "p2" });
      s.expectInZone(GAMBIT, "field");
    });

    it("set health never opens it: C #29 sets your hero to 13 and the trap stays set", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [{ def: GAMBIT, faceUp: false }], library: DECK },
        p2: { hand: [VITAL_KILL, FILLER] },
        active: "p2",
      });
      s.play(VITAL_KILL, { targets: [{ pick: "hero", player: "p1" }] });
      expect(fired(s)).toEqual([]);
      s.expectHealth("p1", 13).expectInZone(GAMBIT, "field");
    });

    it("a hit on a Unit never opens it", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [VANILLA], backrow: [{ def: GAMBIT, faceUp: false }], library: DECK, health: 1 },
        p2: { hand: [FILLER], field: [MENACE] },
        active: "p2",
      });
      const vanilla = s.unit("p1", 1);
      if (vanilla === null) throw new Error("Mr. Vanilla should be on the board");
      s.attack(MENACE, vanilla);
      expect(fired(s)).toEqual([]);
      s.expectInZone(vanilla, "graveyard").expectInZone(GAMBIT, "field");
    });

    it("§2.5 a re-aimed hit that kills the opponent ends the game at the state check", () => {
      const s = lethalAttack({ attacker: VANILLA, health: 4, p2: { health: 4 } });
      expect(fired(s)).toHaveLength(1);
      expect(s.state.result).toMatchObject({ winner: "p1" });
      s.expectHealth("p2", 0);
    });

    it("§2.5 a draw if both heroes are at 0: the re-aimed fatigue kills the opponent and the next one in the same draw kills you", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [PANTHER], backrow: [{ def: GAMBIT, faceUp: false }], health: 1 },
        p2: { hand: [FILLER], field: [GARY], health: 1 },
      });
      s.attack(PANTHER, s.card(GARY));
      expect(fired(s)).toHaveLength(1);
      expect(heroHits(s, "p2")).toEqual([1]);
      expect(s.state.result).toMatchObject({ winner: "draw" });
      s.expectHealth("p2", 0);
      expect(s.state.players.p1.hero.health).toBeLessThanOrEqual(0);
    });

    it("R216 the game ends at that state check, so nothing after it happens: the follow-up neither heals nor draws", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [{ def: GAMBIT, faceUp: false }], library: DECK, health: 1 },
        p2: { hand: [LUNAR, FILLER], health: 3 },
        active: "p2",
      });
      s.play(LUNAR, { targets: [{ pick: "hero", player: "p1" }] });
      expect(s.state.result).toMatchObject({ winner: "p1" });
      expect(s.events.at(-1)?.type).toBe("gameOver");
      s.expectHealth("p1", 1);
      expect(s.hand("p1")).toHaveLength(1);
    });

    it("R216 fused into a Field Trap that stays (C+ #74), it re-aims its own fatigue only until the enemy hero falls, and the follow-ups owed after that do nothing", () => {
      const s = scenario({
        p1: { hand: [FORWARD, FILLER], health: 4 },
        p2: { hand: [RAPID, GAMBIT, TRUE_STRIKE], library: DECK, health: 40 },
      });
      s.play(FORWARD, { zone: 2 }).endTurn();
      s.state.players.p2.mana.current = 10;
      // p2's second play is fused into p1's Field Trap, which keeps Final Gambit's text and stays.
      s.play(RAPID).play(GAMBIT);
      expect(s.backrow("p1", 2)?.defId).toContain(GAMBIT);
      s.state.players.p1.fatigueCount = 20;
      // 4 at 4 health is lethal: re-aimed (4), then heal 10 and draw 3 from an empty deck. Each
      // fatigue, 21, 22 and 23, is lethal at 14 and re-aimed too; p2 falls at the second, so the
      // three follow-ups owed by then neither heal nor draw, and the drain ends at the state check.
      s.play(TRUE_STRIKE, { targets: [{ pick: "hero", player: "p1" }] });
      expect(s.events.filter((event) => event.type === "redirected")).toHaveLength(4);
      expect(heroHits(s, "p2")).toEqual([4, 21, 22, 23]);
      expect(s.state.result).toMatchObject({ winner: "p1" });
      s.expectHealth("p1", 14);
    });

    it("a second Final Gambit finds no lethal hit once the first has re-aimed it, and stays set", () => {
      const s = lethalAttack({ attacker: VANILLA, health: 4, p1Backrow: [{ def: GAMBIT }] });
      expect(fired(s)).toHaveLength(1);
      const [first, second] = [s.backrow("p1", 1), s.backrow("p1", 2)];
      expect(first).toBeNull();
      expect(second?.defId).toBe(GAMBIT);
      expect(second?.faceUp).not.toBe(true);
      s.expectHealth("p1", 14);
    });

    it("the hand cap applies to its draws: the overflow burns (R317)", () => {
      const s = lethalAttack({
        attacker: VANILLA,
        health: 4,
        p1: { hand: [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER] },
      });
      expect(s.hand("p1")).toHaveLength(10);
      expect(s.events.filter((event) => event.type === "burned")).toHaveLength(2);
    });

    it("a prompt its draws open pauses the follow-up, and a round-tripped state answers to the same game", () => {
      const s = lethalAttack({ attacker: VANILLA, health: 4, p1: { library: [HINDER, FILLER, FILLER, FILLER] } });
      const pending = s.state.pending;
      expect(pending).not.toBeNull();
      expect(pending?.playerId).toBe("p1");
      s.expectHealth("p1", 14);
      expect(s.view("p2").pending).toEqual({ forYou: false, pendingFor: "p1" });

      const thawed = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(thawed).toEqual(s.state);
      const pick = s.hand("p1")[0];
      if (pick === undefined) throw new Error("p1 should hold a card to discard");
      const answer = {
        type: "answer",
        choiceId: pending?.id ?? "",
        selection: [{ pick: "instance", instanceId: pick.id }],
        playerId: "p1",
        nonce: "gambit-roundtrip",
      } as Action;
      const live = reduce(s.state, answer);
      const frozen = reduce(thawed, answer);
      expect(live.error).toBeUndefined();
      expect(frozen.state).toEqual(live.state);
      expect(frozen.events).toEqual(live.events);
      // The rest of the follow-up ran: Hinder's draw-again and the two draws after it.
      expect(live.state.players.p1.library).toHaveLength(0);
    });

    it("R386 an Upgrade heals 12 and draws 4", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [{ def: GAMBIT, faceUp: false }], library: DECK, health: 4 },
        p2: { hand: [FILLER], field: [VANILLA] },
        active: "p2",
      });
      const gambit = s.card(GAMBIT);
      stepParam(gambit, "heal", 1);
      stepParam(gambit, "draw", 1);
      s.attack(VANILLA, "hero");
      s.expectHealth("p1", 16);
      expect(s.hand("p1")).toHaveLength(5);
    });

    it("R386 a Degrade heals 8 and draws 2", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [{ def: GAMBIT, faceUp: false }], library: DECK, health: 4 },
        p2: { hand: [FILLER], field: [VANILLA] },
        active: "p2",
      });
      const gambit = s.card(GAMBIT);
      stepParam(gambit, "heal", -1);
      stepParam(gambit, "draw", -1);
      s.attack(VANILLA, "hero");
      s.expectHealth("p1", 12);
      expect(s.hand("p1")).toHaveLength(3);
    });
  });

  describe("radiant", () => {
    it("R33 it is set face-down like the base face", () => {
      const s = scenario({ p1: { hand: [{ def: GAMBIT, radiant: true }, FILLER] }, p2: { hand: [FILLER] } });
      s.play(GAMBIT);
      expect(JSON.stringify(s.view("p2"))).not.toContain(GAMBIT);
    });

    it("re-aims a lethal hit at the enemy hero, then heals 20", () => {
      const s = lethalAttack({ attacker: VANILLA, health: 4, gambit: { def: GAMBIT, radiant: true } });
      expect(fired(s)).toHaveLength(1);
      s.expectHealth("p2", 26).expectHealth("p1", 24);
    });

    it("R58 draws your deck — its size as the step begins — and R317 most of it burns at the hand cap", () => {
      const library = Array.from({ length: 15 }, () => FILLER);
      const s = lethalAttack({
        attacker: VANILLA,
        health: 4,
        gambit: { def: GAMBIT, radiant: true },
        p1: { hand: [FILLER, FILLER], library },
      });
      expect(s.pile("p1", "library")).toHaveLength(0);
      expect(s.hand("p1")).toHaveLength(10);
      expect(s.events.filter((event) => event.type === "burned")).toHaveLength(7);
      // Exactly the deck: no draw past its end, so no fatigue.
      expect(heroHits(s, "p1")).toEqual([]);
      s.expectHealth("p1", 24);
    });

    it("R58 an empty deck draws nothing, so no fatigue", () => {
      const s = lethalAttack({ attacker: VANILLA, health: 4, gambit: { def: GAMBIT, radiant: true }, p1: { library: [] } });
      expect(heroHits(s, "p1")).toEqual([]);
      s.expectHealth("p1", 24);
    });

    it("§4.4 judged after Armor, multipliers and caps, like the base face", () => {
      const s = lethalAttack({
        attacker: MENACE,
        health: 6,
        gambit: { def: GAMBIT, radiant: true },
        p1Backrow: [ANTI_ONESHOT],
      });
      expect(fired(s)).toEqual([]);
      s.expectHealth("p1", 1);
    });

    it("R18 losing health never opens it", () => {
      const s = scenario({
        p1: {
          hand: [STOCKPILE, FILLER],
          backrow: [{ def: GAMBIT, radiant: true, faceUp: false }],
          library: [BLOOD_BEAN, FILLER, FILLER],
          health: 5,
        },
        p2: { hand: [FILLER] },
      });
      s.play(STOCKPILE);
      expect(fired(s)).toEqual([]);
      expect(s.state.result).toMatchObject({ winner: "p2" });
    });

    it("R386 a Degrade heals 18", () => {
      const s = scenario({
        p1: { hand: [FILLER], backrow: [{ def: GAMBIT, radiant: true, faceUp: false }], library: DECK, health: 4 },
        p2: { hand: [FILLER], field: [VANILLA] },
        active: "p2",
      });
      stepParam(s.card(GAMBIT), "heal", -1);
      s.attack(VANILLA, "hero");
      s.expectHealth("p1", 22);
    });
  });
});
