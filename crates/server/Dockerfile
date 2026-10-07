# The server's image (docs/v0.3.0/SURFACE.md §11.3): what Render builds from render.yaml
# (`runtime: docker`, `dockerfilePath: crates/server/Dockerfile`, `dockerContext: .`) and what
# crates/server/tests/deploy/rehearse.sh (CI's `db` job) builds and boots before any deploy.
#
# Build from the repository root, because the server crate builds against the engine, the cards
# and the AI crates beside it in the workspace:
#
#   docker build -f crates/server/Dockerfile -t jackioh-server .
#
# Four stages build `jackioh-server` in release, cargo-chef's way: the tool, the recipe, the
# dependencies, then the server. The dependencies (some 260 crates) sit in a layer of their own that
# a change to the sources alone does not rebuild; the workspace's own four crates and the thin-LTO
# link are compiled on every change. The image is the last stage, a slim Debian with CA
# certificates (the server calls Supabase Auth over TLS and connects to Supabase's Postgres over
# TLS) and the one binary. Nothing at run time needs Node, pnpm or the repository: the catalog
# (crates/cards/catalog.json), the patch history that names the catalog version
# (crates/cards/patches/patches.json) and every migration (crates/server/migrations/*.sql) are
# compiled into the binary with `include_str!`, so the image is the whole deploy.

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------
# On bookworm, the runtime stage's Debian: the binary links the build image's glibc, and plain
# `rust:1.97-slim` is trixie, whose binary asks for GLIBC_2.38 that bookworm's 2.36 does not have
# (the rehearsal's first boot exited on exactly that). Every build stage starts from this one, so
# all of them use the same toolchain, which cargo-chef's cached dependencies need.
#
# cargo-chef itself, at a pinned version and held to its own lock file, in a layer that is rebuilt
# only when this line changes. The registry it was fetched from is dropped; the cook below fetches
# the workspace's own.
FROM rust:1.97-slim-bookworm AS chef
RUN cargo install cargo-chef --version 0.1.78 --locked \
 && rm -rf "$CARGO_HOME/registry"
WORKDIR /src

# The recipe (`cargo chef prepare`): every manifest in the workspace and the lock file, without the
# crates' sources. This stage reruns whenever anything under crates/ changes, which is cheap, and
# writes the same recipe.json byte for byte unless a manifest or the lock file changed; that file is
# all the next stage reads.
#
# Only what the workspace needs to build: the manifest, the lock file every build is held to
# (`--locked`), and the crates. `rust-toolchain.toml` is left out on purpose: it asks rustup for
# rustfmt, clippy and the wasm32 target, none of which a server build uses, and this image already
# carries the toolchain it names. Copying paths rather than `.` also keeps a build context that
# holds node_modules from being read.
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo chef prepare --recipe-path recipe.json

# The dependencies (`cargo chef cook`): the workspace's skeleton rebuilt from the recipe, its
# manifests with an empty source for every target, and every crate `jackioh-server` depends on
# compiled with the release profile (thin LTO included) and the features the real build asks for,
# so the build below reuses all of them. This is the layer that stays cached while Cargo.toml,
# Cargo.lock and the crates' manifests are unchanged.
FROM chef AS build
COPY --from=planner /src/recipe.json recipe.json
RUN cargo chef cook --release --locked -p jackioh-server --recipe-path recipe.json

# The server: the real manifests and sources over the skeleton, and the build the image always ran.
# Only the workspace's own crates (the engine, the cards, the AI and the server) compile here, and
# the server's own compile and its thin-LTO link are most of it. target/ goes in the same layer: the
# image takes only the binary, and a layer cache that keeps every stage (buildx's `mode=max`) then
# holds the dependencies once, in the cook layer, instead of another 300 MB of target/ per build.
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release --locked -p jackioh-server \
 && cp target/release/jackioh-server /jackioh-server \
 && rm -rf target

# ---------------------------------------------------------------------------
# Run
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim

# ca-certificates: the roots rustls trusts for Supabase (Auth, JWKS) and for a TLS DATABASE_URL.
# tini: PID 1 that forwards Render's SIGTERM to the server and reaps children, so a deploy that
# stops this instance stops the server promptly instead of waiting out the kill timeout.
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates tini \
 && rm -rf /var/lib/apt/lists/*

# The server needs no privilege: it listens on $PORT (above 1024) and writes nothing to disk.
RUN useradd --system --no-create-home --shell /usr/sbin/nologin jackioh

COPY --from=build /jackioh-server /app/jackioh-server

USER jackioh

# Render injects PORT (and RENDER_GIT_COMMIT); this is the server's own default, src/env.rs's 8787,
# for a `docker run` that sets neither.
ENV PORT=8787
EXPOSE 8787

# `release` is render.yaml's start: migrate (the ledger, the checksums, one advisory lock per file),
# seed the catalog at the compiled-in catalog version, then serve. Any other subcommand of the
# binary runs in its place: `docker run <image> migrate`, `docker run <image> mint-code`, ….
ENTRYPOINT ["/usr/bin/tini", "--", "/app/jackioh-server"]
CMD ["release"]
