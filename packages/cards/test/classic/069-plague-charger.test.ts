// C #69 Plague Charger — SPEC §8.6 row 69, BUILD M9 Classic row C 69: "Charge; +2 Attack for each Plague
// Counter on it (a self stat layer, §10.4) and First Strike exactly while it has one (a keyword while a
// condition holds, §6.1), both following the tokens as they come and go and gone when it leaves (R78);
// with no token it has neither; radiant 8/4: +4 per token; its tuned number (attack per token) reads
// through `param()` (R386)".
//
// Its yellow glow (R195, `conditionMet`: "while it has a Plague Counter", on the field) is proved, both
// answers, in `test/condition-active.test.ts`.

import { stepParam, type CardInstance } from "@jackioh/engine";
import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/069-plague-charger";

const CHARGER = "classic-069";
const CRAWLER = "classic-053"; // (1) Unit: Cry: place 1 Plague Counter on another permanent.
const MUTATE = "classic-078"; // (1) Field Spell: Activate ♾️: remove a Plague Counter from a permanent …
const VANILLA = "core-008"; // (1) Unit 4/4.
const FLOOD = "core-017"; // (4) Spell: Bounce all Units.
const FILLER = "core-005"; // (1) Spell (§2.5).
const ANCHOR = "core-010"; // (0) Spell: keeps a spent turn open (§2.5).

function kinds(s: Scenario, card: CardInstance | string): string[] {
  return s.stats(card).keywords.map((keyword) => keyword.kind);
}

function at(card: CardInstance): Selection[] {
  return [{ pick: "instance", instanceId: card.id }];
}

describe("C #69 Plague Charger", () => {
  it("declares its one number, a conditional keyword, a self aura and the glow, one script on both faces", () => {
    expect(def.id).toBe(CHARGER);
    expect(def.params).toEqual([{ key: "attack", base: 2, radiant: 4, better: "up", step: 1, min: 1 }]);
    expect(def.base.keywords).toEqual([{ kind: "Charge" }]);
    expect(base.conditionalKeywords).toBeTypeOf("function");
    expect(base.aura).toBeTypeOf("function");
    expect(base.conditionMet).toBeTypeOf("function");
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("with no token it is a 4/2 with Charge and neither First Strike nor a bonus", () => {
      const s = scenario({ p1: { field: [CHARGER], hand: [FILLER] } });

      s.expectStats(CHARGER, { attack: 4, health: 2 });
      expect(kinds(s, CHARGER)).toEqual(["Charge"]);
    });

    it("Charge: it attacks the enemy hero the turn it is played", () => {
      const s = scenario({ p1: { hand: [CHARGER, FILLER] }, p2: { hand: [FILLER], health: 20 } });

      s.play(CHARGER);
      s.attack(CHARGER, "hero");

      s.expectHealth("p2", 16);
    });

    it("§10.4 +2 Attack for each Plague Counter on it, and First Strike while it has one", () => {
      const one = scenario({ p1: { field: [{ def: CHARGER, counters: { plague: 1 } }], hand: [FILLER] } });
      one.expectStats(CHARGER, { attack: 6, health: 2 });
      expect(kinds(one, CHARGER)).toEqual(["Charge", "First Strike"]);

      const three = scenario({ p1: { field: [{ def: CHARGER, counters: { plague: 3 } }], hand: [FILLER] } });
      three.expectStats(CHARGER, { attack: 10, health: 2, maxHealth: 2 });
      expect(kinds(three, CHARGER)).toEqual(["Charge", "First Strike"]);
    });

    it("both follow the tokens as they come: a C #53 Plague Crawler's placement gives it +2 and First Strike", () => {
      const s = scenario({ p1: { hand: [CRAWLER, FILLER], field: [CHARGER] }, p2: { hand: [FILLER] } });
      expect(kinds(s, CHARGER)).toEqual(["Charge"]);

      s.play(CRAWLER, { targets: at(s.card(CHARGER)) });

      s.expectStats(CHARGER, { attack: 6 });
      expect(kinds(s, CHARGER)).toEqual(["Charge", "First Strike"]);
    });

    it("both follow the tokens as they go: C #78 Mutate Spell's removals take the bonus, then First Strike", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: CHARGER, counters: { plague: 2 } }], backrow: [MUTATE] },
        p2: { hand: [FILLER], health: 30 },
      });
      const charger = s.card(CHARGER);
      s.expectStats(charger, { attack: 8 });

      // Each removal comes before the forced attack it buys, which here can only be at the hero.
      s.activate(MUTATE, { targets: at(charger) });
      s.expectStats(charger, { attack: 6 });
      expect(kinds(s, charger)).toEqual(["Charge", "First Strike"]);
      s.expectHealth("p2", 24);

      s.activate(MUTATE, { targets: at(charger) });
      s.expectStats(charger, { attack: 4 });
      expect(kinds(s, charger)).toEqual(["Charge"]);
      s.expectHealth("p2", 20);
    });

    it("R78 they are gone when it leaves: bounced to hand and played again, it is a plain 4/2", () => {
      const s = scenario({
        p1: { hand: [FILLER], field: [{ def: CHARGER, counters: { plague: 2 } }], mana: 10 },
        p2: { hand: [FLOOD, ANCHOR] },
        active: "p2",
      });
      const charger = s.card(CHARGER);

      s.play(FLOOD);
      s.expectInZone(charger, "hand");
      expect(s.card(charger).counters.plague).toBeUndefined();
      s.endTurn();
      s.play(charger);

      s.expectStats(charger, { attack: 4, health: 2 });
      expect(kinds(s, charger)).toEqual(["Charge"]);
    });

    it("§4.3 with a token its First Strike kills a 4/4 before it strikes back; with none, both die", () => {
      const plagued = scenario({
        p1: { hand: [FILLER], field: [{ def: CHARGER, counters: { plague: 1 } }] },
        p2: { hand: [FILLER], field: [VANILLA] },
      });
      plagued.attack(CHARGER, plagued.card(VANILLA));
      plagued.expectInZone(VANILLA, "graveyard");
      plagued.expectInZone(CHARGER, "field");

      const clean = scenario({ p1: { hand: [FILLER], field: [CHARGER] }, p2: { hand: [FILLER], field: [VANILLA] } });
      clean.attack(CHARGER, clean.card(VANILLA));
      clean.expectInZone(VANILLA, "graveyard");
      clean.expectInZone(CHARGER, "graveyard");
    });

    it("the bonus is its own: no other Unit gains from its tokens", () => {
      const s = scenario({ p1: { hand: [FILLER], field: [{ def: CHARGER, counters: { plague: 2 } }, { def: VANILLA, counters: { plague: 2 } }] } });

      s.expectStats(VANILLA, { attack: 4 });
      expect(kinds(s, VANILLA)).toEqual([]);
    });

    it("R386 an Upgrade makes it +3 per token; a Degrade +1", () => {
      const up = scenario({ p1: { field: [{ def: CHARGER, counters: { plague: 2 } }], hand: [FILLER] } });
      stepParam(up.card(CHARGER), "attack", 1);
      up.expectStats(CHARGER, { attack: 10 });

      const down = scenario({ p1: { field: [{ def: CHARGER, counters: { plague: 2 } }], hand: [FILLER] } });
      stepParam(down.card(CHARGER), "attack", -1);
      down.expectStats(CHARGER, { attack: 6 });
    });
  });

  describe("radiant", () => {
    it("is an 8/4 with Charge; with no token neither First Strike nor a bonus", () => {
      const s = scenario({ p1: { field: [{ def: CHARGER, radiant: true }], hand: [FILLER] } });

      s.expectStats(CHARGER, { attack: 8, health: 4 });
      expect(kinds(s, CHARGER)).toEqual(["Charge"]);
    });

    it("+4 Attack for each Plague Counter on it, and First Strike while it has one", () => {
      const s = scenario({ p1: { field: [{ def: CHARGER, radiant: true, counters: { plague: 2 } }], hand: [FILLER] } });

      s.expectStats(CHARGER, { attack: 16, health: 4 });
      expect(kinds(s, CHARGER)).toEqual(["Charge", "First Strike"]);
    });

    it("R386 an Upgrade makes it +5 per token", () => {
      const s = scenario({ p1: { field: [{ def: CHARGER, radiant: true, counters: { plague: 1 } }], hand: [FILLER] } });
      stepParam(s.card(CHARGER), "attack", 1);

      s.expectStats(CHARGER, { attack: 13 });
    });
  });
});
