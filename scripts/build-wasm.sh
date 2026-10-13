#!/bin/sh
# Builds crates/wasm into apps/web/src/wasm/pkg, the module the web client loads
# (docs/v0.3.0/SURFACE.md §10.2). apps/web/package.json's `predev`, `prebuild`, `prebuild:e2e` and
# `pretest` run it, so `pnpm --filter @jackioh/web dev|build|build:e2e|test` always has a fresh pkg.
#
# Idempotent, and it installs nothing that is present:
#   - the wasm32-unknown-unknown target, added through rustup only when it is missing;
#   - wasm-bindgen-cli, downloaded from its GitHub release tarball into .cache/bin/ (gitignored),
#     never compiled, and skipped when .cache/bin/wasm-bindgen already prints the pinned version.
#     It must be the very version of the `wasm-bindgen` crate (Cargo.toml pins `=0.2.129`), or it
#     refuses the module;
#   - cargo rebuilds only what changed, and wasm-bindgen rewrites the same pkg.
#
# The output is apps/web/src/wasm/pkg/{jackioh_wasm.js, jackioh_wasm.d.ts, jackioh_wasm_bg.wasm,
# jackioh_wasm_bg.wasm.d.ts}, gitignored; `--target web` makes the glue an ES module whose default
# export fetches the .wasm and whose `initSync` takes its bytes (apps/web/src/wasm/index.ts).
#
# Needs a Rust toolchain (rust-toolchain.toml pins it). On Vercel, scripts/vercel-install.sh (the
# installCommand) uses the build image's own rustup, under /rust and already on PATH, and installs
# rustup only where there is none; a shell that installed it in an earlier step but never re-read
# its profile is covered by sourcing ~/.cargo/env below.

# JACKIOH_WASM_PREVIEW: 1 (the default) builds the module with crates/wasm's `preview` feature, R1420's
# `previewSets` (the engine's testkit), which the hotseat route's E2E injection uses to play a set
# before it ships (#552); 0 builds it without, as apps/web's `prebuild` does for the production
# bundle, so a module a player loads can preview nothing. Cargo keeps both builds, so switching costs
# one compile each the first time only.

set -eu

WASM_BINDGEN_VERSION="0.2.129"
WASM_TARGET="wasm32-unknown-unknown"

ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
BIN_DIR="$ROOT/.cache/bin"
WASM_BINDGEN="$BIN_DIR/wasm-bindgen"
OUT_DIR="$ROOT/apps/web/src/wasm/pkg"
MODULE="$ROOT/target/$WASM_TARGET/release/jackioh_wasm.wasm"

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

case "${JACKIOH_WASM_PREVIEW:-1}" in
  1) cargo build -p jackioh-wasm --release --target "$WASM_TARGET" --features preview ;;
  0) cargo build -p jackioh-wasm --release --target "$WASM_TARGET" ;;
  *)
    echo "build-wasm: JACKIOH_WASM_PREVIEW is 1 (the default) or 0, not \"$JACKIOH_WASM_PREVIEW\"" >&2
    exit 1
    ;;
esac
mkdir -p "$OUT_DIR"
"$WASM_BINDGEN" --target web --out-dir "$OUT_DIR" "$MODULE"
echo "build-wasm: wrote $OUT_DIR"
