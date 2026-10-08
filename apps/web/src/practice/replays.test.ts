// R768, issue #509: the practice worker keeps each finished free game and replays it, through the
// in-thread host (the same core a Worker runs). A kept game's steps are `viewFor` of the human's seat
// after a fresh fold of that many accepted actions; nothing a response holds is the log, the seed, a
// deck or a card the human's view lacked at that step (CLAUDE.md rule 7); and every game it cannot
// vouch for is listed unavailable and refused.

import { afterEach, describe, expect, it } from "vitest";

import { REPLAY_PAGE_STEPS, createRng, fold, subsystems, viewFor } from "@jackioh/engine";
import type { GameState, ReplayStep } from "@jackioh/engine";
import { AI_GATE_BUDGET } from "@jackioh/ai";
import type { Action, PlayerId } from "@jackioh/shared";

import { replaySeatRun } from "./core.ts";
import type { PracticeCoreEnv } from "./core.ts";
import { PRACTICE_REPLAYS_KEPT } from "./config.ts";
import { createPracticeHost } from "./host.ts";
import type { PracticeHost } from "./host.ts";
import type {
  PracticeDebug,
  PracticeReplayListing,
  PracticeRequestBody,
  PracticeResponse,
  PracticeSnapshot,
  PracticeStartConfig,
} from "./protocol.ts";
import { memorySaveStore, type PracticeReplay, type PracticeSaveStore } from "./saveStore.ts";
import { TUTORIAL_LESSONS } from "../tutorial/lessons.ts";

const ENV: PracticeCoreEnv = { now: () => 0, dev: true, budget: AI_GATE_BUDGET };
const HUMAN: PlayerId = "p2";
const AI: PlayerId = "p1";

const hosts: PracticeHost[] = [];

afterEach(() => {
  for (const host of hosts.splice(0)) host.dispose();
});

function hostOn(saves: PracticeSaveStore, env: Partial<PracticeCoreEnv> = {}): PracticeHost {
  const host = createPracticeHost({ forceInThread: true, env: { ...ENV, ...env, saves } });
  hosts.push(host);
  return host;
}

function config(seed: string, over: Partial<PracticeStartConfig> = {}): PracticeStartConfig {
  return { seed, difficulty: "easy", humanSeat: HUMAN, deck: { kind: "random" }, ...over };
}

function snapshotOf(response: PracticeResponse): PracticeSnapshot {
  if (response.type === "started" || response.type === "snapshot") return response.snapshot;
  throw new Error(`expected a snapshot, got ${JSON.stringify(response).slice(0, 400)}`);
}

async function debug(host: PracticeHost): Promise<PracticeDebug> {
  const response = await host.request({ type: "debug" });
  if (response.type !== "debug") throw new Error(`expected debug, got ${JSON.stringify(response).slice(0, 400)}`);
  return response.debug;
}

async function listed(host: PracticeHost): Promise<PracticeReplayListing[]> {
  const response = await host.request({ type: "replays" });
  if (response.type !== "replays") throw new Error(`expected replays, got ${JSON.stringify(response).slice(0, 400)}`);
  return response.replays;
}

type Page = Extract<PracticeResponse, { type: "replay" }>;

async function page(host: PracticeHost, game: number, from: number, count: number): Promise<Page> {
  const response = await host.request({ type: "replay", game, from, count });
  if (response.type !== "replay") throw new Error(`expected replay, got ${JSON.stringify(response).slice(0, 400)}`);
  return response;
}

/** Every step of a kept game, page after page, each page checked on its way. */
async function allSteps(host: PracticeHost, game: number): Promise<{ steps: ReplayStep[]; total: number; wire: string[] }> {
  const steps: ReplayStep[] = [];
  const wire: string[] = [];
  let total = Infinity;
  while (steps.length < total) {
    const next = await page(host, game, steps.length, REPLAY_PAGE_STEPS);
    expect(Object.keys(next).sort()).toEqual(["from", "game", "id", "steps", "total", "type"]);
    expect(next.from).toBe(steps.length);
    expect(next.steps.length, "a page is never empty").toBeGreaterThan(0);
    steps.push(...next.steps);
    total = next.total;
    wire.push(JSON.stringify(next));
  }
  return { steps, total, wire };
}

async function failed(host: PracticeHost, body: PracticeRequestBody): Promise<string> {
  const response = await host.request(body);
  if (response.type !== "failed") throw new Error(`expected a refusal, got ${JSON.stringify(response).slice(0, 400)}`);
  return response.message;
}

/** A game started and conceded at once: the human loses. */
async function concededGame(host: PracticeHost, seed: string, over: Partial<PracticeStartConfig> = {}): Promise<void> {
  await host.request({ type: "start", config: config(seed, over) });
  const done = snapshotOf(await host.request({ type: "act", action: { type: "concede" } }));
  expect(done.view.result).not.toBeNull();
}

describe("R768 practice replays", () => {
  it(
    "R768 a finished free game is listed, and each step of its replay is viewFor of a fresh fold of that many accepted actions",
    { timeout: 120_000 },
    async () => {
      const seed = "r768-practice-walk";
      const portraits = { p1: "gary", p2: "timmy" } as const;
      const host = hostOn(memorySaveStore());
      let last = snapshotOf(await host.request({ type: "start", config: config(seed, { portraits }) }));
      const live = [last];
      const rng = createRng(`${seed}:human`);
      for (let n = 0; n < 40; n += 1) {
        const { state } = await debug(host);
        let response: PracticeResponse;
        if (last.aiToAct) response = await host.request({ type: "aiStep" });
        else {
          const body = subsystems.chooseAction(state as GameState, HUMAN, rng);
          if (body === null) break;
          response = await host.request({ type: "act", action: body });
        }
        last = snapshotOf(response);
        if ((await debug(host)).log.length === live.length) live.push(last);
      }
      expect(last.view.result, "the walk leaves the game in progress").toBeNull();
      last = snapshotOf(await host.request({ type: "act", action: { type: "concede" } }));
      live.push(last);

      const setup = await debug(host);
      const log = setup.log;
      expect(log.length).toBe(live.length - 1);
      expect(log.length).toBeGreaterThan(REPLAY_PAGE_STEPS);

      const replays = await listed(host);
      expect(replays).toEqual([{ game: 1, endedAt: 0, result: "loss", turns: last.view.turn, steps: live.length, portraits }]);
      expect(Object.keys(replays[0] ?? {}).sort()).toEqual(["endedAt", "game", "portraits", "result", "steps", "turns"]);

      const { steps, total, wire } = await allSteps(host, 1);
      expect(total).toBe(live.length);
      expect(steps).toHaveLength(live.length);
      steps.forEach((step, k) => {
        expect(step.step).toBe(k);
        const folded = fold({
          seed: setup.seed,
          decks: setup.decks,
          handicaps: setup.handicaps,
          ...(setup.dealt === undefined ? {} : { dealt: setup.dealt }),
          ...(setup.lastBoards === undefined ? {} : { lastBoards: setup.lastBoards }),
          log: log.slice(0, k),
        });
        expect(folded.errors).toEqual([]);
        expect(step.view, `step ${String(k)} is viewFor of a fresh fold`).toEqual(viewFor(folded.state, HUMAN));
        expect(step.view, `step ${String(k)} is what the human was shown`).toEqual((live[k] as PracticeSnapshot).view);
        expect(step.turn).toBe(step.view.turn);

        // Nothing of the AI's hand or library that the human's view lacked at this step.
        const shown = JSON.stringify(live[k]);
        const sent = JSON.stringify(step);
        const side = folded.state.players[AI];
        for (const card of [...side.hand, ...side.library]) {
          const id = JSON.stringify(card.id);
          if (shown.includes(id)) continue;
          expect(sent.includes(id), `the AI's hidden card ${id} is in step ${String(k)}`).toBe(false);
        }
      });

      // The boundary: no response carries the log, the seed, a deck or a handicap.
      const sent = [JSON.stringify(replays), ...wire, JSON.stringify(await host.request({ type: "replays" }))];
      for (const text of sent) {
        for (const forbidden of [seed, '"nonce"', '"log"', '"decks"', '"handicaps"']) {
          expect(text.includes(forbidden), `${forbidden} is in a replay response`).toBe(false);
        }
      }
    },
  );

  it("R768 both requests are refused while a game is in progress, and a lesson leaves nothing to replay", async () => {
    const saves = memorySaveStore();
    const host = hostOn(saves);
    expect(await listed(host), "no game yet, nothing kept").toEqual([]);

    await host.request({ type: "start", config: config("r768-refuse") });
    expect(await failed(host, { type: "replays" })).toContain("in progress");
    expect(await failed(host, { type: "replay", game: 1, from: 0, count: 16 })).toContain("in progress");

    const lesson = TUTORIAL_LESSONS[0];
    if (lesson === undefined) throw new Error("no tutorial lesson");
    await host.request({ type: "start", config: config("r768-lesson", { lesson: lesson.id }) });
    const done = snapshotOf(await host.request({ type: "act", action: { type: "concede" } }));
    expect(done.view.result).not.toBeNull();
    expect(await listed(host)).toEqual([]);
    expect(saves.readReplays()).toEqual([]);
    expect(await failed(host, { type: "replay", game: 1, from: 0, count: 16 })).toContain("no finished practice game 1");
  });

  it(
    "R768 only the last PRACTICE_REPLAYS_KEPT finished games are kept, and a fresh worker on the same store lists and replays them",
    { timeout: 120_000 },
    async () => {
      const saves = memorySaveStore();
      const host = hostOn(saves, { now: () => 5_000 });
      for (let n = 1; n <= PRACTICE_REPLAYS_KEPT + 1; n += 1) await concededGame(host, `r768-keep-${String(n)}`);

      const expected = Array.from({ length: PRACTICE_REPLAYS_KEPT }, (_, k) => PRACTICE_REPLAYS_KEPT + 1 - k);
      const replays = await listed(host);
      expect(replays.map((replay) => replay.game)).toEqual(expected);
      for (const replay of replays) {
        expect(replay.endedAt).toBe(5_000);
        expect(replay.result).toBe("loss");
        expect(replay.unavailable).toBeUndefined();
        expect(replay.portraits).toEqual({ p1: "vanilla", p2: "vanilla" });
      }
      expect(await failed(host, { type: "replay", game: 1, from: 0, count: 16 })).toContain("no finished practice game 1");

      // A reload: a new worker on the same store.
      const reloaded = hostOn(saves, { now: () => 5_000 });
      expect(await listed(reloaded)).toEqual(replays);
      const newest = replays[0] as PracticeReplayListing;
      const { steps, total } = await allSteps(reloaded, newest.game);
      expect(total).toBe(newest.steps);
      expect(steps.map((step) => step.step)).toEqual(Array.from({ length: newest.steps }, (_, k) => k));
    },
  );

  it("R768 a kept game from another catalog version, or one that no longer folds to its hash, is listed unavailable and refused", async () => {
    const source = memorySaveStore();
    await concededGame(hostOn(source), "r768-unavailable");
    const [kept] = source.readReplays();
    if (kept === undefined) throw new Error("the conceded game was not kept");

    const copy = (game: number, over: Partial<PracticeReplay>): PracticeReplay => ({
      ...structuredClone(kept),
      ...over,
      summary: { ...kept.summary, game },
    });
    const saves = memorySaveStore(null, [copy(1, { catalog: "v0.0.0-earlier" }), copy(2, { hash: "00000000" }), copy(3, {})]);
    const host = hostOn(saves);

    const replays = await listed(host);
    expect(replays.map((replay) => [replay.game, replay.unavailable])).toEqual([
      [3, undefined],
      [2, "rules_changed"],
      [1, "earlier_patch"],
    ]);
    expect(await failed(host, { type: "replay", game: 1, from: 0, count: 16 })).toContain("earlier_patch");
    expect(await failed(host, { type: "replay", game: 2, from: 0, count: 16 })).toContain("rules_changed");
    expect((await page(host, 3, 0, REPLAY_PAGE_STEPS)).steps.length).toBeGreaterThan(0);
  });

  it("R768 a page follows the seat the human played at each step and stops at a Glitch swap (R677)", () => {
    const endTurn = (playerId: PlayerId, nonce: string): Action => ({ type: "endTurn", playerId, nonce }) as Action;
    // The AI (a…) acts from p1 and the human (h…) from p2; then a swap puts the human in p1.
    const log = [endTurn("p1", "a0"), endTurn("p2", "h0"), endTurn("p1", "h1"), endTurn("p2", "a1")];
    expect(replaySeatRun(log, "p1", 0, 16)).toEqual({ seat: "p2", count: 2 });
    expect(replaySeatRun(log, "p1", 1, 1)).toEqual({ seat: "p2", count: 1 });
    expect(replaySeatRun(log, "p1", 2, 16)).toEqual({ seat: "p1", count: 3 });
    expect(replaySeatRun(log, "p1", 4, 16)).toEqual({ seat: "p1", count: 1 });
    expect(replaySeatRun(log.slice(0, 2), "p2", 0, 16)).toEqual({ seat: "p2", count: 3 });
  });
});
