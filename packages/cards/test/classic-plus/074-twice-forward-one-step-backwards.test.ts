// C+ #74 Twice Forward One Step Backwards — SPEC §8.7 row 74, BUILD M9 Classic+ row C+ 74: "A Field Trap
// (R425) set face-down with printed Brittle 2 and no count while unrevealed (R639); it counts the
// opponent's plays from then on (`memory.plays`; a cast counts, R70; a countered play doesn't) and on
// every second one, after that card resolves, the card, if it still exists (a Unit on the field, a Spell
// in the graveyard, a trap in the backrow), is fused into this (R77, R102): this is the kept instance and
// stays a Field Trap, the opponent's card ceases to exist; the first activation reveals it and starts
// its Brittle, then this gains +1 Brittle, even when there was no card left to fuse (revealing first);
// fused texts act for you where they can (an end-of-turn line or an aura on your side) and a fused Cry
// never runs; it turns face-up at its first activation (R33, R639), and until then the opponent's view
// shows neither the card nor its play count, while a card destroyed unrevealed reads public in its
// graveyard (R97); with no fuse it holds no Brittle and never crumbles; its memory and fused definition
// survive JSON and replay; Brittle, the every-2 step (never below 2) and Brittle gained read through
// `param()`; radiant Brittle 4, and a Radiant copy of the card is fused in, the original staying where
// it is".
//
// p1 sets the trap on its own turn (turn 9), then p2's turn begins and p2 plays. The machinery is proved
// through fixtures in `packages/engine/test/twiceForward.test.ts`.

import { activeBrittleCount, defOf, hashState, reduce, stepParam, subsystems, type GameState } from "@jackioh/engine";
import type { CardView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type PileSetup, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/074-twice-forward-one-step-backwards";

const FORWARD = "classicplus-074";
const REFUSAL = "classicplus-t-ai-09";
const VANILLA = "core-008"; // (1) Unit 4/4
const MENACE = "core-019"; // (3) Unit 9/9 Taunt; End of turn: heal this to full
const STOCKPILE = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
const RAPID = "core-010"; // (0) Spell
const TRUE_STRIKE = "core-044"; // (1) Spell: Pierce. Deal 4 damage. Exile this.
const BEAR = "core-060"; // (1) Trap
const SHREDDER = "core-013"; // (3) Unit: End of turn: deal 2 damage to each enemy Unit and the enemy hero.
const FELINORS = "core-012"; // (2) Unit: Cry: Summon a copy of this.
const TIGER_DOJO = "core-014"; // (4) Field Spell: Aura: your Units have +4 attack, Rush and First Strike.
const HIT_JOB = "core-016"; // (3) Spell: Destroy target Unit.
const HINDER = "core-021"; // (0) Spell, cast on draw
const TIMMY = "core-011"; // (1) Unit
const GUY_ATT = "classicplus-005"; // (2) Unit: Cry: Destroy every backrow card you control.

const deck = (n: number, card = TIMMY): PileSetup[] => Array.from({ length: n }, () => card);

/** p1 sets the trap (lane 2) on turn 9 and ends its turn; p2's turn 10 begins, p2 with mana to spare. */
function setThenTheirTurn(opts: { radiantFace?: boolean; p1?: SideSetup; p2?: SideSetup; tune?: (s: Scenario) => void } = {}): Scenario {
  const s = scenario({
    p1: { hand: [{ def: FORWARD, radiant: opts.radiantFace === true }, VANILLA], library: deck(10), ...opts.p1 },
    p2: { hand: [STOCKPILE, RAPID, VANILLA, VANILLA], library: deck(10), ...opts.p2 },
  });
  opts.tune?.(s);
  s.play(FORWARD, { zone: 2 }).endTurn();
  s.state.players.p2.mana.current = 10;
  return s;
}

function trapOf(s: Scenario): NonNullable<ReturnType<Scenario["backrow"]>> {
  const card = s.backrow("p1", 2);
  if (card === null) throw new Error("no trap in p1's backrow lane 2");
  return card;
}

function playsOf(s: Scenario): number {
  return subsystems.twiceForwardPlays(trapOf(s));
}

function fusedEvents(s: Scenario): number {
  return s.events.filter((event) => event.type === "fused").length;
}

describe("C+ #74 Twice Forward One Step Backwards", () => {
  it("is a (2) Mythic Field Trap printing Brittle 2 (Radiant 4), declaring every 2 (min 2) and Brittle gained 1", () => {
    expect(def.type).toBe("Field Trap");
    expect(def.cost).toBe(2);
    expect(def.rarity).toBe("Mythic");
    expect(def.base.keywords).toEqual([{ kind: "Brittle", n: 2 }]);
    expect(def.radiant.keywords).toEqual([{ kind: "Brittle", n: 4 }]);
    expect(def.params).toEqual([
      { key: "plays", base: 2, radiant: 2, better: "down", step: 1, min: 2 },
      { key: "brittleGain", base: 1, radiant: 1, better: "up", step: 1, min: 1 },
    ]);
    expect(base.triggers?.[0]?.on).toEqual(["cardPlayed", "cardResolved"]);
    expect(radiant.triggers?.[0]?.when).toBeTypeOf("function");
  });

  describe("base", () => {
    it("R639 R425 set face-down, it holds no Brittle; only its controller reads the back", () => {
      const s = setThenTheirTurn();
      const trap = trapOf(s);
      expect(trap.faceUp === true).toBe(false);
      expect(activeBrittleCount(trap)).toBeNull();
      expect(trap.brittle).toBeUndefined();
      expect((s.view("p1").you.backrow[1] as CardView | null)?.brittle).toBeUndefined();
      const theirs = JSON.stringify(s.view("p2"));
      expect(theirs).not.toContain(FORWARD);
      expect(theirs).not.toContain(trap.id);
    });

    it("R639 R97 destroyed while unrevealed, it is shown to the opponent: their view names it", () => {
      const s = scenario({
        p1: { hand: [FORWARD, GUY_ATT], backrow: [], library: deck(10), mana: 10 },
        p2: { hand: [VANILLA], library: deck(10) },
      });
      s.play(FORWARD, { zone: 2 });
      const trap = trapOf(s);
      expect(trap.faceUp === true).toBe(false);
      s.play(GUY_ATT, { zone: 1 });
      s.expectInZone(trap, "graveyard");
      const seen = JSON.stringify(s.view("p2"));
      expect(seen).toContain(trap.id);
      expect(seen).toContain(FORWARD);
    });

    it("R425 R99 their 1st play is counted and leaves it face-down and unfired", () => {
      const s = setThenTheirTurn().play(RAPID);
      expect(playsOf(s)).toBe(1);
      expect(trapOf(s).faceUp === true).toBe(false);
      expect(s.events.some((event) => event.type === "trapFired")).toBe(false);
    });

    it("R33 R639 until it activates the opponent reads neither the card nor its play count", () => {
      const s = setThenTheirTurn().play(RAPID);
      const trap = trapOf(s);
      const seen = s.view("p2").opponent.backrow[1];
      expect(seen).not.toBeNull();
      expect(JSON.stringify(seen)).not.toMatch(/brittle|plays|memory/);
      const theirs = JSON.stringify(s.view("p2"));
      expect(theirs).not.toContain(trap.id);
      expect(theirs).not.toContain(FORWARD);
      expect((s.view("p1").you.backrow[1] as CardView | null)?.brittle).toBeUndefined();
    });

    it("R425 R77 R639 the 2nd, a Unit, is fused into this after it resolves: still a Field Trap, the Unit gone, revealed with Brittle 2 + 1, face-up (R33)", () => {
      const s = setThenTheirTurn().play(RAPID);
      const unit = s.card(VANILLA);
      s.play(VANILLA, { zone: 1 });
      const trap = trapOf(s);
      expect(s.unit("p2", 1)).toBeNull();
      s.expectInZone(unit, "gone");
      expect(defOf(s.state, trap.defId).type).toBe("Field Trap");
      expect(trap.zone).toMatchObject({ z: "field", row: "backrow", lane: 2, player: "p1" });
      expect(activeBrittleCount(trap)).toBe(3);
      expect(trap.faceUp).toBe(true);
      s.expectEvents("cardResolved", "trapFired", "fused");
      expect(JSON.stringify(s.view("p2"))).toContain(trap.defId);
    });

    it("R425 a Spell is fused from its owner's graveyard", () => {
      const s = setThenTheirTurn().play(RAPID).play(STOCKPILE);
      expect(s.pile("p2", "graveyard").some((card) => card.defId === STOCKPILE)).toBe(false);
      expect(fusedEvents(s)).toBe(1);
      expect(activeBrittleCount(trapOf(s))).toBe(3);
    });

    it("R425 a trap is fused from its owner's backrow", () => {
      const s = setThenTheirTurn({ p2: { hand: [RAPID, BEAR, VANILLA] } }).play(RAPID).play(BEAR, { zone: 1 });
      expect(s.backrow("p2", 1)).toBeNull();
      expect(fusedEvents(s)).toBe(1);
    });

    it("R639 R589 R425 with nothing left to fuse (a Spell that exiles itself) it reveals and still gains +1 Brittle, unfired", () => {
      const s = setThenTheirTurn({ p2: { hand: [RAPID, TRUE_STRIKE, VANILLA] } }).play(RAPID);
      s.play(TRUE_STRIKE, { targets: [{ pick: "hero", player: "p1" }] });
      s.expectInZone(TRUE_STRIKE, "exile");
      expect(fusedEvents(s)).toBe(0);
      // No Brittle sits on an unrevealed card: the gain reveals it first, then lands on the started 2.
      expect(activeBrittleCount(trapOf(s))).toBe(3);
      expect(trapOf(s).faceUp).toBe(true);
      expect(JSON.stringify(s.view("p2"))).toContain(trapOf(s).id);
    });

    it("R70 a cast is a play: the cast-on-draw card their turn's draw casts is their 1st, so the next play is fused", () => {
      const s = setThenTheirTurn({ p2: { hand: [STOCKPILE, VANILLA], library: [{ def: HINDER, radiant: true }, ...deck(6)] } });
      expect(playsOf(s)).toBe(1);
      expect(s.pile("p2", "graveyard").map((card) => card.defId)).toEqual([HINDER]);
      s.play(STOCKPILE);
      expect(playsOf(s)).toBe(2);
      expect(s.events.filter((event) => event.type === "fused").map((event) => event.defId)).toEqual([trapOf(s).defId]);
      expect(defOf(s.state, trapOf(s).defId).ingredients?.map((part) => part.defId)).toEqual([STOCKPILE, FORWARD]);
    });

    it("R70 R425 a card their play casts is the later play: the Hinder Stockpile draws is their 2nd, fused once it resolves", () => {
      const s = setThenTheirTurn({ p2: { hand: [STOCKPILE, VANILLA], library: [TIMMY, { def: HINDER, radiant: true }, ...deck(6)] } });
      expect(playsOf(s)).toBe(0);
      s.play(STOCKPILE);
      const played = s.events.flatMap((event) => (event.type === "cardPlayed" && event.player === "p2" ? [event.defId] : []));
      expect(played).toEqual([STOCKPILE, HINDER]);
      expect(playsOf(s)).toBe(2);
      expect(defOf(s.state, trapOf(s).defId).ingredients?.map((part) => part.defId)).toEqual([HINDER, FORWARD]);
      expect(s.pile("p2", "graveyard").map((card) => card.defId)).toEqual([STOCKPILE]);
    });

    it("R448 a countered play is never played and doesn't count", () => {
      const s = setThenTheirTurn({
        p1: { field: [MENACE], backrow: [{ def: REFUSAL, faceUp: false, lane: 4 }] },
        p2: { hand: [HIT_JOB, RAPID, VANILLA] },
      });
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.unit("p1", 1)?.id ?? "" }] });
      expect(s.events.some((event) => event.type === "countered")).toBe(true);
      expect(playsOf(s)).toBe(0);
      s.play(RAPID);
      expect(playsOf(s)).toBe(1);
    });

    it("R425 its own controller's plays never count", () => {
      const s = setThenTheirTurn();
      s.endTurn();
      expect(s.state.active).toBe("p1");
      s.play(VANILLA, { zone: 1 });
      expect(playsOf(s)).toBe(0);
    });

    it("R102 a fused end-of-turn line runs on your side: a fused Shredder hits their hero and Units at your end of turn", () => {
      const s = setThenTheirTurn({ p2: { hand: [RAPID, SHREDDER, VANILLA] } }).play(VANILLA, { zone: 1 }).play(SHREDDER, { zone: 2 });
      expect(s.unit("p2", 2)).toBeNull();
      s.endTurn();
      const health = s.state.players.p2.hero.health;
      s.endTurn();
      expect(s.state.players.p2.hero.health).toBe(health - 2);
      s.expectStats(s.unit("p2", 1) ?? "", { health: 2 });
    });

    it("R102 a fused aura covers your side: a fused Tiger Dojo gives your Units +4 attack", () => {
      const s = setThenTheirTurn({ p1: { field: [TIMMY] }, p2: { hand: [RAPID, TIGER_DOJO, VANILLA] } });
      const before = s.stats(s.unit("p1", 1) ?? "").attack;
      s.play(RAPID).play(TIGER_DOJO, { zone: 1 });
      expect(s.backrow("p2", 1)).toBeNull();
      expect(s.stats(s.unit("p1", 1) ?? "").attack).toBe(before + 4);
    });

    it("R1 R102 a fused Cry never runs: a fused Duplicating Felinors summons nothing on your side", () => {
      const s = setThenTheirTurn({ p2: { hand: [RAPID, FELINORS, VANILLA] } }).play(RAPID).play(FELINORS, { zone: 1 });
      expect(fusedEvents(s)).toBe(1);
      expect(s.unit("p1", 1)).toBeNull();
      // Its own Cry ran as it was played: the copy stays p2's.
      expect(s.unit("p2", 2)?.defId).toBe(FELINORS);
    });

    it("R425 every second play fuses again, +1 Brittle each time", () => {
      const s = setThenTheirTurn({ p2: { hand: [RAPID, RAPID, RAPID, RAPID, VANILLA] } });
      for (let i = 0; i < 4; i += 1) s.play(RAPID);
      expect(playsOf(s)).toBe(4);
      expect(fusedEvents(s)).toBe(2);
      expect(activeBrittleCount(trapOf(s))).toBe(4);
    });

    it("R639 with no fuse it holds no Brittle and never crumbles: turns pass with no count and no crumble", () => {
      const s = setThenTheirTurn({ p1: { library: deck(12) }, p2: { library: deck(12) } });
      const trap = trapOf(s);
      s.endTurn();
      expect(s.state.turn).toBe(11);
      expect(activeBrittleCount(trapOf(s))).toBeNull();
      s.endTurn().endTurn().endTurn().endTurn();
      expect(s.state.turn).toBe(15);
      expect(activeBrittleCount(trapOf(s))).toBeNull();
      s.endTurn().endTurn();
      expect(s.state.turn).toBe(17);
      expect(s.backrow("p1", 2)?.id).toBe(trap.id);
      expect(s.events.some((event) => event.type === "crumbled")).toBe(false);
    });

    it("R179 its count and fused definition survive JSON, and the round trip plays on exactly as the live game", () => {
      const s = setThenTheirTurn({ p2: { hand: [RAPID, VANILLA, RAPID, VANILLA] } }).play(RAPID).play(VANILLA, { zone: 1 });
      const revived = JSON.parse(JSON.stringify(s.state)) as GameState;
      expect(revived).toEqual(s.state);
      expect(revived.transientDefs[trapOf(s).defId]).toBeDefined();
      const next = s.hand("p2").find((card) => card.defId === RAPID);
      const resumed = reduce(revived, { type: "play", playerId: "p2", instanceId: next?.id ?? "", nonce: "forward-json" });
      expect(resumed.error).toBeUndefined();
      s.play(next ?? RAPID);
      expect(hashState(resumed.state)).toBe(hashState(s.state));
      expect(playsOf(s)).toBe(3);
    });

    it("R386 'every N' reads through param(): never below 2, a Degrade makes it 3, and Brittle gained moves with it", () => {
      const floor = setThenTheirTurn({ tune: (s) => stepParam(s.card(FORWARD), "plays", -1) }).play(RAPID).play(STOCKPILE);
      expect(fusedEvents(floor)).toBe(1);

      const slow = setThenTheirTurn({
        p2: { hand: [RAPID, RAPID, RAPID, VANILLA] },
        tune: (s) => {
          stepParam(s.card(FORWARD), "plays", 1);
          stepParam(s.card(FORWARD), "brittleGain", 1);
        },
      });
      slow.play(RAPID).play(RAPID);
      expect(fusedEvents(slow)).toBe(0);
      slow.play(RAPID);
      expect(fusedEvents(slow)).toBe(1);
      expect(activeBrittleCount(trapOf(slow))).toBe(4);
    });

    it("R639 R386 its Brittle is its numbered keyword: an Upgrade's X change starts it at 3 on its first fuse, +1 gained", () => {
      const s = setThenTheirTurn({ tune: (s0) => {
        const card = s0.card(FORWARD);
        card.tuning = { ...(card.tuning ?? {}), x: { ...(card.tuning?.x ?? {}), Brittle: 1 } };
      } });
      // Face-down it holds nothing, tuned or not; the first fuse reveals it and starts the tuned 3.
      expect(activeBrittleCount(trapOf(s))).toBeNull();
      s.play(RAPID).play(VANILLA, { zone: 1 });
      expect(activeBrittleCount(trapOf(s))).toBe(4);
    });
  });

  describe("radiant", () => {
    it("R639 Brittle 4, set face-down with no count", () => {
      const s = setThenTheirTurn({ radiantFace: true });
      expect(activeBrittleCount(trapOf(s))).toBeNull();
      expect(trapOf(s).faceUp === true).toBe(false);
    });

    it("R469 a Radiant copy of the 2nd play is fused in, the original staying where it is", () => {
      const s = setThenTheirTurn({ radiantFace: true }).play(RAPID).play(VANILLA, { zone: 1 });
      expect(s.unit("p2", 1)?.defId).toBe(VANILLA);
      const trap = trapOf(s);
      expect(trap.defId).not.toBe(FORWARD);
      expect(defOf(s.state, trap.defId).type).toBe("Field Trap");
      expect(defOf(s.state, trap.defId).ingredients?.find((part) => part.defId === VANILLA)).toEqual({ defId: VANILLA, radiant: true });
      expect(activeBrittleCount(trap)).toBe(5);
      expect(trap.faceUp).toBe(true);
    });

    it("R425 a card that has left still has its copy fused: the Radiant face fuses on every second play", () => {
      const s = setThenTheirTurn({ radiantFace: true, p2: { hand: [RAPID, TRUE_STRIKE, VANILLA] } }).play(RAPID);
      s.play(TRUE_STRIKE, { targets: [{ pick: "hero", player: "p1" }] });
      s.expectInZone(TRUE_STRIKE, "exile");
      expect(fusedEvents(s)).toBe(1);
      expect(trapOf(s).faceUp).toBe(true);
    });
  });
});
