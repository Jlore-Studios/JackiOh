//! The one integration-test binary for the ported AI tests (SURFACE §1): every
//! `packages/ai/test/<x>.test.ts` is a module of `ai/`.
//!
//! `#[path]` because `tests/ai.rs` and `tests/ai/mod.rs` would otherwise both be candidates for
//! `mod ai;` here at the crate root of this binary.

#[path = "ai/mod.rs"]
mod ai;
