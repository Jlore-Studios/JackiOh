/**
 * R738 — the opponent's aim through the actor (§9.5): a top-level `aim` frame, never an
 * `ActionBody`, shape-checked by `protocol.ts`, relayed to the opponent alone, coalesced to one
 * relay per `AIM_RELAY_INTERVAL_MS` per seat, and dropped when an end names anything the opponent
 * may not see.
 *
 * Runs on the scripted engine port (`test/fakes/engine.ts`), whose `viewFor` shows the opponent's
 * hand as a count — the one fact the hidden-information check reads besides the zones.
 */

import { describe, expect, it, vi } from "vitest";
import type { Aim, PlayerView } from "@jackioh/shared";
import type { ResultRow } from "../../src/api/ports";
import { AIM_RELAY_INTERVAL_MS } from "../../src/config";
import { aimIsPublic } from "../../src/match/actor";
import { createMatchClock } from "../../src/match/clock";
import type { ActorDeps, RecordResultInput } from "../../src/match/contracts";
import { parseClientMessage, type AimRelayMessage } from "../../src/match/protocol";
import { createMatchRegistry } from "../../src/match/registry";
import { createFakeEngine, fakeDeck } from "../fakes/engine";
import { createFakeSocket, type FakeSocket } from "../fakes/socket";
import { createTestDeps, TEST_CATALOG_VERSION } from "../fakes/deps";

const MATCH_ID = "match-aim";

async function harness() {
  const deps = createTestDeps();
  const actorDeps: ActorDeps = {
    store: deps.store,
    timers: deps.timers,
    config: deps.config,
    log: deps.log,
    engine: createFakeEngine(),
    createClock: createMatchClock,
    recordResult: vi.fn(
      async (input: RecordResultInput): Promise<ResultRow> => ({
        matchId: input.matchId,
        players: [input.seats[0].profileId, input.seats[1].profileId],
        winnerProfileId: null,
        reason: input.outcome.reason,
        turns: input.turns,
        endedAt: input.at,
        ratingBefore: [1000, 1000],
        ratingAfter: [1000, 1000],
      }),
    ) as unknown as ActorDeps["recordResult"],
  };
  const registry = createMatchRegistry(actorDeps);
  await registry.start({
    matchId: MATCH_ID,
    seed: "seed-aim",
    catalogVersion: TEST_CATALOG_VERSION,
    ranked: false,
    seats: [
      { profileId: "profile-1", player: "p1", deck: fakeDeck(["test-prompt-self"]) },
      { profileId: "profile-2", player: "p2", deck: fakeDeck(["test-prompt-enemy"]) },
    ],
  });
  const actor = await registry.actorFor(MATCH_ID);
  const p1 = createFakeSocket();
  const p2 = createFakeSocket();
  actor.attach("p1", p1);
  actor.attach("p2", p2);
  await actor.idle();
  p1.clear();
  p2.clear();
  return { deps, actor, p1, p2 };
}

function relays(socket: FakeSocket): AimRelayMessage[] {
  return socket.ofType<AimRelayMessage>("aim");
}

function errors(socket: FakeSocket): { code: string }[] {
  return socket.ofType<{ code: string }>("error");
}

function handCount(view: PlayerView, side: "you" | "opponent"): number {
  const hand = view[side].hand;
  return Array.isArray(hand) ? hand.length : hand.count;
}

const AT_HERO: Aim = {
  source: { at: "zone", player: "p1", row: "units", lane: 2 },
  target: { at: "hero", player: "p2" },
};

describe("R738 — the opponent's aim through the actor (§9.5)", () => {
  it("R738 relays an aim to the opponent alone, stamped with the sender's seat, and writes nothing", async () => {
    const { actor, p1, p2, deps } = await harness();
    p1.receiveJson({ type: "aim", aim: AT_HERO });
    await actor.idle();

    expect(relays(p2)).toEqual([{ type: "aim", from: "p1", aim: AT_HERO }]);
    expect(relays(p1)).toEqual([]);
    expect(errors(p1)).toEqual([]);
    expect(p1.ofType("ack")).toEqual([]);
    expect(deps.store.tables.matchActions).toEqual([]);
    // Never part of the view: no view frame went to either seat for it.
    expect(p1.ofType("view")).toEqual([]);
    expect(p2.ofType("view")).toEqual([]);

    deps.timers.advance(AIM_RELAY_INTERVAL_MS);
    p1.receiveJson({ type: "aim", aim: null });
    await actor.idle();
    expect(relays(p2).at(-1)).toEqual({ type: "aim", from: "p1", aim: null });
  });

  it("R738 shape-checks the frame: anything but public handles is malformed and relays nothing", async () => {
    const { actor, p1, p2 } = await harness();
    for (const aim of [
      { source: { at: "card", instanceId: "p1-h0" }, target: null },
      { source: { at: "zone", player: "p1", row: "units", lane: 0 }, target: null },
      { source: { at: "hand", player: "p1", index: -1 }, target: null },
      { source: { at: "hero", player: "p3" }, target: null },
      { source: { at: "hero", player: "p1" }, target: { at: "hand", player: "p2", index: 0 } },
      { source: { at: "hero", player: "p1" } },
      "aim",
    ]) {
      p1.receiveJson({ type: "aim", aim });
    }
    p1.receiveJson({ type: "aim" });
    await actor.idle();
    expect(errors(p1).map((error) => error.code)).toEqual(Array(8).fill("malformed"));
    expect(relays(p2)).toEqual([]);
  });

  it("R738 keeps only the handle's fields: a smuggled instance id or def id never reaches the opponent", () => {
    const parsed = parseClientMessage(
      JSON.stringify({
        type: "aim",
        aim: {
          source: { at: "hand", player: "p1", index: 0, instanceId: "p1-h0", defId: "core-001" },
          target: { at: "zone", player: "p2", row: "backrow", lane: 1, instanceId: "p2-b1" },
          extra: true,
        },
      }),
    );
    expect(parsed).toEqual({
      type: "aim",
      aim: {
        source: { at: "hand", player: "p1", index: 0 },
        target: { at: "zone", player: "p2", row: "backrow", lane: 1 },
      },
    });
  });

  it("R738 coalesces aims inside AIM_RELAY_INTERVAL_MS: the newest goes alone once the interval ends", async () => {
    const { actor, p1, p2, deps } = await harness();
    const second: Aim = { ...AT_HERO, target: { at: "zone", player: "p2", row: "units", lane: 3 } };
    const third: Aim = { ...AT_HERO, target: { at: "zone", player: "p2", row: "backrow", lane: 4 } };

    p1.receiveJson({ type: "aim", aim: AT_HERO });
    p1.receiveJson({ type: "aim", aim: second });
    p1.receiveJson({ type: "aim", aim: third });
    await actor.idle();
    expect(relays(p2).map((frame) => frame.aim)).toEqual([AT_HERO]);

    deps.timers.advance(AIM_RELAY_INTERVAL_MS - 1);
    expect(relays(p2)).toHaveLength(1);
    deps.timers.advance(1);
    expect(relays(p2).map((frame) => frame.aim)).toEqual([AT_HERO, third]);

    // A repeat of what the opponent was last told is not sent again.
    deps.timers.advance(AIM_RELAY_INTERVAL_MS);
    p1.receiveJson({ type: "aim", aim: third });
    await actor.idle();
    expect(relays(p2)).toHaveLength(2);
  });

  it("R738 keeps a window per seat: one seat's aims never spend the other's", async () => {
    const { actor, p1, p2 } = await harness();
    p1.receiveJson({ type: "aim", aim: AT_HERO });
    p2.receiveJson({ type: "aim", aim: { source: { at: "hero", player: "p2" }, target: { at: "hero", player: "p1" } } });
    await actor.idle();
    expect(relays(p2)).toHaveLength(1);
    expect(relays(p1)).toEqual([{ type: "aim", from: "p2", aim: { source: { at: "hero", player: "p2" }, target: { at: "hero", player: "p1" } } }]);
  });

  it("R738 drops an aim that names a hand card past the sender's hand, or the opponent's hand at all", async () => {
    const { actor, p1, p2, deps } = await harness();
    const count = handCount(actor.viewFor("p2"), "opponent");
    expect(count).toBeGreaterThan(0);

    const fromHand = (player: "p1" | "p2", index: number): Aim => ({
      source: { at: "hand", player, index },
      target: { at: "hero", player: "p2" },
    });
    p1.receiveJson({ type: "aim", aim: fromHand("p1", count) });
    await actor.idle();
    deps.timers.advance(AIM_RELAY_INTERVAL_MS);
    p1.receiveJson({ type: "aim", aim: fromHand("p2", 0) });
    await actor.idle();
    // Silently: no relay, no error.
    expect(relays(p2)).toEqual([]);
    expect(errors(p1)).toEqual([]);

    deps.timers.advance(AIM_RELAY_INTERVAL_MS);
    p1.receiveJson({ type: "aim", aim: fromHand("p1", count - 1) });
    await actor.idle();
    expect(relays(p2).map((frame) => frame.aim)).toEqual([fromHand("p1", count - 1)]);

    // An aim dropped while an arrow is up clears that arrow instead of leaving it standing.
    deps.timers.advance(AIM_RELAY_INTERVAL_MS);
    p1.receiveJson({ type: "aim", aim: fromHand("p1", count) });
    await actor.idle();
    expect(relays(p2).map((frame) => frame.aim)).toEqual([fromHand("p1", count - 1), null]);
  });

  it("R738 drops an aim at a zone past the board's lanes", () => {
    const view = {
      viewer: "p2",
      you: { hand: [], locks: { units: [false, false], backrow: [false] } },
      opponent: { hand: { count: 2 }, locks: { units: [false, false], backrow: [false] } },
    } as unknown as PlayerView;
    const at = (lane: number): Aim => ({ source: { at: "hero", player: "p1" }, target: { at: "zone", player: "p2", row: "backrow", lane } });
    expect(aimIsPublic(at(1), "p1", view)).toBe(true);
    expect(aimIsPublic(at(2), "p1", view)).toBe(false);
    expect(aimIsPublic({ source: { at: "hand", player: "p1", index: 1 }, target: null }, "p1", view)).toBe(true);
    expect(aimIsPublic({ source: { at: "hand", player: "p1", index: 2 }, target: null }, "p1", view)).toBe(false);
  });

  it("R738 clears the sender's arrow when its socket goes", async () => {
    const { actor, p1, p2 } = await harness();
    p1.receiveJson({ type: "aim", aim: AT_HERO });
    await actor.idle();
    p1.drop();
    await actor.idle();
    expect(relays(p2).map((frame) => frame.aim)).toEqual([AT_HERO, null]);
  });
});
