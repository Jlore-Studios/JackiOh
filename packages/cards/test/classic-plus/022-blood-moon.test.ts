// C+ #22 Blood Moon — SPEC §8.7 row 22, BUILD M9 Classic+ row C+ 22: "Face-down Trap in the "would be
// healed" replacement: it fires when an enemy (the other player's hero or one of their Units) would be
// healed, by a heal, Lifesteal, "heal up to" or "heal to full"; the heal that sets it off is converted
// too (R413), and every heal of X on an enemy for the rest of the turn becomes X Pierce damage from
// Blood Moon (Armor skipped, Divine Shield and caps still apply, no Spell Damage), the modifier ending
// at cleanup while the trap lies in the graveyard; Set health is no heal; a friendly heal never fires
// it and the opponent learns nothing of it until it fires (R33, R97); radiant its face is a Field Trap,
// in hand too (pools and filters read Field Trap), it stays face-up once fired and converts every
// enemy heal for as long as it is on the field".

import { applyEffects, cardTypeOf, createRng, makeContext } from "@jackioh/engine";
import { setHealth } from "@jackioh/engine/effects";
import type { GameEvent, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario, type SideSetup } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/022-blood-moon";

const MOON = "classicplus-022";
const FIG = "core-047"; // Heal a target 20.
const RENO = "core-053"; // Cry: heal your hero up to 30.
const MENACE = "core-019"; // 9/9 Taunt; end of turn: heal this to full.
const JILLIAX = "core-056"; // 3/2 Rush, Taunt, Lifesteal, Divine Shield
const ANTI_ONESHOT = "core-073"; // the hero takes at most 5 at once
const SOLARIUS = "classicplus-038"; // Spell Damage +2
const FILLER = "core-005";
const DECK = [FILLER, FILLER, FILLER, FILLER, FILLER, FILLER];

const enemyHero: Selection[] = [{ pick: "hero", player: "p2" }];

/** p1 holds Blood Moon face-down in backrow lane 1; p2 is the active enemy who heals. */
function moonUp(radiantFace = false, p2: SideSetup = {}, p1: SideSetup = {}): Scenario {
  return scenario({
    active: "p2",
    p1: { hand: [FILLER], library: DECK, ...p1, backrow: [{ def: MOON, lane: 1, faceUp: false, radiant: radiantFace }, ...(p1.backrow ?? [])] },
    p2: { hand: [FILLER], library: DECK, ...p2 },
  });
}

function fired(s: Scenario): boolean {
  return s.events.some((event) => event.type === "trapFired" && event.defId === MOON);
}

function moonHits(s: Scenario): Extract<GameEvent, { type: "damage" }>[] {
  const moonIds = new Set(s.events.flatMap((event) => (event.type === "trapFired" && event.defId === MOON ? [event.instanceId] : [])));
  return s.events.flatMap((event) => (event.type === "damage" && event.sourceId !== null && moonIds.has(event.sourceId) ? [event] : []));
}

describe("C+ #22 Blood Moon", () => {
  it("is a (1) Trap whose Radiant face is a Field Trap; both faces replace a heal", () => {
    expect(def.type).toBe("Trap");
    expect(def.radiant.type).toBe("Field Trap");
    expect(base.replacements?.[0]).toMatchObject({ on: "healed", instead: { damage: "pierce", lasting: "thisTurn" } });
    expect(radiant.replacements?.[0]).toMatchObject({ on: "healed", instead: { damage: "pierce" } });
    expect(radiant.staticFlags?.healToDamage).toBe(true);
  });

  describe("base", () => {
    it("R413 an enemy heal fires it and is itself converted: 20 healing becomes 20 Pierce damage", () => {
      const s = moonUp(false, { hand: [FIG, FILLER] });
      s.play(FIG, { targets: enemyHero });
      expect(fired(s)).toBe(true);
      s.expectHealth("p2", 10);
      expect(moonHits(s).map((hit) => hit.amount)).toEqual([20]);
      s.expectInZone(MOON, "graveyard");
      expect(s.events.some((event) => event.type === "healed")).toBe(false);
    });

    it("R413 a heal on an enemy Unit converts the same way", () => {
      const s = moonUp(false, { hand: [FIG, FILLER], field: [{ def: MENACE, lane: 1, damage: 4 }] });
      const menace = s.unit("p2", 1);
      if (menace === null) throw new Error("setup");
      s.play(FIG, { targets: [{ pick: "instance", instanceId: menace.id }] });
      expect(s.card(menace).zone.z).toBe("graveyard");
    });

    it("§6.1 Lifesteal is healing: the attacker's Lifesteal heal becomes damage to its own hero", () => {
      const s = moonUp(false, { field: [{ def: JILLIAX, lane: 1 }] });
      s.attack(s.unit("p2", 1) ?? "", "hero");
      s.expectHealth("p1", 27);
      expect(fired(s)).toBe(true);
      s.expectHealth("p2", 27);
    });

    it("§6.3 \"heal up to\" converts what it would restore", () => {
      const s = moonUp(false, { hand: [RENO, FILLER], health: 22 });
      s.play(RENO);
      expect(fired(s)).toBe(true);
      s.expectHealth("p2", 14);
    });

    it("§6.3 \"heal to full\" converts what it would restore", () => {
      const s = moonUp(false, { field: [{ def: MENACE, lane: 1, damage: 3 }] });
      const menace = s.unit("p2", 1);
      if (menace === null) throw new Error("setup");
      s.endTurn();
      expect(fired(s)).toBe(true);
      expect(s.card(menace).damage).toBe(6);
    });

    it("every enemy heal for the rest of that turn converts, the trap already in the graveyard", () => {
      const s = moonUp(false, { hand: [FIG, FIG, FILLER], mana: 6 });
      const [first, second] = s.hand("p2").filter((card) => card.defId === FIG);
      if (first === undefined || second === undefined) throw new Error("two Figs");
      s.play(first, { targets: enemyHero });
      s.expectHealth("p2", 10);
      s.play(second, { targets: [{ pick: "hero", player: "p2" }] });
      s.expectHealth("p2", -10);
    });

    it("the conversion ends at that turn's cleanup: the next turn's heal heals", () => {
      const s = moonUp(false, { hand: [FIG, FIG, FILLER], health: 40 });
      const [first, second] = s.hand("p2").filter((card) => card.defId === FIG);
      if (first === undefined || second === undefined) throw new Error("two Figs");
      s.play(first, { targets: enemyHero });
      s.expectHealth("p2", 20);
      s.endTurn().endTurn();
      s.play(second, { targets: enemyHero });
      s.expectHealth("p2", 40);
    });

    it("R346 the damage is Pierce: the hero's Armor is skipped", () => {
      const s = moonUp(false, { hand: [FIG, FILLER], armor: 5 });
      s.play(FIG, { targets: enemyHero });
      s.expectHealth("p2", 10);
    });

    it("§4.4 a hit cap still applies: Anti-oneshot Armor holds the 20 to 5", () => {
      const s = moonUp(false, { hand: [FIG, FILLER], backrow: [{ def: ANTI_ONESHOT, lane: 2 }] });
      s.play(FIG, { targets: enemyHero });
      s.expectHealth("p2", 25);
    });

    it("§4.4 Divine Shield still applies: a shielded enemy Unit loses its shield and takes nothing", () => {
      const s = moonUp(false, { hand: [FIG, FILLER], field: [{ def: JILLIAX, lane: 1, damage: 1 }] });
      const jilliax = s.unit("p2", 1);
      if (jilliax === null) throw new Error("setup");
      s.play(FIG, { targets: [{ pick: "instance", instanceId: jilliax.id }] });
      expect(s.card(jilliax).damage).toBe(1);
      expect(s.stats(jilliax).keywords.some((keyword) => keyword.kind === "Divine Shield")).toBe(false);
    });

    it("B5 E6 no Spell Damage: a Trap's damage is not a Spell's", () => {
      const s = moonUp(false, { hand: [FIG, FILLER] }, { field: [{ def: SOLARIUS, lane: 1 }] });
      s.play(FIG, { targets: enemyHero });
      s.expectHealth("p2", 10);
    });

    it("a friendly heal never fires it", () => {
      const s = scenario({
        active: "p1",
        p1: { hand: [FIG, FILLER], health: 5, backrow: [{ def: MOON, lane: 1, faceUp: false }], library: DECK },
        p2: { hand: [FILLER], library: DECK },
      });
      s.play(FIG, { targets: [{ pick: "hero", player: "p1" }] });
      expect(fired(s)).toBe(false);
      s.expectHealth("p1", 25);
      expect(s.backrow("p1", 1)?.defId).toBe(MOON);
    });

    it("§6.3 Set health is no heal: it does not fire", () => {
      const s = moonUp(false, { health: 5 });
      const sink = { state: s.state, events: [] as GameEvent[], rng: createRng(s.state.seed, s.state.rngCursor) };
      applyEffects([setHealth({ to: { of: "selfHero" }, value: 13 })], makeContext(sink, null, { controller: "p2" }));
      expect(sink.events.some((event) => event.type === "trapFired")).toBe(false);
      s.expectHealth("p2", 13);
      expect(s.backrow("p1", 1)?.defId).toBe(MOON);
    });

    it("R33 R97 the opponent learns nothing of it until it fires", () => {
      const s = moonUp(false, { hand: [FIG, FILLER] });
      const before = s.view("p2").opponent.backrow[0];
      expect(before).toMatchObject({ faceDown: true });
      expect(JSON.stringify(before)).not.toContain(MOON);
      expect(JSON.stringify(s.view("p2"))).not.toContain(MOON);
      s.play(FIG, { targets: enemyHero });
      expect(JSON.stringify(s.view("p2").opponent.graveyard)).toContain(MOON);
    });

    it("the state after it fired is plain JSON", () => {
      const s = moonUp(false, { hand: [FIG, FILLER] });
      s.play(FIG, { targets: enemyHero });
      expect(JSON.parse(JSON.stringify(s.state))).toEqual(s.state);
    });
  });

  describe("radiant", () => {
    it("B2.7 its face is a Field Trap, in hand too", () => {
      const s = scenario({ p1: { hand: [{ def: MOON, radiant: true }, MOON, FILLER] }, p2: { hand: [FILLER] } });
      const [shining, plain] = s.hand("p1").filter((card) => card.defId === MOON);
      if (shining === undefined || plain === undefined) throw new Error("setup");
      expect(cardTypeOf(s.state, shining)).toBe("Field Trap");
      expect(cardTypeOf(s.state, plain)).toBe("Trap");
      const hand = s.view("p1").you.hand;
      if (!Array.isArray(hand)) throw new Error("own hand");
      expect(hand.find((card) => card.instanceId === shining.id)?.type).toBe("Field Trap");
    });

    it("R413 fires on an enemy heal, converts it, and stays face-up on the field", () => {
      const s = moonUp(true, { hand: [FIG, FILLER] });
      s.play(FIG, { targets: enemyHero });
      expect(fired(s)).toBe(true);
      s.expectHealth("p2", 10);
      const moon = s.backrow("p1", 1);
      expect(moon?.defId).toBe(MOON);
      expect(moon?.faceUp).toBe(true);
    });

    it("keeps converting every enemy heal on later turns while it stays", () => {
      const s = moonUp(true, { hand: [FIG, FIG, FILLER], health: 60 });
      const [first, second] = s.hand("p2").filter((card) => card.defId === FIG);
      if (first === undefined || second === undefined) throw new Error("two Figs");
      s.play(first, { targets: enemyHero });
      s.expectHealth("p2", 40);
      s.endTurn().endTurn();
      s.play(second, { targets: enemyHero });
      s.expectHealth("p2", 20);
      expect(s.events.filter((event) => event.type === "trapFired")).toHaveLength(1);
    });

    it("a friendly heal is untouched while it stands face-up", () => {
      const s = moonUp(true, { hand: [FIG, FILLER] }, { hand: [FIG, FILLER], health: 5 });
      s.play(FIG, { targets: enemyHero });
      s.endTurn();
      s.play(FIG, { targets: [{ pick: "hero", player: "p1" }] });
      s.expectHealth("p1", 25);
    });
  });
});
