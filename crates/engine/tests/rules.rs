//! One of the two integration-test binaries for the ported engine tests (SURFACE §1): every
//! `packages/engine/test/<x>.test.ts` is a module of `rules/`, those from `a` to `o` here and the
//! rest in `rules_2` (`rules/mod.rs` says why, #2090). Run with
//! `cargo test -p jackioh-engine --features testkit` (Cargo.toml's `required-features`).
//!
//! `#[path]` because `tests/rules.rs` and `tests/rules/mod.rs` would otherwise both be candidates
//! for `mod rules;` here at the crate root of this binary.

#[path = "rules/mod.rs"]
mod rules;
