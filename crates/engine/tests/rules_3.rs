//! The other integration-test binary for the engine's rule tests: the modules of `rules/` from
//! `s` on (`rules/mod_3.rs`; `tests/rules.rs` has the ones from `a` to `o`, `tests/rules_2.rs`
//! the ones from `p` through `r`, #2957). The module is `rules` here too, so a test's path,
//! `rules::<x>::…`, is the same in every binary.
//!
//! `#[path]` names the file, and rustc reads a `#[path]` file's own `mod` lines from its directory,
//! as it does a `mod.rs`'s, so every module of `rules/` is found where it is.

#[path = "rules/mod_3.rs"]
mod rules;
