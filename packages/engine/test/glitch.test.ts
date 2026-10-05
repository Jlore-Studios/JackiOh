// The Glitch Easter egg in the engine (SPEC §7, R661–R664): the System plays counted, the Glitch check
// in the one generation helper, a hidden card kept out of every pool, the price nothing moves and the
// outcome rolled. Through the fixtures in fixtures/glitch.ts; the real Glitch's test
// (packages/cards/test/classic/t-glitch-glitch.test.ts) covers the card again.

import type { CardDef } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { findDef, glitchReplaces, pickGenerated, query } from "../src/catalog";
import { GLITCH_DEF_ID, GLITCH_ODDS_DENOMINATOR, GLITCH_ODDS_PER_SYSTEM_PLAY } from "../src/config";
import { GLITCH_OUTCOMES, rollGlitchOutcome } from "../src/effects";
import { effectiveCost } from "../src/mana";
import { numbersOn, ownCost } from "../src/numbers";
import { recordPlay } from "../src/playCounts";
import { legalActions } from "../src/reduce";
import { hashState } from "../src/replay";
import { createRng, type Rng } from "../src/rng";
import { newInstance, type GameState } from "../src/state";
import { act, frozen, handCard, playing, refusal, replayed, type Run } from "./fixtures/generation";
import {
  ADDER,
  GLITCHFX_ADDED,
  PLAIN,
  SHUFFLER,
  SUMMONER,
  SYSTEM_18,
  SYSTEM_25,
  TRANSFORMER,
  WARDEN,
  WARDEN_SURCHARGE,
  registerGlitch,
} from "./fixtures/glitch";
import { put, slot } from "./fixtures/harness";

/** Past the mulligans, in p1's main phase with 4 mana, the Glitch fixtures registered. */
function glitchGame(seed: string): Run {
  const run = playing(seed);
  registerGlitch();
  return run;
}

/** Play a card from p1's hand (or `player`'s), no choices. */
function play(run: Run, defId: string, player: "p1" | "p2" = "p1"): Run {
  const card = handCard(run.state, defId, player);
  return act(run, { type: "play", instanceId: card.id, playerId: player });
}

/** An rng that answers `int` from a script, one value per draw, and counts its draws like the real one. */
function scriptedRng(values: readonly number[]): Rng {
  let at = 0;
  const rng: Rng = {
    get cursor(): number {
      return at;
    },
    next: () => (values[at++] ?? 0) / GLITCH_ODDS_DENOMINATOR,
    int: (n: number) => Math.min(n - 1, values[at++] ?? 0),
    pick: <T>(list: readonly T[]) => (list.length === 0 ? undefined : list[rng.int(list.length)]),
    shuffle: <T>(list: readonly T[]) => [...list],
    coin: () => rng.next() < 0.5,
    chance: (p: number) => rng.next() < p,
    lucky: <T>(_x: number, roll: () => T) => roll(),
  };
  return rng;
}

function glitchDef(): CardDef {
  const def = findDef(null, GLITCH_DEF_ID);
  if (def === undefined) throw new Error("the Glitch fixture is not registered");
  return def;
}

function handDefIds(state: GameState, player: "p1" | "p2" = "p1"): string[] {
  return state.players[player].hand.map((card) => card.defId);
}

describe("R661 the System plays", () => {
  it("R661 counts C #18 and C #25 whoever plays them, and nothing else; absent before the first", () => {
    let run = glitchGame("glitch-count");
    expect(run.state.systemPlays).toBeUndefined();
    run = play(run, PLAIN);
    expect(run.state.systemPlays).toBeUndefined();
    run = play(run, SYSTEM_18);
    expect(run.state.systemPlays).toBe(1);
    run = play(run, SYSTEM_25);
    expect(run.state.systemPlays).toBe(2);
    run = act(run, { type: "endTurn", playerId: "p1" });
    run = play(run, SYSTEM_25, "p2");
    expect(run.state.systemPlays).toBe(3);
  });

  it("R661 a fused card holding System cards counts once, however many it holds", () => {
    const run = glitchGame("glitch-fused");
    const fusedId = `t-1:${SYSTEM_18}+${SYSTEM_25}`;
    run.state.transientDefs[fusedId] = { ...glitchDef(), id: fusedId, token: false, tags: [] };
    const fused = newInstance(run.state, fusedId, "p1", { z: "resolving", player: "p1" });
    recordPlay(run.state, "p1", fused);
    expect(run.state.systemPlays).toBe(1);
    const plain = newInstance(run.state, PLAIN, "p1", { z: "resolving", player: "p1" });
    recordPlay(run.state, "p1", plain);
    expect(run.state.systemPlays).toBe(1);
  });
});

describe("R661 the Glitch check in pickGenerated", () => {
  it("R661 draws nothing extra while no System card has been played: the same draws, the same card", () => {
    glitchGame("glitch-none");
    const pool = query({ type: "Unit" });
    for (const match of [null, {}, { systemPlays: 0 }]) {
      const a = createRng("glitch-none");
      const b = createRng("glitch-none");
      expect(pickGenerated(a, pool, match)).toBe(b.pick(pool));
      expect(a.cursor).toBe(1);
    }
  });

  it("R661 with n System plays, one more draw below n out of 10000 makes the card Glitch", () => {
    glitchGame("glitch-odds");
    const pool = query({ type: "Unit" });
    const plays = 3;
    const threshold = plays * GLITCH_ODDS_PER_SYSTEM_PLAY;
    expect(pickGenerated(scriptedRng([0, threshold - 1]), pool, { systemPlays: plays })?.id).toBe(GLITCH_DEF_ID);
    const kept = scriptedRng([0, threshold]);
    expect(pickGenerated(kept, pool, { systemPlays: plays })?.id).toBe(pool[0]?.id);
    expect(kept.cursor).toBe(2);
    // The real rng: the check is one draw out of GLITCH_ODDS_DENOMINATOR, after the pick.
    const real = createRng("glitch-odds");
    const probe = createRng("glitch-odds");
    const picked = probe.pick(pool);
    const glitched = probe.int(GLITCH_ODDS_DENOMINATOR) < threshold;
    expect(pickGenerated(real, pool, { systemPlays: plays })?.id).toBe(glitched ? GLITCH_DEF_ID : picked?.id);
    expect(real.cursor).toBe(2);
  });

  it("R661 the chance grows with each System play", () => {
    glitchGame("glitch-grows");
    // A draw of 4 is Glitch from the fifth System play on, never before.
    for (const plays of [1, 2, 3, 4, 5, 6]) {
      expect(glitchReplaces(scriptedRng([4]), { systemPlays: plays })?.id === GLITCH_DEF_ID).toBe(plays >= 5);
    }
  });

  it("R661 an empty pool makes nothing and draws nothing, Glitch check included", () => {
    glitchGame("glitch-empty");
    const rng = createRng("glitch-empty");
    expect(pickGenerated(rng, [], { systemPlays: 5 })).toBeUndefined();
    expect(rng.cursor).toBe(0);
  });

  it("R661 Glitch takes a generated card's place only where it fits, and draws nothing where it cannot", () => {
    glitchGame("glitch-fits");
    const pool = query({ type: "Unit" });
    const refusing = scriptedRng([0, 0]);
    expect(pickGenerated(refusing, pool, { systemPlays: 1 }, () => false)?.id).toBe(pool[0]?.id);
    expect(refusing.cursor).toBe(1);
    expect(pickGenerated(scriptedRng([0, 0]), pool, { systemPlays: 1 }, (def) => def.type === "Spell")?.id).toBe(
      GLITCH_DEF_ID,
    );
  });

  it("R661 cards added to a hand or shuffled into a deck may be Glitch; a summoned or field-transformed card never is", () => {
    // Enough System plays that every check succeeds, so the check is what is being proved.
    let run = glitchGame("glitch-sites");
    run.state.systemPlays = GLITCH_ODDS_DENOMINATOR;
    run = play(run, ADDER);
    expect(handDefIds(run.state).filter((id) => id === GLITCH_DEF_ID)).toHaveLength(GLITCHFX_ADDED);

    const before = run.state.players.p1.library.length;
    run = play(run, SHUFFLER);
    expect(run.state.players.p1.library).toHaveLength(before + 1);
    expect(run.state.players.p1.library.some((card) => card.defId === GLITCH_DEF_ID)).toBe(true);

    run = play(run, SUMMONER);
    const units = run.state.players.p1.units.flatMap((pile) => pile ?? []);
    expect(units.length).toBeGreaterThan(0);
    expect(units.every((card) => findDef(run.state, card.defId)?.type === "Unit")).toBe(true);

    // A hand card transformed into a random Unit may become Glitch, which a hand holds.
    run.state.players.p1.hand = [];
    handCard(run.state, PLAIN);
    run = play(run, TRANSFORMER);
    expect(handDefIds(run.state)).toEqual([GLITCH_DEF_ID]);
  });

  it("R661 a game that plays System cards and generates folds back to the same state", () => {
    const start = glitchGame("glitch-replay");
    const cards = [SYSTEM_18, SYSTEM_25, ADDER, SHUFFLER].map((defId) => handCard(start.state, defId));
    let run = frozen(start);
    for (const card of cards) run = act(run, { type: "play", instanceId: card.id, playerId: "p1" });
    expect(run.state.systemPlays).toBe(2);
    expect(hashState(replayed(run))).toBe(hashState(run.state));
  });
});

describe("R662 a hidden card is in no pool", () => {
  it("R662 no pool holds Glitch, not one that takes every token or asks for tokens only; naming it does", () => {
    glitchGame("glitch-hidden");
    expect(glitchDef().hidden).toBe(true);
    for (const asked of [{}, { withTokens: true }, { token: true }, { tags: ["Token" as const] }, { rarity: "Token" as const }, { type: "Spell" as const, withTokens: true }]) {
      expect(query(asked).some((def) => def.id === GLITCH_DEF_ID)).toBe(false);
    }
    expect(query({ defId: GLITCH_DEF_ID }).map((def) => def.id)).toEqual([GLITCH_DEF_ID]);
  });
});

describe("R663 Glitch is always playable on its owner's turn", () => {
  it("R663 costs (0) whatever changes its price, and no ban refuses it; a plain (0) card beside it is refused", () => {
    let run = glitchGame("glitch-playable");
    put(run.state, WARDEN, slot("p2", "units", 1));
    run.state.players.p1.mana.current = 0;
    const glitch = handCard(run.state, GLITCH_DEF_ID);
    glitch.costMod = WARDEN_SURCHARGE;
    glitch.costOverride = WARDEN_SURCHARGE;
    const plain = handCard(run.state, PLAIN);
    expect(effectiveCost(run.state, glitch)).toBe(0);
    expect(effectiveCost(run.state, plain)).toBe(WARDEN_SURCHARGE);

    const offered = legalActions(run.state, "p1").flatMap((body) => (body.type === "play" ? [body.instanceId] : []));
    expect(offered).toContain(glitch.id);
    expect(offered).not.toContain(plain.id);
    expect(refusal(run, { type: "play", instanceId: plain.id, playerId: "p1" })).toBeDefined();

    run = act(run, { type: "play", instanceId: glitch.id, playerId: "p1" });
    expect(run.state.players.p1.graveyard.map((card) => card.id)).toContain(glitch.id);
    expect(run.state.players.p1.mana.current).toBe(0);
  });

  it("R663 has no cost a Degrade, an Upgrade or KY's Constant could move", () => {
    const run = glitchGame("glitch-untuned");
    const glitch = handCard(run.state, GLITCH_DEF_ID);
    const plain = handCard(run.state, PLAIN);
    expect(ownCost(run.state, glitch)).toBeNull();
    expect(numbersOn(run.state, glitch).some((entry) => entry.ref.kind === "cost")).toBe(false);
    expect(ownCost(run.state, plain)).toBe(0);
    expect(numbersOn(run.state, plain).some((entry) => entry.ref.kind === "cost")).toBe(true);
  });

  it("R663 is not playable on the opponent's turn", () => {
    const run = glitchGame("glitch-not-yours");
    const glitch = handCard(run.state, GLITCH_DEF_ID, "p2");
    expect(legalActions(run.state, "p2").some((body) => body.type === "play" && body.instanceId === glitch.id)).toBe(false);
    expect(refusal(run, { type: "play", instanceId: glitch.id, playerId: "p2" })).toBeDefined();
  });
});

describe("R664 Glitch's outcome", () => {
  it("R664 rolls one of the four outcomes on one draw, each reachable", () => {
    const seen = new Set<string>();
    for (let i = 0; i < 64; i += 1) {
      const rng = createRng(`glitch-outcome-${i}`);
      const probe = createRng(`glitch-outcome-${i}`);
      const outcome = rollGlitchOutcome(rng);
      expect(outcome).toBe(GLITCH_OUTCOMES[probe.int(GLITCH_OUTCOMES.length)]);
      expect(rng.cursor).toBe(1);
      seen.add(outcome);
    }
    expect([...seen].sort()).toEqual([...GLITCH_OUTCOMES].sort());
  });

  it("R664 playing Glitch takes exactly that one draw of the match rng and, until the outcomes are built, changes nothing else", () => {
    let run = glitchGame("glitch-play");
    const glitch = handCard(run.state, GLITCH_DEF_ID);
    const cursor = run.state.rngCursor;
    const health = [run.state.players.p1.hero.health, run.state.players.p2.hero.health];
    run = act(run, { type: "play", instanceId: glitch.id, playerId: "p1" });
    expect(run.state.rngCursor).toBe(cursor + 1);
    expect([run.state.players.p1.hero.health, run.state.players.p2.hero.health]).toEqual(health);
    expect(run.state.result).toBeNull();
    expect(run.state.active).toBe("p1");
  });
});
