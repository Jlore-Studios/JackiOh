#!/bin/sh
# Vercel's install step, run by vercel.json's `installCommand` (which Vercel caps at 256 characters,
# hence a script): the Rust toolchain scripts/build-wasm.sh needs, then the packages.
#
# Vercel's build image ships Rust with rustup under /rust, already on PATH; rustup-init would then
# skip itself and never write $HOME/.cargo/env, so rustup is installed only where there is none.
# `rustup toolchain install` installs what rust-toolchain.toml pins (rustup 1.28 and later; an older
# rustup's `rustup show` does the same), plus the wasm target the module is built for.

set -eu

if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi
rustup toolchain install || rustup show
rustup target add wasm32-unknown-unknown
pnpm install --frozen-lockfile
