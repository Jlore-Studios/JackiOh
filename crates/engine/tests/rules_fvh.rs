//! The engine's rule tests for the Felinors vs Humans set's engine work (wave 1, stage A, tracker #5220):
//! the modules of `rules/` named `fvh_*` (`rules/mod_fvh.rs`), one per part of the plan of record
//! (`docs/felinors-vs-humans-set.md`, FV3): E1's keywords, E2's lasting effects, E3's verbs, E4's
//! discovery and F's set. The module is `rules` here too, so a test's path, `rules::<x>::…`, is the same
//! in every rules binary.
//!
//! `#[path]` names the file, and rustc reads a `#[path]` file's own `mod` lines from its directory,
//! as it does a `mod.rs`'s, so every module of `rules/` is found where it is.

#[path = "rules/mod_fvh.rs"]
mod rules;
