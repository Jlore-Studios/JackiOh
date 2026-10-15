#!/bin/sh
# Builds crates/wasm into apps/web/src/wasm/pkg, the module the web client loads
# (docs/v0.3.0/SURFACE.md §10.2), then crates/wasm/luau into apps/web/src/wasm/pkg/luau, the Luau
# module (#919, SPEC §12.10), unless JACKIOH_WASM_LUAU=0. apps/web/package.json's `predev`,
# `prebuild`, `prebuild:e2e` and `pretest` run it, so `pnpm --filter @jackioh/web
# dev|build|build:e2e|test` always has both fresh. The Luau module sits inside pkg/ so everything
# that handles the normal module's output (.gitignore, eslint's ignores, CI's `web-build` artifact)
# carries it too.
#
# Idempotent, and it installs nothing that is present:
#   - the wasm32-unknown-unknown target, added through rustup only when it is missing;
#   - wasm-bindgen-cli, downloaded from its GitHub release tarball into .cache/bin/ (gitignored),
#     never compiled, and skipped when .cache/bin/wasm-bindgen already prints the pinned version.
#     It must be the very version of the `wasm-bindgen` crate (Cargo.toml pins `=0.2.129`), or it
#     refuses the module;
#   - the wasm32-unknown-emscripten target and the pinned emsdk (EMSDK_VERSION below), cloned into
#     .cache/emsdk (gitignored) and installed there when missing: `emsdk install` fetches the
#     toolchain once, `emsdk activate` rewrites its `.emscripten` config. CI's setup action caches
#     .cache/emsdk keyed on the EMSDK_VERSION line;
#   - cargo rebuilds only what changed, and wasm-bindgen rewrites the same pkg.
#
# The output is apps/web/src/wasm/pkg/{jackioh_wasm.js, jackioh_wasm.d.ts, jackioh_wasm_bg.wasm,
# jackioh_wasm_bg.wasm.d.ts}, gitignored; `--target web` makes the glue an ES module whose default
# export fetches the .wasm and whose `initSync` takes its bytes (apps/web/src/wasm/index.ts).
#
# Needs a Rust toolchain (rust-toolchain.toml pins it), and for the Luau module git and python3 (the
# emsdk's). On Vercel, scripts/vercel-install.sh (the
# installCommand) uses the build image's own rustup, under /rust and already on PATH, and installs
# rustup only where there is none; a shell that installed it in an earlier step but never re-read
# its profile is covered by sourcing ~/.cargo/env below.

# JACKIOH_WASM_PREVIEW: 1 (the default) builds the module with crates/wasm's `preview` feature, R1420's
# `previewSets` (the engine's testkit), which the hotseat route's E2E injection uses to play a set
# before it ships (#552); 0 builds it without, as apps/web's `prebuild` does for the production
# bundle, so a module a player loads can preview nothing. Cargo keeps both builds, so switching costs
# one compile each the first time only.
#
# JACKIOH_WASM_DUOS: 0 (the default) builds stubs in place of crates/wasm's `duos` feature, #936's
# Duos bindings; 1 builds them, as apps/web's `predev` does, since only `/dev/duos` calls them and
# only the dev server serves it. A networked co-op game needs none (the server plays it), so every
# other module (`build:e2e`'s, `test`'s, the production bundle's) leaves them out. The Luau module
# never carries them. Switching recompiles the bindings crate only.

set -eu

WASM_BINDGEN_VERSION="0.2.129"
WASM_TARGET="wasm32-unknown-unknown"
# emsdk 5.0.7's clang is 23.0.0 against Rust 1.97.0's LLVM 22.1.6. The skew is one major, and the
# link holds: Rust's thin LTO stays inside `rustc`, so `emcc` links plain wasm objects. Pinned
# exactly; changing it changes every byte built. CI's setup action reads this line.
EMSDK_VERSION="5.0.7"
LUAU_TARGET="wasm32-unknown-emscripten"

ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
BIN_DIR="$ROOT/.cache/bin"
WASM_BINDGEN="$BIN_DIR/wasm-bindgen"
OUT_DIR="$ROOT/apps/web/src/wasm/pkg"
MODULE="$ROOT/target/$WASM_TARGET/release/jackioh_wasm.wasm"
EMSDK_DIR="$ROOT/.cache/emsdk"
LUAU_OUT_DIR="$OUT_DIR/luau"
LUAU_JS="$ROOT/target/$LUAU_TARGET/release/jackioh-wasm-luau.js"
LUAU_WASM="$ROOT/target/$LUAU_TARGET/release/jackioh_wasm_luau.wasm"

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

# wasm-bindgen-cli, from the release tarball.
installed=$("$WASM_BINDGEN" --version 2>/dev/null || true)
if [ "$installed" != "wasm-bindgen $WASM_BINDGEN_VERSION" ]; then
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) triple="x86_64-unknown-linux-musl" ;;
    Linux-aarch64 | Linux-arm64) triple="aarch64-unknown-linux-gnu" ;;
    Darwin-x86_64) triple="x86_64-apple-darwin" ;;
    Darwin-arm64) triple="aarch64-apple-darwin" ;;
    *)
      echo "build-wasm: no wasm-bindgen-cli release for $(uname -s)-$(uname -m); install it with" >&2
      echo "  cargo install wasm-bindgen-cli --version $WASM_BINDGEN_VERSION --root $ROOT/.cache" >&2
      exit 1
      ;;
  esac
  name="wasm-bindgen-$WASM_BINDGEN_VERSION-$triple"
  url="https://github.com/wasm-bindgen/wasm-bindgen/releases/download/$WASM_BINDGEN_VERSION/$name.tar.gz"
  echo "build-wasm: downloading $name"
  scratch=$(mktemp -d)
  trap 'rm -rf "$scratch"' EXIT
  curl --proto '=https' --tlsv1.2 -fsSL "$url" -o "$scratch/wasm-bindgen.tar.gz"
  tar -xzf "$scratch/wasm-bindgen.tar.gz" -C "$scratch"
  mkdir -p "$BIN_DIR"
  cp "$scratch/$name/wasm-bindgen" "$WASM_BINDGEN"
  chmod +x "$WASM_BINDGEN"
  installed=$("$WASM_BINDGEN" --version)
  if [ "$installed" != "wasm-bindgen $WASM_BINDGEN_VERSION" ]; then
    echo "build-wasm: the downloaded wasm-bindgen says \"$installed\", not $WASM_BINDGEN_VERSION" >&2
    exit 1
  fi
fi

FEATURES=""
case "${JACKIOH_WASM_PREVIEW:-1}" in
  1) FEATURES="preview" ;;
  0) ;;
  *)
    echo "build-wasm: JACKIOH_WASM_PREVIEW is 1 (the default) or 0, not \"$JACKIOH_WASM_PREVIEW\"" >&2
    exit 1
    ;;
esac
case "${JACKIOH_WASM_DUOS:-0}" in
  1) FEATURES="${FEATURES:+$FEATURES,}duos" ;;
  0) ;;
  *)
    echo "build-wasm: JACKIOH_WASM_DUOS is 0 (the default) or 1, not \"$JACKIOH_WASM_DUOS\"" >&2
    exit 1
    ;;
esac
if [ -n "$FEATURES" ]; then
  cargo build -p jackioh-wasm --release --target "$WASM_TARGET" --features "$FEATURES"
else
  cargo build -p jackioh-wasm --release --target "$WASM_TARGET"
fi
mkdir -p "$OUT_DIR"
"$WASM_BINDGEN" --target web --out-dir "$OUT_DIR" "$MODULE"
echo "build-wasm: wrote $OUT_DIR"

# R915009, #919: the Luau module, the engine with the card kit, which the web loads only for a
# practice game with a custom card and for the creator (apps/web/src/wasm/luau.ts). JACKIOH_WASM_LUAU
# 1 (the default) builds it, so every `dev`, `build`, `build:e2e` and `test` has both; 0 skips it,
# for a quick rebuild of the normal module alone (the creator and a practice game with a custom card
# then cannot load). `crates/wasm/luau` is a binary: `cargo` drives the one `emcc` link (its `-s`
# flags live in `.cargo/config.toml`), which emits the MODULARIZE factory `jackioh-wasm-luau.js`
# beside `jackioh_wasm_luau.wasm`; both are copied into pkg/luau/, with a small
# `jackioh_wasm_luau.d.ts` for the factory's types (written here, since `emcc` emits none).
case "${JACKIOH_WASM_LUAU:-1}" in
  1) ;;
  0) exit 0 ;;
  *)
    echo "build-wasm: JACKIOH_WASM_LUAU is 1 (the default) or 0, not \"$JACKIOH_WASM_LUAU\"" >&2
    exit 1
    ;;
esac

if command -v rustup >/dev/null 2>&1; then
  if ! rustup target list --installed | grep -qx "$LUAU_TARGET"; then
    echo "build-wasm: adding the $LUAU_TARGET target"
    rustup target add "$LUAU_TARGET"
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

case "${JACKIOH_WASM_PREVIEW:-1}" in
  1) cargo build -p jackioh-wasm-luau --release --target "$LUAU_TARGET" --features preview ;;
  0) cargo build -p jackioh-wasm-luau --release --target "$LUAU_TARGET" ;;
  *)
    echo "build-wasm: JACKIOH_WASM_PREVIEW is 1 (the default) or 0, not \"$JACKIOH_WASM_PREVIEW\"" >&2
    exit 1
    ;;
esac
mkdir -p "$LUAU_OUT_DIR"
cp "$LUAU_JS" "$LUAU_OUT_DIR/jackioh_wasm_luau.js"
cp "$LUAU_WASM" "$LUAU_OUT_DIR/jackioh_wasm_luau.wasm"
cat > "$LUAU_OUT_DIR/jackioh_wasm_luau.d.ts" <<'TYPES'
// Written by scripts/build-wasm.sh: the types of emscripten's MODULARIZE factory
// (docs/v0.3.0/SURFACE.md §10.2), which emits no types of its own. `apps/web/src/wasm/luau.ts` is
// the only importer.
export interface JackiohWasmLuauModule {
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
  _check_custom_card(card: number): number;
  _jackioh_free(text: number): void;
  _jackioh_last_error(): number;
  UTF8ToString(ptr: number): string;
  stringToNewUTF8(str: string): number;
}
declare const createModule: (options?: { wasmBinary?: BufferSource }) => Promise<JackiohWasmLuauModule>;
export default createModule;
TYPES
echo "build-wasm: wrote $LUAU_OUT_DIR"
