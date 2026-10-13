// The replay player's two sources (#511): the practice worker's `replay` answers and the server's
// `GET /api/replays/:id`, each turned into steps or a `ReplayFailure` with part 1's reasons.

import { afterEach, describe, expect, it, vi } from "vitest";

import { REPLAY_PAGE_STEPS } from "@jackioh/engine/config";
import { AI_GATE_BUDGET } from "@jackioh/ai";
import type { PlayerId } from "@jackioh/shared";

import { ApiRequestError, ApiUnreachableError, API_UNREACHABLE_MESSAGE, getReplaySteps } from "../net/api.ts";
import type { PracticeCoreEnv } from "../practice/core.ts";
import { createPracticeHost } from "../practice/host.ts";
import type { PracticeHost } from "../practice/host.ts";
import type { PracticeReplayListing, PracticeStartConfig } from "../practice/protocol.ts";
import { memorySaveStore } from "../practice/saveStore.ts";
import type { PracticeReplay, PracticeSaveStore } from "../practice/saveStore.ts";
import { foldedReplaySteps } from "../test/replay.ts";
import { ReplayFailure } from "./source.ts";
import { onlineFailure, onlineReplaySource, practiceFailure, practiceReplaySource } from "./sources.ts";

vi.mock("../net/api.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../net/api.ts")>();
  return { ...actual, getReplaySteps: vi.fn() };
});

const ENV: PracticeCoreEnv = { now: () => 0, dev: true, budget: AI_GATE_BUDGET };
const HUMAN: PlayerId = "p2";

const hosts: PracticeHost[] = [];

afterEach(() => {
  for (const host of hosts.splice(0)) host.dispose();
  vi.clearAllMocks();
});

function hostOn(saves: PracticeSaveStore): PracticeHost {
  const host = createPracticeHost({ forceInThread: true, env: { ...ENV, saves } });
  hosts.push(host);
  return host;
}

function config(seed: string): PracticeStartConfig {
  return { seed, difficulty: "easy", humanSeat: HUMAN, deck: { kind: "random" } };
}

/** A game started and conceded at once: the human loses, and the worker keeps it. */
async function concededGame(host: PracticeHost, seed: string): Promise<void> {
  await host.request({ type: "start", config: config(seed) });
  const done = await host.request({ type: "act", action: { type: "concede" } });
  if (done.type !== "snapshot" || done.snapshot.view.result === null) throw new Error("the conceded game did not end");
}

async function listed(host: PracticeHost): Promise<PracticeReplayListing[]> {
  const response = await host.request({ type: "replays" });
  if (response.type !== "replays") throw new Error(`expected replays, got ${response.type}`);
  return response.replays;
}

async function failureOf(page: Promise<unknown>): Promise<ReplayFailure> {
  const caught: unknown = await page.then(
    () => null,
    (cause: unknown) => cause,
  );
  if (!(caught instanceof ReplayFailure)) throw new Error(`expected a ReplayFailure, got ${String(caught)}`);
  return caught;
}

describe("the practice source", () => {
  it("the practice source pages a kept game as the worker answers it", async () => {
    const host = hostOn(memorySaveStore());
    await concededGame(host, "replay-source-kept");
    const [listing] = await listed(host);
    if (listing === undefined) throw new Error("the conceded game was not kept");
    const source = practiceReplaySource(host, listing);
    expect(source.total).toBe(listing.steps);

    const steps = [];
    while (steps.length < source.total) {
      const page = await source.page(steps.length, REPLAY_PAGE_STEPS);
      expect(page.length, "a page is never empty").toBeGreaterThan(0);
      expect(page.length).toBeLessThanOrEqual(REPLAY_PAGE_STEPS);
      expect(page[0]?.step).toBe(steps.length);
      steps.push(...page);
    }
    expect(steps.map((step) => step.step)).toEqual(Array.from({ length: listing.steps }, (_, k) => k));
    expect(steps.every((step) => step.view.viewer === HUMAN)).toBe(true);
    // The worker's own answer to the same request, step for step.
    const answer = await host.request({ type: "replay", game: listing.game, from: 0, count: REPLAY_PAGE_STEPS });
    if (answer.type !== "replay") throw new Error("expected a replay answer");
    expect(await source.page(0, REPLAY_PAGE_STEPS)).toEqual(answer.steps);
  });

  it("a game no longer kept is expired", async () => {
    const host = hostOn(memorySaveStore());
    const failure = await failureOf(practiceReplaySource(host, { game: 99, steps: 5 }).page(0, REPLAY_PAGE_STEPS));
    expect(failure.reason).toBe("expired");
  });

  it("an earlier catalog is earlier_patch and a changed fold is rules_changed", async () => {
    const source = memorySaveStore();
    await concededGame(hostOn(source), "replay-source-unavailable");
    const [kept] = source.readReplays();
    if (kept === undefined) throw new Error("the conceded game was not kept");
    const copy = (game: number, over: Partial<PracticeReplay>): PracticeReplay => ({
      ...structuredClone(kept),
      ...over,
      summary: { ...kept.summary, game },
    });
    const host = hostOn(memorySaveStore(null, [copy(1, { catalog: "v0.0.0-earlier" }), copy(2, { hash: "00000000" }), copy(3, {})]));

    const reasons = new Map<number, string>();
    for (const game of [1, 2, 3]) {
      const outcome = await practiceReplaySource(host, { game, steps: kept.summary.steps }).page(0, REPLAY_PAGE_STEPS).then(
        () => "shown",
        (cause: unknown) => (cause instanceof ReplayFailure ? cause.reason : "not a failure"),
      );
      reasons.set(game, outcome);
    }
    expect([...reasons]).toEqual([
      [1, "earlier_patch"],
      [2, "rules_changed"],
      [3, "shown"],
    ]);
  });

  it("a game in progress is an error carrying the worker's message", async () => {
    const saves = memorySaveStore();
    const host = hostOn(saves);
    await concededGame(host, "replay-source-before");
    const [listing] = await listed(host);
    if (listing === undefined) throw new Error("the conceded game was not kept");
    await host.request({ type: "start", config: config("replay-source-in-progress") });

    const failure = await failureOf(practiceReplaySource(host, listing).page(0, REPLAY_PAGE_STEPS));
    expect(failure.reason).toBe("error");
    expect(failure.message).toMatch(/in progress/u);
  });

  it("reads the worker's refusals by their wording, and anything else as an error", () => {
    expect(practiceFailure("practice game 4 cannot be replayed: earlier_patch").reason).toBe("earlier_patch");
    expect(practiceFailure("practice game 4 cannot be replayed: rules_changed").reason).toBe("rules_changed");
    expect(practiceFailure("no finished practice game 4 is kept on this device").reason).toBe("expired");
    const other = practiceFailure("the practice worker failed: boom");
    expect([other.reason, other.message]).toEqual(["error", "the practice worker failed: boom"]);
  });
});

describe("the online source", () => {
  const TOKEN = "token-p1";
  const steps = foldedReplaySteps();
  const listing = { matchId: "m-1", steps: steps.length };

  it("the online source asks getReplaySteps for the page", async () => {
    vi.mocked(getReplaySteps).mockResolvedValue({
      matchId: "m-1",
      from: REPLAY_PAGE_STEPS,
      steps: steps.slice(REPLAY_PAGE_STEPS, 2 * REPLAY_PAGE_STEPS),
      total: steps.length,
    });
    const source = onlineReplaySource(TOKEN, listing);
    expect(source.total).toBe(steps.length);
    expect(await source.page(REPLAY_PAGE_STEPS, REPLAY_PAGE_STEPS)).toEqual(steps.slice(REPLAY_PAGE_STEPS, 2 * REPLAY_PAGE_STEPS));
    expect(getReplaySteps).toHaveBeenCalledWith(TOKEN, "m-1", REPLAY_PAGE_STEPS, REPLAY_PAGE_STEPS);
  });

  it("410 gone is expired", async () => {
    vi.mocked(getReplaySteps).mockRejectedValue(new ApiRequestError(410, { code: "gone", message: "expired" }));
    const failure = await failureOf(onlineReplaySource(TOKEN, listing).page(0, REPLAY_PAGE_STEPS));
    expect(failure.reason).toBe("expired");
  });

  it("422 replay_unavailable carries its reason", async () => {
    for (const reason of ["earlier_patch", "rules_changed"] as const) {
      vi.mocked(getReplaySteps).mockRejectedValue(
        new ApiRequestError(422, { code: "replay_unavailable", message: `this replay cannot be shown: ${reason}`, details: { reason } }),
      );
      const failure = await failureOf(onlineReplaySource(TOKEN, listing).page(0, REPLAY_PAGE_STEPS));
      expect(failure.reason).toBe(reason);
    }
    // A refusal that names no reason of part 1's is an error with the server's own words.
    const nameless = onlineFailure(new ApiRequestError(422, { code: "replay_unavailable", message: "no reason given" }));
    expect([nameless.reason, nameless.message]).toEqual(["error", "no reason given"]);
  });

  it("an unreachable server is an error with its message", async () => {
    vi.mocked(getReplaySteps).mockRejectedValue(new ApiUnreachableError(new Error("Failed to fetch")));
    const failure = await failureOf(onlineReplaySource(TOKEN, listing).page(0, REPLAY_PAGE_STEPS));
    expect([failure.reason, failure.message]).toEqual(["error", API_UNREACHABLE_MESSAGE]);

    const limited = onlineFailure(new ApiRequestError(429, { code: "rate_limited", message: "slow down" }));
    expect([limited.reason, limited.message]).toEqual(["error", "slow down"]);
  });
});
