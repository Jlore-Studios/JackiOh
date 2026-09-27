// #46 Suppressive Aura — SPEC §8.2, BUILD M4-T4: "Embiggen price chosen with the play (R81);
// −1/−1 to all, 1-health units die, restored on leaving; paid 4 → −2/−2; radiant enemy only −2/−2,
// paid 4 → −4/−4; enough of them together kill an enemy The Rock despite Indestructible (R69)"
// (patch v0.1.1 cut the numbers from −2/−2 and −5/−5, radiant −4/−4 and −10/−10).
//
// Every stat assertion goes through `expectStats`, which reads the engine's `unitView`, so the
// number already has §10.4 layer 5 in it — the aura is never asserted by inspecting the script.
//
// The props, chosen for having no script of their own beyond printed keywords, so nothing but the
// aura moves their numbers:
//   #25 4-mana 7/7 (7/7, Armor 7)   — the survivor, and the unit whose restoration is asserted.
//   #20 Pointmaster (7/1, First Strike) — the 1-health unit §8.2's Engine cell kills.
//   #45 Deft Duelist (4/3, Charge)  — survives −1/−1 and −2/−2.
//   #56 Jilliax (3/2, Rush, Taunt, Lifesteal, Divine Shield) — survives −1/−1, dies to −2/−2.
//   #66 The Rock (10/10, Indestructible) — R69's unit, killed only once its max health reaches 0.
//
// The embiggen price cannot be set on a card the scenario builder PLACES — no setup entry carries
// `embiggened` — which is as it should be: R81 makes the price part of the play. So every paid-4
// case here plays the card out of hand with `{ embiggen: true }` and checks the mana as well, and
// `reduce.playCard` is what writes the answer to `instance.embiggened` (R65).

import { describe, expect, it } from "vitest";
// The harness has no verb for "a permanent leaves the field" and none of #46's own text removes
// one, so the engine's own mover is used for that one step (see the "restored on leaving" test).
import { moveToZone } from "@jackioh/engine";
import { scenario } from "./_harness";
import { base, def, radiant } from "../src/scripts/046-suppressive-aura";

const AURA = "core-046";
/** The radiant face of a card that has to be PLAYED for its embiggen price to exist (R81). */
const RADIANT_AURA = { def: AURA, radiant: true } as const;
const SEVEN_SEVEN = "core-025";
const POINTMASTER = "core-020";
const DUELIST = "core-045";
const JILLIAX = "core-056";
const ROCK = "core-066";

describe("#46 Suppressive Aura", () => {
  it("base: every unit on both sides is −1/−1 while it is in play (§10.4 layer 5)", () => {
    const g = scenario({
      p1: { hand: [AURA], field: [{ def: SEVEN_SEVEN, lane: 1 }] },
      p2: { field: [{ def: SEVEN_SEVEN, lane: 1 }] },
    });
    const mine = g.unit("p1", 1);
    const theirs = g.unit("p2", 1);
    expect(mine).not.toBeNull();
    expect(theirs).not.toBeNull();
    if (mine === null || theirs === null) return;

    g.expectStats(mine, { attack: 7, maxHealth: 7, health: 7 });
    g.expectStats(theirs, { attack: 7, maxHealth: 7, health: 7 });

    g.play(AURA);

    // "All Units": the controller's own board is suppressed too.
    g.expectStats(mine, { attack: 6, maxHealth: 6, health: 6 });
    g.expectStats(theirs, { attack: 6, maxHealth: 6, health: 6 });
    g.expectMana("p1", 2);
  });

  it("base: a 1-health unit dies at the state check, with no destroy effect involved (§4.5)", () => {
    const g = scenario({
      p1: { hand: [AURA] },
      p2: { field: [{ def: POINTMASTER, lane: 2 }, { def: SEVEN_SEVEN, lane: 3 }] },
    });
    const doomed = g.unit("p2", 2);
    const survivor = g.unit("p2", 3);
    if (doomed === null || survivor === null) throw new Error("setup: p2 should hold both units");

    g.play(AURA);

    // §10.4: max health can fall to 0, and §4.5 step 1 collects the unit at the next state check.
    g.expectInZone(doomed, "graveyard").expectEvents("cardPlayed", "destroyed", "enteredGraveyard");
    g.expectStats(survivor, { attack: 6, maxHealth: 6 });
  });

  it("base: leaving the field restores the survivors (§8.2 Engine)", () => {
    const g = scenario({
      p1: { hand: [AURA], field: [{ def: SEVEN_SEVEN, lane: 1 }] },
      p2: { field: [{ def: DUELIST, lane: 1 }] },
    });
    g.play(AURA);
    const mine = g.unit("p1", 1);
    const theirs = g.unit("p2", 1);
    if (mine === null || theirs === null) throw new Error("setup: both units should still be there");
    g.expectStats(mine, { attack: 6, maxHealth: 6 });
    g.expectStats(theirs, { attack: 3, maxHealth: 2 });

    const aura = g.backrow("p1", 1);
    if (aura === null) throw new Error("the Field Spell should be in p1's backrow");
    moveToZone(g.state, aura, "graveyard");

    // The aura is a layer computed on every read, so nothing had to be undone.
    g.expectStats(mine, { attack: 7, maxHealth: 7, health: 7 });
    g.expectStats(theirs, { attack: 4, maxHealth: 3, health: 3 });
  });

  it("R81, R65 base paid 4: the price chosen with the play makes it −2/−2", () => {
    const g = scenario({
      p1: { hand: [AURA], field: [{ def: SEVEN_SEVEN, lane: 1 }] },
      p2: { field: [{ def: JILLIAX, lane: 1 }, { def: DUELIST, lane: 2 }] },
    });
    const mine = g.unit("p1", 1);
    const doomed = g.unit("p2", 1);
    const survivor = g.unit("p2", 2);
    if (mine === null || doomed === null || survivor === null) throw new Error("setup: the units should be there");

    // The price travels in the play action, never as a prompt (§10.6: no Core card asks).
    g.play(AURA, { embiggen: true });

    g.expectMana("p1", 0);
    g.expectStats(mine, { attack: 5, maxHealth: 5 });
    // A 3/2 is at 0 max health now, and the 4/3 is a 2/1.
    g.expectInZone(doomed, "graveyard");
    g.expectStats(survivor, { attack: 2, maxHealth: 1 });
    expect(g.state.pending).toBeNull();
  });

  it("base: playing it for 2 leaves the instance unembiggened (R65's 'chosen price')", () => {
    const g = scenario({ p1: { hand: [AURA] } });
    g.play(AURA);
    expect(g.backrow("p1", 1)?.embiggened).toBe(false);
  });

  it("radiant: enemy units are −2/−2 and the controller's own are untouched", () => {
    const g = scenario({
      p1: { hand: [RADIANT_AURA], field: [{ def: SEVEN_SEVEN, lane: 1 }] },
      p2: { field: [{ def: SEVEN_SEVEN, lane: 1 }] },
    });
    const mine = g.unit("p1", 1);
    const theirs = g.unit("p2", 1);
    if (mine === null || theirs === null) throw new Error("setup: both units should be there");

    g.play(AURA);

    g.expectStats(mine, { attack: 7, maxHealth: 7, health: 7 });
    g.expectStats(theirs, { attack: 5, maxHealth: 5, health: 5 });
  });

  it("R81 radiant paid 4: enemy units are −4/−4", () => {
    const g = scenario({
      p1: { hand: [RADIANT_AURA], field: [{ def: SEVEN_SEVEN, lane: 1 }] },
      p2: { field: [{ def: SEVEN_SEVEN, lane: 1 }] },
    });
    const theirs = g.unit("p2", 1);
    if (theirs === null) throw new Error("setup: the enemy unit should be there");

    g.play(AURA, { embiggen: true });

    g.expectMana("p1", 0);
    g.expectStats(theirs, { attack: 3, maxHealth: 3, health: 3 });
    g.expectStats(SEVEN_SEVEN, { attack: 7, maxHealth: 7 });
  });

  it("R69 radiant paid 4 on top of three more radiant auras: an enemy The Rock at 0 max health dies despite Indestructible", () => {
    const g = scenario({
      p1: {
        hand: [RADIANT_AURA],
        field: [{ def: ROCK, lane: 1 }],
        // Three placed radiant auras at their base price: −6/−6 on the enemy between them.
        backrow: [RADIANT_AURA, RADIANT_AURA, RADIANT_AURA],
      },
      p2: { field: [{ def: ROCK, lane: 1 }] },
    });
    const mine = g.unit("p1", 1);
    const doomed = g.unit("p2", 1);
    if (mine === null || doomed === null) throw new Error("setup: both Rocks should be there");
    g.expectStats(doomed, { attack: 4, maxHealth: 4 });

    g.play(AURA, { embiggen: true });

    // R69: no destroy effect is involved, so Indestructible does not save a unit at 0 max health;
    // it is collected like any other, which is why this is a death and not a survival.
    g.expectInZone(doomed, "graveyard").expectEvents("destroyed");
    const death = g.lastEvents.find(
      (event) => event.type === "destroyed" && event.instanceId === doomed.id,
    );
    expect(death).toMatchObject({ maxHealth: 0, killerId: null });
    // Enemy units only: the auras' own side keeps its 10/10 Rock.
    g.expectStats(mine, { attack: 10, maxHealth: 10, health: 10 });
  });

  it("radiant: an ally at 0 max health would die too, but the radiant aura never touches allies", () => {
    const g = scenario({
      p1: { hand: [RADIANT_AURA], field: [{ def: POINTMASTER, lane: 1 }] },
      p2: { field: [{ def: POINTMASTER, lane: 1 }] },
    });
    const mine = g.unit("p1", 1);
    const doomed = g.unit("p2", 1);
    if (mine === null || doomed === null) throw new Error("setup: both units should be there");

    g.play(AURA);

    g.expectInZone(doomed, "graveyard");
    g.expectInZone(mine, "field").expectStats(mine, { attack: 7, maxHealth: 1 });
  });

  it("both faces are a §10.4 layer-5 aura and ask for nothing at play time (R81, §10.6)", () => {
    expect(def.id).toBe(AURA);
    expect(def.type).toBe("Field Spell");
    expect(def.cost).toEqual({ base: 2, embiggen: 4 });
    for (const face of [base, radiant]) {
      expect(typeof face.aura).toBe("function");
      // The price is not a declared choice and not a prompt: it rides in the play action (R81).
      expect(face.targets).toBeUndefined();
      expect(face.modes).toBeUndefined();
      expect(face.cry).toBeUndefined();
    }
  });
});
