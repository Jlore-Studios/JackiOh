/**
 * The Glitch Easter egg on the server (issue #170, SPEC §7, R675–R678): what the server does with
 * the swap, boards and void outcomes the engine announces. The scripted engine (`test/fakes/
 * engine.ts`) reaches each on demand — `test-glitch-swap` and `test-glitch-void` — under the real
 * registry, actor and results writer, over the in-memory store. The reset outcome (R675) needs
 * nothing of the server: the engine rebuilds its own state, and `(seed, log)` still folds to it.
 *
 * The trust model these prove: only the engine's state moves an account to the other seat or voids
 * a match. A client frame reaches either only as a legal `play` of the card.
 */

import { describe, expect, it } from "vitest";
import type { ActionBody, PlayerView } from "@jackioh/shared";

import { createRouter } from "../../src/api/http";
import type { FrozenTrio, GameRecorder, LastBoardEntry } from "../../src/api/ports";
import { createRecordResult, createVoidMatch } from "../../src/api/results";
import { createSeriesRoutes, startSeries } from "../../src/api/series";
import { GLITCH_BOARDS_SAMPLED, MATCH_VOIDED_CLOSE_CODE } from "../../src/config";
import { createMatchClock } from "../../src/match/clock";
import type { ActorDeps } from "../../src/match/contracts";
import type { EnginePort, FoldArgs } from "../../src/match/engine";
import { createMatchRegistry, type MatchRegistry } from "../../src/match/registry";
import {
  createFakeMatchDirectory,
  createTestDeps,
  jsonRequest,
  TEST_CATALOG_VERSION,
  type TestDeps,
} from "../fakes/deps";
import { createFakeEngine, fakeDeck } from "../fakes/engine";
import { createFakeSocket, type FakeSocket } from "../fakes/socket";

const MATCH_ID = "match-glitch";
const P1 = "profile-1";
const P2 = "profile-2";

type World = {
  deps: TestDeps;
  engine: EnginePort;
  registry: MatchRegistry;
  a: FakeSocket;
  b: FakeSocket;
};

/**
 * A live, ranked match on the real registry: account `P1` began in p1 with `p1Deck`, `P2` in p2.
 * `a` is P1's socket and `b` is P2's, attached the way `wsServer.ts` attaches them.
 */
async function world(options: { p1Deck?: string[]; p2Deck?: string[]; before?: (deps: TestDeps) => void } = {}): Promise<World> {
  const deps = createTestDeps();
  deps.store.seedProfile({ id: P1, rating: 1000, inMatchId: MATCH_ID });
  deps.store.seedProfile({ id: P2, rating: 1000, inMatchId: MATCH_ID });
  options.before?.(deps);
  const engine = createFakeEngine();
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
  const registry = createMatchRegistry(actorDeps);
  await registry.start({
    matchId: MATCH_ID,
    seed: "seed-glitch",
    catalogVersion: TEST_CATALOG_VERSION,
    ranked: true,
    seats: [
      { profileId: P1, player: "p1", deck: options.p1Deck ?? fakeDeck(["test-glitch-swap", "test-lethal"]) },
      { profileId: P2, player: "p2", deck: options.p2Deck ?? fakeDeck() },
    ],
  });
  const a = createFakeSocket();
  const b = createFakeSocket();
  await registry.attach(MATCH_ID, P1, a);
  await registry.attach(MATCH_ID, P2, b);
  await (await registry.actorFor(MATCH_ID)).idle();
  return { deps, engine, registry, a, b };
}

/** One client frame, and the actor's queue drained behind it. */
async function frame(w: World, socket: FakeSocket, nonce: string, body: ActionBody): Promise<Record<string, unknown> | undefined> {
  const actor = await w.registry.actorFor(MATCH_ID);
  socket.clear();
  socket.receiveJson({ type: "action", action: { ...body, nonce } });
  await actor.idle();
  return socket.ofType("ack").concat(socket.ofType("error")).at(-1);
}

function lastView(socket: FakeSocket): PlayerView {
  const views = socket.ofType<{ view: PlayerView }>("view");
  const last = views.at(-1);
  if (last === undefined) throw new Error("no view was sent");
  return last.view;
}

describe("Glitch's swap (R676)", () => {
  it("R676 routes each account's socket to the seat it now plays: views, legal actions and accepted actions", async () => {
    const w = await world();
    expect(lastView(w.a).viewer).toBe("p1");

    // P1 plays Glitch as p1 (hand slot 0): the seats swap.
    expect(await frame(w, w.a, "n1", { type: "play", instanceId: "p1-h0" })).toMatchObject({ type: "ack" });
    expect(lastView(w.a).viewer).toBe("p2");
    expect(lastView(w.b).viewer).toBe("p1");
    // The legal actions travel with the view of the seat played now: P2 (p1, the active seat) may
    // play p1's hand, P1 (p2) only concede.
    const legalOf = (socket: FakeSocket) => socket.ofType<{ legal: ActionBody[] }>("view").at(-1)?.legal ?? [];
    expect(legalOf(w.b)).toContainEqual({ type: "play", instanceId: "p1-h0" });
    expect(legalOf(w.a)).toEqual([{ type: "concede" }]);

    // An action from P1 is stamped p2 now — it is not p2's turn — and the same from P2 is p1's.
    expect(await frame(w, w.a, "n2", { type: "endTurn" })).toMatchObject({ type: "error", code: "illegal_action" });
    const log = () => w.deps.store.tables.matchActions.map((row) => row.action.playerId);
    expect(log()).toEqual(["p1"]);
    expect(await frame(w, w.b, "n3", { type: "endTurn" })).toMatchObject({ type: "ack" });
    expect(log()).toEqual(["p1", "p1"]);
    expect(lastView(w.a).active).toBe("p2");
  });

  it("R676 a client frame cannot move an account: a playerId on the wire is discarded, before and after a swap", async () => {
    const w = await world();
    w.a.receiveJson({ type: "action", action: { type: "endTurn", playerId: "p2", nonce: "x1" }, playerId: "p2" });
    await (await w.registry.actorFor(MATCH_ID)).idle();
    expect(w.deps.store.tables.matchActions.map((row) => row.action.playerId)).toEqual(["p1"]);
    expect(lastView(w.a).viewer).toBe("p1");

    // p2's turn: P2 plays nothing that swaps, and P1's smuggled seat still does not move it.
    await frame(w, w.b, "x2", { type: "endTurn" });
    expect(await frame(w, w.a, "x3", { type: "play", instanceId: "p1-h0" })).toMatchObject({ type: "ack" });
    expect(lastView(w.a).viewer).toBe("p2");
    w.a.receiveJson({ type: "action", action: { type: "concede", playerId: "p1", nonce: "x4" }, playerId: "p1" });
    await (await w.registry.actorFor(MATCH_ID)).idle();
    // Stamped p2, the seat P1 plays now: p1 — P2 — wins.
    expect(w.deps.store.tables.matchActions.at(-1)?.action).toMatchObject({ type: "concede", playerId: "p2" });
    expect(w.deps.store.tables.results[0]?.winnerProfileId).toBe(P2);
  });

  it("R676 credits the result by the seats as they are played at the end: the winning seat's current account wins, Elo included", async () => {
    const w = await world();
    await frame(w, w.a, "n1", { type: "play", instanceId: "p1-h0" });
    // P2 now plays p1, whose hand holds test-lethal at slot 0: seat p1 wins.
    expect(await frame(w, w.b, "n2", { type: "play", instanceId: "p1-h0" })).toMatchObject({ type: "ack" });

    const [result] = w.deps.store.tables.results;
    expect(result).toMatchObject({ matchId: MATCH_ID, winnerProfileId: P2, reason: "hero-death" });
    // Seat p1's row is P2's now, and P2's rating moved up.
    expect(result?.players).toEqual([P2, P1]);
    expect(result?.ratingAfter[0]).toBeGreaterThan(result?.ratingBefore[0] ?? Infinity);
    const profile = w.deps.store.tables.profiles.find((row) => row.id === P2);
    expect(profile?.rating).toBeGreaterThan(1000);
  });

  it("R676 a disconnect grace runs on the seat the account plays now", async () => {
    const w = await world();
    await frame(w, w.a, "n1", { type: "play", instanceId: "p1-h0" });
    const actor = await w.registry.actorFor(MATCH_ID);
    actor.detach("p1"); // P1's connection: the account that began in p1
    await actor.idle();
    expect(actor.clocks().graceDeadline.p2).not.toBeNull();
    expect(actor.clocks().graceDeadline.p1).toBeNull();
  });

  it("R676 a rebuilt actor reads the swap off the folded state", async () => {
    const w = await world();
    await frame(w, w.a, "n1", { type: "play", instanceId: "p1-h0" });
    await w.registry.stop(MATCH_ID);
    const fresh = createFakeSocket();
    expect(await w.registry.attach(MATCH_ID, P1, fresh)).toBe("p1");
    await (await w.registry.actorFor(MATCH_ID)).idle();
    expect(lastView(fresh).viewer).toBe("p2");
  });
});

describe("Glitch's boards (R677)", () => {
  const board = (defId: string): LastBoardEntry[] => [{ defId, radiant: false }];

  it("R677 freezes two other players' last server boards on the match, never either seat's own", async () => {
    let folded: FoldArgs | null = null;
    const w = await world({
      before: (deps) => {
        deps.store.tables.lastBoards.push(
          { profileId: P1, kind: "server", board: board("own-1") },
          { profileId: P2, kind: "server", board: board("own-2") },
          { profileId: "profile-3", kind: "practice", board: board("practice") },
          { profileId: "profile-4", kind: "server", board: [] },
          { profileId: "profile-5", kind: "server", board: board("other-5") },
          { profileId: "profile-6", kind: "server", board: board("other-6") },
          { profileId: "profile-7", kind: "server", board: board("other-7") },
        );
      },
    });
    expect(GLITCH_BOARDS_SAMPLED).toBe(2);
    const row = await w.deps.store.matches.get(MATCH_ID);
    const frozen = row?.glitchBoards;
    expect(frozen).toHaveLength(2);
    const others = [board("other-5"), board("other-6"), board("other-7")];
    for (const entry of frozen ?? []) expect(others).toContainEqual(entry);
    expect(frozen?.[0]).not.toEqual(frozen?.[1]);

    // A rebuild folds with the same boards (§9.3).
    const fold = w.engine.fold;
    w.engine.fold = (args) => {
      folded = args;
      return fold(args);
    };
    await w.registry.stop(MATCH_ID);
    await w.registry.actorFor(MATCH_ID);
    expect(folded).toMatchObject({ glitchBoards: frozen });
  });

  it("R677 passes what exists: one other board leaves the second seat's empty, none omits the field", async () => {
    const one = await world({
      before: (deps) => {
        deps.store.tables.lastBoards.push({ profileId: "profile-5", kind: "server", board: board("other-5") });
      },
    });
    expect((await one.deps.store.matches.get(MATCH_ID))?.glitchBoards).toEqual([board("other-5"), []]);

    const none = await world({
      before: (deps) => {
        deps.store.tables.lastBoards.push({ profileId: P1, kind: "server", board: board("own-1") });
      },
    });
    expect((await none.deps.store.matches.get(MATCH_ID))?.glitchBoards).toBeUndefined();
  });
});

describe("Glitch's void (R678)", () => {
  const voidDecks = { p1Deck: fakeDeck(["test-glitch-void"]) };

  it("R678 writes no result, no rating, no game record and no last board, and the match is gone", async () => {
    const w = await world({
      ...voidDecks,
      before: (deps) => {
        const games: GameRecorder = { patch: "v9.9.9", summarize: () => null };
        deps.games = games;
      },
    });
    await frame(w, w.a, "n1", { type: "play", instanceId: "p1-h0" });

    expect(w.deps.store.tables.results).toEqual([]);
    expect(w.deps.store.tables.gameRecords).toEqual([]);
    expect(w.deps.store.tables.lastBoards).toEqual([]);
    expect(w.deps.store.tables.ratedGames).toEqual([]);
    expect(w.deps.store.tables.profiles.map((row) => row.rating)).toEqual([1000, 1000]);
    // As if it never existed: no row, no log, and both players free to queue.
    expect(await w.deps.store.matches.get(MATCH_ID)).toBeNull();
    expect(w.deps.store.tables.matchActions).toEqual([]);
    expect(w.deps.store.tables.profiles.map((row) => row.inMatchId)).toEqual([null, null]);
    expect(w.registry.has(MATCH_ID)).toBe(false);
  });

  it("R678 closes both sockets with the voided close code, after a last view that shows the void", async () => {
    const w = await world(voidDecks);
    await frame(w, w.a, "n1", { type: "play", instanceId: "p1-h0" });
    for (const socket of [w.a, w.b]) {
      expect(socket.isOpen).toBe(false);
      expect(socket.closeCode).toBe(MATCH_VOIDED_CLOSE_CODE);
    }
    expect(lastView(w.b).result).toEqual({ winner: "draw", reason: "voided" });
    // A socket that arrives later finds no match.
    await expect(w.registry.attach(MATCH_ID, P1, createFakeSocket())).rejects.toThrow("no such match");
  });

  it("R678 logs one line naming the match and both profiles", async () => {
    const w = await world(voidDecks);
    await frame(w, w.a, "n1", { type: "play", instanceId: "p1-h0" });
    const lines = w.deps.log.entries.filter((entry) => entry.event === "match.voided");
    expect(lines).toEqual([
      { level: "warn", event: "match.voided", data: expect.objectContaining({ matchId: MATCH_ID, players: [P1, P2] }) },
    ]);
  });

  it("R678 a voided Conquest game never happened: the series plays the same game again", async () => {
    const deps = createTestDeps();
    deps.matches = createFakeMatchDirectory(deps.store);
    const tokens = [P1, P2].map((id) => {
      deps.store.seedProfile({ id, userId: `user-${id}`, status: "active", rating: 1000 });
      return deps.auth.addUser({ userId: `user-${id}`, email: `${id}@example.test` });
    });
    const trio = (owner: string): FrozenTrio => ({
      name: owner,
      decks: [0, 1, 2].map((slot) => ({ name: `${owner}-${String(slot)}`, cards: [`${owner}-card-${String(slot)}`] })) as FrozenTrio["decks"],
    });
    await startSeries(deps, {
      seriesId: "series-glitch",
      firstMatchId: MATCH_ID,
      sides: [
        { profileId: P1, trio: trio("a") },
        { profileId: P2, trio: trio("b") },
      ],
      seedBase: "seed-base",
      catalogVersion: deps.catalog.version,
      ranked: true,
    });
    const router = createRouter(createSeriesRoutes(), deps);
    for (const token of tokens) {
      const answer = await router(jsonRequest("POST", "/api/series/series-glitch/pick", { slot: 0 }, { token }));
      expect(answer.status).toBe(200);
    }
    expect(deps.matches.started.map((start) => start.matchId)).toEqual([MATCH_ID]);
    const before = await deps.store.series.get("series-glitch");

    // The actor's void: the registry has let the actor go, then `voidMatch`.
    await deps.matches.stop(MATCH_ID);
    await createVoidMatch(deps)({ matchId: MATCH_ID, players: [P1, P2], at: deps.timers.now() });

    const after = await deps.store.series.get("series-glitch");
    expect(after?.status).toBe("playing");
    expect(after?.games).toEqual(before?.games);
    expect(after?.nextMatchId).toBe(MATCH_ID);
    expect(deps.store.tables.results).toEqual([]);
    // Started again: the same id, seats and seed, and both players are in it.
    expect(deps.matches.started).toHaveLength(2);
    expect(deps.matches.started[1]).toEqual(deps.matches.started[0]);
    expect((await deps.store.matches.get(MATCH_ID))?.status).toBe("live");
    expect(deps.store.tables.profiles.map((row) => row.inMatchId)).toEqual([MATCH_ID, MATCH_ID]);
  });
});
