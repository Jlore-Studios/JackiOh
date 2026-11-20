// Reference host for the engine's C module (`crates/wasm/c`, #3093): plain
// Node ESM with no dependencies, driving the built `jackioh_wasm_c.wasm`
// through its raw C ABI exactly as any non-JS host does, following
// `crates/wasm/c/README.md` and `luau/src/Host.luau`.
//
//   node luau/tools/reference-host.mjs fixture <wasm>
//   node luau/tools/reference-host.mjs traces <wasm> <file>
//
// `fixture` folds the hotseat fixture with `fold`, replays its log step by
// step with `reduce`, and holds both final hashes to `HOTSEAT_HASH` (read
// from `crates/engine/src/testkit/golden.rs`, never restated). `traces`
// replays a `cargo jackioh golden record` file step by step, holding
// `hash_state` to `begin.s` and each step's `s` (SURFACE §5.2), ignoring `l`,
// `v` and `e`, and exiting 1 on the first mismatch. Both print a short
// report: the module's size, its load and `init` times, games, steps, and
// the median and worst per-step `reduce` + `hash_state` time.
//
// State stays exact text throughout: `splitState` cuts the `state` member
// out of a `{"state":…,…}` answer (`begin_game`, `reduce`, `fold`) as a
// substring, never decoded and re-encoded, since the module caches states
// by exact text.

import { readFileSync } from "node:fs";

const PANIC_PREFIX = "panicked at ";
const STATUS_FAILED = 1;

// The script lives at `luau/tools/reference-host.mjs`, so the repository
// root is two levels up from it, wherever the caller runs node from.
const REPO_ROOT = new URL("../..", import.meta.url);
const HOTSEAT_FIXTURE = new URL("crates/engine/tests/golden/01-hotseat-full-game.json", REPO_ROOT);
const GOLDEN_SOURCE = new URL("crates/engine/src/testkit/golden.rs", REPO_ROOT);

// Read the bytes, record their length, compile, check the module imports
// nothing, instantiate it with an empty import object, and call `init()`
// once. Load (read + compile + instantiate) and `init` are timed separately.
function loadModule(wasmPath) {
  const loadStarted = performance.now();
  const bytes = readFileSync(wasmPath);
  const size = bytes.length;
  const module = new WebAssembly.Module(bytes);
  const imports = WebAssembly.Module.imports(module);
  if (imports.length > 0) {
    const names = imports.map((entry) => `${entry.module}.${entry.name}`).join(", ");
    throw new Error(`the C module imports ${names}; it must import nothing`);
  }
  const instance = new WebAssembly.Instance(module, {});
  const loadedMs = performance.now() - loadStarted;
  const initStarted = performance.now();
  instance.exports.init();
  const initMs = performance.now() - initStarted;
  return { host: { exports: instance.exports, dead: false }, size, loadMs: loadedMs, initMs };
}

// A fresh view over the module's memory: `memory.grow` during a call
// replaces the buffer, so a view kept across a call may read stale bytes.
function freshMemory(exports) {
  return new Uint8Array(exports.memory.buffer);
}

// Copy the last binding's answer out of the result buffer. The buffer is
// the module's, never freed, and replaced by the next call, so the text is
// copied before anything else runs. Length 0 answers nothing: never touch
// the pointer then.
function readResult(exports) {
  const len = exports.jackioh_result_len();
  if (len === 0) {
    return "";
  }
  const ptr = exports.jackioh_result_ptr();
  return new TextDecoder().decode(freshMemory(exports).slice(ptr, ptr + len));
}

// Call one binding: each string argument crosses as a `(ptr, len)` pair
// written through `jackioh_alloc` (an empty string is `(0, 0)` with no
// buffer), each number argument crosses as a number. Status 1 throws
// `<name> refused: <message>`; a trap marks the instance dead (every later
// call refuses, even though the exports stay callable) and throws
// `<name> trapped: <text>`, where the text is the result buffer's only when
// it starts with `panicked at `.
function call(host, name, ...args) {
  if (host.dead) {
    throw new Error(`${name} refused: instance is dead; open a fresh one`);
  }
  const binding = host.exports[name];
  if (binding === undefined) {
    throw new Error(`${name} refused: unknown binding`);
  }
  for (let i = 0; i < args.length; i++) {
    const kind = typeof args[i];
    if (kind !== "string" && kind !== "number") {
      throw new Error(`${name} refused: argument ${i + 1} is not a string or number`);
    }
  }
  const encoder = new TextEncoder();
  const parts = [];
  const owned = [];
  for (const arg of args) {
    if (typeof arg === "number") {
      parts.push(arg);
    } else if (arg.length === 0) {
      parts.push(0, 0);
    } else {
      const bytes = encoder.encode(arg);
      if (bytes.length === 0) {
        parts.push(0, 0);
      } else {
        const ptr = host.exports.jackioh_alloc(bytes.length);
        freshMemory(host.exports).set(bytes, ptr);
        parts.push(ptr, bytes.length);
        owned.push([ptr, bytes.length]);
      }
    }
  }
  let status;
  try {
    status = binding(...parts);
  } catch (trap) {
    void trap;
    for (const [ptr, len] of owned) {
      host.exports.jackioh_free(ptr, len);
    }
    host.dead = true;
    let text;
    try {
      text = readResult(host.exports);
      if (!text.startsWith(PANIC_PREFIX)) {
        text = "no text";
      }
    } catch (unreadable) {
      void unreadable;
      text = "no text";
    }
    throw new Error(`${name} trapped: ${text}`);
  }
  const answer = readResult(host.exports);
  for (const [ptr, len] of owned) {
    host.exports.jackioh_free(ptr, len);
  }
  if (status === STATUS_FAILED) {
    throw new Error(`${name} refused: ${answer}`);
  }
  return answer;
}

function skipSpace(s, pos) {
  while (pos < s.length) {
    const c = s[pos];
    if (c !== " " && c !== "\t" && c !== "\n" && c !== "\r") {
      break;
    }
    pos++;
  }
  return pos;
}

// `s[pos]` is the opening quote. Returns the closing quote's index, skipping
// `\X` escape pairs so an escaped quote never ends the string early.
function scanString(s, pos) {
  let i = pos + 1;
  while (i < s.length) {
    const code = s.charCodeAt(i);
    if (code === 92) {
      i += 2;
    } else if (code === 34) {
      return i;
    } else {
      i++;
    }
  }
  throw new Error("splitState: unterminated string");
}

// Returns the last index of the JSON value starting at `pos`. Objects and
// arrays are walked with `scanString`/`scanValue` recursion so braces inside
// strings never count; anything else (a number, `true`, `false`, `null`)
// runs to the next `,`, `}` or `]`.
function scanValue(s, pos) {
  pos = skipSpace(s, pos);
  if (pos >= s.length) {
    throw new Error("splitState: bad JSON");
  }
  const c = s[pos];
  if (c === '"') {
    return scanString(s, pos);
  }
  if (c === "{") {
    let i = skipSpace(s, pos + 1);
    if (s[i] === "}") {
      return i;
    }
    for (;;) {
      i = skipSpace(s, i);
      if (s[i] !== '"') {
        throw new Error("splitState: bad JSON");
      }
      i = scanString(s, i) + 1;
      i = skipSpace(s, i);
      if (s[i] !== ":") {
        throw new Error("splitState: bad JSON");
      }
      i = scanValue(s, i + 1);
      i = skipSpace(s, i + 1);
      const d = s[i];
      if (d === ",") {
        i++;
      } else if (d === "}") {
        return i;
      } else {
        throw new Error("splitState: bad JSON");
      }
    }
  }
  if (c === "[") {
    let i = skipSpace(s, pos + 1);
    if (s[i] === "]") {
      return i;
    }
    for (;;) {
      i = scanValue(s, i);
      i = skipSpace(s, i + 1);
      const d = s[i];
      if (d === ",") {
        i++;
      } else if (d === "]") {
        return i;
      } else {
        throw new Error("splitState: bad JSON");
      }
    }
  }
  let stop = s.length;
  for (const mark of [",", "}", "]"]) {
    const at = s.indexOf(mark, pos);
    if (at !== -1 && at < stop) {
      stop = at;
    }
  }
  if (stop === s.length) {
    throw new Error("splitState: bad JSON");
  }
  return stop - 1;
}

// Cut the `state` member out of a `{"state":…,…}` answer (`begin_game`,
// `reduce`, `fold`) as its exact substring. Never decode and re-encode it:
// a re-encode can change the text, and the module caches states by exact
// text.
function splitState(answer) {
  let i = skipSpace(answer, 0);
  if (answer[i] !== "{") {
    throw new Error("splitState: answer is not an object");
  }
  i = skipSpace(answer, i + 1);
  if (answer[i] === "}") {
    throw new Error("splitState: no state member");
  }
  for (;;) {
    i = skipSpace(answer, i);
    if (answer[i] !== '"') {
      throw new Error("splitState: bad JSON");
    }
    const keyEnd = scanString(answer, i);
    const key = answer.slice(i, keyEnd + 1);
    i = skipSpace(answer, keyEnd + 1);
    if (answer[i] !== ":") {
      throw new Error("splitState: bad JSON");
    }
    const valueStart = skipSpace(answer, i + 1);
    const valueEnd = scanValue(answer, valueStart);
    if (key === '"state"') {
      return answer.slice(valueStart, valueEnd + 1);
    }
    i = skipSpace(answer, valueEnd + 1);
    const d = answer[i];
    if (d === ",") {
      i++;
    } else if (d === "}") {
      throw new Error("splitState: no state member");
    } else {
      throw new Error("splitState: bad JSON");
    }
  }
}

// `HOTSEAT_HASH` as `crates/engine/src/testkit/golden.rs` states it, read
// from that file so the value is never restated here.
function readHotseatHash() {
  const source = readFileSync(GOLDEN_SOURCE, "utf8");
  const found = source.match(/pub const HOTSEAT_HASH: &str = "([^"]*)"/);
  if (found === null) {
    throw new Error("golden.rs holds no HOTSEAT_HASH");
  }
  return found[1];
}

// `hash_state` answers the bare hash text (`wasm_core::hash_state` returns
// the hash itself and the boundary writes it verbatim, unquoted), so compare
// it directly.
function hashOf(host, stateText) {
  return call(host, "hash_state", stateText);
}

function report(measured) {
  const { size, loadMs, initMs, games, steps, samples } = measured;
  const sorted = [...samples].sort((a, b) => a - b);
  const median = sorted.length === 0 ? 0 : sorted[Math.floor(sorted.length / 2)];
  const worst = sorted.length === 0 ? 0 : sorted[sorted.length - 1];
  console.log(`size: ${size} B`);
  console.log(`load: ${loadMs.toFixed(1)} ms`);
  console.log(`init: ${initMs.toFixed(1)} ms`);
  console.log(`games: ${games}`);
  console.log(`steps: ${steps}`);
  console.log(`reduce+hash median: ${median.toFixed(1)} ms`);
  console.log(`reduce+hash worst: ${worst.toFixed(1)} ms`);
}

function runFixture(wasmPath) {
  const { host, size, loadMs, initMs } = loadModule(wasmPath);
  const expected = readHotseatHash();
  const fixtureText = readFileSync(HOTSEAT_FIXTURE, "utf8");
  const fixture = JSON.parse(fixtureText);

  const foldAnswer = call(host, "fold", fixtureText);
  const folded = JSON.parse(foldAnswer);
  if (!Array.isArray(folded.errors) || folded.errors.length > 0) {
    const first = Array.isArray(folded.errors) ? folded.errors[0] : folded.errors;
    throw new Error(`the hotseat fixture's fold refused: ${JSON.stringify(first)}`);
  }
  const foldHash = hashOf(host, splitState(foldAnswer));
  if (foldHash !== expected) {
    throw new Error(`the hotseat fixture folds to ${foldHash}, not ${expected}`);
  }

  // The fixture's decks have 20 cards (R1705003): made before `DECK_SIZE`
  // became 25, they fold as a recorded game does, while `create_game` makes
  // a game now and refuses them.
  let refused = false;
  try {
    call(host, "create_game", JSON.stringify({ seed: fixture.seed, decks: fixture.decks }));
  } catch (error) {
    if (!(error instanceof Error) || !error.message.includes("refused")) {
      throw error;
    }
    refused = true;
  }
  if (!refused) {
    throw new Error("create_game answered for the fixture's 20-card decks; expected a refusal");
  }

  // The step-by-step replay opens as `create_recorded_game` does: `fold` of
  // the fixture's setup and an empty log, then one `reduce` per action.
  const samples = [];
  let s = splitState(
    call(host, "fold", JSON.stringify({ seed: fixture.seed, decks: fixture.decks, log: [] })),
  );
  for (let n = 0; n < fixture.log.length; n++) {
    const started = performance.now();
    const r = call(host, "reduce", s, JSON.stringify(fixture.log[n]));
    const reduced = JSON.parse(r);
    if (reduced.error != null) {
      throw new Error(`step ${n}: reduce refused: ${reduced.error}`);
    }
    s = splitState(r);
    const stepHash = hashOf(host, s);
    samples.push(performance.now() - started);
    if (n === fixture.log.length - 1 && stepHash !== expected) {
      throw new Error(`the replayed fixture reaches ${stepHash}, not ${expected}`);
    }
  }
  report({ size, loadMs, initMs, games: 1, steps: fixture.log.length, samples });
}

// Hold one hashed state to its recorded hash. Prints the seed, step, action,
// expected and got, and answers whether the replay may go on.
function checkHash(host, seed, step, action, expected, stateText) {
  const got = hashOf(host, stateText);
  if (got !== expected) {
    console.error(
      `mismatch: seed=${seed} step=${step} action=${action} expected=${expected} got=${got}`,
    );
    process.exitCode = 1;
    return false;
  }
  return true;
}

function runTraces(wasmPath, tracesPath) {
  const { host, size, loadMs, initMs } = loadModule(wasmPath);
  const text = readFileSync(tracesPath, "utf8");
  const lines = text.split("\n").filter((line) => line !== "");
  const samples = [];
  let steps = 0;
  for (const line of lines) {
    const game = JSON.parse(line);
    const created = call(host, "create_game", JSON.stringify(game.args));
    const s = splitState(call(host, "begin_game", created));
    if (!checkHash(host, game.seed, "begin", null, game.begin.s, s)) {
      return;
    }
    let state = s;
    for (let i = 0; i < game.steps.length; i++) {
      const step = game.steps[i];
      const a = JSON.stringify(step.a);
      const started = performance.now();
      const r = call(host, "reduce", state, a);
      const reduced = JSON.parse(r);
      if (reduced.error != null) {
        console.error(
          `mismatch: seed=${game.seed} step=${i} action=${a} expected=${step.s} got=refused: ${reduced.error}`,
        );
        process.exitCode = 1;
        return;
      }
      state = splitState(r);
      const ok = checkHash(host, game.seed, String(i), a, step.s, state);
      samples.push(performance.now() - started);
      steps++;
      if (!ok) {
        return;
      }
    }
  }
  report({ size, loadMs, initMs, games: lines.length, steps, samples });
}

function usage() {
  console.error("usage: reference-host.mjs fixture <wasm>");
  console.error("       reference-host.mjs traces <wasm> <file>");
}

function main() {
  const [mode, ...rest] = process.argv.slice(2);
  try {
    if (mode === "fixture" && rest.length === 1) {
      runFixture(rest[0]);
    } else if (mode === "traces" && rest.length === 2) {
      runTraces(rest[0], rest[1]);
    } else {
      usage();
      process.exitCode = 2;
    }
  } catch (error) {
    console.error(`${mode ?? "?"} failed: ${error instanceof Error ? error.message : error}`);
    process.exitCode = 1;
  }
}

main();
