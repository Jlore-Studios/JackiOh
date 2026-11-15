// The wrapper over the WebAssembly module (`./index.ts`, docs/v0.3.0/SURFACE.md §10.3): how it loads, the
// errors it names and what crosses it. Most of these import it afresh over a stand-in for `./pkg`
// (`vi.doMock`), so a test starts it unloaded, says what the module answers and sees what it is sent; a
// binding the stand-in leaves out is never reached. The engine behind it is proved by the suites that play
// games through it; the last describe here asks the loaded module itself.

import { afterEach, describe, expect, it, vi } from "vitest";

import { DECK_SIZE } from "../wire/engineConfig.ts";
import type { GameState } from "../wire/generated/GameState.ts";
import {
  buildAiDeck,
  catalogVersion,
  createGame,
  findInstance,
  runContent,
  runContentAt,
  seatPlayedBy,
  type DuosState,
} from "./index.ts";

type Wasm = typeof import("./index.ts");

const PKG = "./pkg/jackioh_wasm.js";

/** An argument no binding is reached with: the call throws first. */
const ANY = undefined as never;

/** `./index.ts` imported afresh, unloaded and with empty caches, over a module of `bindings`. */
async function fresh(bindings: Record<string, unknown> = {}): Promise<Wasm> {
  vi.resetModules();
  vi.doMock(PKG, () => ({ default: vi.fn(), initSync: vi.fn(), init: vi.fn(), ...bindings }));
  return await import("./index.ts");
}

/** As `fresh`, and loaded from bytes as `test/setup.ts` loads the real module. */
async function loaded(bindings: Record<string, unknown> = {}): Promise<Wasm> {
  const wasm = await fresh(bindings);
  wasm.loadWasmSync(new Uint8Array(0));
  return wasm;
}

/** `fetch`, answering every request with a response `respond` makes. */
function stubFetch(respond: () => Response) {
  const fetch = vi.fn((_input: URL) => Promise.resolve(respond()));
  vi.stubGlobal("fetch", fetch);
  return fetch;
}

/** The module's own `init` as `loadWasm` calls it: it waits for the response it is handed, and gives it to `take`. */
function initFrom(take: (response: Response) => void = () => undefined) {
  return vi.fn(async (options: { module_or_path: Promise<Response> }): Promise<void> => {
    take(await options.module_or_path);
  });
}

/** The error `run` throws. */
function thrown(run: () => unknown): Error {
  try {
    run();
  } catch (cause) {
    if (cause instanceof Error) return cause;
    throw new Error(`threw a non-Error: ${String(cause)}`, { cause });
  }
  throw new Error("threw nothing");
}

/** The error `pending` rejects with. */
async function failure(pending: Promise<unknown>): Promise<Error> {
  try {
    await pending;
  } catch (cause) {
    if (cause instanceof Error) return cause;
    throw new Error(`rejected with a non-Error: ${String(cause)}`, { cause });
  }
  throw new Error("resolved");
}

afterEach(() => {
  vi.doUnmock(PKG);
  vi.resetModules();
  vi.unstubAllGlobals();
});

/** Every wrapper, by the name its errors give it. */
const WRAPPERS: readonly (readonly [string, (wasm: Wasm) => unknown])[] = [
  ["createGame", (w) => w.createGame(ANY)],
  ["beginGame", (w) => w.beginGame(ANY)],
  ["reduce", (w) => w.reduce(ANY, ANY)],
  ["legalActions", (w) => w.legalActions(ANY, ANY)],
  ["viewFor", (w) => w.viewFor(ANY, ANY)],
  ["seatToAct", (w) => w.seatToAct(ANY)],
  ["hashState", (w) => w.hashState(ANY)],
  ["fold", (w) => w.fold(ANY)],
  ["replayOpen", (w) => w.replayOpen(ANY, ANY)],
  ["replayPage", (w) => w.replayPage(ANY, ANY, ANY, ANY)],
  ["lastBoardFor", (w) => w.lastBoardFor(ANY, ANY)],
  ["seatPlayedBy", (w) => w.seatPlayedBy(ANY, ANY)],
  ["findInstance", (w) => w.findInstance(ANY, ANY)],
  ["combatForecast", (w) => w.combatForecast(ANY, ANY)],
  ["dealEmoteHand", (w) => w.dealEmoteHand(ANY, ANY)],
  ["previewSets", (w) => w.previewSets(ANY)],
  ["registeredCatalog", (w) => w.registeredCatalog()],
  ["learnedParity", (w) => w.learnedParity(ANY)],
  ["catalogVersion", (w) => w.catalogVersion()],
  ["carriedVersions", (w) => w.carriedVersions()],
  ["runContent", (w) => w.runContent()],
  ["runContentAt", (w) => w.runContentAt(ANY)],
  ["runCheckSetup", (w) => w.runCheckSetup(ANY, ANY, ANY)],
  ["runCreate", (w) => w.runCreate(ANY, ANY, ANY)],
  ["runReduce", (w) => w.runReduce(ANY, ANY, ANY)],
  ["runFold", (w) => w.runFold(ANY, ANY)],
  ["runFightSetup", (w) => w.runFightSetup(ANY, ANY)],
  ["runFightReport", (w) => w.runFightReport(ANY, ANY)],
  ["runScore", (w) => w.runScore(ANY, ANY)],
  ["runEmbers", (w) => w.runEmbers(ANY)],
  ["runMaxHp", (w) => w.runMaxHp(ANY, ANY)],
  ["runShopPrices", (w) => w.runShopPrices(ANY, ANY)],
  ["runLevelCost", (w) => w.runLevelCost(ANY)],
  ["runConstants", (w) => w.runConstants()],
  ["runSeed", (w) => w.runSeed(ANY)],
  ["runDailySeed", (w) => w.runDailySeed(ANY, ANY)],
  ["duosCreate", (w) => w.duosCreate(ANY)],
  ["duosBegin", (w) => w.duosBegin(ANY)],
  ["duosReduce", (w) => w.duosReduce(ANY, ANY)],
  ["duosLegalActions", (w) => w.duosLegalActions(ANY, ANY)],
  ["duosViewFor", (w) => w.duosViewFor(ANY, ANY)],
  ["host", (w) => w.practiceCall(ANY)],
  ["constants", (w) => w.practiceConstants()],
  ["aiToAct", (w) => w.aiToAct(ANY, ANY)],
  ["decide", (w) => w.decide(ANY, ANY, ANY, ANY)],
  ["lethalHint", (w) => w.lethalHint(ANY, ANY, ANY)],
  ["buildAiDeck", (w) => w.buildAiDeck(ANY)],
  ["chooseAction", (w) => w.chooseAction(ANY, ANY, ANY)],
  ["craftPreview", (w) => w.craftPreview(ANY, ANY)],
  ["constants", (w) => w.aiConstants()],
  ["constants", (w) => w.dailyConstants()],
  ["engineTables", (w) => w.engineTables()],
  ["validateDeck", (w) => w.validator("validateDeck", ANY)],
  ["face", (w) => w.face(ANY)],
  ["log", (w) => w.log(ANY)],
];

describe("before the module is loaded", () => {
  it("every wrapper throws an error that names it, before it asks the module anything", async () => {
    const wasm = await fresh();
    expect(wasm.wasmLoaded()).toBe(false);
    for (const [name, invoke] of WRAPPERS) {
      expect(thrown(() => invoke(wasm)).message, name).toBe(`the engine is not loaded: await loadWasm() before calling ${name}`);
    }
  });
});

describe("loading", () => {
  it("runs what waits for the module once it loads from bytes, anything later at once, and loads once", async () => {
    const initSync = vi.fn();
    const init = vi.fn();
    const wasm = await fresh({ initSync, init });
    const early = vi.fn();
    wasm.whenWasmLoaded(early);
    expect(early).not.toHaveBeenCalled();

    const bytes = new Uint8Array(0);
    wasm.loadWasmSync(bytes);
    expect(wasm.wasmLoaded()).toBe(true);
    expect(initSync).toHaveBeenCalledWith({ module: bytes });
    expect(init).toHaveBeenCalledTimes(1);
    expect(early).toHaveBeenCalledTimes(1);

    const late = vi.fn();
    wasm.whenWasmLoaded(late);
    expect(late).toHaveBeenCalledTimes(1);

    wasm.loadWasmSync(bytes);
    expect(initSync).toHaveBeenCalledTimes(1);
    expect(init).toHaveBeenCalledTimes(1);
    expect(early).toHaveBeenCalledTimes(1);
  });

  it("#3183 downloads the compressed module once, hands the Worker's answer on as it is, and initialises it once", async () => {
    const response = new Response(new Uint8Array([0, 97, 115, 109]), {
      headers: { "Content-Type": "application/wasm", "Content-Encoding": "br" },
    });
    const fetch = stubFetch(() => response);
    let received: Response | undefined;
    const initWasm = initFrom((handed) => {
      received = handed;
    });
    const init = vi.fn();
    const wasm = await fresh({ default: initWasm, init });

    const first = wasm.loadWasm();
    expect(wasm.loadWasm()).toBe(first);
    await first;
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(String(fetch.mock.calls[0]?.[0])).toContain("/jackioh_wasm_bg.wasm.br");
    expect(received).toBe(response);
    expect(init).toHaveBeenCalledTimes(1);
    expect(wasm.wasmLoaded()).toBe(true);

    await wasm.loadWasm();
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(initWasm).toHaveBeenCalledTimes(1);
  });

  it("#3183 gives the wasm type to a module a static server sends with its encoding and no type", async () => {
    const bytes = new Uint8Array([0, 97, 115, 109]);
    const response = new Response(bytes, { headers: { "Content-Encoding": "br" } });
    stubFetch(() => response);
    let received: Response | undefined;
    const wasm = await fresh({
      default: initFrom((handed) => {
        received = handed;
      }),
    });

    await wasm.loadWasm();
    expect(received).toBeInstanceOf(Response);
    expect(received).not.toBe(response);
    expect(received?.headers.get("Content-Type")).toBe("application/wasm");
    expect(new Uint8Array((await received?.arrayBuffer()) ?? new ArrayBuffer(0))).toEqual(bytes);
  });

  it("#3183 refuses a module that arrived still compressed, drops its body, says why, and tries again on the next call", async () => {
    const response = new Response(new Uint8Array([1, 2, 3]));
    const body = response.body;
    if (body === null) throw new Error("a response of bytes has a body");
    const cancel = vi.spyOn(body, "cancel");
    const fetch = stubFetch(() => response);
    const wasm = await fresh({ default: initFrom() });

    const error = await failure(wasm.loadWasm());
    const url = String(fetch.mock.calls[0]?.[0]);
    expect(error.message).toBe(
      `the engine arrived still compressed from '${url}': the site Worker must serve /assets/*.wasm-*.br with Content-Encoding: br (wrangler.jsonc run_worker_first, apps/edge)`,
    );
    expect(cancel).toHaveBeenCalledTimes(1);
    expect(wasm.wasmLoaded()).toBe(false);

    await failure(wasm.loadWasm());
    expect(fetch).toHaveBeenCalledTimes(2);
  });

  it("names the status and the URL of a fetch that failed", async () => {
    const fetch = stubFetch(() => new Response("missing", { status: 404, statusText: "Not Found" }));
    const wasm = await fresh({ default: initFrom() });
    const error = await failure(wasm.loadWasm());
    expect(error.message).toBe(`failed to fetch the engine: 404 Not Found fetching '${String(fetch.mock.calls[0]?.[0])}'`);
  });

  it("refuses an answer with no body, whatever its type", async () => {
    const fetch = stubFetch(() => new Response(null, { status: 200, statusText: "OK", headers: { "Content-Type": "application/wasm" } }));
    const wasm = await fresh({ default: initFrom() });
    const error = await failure(wasm.loadWasm());
    expect(error.message).toBe(`failed to fetch the engine: 200 OK fetching '${String(fetch.mock.calls[0]?.[0])}'`);
    expect(wasm.wasmLoaded()).toBe(false);
  });

  it("initialises the module once when a load from bytes lands while a download is in flight", async () => {
    stubFetch(() => new Response(new Uint8Array([0]), { headers: { "Content-Type": "application/wasm" } }));
    let finish: () => void = () => undefined;
    const gate = new Promise<void>((resolve) => {
      finish = resolve;
    });
    const init = vi.fn();
    const wasm = await fresh({
      default: vi.fn(async (options: { module_or_path: Promise<Response> }): Promise<void> => {
        await options.module_or_path;
        await gate;
      }),
      init,
    });

    const download = wasm.loadWasm();
    wasm.loadWasmSync(new Uint8Array(0));
    expect(init).toHaveBeenCalledTimes(1);
    finish();
    await download;
    expect(init).toHaveBeenCalledTimes(1);
  });
});

describe("a call", () => {
  it("names a panic, keeping the trap as its cause", async () => {
    const trap = new WebAssembly.RuntimeError("unreachable");
    const wasm = await loaded({
      create_game: () => {
        throw trap;
      },
    });
    const error = thrown(() => wasm.createGame(ANY));
    expect(error.message).toBe("the engine panicked in createGame (the console has its message): unreachable");
    expect(error.cause).toBe(trap);
  });

  it("passes any other error on as it is", async () => {
    const refusal = new Error("a deck the engine refuses");
    const wasm = await loaded({
      create_game: () => {
        throw refusal;
      },
    });
    expect(thrown(() => wasm.createGame(ANY))).toBe(refusal);
  });
});

describe("what crosses", () => {
  const input = { seed: "input" } as unknown as GameState;
  const next = { seed: "next" };
  const event = { type: "turnStarted" };

  it("sends a state behind its mark, and hands back what the module answers when it accepts, a null error dropped", async () => {
    const beginGame = vi.fn((_state: string) => JSON.stringify({ state: next, events: [event], error: null }));
    const wasm = await loaded({ begin_game: beginGame });
    expect(wasm.beginGame(input)).toStrictEqual({ state: next, events: [event] });
    expect(beginGame).toHaveBeenCalledWith(JSON.stringify([" ", input]));
  });

  it("hands back the caller's own state and no events when the module refuses", async () => {
    const wasm = await loaded({
      reduce: (_state: string, _action: string) => JSON.stringify({ state: next, events: [event], error: "not your turn" }),
    });
    const result = wasm.reduce(input, ANY);
    expect(result).toStrictEqual({ state: input, events: [], error: "not your turn" });
    expect(result.state).toBe(input);
  });

  it("reads no seat to act as null", async () => {
    const seats = ["", "p2"];
    const wasm = await loaded({ seat_to_act: (_state: string) => seats.shift() });
    expect(wasm.seatToAct(input)).toBeNull();
    expect(wasm.seatToAct(input)).toBe("p2");
  });

  it("sends the sets to preview", async () => {
    const previewSets = vi.fn((_sets: string) => undefined);
    const wasm = await loaded({ preview_sets: previewSets });
    wasm.previewSets(["Meditative"]);
    expect(previewSets).toHaveBeenCalledWith(JSON.stringify(["Meditative"]));
  });
});

describe("Duos", () => {
  const state = { duel: "both" } as unknown as DuosState;
  const next = { duel: "next" };
  const event = { type: "teamLifeChanged" };
  const sent = JSON.stringify([" ", state]);

  it("creates, begins, lists and views through the module, a state behind its mark", async () => {
    const created = { duel: "created" };
    const view = { seat: "b2" };
    const bindings = {
      duos_create: vi.fn((_args: string) => JSON.stringify(created)),
      duos_begin: vi.fn((_state: string) => JSON.stringify({ state: next, events: [event] })),
      duos_legal_actions: vi.fn((_state: string, _seat: string) => JSON.stringify([{ type: "pass" }])),
      duos_view_for: vi.fn((_state: string, _seat: string) => JSON.stringify(view)),
    };
    const wasm = await loaded(bindings);
    const args = { seed: "duos", decks: { a1: [], a2: [], b1: [], b2: [] } };

    expect(wasm.duosCreate(args)).toStrictEqual(created);
    expect(bindings.duos_create).toHaveBeenCalledWith(JSON.stringify(args));
    expect(wasm.duosBegin(state)).toStrictEqual({ state: next, events: [event] });
    expect(bindings.duos_begin).toHaveBeenCalledWith(sent);
    expect(wasm.duosLegalActions(state, "a1")).toStrictEqual([{ type: "pass" }]);
    expect(bindings.duos_legal_actions).toHaveBeenCalledWith(sent, "a1");
    expect(wasm.duosViewFor(state, "b2")).toStrictEqual(view);
    expect(bindings.duos_view_for).toHaveBeenCalledWith(sent, "b2");
  });

  it("hands back the caller's own state and no events on a refusal, and drops a null error", async () => {
    const answers = [
      JSON.stringify({ state: next, events: [event], error: "not your turn" }),
      JSON.stringify({ state: next, events: [event], error: null }),
    ];
    const duosReduce = vi.fn((_state: string, _action: string) => answers.shift());
    const wasm = await loaded({ duos_reduce: duosReduce });

    const refused = wasm.duosReduce(state, ANY);
    expect(refused).toStrictEqual({ state, events: [], error: "not your turn" });
    expect(refused.state).toBe(state);
    expect(wasm.duosReduce(state, ANY)).toStrictEqual({ state: next, events: [event] });
    expect(duosReduce.mock.calls[0]?.[0]).toBe(sent);
  });
});

describe("the loaded module", () => {
  /** A game of one dealt deck for each seat, as `createGame` answers it, before it begins. */
  function game(): GameState {
    const { deck } = buildAiDeck({ rngSeed: "wasm-index", rngCursor: 0, size: DECK_SIZE });
    return createGame({ seed: "wasm-index", decks: [deck, deck] });
  }

  it("reads each seat as played by the player who began in it while no Glitch has swapped them", () => {
    const state = game();
    expect(seatPlayedBy(state, "p1")).toBe("p1");
    expect(seatPlayedBy(state, "p2")).toBe("p2");
  });

  it("finds a card by its id, and answers undefined, not null, for an id no card has", () => {
    const state = game();
    const card = state.players.p1.library[0];
    if (card === undefined) throw new Error("p1 was dealt no library");
    expect(findInstance(state, card.id)).toMatchObject({ id: card.id, defId: card.defId });
    expect(findInstance(state, "no-such-card")).toBeUndefined();
  });

  it("reads the run's content at the build's own catalog version, and none at a version it does not carry", () => {
    expect(runContentAt(catalogVersion())).toStrictEqual(runContent());
    expect(runContentAt("no-such-version")).toBeNull();
  });
});
