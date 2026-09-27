// #56 Jilliax (SPEC §8.3, BUILD M4-T4 row 56: "All four keywords; radiant all four plus Reborn").
// A keywords-only card, so every test asserts either the computed keyword set (§10.4's keyword
// layer, read through `viewFor`) or the rule each keyword names in §6.1.
//
// Patch v0.1.1: the radiant face is "Rush, Taunt, Lifesteal, Divine Shield, Reborn" — the base list
// plus Reborn, where it used to trade Rush and Divine Shield for Charge and Indestructible.

import { describe, expect, it } from "vitest";
import type { PlayerId } from "@jackioh/shared";
import { scenario, type Scenario } from "./_harness";

/**
 * A turn anchor (§2.5, R82): #10 Rapid Replenish is a 0-cost Spell, so holding one keeps at least
 * one legal action other than ending the turn, conceding and offering a draw on a side whose units
 * have all spent their exertion. It is never played; it only stops `reduce` from auto-ending the
 * turn underneath an assertion. It is a Spell rather than a unit so it never joins the board a test
 * is counting, and so a def-id reference to the attacker stays unambiguous.
 */
const ANCHOR = "core-010";

/** The keyword kinds §10.4 computes for the unit in `lane`, sorted so the set is order-free. */
function keywordKinds(g: Scenario, player: PlayerId, lane: number): string[] {
  const unit = g.view(player).you.units[lane - 1];
  if (unit === null || unit === undefined) throw new Error(`no unit in ${player} lane ${lane}`);
  return unit.keywords.map((keyword) => keyword.kind).sort();
}

describe("#56 Jilliax — base", () => {
  it("§6.1 prints Rush, Taunt, Lifesteal and Divine Shield, and nothing else", () => {
    const g = scenario({ p1: { field: [{ def: "core-056", lane: 1 }] } });

    expect(keywordKinds(g, "p1", 1)).toEqual(["Divine Shield", "Lifesteal", "Rush", "Taunt"]);
    g.expectStats("core-056", { attack: 3, maxHealth: 2, health: 2 });
  });

  it("§4.1 Rush lets it attack a unit on its summon turn but not the hero", () => {
    const g = scenario({
      p1: { hand: ["core-056", "core-005"] },
      p2: { field: [{ def: "core-008", lane: 1, damage: 1 }] },
    });

    g.play("core-056", { zone: 1 });

    // §4.2 step 2: "Rush cannot hit the hero on its summon turn" — the sickness lift is for units.
    expect(() => g.attack("core-056", "hero")).toThrow(/Rush cannot hit the hero/);
    g.expectHealth("p2", 30);

    // The same sick unit may attack a unit: 3 into a Mr. Vanilla at 3 health, which kills it.
    //
    // The kill cannot be read as `health: 0` on the card. R78 resets an instance's damage as it
    // leaves the field, so the graveyard copy reads its printed 4/4 undamaged — an assertion on
    // its computed health could never hold. R89 is where the stats as they were survive: "the
    // `destroyed` event carries what the card was … its attack and max health as the layers
    // computed them at the moment it died".
    g.attack("core-056", "core-008").expectInZone("core-008", "graveyard");

    const killed = g.lastEvents.find((event) => event.type === "destroyed");
    expect(killed).toMatchObject({ defId: "core-008", attack: 4, maxHealth: 4 });
  });

  it("§4.2 step 3 Taunt forces the attacker onto it while any other enemy unit stands", () => {
    const g = scenario({
      p1: { field: [{ def: "core-025", lane: 1 }] },
      p2: {
        field: [
          { def: "core-008", lane: 1 },
          { def: "core-056", lane: 2 },
        ],
      },
    });

    expect(() => g.attack("core-025", "core-008")).toThrow(/Taunt unit must be attacked first/);
    expect(() => g.attack("core-025", "hero")).toThrow(/Taunt unit must be attacked first/);

    g.attack("core-025", "core-056").expectEvents("attackDeclared");
  });

  it("§4.4 step 1 Divine Shield eats the whole strike-back and is then gone", () => {
    const g = scenario({
      p1: { field: [{ def: "core-056", lane: 1 }] },
      p2: { field: [{ def: "core-019", lane: 1 }] },
    });

    // Midrange Menace is 9/9 with Taunt, so it is both a legal target and lethal on the strike-back.
    g.attack("core-056", "core-019");

    g.expectEvents("divineShieldLost").expectInZone("core-056", "field");
    g.expectStats("core-056", { health: 2, maxHealth: 2 });
    expect(keywordKinds(g, "p1", 1)).toEqual(["Lifesteal", "Rush", "Taunt"]);
  });

  it("§4.4 step 8 Lifesteal heals its controller's hero by the amount dealt (R63)", () => {
    const g = scenario({
      // R82/§2.5 TURN ANCHOR: the attack below spends Jilliax's only exertion and kills the only
      // enemy unit, so without a card in hand p1's remaining legal actions would be ending the
      // turn, conceding and offering a draw — `reduce` would auto-end the turn underneath the
      // assertion, p2 would take a turn and both heroes would take fatigue off an empty library
      // (the tell is `turnAutoEnded` followed by `damage amount: 1, sourceId: null`). #10 Rapid
      // Replenish is a 0-cost Spell and therefore always an affordable play, so it holds the turn
      // open without putting a second body on the board — which matters here, because the attacker
      // is named by def id.
      p1: { field: [{ def: "core-056", lane: 1 }], hand: [ANCHOR], health: 20 },
      p2: { field: [{ def: "core-008", lane: 1 }] },
    });

    g.attack("core-056", "core-008");

    // 3 dealt to Mr. Vanilla, so 3 back to the hero — the amount DEALT, not the printed attack.
    g.expectHealth("p1", 23).expectEvents("damage", "healed");
  });

  it("§4.4 step 8 Lifesteal heals nothing when the hit dealt nothing", () => {
    const g = scenario({
      p1: { field: [{ def: "core-056", lane: 1 }], health: 20 },
      // The Rock is Indestructible, so pipeline step 4 stops the hit and 0 is dealt (R63's zero rule).
      p2: { field: [{ def: "core-066", lane: 1 }] },
    });

    g.attack("core-056", "core-066").expectHealth("p1", 20);
  });
});

describe("#56 Jilliax — radiant", () => {
  it("§8 patch v0.1.1: the radiant face keeps all four keywords and adds Reborn", () => {
    const g = scenario({ p1: { field: [{ def: "core-056", radiant: true, lane: 1 }] } });

    expect(keywordKinds(g, "p1", 1)).toEqual(["Divine Shield", "Lifesteal", "Reborn", "Rush", "Taunt"]);
    g.expectStats("core-056", { attack: 6, maxHealth: 4, health: 4 });
  });

  it("§4.1 Rush, not Charge: a played radiant Jilliax may attack a unit but not the hero", () => {
    const g = scenario({
      p1: { hand: [{ def: "core-056", radiant: true }, "core-005"] },
      p2: { field: [{ def: "core-008", lane: 1 }] },
    });

    g.play("core-056", { zone: 1 });
    expect(() => g.attack("core-056", "hero")).toThrow(/Rush cannot hit the hero/);
    g.attack("core-056", "core-008").expectInZone("core-008", "graveyard");
  });

  it("§4.4 step 1 its Divine Shield eats the first strike-back, and §4.5 step 4 Reborn catches the second death", () => {
    const g = scenario({
      p1: { field: [{ def: "core-056", radiant: true, lane: 1 }], hand: [ANCHOR], health: 20 },
      p2: { field: [{ def: "core-019", lane: 1 }], hand: [ANCHOR] },
    });

    // Midrange Menace's 9 back is negated whole by the shield; Jilliax's 6 still heals 6.
    g.attack("core-056", "core-019");
    g.expectEvents("divineShieldLost").expectInZone("core-056", "field");
    g.expectStats("core-056", { health: 4, maxHealth: 4 });
    g.expectHealth("p1", 26);

    // p2's turn: the 9/9 kills the unshielded Jilliax, and Reborn brings it back at 1 health
    // without Reborn — a reset instance (R78), so its printed Divine Shield is whole again.
    g.endTurn();
    g.attack("core-019", "core-056");
    g.expectInZone("core-056", "field");
    g.expectStats("core-056", { health: 1, maxHealth: 4 });
    expect(keywordKinds(g, "p1", 1)).toEqual(["Divine Shield", "Lifesteal", "Rush", "Taunt"]);
  });

  it("§4.2 step 3 Taunt is still on the radiant face", () => {
    // Patch v0.1.1 took Indestructible off this face, so R347 leaves its printed Taunt standing.
    const g = scenario({
      p1: { field: [{ def: "core-025", lane: 1 }] },
      p2: {
        field: [
          { def: "core-008", lane: 1 },
          { def: "core-056", radiant: true, lane: 2 },
        ],
      },
    });

    expect(() => g.attack("core-025", "core-008")).toThrow(/Taunt unit must be attacked first/);
  });
});
