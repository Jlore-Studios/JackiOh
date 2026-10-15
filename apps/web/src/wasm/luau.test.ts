// R915009, #919: the Luau module, the engine with the card kit, which the web loads only for a
// practice game with a custom card and for the creator. A test reads its bytes from disk
// (`test/luau.ts`), as `test/setup.ts` reads the normal module's.

import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import ts from "typescript";
import { afterEach, describe, expect, it } from "vitest";

import { AI_GATE_BUDGET } from "@jackioh/ai";
import type { ActionBody, CustomCard, PlayerId } from "@jackioh/shared";

import { createPracticeCore } from "../practice/core.ts";
import type { PracticeRequestBody, PracticeResponse, PracticeSnapshot, PracticeStartConfig } from "../practice/protocol.ts";
import { luauBytes } from "../test/luau.ts";
import { checkCustomCard, createGame, fold, hashState, luauModuleInUse, useLuauModule } from "./index.ts";

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

afterEach(async () => {
  await useLuauModule(false);
});

describe("R915009 the Luau module checks a custom card for the creator", () => {
  it("R915009 a card that may be saved has no problem, and its definition is of the Custom set", async () => {
    const check = await checkCustomCard(BURNER, luauBytes());
    expect(check.frame).toEqual([]);
    expect(check.script).toEqual([]);
    expect(check.def.id).toBe(BURNER.id);
    expect(check.def.set).toBe("Custom");
    expect(check.def.params).toEqual([{ key: "dmg", base: 3, radiant: 6, better: "up", min: 0 }]);
    expect(luauModuleInUse(), "a check loads the module but leaves the normal one in use").toBe(false);
  });

  it("R915009 a script's problem names its line, and a frame's names its field", async () => {
    const syntax = await checkCustomCard({ ...BURNER, source: "return Card {\n  cry = function(c, n)\n    c:draw(1\n  end,\n}\n" }, luauBytes());
    expect(syntax.script).toHaveLength(1);
    expect(syntax.script[0]?.line).toBe(4);
    const kit = await checkCustomCard({ ...BURNER, source: "return Card {\n  cri = function(c, n) end,\n}\n" }, luauBytes());
    expect(kit.script[0]?.message).toContain("Card knows");
    const frame = await checkCustomCard({ ...BURNER, frame: { ...BURNER.frame, cost: 99 } }, luauBytes());
    expect(frame.frame.map((issue) => issue.field)).toEqual(["cost"]);
  });
});

/** The practice worker's core, as the worker runs it, with the module it asks for switched in. */
function core() {
  const practice = createPracticeCore({ now: () => 0, dev: true, budget: AI_GATE_BUDGET });
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

const HUMAN: PlayerId = "p1";

function tryConfig(seed: string): PracticeStartConfig {
  return {
    seed,
    difficulty: "easy",
    humanSeat: HUMAN,
    deck: { kind: "try", card: BURNER.id },
    customCards: [BURNER],
  };
}

describe("R915009 a practice game with a custom card runs on the Luau module", () => {
  it("R915009 the normal module refuses the game, and the worker plays it on the Luau module against the AI, folding to its hash", async () => {
    const send = core();
    // A seed whose opening hand holds the card: the deal is the seed's, so the search is fixed.
    let opened: PracticeSnapshot | null = null;
    let seed = "";
    for (let n = 1; n <= 40 && opened === null; n += 1) {
      seed = `r915009-${String(n)}`;
      const started = snapshotOf(await send({ type: "start", config: tryConfig(seed) }));
      expect(luauModuleInUse()).toBe(true);
      const hand = Array.isArray(started.view.you.hand) ? started.view.you.hand : [];
      if (hand.some((card) => card.defId === BURNER.id)) opened = started;
    }
    if (opened === null) throw new Error("no seed in 40 deals the custom card into the opening hand");
    expect(opened.view.defs?.[BURNER.id]?.set, "the view hands the page the card's definition").toBe("Custom");

    // The mulligans, keeping everything; then the human's main phase, the AI acting when it owes.
    let last = opened;
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
    const before = last.view.opponent.hero.health;
    const play = last.legal.find(
      (action): action is Extract<ActionBody, { type: "play" }> =>
        action.type === "play" &&
        Array.isArray(last.view.you.hand) &&
        last.view.you.hand.some((card) => card.instanceId === action.instanceId && card.defId === BURNER.id),
    );
    expect(play, "the custom card is playable").toBeDefined();
    if (play === undefined) return;
    const after = snapshotOf(await send({ type: "act", action: play }));
    expect(after.error).toBeNull();
    expect(after.view.opponent.hero.health).toBe(before - 3);
    expect(after.view.events.some((event) => event.type === "scriptFailed")).toBe(false);

    // The AI takes its turn on the same module.
    const ended = snapshotOf(await send({ type: "act", action: { type: "endTurn" } }));
    let ai = ended;
    for (let step = 0; step < 40 && ai.aiToAct; step += 1) ai = snapshotOf(await send({ type: "aiStep" }));
    expect(ai.view.result === null || ai.view.result !== undefined).toBe(true);

    // The log folds back to the same game, the custom card with it; the normal module will not.
    const debug = await send({ type: "debug" });
    if (debug.type !== "debug") throw new Error("a dev core answers debug");
    const { decks, handicaps, log, dealt, hash, customCards } = debug.debug;
    expect(customCards, "the debug snapshot carries the game's custom cards, which its fold needs").toEqual([BURNER]);
    const input = { seed, decks, handicaps, log, customCards, ...(dealt === undefined ? {} : { dealt }) };
    expect(hashState(fold(input).state)).toBe(hash);
    await useLuauModule(false);
    expect(() => createGame({ seed, decks, customCards: [BURNER] })).toThrow(/Luau module/);
  });

  it("R915009 a game with no custom card stays on the normal module", async () => {
    const send = core();
    await send({ type: "start", config: { seed: "r915009-plain", difficulty: "easy", humanSeat: HUMAN, deck: { kind: "random" } } });
    expect(luauModuleInUse()).toBe(false);
  });
});

describe("R915009 no normal route loads the Luau module", () => {
  const SRC = join(dirname(fileURLToPath(import.meta.url)), "..");

  function sources(dir: string): string[] {
    return readdirSync(dir).flatMap((name) => {
      const path = join(dir, name);
      if (statSync(path).isDirectory()) return name === "test" || name === "pkg" ? [] : sources(path);
      return /\.(ts|tsx)$/u.test(name) && !/\.test\.tsx?$/u.test(name) ? [path] : [];
    });
  }

  /** The module names a file imports statically (`import … from`, `export … from`). */
  function staticImports(path: string): string[] {
    const text = readFileSync(path, "utf8");
    const file = ts.createSourceFile(path, text, ts.ScriptTarget.Latest, true, path.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS);
    const found: string[] = [];
    for (const node of file.statements) {
      if ((ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) && node.moduleSpecifier !== undefined && ts.isStringLiteral(node.moduleSpecifier)) {
        const typeOnly = ts.isImportDeclaration(node) ? node.importClause?.isTypeOnly === true : node.isTypeOnly;
        if (!typeOnly) found.push(node.moduleSpecifier.text);
      }
    }
    return found;
  }

  it("R915009 nothing imports the Luau module but by a dynamic import in wasm/index.ts", () => {
    const files = sources(SRC);
    expect(files.length).toBeGreaterThan(100);
    const importers = files
      .map((path) => relative(SRC, path))
      .filter((name) => name !== "wasm/luau.ts")
      .filter((name) => staticImports(join(SRC, name)).some((specifier) => /luau\.ts|pkg\/luau/u.test(specifier)));
    expect(importers).toEqual([]);
    const index = readFileSync(join(SRC, "wasm/index.ts"), "utf8");
    expect(index).toContain('import("./luau.ts")');
  });
});
