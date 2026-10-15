// R915006: a practice game with custom cards keeps them in its setup, so it resumes after a reload
// (R668) and replays once it is over (R768), each on the Luau module the worker switches to for it
// (R915009), whichever module was in use before.

import { afterEach, describe, expect, it } from "vitest";

import { AI_GATE_BUDGET } from "@jackioh/ai";
import type { CustomCard, PlayerId } from "@jackioh/shared";

import { luauBytes } from "../test/luau.ts";
import { luauModuleInUse, useLuauModule } from "../wasm/index.ts";
import { createPracticeCore } from "./core.ts";
import type { PracticeRequestBody, PracticeResponse, PracticeSnapshot, PracticeStartConfig } from "./protocol.ts";
import { memorySaveStore, type PracticeSaveStore } from "./saveStore.ts";

/** A 0-cost Spell: "Deal {dmg} damage to the enemy hero." */
const BURNER: CustomCard = {
  id: "custom-burner",
  version: 1,
  frame: {
    name: "Burner",
    type: "Spell",
    cost: 0,
    rarity: "Common",
    text: "Deal {dmg} damage to the enemy hero.",
    params: [{ key: "dmg", base: 3, radiant: 6 }],
  },
  source: 'return Card {\n  cry = function(c, n)\n    c:damage("the enemy hero", n.dmg)\n  end,\n}\n',
};

const HUMAN: PlayerId = "p1";

const CONFIG: PracticeStartConfig = {
  seed: "r915006-web",
  difficulty: "easy",
  humanSeat: HUMAN,
  deck: { kind: "try", card: BURNER.id },
  customCards: [BURNER],
};

afterEach(async () => {
  await useLuauModule(false);
});

/** The worker's core over `saves`, switching modules as the worker does before each request. */
function workerOver(saves: PracticeSaveStore) {
  const practice = createPracticeCore({ now: () => 0, dev: true, budget: AI_GATE_BUDGET, saves });
  let id = 0;
  return async (body: PracticeRequestBody): Promise<PracticeResponse> => {
    id += 1;
    const request = { id, ...body } as Parameters<typeof practice.handle>[0];
    const luau = practice.needsLuau(request);
    if (luau !== null) await useLuauModule(luau, luauBytes());
    return practice.handle(request);
  };
}

function snapshotOf(response: PracticeResponse): PracticeSnapshot {
  if (response.type === "started" || response.type === "snapshot") return response.snapshot;
  throw new Error(`expected a snapshot, got ${JSON.stringify(response).slice(0, 300)}`);
}

async function hashOf(send: (body: PracticeRequestBody) => Promise<PracticeResponse>): Promise<string> {
  const debug = await send({ type: "debug" });
  if (debug.type !== "debug") throw new Error("a dev core answers debug");
  return debug.debug.hash;
}

/** Plays on from `from`: the mulligans kept, the AI's steps taken, and the human's turn passed `passes` times. */
async function playOn(
  send: (body: PracticeRequestBody) => Promise<PracticeResponse>,
  from: PracticeSnapshot,
  passes: number,
): Promise<PracticeSnapshot> {
  let last = from;
  let passed = 0;
  for (let step = 0; step < 80 && passed < passes && last.view.result === null; step += 1) {
    const { view } = last;
    if (last.aiToAct) {
      last = snapshotOf(await send({ type: "aiStep" }));
      continue;
    }
    if (view.pending !== null && view.pending.forYou && view.pending.kind === "mulligan") {
      last = snapshotOf(await send({ type: "act", action: { type: "mulligan", keep: view.pending.options.map((option) => option.key) } }));
      continue;
    }
    const pass = last.legal.find((action) => action.type === "endTurn");
    if (pass === undefined) break;
    last = snapshotOf(await send({ type: "act", action: pass }));
    passed += 1;
  }
  return last;
}

describe("R915006 a practice game with custom cards resumes and replays", () => {
  it("R915006 a reload folds the saved game again on the Luau module, to the same state, with its custom card", async () => {
    const saves = memorySaveStore();
    const first = workerOver(saves);
    const started = snapshotOf(await first({ type: "start", config: CONFIG }));
    expect(luauModuleInUse()).toBe(true);
    await playOn(first, started, 2);
    const hash = await hashOf(first);
    expect(saves.read()?.config.customCards, "the worker's save keeps the cards").toEqual([BURNER]);
    expect(saves.read()?.log.length ?? 0).toBeGreaterThan(0);

    // The page reloads: a new worker, on the normal module, asked to pick the game up.
    await useLuauModule(false);
    const second = workerOver(saves);
    const resumed = await second({ type: "resume", config: CONFIG });
    expect(resumed.type, JSON.stringify(resumed).slice(0, 300)).toBe("started");
    expect(luauModuleInUse(), "a resume of a custom game runs on the Luau module").toBe(true);
    expect(await hashOf(second)).toBe(hash);
    expect(snapshotOf(resumed).view.defs?.[BURNER.id]?.name).toBe("Burner");
  });

  it("R915006 a finished custom game is kept, listed, and replayed on the Luau module", async () => {
    const saves = memorySaveStore();
    const send = workerOver(saves);
    const started = snapshotOf(await send({ type: "start", config: CONFIG }));
    const played = await playOn(send, started, 1);
    const over = snapshotOf(await send({ type: "act", action: { type: "concede" } }));
    expect(over.view.result).not.toBeNull();
    expect(played.view.result).toBeNull();
    expect(saves.readReplays()).toHaveLength(1);
    expect(saves.readReplays()[0]?.config.customCards).toEqual([BURNER]);

    await useLuauModule(false);
    const listed = await send({ type: "replays" });
    if (listed.type !== "replays") throw new Error(JSON.stringify(listed).slice(0, 300));
    expect(listed.replays).toHaveLength(1);
    const game = listed.replays[0]?.game ?? 0;
    expect(listed.replays[0]?.unavailable).toBeUndefined();

    const page = await send({ type: "replay", game, from: 0, count: 3 });
    expect(page.type, JSON.stringify(page).slice(0, 300)).toBe("replay");
    expect(luauModuleInUse(), "a custom game's replay folds on the Luau module").toBe(true);
    if (page.type === "replay") {
      // Step 0 and one per logged action (R768).
      expect(page.total).toBe((saves.readReplays()[0]?.log.length ?? -1) + 1);
      expect(page.steps.length).toBeGreaterThan(0);
    }
  });
});
