// Temporary hand-written wrappers for part C's frozen Assault surface. Part B owns the equivalent
// wrappers in `wasm/index.ts`; the fullsend reconcile replaces these direct structural calls with
// those exports. Every argument and answer is JSON, exactly as the run wrappers are.

import * as normal from "../wasm/pkg/jackioh_wasm.js";

import type { PlayerId } from "../wire/index.ts";
import type { ReplayInput } from "../wasm/index.ts";
import { wasmLoaded } from "../wasm/index.ts";
import type {
  AssaultAction,
  AssaultActionResult,
  AssaultConstants,
  AssaultContent,
  AssaultDay,
  AssaultFightSetup,
  AssaultFile,
  AssaultRecord,
  AssaultResult,
  AssaultSetup,
  AssaultState,
} from "./types.ts";

type AssaultWasm = {
  assault_content(): string;
  assault_content_at(version: string): string;
  assault_day_deck(date: string): string;
  assault_day_seed(date: string): string;
  assault_day(date: string, content: string): string;
  assault_create(setup: string, record: string, day: string): string;
  assault_reduce(state: string, day: string, action: string): string;
  assault_fight_setup(state: string, day: string): string;
  assault_fight_report(game: string, human: string): string;
  assault_record(record: string, state: string): string;
  assault_fold(file: string, day: string): string;
  assault_constants(): string;
};

const assaultApi = normal as unknown as AssaultWasm;

function json(value: unknown): string {
  return JSON.stringify(value);
}

function parsed<T>(text: string): T {
  return JSON.parse(text) as T;
}

function call<T>(name: string, run: () => T): T {
  if (!wasmLoaded()) throw new Error(`the engine is not loaded: await loadWasm() before calling ${name}`);
  try {
    return run();
  } catch (cause) {
    if (typeof WebAssembly !== "undefined" && cause instanceof WebAssembly.RuntimeError) {
      throw new Error(`the engine panicked in ${name} (the console has its message): ${cause.message}`, { cause });
    }
    throw cause;
  }
}

export function assaultContent(): AssaultContent {
  return call("assaultContent", () => parsed<AssaultContent>(assaultApi.assault_content()));
}

export function assaultContentAt(version: string): AssaultContent | null {
  return call("assaultContentAt", () => parsed<AssaultContent | null>(assaultApi.assault_content_at(json(version))));
}

export function assaultDayDeck(date: string): string[] {
  return call("assaultDayDeck", () => parsed<string[]>(assaultApi.assault_day_deck(json(date))));
}

export function assaultDaySeed(date: string): string {
  return call("assaultDaySeed", () => parsed<string>(assaultApi.assault_day_seed(json(date))));
}

export function assaultDay(date: string, content: AssaultContent): AssaultDay {
  return call("assaultDay", () => parsed<AssaultDay>(assaultApi.assault_day(json(date), json(content))));
}

export function assaultCreate(setup: AssaultSetup, record: AssaultRecord, day: AssaultDay): AssaultResult {
  return call("assaultCreate", () =>
    parsed<AssaultResult>(assaultApi.assault_create(json(setup), json(record), json(day))),
  );
}

export function assaultReduce(state: AssaultState, day: AssaultDay, action: AssaultAction): AssaultResult {
  return call("assaultReduce", () =>
    parsed<AssaultResult>(assaultApi.assault_reduce(json(state), json(day), json(action))),
  );
}

export function assaultFightSetup(state: AssaultState, day: AssaultDay): AssaultFightSetup | null {
  return call("assaultFightSetup", () =>
    parsed<AssaultFightSetup | null>(assaultApi.assault_fight_setup(json(state), json(day))),
  );
}

export function assaultFightReport(game: ReplayInput, human: PlayerId): AssaultActionResult {
  return call("assaultFightReport", () =>
    parsed<AssaultActionResult>(assaultApi.assault_fight_report(json(game), json(human))),
  );
}

export function assaultRecord(record: AssaultRecord, state: AssaultState): AssaultRecord {
  return call("assaultRecord", () =>
    parsed<AssaultRecord>(assaultApi.assault_record(json(record), json(state))),
  );
}

export function assaultFold(file: AssaultFile, day: AssaultDay): AssaultResult {
  return call("assaultFold", () => parsed<AssaultResult>(assaultApi.assault_fold(json(file), json(day))));
}

export function assaultConstants(): AssaultConstants {
  return call("assaultConstants", () => parsed<AssaultConstants>(assaultApi.assault_constants()));
}
