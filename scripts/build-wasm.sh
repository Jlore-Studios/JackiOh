#!/bin/sh
# Builds crates/wasm into apps/web/src/wasm/pkg, the module the web client loads
# (docs/v0.3.0/SURFACE.md §10.2). apps/web/package.json's `predev`, `prebuild`, `prebuild:staging`,
# `prebuild:e2e` and `pretest` run it, so `pnpm --filter @jackioh/web
# dev|build|build:staging|build:e2e|test` always has it fresh. JACKIOH_WASM_C=1 builds the C module
# too (below).
#
# Idempotent, and it installs nothing that is present:
#   - the wasm32-unknown-unknown target, added through rustup only when it is missing;
#   - wasm-bindgen-cli, downloaded from its GitHub release tarball into .cache/bin/ (gitignored),
#     never compiled, and skipped when .cache/bin/wasm-bindgen already prints the pinned version.
#     It must be the very version of the `wasm-bindgen` crate (Cargo.toml pins `=0.2.129`), or it
#     refuses the module;
#   - cargo rebuilds only what changed, and wasm-bindgen rewrites the same pkg. Both deploys build on
#     GitHub's runners, whose setup action restores target/ (Swatinem/rust-cache), so nothing here
#     keeps the built module.
#
# The output is apps/web/src/wasm/pkg/{jackioh_wasm.js, jackioh_wasm.d.ts, jackioh_wasm_bg.wasm,
# jackioh_wasm_bg.wasm.d.ts}, gitignored; `--target web` makes the glue an ES module whose default
# export fetches the .wasm and whose `initSync` takes its bytes (apps/web/src/wasm/index.ts).
#
# Needs a Rust toolchain (rust-toolchain.toml pins it). A shell that installed rustup in an earlier
# step but never re-read its profile is covered by sourcing ~/.cargo/env below.

# JACKIOH_WASM_PREVIEW: 1 (the default) builds the module with crates/wasm's `preview` feature, R1420's
# `previewSets` (the engine's testkit), which the hotseat route's E2E injection uses to play a set
# before it ships (#552); 0 builds it without, as apps/web's `prebuild` does for the production
# bundle, so a module a player loads can preview nothing. Cargo keeps both builds, so switching costs
# one compile each the first time only.
#
# JACKIOH_WASM_DUOS: 0 (the default) builds stubs in place of crates/wasm's `duos` feature, #936's
# Duos bindings; 1 builds them, as apps/web's `predev` does, since only `/dev/duos` calls them and
# only the dev server serves it. A networked co-op game needs none (the server plays it), so every
# other module (`build:e2e`'s, `test`'s, the production bundle's) leaves them out. Switching
# recompiles the bindings crate only.
#
# JACKIOH_WASM_NORMAL: 1 (the default) builds the browser module; 0 skips it, with its wasm-bindgen,
# and builds the C module alone (JACKIOH_WASM_C=1; with it still 0 nothing is built, and the script
# fails). CI builds each module in a job of its own, side by side (ci.yml's `web (wasm module)` and
# `web (wasm c module)`).
#
# JACKIOH_WASM_C: 0 (the default) skips the C module; 1 builds it, #3093's `crates/wasm/c`, the engine's
# C boundary as a plain `wasm32-unknown-unknown` cdylib with no host imports, with neither `preview` nor
# `duos`, and checks with node that its import section is empty. It needs no emcc and no wasm-bindgen,
# and nothing in apps/web loads it (a host that is not JavaScript does: crates/wasm/c/README.md), so
# it writes nothing into pkg/: the module is `jackioh_wasm_c.wasm` in cargo's release directory for
# that target. ci.yml's `web (wasm c module)` job sets it, with JACKIOH_WASM_NORMAL=0, so the module a
# Roblox port translates (#2898) cannot rot, and records its size.
#
# CARGO_TARGET_DIR: where cargo builds, which is where the modules are read from; unset, target/
# under the root. Cargo runs from the root, so a relative one is the root's. JACKIOH_CACHE_DIR: where
# wasm-bindgen (bin/) is kept; unset, .cache/ under the root. The night bot's model job sets both, so
# every item's worktree builds into the checkout's cached target/ and uses the one wasm-bindgen the
# job restored, instead of starting from nothing each time (#1263).

set -eu

WASM_BINDGEN_VERSION="0.2.129"
WASM_TARGET="wasm32-unknown-unknown"

ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
TOOLS_DIR="${JACKIOH_CACHE_DIR:-$ROOT/.cache}"
BIN_DIR="$TOOLS_DIR/bin"
WASM_BINDGEN="$BIN_DIR/wasm-bindgen"
OUT_DIR="$ROOT/apps/web/src/wasm/pkg"
MODULE="$TARGET_DIR/$WASM_TARGET/release/jackioh_wasm.wasm"
C_MODULE="$TARGET_DIR/$WASM_TARGET/release/jackioh_wasm_c.wasm"

cd "$ROOT"

if ! command -v cargo >/dev/null 2>&1 && [ -f "${HOME:-}/.cargo/env" ]; then
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "build-wasm: cargo is not on PATH. Install Rust (https://rustup.rs); rust-toolchain.toml pins the version." >&2
  exit 1
fi

case "${JACKIOH_WASM_NORMAL:-1}" in
  1 | 0) ;;
  *)
    echo "build-wasm: JACKIOH_WASM_NORMAL is 1 (the default) or 0, not \"$JACKIOH_WASM_NORMAL\"" >&2
    exit 1
    ;;
esac
case "${JACKIOH_WASM_C:-0}" in
  0 | 1) ;;
  *)
    echo "build-wasm: JACKIOH_WASM_C is 0 (the default) or 1, not \"$JACKIOH_WASM_C\"" >&2
    exit 1
    ;;
esac
if [ "${JACKIOH_WASM_NORMAL:-1}" = 0 ] && [ "${JACKIOH_WASM_C:-0}" != 1 ]; then
  echo "build-wasm: JACKIOH_WASM_NORMAL=0 without JACKIOH_WASM_C=1 builds nothing" >&2
  exit 1
fi

if [ "${JACKIOH_WASM_NORMAL:-1}" = 1 ]; then
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
        echo "  cargo install wasm-bindgen-cli --version $WASM_BINDGEN_VERSION --root $TOOLS_DIR" >&2
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
fi

# #3093: the C module, the engine's C boundary for a host that is not JavaScript. JACKIOH_WASM_C 0 (the
# default) skips it. It shares the normal module's target, and so its rustup check, but takes no
# feature: a module with no way to preview a set and no Duos body is the one such a host ships. A link
# that imported anything from its host (a wasm-bindgen glue, a JavaScript function, a system call)
# would not instantiate with an empty import object, so the built module's imports are read with node,
# which `WebAssembly.Module.imports` answers, and any one fails the build.
if [ "${JACKIOH_WASM_C:-0}" = 1 ]; then
  if command -v rustup >/dev/null 2>&1; then
    if ! rustup target list --installed | grep -qx "$WASM_TARGET"; then
      echo "build-wasm: adding the $WASM_TARGET target"
      rustup target add "$WASM_TARGET"
    fi
  fi
  cargo build -p jackioh-wasm-c --release --target "$WASM_TARGET"
  if ! command -v node >/dev/null 2>&1; then
    echo "build-wasm: node is needed to check that the C module imports nothing (JACKIOH_WASM_C=1)" >&2
    exit 1
  fi
  node -e 'const fs = require("fs"); const found = WebAssembly.Module.imports(new WebAssembly.Module(fs.readFileSync(process.argv[1]))); if (found.length > 0) { console.error("build-wasm: the C module imports " + JSON.stringify(found)); process.exit(1); }' "$C_MODULE"
  echo "build-wasm: built $C_MODULE, which imports nothing"
fi
