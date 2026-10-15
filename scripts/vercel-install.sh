#!/bin/sh
# Vercel's install step, run by vercel.json's `installCommand` (which Vercel caps at 256 characters,
# hence a script): the Rust toolchain scripts/build-wasm.sh needs, then the packages.
#
# Vercel's build image ships Rust with rustup under /rust, already on PATH; rustup-init would then
# skip itself and never write $HOME/.cargo/env, so rustup is installed only where there is none.
# `rustup toolchain install` installs what rust-toolchain.toml pins (rustup 1.28 and later; an older
# rustup's `rustup show` does the same), plus the two wasm targets: the normal module's and the Luau
# module's (#919). scripts/build-wasm.sh clones and installs the pinned emsdk into .cache/emsdk itself
# when it is missing, so this step only checks for the tools that clone needs.

set -eu

if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi
rustup toolchain install || rustup show
rustup target add wasm32-unknown-unknown wasm32-unknown-emscripten
if ! command -v git >/dev/null 2>&1 || ! command -v python3 >/dev/null 2>&1; then
  echo "vercel-install: the Luau module's build needs git and python3 for the emsdk" >&2
  exit 1
fi
pnpm install --frozen-lockfile
