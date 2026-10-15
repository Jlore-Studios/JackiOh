#!/bin/sh
# Builds crates/wasm into apps/web/src/wasm/pkg, the module the web client loads
# (docs/v0.3.0/SURFACE.md §10.2). apps/web/package.json's `predev`, `prebuild`, `prebuild:e2e` and
# `pretest` run it, so `pnpm --filter @jackioh/web dev|build|build:e2e|test` always has a fresh pkg.
#
# Idempotent, and it installs nothing that is present:
#   - the wasm32-unknown-emscripten target, added through rustup only when it is missing;
#   - the pinned emsdk (v0.4.0 part 5), cloned into .cache/emsdk (gitignored) and installed there
#     when missing: `emsdk install` fetches the toolchain once, `emsdk activate` rewrites the
#     `.emscripten` config. Part 8 (#450) caches this in CI; until it lands, every machine that
#     builds the module downloads it here, including Vercel and the deploy job.
#   - cargo rebuilds only what changed.
#
# `crates/wasm` is a binary: `cargo` drives the one `emcc` link (whose `-s` flags live in
# `.cargo/config.toml`), which emits the `MODULARIZE` factory `jackioh-wasm.js` beside
# `jackioh_wasm.wasm`; both are copied into `apps/web/src/wasm/pkg/` as `jackioh_wasm.js` and
# `jackioh_wasm.wasm` (gitignored), with a small `jackioh_wasm.d.ts` for the factory's types
# (written here, since `emcc` emits no types).
#
# Needs a Rust toolchain (rust-toolchain.toml pins it), git, python3 and a C++ compiler for the
# host half of the build. On Vercel, scripts/vercel-install.sh (the installCommand) uses the build
# image's own rustup, under /rust and already on PATH, and installs rustup only where there is
# none; a shell that installed it in an earlier step but never re-read its profile is covered by
# sourcing ~/.cargo/env below.
#
# JACKIOH_WASM_PREVIEW: 1 (the default) builds the module with crates/wasm's `preview` feature, R1420's
# `previewSets` (the engine's testkit), which the hotseat route's E2E injection uses to play a set
# before it ships (#552); 0 builds it without, as apps/web's `prebuild` does for the production
# bundle, so a module a player loads can preview nothing. Cargo keeps both builds, so switching costs
# one compile each the first time only.
#
# JACKIOH_WASM_DUOS: 1 (the default) builds crates/wasm's `duos` feature, #936's Duos bindings, which
# only `/dev/duos` calls and only a development build serves; 0 builds stubs in their place, as
# apps/web's `prebuild` (the production bundle) and `pretest` (the unit tests' module, which
# apps/web/src/wasm/size.test.ts weighs for #442 L12) do. Switching relinks the bindings crate only.

set -eu

# v0.4.0 part 5: emsdk 5.0.7's clang is 23.0.0 against Rust 1.97.0's LLVM 22.1.6. The skew is one
# major, and the link holds: Rust's thin LTO stays inside `rustc`, so `emcc` links plain wasm
# objects (proven by `cargo build` below). Pinned exactly; changing it changes every byte built.
EMSDK_VERSION="5.0.7"
WASM_TARGET="wasm32-unknown-emscripten"

ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
EMSDK_DIR="$ROOT/.cache/emsdk"
OUT_DIR="$ROOT/apps/web/src/wasm/pkg"
MODULE_JS="$ROOT/target/$WASM_TARGET/release/jackioh-wasm.js"
MODULE_WASM="$ROOT/target/$WASM_TARGET/release/jackioh_wasm.wasm"

cd "$ROOT"

if ! command -v cargo >/dev/null 2>&1 && [ -f "${HOME:-}/.cargo/env" ]; then
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "build-wasm: cargo is not on PATH. Install Rust (https://rustup.rs); rust-toolchain.toml pins the version." >&2
  exit 1
fi

# The target. rust-toolchain.toml lists it, but a toolchain installed before that line existed has
# none, and rustup adds a toolchain file's targets only when it installs the toolchain itself.
if command -v rustup >/dev/null 2>&1; then
  if ! rustup target list --installed | grep -qx "$WASM_TARGET"; then
    echo "build-wasm: adding the $WASM_TARGET target"
    rustup target add "$WASM_TARGET"
  fi
fi

# emsdk, cloned and installed once into .cache/emsdk.
if [ ! -x "$EMSDK_DIR/emsdk" ]; then
  echo "build-wasm: cloning emsdk $EMSDK_VERSION into $EMSDK_DIR"
  git clone --depth 1 --branch "$EMSDK_VERSION" https://github.com/emscripten-core/emsdk.git "$EMSDK_DIR"
fi
if [ ! -d "$EMSDK_DIR/upstream/emscripten" ]; then
  echo "build-wasm: installing emsdk $EMSDK_VERSION (one download, then cached)"
  (cd "$EMSDK_DIR" && ./emsdk install "$EMSDK_VERSION")
fi
(cd "$EMSDK_DIR" && ./emsdk activate "$EMSDK_VERSION" >/dev/null)

# The toolchain on PATH for `cargo` (which calls `emcc` as its linker) and for the C++ `mlua`
# compiles (`cc` picks `emcc`/`em++` up from PATH for this target). `emsdk_env.sh` is not sourced:
# it assumes bash, and these three variables plus PATH are all `emcc` reads.
NODE_BIN=""
for dir in "$EMSDK_DIR"/node/*_64bit/bin; do
  if [ -x "$dir/node" ]; then NODE_BIN="$dir"; fi
done
if [ -z "$NODE_BIN" ]; then
  echo "build-wasm: no node under $EMSDK_DIR/node; reinstall the emsdk" >&2
  exit 1
fi
export EMSDK="$EMSDK_DIR"
export EM_CONFIG="$EMSDK_DIR/.emscripten"
export EMSDK_NODE="$NODE_BIN/node"
export PATH="$EMSDK_DIR:$EMSDK_DIR/upstream/emscripten:$NODE_BIN:$PATH"
# The C++ for this target is explicit, so a host `CXX`/`CC` never leaks into it; the plain ones
# stay as the host build's fallback, which wants a compiler but takes any.
export CC_wasm32_unknown_emscripten="emcc"
export CXX_wasm32_unknown_emscripten="em++"

FEATURES=""
case "${JACKIOH_WASM_PREVIEW:-1}" in
  1) FEATURES="preview" ;;
  0) ;;
  *)
    echo "build-wasm: JACKIOH_WASM_PREVIEW is 1 (the default) or 0, not \"$JACKIOH_WASM_PREVIEW\"" >&2
    exit 1
    ;;
esac
case "${JACKIOH_WASM_DUOS:-1}" in
  1) FEATURES="${FEATURES:+$FEATURES,}duos" ;;
  0) ;;
  *)
    echo "build-wasm: JACKIOH_WASM_DUOS is 1 (the default) or 0, not \"$JACKIOH_WASM_DUOS\"" >&2
    exit 1
    ;;
esac
if [ -n "$FEATURES" ]; then
  cargo build -p jackioh-wasm --release --target "$WASM_TARGET" --features "$FEATURES"
else
  cargo build -p jackioh-wasm --release --target "$WASM_TARGET"
fi
mkdir -p "$OUT_DIR"
cp "$MODULE_JS" "$OUT_DIR/jackioh_wasm.js"
cp "$MODULE_WASM" "$OUT_DIR/jackioh_wasm.wasm"
cat > "$OUT_DIR/jackioh_wasm.d.ts" <<'TYPES'
// Written by scripts/build-wasm.sh: the types of emscripten's MODULARIZE factory
// (docs/v0.3.0/SURFACE.md §10.2), which emits no types of its own. `apps/web/src/wasm/index.ts`
// is the only importer.
export interface JackiohWasmModule {
  _init(): void;
  _catalog(): number;
  _catalog_version(): number;
  _carried_versions(): number;
  _create_game(args: number): number;
  _begin_game(state: number): number;
  _reduce(state: number, action: number): number;
  _legal_actions(state: number, player: number): number;
  _view_for(state: number, player: number): number;
  _seat_to_act(state: number): number;
  _hash_state(state: number): number;
  _fold(args: number): number;
  _replay_open(args: number, record: number): number;
  _replay_page(checkpoints: number, seat: number, from: number, count: number): number;
  _last_board_for(state: number, seat: number): number;
  _seat_played_by(state: number, home: number): number;
  _find_instance(state: number, instance: number): number;
  _deal_emote_hand(seed: number, seat: number): number;
  _preview_sets(sets: number): number;
  _ai_to_act(state: number, seat: number): number;
  _ai_decide(state: number, seat: number, options: number, deadlineMs: number): number;
  _build_ai_deck(options: number): number;
  _choose_action(state: number, seat: number, rngSeed: number, rngCursor: number): number;
  _constants(): number;
  _craft_preview(recipe: number, cost: number): number;
  _engine_tables(): number;
  _validator(call: number, input: number): number;
  _duos_create(options: number): number;
  _duos_begin(state: number): number;
  _duos_reduce(state: number, action: number): number;
  _duos_legal_actions(state: number, seat: number): number;
  _duos_view_for(state: number, seat: number): number;
  _jackioh_free(text: number): void;
  _jackioh_last_error(): number;
  UTF8ToString(ptr: number): string;
  stringToNewUTF8(str: string): number;
}
declare const createModule: (options?: { wasmBinary?: BufferSource }) => Promise<JackiohWasmModule>;
export default createModule;
TYPES
echo "build-wasm: wrote $OUT_DIR"
