// #8 Mr. Vanilla — SPEC §8.1 row 8, BUILD M4-T4 must-pass: "A 4/4 with no text: Sheepish turns it
// into a Sheep like any unit; radiant a 12/12 with no text, and made Radiant on the field it takes the
// 12/12 face at once, keeping its damage" (patch v0.1.1: it used to be Immutable, and its Radiant face
// had Divine Shield).

import { describe, expect, it } from "vitest";
import type { CardInstance } from "@jackioh/engine";
import { scenario, type Scenario } from "./_harness";

/** The §10.4-computed keyword kinds of a card on the field. */
function keywordKinds(s: Scenario, card: CardInstance): string[] {
  return s.stats(card).keywords.map((keyword) => keyword.kind);
}

const FILLER = "core-016"; // #16 Hit Job: an inert hand and library card, so no turn auto-ends.
const MENACE = "core-019"; // #19 Midrange Menace, 9/9 Taunt.
const GIGA = "core-029"; // #29 GIGA Glowy Jelly Bean.

describe("#8 Mr. Vanilla — base", () => {
  it("enters as a 4/4 with no keywords and no text of its own", () => {
    const s = scenario({
      seed: "core-008-base",
      p1: { hand: ["core-008"], mana: 4 },
      p2: { field: ["core-020"] },
    });

    s.play("core-008");

    s.expectStats("core-008", { attack: 4, health: 4, maxHealth: 4 });
    expect(keywordKinds(s, s.card("core-008"))).toEqual([]);
    // No text: the play emits the play itself and nothing else — no Cry, no trigger. Its announce
    // (§10.5 step 3a, R448) is part of the play.
    expect(s.lastEvents.map((event) => event.type)).toEqual([
      "manaChanged",
      "cardAnnounced",
      "cardPlayed",
      "summoned",
      "cardResolved",
    ]);
  });

  it("patch v0.1.1: no longer Immutable, so Sheepish transforms it into a Sheep Token", () => {
    const s = scenario({
      seed: "core-008-sheepish",
      p1: { hand: ["core-008"], mana: 4 },
      // #41 Sheepish: "When your opponent plays a Unit: Transform it into a Sheep Token" (§8.2).
      p2: { backrow: ["core-041"], field: ["core-020"] },
    });
    const vanilla = s.card("core-008");

    s.play(vanilla);

    s.expectInZone(vanilla, "gone");
    expect(s.unit("p1", 1)?.defId).toBe("core-t-sheep");
    s.expectInZone("core-041", "graveyard");
  });
});

describe("#8 Mr. Vanilla — radiant", () => {
  it("R275 the Radiant face is a 12/12 with no keywords: its stats are its whole upgrade", () => {
    const s = scenario({
      seed: "core-008-radiant",
      // #26 Glowy Jelly Bean makes a chosen hand card Radiant (§8.2 row 26).
      p1: { hand: ["core-026", "core-008"], mana: 8 },
      p2: { field: ["core-020"] },
    });
    const vanilla = s.card("core-008");

    s.play("core-026", { targets: [{ pick: "instance", instanceId: vanilla.id }] });
    s.play(vanilla);

    s.expectStats(vanilla, { attack: 12, health: 12, maxHealth: 12 });
    expect(keywordKinds(s, vanilla)).toEqual([]);
  });

  it("§5.2 made Radiant on the field it takes the 12/12 face at once and keeps its damage", () => {
    const s = scenario({
      seed: "core-008-radiant-flip",
      // Radiant #29 GIGA Glowy Jelly Bean: "every card in your hand and every permanent you control
      // becomes Radiant" (§8.2 row 29); cost 6, so the mana is seeded above the refresh (§2.3).
      p1: {
        hand: [{ def: GIGA, radiant: true }, FILLER],
        field: [{ def: "core-008", damage: 3 }],
        mana: 6,
        library: [FILLER],
      },
      p2: { hand: [FILLER], field: [MENACE], library: [FILLER] },
    });
    const vanilla = s.card("core-008");
    s.expectStats(vanilla, { attack: 4, health: 1, maxHealth: 4 });

    s.play(GIGA);

    expect(s.card(vanilla).radiant).toBe(true);
    s.expectStats(vanilla, { attack: 12, health: 9, maxHealth: 12 });

    // The damage came along (R22): at 9 health it trades with #19's 9 on p2's turn.
    s.endTurn();
    s.attack(MENACE, vanilla);
    s.expectInZone(vanilla, "graveyard");
    s.expectInZone(MENACE, "graveyard");
  });
});
