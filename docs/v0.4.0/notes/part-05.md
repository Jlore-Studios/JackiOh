# v0.4.0 Part 5: Luau in the browser — the emscripten module, its size and speed, and the go/no-go

Notes for issue #447 (v0.4.0 part 5 of 29): `crates/wasm` rebuilt for `wasm32-unknown-emscripten`,
measured against the branch point, and the go/no-go.

## 1. What changed

- `crates/wasm` is a binary now (`src/main.rs`; `src/lib.rs` deleted): `rustc` only emits
  emscripten's JavaScript glue when the `emcc` link produces a program — a `cdylib` link through
  `emcc` yields a bare `.wasm` with no glue (proven by experiment). Every binding is a
  `#[unsafe(no_mangle)] extern "C"` function over NUL-terminated UTF-8 C strings with the same
  names and JSON as SURFACE §10.1, plus `jackioh_free` (frees answers and spent argument buffers —
  both live on the one emscripten heap, Rust's allocator here being libc's `malloc`) and
  `jackioh_last_error` (a failure answers NULL and stows its message there). Dropped
  `wasm-bindgen`, `js-sys` and `console_error_panic_hook` from the crate and the workspace: the
  panic hook writes through `emscripten_console_error` (stderr off-target, so host `cargo test`
  and `clippy` still link), and the AI's clock reads libc's `SystemTime`. `ai_to_act` answers the
  tri-state 1/0/-1, `preview_sets` a 0/1 status. The last-four-states cache is unchanged, and so is
  the `["\u00a0",<state>]` envelope (it buys nothing on `stringToNewUTF8`, which walks the whole
  string either way, but both sides keep one shape).
- `.cargo/config.toml` holds the link: `-O3`, `-sINITIAL_MEMORY=64MB`, `-sSTACK_SIZE=1MB`,
  `-sMODULARIZE=1 -sEXPORT_ES6=1 -sENVIRONMENT=web,worker,node -sALLOW_MEMORY_GROWTH=1
  -sDYNAMIC_EXECUTION=0` and the exported functions and runtime methods (`UTF8ToString`,
  `stringToNewUTF8`). `-O3` because emcc's default is `-O0` (unoptimized syslibs, no wasm-opt,
  assertions on): the first `-O0` build ran the golden step at 2.5× main. `64MB` because
  `register_all`'s Luau init grew out of the 16MB default (162–213 ms, over budget); `1MB` of stack
  is parity with the old module.
- `rust-toolchain.toml` targets `wasm32-unknown-emscripten`.
- `scripts/build-wasm.sh` clones and installs the pinned emsdk 5.0.7 into `.cache/emsdk` when
  missing (it sets `EMSDK`/`EM_CONFIG`/PATH directly — `emsdk_env.sh` assumes bash and the script
  is `sh` — and pins `CC`/`CXX` for the target to `emcc`/`em++` so a host compiler never leaks in),
  builds the bin, and copies `jackioh-wasm.js` + `jackioh_wasm.wasm` into
  `apps/web/src/wasm/pkg/` as `jackioh_wasm.js` + `jackioh_wasm.wasm`, with a generated
  `jackioh_wasm.d.ts` for the factory's types (emcc emits none).
- `apps/web/src/wasm/index.ts` loads the factory (`wasmBinary`: fetched bytes on the page and in
  the practice worker, bytes from disk under jsdom) and keeps every typed function's signature.
  `loadWasmSync` is async now — emscripten has no synchronous instantiate — and `test/setup.ts`
  awaits it. Strings cross through `put`/`take`/`withArgs`, which free every argument pointer
  through `jackioh_free`.
- `scripts/vercel-install.sh` adds the emscripten target and checks for git/python3 (the emsdk's
  needs); `vercel.json` and `public/_headers` are unchanged (`'wasm-unsafe-eval'` already covers
  the module; `cloudflare-config.test.ts` green, 8 tests).
- emsdk 5.0.7's clang is 23.0.0 against Rust 1.97.0's LLVM 22.1.6 (one-major skew, pinned exactly in
  `build-wasm.sh`). The link holds: Rust's thin LTO stays inside `rustc`, so `emcc` links plain
  wasm objects.

## 2. Baselines and budgets

`main` at the branch point is `f01a13d5d` (pre-Luau): this tree's HEAD cannot build the old
`wasm32-unknown-unknown` module at all — `mlua` 0.12.2 does not compile there (`libc::fclose` and
friends are missing) — so the branch delta covers Luau itself as well as the emscripten runtime.
That is the honest comparison: there is no Luau-bearing `wasm-bindgen` module to compare against.
(Incidental finding: part 4 left `sh scripts/build-wasm.sh` red on this tree for the old target;
this part is the fix.)

The golden step is this part's own probe (the #306 4.7 ms script is not in the repo): one
practice-like step through the wrapper — `chooseAction` + `reduce` + `viewFor` × 2 +
`legalActions` + `seatToAct` + `aiToAct` — timed over 40 fresh states from seeded `cheap20` games
after 5 warm-up steps, in vitest (jsdom). `register_all` is the module's `init()` after
instantiation, in plain node. Same probe, same box, both sides; the box's load moves the absolutes
(quiet vs loaded baselines below) but both sides were measured under each.

| Number | `main` (f01a13d5d) | Branch | Budget | Verdict |
|---|---|---|---|---|
| Module + glue, gzipped (`gzip -9`) | 2,945,562 B (wasm 2,939,781 + glue 5,781; raw 15,566,720 + 31,532) | 3,772,347 B (wasm 3,754,264 + glue 18,083; raw 12,811,988 + 64,142) | within 1 MiB (≤ 4,045,338) | GO (+826,785 B, +0.79 MiB) |
| Golden step, median of 40 | 5.49 ms quiet / 10.01 ms loaded | 8.97 / 9.54 ms (two runs) | ≤ 2× main (≤ ~11–20 ms) | GO (~1.6× the quiet baseline) |
| `register_all` | 50–167 ms over 6 runs (node) | 92–187 ms over 9 runs (node, all < 200) | < 200 ms | GO |
| e2e smoke (specs 01, 06, 10, 13, 19) | n/a (pre-Luau) | not run here — no Chrome or Cypress on this box | green in CI | FOR THE RECONCILE |

Luau's compiler stays in the module: with only the 3 pilot cards' bytecode linked, size passes
with ~222 KB of margin, so nothing was stripped (`mlua` offers no bytecode-only feature to take).

## 3. Go / no-go

GO on every number this box can measure, by the table above. The e2e smoke gate is for the
reconcile's pull request in CI: this part ran the closest local equivalents instead — the web's
`replay`, `hotseat`, `practice/core` and `wire/codes` suites (267 tests) green on the shipped
binary, plus `typecheck`, `eslint` on the touched files, `cargo fmt --check` and
`cargo clippy -p jackioh-wasm --all-targets -- -D warnings` clean.

Decisions the plan did not settle: binary over cdylib (glue needs a program link); `lib.rs` →
`main.rs` (the plan named `lib.rs`); one `jackioh_free` for arguments too (same heap);
async `loadWasmSync`; envelope kept; `-O3`/`64MB`/`1MB` link flags (measured, not guessed);
baseline at `f01a13d5d` (HEAD's old-target build is impossible with Luau); this part's own golden
step (the #306 script is gone); `vercel-install.sh` edited instead of `vercel.json` (the install
step lives there); Luau compiler kept (budget holds). Rejected: a `TextEncoder.encodeInto`
`HEAPU8` input path to buy back crossing speed (gate passes at ~1.6×; complexity not justified —
revisit if CI hardware disagrees), and `-Os` (speed gate needed `-O3`; size still passes).
