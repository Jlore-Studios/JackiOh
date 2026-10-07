# `.fullsend/`: the v0.3.0 run's scratch

The Rust rewrite (#306) runs as fullsend over the `staging` branch (`docs/v0.3.0/README.md` §3).
This directory is that run's working memory, kept on `staging` only and deleted by part 37 (cull).

- `notes/part-<NN>.md`: each part's builder notes (the fullsend builder template: `BUILDS-RUN`,
  `FILES`, `SURFACE`, `DEPENDS-ON`, `GAPS`), and `notes/part-<NN>.assumptions`, its answers to the
  fixed assumption keys of `docs/v0.3.0/SURFACE.md` §16. Part 31 (contact and reconcile) reads them
  all; `notes/spec-gaps.md` is where it records a hole in SURFACE.md.
- `damage/part-<NN>.txt`: what the compiler said when a part was allowed to build (part 1's
  `cargo check --workspace`, and the contact and green parts later).
- `waves.md`: the commit each wave started and ended at, one `<marker> <sha>` line each (the git
  tags the plan names cannot be pushed from the build sessions).
