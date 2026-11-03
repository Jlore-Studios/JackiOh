//! One of the three integration-test binaries for the engine's rule tests (SURFACE §1): the modules of
//! `rules/` from `a` to `o` here, the ones from `p` through `r` in `rules_2` and the rest in `rules_3`
//! (`rules/mod.rs` says why, #2090, #2957). Run with `cargo test -p jackioh-engine --features testkit`
//! (Cargo.toml's `required-features`).
//!
//! `#[path]` because `tests/rules.rs` and `tests/rules/mod.rs` would otherwise both be candidates
//! for `mod rules;` here at the crate root of this binary.

#[path = "rules/mod.rs"]
mod rules;
