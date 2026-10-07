// What crossing into WebAssembly costs a state, piece by piece (.fullsend/notes/perf.md).
//
//   pnpm exec tsx .fullsend/notes/perf-bench/pass.mts "$PWD"
import { readFileSync } from "node:fs";

const repo = process.argv[2]!;
const w = await import(`${repo}/apps/web/src/wasm/index.ts`);
const pkg = await import(`${repo}/apps/web/src/wasm/pkg/jackioh_wasm.js`);
w.loadWasmSync(readFileSync(`${repo}/apps/web/src/wasm/pkg/jackioh_wasm_bg.wasm`));
const lines = readFileSync(`${repo}/crates/engine/tests/golden/games.jsonl`, "utf8")
  .split("\n")
  .filter((l) => l.length > 0)
  .map((l) => JSON.parse(l));
const line = lines[2];
let state = w.beginGame(w.createGame({ seed: line.args.seed, decks: line.args.decks })).state;
for (let i = 0; i < 100; i += 1) state = w.reduce(state, line.steps[i].a).state;
const text = JSON.stringify(state);
let firstNonAscii = -1;
for (let i = 0; i < text.length; i += 1) {
  if (text.charCodeAt(i) > 0x7f) {
    firstNonAscii = i;
    break;
  }
}
console.log(`state ${text.length} chars, first non-ASCII at ${firstNonAscii}`);
const enc = new TextEncoder();
const buf = new Uint8Array(text.length * 3);
function bench(name: string, run: () => unknown, iters = 500) {
  for (let i = 0; i < 50; i += 1) run();
  const c0 = process.cpuUsage();
  for (let i = 0; i < iters; i += 1) run();
  const c = process.cpuUsage(c0);
  console.log(`  ${name.padEnd(40)} ${((c.user + c.system) / 1000 / iters).toFixed(3)} ms cpu`);
}
bench("TextEncoder.encodeInto(text)", () => enc.encodeInto(text, buf));
bench("JS ASCII copy loop (wasm-bindgen's)", () => {
  let o = 0;
  for (; o < text.length; o += 1) {
    const code = text.charCodeAt(o);
    if (code > 0x7f) break;
    buf[o] = code;
  }
  return o;
});
bench("JSON.stringify(state)", () => JSON.stringify(state));
bench("seat_to_act(text) (cache hit)", () => pkg.seat_to_act(text));
// The wrapper's envelope (apps/web/src/wasm/index.ts `stateJson`): a no-break space third in the text.
const enveloped = JSON.stringify([" ", state]);
bench("seat_to_act(enveloped) (cache hit)", () => pkg.seat_to_act(enveloped));
bench("view_for(text) (cache hit)", () => pkg.view_for(text, "p1"));
bench("view_for(enveloped) (cache hit)", () => pkg.view_for(enveloped, "p1"));
bench("w.seatToAct(state) (stringify + cross)", () => w.seatToAct(state));
bench("w.viewFor(state)", () => w.viewFor(state, "p1"));
