//! The other integration-test binary for the engine's rule tests: the modules of `rules/` from `p`
//! on (`rules/mod_2.rs`; `tests/rules.rs` has the rest, #2090). The module is `rules` here too, so
//! a test's path, `rules::<x>::…`, is the same in either binary.
//!
//! `#[path]` names the file, and rustc reads a `#[path]` file's own `mod` lines from its directory,
//! as it does a `mod.rs`'s, so every module of `rules/` is found where it is.

#[path = "rules/mod_2.rs"]
mod rules;
