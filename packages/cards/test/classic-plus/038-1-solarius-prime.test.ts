// C+ #38.1 Solarius-Prime — SPEC §8.7 row 38.1, BUILD M9 Classic+ row C+ 38.1: "Spell Damage +3; Cry
// casts 5 random non-token Spells of any set one after another (R380), every choice random with no
// prompt, each target aimed by its declaration (R654: harm at enemies, help at friends), X the current
// mana and at least 1 (R348), each
// cast a play (R70) whose hits its own Spell Damage raises (it is on the field during its Cry); a cast
// with no legal target fizzles and the next goes; a game that ends midway stops the rest; the casts are
// public, what they add to your hand hidden (R97); Spell Damage and casts read through `param()`;
// radiant Spell Damage +7 and the Spells are Radiant".
//
// The casts are random, so these tests read their shape off the event stream rather than naming the
// Spells: a cast is a `cardAnnounced` by its caster at cost 0, and the Prime's own casts are the ones
// announced while only the Prime itself is still resolving (a Spell that casts more, a Call to Chaos,
// opens a deeper level until its own `cardResolved`).

import { defOf, stepParam } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";

const PRIME = "classicplus-038-1";
const MENACE = "core-019";
const FILLER = "core-005";
const SEEDS = Array.from({ length: 12 }, (_, i) => `prime-${i + 1}`);

type Announce = Extract<GameEvent, { type: "cardAnnounced" }>;

/** Everything announced during the Prime's play, each with how deep in nested casts it was made. */
function castsDuring(s: Scenario, primeId: string): { event: Announce; depth: number }[] {
  const open: string[] = [];
  const out: { event: Announce; depth: number }[] = [];
  let inside = false;
  for (const event of s.events) {
    if (event.type === "cardAnnounced") {
      if (event.instanceId === primeId) inside = true;
      else if (inside) out.push({ event, depth: open.length });
      if (inside) open.push(event.instanceId);
    } else if ((event.type === "cardResolved" || event.type === "countered") && inside) {
      const at = open.lastIndexOf(event.instanceId);
      if (at >= 0) open.splice(at, 1);
      if (event.instanceId === primeId) inside = false;
    }
  }
  return out;
}

/**
 * What each of the Prime's casts dealt the enemy hero in all, by cast: a Spell's damage is raised by
 * the Spell Damage on its side, a split's total once (each of its hits is 1) and a single hit whole.
 */
function heroDamageByCast(s: Scenario, prime: string): number[] {
  const casts = new Set(castsDuring(s, prime).map((cast) => cast.event.instanceId));
  const totals = new Map<string, number>();
  // Only while the Prime stands as it was played: the first event that names it after its own play
  // (a destroy, a bounce, a Transform, a Vanilla, a keyword change …) ends what its Spell Damage covers.
  const own = new Set(["cardAnnounced", "cardPlayed", "summoned", "cardResolved"]);
  const changed = s.events.findIndex((event) => "instanceId" in event && event.instanceId === prime && !own.has(event.type));
  for (const event of changed < 0 ? s.events : s.events.slice(0, changed)) {
    if (event.type !== "damage" || event.sourceId === null || !casts.has(event.sourceId)) continue;
    if (event.targetId !== "hero-p2") continue;
    totals.set(event.sourceId, (totals.get(event.sourceId) ?? 0) + event.amount);
  }
  return [...totals.values()];
}

/** Play a Prime; returns the game and the Prime's id (a token: a Flood it casts can end it, R11). */
function playPrime(
  seed: string,
  opts: { radiant?: boolean; p2Health?: number; casts?: number } = {},
): { s: Scenario; prime: string } {
  const s = scenario({
    seed,
    p1: {
      hand: [{ def: PRIME, radiant: opts.radiant === true }, FILLER],
      field: [MENACE],
      library: [FILLER, FILLER, FILLER, FILLER, FILLER],
    },
    p2: {
      hand: [FILLER],
      field: [MENACE],
      library: [FILLER, FILLER, FILLER],
      ...(opts.p2Health === undefined ? {} : { health: opts.p2Health }),
    },
  });
  const prime = s.card(PRIME).id;
  if (opts.casts !== undefined) stepParam(s.card(PRIME), "casts", opts.casts);
  s.play(PRIME);
  // A cast Spell may hand the opponent a choice of their own (C #8 Pickle's modes): theirs to make,
  // never answered at random (R452 drives only the caster's). Answer it so the casts can go on.
  for (let guard = 0; s.state.pending !== null && s.state.pending.playerId === "p2" && guard < 20; guard += 1) {
    const pending = s.state.pending;
    s.answer(pending.options.slice(0, Math.max(1, pending.min)).map((option) => option.key));
  }
  return { s, prime };
}

describe("C+ #38.1 Solarius-Prime", () => {
  describe("base", () => {
    it("prints Spell Damage +3", () => {
      const s = scenario({ p1: { field: [PRIME] } });
      expect(s.stats(PRIME).keywords).toContainEqual({ kind: "Spell Damage", n: 3 });
    });

    it("R70 R452 its Cry casts exactly 5 Spells, one after another, free, with no prompt of its caster's", () => {
      for (const seed of SEEDS) {
        const { s, prime } = playPrime(seed);
        const own = castsDuring(s, prime).filter((cast) => cast.depth === 1);
        expect(own, seed).toHaveLength(5);
        for (const { event } of own) {
          expect(event.player).toBe("p1");
          expect(event.costPaid).toBe(0);
          expect(event.cardType).toBe("Spell");
          expect(defOf(s.state, event.defId).token).toBe(false);
        }
        expect(s.state.pending, seed).toBeNull();
      }
    });

    it("R380 R387 the Spells come from every set, never a token and never itself", () => {
      const sets = new Set<string>();
      for (const seed of SEEDS) {
        const { s, prime } = playPrime(seed);
        for (const { event } of castsDuring(s, prime)) {
          expect(event.defId).not.toBe(PRIME);
          sets.add(defOf(s.state, event.defId).set);
        }
      }
      expect([...sets].sort()).toEqual(["Classic", "Classic+", "Core"]);
    });

    it("R654 each pick aims by its declaration: harm never at your hero, help never at an enemy", () => {
      // The helpful Spells a random cast can pick: their target declarations aim "help".
      const HELP = new Set(["classic-003", "classicplus-010", "classicplus-057", "classicplus-071", "core-047", "core-063"]);
      let aimed = 0;
      for (const seed of SEEDS) {
        const { s, prime } = playPrime(seed);
        const mine = new Set(["hero-p1", prime, ...s.pile("p1", "graveyard").map((card) => card.id)]);
        const foe = new Set([
          "hero-p2",
          ...s.state.players.p2.units.flatMap((pile) => pile ?? []).map((card) => card.id),
          ...s.pile("p2", "graveyard").map((card) => card.id),
        ]);
        for (const { event } of castsDuring(s, prime)) {
          for (const target of event.targets) {
            if (HELP.has(event.defId)) {
              expect(foe.has(target), `${seed} ${event.defId}`).toBe(false);
            } else {
              expect(target === "hero-p1", `${seed} ${event.defId}`).toBe(false);
            }
            if (!mine.has(target)) aimed += 1;
          }
        }
      }
      expect(aimed).toBeGreaterThan(0);
    });

    it("R70 each cast is a play: the turn's log counts all of them", () => {
      const { s, prime } = playPrime(SEEDS[0] ?? "");
      const casts = castsDuring(s, prime);
      const played = s.state.players.p1.turnLog.playedIds;
      for (const { event } of casts) expect(played).toContain(event.instanceId);
    });

    it("its own Spell Damage raises the casts' hits: each cast deals the enemy hero at least 4 in all", () => {
      let checked = 0;
      for (const seed of SEEDS) {
        const { s, prime } = playPrime(seed);
        for (const total of heroDamageByCast(s, prime)) {
          expect(total, seed).toBeGreaterThanOrEqual(4);
          checked += 1;
        }
      }
      expect(checked).toBeGreaterThan(0);
    });

    it("a game that ends midway stops the rest: nothing is announced after the game is over", () => {
      let ended = 0;
      for (const seed of SEEDS) {
        const { s } = playPrime(seed, { p2Health: 1 });
        const over = s.events.findIndex((event) => event.type === "gameOver");
        if (over < 0) continue;
        ended += 1;
        expect(s.events.slice(over).some((event) => event.type === "cardAnnounced"), seed).toBe(false);
        expect(s.state.result?.winner).toBe("p1");
      }
      expect(ended).toBeGreaterThan(0);
    });

    it("R97 the casts are public; a card a cast adds to your hand is hidden from the opponent", () => {
      for (const seed of SEEDS) {
        const { s, prime } = playPrime(seed);
        const theirs = s.view("p2");
        // A cast is public: its announce names it, unless the card now lies where p2 may not read it.
        const hand = s.hand("p1").map((card) => card.id);
        const unread = new Set([...hand, ...s.pile("p1", "library").map((card) => card.id)]);
        const casts = new Set(castsDuring(s, prime).map((cast) => cast.event.instanceId));
        for (const event of theirs.events) {
          if (event.type === "cardAnnounced" && casts.has(event.instanceId) && !unread.has(event.instanceId)) {
            expect(event.defId, seed).not.toBe("hidden");
          }
        }
        for (const event of theirs.events) {
          if (event.type === "addedToHand" || event.type === "drawn") {
            if (hand.includes(event.instanceId)) expect(event.defId, seed).not.toBe(s.card(event.instanceId).defId);
          }
        }
        expect(Array.isArray(theirs.opponent.hand)).toBe(false);
      }
    });

    it("R386 the casts read through param(): a Degrade makes 4", () => {
      const { s, prime } = playPrime(SEEDS[1] ?? "", { casts: -1 });
      expect(castsDuring(s, prime).filter((cast) => cast.depth === 1)).toHaveLength(4);
    });
  });

  describe("radiant", () => {
    it("prints Spell Damage +7", () => {
      const s = scenario({ p1: { field: [{ def: PRIME, radiant: true }] } });
      expect(s.stats(PRIME).keywords).toContainEqual({ kind: "Spell Damage", n: 7 });
    });

    it("casts 5 Radiant Spells", () => {
      for (const seed of SEEDS.slice(0, 6)) {
        const { s, prime } = playPrime(seed, { radiant: true });
        const own = castsDuring(s, prime).filter((cast) => cast.depth === 1);
        expect(own, seed).toHaveLength(5);
        for (const { event } of own) {
          // Read off its resolution, since a cast may since have ceased to exist (a Transform).
          const resolved = s.events.find((e) => e.type === "cardResolved" && e.instanceId === event.instanceId);
          expect(resolved?.type === "cardResolved" && resolved.radiant === true, `${seed} ${event.defId}`).toBe(true);
        }
      }
    });

    it("its Spell Damage +7 raises the casts: each deals the enemy hero at least 8 in all", () => {
      let checked = 0;
      for (const seed of SEEDS) {
        const { s, prime } = playPrime(seed, { radiant: true });
        for (const total of heroDamageByCast(s, prime)) {
          expect(total, seed).toBeGreaterThanOrEqual(8);
          checked += 1;
        }
      }
      expect(checked).toBeGreaterThan(0);
    });
  });
});
