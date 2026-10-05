/**
 * Rematch offers after a finished non-series match (SPEC §9.5, R659):
 * `POST`/`GET /api/matches/:matchId/rematch` (`src/api/rematch.ts`), the rating move a
 * double-or-nothing rematch makes (`src/api/ranked.ts`, `src/api/results.ts`), and the presence
 * the registry reports off the live actor's sockets (`src/match/registry.ts`, `src/match/actor.ts`).
 *
 *  - **R659**: a double-or-nothing rematch is ranked-only and doubles each side's Glicko rating
 *    delta, with deviation and volatility from the single update.
 *  - Equal stakes from both seats create exactly one match with the finished decks, a fresh seed
 *    and the same seats; mismatched stakes create nothing.
 *  - A non-seat learns nothing (404); a live match (422), a series game (422) and a double from
 *    an unranked match (422) are refused.
 *  - Stale offers are ignored past `REMATCH_OFFER_TTL_MS`; `opponentHere` follows the opponent's
 *    socket, false once it closes or the actor is gone.
 *
 * Everything runs on `createManualTimers()` through `createTestDeps()`, with the store-writing
 * fake match directory (`queue.test.ts`'s precedent), so offer expiry and the created row are set
 * by this file rather than by the host clock.
 */

import { beforeEach, describe, expect, it } from "vitest";

import { REMATCH_OFFER_TTL_MS } from "../../src/config";
import { createRouter, type Router } from "../../src/api/http";
import type { MatchRow, QueueMode, SeriesRow } from "../../src/api/ports";
import { rateRankedGame } from "../../src/api/ranked";
import {
  STAKE_DOUBLE,
  STAKE_NORMAL,
  clearRematchOffers,
  createRematchRoutes,
  type RematchOfferBody,
  type RematchStatusBody,
} from "../../src/api/rematch";
import { createRecordResult } from "../../src/api/results";
import { createMatchClock } from "../../src/match/clock";
import { createMatchRegistry } from "../../src/match/registry";
import { createFakeEngine, fakeDeck } from "../fakes/engine";
import { createFakeSocket } from "../fakes/socket";
import {
  createFakeMatchDirectory,
  createTestDeps,
  jsonRequest,
  readJson,
  type TestDeps,
} from "../fakes/deps";

const A = "profile-a";
const B = "profile-b";
const FINISHED = "match-finished";

let deps: TestDeps;
let router: Router;

/** An active profile with a token that verifies as it (`queue.test.ts`'s fixture). */
function activeProfile(target: TestDeps, id: string, rating = 1000): string {
  const userId = `user-${id}`;
  target.store.seedProfile({ id, userId, status: "active", rating });
  return target.auth.addUser({ userId, email: `${id}@example.test` });
}

function clocks(now: number): MatchRow["clocks"] {
  return {
    turnDeadline: null,
    promptDeadline: null,
    graceDeadline: { p1: null, p2: null },
    ceilingAt: now + 3_600_000,
  };
}

/** A finished Best-of-1 match between the two profiles, ranked unless told otherwise. */
async function finishedMatch(
  target: TestDeps,
  id: string,
  p1: string,
  p2: string,
  extra: Partial<MatchRow> = {},
): Promise<MatchRow> {
  const now = target.timers.now();
  const row: MatchRow = {
    id,
    seed: `seed-${id}`,
    players: [p1, p2],
    decks: [
      [`${p1}-card-1`, `${p1}-card-2`],
      [`${p2}-card-1`, `${p2}-card-2`],
    ],
    catalogVersion: target.catalog.version,
    ranked: true,
    status: "finished",
    createdAt: now,
    finishedAt: now,
    clocks: clocks(now),
    portraits: ["vanilla", "vanilla"],
    ...extra,
  };
  await target.store.matches.create(row);
  return row;
}

/** Queue tickets the finished match paired, so `matches.modeOf` reads its mode. */
async function pairedTickets(target: TestDeps, matchId: string, mode: QueueMode): Promise<void> {
  const now = target.timers.now();
  for (const profileId of [A, B]) {
    await target.store.tickets.insert({
      id: `ticket-${matchId}-${profileId}`,
      profileId,
      rating: 1000,
      mode,
      deck: [],
      portrait: null,
      trio: null,
      catalogVersion: target.catalog.version,
      enqueuedAt: now,
      status: "matched",
      matchId,
    });
  }
}

async function offer(token: string, matchId: string, stakes: number): Promise<Response> {
  return router(jsonRequest("POST", `/api/matches/${matchId}/rematch`, { stakes }, { token }));
}

async function status(token: string, matchId: string): Promise<Response> {
  return router(jsonRequest("GET", `/api/matches/${matchId}/rematch`, undefined, { token }));
}

beforeEach(() => {
  clearRematchOffers();
  deps = createTestDeps();
  // The directory WRITES THE MATCH ROW, because `createMatchRegistry.start` does
  // (`queue.test.ts`'s precedent): the created rematch is asserted off the store.
  deps.matches = createFakeMatchDirectory(deps.store);
  router = createRouter(createRematchRoutes(), deps);
});

// ---------------------------------------------------------------------------
// R659 — the rating move
// ---------------------------------------------------------------------------

describe("R659 double-or-nothing", () => {
  it("R659 doubles each side's rating movement, with deviation and volatility from the single update", async () => {
    const sides = (a: string, b: string) =>
      [
        { kind: "player", profileId: a },
        { kind: "player", profileId: b },
      ] as const;
    for (const profileId of [A, B, "profile-c", "profile-d"]) {
      deps.store.seedProfile({ id: profileId });
    }
    const single = await deps.store.tx((t) =>
      rateRankedGame(t, deps, {
        id: "single",
        kind: "match",
        catalogVersion: deps.catalog.version,
        sides: sides(A, B),
        winnerSide: 0,
        reason: "hero-death",
        at: deps.timers.now(),
      }),
    );
    const doubled = await deps.store.tx((t) =>
      rateRankedGame(t, deps, {
        id: "doubled",
        kind: "match",
        catalogVersion: deps.catalog.version,
        sides: sides("profile-c", "profile-d"),
        winnerSide: 0,
        reason: "hero-death",
        at: deps.timers.now(),
        stake: STAKE_DOUBLE,
      }),
    );

    for (const side of [0, 1] as const) {
      const before = single.sides[side].before.rating;
      const singleDelta = single.sides[side].after.rating - before;
      // The same starting ratings, so the single update's delta is the doubled game's unit.
      expect(single.sides[side].before.rating).toBe(doubled.sides[side].before.rating);
      expect(doubled.sides[side].after.rating - before).toBeCloseTo(2 * singleDelta, 10);
      // Confidence is not doubled: the same update's deviation and volatility.
      expect(doubled.sides[side].after.deviation).toBe(single.sides[side].after.deviation);
      expect(doubled.sides[side].after.volatility).toBe(single.sides[side].after.volatility);
    }
  });

  it("R659 a finished double-or-nothing rematch moves each side's rating twice as far", async () => {
    for (const [pair, id, stake] of [
      [[A, B], "double-game", STAKE_DOUBLE],
      [["profile-c", "profile-d"], "single-game", STAKE_NORMAL],
    ] as const) {
      const [p1, p2] = pair;
      deps.store.seedProfile({ id: p1 });
      deps.store.seedProfile({ id: p2 });
      await finishedMatch(deps, id, p1, p2, stake === STAKE_DOUBLE ? { stake } : {});
      await createRecordResult(deps)({
        matchId: id,
        seats: [
          { profileId: p1, player: "p1", deck: [] },
          { profileId: p2, player: "p2", deck: [] },
        ],
        outcome: { winner: "p1", reason: "hero-death" },
        turns: 9,
        at: deps.timers.now(),
      });
    }
    const [a, b, c, d] = await deps.store.profiles.getMany([A, B, "profile-c", "profile-d"]);
    if (a === undefined || b === undefined || c === undefined || d === undefined) {
      throw new Error("premise: all four profiles exist");
    }
    // Both pairs started at the same rating, so the doubled game's movement is twice the single's.
    expect(a.rating - 1000).toBeCloseTo(2 * (c.rating - 1000), 8);
    expect(1000 - b.rating).toBeCloseTo(2 * (1000 - d.rating), 8);
  });

  it("R659 a double from an unranked match is refused", async () => {
    const tokenA = activeProfile(deps, A);
    activeProfile(deps, B);
    await finishedMatch(deps, FINISHED, A, B, { ranked: false });

    const refused = await offer(tokenA, FINISHED, STAKE_DOUBLE);
    expect(refused.status).toBe(422);
    expect((await readJson<{ error: { code: string } }>(refused)).error.code).toBe(
      "double_requires_ranked",
    );
    // And nothing was offered on the way to the refusal.
    const body = await readJson<RematchStatusBody>(await status(tokenA, FINISHED));
    expect(body.youOffered).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Offers and creation
// ---------------------------------------------------------------------------

describe("rematch offers", () => {
  it("upserts one seat's offer and reports both seats' offers back", async () => {
    const tokenA = activeProfile(deps, A);
    const tokenB = activeProfile(deps, B);
    await finishedMatch(deps, FINISHED, A, B);

    expect((await readJson<RematchOfferBody>(await offer(tokenA, FINISHED, STAKE_NORMAL))).matchId).toBeNull();

    const seenByA = await readJson<RematchStatusBody>(await status(tokenA, FINISHED));
    expect(seenByA).toMatchObject({ youOffered: STAKE_NORMAL, opponentOffer: null, matchId: null });
    const seenByB = await readJson<RematchStatusBody>(await status(tokenB, FINISHED));
    expect(seenByB).toMatchObject({ youOffered: null, opponentOffer: STAKE_NORMAL, matchId: null });
  });

  it("matching offers create exactly one match with the finished decks, a fresh seed and the same seats", async () => {
    const tokenA = activeProfile(deps, A);
    const tokenB = activeProfile(deps, B);
    const finished = await finishedMatch(deps, FINISHED, A, B);
    await pairedTickets(deps, FINISHED, "bo1");

    expect((await readJson<RematchOfferBody>(await offer(tokenA, FINISHED, STAKE_NORMAL))).matchId).toBeNull();
    const created = await readJson<RematchOfferBody>(await offer(tokenB, FINISHED, STAKE_NORMAL));
    expect(typeof created.matchId).toBe("string");
    const rematchId = created.matchId as string;

    const rematch = await deps.store.matches.get(rematchId);
    expect(rematch).toMatchObject({
      players: [A, B],
      decks: finished.decks,
      ranked: true,
      mode: "bo1",
      status: "live",
    });
    expect(rematch?.stake).toBeUndefined();
    expect(rematch?.seed).not.toBe(finished.seed);
    // Both seats are in the new game.
    expect((await deps.store.profiles.getById(A))?.inMatchId).toBe(rematchId);
    expect((await deps.store.profiles.getById(B))?.inMatchId).toBe(rematchId);
    // The directory started it, so a reconnecting socket finds a live actor's match row.
    expect(deps.matches.started.map((input) => input.matchId)).toContain(rematchId);

    // Offering again changes nothing: the same id comes back, no second game.
    const again = await readJson<RematchOfferBody>(await offer(tokenA, FINISHED, STAKE_NORMAL));
    expect(again.matchId).toBe(rematchId);
    expect((await deps.store.matches.live()).map((match) => match.id)).toEqual([rematchId]);
    // A late poller still learns where to go.
    expect((await readJson<RematchStatusBody>(await status(tokenA, FINISHED))).matchId).toBe(rematchId);
  });

  it("a double-or-nothing both seats want is ranked with stakes on its row", async () => {
    const tokenA = activeProfile(deps, A);
    const tokenB = activeProfile(deps, B);
    await finishedMatch(deps, FINISHED, A, B);
    await pairedTickets(deps, FINISHED, "bo1");

    await offer(tokenA, FINISHED, STAKE_DOUBLE);
    const created = await readJson<RematchOfferBody>(await offer(tokenB, FINISHED, STAKE_DOUBLE));
    const rematch = await deps.store.matches.get(created.matchId as string);
    expect(rematch).toMatchObject({ ranked: true, stake: STAKE_DOUBLE, mode: "bo1" });
  });

  it("mismatched stakes create nothing", async () => {
    const tokenA = activeProfile(deps, A);
    const tokenB = activeProfile(deps, B);
    await finishedMatch(deps, FINISHED, A, B);

    expect((await readJson<RematchOfferBody>(await offer(tokenA, FINISHED, STAKE_NORMAL))).matchId).toBeNull();
    expect((await readJson<RematchOfferBody>(await offer(tokenB, FINISHED, STAKE_DOUBLE))).matchId).toBeNull();
    expect(await deps.store.matches.live()).toEqual([]);
    expect(deps.matches.started).toEqual([]);
  });

  it("an All Random rematch deals fresh decks from the new seed", async () => {
    const tokenA = activeProfile(deps, A);
    const tokenB = activeProfile(deps, B);
    const finished = await finishedMatch(deps, FINISHED, A, B);
    await pairedTickets(deps, FINISHED, "random");

    await offer(tokenA, FINISHED, STAKE_NORMAL);
    const created = await readJson<RematchOfferBody>(await offer(tokenB, FINISHED, STAKE_NORMAL));
    const rematch = await deps.store.matches.get(created.matchId as string);
    if (rematch === null) throw new Error("premise: the rematch was created");
    expect(rematch.mode).toBe("random");
    // Fresh decks from the new seed (`queue.ts`'s suffix scheme), not the finished ones.
    expect(rematch.decks).toEqual([
      deps.dealRandomDeck(`${rematch.seed}:p1-deck`),
      deps.dealRandomDeck(`${rematch.seed}:p2-deck`),
    ]);
    expect(rematch.decks).not.toEqual(finished.decks);
  });

  it("a stale offer is ignored past the TTL", async () => {
    const tokenA = activeProfile(deps, A);
    const tokenB = activeProfile(deps, B);
    await finishedMatch(deps, FINISHED, A, B);

    await offer(tokenA, FINISHED, STAKE_NORMAL);
    deps.timers.advance(REMATCH_OFFER_TTL_MS + 1);

    // A's offer lapsed, so B's is the first live one: no game, and A must offer again.
    expect((await readJson<RematchOfferBody>(await offer(tokenB, FINISHED, STAKE_NORMAL))).matchId).toBeNull();
    expect(await deps.store.matches.live()).toEqual([]);
    expect((await readJson<RematchStatusBody>(await status(tokenA, FINISHED))).youOffered).toBeNull();

    await offer(tokenA, FINISHED, STAKE_NORMAL);
    const created = await readJson<RematchOfferBody>(await status(tokenB, FINISHED));
    expect(created.matchId).not.toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Refusals
// ---------------------------------------------------------------------------

describe("rematch refusals", () => {
  it("a non-seat gets 404 on both endpoints, like for a missing match", async () => {
    const tokenA = activeProfile(deps, A);
    activeProfile(deps, B);
    const tokenC = activeProfile(deps, "profile-c");
    await finishedMatch(deps, FINISHED, A, B);

    const posted = await offer(tokenC, FINISHED, STAKE_NORMAL);
    expect(posted.status).toBe(404);
    expect((await readJson<{ error: { code: string } }>(posted)).error.code).toBe("not_found");
    const read = await status(tokenC, FINISHED);
    expect(read.status).toBe(404);
    expect((await readJson<{ error: { code: string } }>(read)).error.code).toBe("not_found");

    const missing = await offer(tokenA, "no-such-match", STAKE_NORMAL);
    expect(missing.status).toBe(404);
  });

  it("an unfinished match gets 422", async () => {
    const tokenA = activeProfile(deps, A);
    activeProfile(deps, B);
    await finishedMatch(deps, FINISHED, A, B, { status: "live", finishedAt: null });

    const refused = await offer(tokenA, FINISHED, STAKE_NORMAL);
    expect(refused.status).toBe(422);
    expect((await readJson<{ error: { code: string } }>(refused)).error.code).toBe(
      "match_not_finished",
    );
  });

  it("a series game is refused, whatever its status", async () => {
    const tokenA = activeProfile(deps, A);
    activeProfile(deps, B);
    await finishedMatch(deps, FINISHED, A, B);
    const trio = (name: string) => ({
      name,
      decks: [
        { name: "one", cards: [] },
        { name: "two", cards: [] },
        { name: "three", cards: [] },
      ] as [{ name: string; cards: string[] }, { name: string; cards: string[] }, { name: string; cards: string[] }],
    });
    const series: SeriesRow = {
      id: "series-1",
      sides: [
        { profileId: A, trio: trio("a"), wins: 0, pick: null },
        { profileId: B, trio: trio("b"), wins: 0, pick: null },
      ],
      catalogVersion: deps.catalog.version,
      seedBase: "seed-base",
      status: "playing",
      games: [{ gameNo: 1, matchId: FINISHED, slots: [0, 0], first: "p1", winner: null, reason: null }],
      nextMatchId: FINISHED,
      pickDeadline: null,
      winner: null,
      endReason: null,
      ratingBefore: null,
      ratingAfter: null,
      createdAt: deps.timers.now(),
      updatedAt: deps.timers.now(),
      endedAt: null,
      version: 0,
    };
    await deps.store.series.create(series);

    const refused = await offer(tokenA, FINISHED, STAKE_NORMAL);
    expect(refused.status).toBe(422);
    expect((await readJson<{ error: { code: string } }>(refused)).error.code).toBe("series_game");
  });

  it("a bad stakes value is a 400", async () => {
    const tokenA = activeProfile(deps, A);
    activeProfile(deps, B);
    await finishedMatch(deps, FINISHED, A, B);

    const refused = await router(
      jsonRequest("POST", `/api/matches/${FINISHED}/rematch`, { stakes: 3 }, { token: tokenA }),
    );
    expect(refused.status).toBe(400);
  });
});

// ---------------------------------------------------------------------------
// Presence
// ---------------------------------------------------------------------------

describe("rematch presence", () => {
  it("reports the opponent's offer only while their socket is open, via the status", async () => {
    const tokenA = activeProfile(deps, A);
    const tokenB = activeProfile(deps, B);
    await finishedMatch(deps, FINISHED, A, B);

    // No live actor: nobody is here.
    expect((await readJson<RematchStatusBody>(await status(tokenA, FINISHED))).opponentHere).toBe(false);

    deps.matches.presenceOf = () => ({ p1: true, p2: true });
    expect((await readJson<RematchStatusBody>(await status(tokenA, FINISHED))).opponentHere).toBe(true);

    // Only the opponent's seat counts: mine open alone is still gone.
    deps.matches.presenceOf = () => ({ p1: true, p2: false });
    expect((await readJson<RematchStatusBody>(await status(tokenB, FINISHED))).opponentHere).toBe(true);
    expect((await readJson<RematchStatusBody>(await status(tokenA, FINISHED))).opponentHere).toBe(false);
  });

  it("the registry's presence follows the live actor's sockets and dies with it", async () => {
    const engine = createFakeEngine();
    const registry = createMatchRegistry({
      store: deps.store,
      timers: deps.timers,
      config: deps.config,
      log: deps.log,
      engine,
      createClock: createMatchClock,
      recordResult: async () => {
        throw new Error("unreachable: no game ends here");
      },
    });
    const matchId = "match-presence";
    await registry.start({
      matchId,
      seed: "seed-presence",
      catalogVersion: deps.catalog.version,
      ranked: false,
      seats: [
        { profileId: A, player: "p1", deck: fakeDeck() },
        { profileId: B, player: "p2", deck: fakeDeck() },
      ],
    });

    // No sockets yet: neither seat is here.
    expect(registry.presenceOf(matchId)).toEqual({ p1: false, p2: false });
    // An id with no actor is nobody, not an empty room.
    expect(registry.presenceOf("no-such-match")).toBeNull();

    const p1 = createFakeSocket();
    const p2 = createFakeSocket();
    await registry.attach(matchId, A, p1);
    await registry.attach(matchId, B, p2);
    expect(registry.presenceOf(matchId)).toEqual({ p1: true, p2: true });

    // The opponent closes the tab: their seat reads as gone.
    p2.drop();
    expect(registry.presenceOf(matchId)).toEqual({ p1: true, p2: false });

    await registry.stop(matchId);
    expect(registry.presenceOf(matchId)).toBeNull();
  });
});
