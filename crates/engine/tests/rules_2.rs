//! One of the three integration-test binaries for the engine's rule tests: the modules of
//! `rules/` from `p` through `r` (`rules/mod_2.rs`; `tests/rules.rs` has the ones from `a` to `o`
//! and `tests/rules_3.rs` the rest, #2957). The module is `rules` here too, so a test's path,
//! `rules::<x>::…`, is the same in every binary.
//!
//! `#[path]` names the file, and rustc reads a `#[path]` file's own `mod` lines from its directory,
//! as it does a `mod.rs`'s, so every module of `rules/` is found where it is.

#[path = "rules/mod_2.rs"]
mod rules;
