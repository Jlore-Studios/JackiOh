// C+ #47 Jogg's Box — SPEC §8.7 row 47, BUILD M9 Classic+ row C+ 47: "Casts 10 random non-token Spells
// of any set but Jogg's Box (R380, R387) one after another, every target, mode and Discover pick random
// with X the current mana and at least 1, so it never opens a prompt; each cast is a play (R70); a cast
// with no legal target fizzles and the next goes; a Call to Chaos among them counts against
// `CALL_TO_CHAOS_CHAIN_CAP` (R28); a game that ends midway stops the rest; a fixed seed casts the same
// ten; the cast count reads through `param()` (step 2); radiant Echo 1 runs ten more with fresh random
// picks". The R28 count is proved again in packages/engine/test/effects-cast-chaos.test.ts.

import {
  CALL_TO_CHAOS_CHAIN_CAP,
  cardsPlayedThisTurn,
  defOf,
  hashState,
  reduce,
  registerCatalog,
  registerScripts,
  registeredScripts,
  stepParam,
  subsystems,
  type GameState,
  type Hook,
} from "@jackioh/engine";
import type { GameEvent, Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { CATALOG, CATALOG_VERSION } from "../../src/catalog-data";
import { CARDS } from "../../src/index";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/047-joggs-box";

const BOX = "classicplus-047";
const FILLER = "core-005";
const VANILLA = "core-008";
const CALLS = ["core-095", "classicplus-073"];

type Played = Extract<GameEvent, { type: "cardPlayed" }>;

function box(opts: { seed: string; radiant?: boolean; empty?: boolean; health?: number; library?: readonly string[] }): Scenario {
  return scenario({
    seed: opts.seed,
    p1: {
      hand: [{ def: BOX, ...(opts.radiant === true ? { radiant: true } : {}) }, FILLER],
      library: opts.library ?? [VANILLA, VANILLA, VANILLA],
      field: opts.empty === true ? [] : [VANILLA],
      ...(opts.health === undefined ? {} : { health: opts.health }),
    },
    p2: {
      hand: [FILLER, FILLER],
      field: opts.empty === true ? [] : [VANILLA],
      library: [VANILLA, VANILLA],
      ...(opts.health === undefined ? {} : { health: opts.health }),
    },
  });
}

/** The casts Jogg's Box made itself: plays begun while it alone was resolving (nested casts left out). */
function casts(events: readonly GameEvent[], boxId: string): Played[] {
  const open: string[] = [];
  const out: Played[] = [];
  for (const event of events) {
    if (event.type === "cardPlayed") {
      if (open.length === 1 && open[0] === boxId) out.push(event);
      open.push(event.instanceId);
    }
    if (event.type === "cardResolved") {
      const at = open.lastIndexOf(event.instanceId);
      if (at >= 0) open.splice(at, 1);
    }
  }
  return out;
}

/** Play the Box and answer every prompt the other player is asked with its first options. */
function open(s: Scenario): Played[] {
  const id = s.card(BOX).id;
  const from = s.events.length;
  s.play(BOX);
  for (let guard = 0; guard < 20 && s.state.pending !== null && s.state.result === null; guard += 1) {
    const pending = s.state.pending;
    s.answer(pending.options.slice(0, Math.max(1, pending.min)).map((option) => option.selection));
  }
  return casts(s.events.slice(from), id);
}

/** Seeds that cast all their Spells with no prompt and no end of the game. */
function quietSeed(make: (seed: string) => Scenario, want: number): { s: Scenario; cast: Played[] } {
  for (let i = 0; i < 60; i += 1) {
    const s = make(`jogg-quiet-${i}`);
    const id = s.card(BOX).id;
    const turn = s.state.turn;
    s.play(BOX);
    const cast = casts(s.lastEvents, id);
    // R82: a turn left with nothing to do ends itself, which would clear this turn's counts; keep a
    // seed whose turn is still going.
    const quiet = s.state.pending === null && s.state.result === null && s.state.turn === turn;
    if (quiet && cast.length === want) return { s, cast };
  }
  throw new Error("no quiet seed");
}

describe("C+ #47 Jogg's Box", () => {
  it("is a (4) Legendary Spell; the Radiant face adds Echo 1", () => {
    expect(def.cost).toBe(4);
    expect(def.rarity).toBe("Legendary");
    expect(base.staticFlags).toBeUndefined();
    expect(radiant.staticFlags).toEqual({ echo: 1 });
    expect(radiant.cry).toBe(base.cry);
  });

  describe("base", () => {
    it("R452 R70 it casts 10 random Spells one after another, each a free play landing with you (R87)", () => {
      const { s, cast } = quietSeed((seed) => box({ seed }), 10);
      expect(cast).toHaveLength(10);
      for (const played of cast) {
        expect(played.player).toBe("p1");
        expect(played.costPaid).toBe(0);
        const card = s.card(played.instanceId);
        expect(card.owner).toBe("p1");
        expect(card.zone.z).not.toBe("resolving");
      }
      expect(cardsPlayedThisTurn(s.state, "p1")).toBeGreaterThanOrEqual(11);
      s.expectInZone(BOX, "graveyard");
    });

    it("R380 R387 over many seeds: non-token Spells of every set, never Jogg's Box", () => {
      const seen = new Set<string>();
      for (let i = 0; i < 25; i += 1) {
        for (const played of open(box({ seed: `jogg-pool-${i}` }))) seen.add(played.defId);
      }
      const state = box({ seed: "pool" }).state;
      const defs = [...seen].map((id) => defOf(state, id));
      expect(seen.has(BOX)).toBe(false);
      expect(defs.every((entry) => entry.type === "Spell" && !entry.token)).toBe(true);
      expect(new Set(defs.map((entry) => entry.set))).toEqual(new Set(["Core", "Classic", "Classic+"]));
    });

    it("R387 with Jogg's Box and Fig of Life the only Spells, all ten casts are Fig of Life", () => {
      const FIG = "core-047"; // (3) Spell: heal a target 20
      const spells = Object.entries(CATALOG).filter(([id, entry]) => entry.type !== "Spell" || id === BOX || id === FIG);
      try {
        registerCatalog(Object.fromEntries(spells), CATALOG_VERSION);
        const s = scenario({
          seed: "jogg-only-fig",
          p1: { hand: [BOX, VANILLA], field: [VANILLA], library: [VANILLA] },
          p2: { hand: [VANILLA], field: [VANILLA], library: [VANILLA] },
        });
        const id = s.card(BOX).id;
        s.play(BOX);
        const cast = casts(s.lastEvents, id).map((played) => played.defId);
        expect(cast).toEqual(Array.from({ length: 10 }, () => FIG));
      } finally {
        registerCatalog(CATALOG, CATALOG_VERSION);
      }
    });

    it("R452 R471 a cast Book of Plague (C #70) places its tokens at random: its caster is never asked", () => {
      const PLAGUE_BOOK = "classic-070"; // (1) Spell: "Place {tokens} Plague Counters."
      const spells = Object.entries(CATALOG).filter(([id, entry]) => entry.type !== "Spell" || id === BOX || id === PLAGUE_BOOK);
      try {
        registerCatalog(Object.fromEntries(spells), CATALOG_VERSION);
        const s = scenario({
          seed: "jogg-only-plague",
          p1: { hand: [BOX, VANILLA], field: [VANILLA], library: [VANILLA] },
          p2: { hand: [VANILLA], field: [VANILLA], library: [VANILLA] },
        });
        s.play(BOX);
        expect(s.state.pending).toBeNull();
        const placed = [s.unit("p1", 1), s.unit("p2", 1)].reduce((sum, unit) => sum + (unit?.counters.plague ?? 0), 0);
        expect(placed).toBeGreaterThanOrEqual(10);
      } finally {
        registerCatalog(CATALOG, CATALOG_VERSION);
      }
    });

    it("§6.2 R452 every choice is random: its caster is never asked", () => {
      for (let i = 0; i < 25; i += 1) {
        const s = box({ seed: `jogg-ask-${i}` });
        s.play(BOX);
        for (let guard = 0; guard < 20 && s.state.pending !== null; guard += 1) {
          expect(s.state.pending.playerId).toBe("p2");
          const pending = s.state.pending;
          s.answer(pending.options.slice(0, Math.max(1, pending.min)).map((option) => option.selection));
        }
      }
    });

    it("R453 an X Spell it casts takes your current mana as X, at least 1", () => {
      let checked = 0;
      for (let i = 0; i < 60 && checked < 3; i += 1) {
        const s = box({ seed: `jogg-x-${i}` });
        const id = s.card(BOX).id;
        s.play(BOX);
        const top = new Set(casts(s.lastEvents, id).map((played) => played.instanceId));
        let mana = s.state.players.p1.mana.current;
        let paid = false;
        for (const event of s.lastEvents) {
          if (event.type === "manaChanged" && event.player === "p1") {
            mana = event.current;
            paid = true;
          }
          if (event.type !== "cardPlayed" || !top.has(event.instanceId) || event.x === undefined) continue;
          expect(paid).toBe(true);
          expect(event.x).toBe(Math.max(1, mana));
          checked += 1;
        }
      }
      expect(checked).toBeGreaterThan(0);
    });

    it("a cast with no legal target fizzles and the next goes", () => {
      const unitOnly = (id: string): boolean =>
        (CARDS[id]?.base.targets ?? []).some((decl) => decl.kind === "target" && decl.filter?.of?.join() === "unit");
      for (let i = 0; i < 80; i += 1) {
        const s = box({ seed: `jogg-fizzle-${i}`, empty: true });
        const id = s.card(BOX).id;
        s.play(BOX);
        const cast = casts(s.lastEvents, id);
        if (s.state.pending !== null || s.state.result !== null || !unitOnly(cast[0]?.defId ?? "")) continue;
        expect(cast).toHaveLength(10);
        return;
      }
      throw new Error("no seed opened on a Unit-only target with an empty board");
    });

    it("R593 R28 a Call to Chaos among them is the first cast of its chain, which stops at CALL_TO_CHAOS_CHAIN_CAP", () => {
      const saved = registeredScripts();
      // Each real cast's depth by instance: a Zephyrs-style scorer among the casts runs a Call's Cry on a
      // simulated state too, and those runs are no cast of this game (R29).
      const runs: { id: string; depth: number }[] = [];
      const recurse: Hook = (ctx) => {
        runs.push({ id: ctx.self?.id ?? "", depth: subsystems.chaosChainOf(ctx.self) });
        return [subsystems.castRandomCallToChaos()];
      };
      registerScripts({ ...saved, ...Object.fromEntries(CALLS.map((id) => [id, { base: { cry: recurse }, radiant: { cry: recurse } }])) });
      try {
        for (let i = 0; i < 80; i += 1) {
          runs.length = 0;
          const s = box({ seed: `jogg-chaos-${i}` });
          s.play(BOX);
          const played = new Set(s.lastEvents.flatMap((event) => (event.type === "cardPlayed" ? [event.instanceId] : [])));
          const depths = runs.filter((run) => played.has(run.id)).map((run) => run.depth);
          if (depths.length === 0) continue;
          // Links 1 to the cap, then nothing more of that chain (a later Call the Box casts starts at 1).
          expect(depths.slice(0, CALL_TO_CHAOS_CHAIN_CAP)).toEqual(Array.from({ length: CALL_TO_CHAOS_CHAIN_CAP }, (_, n) => n + 1));
          expect(depths[CALL_TO_CHAOS_CHAIN_CAP] ?? 1).toBe(1);
          return;
        }
        throw new Error("no seed cast a Call to Chaos");
      } finally {
        registerScripts(saved);
      }
    });

    it("§2.5 a game that ends midway stops the rest", () => {
      for (let i = 0; i < 60; i += 1) {
        const s = box({ seed: `jogg-end-${i}`, health: 1, library: [] });
        const id = s.card(BOX).id;
        s.play(BOX);
        const over = s.lastEvents.findIndex((event) => event.type === "gameOver");
        // A game ended by the tenth cast has nothing left to stop: look for one that ends sooner.
        if (over < 0 || casts(s.lastEvents, id).length >= 10) continue;
        expect(s.state.result).not.toBeNull();
        expect(casts(s.lastEvents, id).length).toBeLessThan(10);
        expect(s.lastEvents.slice(over + 1).some((event) => event.type === "cardPlayed" || event.type === "cardAnnounced")).toBe(false);
        return;
      }
      throw new Error("no seed ended the game");
    });

    it("§9.3 a fixed seed casts the same ten", () => {
      const a = box({ seed: "jogg-same" });
      const b = box({ seed: "jogg-same" });
      expect(open(a).map((played) => played.defId)).toEqual(open(b).map((played) => played.defId));
      expect(hashState(a.state)).toBe(hashState(b.state));
    });

    it("R113 another player's prompt pauses the rest; answered after a JSON round trip, the rest follow", () => {
      for (let i = 0; i < 60; i += 1) {
        const s = box({ seed: `jogg-pause-${i}` });
        const id = s.card(BOX).id;
        const from = s.events.length;
        s.play(BOX);
        const pending = s.state.pending;
        if (pending === null || s.state.result !== null) continue;
        expect(pending.playerId).toBe("p2");
        expect(casts(s.lastEvents, id).length).toBeLessThan(10);

        const revived = JSON.parse(JSON.stringify(s.state)) as GameState;
        expect(revived).toEqual(s.state);
        const picks: Selection[] = pending.options.slice(0, Math.max(1, pending.min)).map((option) => option.selection);
        const resumed = reduce(revived, { type: "answer", playerId: "p2", choiceId: pending.id, selection: picks, nonce: "jogg-pause" });
        expect(resumed.error).toBeUndefined();
        s.answer(picks);
        expect(hashState(resumed.state)).toBe(hashState(s.state));

        for (let guard = 0; guard < 20 && s.state.pending !== null; guard += 1) {
          const next = s.state.pending;
          s.answer(next.options.slice(0, Math.max(1, next.min)).map((option) => option.selection));
        }
        if (s.state.result === null) expect(casts(s.events.slice(from), id)).toHaveLength(10);
        return;
      }
      throw new Error("no seed paused on the other player's prompt");
    });

    it("R386 an Upgrade casts 12 (step 2); a Degrade 8", () => {
      const up = quietSeed((seed) => {
        const s = box({ seed });
        stepParam(s.card(BOX), "casts", 1);
        return s;
      }, 12);
      expect(up.cast).toHaveLength(12);

      const down = quietSeed((seed) => {
        const s = box({ seed });
        stepParam(s.card(BOX), "casts", -1);
        return s;
      }, 8);
      expect(down.cast).toHaveLength(8);
    });
  });

  describe("radiant", () => {
    it("R30 Echo 1 runs ten more with fresh random picks: twenty casts", () => {
      const { cast } = quietSeed((seed) => box({ seed, radiant: true }), 20);
      expect(cast).toHaveLength(20);
      const first = cast.slice(0, 10).map((played) => played.defId);
      const second = cast.slice(10).map((played) => played.defId);
      expect(second).not.toEqual(first);
      expect(cast.some((played) => played.defId === BOX)).toBe(false);
    });

    it("R386 an Upgrade casts 12 on each run", () => {
      const { cast } = quietSeed((seed) => {
        const s = box({ seed, radiant: true });
        stepParam(s.card(BOX), "casts", 1);
        return s;
      }, 24);
      expect(cast).toHaveLength(24);
    });
  });
});
