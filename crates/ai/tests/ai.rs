//! The integration-test binary for the ported AI tests (SURFACE §1): every
//! `packages/ai/test/<x>.test.ts` is a module of `ai/`. The four that play the most games run in
//! `ai_slow_a.rs` and `ai_slow_b.rs` instead, so CI can give each a job of its own (#873).
//!
//! `#[path]` because `tests/ai.rs` and `tests/ai/mod.rs` would otherwise both be candidates for
//! `mod ai;` here at the crate root of this binary.

#[path = "ai/mod.rs"]
mod ai;
