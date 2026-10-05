/**
 * R433 on the server: an All Random match (R258) deals both seats' decks, so neither player is shown
 * theirs going in, and each seat's own library lists every card as unknown until it leaves (R312).
 * The registry reads the match's mode off what made it (`matches.modeOf`) and tells `createGame` and
 * every rebuild's `fold` both seats were dealt; a Best-of-1 match's decks were built and list in full
 * (R310). The real engine port under the real registry, actor and results writer, over the in-memory
 * store.
 */

import { describe, expect, it } from "vitest";
import type { PlayerView } from "@jackioh/shared";
import type { QueueMode } from "../../src/api/ports";
import { createRecordResult, createVoidMatch } from "../../src/api/results";
import { createMatchClock } from "../../src/match/clock";
import type { ActorDeps } from "../../src/match/contracts";
import type { EnginePort } from "../../src/match/engine";
import { enginePort } from "../../src/match/engine.real.ts";
import { createMatchRegistry, type MatchRegistry } from "../../src/match/registry";
import { createTestDeps, TEST_CATALOG_VERSION, type TestDeps } from "../fakes/deps";

const P1 = "profile-1";
const P2 = "profile-2";

function world(): { deps: TestDeps; engine: EnginePort; registry: MatchRegistry } {
  const deps = createTestDeps();
  deps.store.seedProfile({ id: P1, rating: 1000 });
  deps.store.seedProfile({ id: P2, rating: 1000 });
  const engine = enginePort();
  const actorDeps: ActorDeps = {
    store: deps.store,
    timers: deps.timers,
    config: deps.config,
    log: deps.log,
    engine,
    createClock: createMatchClock,
    recordResult: createRecordResult(deps),
    voidMatch: createVoidMatch(deps),
  };
  return { deps, engine, registry: createMatchRegistry(actorDeps) };
}

/** The two queue tickets a match in `mode` was paired from (§9.5): what tells the registry its mode. */
function pairedTickets(deps: TestDeps, matchId: string, mode: QueueMode): void {
  for (const profileId of [P1, P2]) {
    deps.store.tables.tickets.push({
      id: `ticket-${matchId}-${profileId}`,
      profileId,
      rating: 1000,
      mode,
      deck: [],
      trio: null,
      catalogVersion: TEST_CATALOG_VERSION,
      enqueuedAt: 0,
      status: "matched",
      matchId,
    });
  }
}

/** Starts a match on two decks R258's deal gives for `seed`, as the queue and the rooms deal them. */
async function start(registry: MatchRegistry, engine: EnginePort, matchId: string, seed: string): Promise<void> {
  await registry.start({
    matchId,
    seed,
    catalogVersion: TEST_CATALOG_VERSION,
    ranked: false,
    seats: [
      { profileId: P1, player: "p1", deck: engine.dealRandomDeck(`${seed}:p1-deck`) },
      { profileId: P2, player: "p2", deck: engine.dealRandomDeck(`${seed}:p2-deck`) },
    ],
  });
}

/** How many cards the seat's own library list names (R310), the unknown ones aside. */
function listed(view: PlayerView): number {
  return (view.you.ownLibrary?.cards ?? []).reduce((sum, entry) => sum + entry.count, 0);
}

describe("R433 a dealt deck lists only the cards its owner has been shown", () => {
  it("R433 an All Random match lists neither seat's starting library, and a rebuilt actor folds the same game", async () => {
    const { deps, engine, registry } = world();
    pairedTickets(deps, "m-random", "random");
    await start(registry, engine, "m-random", "r433-random");
    const actor = await registry.actorFor("m-random");

    for (const player of ["p1", "p2"] as const) {
      const view = actor.viewFor(player);
      expect(view.you.libraryCount).toBeGreaterThan(0);
      expect(view.you.ownLibrary).toEqual({ cards: [], unknown: view.you.libraryCount });
    }

    const live = engine.hashState(actor.engineState());
    await registry.stop("m-random");
    const rebuilt = await registry.actorFor("m-random");
    expect(engine.hashState(rebuilt.engineState())).toBe(live);
    for (const player of ["p1", "p2"] as const) {
      const view = rebuilt.viewFor(player);
      expect(view.you.ownLibrary).toEqual({ cards: [], unknown: view.you.libraryCount });
    }
  });

  it("R433, R310 a Best-of-1 match's decks were built: each seat's own library is listed in full", async () => {
    const { deps, engine, registry } = world();
    pairedTickets(deps, "m-bo1", "bo1");
    await start(registry, engine, "m-bo1", "r433-bo1");
    const actor = await registry.actorFor("m-bo1");

    for (const player of ["p1", "p2"] as const) {
      const view = actor.viewFor(player);
      expect(view.you.libraryCount).toBeGreaterThan(0);
      expect(view.you.ownLibrary?.unknown).toBe(0);
      expect(listed(view)).toBe(view.you.libraryCount);
    }
  });

  it("R433 a match nothing a mode can be read off made is not dealt", async () => {
    const { engine, registry } = world();
    await start(registry, engine, "m-none", "r433-none");
    const actor = await registry.actorFor("m-none");

    for (const player of ["p1", "p2"] as const) {
      const view = actor.viewFor(player);
      expect(view.you.libraryCount).toBeGreaterThan(0);
      expect(view.you.ownLibrary?.unknown).toBe(0);
      expect(listed(view)).toBe(view.you.libraryCount);
    }
  });
});
