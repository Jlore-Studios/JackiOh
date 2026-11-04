// R2882000: today's deal, one random practice deal a UTC day (SPEC §9.9): the day's seed and seat, a random deck at
// the setup's difficulty and pick, and the same game on every device, through the REAL practice core.

import { describe, expect, it } from "vitest";

import { AI_GATE_BUDGET } from "@jackioh/ai";
import type { ActionBody } from "@jackioh/shared";

import { createPracticeCore } from "./core.ts";
import type { PracticeDebug, PracticeResponse, PracticeSnapshot, PracticeStartConfig } from "./protocol.ts";
import { dailyDealConfig, dailyDealOf } from "./todaysDeal.ts";
import { snapshotOf } from "../test/fixtures.ts";

const TODAY = Date.parse("2026-11-03T12:00:00.000Z");
const YESTERDAY = Date.parse("2026-11-02T12:00:00.000Z");

/** Most requests one play sends: enough for the opening and a few turns of the AI's own decisions. */
const REQUESTS = 30;

/** The human's next intent: its mulligan kept whole, its turn ended, or the first answer to a prompt it owes. */
function humanMove(snapshot: PracticeSnapshot): ActionBody | undefined {
  const pending = snapshot.view.pending;
  if (pending === null || !pending.forYou) return snapshot.legal.find((action) => action.type === "endTurn");
  if (pending.kind === "mulligan") return { type: "mulligan", keep: pending.options.map((option) => option.key) };
  return snapshot.legal[0];
}

/** A fresh core on the gates' budget and a frozen clock, as core.test.ts's: start `config`, play `requests`, answer `debug`. */
function playOn(config: PracticeStartConfig, requests: number): PracticeDebug {
  const core = createPracticeCore({ now: () => 0, dev: true, budget: AI_GATE_BUDGET });
  let id = 0;
  const send = (body: Record<string, unknown>): PracticeResponse => {
    id += 1;
    return core.handle({ id, ...body } as Parameters<typeof core.handle>[0]);
  };
  let last = snapshotOf(send({ type: "start", config }));
  for (let n = 0; n < requests && last.view.result === null; n += 1) {
    if (last.aiToAct) {
      last = snapshotOf(send({ type: "aiStep" }));
      continue;
    }
    const action = humanMove(last);
    if (action === undefined) break;
    last = snapshotOf(send({ type: "act", action }));
  }
  const answer = send({ type: "debug" });
  if (answer.type !== "debug") throw new Error(`expected debug, got ${answer.type}`);
  return answer.debug;
}

describe("R2882000 today's deal", () => {
  it("R2882000 the deal is the UTC day's: seed and seat roll over at UTC midnight", () => {
    expect(dailyDealOf(Date.parse("2026-11-02T23:59:59.999Z"))).toEqual({
      day: "2026-11-02",
      seed: "daily-2026-11-02",
      humanSeat: "p2",
    });
    expect(dailyDealOf(Date.parse("2026-11-03T00:00:00.000Z"))).toEqual({
      day: "2026-11-03",
      seed: "daily-2026-11-03",
      humanSeat: "p1",
    });
    // The seat alternates every day, a month's end included: the day of the month's parity gives p1 on both of these.
    expect(dailyDealOf(Date.parse("2026-10-31T12:00:00.000Z")).humanSeat).toBe("p2");
    expect(dailyDealOf(Date.parse("2026-11-01T12:00:00.000Z")).humanSeat).toBe("p1");
    expect(dailyDealOf(Date.parse("2026-11-04T12:00:00.000Z")).humanSeat).toBe("p2");
    // 23:30 at UTC-5 is 04:30 UTC the next day.
    expect(dailyDealOf(Date.parse("2026-11-03T23:30:00-05:00")).day).toBe("2026-11-04");
  });

  it("R2882000 a day's deal is a random deck on the setup's difficulty and pick", () => {
    expect(dailyDealConfig(TODAY, "hard", null)).toEqual({
      seed: "daily-2026-11-03",
      difficulty: "hard",
      humanSeat: "p1",
      deck: { kind: "random" },
      dailyDeal: "2026-11-03",
    });
    expect(dailyDealConfig(TODAY, "easy", "Classic+").deck).toEqual({ kind: "random", leanSet: "Classic+" });
  });

  it("R2882000 two devices deal today's and yesterday's deal alike, and the two days differ", () => {
    const today = [playOn(dailyDealConfig(TODAY, "medium", null), 0), playOn(dailyDealConfig(TODAY, "medium", null), 0)];
    const yesterday = [
      playOn(dailyDealConfig(YESTERDAY, "medium", null), 0),
      playOn(dailyDealConfig(YESTERDAY, "medium", null), 0),
    ];
    expect(today[1]?.decks).toEqual(today[0]?.decks);
    expect(yesterday[1]?.decks).toEqual(yesterday[0]?.decks);
    expect(today[0]?.decks).not.toEqual(yesterday[0]?.decks);
    expect(today[0]?.seed).toBe("daily-2026-11-03");
    expect(yesterday[0]?.seed).toBe("daily-2026-11-02");
  });

  it("R2882000 two devices playing today's deal get the same AI moves", { timeout: 120_000 }, () => {
    const first = playOn(dailyDealConfig(TODAY, "medium", null), REQUESTS);
    const second = playOn(dailyDealConfig(TODAY, "medium", null), REQUESTS);
    expect(second.log).toEqual(first.log);
    expect(second.hash).toBe(first.hash);
    // The AI's own actions carry its nonces (`a…`); more than one means it played, not only dealt.
    expect(first.log.filter((action) => action.nonce.startsWith("a")).length).toBeGreaterThan(1);
  });
});
