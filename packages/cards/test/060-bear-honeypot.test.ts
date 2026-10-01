// #60 Bear Honeypot (SPEC §8.3, BUILD M4-T4 row 60: "Fires after the opponent's ≤1-cost play
// resolves (R17, R56); a unit is attacked by each token in order until dead, one combat each (R53);
// radiant any card and fills the board"), and patch v0.2.0's condition (R430): "if you have an empty
// unit zone" — while its controller's unit row has no empty, unlocked, unreserved zone the trap does
// not fire and is not consumed.
//
// Every test here puts the trap face-down in p1's backrow and makes p2 the active player, which is
// the whole point of a Trap: it fires on the OPPONENT's turn and resolves to completion before their
// action continues (§10.3). Bear Honeypot opens no prompt, so the "a trap may prompt its own
// controller" path of §10.3 has nothing to exercise here.
//
// Rulings proved here: R17 (this trap fires after the played card has resolved, the moment #41
// Sheepish shares since R427), R56 (the threshold reads the cost actually paid), R70 (a cast pays 0,
// so it is "costing 1 or less"), R53 (forced attacks in full), R64 ("fill your board" is every empty
// unlocked unit zone, left to right), R11 (a dead unit token ceases to exist), R430 (no empty unit
// zone: the trap waits), §5.1 (a Trap is consumed when it fires).

import { describe, expect, it } from "vitest";
import type { PlayerId } from "@jackioh/shared";
import { scenario, type Scenario } from "./_harness";

/** The trap, face-down in p1's backrow lane 3 — a Trap is only ever face-down until it fires (§5.1). */
function armed(radiant = false): { def: string; radiant?: boolean; faceUp: boolean; lane: number } {
  return { def: "core-060", ...(radiant ? { radiant: true } : {}), faceUp: false, lane: 3 };
}

function unitsOf(g: Scenario, player: PlayerId): string[] {
  return [1, 2, 3, 4, 5].flatMap((lane) => {
    const unit = g.unit(player, lane);
    return unit === null ? [] : [unit.defId];
  });
}

function countOf(g: Scenario, type: "trapFired" | "attackDeclared" | "summoned"): number {
  return g.events.filter((event) => event.type === type).length;
}

/** p1 needs no hand: it is not their turn. p2 keeps a spare card so R82 never auto-ends the turn. */
const SPARE = { hand: ["core-005"], library: ["core-011", "core-016"] };

describe("#60 Bear Honeypot — base, when it fires", () => {
  it("R56 fires on the opponent's 1-cost play and summons 2 Rush Tokens", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed()] },
      p2: { ...SPARE, hand: ["core-015", "core-005"] },
    });

    g.play("core-015", { zone: 1 });

    g.expectEvents("cardPlayed", "trapFired", "summoned");
    expect(unitsOf(g, "p1")).toEqual(["core-t-rush", "core-t-rush"]);
  });

  it("R56 a 2-cost play does not fire it: the trap stays armed and face-down", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed()] },
      p2: { ...SPARE, hand: ["core-020", "core-005"] },
    });

    g.play("core-020", { zone: 1 });

    expect(countOf(g, "trapFired")).toBe(0);
    expect(unitsOf(g, "p1")).toEqual([]);
    g.expectInZone("core-060", "field");
    expect(g.backrow("p1", 3)?.faceUp).toBe(false);
  });

  it("R70 a cast pays 0, so a cast card is 'costing 1 or less'", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed()] },
      // #21 Hinder is cost 0 and cast on draw: drawing it casts it (§2.4, R58). Its Radiant face
      // discards nothing (R431), so the cast asks no question.
      p2: { hand: ["core-005"], library: [{ def: "core-021", radiant: true }, "core-011"] },
    });

    g.startTurn();

    g.expectEvents("drawn", "cardPlayed", "trapFired", "summoned");
    expect(unitsOf(g, "p1")).toEqual(["core-t-rush", "core-t-rush"]);
  });

  it("'the opponent plays': the trap's own controller never sets it off", () => {
    const g = scenario({
      p1: { ...SPARE, backrow: [armed()], hand: ["core-011", "core-005"] },
      p2: { ...SPARE },
    });

    g.play("core-011", { zone: 1 });

    expect(countOf(g, "trapFired")).toBe(0);
    g.expectInZone("core-060", "field");
  });

  it("§5.1 a Trap is consumed when it fires and goes to its owner's graveyard", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed()] },
      p2: { ...SPARE, hand: ["core-005", "core-011"] },
    });

    g.play("core-005");

    g.expectEvents("trapFired", "enteredGraveyard").expectInZone("core-060", "graveyard");
    expect(countOf(g, "trapFired")).toBe(1);
  });
});

describe("#60 Bear Honeypot — R430, only with an empty unit zone", () => {
  /** p1's unit row, lanes 1–5, filled with #19s except the lanes given. */
  function row(except: readonly number[] = []): { def: string; lane: number }[] {
    return [1, 2, 3, 4, 5].filter((lane) => !except.includes(lane)).map((lane) => ({ def: "core-019", lane }));
  }

  it("R430 a full unit row: the 1-cost play resolves, and the trap neither fires nor is consumed", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed()], field: row() },
      p2: { ...SPARE, hand: ["core-015", "core-005"] },
    });

    g.play("core-015", { zone: 1 });

    expect(countOf(g, "trapFired")).toBe(0);
    expect(countOf(g, "attackDeclared")).toBe(0);
    g.expectInZone("core-060", "field");
    expect(g.backrow("p1", 3)?.faceUp).toBe(false);
    expect(unitsOf(g, "p2")).toEqual(["core-015", "core-t-rush"]);
  });

  it("R430 a Locked zone is not an empty unit zone: the trap waits", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed()], field: row([5]) },
      p2: { ...SPARE, hand: ["core-015", "core-005"] },
    });
    g.state.players.p1.locks.units[4] = true;

    g.play("core-015", { zone: 1 });

    expect(countOf(g, "trapFired")).toBe(0);
    g.expectInZone("core-060", "field");
  });

  it("R430, R64 a zone a dying Reborn unit holds is not an empty unit zone either", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed()], field: row([5]) },
      p2: { ...SPARE, hand: ["core-015", "core-005"] },
    });
    g.state.reserved.push({ player: "p1", row: "units", lane: 5 });

    g.play("core-015", { zone: 1 });

    expect(countOf(g, "trapFired")).toBe(0);
    g.expectInZone("core-060", "field");
  });

  it("R430, R64 one empty zone is enough: it fires, the one token that fits arrives and attacks, the other fizzles", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed()], field: row([4]) },
      p2: { ...SPARE, hand: ["core-015", "core-005"] },
    });

    g.play("core-015", { zone: 1 });

    expect(countOf(g, "trapFired")).toBe(1);
    expect(g.unit("p1", 4)?.defId).toBe("core-t-rush");
    expect(unitsOf(g, "p1").filter((defId) => defId === "core-t-rush")).toHaveLength(1);
    g.expectInZone("core-060", "graveyard");
    // The played 1/1 met the one token and died to it.
    expect(g.unit("p2", 1)?.defId).not.toBe("core-015");
  });

  it("R430 a trap that waited fires on a later play once a zone is free", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed()], field: row() },
      p2: { ...SPARE, hand: ["core-015", "core-016", "core-005", "core-011"], mana: 10 },
    });
    g.play("core-015", { zone: 1 });
    expect(countOf(g, "trapFired")).toBe(0);

    // #16 Hit Job (3) opens p1's lane 2; it costs too much to set the trap off itself.
    const menace = g.unit("p1", 2);
    expect(menace).not.toBeNull();
    g.play("core-016", { targets: [{ pick: "instance", instanceId: menace?.id ?? "" }] });
    expect(countOf(g, "trapFired")).toBe(0);

    // #5 Stockpile (1) now finds the room: the trap fires, one token fits, and a Spell is attacked by nothing.
    g.play("core-005");

    expect(countOf(g, "trapFired")).toBe(1);
    expect(g.unit("p1", 2)?.defId).toBe("core-t-rush");
    expect(countOf(g, "attackDeclared")).toBe(0);
    g.expectInZone("core-060", "graveyard");
  });

  it("R430 the radiant face waits on a full row too", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed(true)], field: row() },
      p2: { ...SPARE, hand: ["core-020", "core-005"] },
    });

    g.play("core-020", { zone: 1 });

    expect(countOf(g, "trapFired")).toBe(0);
    g.expectInZone("core-060", "field");
    expect(g.backrow("p1", 3)?.faceUp).toBe(false);
  });
});

describe("#60 Bear Honeypot — base, R17's timing", () => {
  it("R17 fires AFTER the played card has resolved, so its Cry has already happened", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed()] },
      // #15 Me and Mr Token is a 1-cost Unit whose Cry summons a Rush Token for ITS controller.
      p2: { ...SPARE, hand: ["core-015", "core-005"] },
    });

    g.play("core-015", { zone: 1 });

    // cardPlayed → Mr Token summoned → its Cry's token summoned → only then trapFired.
    g.expectEvents("cardPlayed", "summoned", "summoned", "trapFired", "summoned");
    // The Cry's token is p2's and was never the forced-attack target.
    expect(unitsOf(g, "p2")).toContain("core-t-rush");
  });

  it("the Engine cell: the played unit is on the field when the tokens arrive", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed()] },
      p2: { ...SPARE, hand: ["core-015", "core-005"] },
    });

    g.play("core-015", { zone: 1 });

    // It was attacked, which is only possible because it was on the field (§4.2 step 2).
    g.expectEvents("trapFired", "attackDeclared");
  });
});

describe("#60 Bear Honeypot — base, the forced attacks (R53)", () => {
  it("R53 each forced attack is its own combat, and the second token does not attack a dead target", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed()] },
      // Me and Mr Token is 1/1: one 3-damage hit kills it, and it strikes back for 1.
      p2: { ...SPARE, hand: ["core-015", "core-005"] },
    });

    g.play("core-015", { zone: 1 });

    expect(countOf(g, "attackDeclared")).toBe(1);
    g.expectInZone("core-015", "graveyard");
    expect(unitsOf(g, "p1")).toEqual(["core-t-rush", "core-t-rush"]);
  });

  it("R53 the target still strikes back", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed()] },
      p2: { ...SPARE, hand: ["core-015", "core-005"] },
    });

    g.play("core-015", { zone: 1 });

    const first = g.unit("p1", 1);
    const second = g.unit("p1", 2);
    expect(first).not.toBeNull();
    expect(second).not.toBeNull();
    if (first === null || second === null) return;

    // 1 damage came back onto the attacker; the token that never attacked is untouched.
    g.expectStats(first, { health: 2, maxHealth: 3 });
    g.expectStats(second, { health: 3, maxHealth: 3 });
  });

  it("R53 a forced attack spends no exertion and ignores summoning sickness", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed()] },
      p2: { ...SPARE, hand: ["core-015", "core-005"] },
    });

    g.play("core-015", { zone: 1 });

    // The tokens were summoned this very turn and attacked anyway (sickness skipped), and the
    // attack cost them nothing, so their own turn is still ahead of them.
    const first = g.unit("p1", 1);
    expect(first?.exertion).toEqual({ attacked: false, switched: false });
  });

  it("R53 a token that dies to the strike-back ceases to exist (R11), and the run stops", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed()] },
      // Pointmaster is 7/2 with First Strike, so it kills the first token outright — but it is a
      // 2-cost card, so only the radiant face reaches it. Tempo Timmy is the 1-cost First Strike
      // unit: 3/3, it strikes first for 3 and the token deals nothing back (§4.3).
      p2: { ...SPARE, hand: ["core-011", "core-005"] },
    });

    g.play("core-011", { zone: 1 });

    const survivors = unitsOf(g, "p1");
    // Both tokens attack: each dies to First Strike without hurting Timmy, so Timmy stands.
    expect(countOf(g, "attackDeclared")).toBe(2);
    expect(survivors).toEqual([]);
    g.expectInZone("core-011", "field").expectStats("core-011", { health: 3, maxHealth: 3 });
  });

  it("a played Spell summons the tokens and nothing attacks", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed()] },
      p2: { ...SPARE, hand: ["core-005", "core-011"] },
    });

    g.play("core-005");

    expect(unitsOf(g, "p1")).toEqual(["core-t-rush", "core-t-rush"]);
    expect(countOf(g, "attackDeclared")).toBe(0);
  });
});

describe("#60 Bear Honeypot — radiant", () => {
  it("'Any card': a 3-cost play fires it", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed(true)] },
      // Mana Well is a 3-cost Field Spell: no cost threshold left, and not a Unit, so no attacks.
      p2: { ...SPARE, hand: ["core-006", "core-005"] },
    });

    g.play("core-006", { zone: 1 });

    g.expectEvents("cardPlayed", "trapFired", "summoned");
    expect(countOf(g, "attackDeclared")).toBe(0);
  });

  it("R64 fills your board: every empty unlocked unit zone, left to right", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed(true)] },
      p2: { ...SPARE, hand: ["core-006", "core-005"] },
    });

    g.play("core-006", { zone: 1 });

    expect(unitsOf(g, "p1")).toEqual(Array.from({ length: 5 }, () => "core-t-rush"));
    for (const lane of [1, 2, 3, 4, 5]) expect(g.unit("p1", lane)?.defId).toBe("core-t-rush");
  });

  it("R64 fills only the EMPTY zones, leaving what is already there", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed(true)], field: [{ def: "core-008", lane: 1 }] },
      p2: { ...SPARE, hand: ["core-006", "core-005"] },
    });

    g.play("core-006", { zone: 1 });

    expect(unitsOf(g, "p1")).toEqual([
      "core-008",
      "core-t-rush",
      "core-t-rush",
      "core-t-rush",
      "core-t-rush",
    ]);
  });

  it('§8 Conventions "same": a played Unit is still attacked, one combat at a time (R53)', () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed(true)] },
      // Big D-fender is a 2-cost 0/8: three 3-damage combats kill it, and R63 makes its 0-attack
      // strike-back no damage instance at all, so every token survives to take its turn in the run.
      p2: { ...SPARE, hand: ["core-001", "core-005"] },
    });

    g.play("core-001", { zone: 1 });

    // Combats 1 and 2 leave it at 2 health, combat 3 kills it, and tokens 4 and 5 never attack
    // because the target has left the field (R53's last clause).
    expect(countOf(g, "attackDeclared")).toBe(3);
    g.expectInZone("core-001", "graveyard");
    expect(unitsOf(g, "p1")).toEqual(Array.from({ length: 5 }, () => "core-t-rush"));
  });

  it("§4.3 First Strike on the target kills each forced attacker before it lands its hit", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed(true)] },
      // Pointmaster is a 2-cost 7/1 with First Strike: 7 into a 3-health token, and "if D is
      // destroyed here it deals nothing", so all five tokens die and Pointmaster stands at 1.
      p2: { ...SPARE, hand: ["core-020", "core-005"] },
    });

    g.play("core-020", { zone: 1 });

    expect(countOf(g, "attackDeclared")).toBe(5);
    expect(unitsOf(g, "p1")).toEqual([]);
    g.expectInZone("core-020", "field").expectStats("core-020", { health: 1, maxHealth: 1 });
  });

  it("R56 the radiant face keeps no threshold, but still only answers the opponent", () => {
    const g = scenario({
      p1: { ...SPARE, backrow: [armed(true)], hand: ["core-006", "core-005"] },
      p2: { ...SPARE },
    });

    g.play("core-006", { zone: 1 });

    expect(countOf(g, "trapFired")).toBe(0);
    g.expectInZone("core-060", "field");
  });

  it("§5.1 the radiant face is still a Trap, not a Field Trap: it is consumed", () => {
    const g = scenario({
      active: "p2",
      p1: { backrow: [armed(true)] },
      p2: { ...SPARE, hand: ["core-006", "core-005"] },
    });

    g.play("core-006", { zone: 1 });

    g.expectInZone("core-060", "graveyard");
  });
});
