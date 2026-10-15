// R915006: a practice game with custom cards keeps them in its setup, so it resumes after a reload
// (R668) and replays once it is over (R768), each on the Luau module the worker switches to for it
// (R915009), whichever module was in use before. R962000: a sandbox game with a custom card spawns it
// on the same module.

import { afterEach, describe, expect, it } from "vitest";

import { AI_GATE_BUDGET } from "@jackioh/ai";
import { opponentOf, type ActionBody, type CustomCard, type PlayerId } from "@jackioh/shared";

import { luauBytes } from "../test/luau.ts";
import { fold, hashState, luauModuleInUse, useLuauModule } from "../wasm/index.ts";
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
    const played = await playOn(first, started, 2);
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
    // The same view as before the reload: a "try" deck is dealt (R433), so the card's definition is
    // in the view once the card has been shown, and the hash above holds the game's own copy.
    expect(snapshotOf(resumed).view.defs?.[BURNER.id]).toEqual(played.view.defs?.[BURNER.id]);
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

describe("R962000 a sandbox game spawns the custom cards it carries", () => {
  it("R962000 a sandbox game with a custom card spawns it on the Luau module, plays it as the dealt one, and folds with both", async () => {
    const send = workerOver(memorySaveStore());
    const seed = "r962000-web";
    let last = snapshotOf(await send({ type: "start", config: { ...CONFIG, seed, sandbox: true } }));
    expect(luauModuleInUse()).toBe(true);

    // The mulligans, keeping everything; then the human's main phase, the AI acting when it owes.
    for (let step = 0; step < 60; step += 1) {
      const { view } = last;
      if (last.aiToAct) {
        last = snapshotOf(await send({ type: "aiStep" }));
        continue;
      }
      if (view.pending !== null && view.pending.forYou && view.pending.kind === "mulligan") {
        last = snapshotOf(await send({ type: "act", action: { type: "mulligan", keep: view.pending.options.map((option) => option.key) } }));
        continue;
      }
      if (view.active === HUMAN && view.phase === "main" && view.pending === null) break;
      const pass = last.legal.find((action) => action.type === "endTurn");
      if (pass === undefined) throw new Error("the human owes something the test does not answer");
      last = snapshotOf(await send({ type: "act", action: pass }));
    }
    expect(last.view.active === HUMAN && last.view.phase === "main").toBe(true);

    const spawn = (defId: string): ActionBody => ({ type: "sandbox", op: { kind: "spawn", player: HUMAN, defId, to: { zone: "hand" } } });
    const refused = snapshotOf(await send({ type: "act", action: spawn("custom-other") }));
    expect(refused.error, "a custom card the game does not carry").toBe('there is no card "custom-other" to spawn');

    const handOf = (snapshot: PracticeSnapshot) => (Array.isArray(snapshot.view.you.hand) ? snapshot.view.you.hand : []);
    const before = new Set(handOf(last).map((card) => card.instanceId));
    const spawned = snapshotOf(await send({ type: "act", action: spawn(BURNER.id) }));
    expect(spawned.error).toBeNull();
    const fresh = handOf(spawned).filter((card) => !before.has(card.instanceId));
    expect(fresh.map((card) => card.defId)).toEqual([BURNER.id]);
    expect(spawned.view.defs?.[BURNER.id]?.set, "the view hands the page the game's definition").toBe("Custom");

    const health = spawned.view.opponent.hero.health;
    const play = spawned.legal.find(
      (action): action is Extract<ActionBody, { type: "play" }> => action.type === "play" && action.instanceId === fresh[0]?.instanceId,
    );
    expect(play, "the spawned card is playable").toBeDefined();
    if (play === undefined) return;
    const after = snapshotOf(await send({ type: "act", action: play }));
    expect(after.error).toBeNull();
    expect(after.view.opponent.hero.health, "its script ran on the Luau module").toBe(health - 3);
    expect(after.view.events.some((event) => event.type === "scriptFailed")).toBe(false);

    // Either seat: one spawned into the AI's hand is a card it holds, which it may read (R915007).
    const ai = opponentOf(HUMAN);
    const given = snapshotOf(await send({ type: "act", action: { type: "sandbox", op: { kind: "spawn", player: ai, defId: BURNER.id, to: { zone: "hand" } } } }));
    expect(given.error).toBeNull();
    let turn = snapshotOf(await send({ type: "act", action: { type: "endTurn" } }));
    const events = [...turn.view.events];
    for (let step = 0; step < 40 && turn.aiToAct; step += 1) {
      turn = snapshotOf(await send({ type: "aiStep" }));
      expect(turn.error).toBeNull();
      events.push(...turn.view.events);
    }
    expect(
      events.some((event) => event.type === "cardPlayed" && event.player === ai && event.defId === BURNER.id),
      "the AI played it on its turn",
    ).toBe(true);
    expect(events.some((event) => event.type === "scriptFailed")).toBe(false);

    // The log, the spawns in it, folds back to the same game with the custom card and the flag.
    const debug = await send({ type: "debug" });
    if (debug.type !== "debug") throw new Error("a dev core answers debug");
    const { decks, handicaps, log, dealt, hash, customCards, sandbox } = debug.debug;
    expect(sandbox).toBe(true);
    expect(customCards).toEqual([BURNER]);
    expect(log.some((action) => action.type === "sandbox")).toBe(true);
    const input = { seed, decks, handicaps, log, customCards, sandbox, ...(dealt === undefined ? {} : { dealt }) };
    expect(hashState(fold(input).state)).toBe(hash);
  });
});
