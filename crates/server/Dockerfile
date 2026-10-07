# The server's image (docs/v0.3.0/SURFACE.md §11.3): what Render builds from render.yaml
# (`runtime: docker`, `dockerfilePath: crates/server/Dockerfile`, `dockerContext: .`) and what
# crates/server/tests/deploy/rehearse.sh (CI's `db` job) builds and boots before any deploy.
#
# Build from the repository root, because the server crate builds against the engine, the cards
# and the AI crates beside it in the workspace:
#
#   docker build -f crates/server/Dockerfile -t jackioh-server .
#
# Two stages. The first compiles `jackioh-server` in release; the second is a slim Debian with
# CA certificates (the server calls Supabase Auth over TLS and connects to Supabase's Postgres over
# TLS) and the one binary. Nothing at run time needs Node, pnpm or the repository: the catalog
# (crates/cards/catalog.json), the patch history that names the catalog version
# (crates/cards/patches/patches.json) and every migration (crates/server/migrations/*.sql) are
# compiled into the binary with `include_str!`, so the image is the whole deploy.

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------
# On bookworm, the runtime stage's Debian: the binary links the build image's glibc, and plain
# `rust:1.97-slim` is trixie, whose binary asks for GLIBC_2.38 that bookworm's 2.36 does not have
# (the rehearsal's first boot exited on exactly that).
FROM rust:1.97-slim-bookworm AS build

WORKDIR /src

# Only what the workspace needs to build: the manifest, the lock file every build is held to
# (`--locked`), and the crates. `rust-toolchain.toml` is left out on purpose: it asks rustup for
# rustfmt, clippy and the wasm32 target, none of which a server build uses, and this image already
# carries the toolchain it names. Copying paths rather than `.` also keeps a build context that
# holds node_modules from being read.
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release --locked -p jackioh-server \
 && cp target/release/jackioh-server /jackioh-server

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
