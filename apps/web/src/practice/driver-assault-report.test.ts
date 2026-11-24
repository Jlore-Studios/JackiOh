// R4582000 (issue #4578, the reconcile of parts A, B and D): an Infinite Assault fight end to end in
// the practice driver, with nothing mocked. The day is dealt by the WebAssembly module from the
// shipped stand-ins, the attempt's level 1 fight is set up by `assaultFightSetup`, the machine deals
// that setup (its seed, both decks and handicaps, the enemy's marks), and the finished fight's report,
// folded by the real `assaultFightReport`, moves the attempt: a conceded fight ends it lost.

import { describe, expect, it } from "vitest";

import { AI_GATE_BUDGET } from "@jackioh/ai";
import type { AssaultRecord, AssaultSetup } from "@jackioh/run";

import {
  assaultContent,
  assaultCreate,
  assaultDay,
  assaultDayDeck,
  assaultFightSetup,
  assaultReduce,
} from "../wasm/index.ts";
import { createPracticeDriver } from "./driver.ts";
import type { PracticeRequest, PracticeRequestBody, PracticeResponse, PracticeStartConfig } from "./protocol.ts";

const DATE = "2026-11-23";

describe("R4582000 an Assault fight plays its level and reports to the attempt", () => {
  it("R4582000 level 1's fight is dealt from its setup, and its report ends a conceded attempt lost", () => {
    const day = assaultDay(DATE, assaultContent());
    const setup: AssaultSetup = { date: DATE, deck: assaultDayDeck(DATE), source: "dayDeck", startLevel: 1, attempt: 1 };
    const record: AssaultRecord = { date: DATE, best: 0, checkpoints: [], attempts: 0, repelledOn: null };
    const created = assaultCreate(setup, record, day);
    if ("error" in created) throw new Error(created.error);
    const fighting = assaultReduce(created.state, day, { kind: "fight" });
    if ("error" in fighting) throw new Error(fighting.error);
    const fight = assaultFightSetup(fighting.state, day);
    if (fight === null) throw new Error("a fight phase has a fight setup");

    const core = createPracticeDriver({ now: () => 0, dev: true, budget: AI_GATE_BUDGET });
    let id = 0;
    const send = (body: PracticeRequestBody): PracticeResponse => {
      id += 1;
      return core.handle({ id, ...body } as PracticeRequest);
    };
    const config: PracticeStartConfig = { seed: fight.seed, difficulty: "easy", humanSeat: "p1", deck: { kind: "random" }, assault: fight };
    expect(send({ type: "start", config }).type).toBe("started");

    // The machine dealt the Assault's game, not a free one.
    const debug = send({ type: "debug" });
    if (debug.type !== "debug") throw new Error(`expected debug, got ${debug.type}`);
    expect(debug.debug.seed).toBe(`${day.seed}-L1-A1`);
    expect(debug.debug.decks).toEqual([fight.humanDeck, fight.enemyDeck]);
    expect(debug.debug.handicaps).toEqual({ p1: fight.humanHandicap, p2: fight.enemyHandicap });

    expect(send({ type: "act", action: { type: "concede" } }).type).toBe("snapshot");
    const answered = send({ type: "assaultReport" });
    if (answered.type !== "assaultReport" || !("action" in answered)) {
      throw new Error(`expected a report, got ${JSON.stringify(answered).slice(0, 400)}`);
    }
    expect(answered.action).toEqual(expect.objectContaining({ kind: "fightOver", won: false }));

    const after = assaultReduce(fighting.state, day, answered.action);
    if ("error" in after) throw new Error(after.error);
    expect(after.state.phase).toBe("lost");
  });
});
