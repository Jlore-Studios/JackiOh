//! The AI integration tests too slow to share the `ai` binary (#873), first half: full `play_match`
//! games (`match_`) and the shadow-ban sweeps (`shadow_ban`), at AI_GATE_BUDGET. CI runs this as
//! `rust (test ai slow a)`; locally `cargo test -p jackioh-ai --test ai_slow_a`.
//!
//! `#[path]` because the modules live beside the rest in `tests/ai/`, whose `mod.rs` does not name
//! them, so that each runs in one binary only.

#[path = "ai/match_.rs"]
mod match_;
#[path = "ai/shadow_ban.rs"]
mod shadow_ban;
#[path = "ai/support.rs"]
mod support;
