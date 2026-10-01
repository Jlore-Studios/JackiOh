// #67 Zoomerbin Oomen — SPEC §8.3, BUILD M4-T4: "Random trap face-down and unpaid into own lane's
// backrow; occupied or Locked → nothing (R47); pool = the five Cost (1) traps, radiant all six".
//
// §8.3's row: "Cry: summon a random Cost (1) Trap face-down into your backrow zone in this lane" →
// "A random Radiant Trap" (R275). Patch v0.1.1 made #85 Unlicensed Experimentation cost 2, so the
// base pool is #18, #41, #60, #71, #96 — and since patch v0.2.0's one format (R380) Classic+ #22 —
// and the radiant face's is every trap of every set, #85 included, summoned Radiant; zone occupied or Locked → fizzles. A Radiant face-down trap is still hidden from the opponent, face and all (R33,
// R97, R177).
//
// §3.1 fixes what "this lane" means: "the backrow zone in the same column as the unit". §8's
// Conventions fix the fizzle: the Cry does nothing and "the unit still enters". R1 fixes the
// unpaid, dormant arrival: a summon fires no Cry and pays no mana.

import { describe, expect, it } from "vitest";
import { scenario, type ScenarioOptions } from "./_harness";
import { base, radiant } from "../src/scripts/067-zoomerbin-oomen";
import { TRAP_TYPES, catalog } from "../src/query";

const OOMEN = "core-067"; // Unit 1/2 → 2/4, cost 1, Human.
const MANA_WELL = "core-006"; // A Field Spell: something to occupy a backrow zone with.
const SIPHON_SQUAD = "classic-088"; // Field Trap that Tributes itself while the opponent has no Units (R403).

/**
 * The base face's pool, by catalog id: the Cost (1) traps of every set (R380) — Core #18, #41, #60,
 * #71, #96 and Classic+ #22 Blood Moon, the one new Cost (1) Trap (docs/classic-sets.md B2.6).
 */
const TRAP_POOL = ["core-018", "core-041", "core-060", "core-071", "core-096", "classicplus-022"];
/** The radiant face's pool: every trap of every set, Core #85 (Cost (2)) included. */
const RADIANT_TRAP_POOL = catalog.query({ type: TRAP_TYPES }).map((def) => def.id);

type Board = ReturnType<typeof scenario>;

/**
 * R82: a turn whose only legal actions are ending it, conceding and offering a draw auto-ends by
 * itself, and `reduce` runs that check after EVERY action — so a play that empties the hand and
 * leaves no unit hands the turn over: the opponent draws (taking fatigue on an empty library),
 * start-of-turn triggers fire, and the numbers under test move underneath the assertion. Every
 * scenario below therefore keeps one free 0-cost Spell in p1's hand. It is never played; it only
 * keeps one legal action on the turn. (Reported as a harness gap: `scenario` could hold the turn
 * open by itself.)
 */
const ANCHOR = "core-010"; // Rapid Replenish, Spell, cost 0 — always an affordable play.

function board(opts: ScenarioOptions = {}): Board {
  const p1 = opts.p1 ?? {};
  return scenario({ ...opts, p1: { ...p1, hand: [...(p1.hand ?? []), ANCHOR] } });
}

const LANE = 3;

/** Every backrow zone p1 holds, by catalog id, with `null` for an empty one. */
function backrowIds(s: Board): (string | null)[] {
  return [1, 2, 3, 4, 5].map((lane) => s.backrow("p1", lane)?.defId ?? null);
}

describe("#67 Zoomerbin Oomen", () => {
  // -------------------------------------------------------------------------------------------
  // The pool (§5.1, R60)
  // -------------------------------------------------------------------------------------------

  it("BUILD row 67, R380 the base pool is every set's Cost (1) traps, and the radiant pool every trap", () => {
    const ids = (defs: { id: string }[]): string[] => defs.map((entry) => entry.id);
    // The base face asks for Cost (1) traps, the radiant face for any trap. Field Trap counts as
    // Trap (§8 #51, R35, R61), and #85 costs 2, so only the radiant query reaches it.
    expect(ids(catalog.query({ type: TRAP_TYPES, cost: 1 }))).toEqual(TRAP_POOL);
    const everyTrap = ids(catalog.query({ type: TRAP_TYPES }));
    for (const id of [...TRAP_POOL, "core-085"]) expect(everyTrap).toContain(id);
    expect(everyTrap.some((id) => id.startsWith("classic-"))).toBe(true);
    expect(everyTrap.every((id) => catalog.query({ defId: id })[0]?.type.includes("Trap"))).toBe(true);
  });

  it("R81 the Cry asks for nothing: the lane is the unit's own, not a declared pick", () => {
    expect(base.targets).toBeUndefined();
    expect(base.modes).toBeUndefined();
    expect(radiant.targets).toBeUndefined();
    expect(typeof base.cry).toBe("function");
    expect(typeof radiant.cry).toBe("function");
  });

  // -------------------------------------------------------------------------------------------
  // Base: the summon (§3.1, §3.2, R1, R33)
  // -------------------------------------------------------------------------------------------

  it("§8.3 summons a trap from the pool into the unit's OWN lane's backrow zone", () => {
    const s = board({ p1: { hand: [OOMEN] } });
    s.play(OOMEN, { zone: LANE });

    const trap = s.backrow("p1", LANE);
    expect(trap).not.toBeNull();
    expect(TRAP_POOL).toContain(trap?.defId);
    // §3.1: only that column, never the leftmost free zone (R64's fallback is not what this says).
    expect(backrowIds(s)).toEqual([null, null, trap?.defId ?? null, null, null]);
  });

  it("§3.2/R33 the trap arrives face-down and not Radiant, and R1 leaves it unpaid", () => {
    const s = board({ p1: { hand: [OOMEN] } });
    s.play(OOMEN, { zone: LANE });

    expect(s.backrow("p1", LANE)?.faceUp).not.toBe(true);
    // The base face makes an ordinary trap: "Radiant" is the radiant face's word (R275).
    expect(s.backrow("p1", LANE)?.radiant).toBe(false);
    // Only Oomen's own cost of 1 was paid: a summon pays nothing (R1, §6.3 Summon).
    s.expectMana("p1", 3).expectEvents("cardPlayed", "summoned", "summoned");
  });

  it("R60 the seeded pick stays inside the pool and reaches every member of it", () => {
    const seen = new Set<string>();
    for (let seed = 0; seed < 40; seed += 1) {
      const s = board({ seed: `oomen-${seed}`, p1: { hand: [OOMEN] } });
      s.play(OOMEN, { zone: LANE });
      const trap = s.backrow("p1", LANE);
      expect(trap).not.toBeNull();
      expect(TRAP_POOL).toContain(trap?.defId);
      if (trap !== null) seen.add(trap.defId);
    }
    expect([...seen].sort()).toEqual([...TRAP_POOL].sort());
  });

  it("R60 the same seed gives the same trap: the pick replays (§9.3)", () => {
    const pick = (): string | undefined => {
      const s = board({ seed: "oomen-replay", p1: { hand: [OOMEN] } });
      s.play(OOMEN, { zone: LANE });
      return s.backrow("p1", LANE)?.defId;
    };
    expect(pick()).toBe(pick());
  });

  // -------------------------------------------------------------------------------------------
  // Base: the fizzle (R47, §8 Conventions)
  // -------------------------------------------------------------------------------------------

  it("R47 an occupied backrow zone fizzles the summon, and the unit still enters", () => {
    const s = board({ p1: { hand: [OOMEN], backrow: [{ def: MANA_WELL, lane: LANE }] } });
    s.play(OOMEN, { zone: LANE });

    s.expectInZone(OOMEN, "field");
    // Nothing moved, nothing spilled into another lane (§3.1: this card names one zone only).
    expect(backrowIds(s)).toEqual([null, null, MANA_WELL, null, null]);
  });

  it("R47 a Locked backrow zone fizzles the summon, and the unit still enters", () => {
    const s = board({ p1: { hand: [OOMEN] } });
    // §3.2 Lock is a zone flag. The harness exposes no way to lock a zone (reported as a harness
    // gap: `SideSetup.locks` or `s.lock(player, row, lane)`), and #36 Magic Jammed only locks the
    // zone of a backrow card it destroys, so the flag is set here directly — a test-only liberty.
    s.state.players.p1.locks.backrow[LANE - 1] = true;
    s.play(OOMEN, { zone: LANE });

    s.expectInZone(OOMEN, "field");
    expect(backrowIds(s)).toEqual([null, null, null, null, null]);
  });

  it("§3.1 lane 1 and lane 5 are read as the unit's own column too", () => {
    for (const lane of [1, 5]) {
      const s = board({ p1: { hand: [OOMEN] } });
      s.play(OOMEN, { zone: lane });
      expect(s.backrow("p1", lane)).not.toBeNull();
      expect(TRAP_POOL).toContain(s.backrow("p1", lane)?.defId);
    }
  });

  it("§3.1 the trap never lands on the opponent's side ('YOUR backrow zone')", () => {
    const s = board({ p1: { hand: [OOMEN] } });
    s.play(OOMEN, { zone: LANE });
    expect([1, 2, 3, 4, 5].map((lane) => s.backrow("p2", lane))).toEqual([null, null, null, null, null]);
  });

  // -------------------------------------------------------------------------------------------
  // Radiant: "a random Radiant Trap" (§8 Conventions, R275)
  // -------------------------------------------------------------------------------------------

  it("R275 the radiant face is 2/4 and summons a Radiant trap into its own lane, face-down", () => {
    const s = board({ p1: { hand: [{ def: OOMEN, radiant: true }] } });
    s.play(OOMEN, { zone: LANE });

    s.expectStats(OOMEN, { attack: 2, health: 4, maxHealth: 4 });
    const trap = s.backrow("p1", LANE);
    expect(trap).not.toBeNull();
    expect(RADIANT_TRAP_POOL).toContain(trap?.defId);
    expect(trap?.radiant).toBe(true);
    expect(trap?.faceUp).not.toBe(true);
    expect(backrowIds(s)).toEqual([null, null, trap?.defId ?? null, null, null]);
    // Still a summon: only Oomen's own cost was paid (R1).
    s.expectMana("p1", 3);
  });

  it("R33, R97 the Radiant trap is hidden from the opponent, face and all; its controller reads it", () => {
    const s = board({ seed: "oomen-radiant-hidden", p1: { hand: [{ def: OOMEN, radiant: true }] } });
    s.play(OOMEN, { zone: LANE });
    const trap = s.backrow("p1", LANE);
    if (trap === null) throw new Error("the Radiant Oomen should have summoned a trap");

    // The opponent is told the zone is occupied and what its back shows (R351), nothing more (§10.8).
    const theirs = s.view("p2");
    const shownCost = s.view("p1").you.backrow[LANE - 1]?.cost;
    expect(theirs.opponent.backrow[LANE - 1]).toEqual({ faceDown: true, cost: shownCost });
    // Nowhere in their view — the board, the events, a prompt — is the card named or its face shown.
    const serialized = JSON.stringify(theirs);
    expect(serialized).not.toContain(`"${trap.id}"`);
    expect(serialized).not.toContain(`"${trap.defId}"`);

    // Its controller reads it, Radiant face included (R33).
    const mine = s.view("p1").you.backrow[LANE - 1];
    expect(mine).toMatchObject({ faceDown: false, defId: trap.defId, radiant: true });
  });

  it("§8.3 the radiant pool drops the cost clause: every pick is a trap of any set, each Radiant", () => {
    const seen = new Set<string>();
    for (let seed = 0; seed < 40; seed += 1) {
      const s = board({ seed: `oomen-radiant-${seed}`, p1: { hand: [{ def: OOMEN, radiant: true }] } });
      s.play(OOMEN, { zone: LANE });
      // The pick is read off its `summoned` event, not off lane 3's zone: R403 makes C #88 Siphon
      // Squad live from the moment it is set, its self-Tribute included, and p2 controls no Units
      // here, so the state check right after the summon Tributes it. It still came, Radiant.
      const picks = s.events.flatMap((e) => (e.type === "summoned" && e.row === "backrow" ? [e.instanceId] : []));
      expect(picks).toHaveLength(1);
      const trap = s.card(picks[0] ?? "");
      expect(trap.radiant).toBe(true);
      expect(RADIANT_TRAP_POOL).toContain(trap.defId);
      // Every other pick stays where §3.1 put it.
      if (trap.defId === SIPHON_SQUAD) s.expectInZone(trap.id, "graveyard");
      else expect(s.backrow("p1", LANE)?.id).toBe(trap.id);
      seen.add(trap.defId);
    }
    // Forty seeds over a pool of every set's traps reach beyond the Cost (1) ones.
    expect([...seen].some((id) => !TRAP_POOL.includes(id))).toBe(true);
    expect(seen.size).toBeGreaterThan(TRAP_POOL.length);
  });

  it("R47 the radiant face fizzles on an occupied zone, and the unit still enters", () => {
    const s = board({ p1: { hand: [{ def: OOMEN, radiant: true }], backrow: [{ def: MANA_WELL, lane: LANE }] } });
    s.play(OOMEN, { zone: LANE });

    s.expectInZone(OOMEN, "field");
    expect(backrowIds(s)).toEqual([null, null, MANA_WELL, null, null]);
    // The Field Spell already there is not made Radiant: the summon made nothing.
    expect(s.backrow("p1", LANE)?.radiant).toBe(false);
  });

  it("R47 the radiant face fizzles on a Locked zone too, and the unit still enters", () => {
    const s = board({ p1: { hand: [{ def: OOMEN, radiant: true }] } });
    s.state.players.p1.locks.backrow[LANE - 1] = true;
    s.play(OOMEN, { zone: LANE });

    s.expectInZone(OOMEN, "field").expectStats(OOMEN, { attack: 2, health: 4 });
    expect(backrowIds(s)).toEqual([null, null, null, null, null]);
  });
});
