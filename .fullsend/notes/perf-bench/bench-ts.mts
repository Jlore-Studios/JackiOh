// TS engine benchmark: the golden replay (the same work crates/engine/tests/golden.rs does per step:
// legalActions for the actor, reduce, hashState, viewFor x2) over the first N lines of games.jsonl,
// then a micro-benchmark of each call on one mid-game state.
//
//   pnpm exec tsx .fullsend/notes/perf-bench/bench-ts.mts "$PWD" [games=200] [line=2] [step=100] [iters=200]
// (perf.md ran 60 2 100 200-500; the TS engine is packages/engine, still in the tree for the port.)
import { readFileSync } from "node:fs";
import { performance } from "node:perf_hooks";

const repo = process.argv[2]!;
const GAMES = Number(process.argv[3] ?? 200);
const LINE = Number(process.argv[4] ?? 2);
const STEP = Number(process.argv[5] ?? 100);
const ITERS = Number(process.argv[6] ?? 200);

const engine = await import(`${repo}/packages/engine/src/index.ts`);
const cards = await import(`${repo}/packages/cards/src/index.ts`);
const { beginGame, createGame, hashState, legalActions, reduce, viewFor } = engine;
cards.registerAll();

type Line = { seed: string; args: { seed: string; decks: [string[], string[]]; handicaps: unknown }; steps: { a: Recorded }[] };
// A recorded action: the engine reads the whole object; the bench reads its type and actor.
type Recorded = { type: string; playerId: "p1" | "p2" } & Record<string, unknown>;
const lines: Line[] = readFileSync(`${repo}/crates/engine/tests/golden/games.jsonl`, "utf8")
  .split("\n")
  .filter((l) => l.length > 0)
  .map((l) => JSON.parse(l));

function setup(line: Line) {
  const args = line.args.handicaps === null
    ? { seed: line.args.seed, decks: line.args.decks }
    : { seed: line.args.seed, decks: line.args.decks, handicaps: line.args.handicaps };
  return beginGame(createGame(args)).state;
}

// ---- golden replay ----
const cpu0 = process.cpuUsage();
const t = { setup: 0, legal: 0, reduce: 0, hash: 0, view: 0 };
let steps = 0;
const wall0 = performance.now();
for (const line of lines.slice(0, GAMES)) {
  let t0 = performance.now();
  let state = setup(line);
  t.setup += performance.now() - t0;
  for (const step of line.steps) {
    t0 = performance.now();
    legalActions(state, step.a.playerId);
    const t1 = performance.now();
    const r = reduce(state, step.a);
    const t2 = performance.now();
    if (r.error !== undefined) throw new Error(`${line.seed}: refused ${r.error}`);
    state = r.state;
    hashState(state);
    const t3 = performance.now();
    viewFor(state, "p1");
    viewFor(state, "p2");
    const t4 = performance.now();
    t.legal += t1 - t0;
    t.reduce += t2 - t1;
    t.hash += t3 - t2;
    t.view += t4 - t3;
    steps += 1;
  }
}
const wall = performance.now() - wall0;
const cpu = process.cpuUsage(cpu0);
console.log(`golden replay: ${GAMES} games, ${steps} steps, wall ${(wall / 1000).toFixed(2)} s, cpu ${((cpu.user + cpu.system) / 1e6).toFixed(2)} s`);
console.log(
  `  per step (ms): legal ${(t.legal / steps).toFixed(3)}  reduce ${(t.reduce / steps).toFixed(3)}  hash ${(t.hash / steps).toFixed(3)}  view x2 ${(t.view / steps).toFixed(3)}  (setup total ${(t.setup / 1000).toFixed(2)} s)`,
);

// ---- micro-benchmark on one mid-game state ----
const line = lines[LINE]!;
const at = Math.min(STEP, line.steps.length - 1);
let state = setup(line);
for (let i = 0; i < at; i += 1) state = reduce(state, line.steps[i]!.a).state;
const next = line.steps[at]!.a;
const size = JSON.stringify(state).length;
console.log(`micro: ${line.seed} step ${at}, state ${(size / 1024).toFixed(1)} KB, next ${next.type} by ${next.playerId}, ${ITERS} iters`);

function bench(name: string, run: () => unknown) {
  for (let i = 0; i < Math.min(20, ITERS); i += 1) run();
  const c0 = process.cpuUsage();
  const t0 = performance.now();
  for (let i = 0; i < ITERS; i += 1) run();
  const ms = (performance.now() - t0) / ITERS;
  const c = process.cpuUsage(c0);
  console.log(`  ${name.padEnd(22)} ${ms.toFixed(3)} ms wall, ${((c.user + c.system) / 1000 / ITERS).toFixed(3)} ms cpu`);
}
bench("viewFor(p1)", () => viewFor(state, "p1"));
bench("viewFor(p2)", () => viewFor(state, "p2"));
bench("legalActions(actor)", () => legalActions(state, next.playerId));
bench("reduce(next)", () => reduce(state, next));
bench("hashState", () => hashState(state));
bench("JSON.stringify(state)", () => JSON.stringify(state));
